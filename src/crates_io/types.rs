//! Agent-facing wire types for `search-crates` and request-echo derivation.

use serde::{Deserialize, Serialize};
use url::Url;

use super::urls::clamp_search_pagination;
use crate::cli::SortKind;
use crate::config::MAX_PER_PAGE;
use crate::domain::SearchQuery;

/// One hit from crates.io search.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrateSearchHit {
    /// Crate name on crates.io.
    pub name: String,
    /// Short crate description (may be empty).
    pub description: String,
    /// Total download count.
    pub downloads: u64,
    /// Newest or default version string for display.
    pub version: String,
    /// Documentation URL when provided by crates.io.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<String>,
    /// Highest published version when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_version: Option<String>,
    /// Highest stable version when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_stable_version: Option<String>,
    /// Default version selected by crates.io.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_version: Option<String>,
    /// Recent download count when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_downloads: Option<u64>,
    /// Whether the query matched the crate name exactly.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exact_match: Option<bool>,
    /// Whether the default version is yanked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yanked: Option<bool>,
    /// Source repository URL when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
    /// Project homepage URL when present.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// Pagination meta from crates.io (may contain `page=` or `seek=` tokens).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchMeta {
    /// Total hits reported by crates.io.
    pub total: u64,
    /// Opaque next-page token when more results exist.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Opaque previous-page token when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_page: Option<String>,
}

/// Full search-crates data payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchCratesData {
    /// Original search query string.
    pub query: String,
    /// 1-based page number used for the request.
    pub page: u32,
    /// Page size used for the request.
    pub per_page: u32,
    /// Sort order API token.
    pub sort: String,
    /// Search hits on this page.
    pub hits: Vec<CrateSearchHit>,
    /// Pagination metadata from crates.io.
    pub meta: SearchMeta,
    /// True when the HTTP body was served from the local disk cache.
    pub cache_hit: bool,
    /// True when hits were reduced to fit `--max-output-bytes` (JSON path).
    pub truncated: bool,
}

/// Echo fields for `search-crates` taken from the **effective request URL**
/// (single source of truth after URL planning, including `--page-token`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchEcho {
    /// Query string (`q=`).
    pub query: String,
    /// 1-based page.
    pub page: u32,
    /// Page size.
    pub per_page: u32,
    /// Sort token.
    pub sort: String,
}

impl SearchEcho {
    /// Build from CLI-side fallbacks before URL planning.
    pub fn from_cli(query: &SearchQuery, page: u32, per_page: u32, sort: SortKind) -> Self {
        let (per_page, page) = clamp_search_pagination(per_page, page);
        Self {
            query: query.as_str().to_string(),
            page,
            per_page,
            sort: sort.as_api_str().to_string(),
        }
    }
}

/// Derive echo params from the planned URL query string.
///
/// Missing pairs fall back to `fallback` so pure `seek=` tokens and partial
/// tokens still produce a coherent agent-facing payload.
pub fn echo_params_from_url(url: &Url, fallback: &SearchEcho) -> SearchEcho {
    let mut query = fallback.query.clone();
    let mut page = fallback.page;
    let mut per_page = fallback.per_page;
    let mut sort = fallback.sort.clone();
    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "q" => query = v.into_owned(),
            "page" => {
                if let Ok(n) = v.parse::<u32>() {
                    page = n.max(1);
                }
            }
            "per_page" => {
                if let Ok(n) = v.parse::<u32>() {
                    per_page = n.clamp(1, MAX_PER_PAGE);
                }
            }
            "sort" => sort = v.into_owned(),
            _ => {}
        }
    }
    let (per_page, page) = clamp_search_pagination(per_page, page);
    SearchEcho {
        query,
        page,
        per_page,
        sort,
    }
}
