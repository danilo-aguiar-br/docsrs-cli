//! crates.io search-crates operation.

use serde::{Deserialize, Serialize};
use url::Url;

use crate::cli::SortKind;
use crate::config::{
    MAX_CRATE_DESCRIPTION_CHARS, MAX_CRATE_NAME_CHARS, MAX_PER_PAGE, MAX_URL_FIELD_CHARS,
    MAX_VERSION_CHARS,
};
use crate::domain::{AllowedOrigin, SearchQuery};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::{HttpClient, decode_utf8, require_content_type_json};

/// Truncate `s` to at most `max_chars` Unicode scalars (char-boundary safe).
///
/// Used when mapping untrusted crates.io JSON fields into agent payloads so a
/// single hostile string cannot force multi-MB allocations before output budget.
fn truncate_chars(s: &str, max_chars: usize) -> String {
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
/// Used for defensive URL echo / seek tokens. Explicit CLI flags must go through
/// [`validate_search_pagination`] so invalid agent input fails closed (exit 65).
pub fn clamp_search_pagination(per_page: u32, page: u32) -> (u32, u32) {
    (per_page.clamp(1, MAX_PER_PAGE), page.max(1))
}

/// Validate explicit CLI pagination flags (fail-closed).
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when `page < 1` or `per_page` is outside
/// `1..=MAX_PER_PAGE` (100).
pub fn validate_search_pagination(per_page: u32, page: u32) -> AppResult<(u32, u32)> {
    if page < 1 {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "page must be >= 1 (got 0 or missing)",
        ));
    }
    if !(1..=MAX_PER_PAGE).contains(&per_page) {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("per_page must be 1..={MAX_PER_PAGE} (got {per_page})"),
        ));
    }
    Ok((per_page, page))
}

/// Absolute crates.io API base (`…/api/v1/crates`) for an allowlisted origin.
fn crates_api_base(origin: &AllowedOrigin) -> String {
    format!("{}/api/v1/crates", origin.as_str().trim_end_matches('/'))
}

/// Build the planned crates.io search URL (production origin).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the base URL cannot be parsed.
pub fn planned_url(query: &SearchQuery, per_page: u32, sort: SortKind, page: u32) -> AppResult<Url> {
    planned_url_on_host(&AllowedOrigin::crates_io_default(), query, per_page, sort, page)
}

/// Build the crates.io search URL for an allowlisted origin (`page` pagination).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the composed base URL is invalid.
pub fn planned_url_on_host(
    origin: &AllowedOrigin,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<Url> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let base = crates_api_base(origin);
    let mut url = Url::parse(&base)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid crates.io base URL", e))?;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("q", query.as_str());
        q.append_pair("per_page", &per_page.to_string());
        q.append_pair("sort", sort.as_api_str());
        q.append_pair("page", &page.to_string());
    }
    Ok(url)
}

/// Build search URL using an opaque `next_page` / `prev_page` token from crates.io meta.
///
/// Accepts:
/// - full query string (`?q=…&page=2` or `q=…&page=2`)
/// - pure page number string
/// - `seek=…` tokens
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the base URL is invalid.
/// Returns [`ErrorKind::InvalidInput`] when the token cannot form a valid URL.
pub fn planned_url_with_page_token(
    origin: &AllowedOrigin,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page_token: &str,
) -> AppResult<Url> {
    let token = page_token.trim();
    if token.is_empty() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "page-token is empty",
        ));
    }
    // Reject obvious garbage that is neither a page number, query string, nor seek token
    // (GAP-W-007 — fail local before remote "bad request" wording confuses agents).
    if token.chars().any(|c| c.is_control())
        || token.len() > crate::config::MAX_URL_FIELD_CHARS
    {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "page-token is invalid (control characters or exceeds max length)",
        ));
    }
    // Pure numeric → page number
    if let Ok(page) = token.parse::<u32>() {
        return planned_url_on_host(origin, query, per_page, sort, page);
    }
    let base = crates_api_base(origin);
    let base_url = Url::parse(&base)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid crates.io base URL", e))?;

    let qs = token.trim_start_matches('?');
    // If token already carries a full query, use it as the request query.
    if qs.contains('=') {
        let mut url = base_url;
        url.set_query(Some(qs));
        return Ok(url);
    }
    // Opaque seek-like token
    let (per_page, _) = clamp_search_pagination(per_page, 1);
    let mut url = base_url;
    {
        let mut q = url.query_pairs_mut();
        q.append_pair("q", query.as_str());
        q.append_pair("per_page", &per_page.to_string());
        q.append_pair("sort", sort.as_api_str());
        q.append_pair("seek", qs);
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
    cache_hit: bool,
) -> AppResult<SearchCratesData> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let parsed: ApiResponse = serde_json::from_str(text).map_err(|e| {
        AppError::with_source(ErrorKind::Parse, "failed to parse crates.io JSON", e)
    })?;

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

/// Execute search-crates against crates.io (production origin).
///
/// # Errors
///
/// Propagates URL build, HTTP, UTF-8 decode, and JSON parse failures from
/// [`search_crates_on_origin`].
pub async fn search_crates(
    http: &HttpClient,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<SearchCratesData> {
    search_crates_on_origin(
        http,
        &AllowedOrigin::crates_io_default(),
        query,
        per_page,
        sort,
        page,
    )
    .await
}

/// Execute search-crates against an allowlisted origin (offline mocks).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the origin URL is invalid.
/// Propagates HTTP and parse errors from [`search_crates_at`].
pub async fn search_crates_on_origin(
    http: &HttpClient,
    origin: &AllowedOrigin,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<SearchCratesData> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let url = planned_url_on_host(origin, query, per_page, sort, page)?;
    search_crates_at(http, &url, query, per_page, sort, page).await
}

/// Execute search against a pre-built URL (mock hosts in tests).
///
/// Echo fields (`query`/`page`/`per_page`/`sort`) are taken from the **URL**
/// via [`echo_params_from_url`], with CLI values as fallback for missing pairs.
///
/// # Errors
///
/// Propagates HTTP transport errors from [`HttpClient::get_json`].
/// Maps non-success HTTP statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] when the body is not UTF-8 or not valid crates.io JSON.
pub async fn search_crates_at(
    http: &HttpClient,
    url: &Url,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<SearchCratesData> {
    let fallback = SearchEcho::from_cli(query, page, per_page, sort);
    let echo = echo_params_from_url(url, &fallback);
    let resp = http.get_json(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(404, "crates.io search"));
    }
    if resp.status.as_u16() == 400 {
        // GAP-W-007: distinguish remote rejection of pagination/query from generic remote 400.
        return Err(AppError::from_http_status(
            400,
            "crates.io search (page-token or query rejected by remote)",
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "crates.io search",
        ));
    }
    // Fail closed when origin advertises a non-JSON type (MIME confusion).
    require_content_type_json(resp.content_type.as_deref())?;

    let text = decode_utf8(&resp.body)?;
    parse_search_body(
        &text,
        &echo.query,
        echo.page,
        echo.per_page,
        &echo.sort,
        resp.cache_hit,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::SortKind;
    use crate::domain::SearchQuery;

    fn q(s: &str) -> SearchQuery {
        SearchQuery::parse(s, true).unwrap()
    }

    #[test]
    fn planned_url_clamps_and_encodes() {
        let u = planned_url(&q("serde json"), 0, SortKind::Alphabetical, 0).unwrap();
        assert!(u.as_str().contains("per_page=1"));
        assert!(u.as_str().contains("page=1"));
        assert!(u.as_str().contains("sort=alphabetical"));
        assert!(u.as_str().contains("q=serde"));
        let u2 = planned_url(&q("x"), 500, SortKind::Relevance, 3).unwrap();
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
    fn validate_search_pagination_rejects_invalid() {
        assert!(validate_search_pagination(10, 0).is_err());
        assert!(validate_search_pagination(0, 1).is_err());
        assert!(validate_search_pagination(200, 1).is_err());
        assert_eq!(validate_search_pagination(25, 3).unwrap(), (25, 3));
        assert_eq!(validate_search_pagination(100, 1).unwrap(), (100, 1));
    }

    #[test]
    fn parse_fixture_serde() {
        let body = include_str!("../tests/fixtures/crates_io/search_serde.json");
        let data = parse_search_body(body, "serde", 1, 10, "relevance", false).unwrap();
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
        let data = parse_search_body(body, "example", 1, 10, "downloads", false).unwrap();
        assert_eq!(
            data.meta.next_page.as_deref(),
            Some("seek=ABC123&per_page=10")
        );
        assert_eq!(data.meta.prev_page.as_deref(), Some("page=1"));
        assert_eq!(data.hits[0].version, "0.1.0");
    }

    #[test]
    fn parse_invalid_json() {
        let err = parse_search_body("{", "q", 1, 10, "relevance", false).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Parse);
    }

    #[test]
    fn parse_missing_versions_uses_unknown() {
        let body = r#"{"crates":[{"name":"x","downloads":1}],"meta":{"total":1}}"#;
        let data = parse_search_body(body, "x", 1, 10, "new", false).unwrap();
        assert_eq!(data.hits[0].version, "unknown");
        assert!(data.meta.next_page.is_none());
    }

    #[test]
    fn planned_url_on_host_http_base() {
        let origin = AllowedOrigin::parse_with("http://127.0.0.1:9", true).unwrap();
        let u = planned_url_on_host(&origin, &q("q"), 10, SortKind::New, 1).unwrap();
        assert!(u.as_str().starts_with("http://127.0.0.1:9/api/v1/crates"));
    }

    #[test]
    fn echo_params_from_page_token_url() {
        let fallback = SearchEcho::from_cli(&q(""), 1, 10, SortKind::Relevance);
        let origin = AllowedOrigin::crates_io_default();
        let url = planned_url_with_page_token(
            &origin,
            &q(""),
            10,
            SortKind::Relevance,
            "?q=serde&per_page=2&sort=relevance&page=2",
        )
        .unwrap();
        let echo = echo_params_from_url(&url, &fallback);
        assert_eq!(echo.query, "serde");
        assert_eq!(echo.page, 2);
        assert_eq!(echo.per_page, 2);
        assert_eq!(echo.sort, "relevance");
    }

    #[test]
    fn echo_params_fallback_when_pairs_missing() {
        let fallback = SearchEcho::from_cli(&q("local"), 3, 25, SortKind::Downloads);
        let url = Url::parse("https://crates.io/api/v1/crates?seek=ABC").unwrap();
        let echo = echo_params_from_url(&url, &fallback);
        assert_eq!(echo.query, "local");
        assert_eq!(echo.page, 3);
        assert_eq!(echo.per_page, 25);
        assert_eq!(echo.sort, "downloads");
    }

    #[test]
    fn parse_caps_oversized_api_strings() {
        let long_name = "a".repeat(MAX_CRATE_NAME_CHARS + 50);
        let long_desc = "d".repeat(MAX_CRATE_DESCRIPTION_CHARS + 100);
        let long_url = format!("https://example.com/{}", "p".repeat(MAX_URL_FIELD_CHARS));
        let long_token = "t".repeat(MAX_URL_FIELD_CHARS + 10);
        let body = format!(
            r#"{{"crates":[{{"name":"{long_name}","description":"{long_desc}","downloads":1,"newest_version":"1.0.0","documentation":"{long_url}","repository":"{long_url}","homepage":"{long_url}"}}],"meta":{{"total":1,"next_page":"{long_token}","prev_page":"{long_token}"}}}}"#
        );
        let data = parse_search_body(&body, "q", 1, 10, "relevance", false).unwrap();
        assert_eq!(data.hits[0].name.chars().count(), MAX_CRATE_NAME_CHARS);
        assert_eq!(
            data.hits[0].description.chars().count(),
            MAX_CRATE_DESCRIPTION_CHARS
        );
        assert_eq!(
            data.hits[0]
                .documentation
                .as_ref()
                .map(|s| s.chars().count()),
            Some(MAX_URL_FIELD_CHARS)
        );
        assert_eq!(
            data.meta.next_page.as_ref().map(|s| s.chars().count()),
            Some(MAX_URL_FIELD_CHARS)
        );
        assert_eq!(
            data.meta.prev_page.as_ref().map(|s| s.chars().count()),
            Some(MAX_URL_FIELD_CHARS)
        );
    }

    #[test]
    fn truncate_chars_preserves_short_and_utf8() {
        assert_eq!(truncate_chars("abc", 10), "abc");
        assert_eq!(truncate_chars("abcdef", 3), "abc");
        // Multi-byte: take by char, not byte.
        assert_eq!(truncate_chars("日本語abc", 2), "日本");
    }
}
