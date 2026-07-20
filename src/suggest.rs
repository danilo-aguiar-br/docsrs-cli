//! `--suggest` ranking helpers for get-item recovery (in-memory only).
//!
//! One-shot: operates on an already-fetched all.html catalog — no extra HTTP.
//! Memory: single scored vec; bounded edit distance (max 2) avoids O(n·m) blowups
//! on adversarial leaf lengths (item paths already length-capped at the boundary).

use std::collections::HashSet;

use crate::docs_rs::SearchInCrateHit;
use crate::domain::MatchMode;
use crate::item_kind::ItemKind;

/// Rank nearby **method leaf** names (from parent-page anchors) for `--suggest`.
///
/// `names` are bare method identifiers (`new`, `spawn`, …) already scraped from
/// `id="method.X"`. Labels are formatted as `{parent}::{method} (method)`.
pub(crate) fn rank_method_names(
    needle: &str,
    parent_path: &str,
    names: &[String],
    limit: usize,
) -> Vec<String> {
    let needle = needle.trim();
    if needle.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(u8, &str)> = Vec::new();
    for name in names {
        let leaf = name.as_str();
        if leaf.eq_ignore_ascii_case(needle) {
            scored.push((0, leaf));
            continue;
        }
        if leaf.to_ascii_lowercase().starts_with(&needle.to_ascii_lowercase()) {
            scored.push((10, leaf));
            continue;
        }
        if leaf.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()) {
            scored.push((20, leaf));
            continue;
        }
        if let Some(d) = edit_distance_u8(needle, leaf, 2) {
            scored.push((30 + d, leaf));
        }
    }
    scored.sort_by_key(|(s, n)| (*s, (*n).to_string()));
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (_, leaf) in scored {
        if seen.insert(leaf.to_string()) {
            out.push(format!("{parent_path}::{leaf} (method)"));
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
pub(crate) fn rank_suggestions(leaf: &str, hits: &[SearchInCrateHit], limit: usize) -> Vec<String> {
    let needle = leaf.trim();
    if needle.is_empty() || limit == 0 {
        return Vec::new();
    }
    let mut scored: Vec<(u8, &SearchInCrateHit)> = Vec::new();
    for h in hits {
        if let Some(s) = MatchMode::Exact.score(&h.name, needle) {
            scored.push((s, h));
            continue;
        }
        if let Some(s) = MatchMode::Prefix.score(&h.name, needle) {
            scored.push((s.saturating_add(10), h));
            continue;
        }
        if let Some(s) = MatchMode::Substring.score(&h.name, needle) {
            scored.push((s.saturating_add(20), h));
            continue;
        }
        let leaf_hit = h.name.rsplit("::").next().unwrap_or(h.name.as_str());
        if let Some(d) = edit_distance_u8(needle, leaf_hit, 2) {
            scored.push((30 + d, h));
        }
    }
    scored.sort_by_key(|(s, h)| (*s, h.name.clone()));
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (_, h) in scored {
        let label = format!("{} ({})", h.name, h.kind);
        if seen.insert(h.name.clone()) {
            out.push(label);
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
    if d <= max_u {
        Some(d as u8)
    } else {
        None
    }
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
        .filter(|h| h.kind == kind_s && crate::docs_rs::hits::leaf_eq_ignore_ascii(&h.name, &needle))
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
    crate::docs_rs::hits::unique_best_score_hit(
        hits.iter().filter(|h| {
            h.kind == kind_s && crate::docs_rs::hits::leaf_eq_ignore_ascii(&h.name, &needle)
        }),
    )
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
}
