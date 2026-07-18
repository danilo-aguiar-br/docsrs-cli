//! docs.rs operations: readme, get-item, search-in-crate.
//!
//! # Extraction posture (Rules Rust — web scraping / data extraction)
//!
//! - **Transport** is HTTP GET ([`HttpClient`]); this module only parses bytes.
//! - **Structure** uses `scraper` (html5ever) + CSS selectors — never regex for
//!   field extraction from HTML.
//! - **Sanitization** may use process-static regex only to strip `on*` handlers
//!   and `javascript:` URLs after DOM script/style removal (XSS hygiene, not
//!   content discovery).
//! - **Markdown** conversion via `htmd` on the sanitized fragment.
//! - **Provenance:** every success payload includes `source_url` (final URL).
//! - **Encoding:** UTF-8 only after BOM strip ([`crate::http::decode_utf8`]);
//!   docs.rs / doc.rust-lang.org ship UTF-8. `encoding_rs` multi-charset is OOS.
//! - **Not a crawl:** no link frontier, no sitemap, no RSS/Atom, no robots meta
//!   index policy (agent one-shot read, not dataset indexing).
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **I/O stage:** HTTPS GET via [`HttpClient`] on multi-thread Tokio workers.
//! - **CPU stage:** HTML scrape + markdown conversion via
//!   [`ConcurrencyBudget::run_cpu_bound`] (`spawn_blocking` + Semaphore).
//! - **Hit scan:** large `all.html` candidate lists use `rayon` after a size
//!   threshold; small lists stay sequential to avoid pool overhead.

use std::collections::HashSet;
use std::sync::LazyLock;

use rayon::prelude::*;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::{
    HOST_DOC_RUST_LANG_ORG, HOST_DOCS_RS, MAX_SEARCH_IN_CRATE_LIMIT, is_stdlib_crate,
    stdlib_channel,
};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::{HttpClient, content_type_looks_html, decode_utf8};
use crate::item_kind::{ItemKind, rustc_crate_name};

/// Crate overview (rustdoc crate docblock, not git README).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadmeData {
    /// Requested crate name.
    pub crate_name: String,
    /// Requested version token.
    pub version: String,
    /// Version resolved by docs.rs redirects when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_version: Option<String>,
    /// Markdown extracted from the crate overview.
    pub markdown: String,
    /// True when the overview body is empty.
    pub empty: bool,
    /// True when output was truncated by `max_output_bytes`.
    pub truncated: bool,
    /// Final source URL of the HTML page.
    pub source_url: String,
}

/// Typed item documentation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetItemData {
    /// Requested crate name.
    pub crate_name: String,
    /// Normalized item kind string.
    pub item_type: String,
    /// Requested item path.
    pub item_path: String,
    /// Requested version token.
    pub version: String,
    /// Version resolved by docs.rs redirects when known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_version: Option<String>,
    /// Markdown extracted from the item page.
    pub markdown: String,
    /// True when the item body is empty.
    pub empty: bool,
    /// True when output was truncated by `max_output_bytes`.
    pub truncated: bool,
    /// Final source URL of the HTML page.
    pub source_url: String,
    /// Page title when available.
    pub title: String,
}

/// Single all.html hit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchInCrateHit {
    /// Symbol or path name from all.html.
    pub name: String,
    /// Item kind string (`struct`, `fn`, …).
    pub kind: String,
    /// Absolute URL of the item documentation page.
    pub url: String,
}

/// search-in-crate result set (`truncated` is true when `total > emitted`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInCrateData {
    /// Requested crate name.
    pub crate_name: String,
    /// Substring filter applied to hits.
    pub query: String,
    /// Requested version token.
    pub version: String,
    /// Total classified hits before the limit.
    pub total: usize,
    /// Number of hits actually emitted.
    pub emitted: usize,
    /// Emitted hits.
    pub hits: Vec<SearchInCrateHit>,
    /// True when the hit list was cut by `--limit` (`total > emitted`).
    pub truncated: bool,
    /// Final source URL of the all.html page.
    pub source_url: String,
}

fn default_docs_origin(crate_name: &str) -> String {
    if is_stdlib_crate(crate_name) {
        format!("https://{HOST_DOC_RUST_LANG_ORG}")
    } else {
        format!("https://{HOST_DOCS_RS}")
    }
}

/// Build crate index URL with rustc hyphen→underscore segment.
///
/// # Errors
///
/// Propagates [`ErrorKind::Internal`] from [`readme_url_on_origin`] when the URL is invalid.
pub fn readme_url(crate_name: &str, version: &str) -> AppResult<Url> {
    readme_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build readme URL against a custom origin (wiremock tests).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
pub fn readme_url_on_origin(origin: &str, crate_name: &str, version: &str) -> AppResult<Url> {
    let origin = origin.trim_end_matches('/');
    let s = if is_stdlib_crate(crate_name) {
        // doc.rust-lang.org/{channel}/{crate}/index.html
        let channel = stdlib_channel(version);
        format!("{origin}/{channel}/{crate_name}/index.html")
    } else {
        let rustc = rustc_crate_name(crate_name);
        format!("{origin}/{crate_name}/{version}/{rustc}/index.html")
    };
    Url::parse(&s).map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid readme URL", e))
}

/// Build rustdoc item or module URL.
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] or [`ErrorKind::Internal`] from
/// [`get_item_url_on_origin`].
pub fn get_item_url(
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    get_item_url_on_origin(
        &default_docs_origin(crate_name),
        crate_name,
        version,
        kind,
        segments,
    )
}

/// Build get-item URL against a custom origin (wiremock tests).
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when a non-module path lacks an item name.
/// Returns [`ErrorKind::Internal`] when the assembled path is not a valid URL.
pub fn get_item_url_on_origin(
    origin: &str,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    let origin = origin.trim_end_matches('/');
    let rustc_root = rustc_crate_name(crate_name);
    // Optional leading crate prefix (`tokio::runtime::Runtime`) is stripped only when
    // it is a path prefix (2+ segments) or the target is the crate-root module.
    // A single segment equal to the crate name is the item itself (e.g. attribute
    // `async_trait` on crate `async-trait`) and must NOT be stripped.
    let segs: Vec<String> = {
        let mut s = segments.to_vec();
        if let Some(first) = s.first() {
            let f = first.as_str();
            let is_crate_prefix = f == crate_name || f == rustc_root.as_str();
            if is_crate_prefix && (s.len() >= 2 || kind == ItemKind::Module) {
                s.remove(0);
            }
        }
        s
    };

    let url_str = if is_stdlib_crate(crate_name) {
        // doc.rust-lang.org/{channel}/{crate}/[mod/]{kind}.{Name}.html
        let channel = stdlib_channel(version);
        if kind == ItemKind::Module {
            let mut parts: Vec<String> = Vec::new();
            for p in &segs {
                parts.push(rustc_crate_name(p));
            }
            if parts.is_empty() {
                format!("{origin}/{channel}/{crate_name}/index.html")
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/index.html",
                    parts.join("/")
                )
            }
        } else {
            if segs.is_empty() {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "item path missing item name",
                ));
            }
            let item_name = segs.last().ok_or_else(|| {
                AppError::new(ErrorKind::InvalidInput, "item path missing item name")
            })?;
            let mod_parts: Vec<String> = segs[..segs.len().saturating_sub(1)]
                .iter()
                .map(|p| rustc_crate_name(p))
                .collect();
            if mod_parts.is_empty() {
                format!(
                    "{origin}/{channel}/{crate_name}/{}.{}.html",
                    kind.file_prefix(),
                    item_name
                )
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html",
                    mod_parts.join("/"),
                    kind.file_prefix(),
                    item_name
                )
            }
        }
    } else if kind == ItemKind::Module {
        let mut parts: Vec<String> = vec![rustc_root];
        for p in &segs {
            parts.push(rustc_crate_name(p));
        }
        format!(
            "{origin}/{crate_name}/{version}/{}/index.html",
            parts.join("/")
        )
    } else {
        if segs.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "item path missing item name",
            ));
        }
        let item_name = segs
            .last()
            .ok_or_else(|| AppError::new(ErrorKind::InvalidInput, "item path missing item name"))?;
        let mod_parts: Vec<String> = if segs.len() == 1 {
            vec![rustc_root]
        } else {
            let mut m = vec![rustc_root];
            for p in &segs[..segs.len() - 1] {
                m.push(rustc_crate_name(p));
            }
            m
        };
        format!(
            "{origin}/{crate_name}/{version}/{}/{}.{}.html",
            mod_parts.join("/"),
            kind.file_prefix(),
            item_name
        )
    };

    Url::parse(&url_str)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid get-item URL", e))
}

/// Build all.html index URL.
///
/// # Errors
///
/// Propagates [`ErrorKind::Internal`] from [`all_html_url_on_origin`].
pub fn all_html_url(crate_name: &str, version: &str) -> AppResult<Url> {
    all_html_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build all.html URL against a custom origin (wiremock tests).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
pub fn all_html_url_on_origin(origin: &str, crate_name: &str, version: &str) -> AppResult<Url> {
    let origin = origin.trim_end_matches('/');
    let s = if is_stdlib_crate(crate_name) {
        let channel = stdlib_channel(version);
        format!("{origin}/{channel}/{crate_name}/all.html")
    } else {
        let rustc = rustc_crate_name(crate_name);
        format!("{origin}/{crate_name}/{version}/{rustc}/all.html")
    };
    Url::parse(&s)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid all.html URL", e))
}

fn extract_resolved_version(final_url: &Url, requested: &str) -> Option<String> {
    let mut segs = final_url.path_segments()?.filter(|s| !s.is_empty());
    let _pkg = segs.next()?;
    let ver = segs.next()?;
    if ver == "latest" {
        return None;
    }
    if requested == "latest" || ver != requested {
        return Some(ver.to_string());
    }
    Some(ver.to_string())
}

/// Compiled once: strip `on*` event handlers from HTML fragments (hot path for every HTML→MD).
static RE_ON_HANDLERS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
        .expect("hardcoded on* handler strip regex is valid by construction")
});

/// Compiled once: strip `javascript:` URLs from href/src attributes.
static RE_JS_URLS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)(href|src)\s*=\s*["']\s*javascript:[^"']*["']"#)
        .expect("hardcoded javascript: URL strip regex is valid by construction")
});

/// Remove script/style nodes and on* handlers before Markdown conversion.
///
/// Extraction of doc content uses CSS selectors elsewhere in this module.
/// Regexes here are **security scrub only** (`on*` / `javascript:`), compiled
/// once via process-static [`LazyLock`] — not used to scrape fields from HTML.
pub fn sanitize_html_fragment(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    let drop_sel = Selector::parse("script, style, noscript")
        .expect("hardcoded scraper selector 'script, style, noscript' is valid by construction");
    // Prefer reconstructing from root children excluding dropped tags.
    if let Some(root) = fragment.tree.root().children().next() {
        let _ = root;
    }
    // Collect HTML of non-script/style nodes by stripping matches from a working copy.
    // Capacity ≈ input length: script/style removals only shrink the buffer.
    // Precondition: `html` is already body-capped (HARD_MAX_BODY_BYTES) and resident in RAM.
    let mut out = String::with_capacity(html.len());
    out.push_str(html);
    for el in fragment.select(&drop_sel) {
        let tag_html = el.html();
        out = out.replace(&tag_html, "");
    }
    // Strip event handlers: onload=, onclick=, etc.
    out = RE_ON_HANDLERS.replace_all(&out, "").into_owned();
    // Drop javascript: URLs in href/src
    out = RE_JS_URLS.replace_all(&out, "").into_owned();
    out
}

fn html_to_markdown(html: &str) -> AppResult<String> {
    let cleaned = sanitize_html_fragment(html);
    htmd::convert(&cleaned).map_err(|e| {
        AppError::new(
            ErrorKind::Parse,
            format!("HTML to Markdown conversion failed: {e}"),
        )
    })
}

fn first_inner_html(document: &Html, selectors: &[&str]) -> Option<String> {
    for sel in selectors {
        if let Ok(s) = Selector::parse(sel)
            && let Some(n) = document.select(&s).next()
        {
            let inner = n.inner_html();
            if !inner.trim().is_empty() {
                return Some(inner);
            }
        }
    }
    None
}

/// Fetch and convert crate overview docblock (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and HTML conversion failures from
/// [`fetch_readme_on_origin`].
pub async fn fetch_readme(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
) -> AppResult<ReadmeData> {
    fetch_readme_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
    )
    .await
}

/// Fetch readme against a configurable origin (offline mocks).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the origin URL is invalid.
/// Propagates fetch failures from [`fetch_readme_at`].
pub async fn fetch_readme_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
) -> AppResult<ReadmeData> {
    let url = readme_url_on_origin(origin, crate_name, version)?;
    fetch_readme_at(http, crate_name, version, &url).await
}

/// Fetch readme from a prebuilt URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] on non-UTF-8 bodies or HTML→Markdown failure.
pub async fn fetch_readme_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    url: &Url,
) -> AppResult<ReadmeData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("crate or version not found: {crate_name}@{version}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "docs.rs readme",
        ));
    }
    if resp.content_type.is_some() && !content_type_looks_html(resp.content_type.as_deref()) {
        tracing::warn!(content_type = ?resp.content_type, "unexpected Content-Type from docs.rs");
    }

    let body = resp.body.clone();
    let (markdown, empty) = http
        .budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            extract_readme_markdown_from_html(&text)
        })
        .await?;
    let resolved = extract_resolved_version(&resp.final_url, version);
    Ok(ReadmeData {
        crate_name: crate_name.to_string(),
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
    })
}

/// Fetch and convert a typed rustdoc item page (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and HTML conversion failures from
/// [`fetch_item_on_origin`].
pub async fn fetch_item(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    fetch_item_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
        kind,
        segments,
    )
    .await
}

/// Fetch item against a configurable origin (offline mocks).
///
/// # Errors
///
/// Propagates URL build errors from [`get_item_url_on_origin`] and fetch errors
/// from [`fetch_item_at`].
pub async fn fetch_item_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    let url = get_item_url_on_origin(origin, crate_name, version, kind, segments)?;
    fetch_item_at(http, crate_name, version, kind, segments, &url).await
}

/// Fetch item from a prebuilt URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] on non-UTF-8 bodies or HTML→Markdown failure.
pub async fn fetch_item_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
    url: &Url,
) -> AppResult<GetItemData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("item not found at {url}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "docs.rs get-item",
        ));
    }

    let body = resp.body.clone();
    let (markdown, empty) = http
        .budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            extract_item_markdown_from_html(&text)
        })
        .await?;
    let item_path = segments.join("::");
    let title = format!("{item_path} ({})", kind.as_str());
    let resolved = extract_resolved_version(&resp.final_url, version);

    Ok(GetItemData {
        crate_name: crate_name.to_string(),
        item_type: kind.as_str().to_string(),
        item_path,
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
        title,
    })
}

/// Parse all.html and filter symbols (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and parse failures from
/// [`search_in_crate_on_origin`].
pub async fn search_in_crate(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
) -> AppResult<SearchInCrateData> {
    search_in_crate_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
        query,
        item_type,
        limit,
    )
    .await
}

/// search-in-crate against a configurable origin (offline mocks).
///
/// # Errors
///
/// Propagates URL build errors from [`all_html_url_on_origin`] and fetch errors
/// from [`search_in_crate_at`].
pub async fn search_in_crate_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
) -> AppResult<SearchInCrateData> {
    let url = all_html_url_on_origin(origin, crate_name, version)?;
    search_in_crate_at(http, crate_name, version, query, item_type, limit, &url).await
}

/// search-in-crate against a prebuilt all.html URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`ErrorKind::Parse`] on non-UTF-8 bodies or href join failures.
/// Propagates parse errors from [`search_in_crate_from_html`].
pub async fn search_in_crate_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
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
            "docs.rs all.html",
        ));
    }

    let body = resp.body.clone();
    let source_url = resp.final_url.to_string();
    let crate_name = crate_name.to_string();
    let version = version.to_string();
    let query = query.to_string();
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
                &source_url,
            )
        })
        .await
}

/// Join relative/absolute/full href against crate rustdoc base.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when a relative or absolute-path href cannot be joined.
pub fn join_href(base: &Url, href: &str) -> AppResult<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(href.to_string());
    }
    if href.starts_with('/') {
        let joined = Url::parse(&format!("https://docs.rs{href}")).map_err(|e| {
            AppError::with_source(ErrorKind::Parse, "failed to join absolute path href", e)
        })?;
        return Ok(joined.to_string());
    }
    base.join(href)
        .map(|u| u.to_string())
        .map_err(|e| AppError::with_source(ErrorKind::Parse, "failed to join relative href", e))
}

/// Minimum candidate count before `rayon` fan-out (overhead below this is pure loss).
const RAYON_HIT_THRESHOLD: usize = 64;

/// One classified hit candidate before dedup/limit (index preserves document order).
type ClassifiedHit = (usize, String, ItemKind, String);

/// Pure parse of all.html body for offline tests and CPU workers.
///
/// Large candidate lists use `rayon`; small lists stay sequential.
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when the synthetic base URL is invalid.
/// Returns [`ErrorKind::Parse`] when an href cannot be joined via [`join_href`].
pub fn parse_all_html_hits(
    html: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: usize,
) -> AppResult<Vec<SearchInCrateHit>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let document = Html::parse_document(html);
    let a_sel = Selector::parse("#main-content a")
        .expect("hardcoded scraper selector '#main-content a' is valid by construction");
    let rustc = rustc_crate_name(crate_name);
    let base = Url::parse(&format!(
        "https://{HOST_DOCS_RS}/{crate_name}/{version}/{rustc}/"
    ))
    .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid all.html base", e))?;
    let q = query.trim().to_ascii_lowercase();

    // Materialize (name, href) so we can choose sequential vs rayon without holding DOM refs.
    let candidates: Vec<(String, String)> = document
        .select(&a_sel)
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
        return filter_hits_sequential(candidates, &base, &q, item_type, limit);
    }
    filter_hits_parallel(candidates, &base, &q, item_type, limit)
}

fn filter_hits_sequential(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
) -> AppResult<Vec<SearchInCrateHit>> {
    let mut hits = Vec::with_capacity(limit.min(256));
    let mut seen: HashSet<(String, ItemKind)> = HashSet::new();
    for (name, href) in candidates {
        if let Some(hit) = classify_hit(name, &href, base, q, item_type, &mut seen)? {
            hits.push(hit);
            if hits.len() >= limit {
                break;
            }
        }
    }
    Ok(hits)
}

fn filter_hits_parallel(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
) -> AppResult<Vec<SearchInCrateHit>> {
    // Parallel classify/filter; preserve original order via index for stable agent output.
    let mapped: AppResult<Vec<Option<ClassifiedHit>>> = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, (name, href))| {
            let Some(kind) = ItemKind::from_href(&href) else {
                return Ok(None);
            };
            if let Some(filter) = item_type
                && kind != filter
            {
                return Ok(None);
            }
            if !q.is_empty() && !name.to_ascii_lowercase().contains(q) {
                return Ok(None);
            }
            let abs = join_href(base, &href)?;
            Ok(Some((idx, name, kind, abs)))
        })
        .collect();
    let mut rows: Vec<ClassifiedHit> = mapped?.into_iter().flatten().collect();
    rows.par_sort_unstable_by_key(|(idx, _, _, _)| *idx);

    let mut hits = Vec::with_capacity(limit.min(256));
    let mut seen: HashSet<(String, ItemKind)> = HashSet::new();
    for (_idx, name, kind, abs) in rows {
        if !seen.insert((name.clone(), kind)) {
            continue;
        }
        hits.push(SearchInCrateHit {
            name,
            kind: kind.as_str().to_string(),
            url: abs,
        });
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

fn classify_hit(
    name: String,
    href: &str,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    seen: &mut HashSet<(String, ItemKind)>,
) -> AppResult<Option<SearchInCrateHit>> {
    let Some(kind) = ItemKind::from_href(href) else {
        return Ok(None);
    };
    if let Some(filter) = item_type
        && kind != filter
    {
        return Ok(None);
    }
    if !q.is_empty() && !name.to_ascii_lowercase().contains(q) {
        return Ok(None);
    }
    let abs = join_href(base, href)?;
    if !seen.insert((name.clone(), kind)) {
        return Ok(None);
    }
    Ok(Some(SearchInCrateHit {
        name,
        kind: kind.as_str().to_string(),
        url: abs,
    }))
}

/// Extract readme markdown from raw HTML (offline tests / pure path).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_readme_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    let document = Html::parse_document(html);
    let content = first_inner_html(
        &document,
        &[
            ".rustdoc .docblock",
            "#main-content .docblock",
            ".docblock",
            ".rustdoc-main .item-decl",
            ".item-decl",
        ],
    );
    match content {
        Some(h) => Ok((html_to_markdown(&h)?, false)),
        None => Ok((String::new(), true)),
    }
}

/// Extract get-item markdown from raw HTML (offline tests / pure path).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_item_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    let document = Html::parse_document(html);
    let content = first_inner_html(&document, &["#main-content", ".item-decl", ".docblock"])
        .unwrap_or_default();
    let empty = content.trim().is_empty();
    if empty {
        Ok((String::new(), true))
    } else {
        Ok((html_to_markdown(&content)?, false))
    }
}

/// Build SearchInCrateData from HTML body without network (offline tests).
///
/// `--limit 0` is honoured: `emitted = 0`, `hits = []`, and `truncated` is true
/// when the unfiltered total is greater than zero. Values above
/// [`MAX_SEARCH_IN_CRATE_LIMIT`] are capped.
///
/// # Errors
///
/// Propagates parse failures from [`parse_all_html_hits`].
pub fn search_in_crate_from_html(
    html: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
    source_url: &str,
) -> AppResult<SearchInCrateData> {
    let limit = (limit as usize).min(MAX_SEARCH_IN_CRATE_LIMIT as usize);
    let all = parse_all_html_hits(html, crate_name, version, query, item_type, usize::MAX)?;
    let total = all.len();
    let hits: Vec<_> = all.into_iter().take(limit).collect();
    let emitted = hits.len();
    Ok(SearchInCrateData {
        crate_name: crate_name.to_string(),
        query: query.to_string(),
        version: version.to_string(),
        total,
        emitted,
        hits,
        truncated: total > emitted,
        source_url: source_url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_url_hyphen() {
        let u = readme_url("async-trait", "latest").unwrap();
        assert_eq!(
            u.as_str(),
            "https://docs.rs/async-trait/latest/async_trait/index.html"
        );
    }

    #[test]
    fn get_item_nested() {
        let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
        let u = get_item_url("tokio", "latest", ItemKind::Struct, &segs).unwrap();
        assert!(u.as_str().contains("/tokio/runtime/struct.Runtime.html"));
    }

    #[test]
    fn get_item_without_crate_prefix() {
        let segs = vec!["Parser".into()];
        let u = get_item_url("clap", "latest", ItemKind::Trait, &segs).unwrap();
        assert!(
            u.as_str().ends_with("/clap/trait.Parser.html")
                || u.path().ends_with("/clap/trait.Parser.html")
        );
    }

    #[test]
    fn get_item_keeps_single_segment_equal_to_crate_name() {
        // Attribute/macro items often share the rustc crate name.
        let segs = vec!["async_trait".into()];
        let u = get_item_url("async-trait", "latest", ItemKind::Attribute, &segs).unwrap();
        assert!(
            u.as_str()
                .ends_with("/async-trait/latest/async_trait/attr.async_trait.html"),
            "url={u}"
        );
    }

    #[test]
    fn get_item_module_single_segment_crate_name_is_root() {
        let segs = vec!["async_trait".into()];
        let u = get_item_url("async-trait", "latest", ItemKind::Module, &segs).unwrap();
        assert!(
            u.as_str()
                .ends_with("/async-trait/latest/async_trait/index.html"),
            "url={u}"
        );
    }

    #[test]
    fn join_relative() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let j = join_href(&base, "de/trait.Deserialize.html").unwrap();
        assert!(j.contains("/serde/de/trait.Deserialize.html"));
    }

    #[test]
    fn join_absolute_path_and_fragment() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let j = join_href(&base, "/serde/latest/serde/struct.Error.html").unwrap();
        assert!(j.starts_with("https://docs.rs/serde/"));
        let f = join_href(&base, "struct.Error.html#method.to_string").unwrap();
        assert!(f.contains("#method.to_string"));
    }

    #[test]
    fn sanitize_strips_script() {
        let raw = r#"<div>ok<script>alert(1)</script><p onclick="x()">hi</p></div>"#;
        let s = sanitize_html_fragment(raw);
        assert!(!s.contains("script"));
        assert!(!s.contains("onclick"));
        assert!(s.contains("hi") || s.contains("ok"));
    }

    #[test]
    fn readme_fallback_item_decl_converts() {
        let html = r#"<!DOCTYPE html><html><body class="rustdoc-main">
        <div class="item-decl"><pre>pub use foo;</pre></div>
        </body></html>"#;
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.trim().is_empty());
    }

    #[test]
    fn all_html_order_stable() {
        let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="struct.A.html">A</a>
          <a href="trait.B.html">B</a>
          <a href="fn.c.html">c</a>
          <a href="union.U.html">U</a>
          <a href="attr.x.html">x</a>
          <a href="derive.D.html">D</a>
        </div></body></html>"#;
        let hits = parse_all_html_hits(html, "demo", "1.0.0", "", None, 100).unwrap();
        assert_eq!(hits.len(), 6);
        assert_eq!(hits[0].name, "A");
        assert_eq!(hits[0].kind, "struct");
        assert_eq!(hits[3].kind, "union");
        assert_eq!(hits[4].kind, "attribute");
        assert_eq!(hits[5].kind, "derive");
    }

    #[test]
    fn std_core_alloc_urls() {
        let u = readme_url("std", "latest").unwrap();
        assert_eq!(
            u.as_str(),
            "https://doc.rust-lang.org/stable/std/index.html"
        );
        let u = get_item_url(
            "std",
            "latest",
            ItemKind::Struct,
            &["option".into(), "Option".into()],
        )
        .unwrap();
        assert_eq!(
            u.as_str(),
            "https://doc.rust-lang.org/stable/std/option/struct.Option.html"
        );
        let a = all_html_url("core", "nightly").unwrap();
        assert_eq!(
            a.as_str(),
            "https://doc.rust-lang.org/nightly/core/all.html"
        );
    }

    #[test]
    fn extract_item_from_fixture() {
        let html = include_str!("../tests/fixtures/docs_rs/get_item_main.html");
        let (md, empty) = extract_item_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(md.contains("Runtime") || md.contains("Tokio") || md.contains("runtime"));
    }

    #[test]
    fn extract_readme_primary_docblock_fixture() {
        let html = include_str!("../tests/fixtures/docs_rs/readme_docblock.html");
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.to_ascii_lowercase().contains("alert"));
    }

    #[test]
    fn search_in_crate_from_html_limit_and_filter() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            Some(ItemKind::Struct),
            1,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert_eq!(data.total, 1);
        assert_eq!(data.emitted, 1);
        assert_eq!(data.hits[0].kind, "struct");
        assert!(!data.truncated);
    }

    #[test]
    fn search_in_crate_truncated_when_limit_cuts_hits() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            2,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert!(data.total > 2, "fixture must have more than 2 hits");
        assert_eq!(data.emitted, 2);
        assert!(data.truncated);
        let full = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            1000,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert_eq!(full.total, full.emitted);
        assert!(!full.truncated);
    }

    #[test]
    fn search_in_crate_limit_zero_emits_nothing() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            0,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert!(data.total > 0, "fixture must have hits for total");
        assert_eq!(data.emitted, 0);
        assert!(data.hits.is_empty());
        assert!(data.truncated);
        assert!(
            parse_all_html_hits(html, "demo", "1.0.0", "", None, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn resolved_version_from_url() {
        let u = Url::parse("https://docs.rs/serde/1.0.210/serde/index.html").unwrap();
        assert_eq!(
            extract_resolved_version(&u, "latest").as_deref(),
            Some("1.0.210")
        );
        let u2 = Url::parse("https://docs.rs/serde/latest/serde/index.html").unwrap();
        assert!(extract_resolved_version(&u2, "latest").is_none());
    }

    #[test]
    fn module_url_template() {
        let segs = vec!["serde".into(), "de".into()];
        let u = get_item_url("serde", "latest", ItemKind::Module, &segs).unwrap();
        assert!(u.as_str().ends_with("/serde/de/index.html"));
    }

    #[test]
    fn empty_html_readme() {
        let (md, empty) = extract_readme_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }

    #[test]
    fn get_item_empty_segments_errors() {
        let err = get_item_url("clap", "latest", ItemKind::Struct, &[]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn join_full_http_url_passthrough() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let full = "https://docs.rs/serde/latest/serde/struct.Error.html";
        assert_eq!(join_href(&base, full).unwrap(), full);
        let http = "http://example.invalid/x.html";
        assert_eq!(join_href(&base, http).unwrap(), http);
    }

    #[test]
    fn origin_builders_trim_trailing_slash() {
        let u = readme_url_on_origin("https://docs.rs/", "demo", "1.0.0").unwrap();
        assert_eq!(u.as_str(), "https://docs.rs/demo/1.0.0/demo/index.html");
        let a = all_html_url_on_origin("https://docs.rs/", "demo", "1.0.0").unwrap();
        assert_eq!(a.as_str(), "https://docs.rs/demo/1.0.0/demo/all.html");
    }

    #[test]
    fn empty_item_html_extract() {
        let (md, empty) = extract_item_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }
}
