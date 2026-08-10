//! `--suggest` ranking helpers for get-item recovery (in-memory only).
//!
//! One-shot: operates on an already-fetched all.html catalog — no extra HTTP.
//! Memory: single scored vec; bounded edit distance (max 2) avoids O(n·m) blowups
//! on adversarial leaf lengths (item paths already length-capped at the boundary).

use std::collections::HashSet;

use crate::docs_rs::SearchInCrateHit;
use crate::domain::MatchMode;
use crate::error::Suggestion;
use crate::item_kind::ItemKind;

/// Rank band for a candidate whose leaf starts with the needle.
///
/// The three bands below encode the ordering `exact < prefix < substring <
/// edit distance`, and both ranking functions read them from here. As bare
/// literals repeated across the two, changing one band in one place would have
/// silently reordered suggestions in the other.
const RANK_PREFIX: u8 = 10;
/// Rank band for a candidate that merely contains the needle.
const RANK_SUBSTRING: u8 = 20;
/// Base rank for edit-distance matches; the distance itself is added on top.
///
/// Sits above [`RANK_SUBSTRING`] by more than the maximum distance
/// ([`MAX_EDIT_DISTANCE`]) so no fuzzy hit can ever outrank a literal one.
const RANK_EDIT_BASE: u8 = 30;
/// Maximum edit distance accepted as a suggestion.
const MAX_EDIT_DISTANCE: u8 = 2;

/// Rank nearby **member leaf** names (from parent-page anchors) for `--suggest`.
///
/// `names` are bare member identifiers (`new`, `Item`, `MAX`, …) already scraped
/// from the anchor family the caller asked for. Each result pairs
/// `{parent}::{member}` with `kind_label`, the CLI kind the user would retype —
/// a suggestion labelled `(method)` for an associated type would send them
/// straight back into the same not-found.
pub(crate) fn rank_member_names(
    needle: &str,
    parent_path: &str,
    kind_label: &str,
    names: &[String],
    limit: usize,
) -> Vec<Suggestion> {
    let needle = needle.trim();
    if needle.is_empty() || limit == 0 {
        return Vec::new();
    }
    // Lowercase the needle once: it is loop-invariant (was re-allocated per candidate).
    let needle_lower = needle.to_ascii_lowercase();
    let mut scored: Vec<(u8, &str)> = Vec::with_capacity(names.len());
    for name in names {
        let leaf = name.as_str();
        if leaf.eq_ignore_ascii_case(needle) {
            scored.push((0, leaf));
            continue;
        }
        // One lowercase per candidate feeds both prefix and substring checks.
        let leaf_lower = leaf.to_ascii_lowercase();
        if leaf_lower.starts_with(&needle_lower) {
            scored.push((RANK_PREFIX, leaf));
            continue;
        }
        if leaf_lower.contains(&needle_lower) {
            scored.push((RANK_SUBSTRING, leaf));
            continue;
        }
        if let Some(d) = edit_distance_u8(needle, leaf, MAX_EDIT_DISTANCE) {
            scored.push((RANK_EDIT_BASE + d, leaf));
        }
    }
    // `sort_by` on `&str` keys: `sort_by_key` would allocate a `String` per comparison.
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(b.1)));
    let mut out = Vec::with_capacity(limit.min(scored.len()));
    let mut seen: HashSet<&str> = HashSet::with_capacity(scored.len());
    for (_, leaf) in scored {
        if seen.insert(leaf) {
            out.push(Suggestion::new(
                format!("{parent_path}::{leaf}"),
                kind_label,
            ));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// Rank nearby symbols for `--suggest` (exact → prefix → substring → edit distance).
///
/// Single in-memory pass over an all.html catalog (one prior HTTP fetch).
pub(crate) fn rank_suggestions(
    leaf: &str,
    hits: &[SearchInCrateHit],
    limit: usize,
) -> Vec<Suggestion> {
    let needle = leaf.trim();
    if needle.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(u8, &SearchInCrateHit)> = Vec::with_capacity(hits.len());
    for h in hits {
        if let Some(s) = MatchMode::Exact.score(&h.name, needle) {
            scored.push((s, h));
            continue;
        }
        if let Some(s) = MatchMode::Prefix.score(&h.name, needle) {
            scored.push((s.saturating_add(RANK_PREFIX), h));
            continue;
        }
        if let Some(s) = MatchMode::Substring.score(&h.name, needle) {
            scored.push((s.saturating_add(RANK_SUBSTRING), h));
            continue;
        }
        let leaf_hit = h.name.rsplit("::").next().unwrap_or(h.name.as_str());
        if let Some(d) = edit_distance_u8(needle, leaf_hit, MAX_EDIT_DISTANCE) {
            scored.push((RANK_EDIT_BASE + d, h));
        }
    }
    // `sort_by` borrows the name: `sort_by_key` cloned a `String` on every comparison.
    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.name.cmp(&b.1.name)));
    let mut out = Vec::with_capacity(limit.min(scored.len()));
    let mut seen: HashSet<&str> = HashSet::with_capacity(scored.len());
    for (_, h) in scored {
        // Build only after dedup wins: skipped duplicates no longer allocate.
        if seen.insert(h.name.as_str()) {
            out.push(Suggestion::new(h.name.as_str(), h.kind.as_str()));
            if out.len() >= limit {
                break;
            }
        }
    }
    out
}

/// Bounded Levenshtein distance; returns `None` when distance would exceed `max`.
pub(crate) fn edit_distance_u8(a: &str, b: &str, max: u8) -> Option<u8> {
    let a = a.to_ascii_lowercase();
    let b = b.to_ascii_lowercase();
    let (a, b) = (a.as_bytes(), b.as_bytes());
    let (n, m) = (a.len(), b.len());
    if n.abs_diff(m) > max as usize {
        return None;
    }
    let max_u = max as usize;
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        let mut row_min = cur[0];
        for j in 1..=m {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
            row_min = row_min.min(cur[j]);
        }
        if row_min > max_u {
            return None;
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    let d = prev[m];
    if d <= max_u { Some(d as u8) } else { None }
}

/// Pick a unique reexport path when the short path 404s (kind-aware).
pub(crate) fn pick_unique_reexport_path(
    leaf: &str,
    kind: ItemKind,
    hits: &[SearchInCrateHit],
) -> Option<String> {
    let needle = leaf.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    let kind_s = kind.as_str();
    let mut exact: Vec<&SearchInCrateHit> = hits
        .iter()
        .filter(|h| {
            h.kind == kind_s && crate::docs_rs::hits::leaf_eq_ignore_ascii(&h.name, &needle)
        })
        .collect();
    if exact.is_empty() {
        // Fall back to kind-matched hits whose full name ends with ::leaf or equals leaf.
        exact = hits
            .iter()
            .filter(|h| {
                h.kind == kind_s
                    && (h.name.eq_ignore_ascii_case(&needle)
                        || h.name
                            .to_ascii_lowercase()
                            .ends_with(&format!("::{needle}")))
            })
            .collect();
    }
    if exact.len() == 1 {
        return Some(exact[0].name.clone());
    }
    crate::docs_rs::hits::unique_best_score_hit(hits.iter().filter(|h| {
        h.kind == kind_s && crate::docs_rs::hits::leaf_eq_ignore_ascii(&h.name, &needle)
    }))
    .map(|h| h.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_distance_bounds() {
        assert_eq!(edit_distance_u8("abc", "abc", 0), Some(0));
        assert_eq!(edit_distance_u8("abc", "abx", 1), Some(1));
        assert_eq!(edit_distance_u8("abc", "xyz", 1), None);
    }

    #[test]
    fn rank_empty_leaf() {
        assert!(rank_suggestions("", &[], 5).is_empty());
        assert!(rank_suggestions("x", &[], 0).is_empty());
    }

    #[test]
    fn edit_distance_is_case_insensitive() {
        // Byte-wise case folding must match the previous `to_ascii_lowercase` pass.
        assert_eq!(edit_distance_u8("ABC", "abc", 0), Some(0));
        assert_eq!(edit_distance_u8("Spawn", "sPaWn", 1), Some(0));
        assert_eq!(edit_distance_u8("New", "neq", 1), Some(1));
        // Non-ASCII bytes are left untouched by both spellings.
        assert_eq!(edit_distance_u8("café", "café", 0), Some(0));
    }

    #[test]
    fn method_ranking_orders_exact_then_prefix_then_substring() {
        let names = vec![
            "try_new".to_string(),
            "new".to_string(),
            "new_current_thread".to_string(),
            "renew".to_string(),
        ];
        let out = rank_member_names("new", "runtime::Runtime", "method", &names, 10);
        // Assert on `path`, not on a rendered label: the ordering is what this
        // test owns, and the sentence belongs to the two renderers.
        assert_eq!(
            out.iter().map(|s| s.path.as_str()).collect::<Vec<_>>(),
            vec![
                "runtime::Runtime::new",
                "runtime::Runtime::new_current_thread",
                "runtime::Runtime::renew",
                "runtime::Runtime::try_new",
            ]
        );
        assert!(out.iter().all(|s| s.kind == "method"), "{out:?}");
    }

    #[test]
    fn method_ranking_dedupes_and_honours_limit() {
        let names = vec!["new".to_string(), "new".to_string(), "next".to_string()];
        let out = rank_member_names("new", "P", "method", &names, 1);
        assert_eq!(out, vec![Suggestion::new("P::new", "method")]);
    }

    fn hit(name: &str, kind: &str) -> SearchInCrateHit {
        SearchInCrateHit {
            name: name.to_string(),
            kind: kind.to_string(),
            url: format!("https://docs.rs/{name}"),
            score: None,
        }
    }

    #[test]
    fn suggestion_ranking_is_stable_and_labelled() {
        let hits = vec![
            hit("tokio::runtime::Builder", "struct"),
            hit("Runtime", "struct"),
            hit("runtime", "mod"),
            hit("Runtime", "struct"),
        ];
        let out = rank_suggestions("Runtime", &hits, 10);
        // Exact/prefix winners first; duplicate names collapse to one entry.
        assert_eq!(out.first(), Some(&Suggestion::new("Runtime", "struct")));
        assert_eq!(out.iter().filter(|s| s.path == "Runtime").count(), 1);
    }

    #[test]
    fn suggestion_ranking_reaches_the_edit_distance_arm() {
        // BUG-03: this arm was unreachable end-to-end because the caller pre-filtered
        // the catalog with MatchMode::Prefix, so a typo left zero hits to rank. The
        // ranker itself is fine; the guard here locks the contract it must honour.
        let hits = vec![
            hit("tokio::runtime::Runtime", "struct"),
            hit("tokio::runtime::Builder", "struct"),
            hit("tokio::task::spawn", "fn"),
        ];
        // `Runtimee` matches neither exact, nor prefix, nor substring.
        let out = rank_suggestions("Runtimee", &hits, 5);
        assert_eq!(
            out.first(),
            Some(&Suggestion::new("tokio::runtime::Runtime", "struct")),
            "distance-1 leaf must win the ranking: {out:?}"
        );
        // A leaf beyond the bounded distance stays out of the list entirely.
        assert!(rank_suggestions("zzzzzzzz", &hits, 5).is_empty());
    }

    /// Measurement harness for the ranking hot loop (not a correctness gate).
    ///
    /// Run with `cargo test --release -- --ignored --nocapture suggest_ranking_timing`.
    #[test]
    #[ignore = "measurement only; prints timings instead of asserting"]
    fn suggest_ranking_timing() {
        use std::time::Instant;

        for count in [64usize, 1_000, 10_000, 50_000] {
            let names: Vec<String> = (0..count).map(|i| format!("method_name_{i}")).collect();
            let started = Instant::now();
            let out = rank_member_names("method_name_42", "P", "method", &names, 20);
            let elapsed = started.elapsed();
            println!(
                "rank_member_names candidates={count} emitted={} elapsed={elapsed:?}",
                out.len()
            );
        }
    }
}
