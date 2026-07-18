//! crates.io search-crates operation.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::{HOST_CRATES_IO, MAX_PER_PAGE};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::{HttpClient, content_type_looks_json, decode_utf8};

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

/// Clamp crates.io pagination to product bounds (`per_page` 1..=100, `page` >= 1).
///
/// Dry-run `planned_params` and the request URL MUST use these same values so
/// agents never see `page=0` in params while the URL has `page=1`.
pub fn clamp_search_pagination(per_page: u32, page: u32) -> (u32, u32) {
    (per_page.clamp(1, MAX_PER_PAGE), page.max(1))
}

/// Build the planned crates.io search URL.
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the base URL cannot be parsed.
pub fn planned_url(query: &str, per_page: u32, sort: &str, page: u32) -> AppResult<Url> {
    planned_url_on_host(HOST_CRATES_IO, query, per_page, sort, page)
}

/// Build search URL against an arbitrary host (used by tests / local mocks).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when `host` does not form a valid base URL.
pub fn planned_url_on_host(
    host: &str,
    query: &str,
    per_page: u32,
    sort: &str,
    page: u32,
) -> AppResult<Url> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let scheme = if host.starts_with("http://") || host.starts_with("https://") {
        ""
    } else {
        "https://"
    };
    let base = if scheme.is_empty() {
        format!("{host}/api/v1/crates")
    } else {
        format!("{scheme}{host}/api/v1/crates")
    };
    let mut url = Url::parse(&base)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid crates.io base URL", e))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("q", query);
        q.append_pair("per_page", &per_page.to_string());
        q.append_pair("sort", sort);
        q.append_pair("page", &page.to_string());
    }
    Ok(url)
}

/// Parse crates.io JSON body into product types (offline-testable).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when `text` is not valid crates.io search JSON.
pub fn parse_search_body(
    text: &str,
    query: &str,
    page: u32,
    per_page: u32,
    sort: &str,
) -> AppResult<SearchCratesData> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let parsed: ApiResponse = serde_json::from_str(text).map_err(|e| {
        AppError::with_source(ErrorKind::Parse, "failed to parse crates.io JSON", e)
    })?;

    let hits = parsed
        .crates
        .into_iter()
        .map(|c| CrateSearchHit {
            name: c.name,
            description: c
                .description
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "No description available".to_string()),
            downloads: c.downloads,
            // `or_else` avoids eager clone of `max_version` when `newest_version` is Some.
            version: c
                .newest_version
                .or_else(|| c.max_version.clone())
                .unwrap_or_else(|| "unknown".to_string()),
            documentation: c.documentation,
            max_version: c.max_version,
            max_stable_version: c.max_stable_version,
            default_version: c.default_version,
            recent_downloads: c.recent_downloads,
            exact_match: c.exact_match,
            yanked: c.yanked,
            repository: c.repository,
            homepage: c.homepage,
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
            next_page: parsed.meta.next_page,
            prev_page: parsed.meta.prev_page,
        },
    })
}

/// Execute search-crates against crates.io (production origin).
///
/// # Errors
///
/// Propagates URL build, HTTP, UTF-8 decode, and JSON parse failures from
/// [`search_crates_on_origin`].
pub async fn search_crates(
    http: &HttpClient,
    query: &str,
    per_page: u32,
    sort: &str,
    page: u32,
) -> AppResult<SearchCratesData> {
    search_crates_on_origin(
        http,
        &format!("https://{HOST_CRATES_IO}"),
        query,
        per_page,
        sort,
        page,
    )
    .await
}

/// Execute search-crates against a configurable origin (offline mocks).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the origin URL is invalid.
/// Propagates HTTP and parse errors from [`search_crates_at`].
pub async fn search_crates_on_origin(
    http: &HttpClient,
    origin: &str,
    query: &str,
    per_page: u32,
    sort: &str,
    page: u32,
) -> AppResult<SearchCratesData> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let url = planned_url_on_host(origin, query, per_page, sort, page)?;
    search_crates_at(http, &url, query, per_page, sort, page).await
}

/// Execute search against a pre-built URL (mock hosts in tests).
///
/// # Errors
///
/// Propagates HTTP transport errors from [`HttpClient::get_json`].
/// Maps non-success HTTP statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] when the body is not UTF-8 or not valid crates.io JSON.
pub async fn search_crates_at(
    http: &HttpClient,
    url: &Url,
    query: &str,
    per_page: u32,
    sort: &str,
    page: u32,
) -> AppResult<SearchCratesData> {
    let resp = http.get_json(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(404, "crates.io search"));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "crates.io search",
        ));
    }
    if resp.content_type.is_some() && !content_type_looks_json(resp.content_type.as_deref()) {
        tracing::warn!(
            content_type = ?resp.content_type,
            "unexpected Content-Type from crates.io"
        );
    }

    let text = decode_utf8(&resp.body)?;
    parse_search_body(&text, query, page, per_page, sort)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_url_clamps_and_encodes() {
        let u = planned_url("serde json", 0, "alphabetical", 0).unwrap();
        assert!(u.as_str().contains("per_page=1"));
        assert!(u.as_str().contains("page=1"));
        assert!(u.as_str().contains("sort=alphabetical"));
        assert!(u.as_str().contains("q=serde"));
        let u2 = planned_url("x", 500, "relevance", 3).unwrap();
        assert!(u2.as_str().contains("per_page=100"));
        assert!(u2.as_str().contains("page=3"));
    }

    #[test]
    fn clamp_search_pagination_page_and_per_page() {
        assert_eq!(clamp_search_pagination(0, 0), (1, 1));
        assert_eq!(clamp_search_pagination(10, 0), (10, 1));
        assert_eq!(clamp_search_pagination(500, 2), (100, 2));
        assert_eq!(clamp_search_pagination(25, 3), (25, 3));
    }

    #[test]
    fn parse_fixture_serde() {
        let body = include_str!("../tests/fixtures/crates_io/search_serde.json");
        let data = parse_search_body(body, "serde", 1, 10, "relevance").unwrap();
        assert_eq!(data.hits.len(), 2);
        assert_eq!(data.hits[0].name, "serde");
        assert_eq!(data.hits[0].exact_match, Some(true));
        assert_eq!(data.hits[0].version, "1.0.210");
        assert_eq!(data.hits[1].description, "No description available");
        assert_eq!(data.meta.total, 2);
        assert_eq!(data.meta.next_page.as_deref(), Some("page=2"));
    }

    #[test]
    fn parse_fixture_seek_meta() {
        let body = include_str!("../tests/fixtures/crates_io/search_seek_meta.json");
        let data = parse_search_body(body, "example", 1, 10, "downloads").unwrap();
        assert_eq!(
            data.meta.next_page.as_deref(),
            Some("seek=ABC123&per_page=10")
        );
        assert_eq!(data.meta.prev_page.as_deref(), Some("page=1"));
        assert_eq!(data.hits[0].version, "0.1.0");
    }

    #[test]
    fn parse_invalid_json() {
        let err = parse_search_body("{", "q", 1, 10, "relevance").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Parse);
    }

    #[test]
    fn parse_missing_versions_uses_unknown() {
        let body = r#"{"crates":[{"name":"x","downloads":1}],"meta":{"total":1}}"#;
        let data = parse_search_body(body, "x", 1, 10, "new").unwrap();
        assert_eq!(data.hits[0].version, "unknown");
        assert!(data.meta.next_page.is_none());
    }

    #[test]
    fn planned_url_on_host_http_base() {
        let u = planned_url_on_host("http://127.0.0.1:9", "q", 10, "new", 1).unwrap();
        assert!(u.as_str().starts_with("http://127.0.0.1:9/api/v1/crates"));
    }
}
