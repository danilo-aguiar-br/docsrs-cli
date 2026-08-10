//! crates.io JSON body parsing and hostile-field capping.

use serde::Deserialize;

use super::types::{CrateSearchHit, SearchCratesData, SearchMeta};
use super::urls::clamp_search_pagination;
use crate::config::{
    MAX_CRATE_DESCRIPTION_CHARS, MAX_CRATE_NAME_CHARS, MAX_URL_FIELD_CHARS, MAX_VERSION_CHARS,
};
use crate::error::{AppError, AppResult, ErrorDetail};

/// Truncate `s` to at most `max_chars` Unicode scalars (char-boundary safe).
///
/// Used when mapping untrusted crates.io JSON fields into agent payloads so a
/// single hostile string cannot force multi-MB allocations before output budget.
pub(super) fn truncate_chars(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }
    s.chars().take(max_chars).collect()
}

/// Cap an optional string field; empty after trim becomes `None`.
fn cap_optional_field(value: Option<String>, max_chars: usize) -> Option<String> {
    value.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(truncate_chars(t, max_chars))
        }
    })
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    crates: Vec<ApiCrate>,
    meta: ApiMeta,
}

#[derive(Debug, Deserialize)]
struct ApiCrate {
    name: String,
    description: Option<String>,
    downloads: u64,
    newest_version: Option<String>,
    max_version: Option<String>,
    max_stable_version: Option<String>,
    default_version: Option<String>,
    documentation: Option<String>,
    recent_downloads: Option<u64>,
    exact_match: Option<bool>,
    yanked: Option<bool>,
    repository: Option<String>,
    homepage: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ApiMeta {
    total: u64,
    next_page: Option<String>,
    prev_page: Option<String>,
}

/// Parse crates.io JSON body into product types (offline-testable).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Parse`] when `text` is not valid crates.io search JSON.
pub fn parse_search_body(
    text: &str,
    query: &str,
    page: u32,
    per_page: u32,
    sort: &str,
    cache_hit: bool,
) -> AppResult<SearchCratesData> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let parsed: ApiResponse = serde_json::from_str(text)
        .map_err(|e| AppError::of_with_source(ErrorDetail::CratesIoJson, e))?;

    let hits = parsed
        .crates
        .into_iter()
        .map(|c| {
            let version_raw = c
                .newest_version
                .or_else(|| c.max_version.clone())
                .unwrap_or_else(|| "unknown".to_string());
            CrateSearchHit {
                // Cap name length (hostile API); charset is not re-validated here —
                // crates.io is the source of truth for published names.
                name: truncate_chars(&c.name, MAX_CRATE_NAME_CHARS),
                description: c
                    .description
                    .filter(|s| !s.is_empty())
                    .map(|s| truncate_chars(&s, MAX_CRATE_DESCRIPTION_CHARS))
                    .unwrap_or_else(|| "No description available".to_string()),
                downloads: c.downloads,
                version: truncate_chars(&version_raw, MAX_VERSION_CHARS),
                documentation: cap_optional_field(c.documentation, MAX_URL_FIELD_CHARS),
                max_version: cap_optional_field(c.max_version, MAX_VERSION_CHARS),
                max_stable_version: cap_optional_field(c.max_stable_version, MAX_VERSION_CHARS),
                default_version: cap_optional_field(c.default_version, MAX_VERSION_CHARS),
                recent_downloads: c.recent_downloads,
                exact_match: c.exact_match,
                yanked: c.yanked,
                repository: cap_optional_field(c.repository, MAX_URL_FIELD_CHARS),
                homepage: cap_optional_field(c.homepage, MAX_URL_FIELD_CHARS),
            }
        })
        .collect();

    Ok(SearchCratesData {
        query: query.to_string(),
        page,
        per_page,
        sort: sort.to_string(),
        hits,
        meta: SearchMeta {
            total: parsed.meta.total,
            next_page: cap_optional_field(parsed.meta.next_page, MAX_URL_FIELD_CHARS),
            prev_page: cap_optional_field(parsed.meta.prev_page, MAX_URL_FIELD_CHARS),
        },
        cache_hit,
        truncated: false,
    })
}
