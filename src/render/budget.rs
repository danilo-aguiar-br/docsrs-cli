//! `max_output_bytes` enforcement: UTF-8-safe truncation and hit-list shrinking.
//!
//! Budgeting always cuts on a structural boundary — a whole hit, or a char
//! boundary inside markdown. Slicing raw JSON text would emit an unparseable
//! document, which is strictly worse for an agent than an over-budget one.

use super::envelope::success_envelope;
use crate::config::Config;
use crate::crates_io::SearchCratesData;
use crate::docs_rs::{GetItemData, ReadmeData, SearchInCrateData};

/// Truncate UTF-8 text on a char boundary; returns (text, truncated).
pub fn truncate_output(text: &str, max_bytes: u64) -> (String, bool) {
    if max_bytes == 0 {
        return (String::new(), true);
    }
    let max = max_bytes as usize;
    if text.len() <= max {
        return (text.to_string(), false);
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

/// Reduce `search-crates` hits until the success envelope fits `max_output_bytes`.
///
/// Preserves JSON parseability (never mid-string cuts). Sets `truncated=true` when
/// any hit is dropped for budget. If even zero hits exceeds the budget, returns
/// the zero-hit payload with `truncated=true` (agents should raise the cap).
pub fn apply_output_budget_search_crates(
    mut data: SearchCratesData,
    max_bytes: u64,
    duration_ms: u64,
    source_url: Option<&str>,
) -> SearchCratesData {
    if max_bytes == 0 {
        data.hits.clear();
        data.truncated = true;
        return data;
    }
    loop {
        let env = success_envelope("search-crates", &data, duration_ms, source_url);
        let len = match serde_json::to_vec(&env) {
            Ok(buf) => buf.len() as u64,
            Err(_) => return data,
        };
        if len <= max_bytes {
            return data;
        }
        if data.hits.is_empty() {
            data.truncated = true;
            return data;
        }
        data.hits.pop();
        data.truncated = true;
    }
}

/// Reduce `search-in-crate` hits until the success envelope fits `max_output_bytes`.
///
/// `truncated` becomes true when budget (or a prior limit) cuts the list.
pub fn apply_output_budget_search_in_crate(
    mut data: SearchInCrateData,
    max_bytes: u64,
    duration_ms: u64,
    source_url: Option<&str>,
) -> SearchInCrateData {
    if max_bytes == 0 {
        data.hits.clear();
        data.emitted = 0;
        data.truncated = true;
        return data;
    }
    loop {
        let env = success_envelope("search-in-crate", &data, duration_ms, source_url);
        let len = match serde_json::to_vec(&env) {
            Ok(buf) => buf.len() as u64,
            Err(_) => return data,
        };
        if len <= max_bytes {
            return data;
        }
        if data.hits.is_empty() {
            data.emitted = 0;
            data.truncated = true;
            return data;
        }
        data.hits.pop();
        data.emitted = data.hits.len();
        data.truncated = true;
    }
}

/// Apply max_output_bytes truncation to readme payload.
pub fn apply_truncation_to_readme(mut data: ReadmeData, cfg: &Config) -> ReadmeData {
    let (md, trunc) = truncate_output(&data.markdown, cfg.max_output_bytes);
    data.markdown = md;
    data.truncated = trunc;
    data
}

/// Apply max_output_bytes truncation to get-item payload.
pub fn apply_truncation_to_item(mut data: GetItemData, cfg: &Config) -> GetItemData {
    let (md, trunc) = truncate_output(&data.markdown, cfg.max_output_bytes);
    data.markdown = md;
    data.truncated = trunc;
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crates_io::{CrateSearchHit, SearchMeta};

    #[test]
    fn truncate_marks_flag() {
        let (out, trunc) = truncate_output("abcdef", 3);
        assert!(trunc);
        assert_eq!(out, "abc");
    }

    #[test]
    fn truncate_zero_and_utf8_boundary() {
        let (out, trunc) = truncate_output("abc", 0);
        assert!(trunc);
        assert!(out.is_empty());
        let s = "áéí"; // multi-byte
        let (out, trunc) = truncate_output(s, 2);
        assert!(trunc);
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn apply_truncation_sets_flags() {
        let cfg = Config {
            max_output_bytes: 4,
            ..Config::default()
        };
        let r = apply_truncation_to_readme(
            ReadmeData {
                crate_name: "c".into(),
                version: "1".into(),
                resolved_version: None,
                markdown: "hello world".into(),
                empty: false,
                truncated: false,
                source_url: "u".into(),
                cache_hit: false,
            },
            &cfg,
        );
        assert!(r.truncated);
        assert!(r.markdown.len() <= 4);
        let i = apply_truncation_to_item(
            GetItemData {
                crate_name: "c".into(),
                item_type: "fn".into(),
                item_path: "c::f".into(),
                item_name: "f".into(),
                version: "1".into(),
                resolved_version: None,
                markdown: "abcdef".into(),
                empty: false,
                truncated: false,
                source_url: "u".into(),
                title: "t".into(),
                cache_hit: false,
                extraction: None,
                anchor_family: None,
                resolved_item_path: None,
            },
            &cfg,
        );
        assert!(i.truncated);
        assert_eq!(i.markdown, "abcd");
    }

    #[test]
    fn output_budget_reduces_search_hits() {
        let mut hits = Vec::new();
        for i in 0..20 {
            hits.push(CrateSearchHit {
                name: format!("crate-{i}"),
                description: "A".repeat(80),
                downloads: i as u64,
                version: "1.0.0".into(),
                documentation: Some(format!("https://docs.rs/crate-{i}")),
                max_version: None,
                max_stable_version: None,
                default_version: None,
                recent_downloads: None,
                exact_match: None,
                yanked: None,
                repository: None,
                homepage: None,
            });
        }
        let data = SearchCratesData {
            query: "crate".into(),
            page: 1,
            per_page: 20,
            sort: "relevance".into(),
            hits,
            meta: SearchMeta {
                total: 20,
                next_page: None,
                prev_page: None,
            },
            cache_hit: false,
            truncated: false,
        };
        let budgeted = apply_output_budget_search_crates(data, 800, 1, Some("https://crates.io"));
        assert!(budgeted.truncated);
        assert!(budgeted.hits.len() < 20);
        let env = success_envelope("search-crates", &budgeted, 1, Some("https://crates.io"));
        let len = serde_json::to_vec(&env).expect("serialize").len() as u64;
        assert!(len <= 800, "envelope len {len} > 800");
    }
}
