//! Path addressing and the `--filter` predicate grammar.
//!
//! Both `--filter` and `--dedupe-by` address fields by dotted path and compare
//! them as scalars, so resolution and comparison live together here, separate
//! from the pipeline that applies them.

use serde_json::Value;

use crate::error::{AppError, AppResult, ErrorDetail};

/// Comparison operator of a single `--filter` expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterOp {
    /// `key=value` / `key==value` — exact string equality.
    Eq,
    /// `key!=value` — string inequality (a missing key never matches).
    Ne,
    /// `key~value` — substring containment.
    Contains,
}

/// One parsed `--filter` expression.
#[derive(Debug, Clone)]
pub struct Filter {
    /// Dotted path resolved against each element.
    pub path: String,
    /// Comparison operator.
    pub op: FilterOp,
    /// Right-hand literal, compared against the scalar rendering of the field.
    pub value: String,
}

impl Filter {
    /// Parse one `key=value` / `key!=value` / `key==value` / `key~value` expression.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] (exit 65) when no operator is present or
    /// the key side is empty. A malformed expression is fail-closed on purpose: a
    /// silently empty result set hides a typo far longer than an error does.
    pub fn parse(expr: &str) -> AppResult<Self> {
        let bytes = expr.as_bytes();
        for (idx, _) in expr.char_indices() {
            let rest = &bytes[idx..];
            let (op, width) = if rest.starts_with(b"!=") {
                (FilterOp::Ne, 2)
            } else if rest.starts_with(b"==") {
                (FilterOp::Eq, 2)
            } else if rest.starts_with(b"=") {
                (FilterOp::Eq, 1)
            } else if rest.starts_with(b"~") {
                (FilterOp::Contains, 1)
            } else {
                continue;
            };
            let path = expr[..idx].trim();
            if path.is_empty() {
                return Err(invalid_filter(expr, "key side is empty"));
            }
            return Ok(Self {
                path: path.to_string(),
                op,
                value: expr[idx + width..].to_string(),
            });
        }
        Err(invalid_filter(
            expr,
            "expected key=value, key!=value or key~substring",
        ))
    }

    /// Whether `element` satisfies this predicate.
    pub(super) fn matches(&self, element: &Value, strip: &[&str]) -> bool {
        let Some(found) = resolve_path(element, &self.path, strip) else {
            // A missing field never matches — not even under `!=`.
            return false;
        };
        let Some(actual) = scalar_text(found) else {
            return false;
        };
        match self.op {
            FilterOp::Eq => actual == self.value,
            FilterOp::Ne => actual != self.value,
            FilterOp::Contains => actual.contains(&self.value),
        }
    }
}

fn invalid_filter(expr: &str, why: &str) -> AppError {
    AppError::of(ErrorDetail::InvalidFilterExpression {
        expr: expr.to_string(),
        reason: why.to_string(),
    })
}

/// Resolve a dotted path against `root`.
///
/// A leading `data.` segment, and a leading segment naming the located result array,
/// are tolerated so `data.hits.name`, `hits.name` and `name` all address the same
/// field of an element.
pub(super) fn resolve_path<'a>(root: &'a Value, path: &str, strip: &[&str]) -> Option<&'a Value> {
    let mut segments: Vec<&str> = path.split('.').filter(|s| !s.is_empty()).collect();
    if segments.first() == Some(&"data") && segments.len() > 1 {
        segments.remove(0);
    }
    if segments.len() > 1 && segments.first().is_some_and(|s| strip.contains(s)) {
        segments.remove(0);
    }
    let mut cur = root;
    for seg in segments {
        cur = cur.get(seg)?;
    }
    Some(cur)
}

/// Scalar rendering used by `--filter` and `--dedupe-by`. Containers and `null`
/// yield `None` and therefore never match.
pub(super) fn scalar_text(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;

    #[test]
    fn parses_every_operator_form() {
        let eq = Filter::parse("name=serde").expect("valid");
        assert_eq!(eq.op, FilterOp::Eq);
        assert_eq!(eq.path, "name");
        assert_eq!(eq.value, "serde");
        assert_eq!(
            Filter::parse("name==serde").expect("valid").op,
            FilterOp::Eq
        );
        assert_eq!(
            Filter::parse("name!=serde").expect("valid").op,
            FilterOp::Ne
        );
        assert_eq!(
            Filter::parse("name~serde").expect("valid").op,
            FilterOp::Contains
        );
    }

    #[test]
    fn double_equals_is_not_split_as_single_equals() {
        let f = Filter::parse("name==x").expect("valid");
        assert_eq!(f.value, "x", "`==` consumes both bytes");
    }

    #[test]
    fn malformed_expressions_are_invalid_input() {
        for bad in ["nope", "=x", "  =x"] {
            let e = Filter::parse(bad).expect_err("must reject");
            assert_eq!(e.kind(), ErrorKind::InvalidInput, "input {bad:?}");
            assert_eq!(e.kind().exit_code(), 65);
        }
    }

    #[test]
    fn empty_value_is_allowed() {
        let f = Filter::parse("name=").expect("valid");
        assert!(f.value.is_empty());
    }

    #[test]
    fn scalar_text_covers_scalars_only() {
        assert_eq!(scalar_text(&serde_json::json!("a")).as_deref(), Some("a"));
        assert_eq!(scalar_text(&serde_json::json!(7)).as_deref(), Some("7"));
        assert_eq!(
            scalar_text(&serde_json::json!(true)).as_deref(),
            Some("true")
        );
        assert!(scalar_text(&serde_json::json!(null)).is_none());
        assert!(scalar_text(&serde_json::json!([1])).is_none());
        assert!(scalar_text(&serde_json::json!({"a": 1})).is_none());
    }

    #[test]
    fn resolve_path_strips_data_and_array_prefixes() {
        let el = serde_json::json!({"name": "serde", "meta": {"total": 3}});
        for path in ["name", "hits.name", "data.hits.name"] {
            assert_eq!(
                resolve_path(&el, path, &["hits"])
                    .and_then(scalar_text)
                    .as_deref(),
                Some("serde"),
                "path {path}"
            );
        }
        assert_eq!(
            resolve_path(&el, "meta.total", &["hits"]).and_then(scalar_text),
            Some("3".to_string()),
            "nested paths still resolve"
        );
        assert!(resolve_path(&el, "absent", &[]).is_none());
    }

    #[test]
    fn missing_field_never_matches_any_operator() {
        let el = serde_json::json!({"version": "1"});
        for expr in ["name=x", "name!=x", "name~x"] {
            let f = Filter::parse(expr).expect("valid");
            assert!(!f.matches(&el, &[]), "expr {expr}");
        }
    }
}
