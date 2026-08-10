//! crates.io search URL builders and pagination bounds.

use url::Url;

use crate::cli::SortKind;
use crate::config::MAX_PER_PAGE;
use crate::domain::{AllowedOrigin, SearchQuery};
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp, Subject};

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
/// Returns [`crate::error::ErrorKind::InvalidInput`] when `page < 1` or `per_page` is outside
/// `1..=MAX_PER_PAGE` (100).
pub fn validate_search_pagination(per_page: u32, page: u32) -> AppResult<(u32, u32)> {
    if page < 1 {
        return Err(AppError::of(ErrorDetail::PageBelowOne));
    }
    if !(1..=MAX_PER_PAGE).contains(&per_page) {
        return Err(AppError::of(ErrorDetail::PerPageOutOfRange {
            max: MAX_PER_PAGE,
            got: per_page,
        }));
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
/// Returns [`crate::error::ErrorKind::Internal`] when the base URL cannot be parsed.
pub fn planned_url(
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<Url> {
    planned_url_on_host(
        &AllowedOrigin::crates_io_default(),
        query,
        per_page,
        sort,
        page,
    )
}

/// Build the crates.io search URL for an allowlisted origin (`page` pagination).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] when the composed base URL is invalid.
pub fn planned_url_on_host(
    origin: &AllowedOrigin,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page: u32,
) -> AppResult<Url> {
    let (per_page, page) = clamp_search_pagination(per_page, page);
    let base = crates_api_base(origin);
    let mut url = Url::parse(&base).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::UrlBuild,
            },
            e,
        )
    })?;
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
/// Returns [`crate::error::ErrorKind::Internal`] when the base URL is invalid.
/// Returns [`crate::error::ErrorKind::InvalidInput`] when the token cannot form a valid URL.
pub fn planned_url_with_page_token(
    origin: &AllowedOrigin,
    query: &SearchQuery,
    per_page: u32,
    sort: SortKind,
    page_token: &str,
) -> AppResult<Url> {
    let token = page_token.trim();
    if token.is_empty() {
        return Err(AppError::of(ErrorDetail::Empty {
            subject: Subject::PageToken,
        }));
    }
    // Reject obvious garbage that is neither a page number, query string, nor seek token
    // (GAP-W-007 — fail local before remote "bad request" wording confuses agents).
    if token.chars().any(|c| c.is_control()) || token.len() > crate::config::MAX_URL_FIELD_CHARS {
        return Err(AppError::of(ErrorDetail::ControlCharacters {
            subject: Subject::PageToken,
        }));
    }
    // Pure numeric → page number
    if let Ok(page) = token.parse::<u32>() {
        return planned_url_on_host(origin, query, per_page, sort, page);
    }
    let base = crates_api_base(origin);
    let base_url = Url::parse(&base).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::UrlBuild,
            },
            e,
        )
    })?;

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
