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
use crate::domain::MatchMode;
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
    /// True when the HTTP body was served from the local disk cache.
    pub cache_hit: bool,
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
    /// Last path segment (leaf name) for agent convenience.
    pub item_name: String,
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
    /// True when the HTTP body was served from the local disk cache.
    pub cache_hit: bool,
    /// How markdown was scoped for associated methods: `method` or `item_page`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction: Option<String>,
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
    /// Match quality score (0 = best). Omitted when query is empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<u8>,
}

/// search-in-crate result set (`truncated` is true when `total > emitted`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInCrateData {
    /// Requested crate name.
    pub crate_name: String,
    /// Text filter applied to hits.
    pub query: String,
    /// Requested version token.
    pub version: String,
    /// Echo of the applied item-type filter when set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_type: Option<String>,
    /// Match mode applied (`exact` | `prefix` | `substring`).
    pub match_mode: String,
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
    /// True when the HTTP body was served from the local disk cache.
    pub cache_hit: bool,
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

/// Detect associated method / inherent method path: `Type::method` where Type is
/// UpperCamel and method starts lowercase (or kind forced as method via parse alias).
fn is_method_path(kind: ItemKind, segs: &[String]) -> bool {
    if segs.len() < 2 {
        return false;
    }
    if kind != ItemKind::Fn {
        return false;
    }
    let parent = segs[segs.len() - 2].as_str();
    let method = segs[segs.len() - 1].as_str();
    let parent_type = parent
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase());
    let method_fn = method
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_');
    parent_type && method_fn
}

/// Parent type kinds tried when resolving a method URL (struct first — common case).
const METHOD_PARENT_KINDS: &[ItemKind] = &[
    ItemKind::Struct,
    ItemKind::Enum,
    ItemKind::Trait,
    ItemKind::Type,
    ItemKind::Union,
];

/// Build get-item URL against a custom origin (wiremock tests).
///
/// Associated methods (`Runtime::new`) resolve to the parent type page plus
/// `#method.{name}` (rustdoc layout). Free functions keep `{kind}.{name}.html`.
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
    get_item_url_on_origin_with_parent_kind(origin, crate_name, version, kind, segments, None)
}

/// Like [`get_item_url_on_origin`] but forces the parent type kind for methods.
pub fn get_item_url_on_origin_with_parent_kind(
    origin: &str,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
    parent_kind_override: Option<ItemKind>,
) -> AppResult<Url> {
    let origin = origin.trim_end_matches('/');
    let rustc_root = rustc_crate_name(crate_name);
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

    // Associated method: Type::method → parent page + #method.name
    if is_method_path(kind, &segs) {
        let method_name = segs
            .last()
            .expect("is_method_path requires >=2 segs")
            .clone();
        let parent_name = segs[segs.len() - 2].clone();
        let parent_kind = parent_kind_override.unwrap_or(ItemKind::Struct);
        let mod_parts: Vec<String> = if segs.len() == 2 {
            if is_stdlib_crate(crate_name) {
                Vec::new()
            } else {
                vec![rustc_root.clone()]
            }
        } else {
            let mut m = if is_stdlib_crate(crate_name) {
                Vec::new()
            } else {
                vec![rustc_root.clone()]
            };
            for p in &segs[..segs.len() - 2] {
                m.push(rustc_crate_name(p));
            }
            m
        };
        let url_str = if is_stdlib_crate(crate_name) {
            let channel = stdlib_channel(version);
            if mod_parts.is_empty() {
                format!(
                    "{origin}/{channel}/{crate_name}/{}.{}.html#method.{method_name}",
                    parent_kind.file_prefix(),
                    parent_name
                )
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html#method.{method_name}",
                    mod_parts.join("/"),
                    parent_kind.file_prefix(),
                    parent_name
                )
            }
        } else if mod_parts.is_empty() {
            format!(
                "{origin}/{crate_name}/{version}/{}/{}.{}.html#method.{method_name}",
                rustc_root,
                parent_kind.file_prefix(),
                parent_name
            )
        } else {
            format!(
                "{origin}/{crate_name}/{version}/{}/{}.{}.html#method.{method_name}",
                mod_parts.join("/"),
                parent_kind.file_prefix(),
                parent_name
            )
        };
        return Url::parse(&url_str)
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid get-item URL", e));
    }

    let url_str = if is_stdlib_crate(crate_name) {
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

#[cfg_attr(not(test), allow(dead_code))]
fn extract_resolved_version(final_url: &Url, requested: &str) -> Option<String> {
    extract_resolved_version_for_crate(final_url, requested, None)
}

/// Resolve version/channel from the final URL, aware of docs.rs vs stdlib layout.
///
/// - docs.rs: `/{pkg}/{version}/…` — second path segment is the version
/// - doc.rust-lang.org: `/{channel}/{crate}/…` — first path segment is channel/version
fn extract_resolved_version_for_crate(
    final_url: &Url,
    requested: &str,
    crate_name: Option<&str>,
) -> Option<String> {
    let host = final_url.host_str().unwrap_or("");
    let segs: Vec<&str> = final_url
        .path_segments()?
        .filter(|s| !s.is_empty())
        .collect();
    if segs.is_empty() {
        return None;
    }

    let is_stdlib_host = host.eq_ignore_ascii_case(HOST_DOC_RUST_LANG_ORG)
        || crate_name.is_some_and(is_stdlib_crate)
        || segs
            .get(1)
            .is_some_and(|s| matches!(*s, "std" | "core" | "alloc"));

    if is_stdlib_host {
        // /{channel}/{crate}/… — never emit crate name as version
        let channel = segs.first()?;
        if matches!(*channel, "std" | "core" | "alloc") {
            return None;
        }
        return Some((*channel).to_string());
    }

    // docs.rs: /{pkg}/{version}/…
    let ver = segs.get(1)?;
    if *ver == "latest" {
        return None;
    }
    if requested == "latest" || *ver != requested {
        return Some((*ver).to_string());
    }
    Some((*ver).to_string())
}

/// Try to scrape a concrete SemVer from docs.rs HTML when the URL still says `/latest/`.
///
/// Only accepts versions that belong to `crate_name` (never a dependency hit).
fn scrape_docs_rs_version_from_html(html: &str, crate_name: &str) -> Option<String> {
    let crate_name = crate_name.trim();
    if crate_name.is_empty() || is_stdlib_crate(crate_name) {
        return None;
    }
    // Crate names are already validated (`[A-Za-z][A-Za-z0-9_-]*`); escape for safety.
    let escaped = regex::escape(crate_name);
    // Prefer paths for THIS crate: `/tokio/1.53.0/` or `docs.rs/tokio/1.53.0`
    let re_path = Regex::new(&format!(
        r"(?i)(?:https?://docs\.rs/|/){escaped}/([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)\b"
    ))
    .ok()?;
    if let Some(c) = re_path.captures(html) {
        return c.get(1).map(|m| m.as_str().to_string());
    }
    // Title may include "crate version" on some rustdoc skins
    let re_title = Regex::new(&format!(
        r"(?i)<title>[^<]*?\b{escaped}\b[^<]*?\b([0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?)\b"
    ))
    .ok()?;
    re_title
        .captures(html)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
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
    let md = htmd::convert(&cleaned).map_err(|e| {
        AppError::new(
            ErrorKind::Parse,
            format!("HTML to Markdown conversion failed: {e}"),
        )
    })?;
    Ok(scrub_rustdoc_chrome(&md))
}

/// Strip rustdoc UI chrome that pollutes LLM context (`§`, "Copy item path", …).
pub fn scrub_rustdoc_chrome(md: &str) -> String {
    let mut out = md.to_string();
    // Common rustdoc heading anchor glyph left by htmd.
    out = out.replace('§', "");
    // "Copy item path" UI control sometimes survives as text.
    out = out.replace("Copy item path", "");
    // Collapse accidental double spaces left after removals (not full reflow).
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out
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
    let mut resolved =
        extract_resolved_version_for_crate(&resp.final_url, version, Some(crate_name));
    if resolved.is_none() {
        // Body still available only as markdown path; re-decode for scrape when latest.
        if let Ok(text) = decode_utf8(&resp.body) {
            resolved = scrape_docs_rs_version_from_html(&text, crate_name);
        }
    }
    Ok(ReadmeData {
        crate_name: crate_name.to_string(),
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
        cache_hit: resp.cache_hit,
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
/// For associated methods, tries parent type pages (struct → enum → trait → …)
/// until one returns HTTP 200.
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
    // Strip crate prefix the same way URL builder does for method detection.
    let rustc_root = rustc_crate_name(crate_name);
    let segs_for_detect: Vec<String> = {
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

    if is_method_path(kind, &segs_for_detect) {
        let mut last_err: Option<AppError> = None;
        for parent_kind in METHOD_PARENT_KINDS {
            let url = get_item_url_on_origin_with_parent_kind(
                origin,
                crate_name,
                version,
                kind,
                segments,
                Some(*parent_kind),
            )?;
            match fetch_item_at(http, crate_name, version, kind, segments, &url).await {
                Ok(data) => return Ok(data),
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    last_err = Some(e);
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        return Err(last_err.unwrap_or_else(|| {
            AppError::new(ErrorKind::NotFound, "method parent type page not found")
        }));
    }

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
    // HTTP does not send fragments; strip for request, keep for source_url.
    let mut fetch_url = url.clone();
    let fragment = fetch_url.fragment().map(str::to_string);
    fetch_url.set_fragment(None);

    let resp = http.get_html(&fetch_url).await?;
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

    let method_anchor = fragment
        .as_deref()
        .and_then(|f| f.strip_prefix("method."))
        .map(str::to_string);
    let body = resp.body.clone();
    let method_anchor_cpu = method_anchor.clone();
    let ((markdown, empty), extraction) = http
        .budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            if let Some(ref m) = method_anchor_cpu {
                let (md, empty, scope) = extract_method_markdown_scoped(&text, m)?;
                Ok(((md, empty), Some(scope.to_string())))
            } else {
                let (md, empty) = extract_item_markdown_from_html(&text)?;
                Ok(((md, empty), None))
            }
        })
        .await?;
    let item_path = segments.join("::");
    let item_name = segments
        .last()
        .cloned()
        .unwrap_or_else(|| item_path.clone());
    let title = format!("{item_path} ({})", kind.as_str());
    let mut resolved =
        extract_resolved_version_for_crate(&resp.final_url, version, Some(crate_name));
    if resolved.is_none() {
        if let Ok(text) = decode_utf8(&resp.body) {
            resolved = scrape_docs_rs_version_from_html(&text, crate_name);
        }
    }

    let mut source_url = resp.final_url.clone();
    if let Some(f) = fragment {
        source_url.set_fragment(Some(&f));
    }

    Ok(GetItemData {
        crate_name: crate_name.to_string(),
        item_type: kind.as_str().to_string(),
        item_path,
        item_name,
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: source_url.to_string(),
        title,
        cache_hit: resp.cache_hit,
        extraction,
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
    match_mode: MatchMode,
) -> AppResult<SearchInCrateData> {
    search_in_crate_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
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
#[allow(clippy::too_many_arguments)]
pub async fn search_in_crate_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
    query: &str,
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
#[allow(clippy::too_many_arguments)]
pub async fn search_in_crate_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    query: &str,
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
            "docs.rs all.html",
        ));
    }

    let body = resp.body.clone();
    let source_url = resp.final_url.to_string();
    let cache_hit = resp.cache_hit;
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
                match_mode,
                &source_url,
                cache_hit,
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
/// One classified hit candidate before dedup/limit: (index, name, kind, url, score).
type ClassifiedHit = (usize, String, ItemKind, String, u8);

/// Pure parse of all.html body for offline tests and CPU workers.
///
/// Large candidate lists use `rayon`; small lists stay sequential.
/// Results are scored and sorted (exact leaf first) before applying `limit`.
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
    match_mode: MatchMode,
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
        return filter_hits_sequential(candidates, &base, &q, item_type, limit, match_mode);
    }
    filter_hits_parallel(candidates, &base, &q, item_type, limit, match_mode)
}

fn filter_hits_sequential(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
    match_mode: MatchMode,
) -> AppResult<Vec<SearchInCrateHit>> {
    let mut rows: Vec<ClassifiedHit> = Vec::new();
    for (idx, (name, href)) in candidates.into_iter().enumerate() {
        if let Some(hit) = classify_hit_row(idx, name, &href, base, q, item_type, match_mode)? {
            rows.push(hit);
        }
    }
    finalize_hits(rows, limit)
}

fn filter_hits_parallel(
    candidates: Vec<(String, String)>,
    base: &Url,
    q: &str,
    item_type: Option<ItemKind>,
    limit: usize,
    match_mode: MatchMode,
) -> AppResult<Vec<SearchInCrateHit>> {
    let mapped: AppResult<Vec<Option<ClassifiedHit>>> = candidates
        .into_par_iter()
        .enumerate()
        .map(|(idx, (name, href))| {
            classify_hit_row(idx, name, &href, base, q, item_type, match_mode)
        })
        .collect();
    let rows: Vec<ClassifiedHit> = mapped?.into_iter().flatten().collect();
    finalize_hits(rows, limit)
}

fn finalize_hits(mut rows: Vec<ClassifiedHit>, limit: usize) -> AppResult<Vec<SearchInCrateHit>> {
    // Score ascending, then original document order for stability.
    rows.sort_by(|a, b| a.4.cmp(&b.4).then_with(|| a.0.cmp(&b.0)));
    let mut hits = Vec::with_capacity(limit.min(256));
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

fn classify_hit_row(
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
    let abs = join_href(base, href)?;
    Ok(Some((idx, name, kind, abs, score)))
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

/// Extract markdown for a single associated method by rustdoc `id="method.X"`.
///
/// Locates the method anchor and prefers the enclosing `details.method-toggle`
/// (signature + docblock). Falls back to the full item page when the anchor is
/// missing (`extraction` = `item_page`).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_from_html(
    html: &str,
    method_name: &str,
) -> AppResult<(String, bool)> {
    let (md, empty, _) = extract_method_markdown_scoped(html, method_name)?;
    Ok((md, empty))
}

/// Like [`extract_method_markdown_from_html`] but reports extraction scope.
///
/// Returns `(markdown, empty, scope)` where `scope` is `"method"` or `"item_page"`.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_scoped(
    html: &str,
    method_name: &str,
) -> AppResult<(String, bool, &'static str)> {
    let want_id = format!("method.{method_name}");
    let document = Html::parse_document(html);
    // Attribute selector avoids CSS id issues with dots in method.X.
    let sel = match Selector::parse(&format!(r#"[id="{want_id}"]"#)) {
        Ok(s) => s,
        Err(_) => {
            let (md, empty) = extract_item_markdown_from_html(html)?;
            return Ok((md, empty, "item_page"));
        }
    };
    if let Some(anchor) = document.select(&sel).next() {
        let frag_html = method_container_html(anchor);
        let md = html_to_markdown(&frag_html)?;
        let empty = md.trim().is_empty();
        if !empty {
            return Ok((md, empty, "method"));
        }
    }
    let (md, empty) = extract_item_markdown_from_html(html)?;
    Ok((md, empty, "item_page"))
}

/// Prefer the enclosing rustdoc `details.method-toggle` (signature + docs).
fn method_container_html(anchor: scraper::ElementRef<'_>) -> String {
    let mut node = anchor.parent();
    while let Some(n) = node {
        if let Some(el) = scraper::ElementRef::wrap(n) {
            if el.value().name() == "details" {
                return el.html();
            }
            node = el.parent();
        } else {
            break;
        }
    }
    // Fall back to the section/element that carries the method id.
    anchor.html()
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
#[allow(clippy::too_many_arguments)]
pub fn search_in_crate_from_html(
    html: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
    match_mode: MatchMode,
    source_url: &str,
    cache_hit: bool,
) -> AppResult<SearchInCrateData> {
    let limit = (limit as usize).min(MAX_SEARCH_IN_CRATE_LIMIT as usize);
    let all = parse_all_html_hits(
        html,
        crate_name,
        version,
        query,
        item_type,
        usize::MAX,
        match_mode,
    )?;
    let total = all.len();
    let hits: Vec<_> = all.into_iter().take(limit).collect();
    let emitted = hits.len();
    // Empty query: scores are 0 — omit score noise on wire for cleaner lists.
    let hits = if query.trim().is_empty() {
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
        crate_name: crate_name.to_string(),
        query: query.to_string(),
        version: version.to_string(),
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
        let hits = parse_all_html_hits(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            100,
            crate::domain::MatchMode::Prefix,
        )
        .unwrap();
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
            MatchMode::Prefix,
            "https://docs.rs/demo/1.0.0/demo/all.html",
            false,
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
            MatchMode::Prefix,
            "https://docs.rs/demo/1.0.0/demo/all.html",
            false,
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
            MatchMode::Prefix,
            "https://docs.rs/demo/1.0.0/demo/all.html",
            false,
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
            MatchMode::Prefix,
            "https://docs.rs/demo/1.0.0/demo/all.html",
            false,
        )
        .unwrap();
        assert!(data.total > 0, "fixture must have hits for total");
        assert_eq!(data.emitted, 0);
        assert!(data.hits.is_empty());
        assert!(data.truncated);
    }

    #[test]
    fn serialize_exact_match_beats_deserializer() {
        let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="trait.Deserialize.html">Deserialize</a>
          <a href="trait.Serialize.html">Serialize</a>
          <a href="struct.Deserializer.html">Deserializer</a>
        </div></body></html>"#;
        let hits = parse_all_html_hits(
            html,
            "serde",
            "1.0.0",
            "Serialize",
            Some(ItemKind::Trait),
            10,
            MatchMode::Prefix,
        )
        .unwrap();
        assert_eq!(hits[0].name, "Serialize");
        assert!(hits.iter().all(|h| h.name == "Serialize"));
    }

    #[test]
    fn method_url_uses_anchor() {
        let segs = vec!["runtime".into(), "Runtime".into(), "new".into()];
        let u = get_item_url("tokio", "latest", ItemKind::Fn, &segs).unwrap();
        assert!(
            u.as_str().contains("struct.Runtime.html#method.new"),
            "url={u}"
        );
    }

    #[test]
    fn stdlib_resolved_version_is_channel() {
        let u = Url::parse("https://doc.rust-lang.org/stable/std/index.html").unwrap();
        assert_eq!(
            extract_resolved_version_for_crate(&u, "latest", Some("std")).as_deref(),
            Some("stable")
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
    fn scrape_version_prefers_target_crate_not_deps() {
        let html = r#"
        <title>tokio - Rust</title>
        <a href="https://docs.rs/bytes/1.2.0/bytes/">bytes</a>
        <a href="/tokio/1.53.0/tokio/runtime/index.html">runtime</a>
        <a href="https://docs.rs/mio/0.8.0/mio/">mio</a>
        "#;
        assert_eq!(
            scrape_docs_rs_version_from_html(html, "tokio").as_deref(),
            Some("1.53.0")
        );
        // Must not pick dependency when crate path missing
        let html2 = r#"<title>x</title><a href="https://docs.rs/bytes/1.2.0/bytes/">b</a>"#;
        assert_eq!(scrape_docs_rs_version_from_html(html2, "tokio"), None);
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

    #[test]
    fn method_extract_scopes_to_details_not_full_page() {
        let html = include_str!("../tests/fixtures/docs_rs/method_runtime_new.html");
        let (md, empty, scope) = extract_method_markdown_scoped(html, "new").unwrap();
        assert!(!empty);
        assert_eq!(scope, "method");
        assert!(
            !md.contains("parent page noise"),
            "must not dump full parent page: {md}"
        );
        assert!(
            md.to_ascii_lowercase().contains("new") || md.contains("Creates a new runtime"),
            "expected method content: {md}"
        );
        assert!(!md.contains('§'));
        assert!(!md.contains("Copy item path"));
    }

    #[test]
    fn scrub_rustdoc_chrome_strips_section_sign() {
        let dirty = "## [§](#serde)Serde\nCopy item path\nbody";
        let clean = scrub_rustdoc_chrome(dirty);
        assert!(!clean.contains('§'));
        assert!(!clean.contains("Copy item path"));
        assert!(clean.contains("Serde"));
    }

    #[test]
    fn method_missing_anchor_falls_back_item_page() {
        let html = r#"<html><body><div id="main-content"><h1>Struct Runtime</h1><p>only parent</p></div></body></html>"#;
        let (md, empty, scope) = extract_method_markdown_scoped(html, "new").unwrap();
        assert_eq!(scope, "item_page");
        assert!(!empty);
        assert!(md.contains("Runtime") || md.contains("parent"));
    }
}
