//! Envelope transformations applied by the agent-native reduction pipeline.
//!
//! Single responsibility: given a `serde_json` value, locate the result array and
//! reshape it. Nothing here reads CLI flags or decides *whether* a knob applies —
//! that is the plan's job in [`super::AgentOps`]. Keeping the two apart means a
//! change to argument parsing cannot silently alter how a payload is rewritten.

use std::cmp::Ordering;

use serde_json::{Map, Value};

use super::filter::{resolve_path, scalar_text};
use super::{EMITTED_COUNT_KEYS, RESULT_ARRAY_KEYS};

/// Name of the array key inside `data`, or `None` when `data` is itself an array
/// or carries no array of objects.
///
/// Probes [`RESULT_ARRAY_KEYS`] in order first, then falls back to the first array
/// whose head element is an object. An array of scalars is never a result list.
pub(super) fn locate_array_key(data: &Value) -> Option<String> {
    let obj = data.as_object()?;
    for key in RESULT_ARRAY_KEYS {
        if obj.get(*key).is_some_and(Value::is_array) {
            return Some((*key).to_string());
        }
    }
    obj.iter()
        .find(|(_, v)| {
            v.as_array()
                .is_some_and(|a| a.first().is_some_and(Value::is_object))
        })
        .map(|(k, _)| k.clone())
}

/// Mutable handle to the result array, addressed by `key` or as `data` itself.
pub(super) fn array_mut<'a>(data: &'a mut Value, key: Option<&str>) -> Option<&'a mut Vec<Value>> {
    match key {
        Some(k) => data.get_mut(k)?.as_array_mut(),
        None => data.as_array_mut(),
    }
}

/// Drop later elements repeating an already-seen value at `key`.
pub(super) fn dedupe(list: &mut Vec<Value>, key: &str, strip: &[&str]) {
    let mut seen: Vec<String> = Vec::new();
    list.retain(|el| {
        let Some(text) = resolve_path(el, key, strip).and_then(scalar_text) else {
            // Elements without the key are always kept — dropping them would
            // silently delete rows the caller never asked to deduplicate.
            return true;
        };
        if seen.contains(&text) {
            false
        } else {
            seen.push(text);
            true
        }
    });
}

/// Sort ascending by the value at `key`, keeping ties in their original order.
///
/// The sort is **stable** on purpose: reduction runs after the upstream ordering
/// (relevance from crates.io, index order from `all.html`), so equal keys must not
/// scramble a ranking the caller may still be relying on.
///
/// Comparison is by JSON type, not by rendered text: two numbers compare
/// numerically, so `9` sorts before `10` instead of after it. Elements missing the
/// key sort **last**, because an absent value is not "smaller than everything" — it
/// is unknown, and burying unknowns at the top would hide the ranked head.
pub(super) fn sort_by(list: &mut [Value], key: &str, strip: &[&str]) {
    list.sort_by(|a, b| {
        let ka = resolve_path(a, key, strip);
        let kb = resolve_path(b, key, strip);
        match (ka, kb) {
            (None, None) => Ordering::Equal,
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (Some(x), Some(y)) => compare_values(x, y),
        }
    });
}

/// Total order over the JSON values a sort key can hold.
///
/// `partial_cmp` on floats can return `None` for NaN, which would make the ordering
/// non-total and leave the sort's result unspecified. Falling back to `Equal` keeps
/// the comparator total, and stability then preserves the original relative order.
fn compare_values(a: &Value, b: &Value) -> Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x
            .as_f64()
            .partial_cmp(&y.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(x), Value::String(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        // Mixed or composite kinds have no meaningful ordering; compare their
        // scalar rendering so the result is deterministic rather than arbitrary.
        _ => scalar_text(a)
            .unwrap_or_default()
            .cmp(&scalar_text(b).unwrap_or_default()),
    }
}

/// Build a new object holding only `paths`. Missing paths are skipped, never
/// emitted as JSON `null` (a dead key costs the agent tokens for no information).
pub(super) fn project(element: &Value, paths: &[String], strip: &[&str]) -> Value {
    let mut out = Map::new();
    for path in paths {
        if let Some(found) = resolve_path(element, path, strip) {
            let leaf = path.rsplit('.').next().unwrap_or(path.as_str());
            out.insert(leaf.to_string(), found.clone());
        }
    }
    Value::Object(out)
}

/// Rewrite sibling counters that describe the result array to its current length.
///
/// # Why this is necessary
///
/// `search-in-crate` publishes `emitted` as "hits actually emitted". Once reduction
/// drops rows, that field describes an array that no longer exists: measured live,
/// `--filter kind=struct` left `emitted: 62` beside 32 hits. An agent paginating on
/// `emitted` would then be wrong by thirty elements. A field that names the array
/// has to follow the array.
///
/// # Why the allowlist is explicit
///
/// Only keys in [`EMITTED_COUNT_KEYS`] are touched. The tempting generic rule —
/// "rewrite any integer sibling equal to the pre-reduction length" — destroys
/// information: in that same envelope `total` also held 62, yet `total` counts what
/// the upstream index classified, not what this envelope carries. Rewriting it would
/// erase the only number that survives reduction intact.
pub(super) fn reconcile_counts(data: &mut Value, len: usize) {
    let Some(obj) = data.as_object_mut() else {
        return;
    };
    for key in EMITTED_COUNT_KEYS {
        if let Some(slot) = obj.get_mut(*key) {
            if slot.is_u64() {
                *slot = Value::from(len);
            }
        }
    }
}

/// Truncate every string in the tree to `limit` **characters** (never bytes, so a
/// multi-byte sequence is never split). Returns `true` when anything was shortened.
pub(super) fn truncate_strings(v: &mut Value, limit: usize) -> bool {
    match v {
        Value::String(s) => {
            if s.chars().count() > limit {
                *s = s.chars().take(limit).collect();
                true
            } else {
                false
            }
        }
        Value::Array(a) => a
            .iter_mut()
            .fold(false, |acc, e| truncate_strings(e, limit) || acc),
        Value::Object(o) => o
            .iter_mut()
            .fold(false, |acc, (_, e)| truncate_strings(e, limit) || acc),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locate_array_key_prefers_known_keys_then_arrays_of_objects() {
        let known = serde_json::json!({"notes": [{"a": 1}], "hits": []});
        assert_eq!(locate_array_key(&known).as_deref(), Some("hits"));
        let fallback = serde_json::json!({"tags": ["a", "b"], "rows": [{"a": 1}]});
        assert_eq!(
            locate_array_key(&fallback).as_deref(),
            Some("rows"),
            "arrays of scalars are not result lists"
        );
        assert!(locate_array_key(&serde_json::json!({"a": 1})).is_none());
    }

    #[test]
    fn project_skips_missing_paths_instead_of_emitting_null() {
        let el = serde_json::json!({"name": "serde", "version": "1.0.0"});
        let paths = vec!["name".to_string(), "downloads".to_string()];
        let out = project(&el, &paths, &[]);
        assert_eq!(out, serde_json::json!({"name": "serde"}));
        assert!(out.get("downloads").is_none());
    }

    #[test]
    fn truncate_strings_counts_characters_not_bytes() {
        let mut v = serde_json::json!({"a": "áéíóúx", "n": 42});
        assert!(truncate_strings(&mut v, 5));
        assert_eq!(v["a"], "áéíóú");
        // Non-strings are untouched and never report a truncation.
        assert_eq!(v["n"], 42);
        assert!(!truncate_strings(&mut v, 5));
    }

    #[test]
    fn array_mut_addresses_both_shapes() {
        let mut keyed = serde_json::json!({"hits": [1, 2]});
        assert_eq!(
            array_mut(&mut keyed, Some("hits")).map(|l| l.len()),
            Some(2)
        );
        let mut bare = serde_json::json!([1, 2, 3]);
        assert_eq!(array_mut(&mut bare, None).map(|l| l.len()), Some(3));
        assert!(array_mut(&mut keyed, Some("absent")).is_none());
    }
}
