//! all.html search: async fetch + parallel/sequential hit classification.
//!
//! Parallelism: `rayon` only above [`RAYON_HIT_THRESHOLD`]; small lists stay sequential.

use std::collections::HashSet;
use std::sync::LazyLock;

use rayon::prelude::*;
use scraper::{Html, Selector};
use url::Url;

/// Compiled once: all.html symbol links under main content (SCRAPE-R-004).
static SEL_MAIN_CONTENT_A: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("#main-content a")
        .expect("hardcoded scraper selector '#main-content a' is valid by construction")
});

use crate::config::{MAX_SEARCH_IN_CRATE_LIMIT, RAYON_HIT_THRESHOLD};
use crate::domain::{AllowedOrigin, CrateName, MatchMode, SearchQuery, VersionArg};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::{HttpClient, decode_utf8, require_content_type_html};
use crate::item_kind::ItemKind;
use crate::shutdown::CancelFlag;

use super::types::{SearchInCrateData, SearchInCrateHit};
use super::urls::all_html_url_on_origin;

/// Parse all.html and filter symbols (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and parse failures from
/// [`search_in_crate_on_origin`].
pub async fn search_in_crate(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    query: &SearchQuery,
    item_type: Option<ItemKind>,
    limit: u32,
    match_mode: MatchMode,
) -> AppResult<SearchInCrateData> {
    // Prefer stdlib origin for std/core/alloc (same rule as ops::docs_origin_for_crate).
    let origin = if crate_name.is_stdlib() {
        AllowedOrigin::stdlib_docs_default()
    } else {
        AllowedOrigin::docs_rs_default()
    };
    search_in_crate_on_origin(
        http,
        &origin,
        crate_name,
        version,
        query,
        item_type,
        limit,
        match_mode,
    )
    .await
}

/// search-in-crate against a configurable origin (offline mocks).
///
/// # Errors
///
/// Propagates URL build errors from [`all_html_url_on_origin`] and fetch errors
/// from [`search_in_crate_at`].
#[allow(clippy::too_many_arguments)] // domain triple + filter/limit/mode are intentional
pub async fn search_in_crate_on_origin(
    http: &HttpClient,
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
    query: &SearchQuery,
    item_type: Option<ItemKind>,
    limit: u32,
    match_mode: MatchMode,
) -> AppResult<SearchInCrateData> {
    let url = all_html_url_on_origin(origin, crate_name, version)?;
    search_in_crate_at(
        http, crate_name, version, query, item_type, limit, match_mode, &url,
    )
    .await
}

/// search-in-crate against a prebuilt all.html URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] on non-UTF-8 bodies or href join failures.
/// Propagates parse errors from [`search_in_crate_from_html`].
#[allow(clippy::too_many_arguments)] // domain triple + filter/limit/mode + url
pub async fn search_in_crate_at(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    query: &SearchQuery,
    item_type: Option<ItemKind>,
    limit: u32,
    match_mode: MatchMode,
    url: &Url,
) -> AppResult<SearchInCrateData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("all.html not found for {crate_name}@{version}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "rustdoc all.html",
        ));
    }
    require_content_type_html(resp.content_type.as_deref())?;

    // Move body into the CPU worker (one-shot memory: no clone for a second decode).
    let body = resp.body;
    let source_url = resp.final_url.to_string();
    let cache_hit = resp.cache_hit;
    let crate_name = crate_name.clone();
    let version = version.clone();
    let query = query.clone();
    let cancel = http.cancel_flag().clone();
    http.budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            search_in_crate_from_html(
                &text,
                &crate_name,
                &version,
                &query,
                item_type,
                limit,
                match_mode,
                &source_url,
                cache_hit,
                &cancel,
            )
        })
        .await
}

/// Resolve an all.html `href` against `base` (`source_url`), same-origin only.
///
/// - Relative and path-absolute hrefs use [`Url::join`] against `base` (never a
///   hardcoded docs.rs host — SCRAPE-Q-001).
/// - Absolute `http(s)://` hrefs are accepted only when **scheme + host** match
///   `base` (host compared case-insensitively).
/// - Off-origin absolute URLs and join/parse failures return [`None`] so callers
///   can soft-skip the hit without failing the entire search (SCRAPE-S-001).
#[must_use]
pub fn resolve_hit_url(base: &Url, href: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    let joined = if href.starts_with("http://") || href.starts_with("https://") {
        Url::parse(href).ok()?
    } else {
        // Url::join handles relative segments and absolute paths (`/…`).
        base.join(href).ok()?
    };
    if !same_origin(base, &joined) {
        return None;
    }
    Some(joined.to_string())
}

/// True when `candidate` shares scheme and host with `base` (SCRAPE-S-001).
fn same_origin(base: &Url, candidate: &Url) -> bool {
    if candidate.scheme() != base.scheme() {
        return false;
    }
    match (base.host_str(), candidate.host_str()) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (None, None) => true,
        _ => false,
    }
}

/// Join relative/absolute href against crate rustdoc base (same-origin).
///
/// Prefer [`resolve_hit_url`] in hit classification (soft-skip). This wrapper
/// maps off-origin / join failure to [`ErrorKind::Parse`] for callers that want
/// a hard error.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when the href is off-origin or not joinable.
pub fn join_href(base: &Url, href: &str) -> AppResult<String> {
    resolve_hit_url(base, href).ok_or_else(|| {
        AppError::new(
            ErrorKind::Parse,
            "failed to join href (off-origin or invalid against source_url base)",
        )
    })
}

/// One classified hit candidate before dedup/limit (index preserves document order).
/// One classified hit candidate before dedup/limit: (index, name, kind, url, score).
type ClassifiedHit = (usize, String, ItemKind, String, u8);

/// Pure parse of all.html body for offline tests and CPU workers.
///
/// `base` must be the final all.html URL (or its directory). Relative and
/// absolute-path hrefs resolve via [`resolve_hit_url`] against this URL — never
/// against a hardcoded `docs.rs` host — so stdlib (`doc.rust-lang.org`) and
/// mock origins keep correct hit URLs (SCRAPE-Q-001). Off-origin absolute hrefs
/// are soft-skipped (SCRAPE-S-001).
///
/// Large candidate lists use `rayon`; small lists stay sequential.
/// Results are scored and sorted (exact leaf first) before applying `limit`.
///
/// # Errors
///
/// Currently infallible for join failures (soft-skip); reserved for future
/// hard parse failures of the document itself.
pub fn parse_all_html_hits(
    html: &str,
    base: &Url,
    query: &SearchQuery,
    item_type: Option<ItemKind>,
    limit: usize,
    match_mode: MatchMode,
    cancel: &CancelFlag,
) -> AppResult<Vec<SearchInCrateHit>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    cancel.check()?;
    let document = Html::parse_document(html);
    let q = query.as_str().trim().to_ascii_lowercase();

    let candidates: Vec<(String, String)> = document
        .select(&SEL_MAIN_CONTENT_A)
        .filter_map(|a| {
            let name = a.text().collect::<String>().trim().to_string();
            if name.is_empty() {
                return None;
            }
            let href = a.value().attr("href")?.to_string();
            if href.is_empty() {
                return None;
            }
            Some((name, href))
        })
        .collect();

    if candidates.len() < RAYON_HIT_THRESHOLD {
        return filter_hits_sequential(candidates, base, &q, item_type, limit, match_mode, cancel);
    }
    filter_hits_parallel(candidates, base, &q, item_type, limit, match_mode, cancel)
}

pub(super) fn filter_hits_sequential(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
    match_mode: MatchMode,
    cancel: &CancelFlag,
) -> AppResult<Vec<SearchInCrateHit>> {
    // Pre-size for memory discipline (Rules Rust — gerenciamento de memória).
    let mut rows: Vec<ClassifiedHit> = Vec::with_capacity(candidates.len());
    for (idx, (name, href)) in candidates.into_iter().enumerate() {
        cancel.check()?;
        if let Some(hit) = classify_hit_row(idx, name, &href, base, q, item_type, match_mode)? {
            rows.push(hit);
        }
    }
    finalize_hits(rows, limit)
}

pub(super) fn filter_hits_parallel(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
    match_mode: MatchMode,
    cancel: &CancelFlag,
) -> AppResult<Vec<SearchInCrateHit>> {
    // Clone Arc-backed flag once for the rayon pool (cheap; no product env).
    let cancel = cancel.clone();
    let mapped: AppResult<Vec<Option<ClassifiedHit>>> = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, (name, href))| {
            // Cooperative cancel: first cancelled worker returns Interrupted/Terminated.
            cancel.check()?;
            classify_hit_row(idx, name, &href, base, q, item_type, match_mode)
        })
        .collect();
    let rows: Vec<ClassifiedHit> = mapped?.into_iter().flatten().collect();
    finalize_hits(rows, limit)
}

pub(super) fn finalize_hits(mut rows: Vec<ClassifiedHit>, limit: usize) -> AppResult<Vec<SearchInCrateHit>> {
    // Score ascending, then original document order for stability.
    rows.sort_by(|a, b| a.4.cmp(&b.4).then_with(|| a.0.cmp(&b.0)));
    // Capacity: min(limit, rows) so `usize::MAX` total-scan never overflows (SCRAPE-S-005).
    let mut hits = Vec::with_capacity(limit.min(rows.len()));
    let mut seen: HashSet<(String, ItemKind)> = HashSet::new();
    for (_idx, name, kind, abs, score) in rows {
        if !seen.insert((name.clone(), kind)) {
            continue;
        }
        hits.push(SearchInCrateHit {
            name,
            kind: kind.as_str().to_string(),
            url: abs,
            score: Some(score),
        });
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

pub(super) fn classify_hit_row(
    idx: usize,
    name: String,
    href: &str,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    match_mode: MatchMode,
) -> AppResult<Option<ClassifiedHit>> {
    let Some(kind) = ItemKind::from_href(href) else {
        return Ok(None);
    };
    if let Some(filter) = item_type
        && kind != filter
    {
        return Ok(None);
    }
    let Some(score) = match_mode.score(&name, q) else {
        return Ok(None);
    };
    // Soft-skip off-origin / unjoinable hrefs — never abort the whole search (SCRAPE-S-001).
    let Some(abs) = resolve_hit_url(base, href) else {
        return Ok(None);
    };
    Ok(Some((idx, name, kind, abs, score)))
}

/// Build SearchInCrateData from HTML body without network (offline tests).
///
/// `--limit 0` is honoured: `emitted = 0`, `hits = []`, and `truncated` is true
/// when the unfiltered total is greater than zero. Values above
/// [`MAX_SEARCH_IN_CRATE_LIMIT`] are capped.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when `source_url` is not a valid URL base.
/// Propagates parse failures from [`parse_all_html_hits`].
#[allow(clippy::too_many_arguments)] // pure offline path mirrors the async API surface
pub fn search_in_crate_from_html(
    html: &str,
    crate_name: &CrateName,
    version: &VersionArg,
    query: &SearchQuery,
    item_type: Option<ItemKind>,
    limit: u32,
    match_mode: MatchMode,
    source_url: &str,
    cache_hit: bool,
    cancel: &CancelFlag,
) -> AppResult<SearchInCrateData> {
    let limit = (limit as usize).min(MAX_SEARCH_IN_CRATE_LIMIT as usize);
    // Join base = final all.html URL (stdlib / mock / docs.rs). Never rebuild host.
    let base = Url::parse(source_url).map_err(|e| {
        AppError::with_source(ErrorKind::Parse, "invalid source_url for hit join base", e)
    })?;
    let all = parse_all_html_hits(html, &base, query, item_type, usize::MAX, match_mode, cancel)?;
    let total = all.len();
    let hits: Vec<_> = all.into_iter().take(limit).collect();
    let emitted = hits.len();
    // Empty query: scores are 0 — omit score noise on wire for cleaner lists.
    let hits = if query.as_str().trim().is_empty() {
        hits.into_iter()
            .map(|mut h| {
                h.score = None;
                h
            })
            .collect()
    } else {
        hits
    };
    Ok(SearchInCrateData {
        crate_name: crate_name.as_str().to_string(),
        query: query.as_str().to_string(),
        version: version.as_str().to_string(),
        item_type: item_type.map(|k| k.as_str().to_string()),
        match_mode: match_mode.as_str().to_string(),
        total,
        emitted,
        hits,
        truncated: total > emitted,
        source_url: source_url.to_string(),
        cache_hit,
    })
}
