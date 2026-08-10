//! crates.io search execution over [`crate::http::HttpClient`].

use url::Url;

use super::parse::parse_search_body;
use super::types::{SearchCratesData, SearchEcho, echo_params_from_url};
use super::urls::{clamp_search_pagination, planned_url_on_host};
use crate::cli::SortKind;
use crate::domain::{AllowedOrigin, SearchQuery};
use crate::error::{AppError, AppResult};
use crate::http::{HttpClient, decode_utf8, require_content_type_json};

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
/// Returns [`crate::error::ErrorKind::Internal`] when the origin URL is invalid.
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
/// Returns [`crate::error::ErrorKind::Parse`] when the body is not UTF-8 or not
/// valid crates.io JSON.
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
