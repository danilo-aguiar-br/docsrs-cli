//! Leaf-hit disambiguation helpers (DRY — shared by URL recovery and `--suggest`).
//!
//! One-shot / memory: pure in-memory scans over an already-fetched catalog;
//! no HTTP, no locks.

use super::types::SearchInCrateHit;

/// Last `::` segment of a rustdoc path, lowercased for ASCII compare.
#[inline]
pub(crate) fn leaf_name(full_name: &str) -> &str {
    full_name.rsplit("::").next().unwrap_or(full_name)
}

/// True when the path leaf equals `needle_lower` (ASCII case-insensitive).
#[inline]
pub(crate) fn leaf_eq_ignore_ascii(full_name: &str, needle_lower: &str) -> bool {
    leaf_name(full_name).eq_ignore_ascii_case(needle_lower)
}

/// Among `hits`, pick the unique best (lowest) score; `None` if empty or tied.
pub(crate) fn unique_best_score_hit<'a>(
    hits: impl Iterator<Item = &'a SearchInCrateHit>,
) -> Option<&'a SearchInCrateHit> {
    let mut best: Option<(u8, &SearchInCrateHit)> = None;
    let mut best_count = 0u32;
    for h in hits {
        let score = h.score.unwrap_or(0);
        match best {
            None => {
                best = Some((score, h));
                best_count = 1;
            }
            Some((s, _)) if score < s => {
                best = Some((score, h));
                best_count = 1;
            }
            Some((s, _)) if score == s => best_count += 1,
            _ => {}
        }
    }
    if best_count == 1 {
        best.map(|(_, h)| h)
    } else {
        None
    }
}
