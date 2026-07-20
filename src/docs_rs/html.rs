//! HTML sanitize, version scrape, and markdown extraction (CPU-bound pure paths).
//!
//! Runs under `spawn_blocking` via fetch/search callers — no network here.
//!
//! Field extraction uses `scraper` CSS selectors + typed path segments — never
//! regex over the document for content fields. Process-static regexes below are
//! **security scrub only** (`on*` handlers / `javascript:` URLs).
//!
//! Fixed CSS selectors are compiled once via [`LazyLock`] (SCRAPE-R-004) so the
//! one-shot CPU path does not re-parse the same selector strings on every page.

use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};
use url::Url;

use crate::config::{HOST_DOC_RUST_LANG_ORG, is_stdlib_crate};
use crate::domain::compile_bounded_regex;
use crate::error::{AppError, AppResult, ErrorKind};

/// Process-static CSS selectors (valid by construction — panic on init is a bug).
static SEL_A_HREF: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a[href]").expect("hardcoded scraper selector 'a[href]' is valid by construction")
});
static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("title").expect("hardcoded scraper selector 'title' is valid by construction")
});
static SEL_DROP_SCRIPT_STYLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script, style, noscript")
        .expect("hardcoded scraper selector 'script, style, noscript' is valid by construction")
});
static SEL_RUSTDOC_DOCBLOCK: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".rustdoc .docblock")
        .expect("hardcoded scraper selector '.rustdoc .docblock' is valid by construction")
});
static SEL_MAIN_DOCBLOCK: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("#main-content .docblock")
        .expect("hardcoded scraper selector '#main-content .docblock' is valid by construction")
});
static SEL_DOCBLOCK: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".docblock")
        .expect("hardcoded scraper selector '.docblock' is valid by construction")
});
static SEL_RUSTDOC_MAIN_ITEM_DECL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".rustdoc-main .item-decl")
        .expect("hardcoded scraper selector '.rustdoc-main .item-decl' is valid by construction")
});
static SEL_ITEM_DECL: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".item-decl")
        .expect("hardcoded scraper selector '.item-decl' is valid by construction")
});
static SEL_MAIN_CONTENT: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("#main-content")
        .expect("hardcoded scraper selector '#main-content' is valid by construction")
});
/// Any element with an `id` attribute — used for method anchors without dynamic CSS (SCRAPE-S-002).
static SEL_WITH_ID: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("[id]").expect("hardcoded scraper selector '[id]' is valid by construction")
});

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn extract_resolved_version(final_url: &Url, requested: &str) -> Option<String> {
    extract_resolved_version_for_crate(final_url, requested, None)
}

/// Resolve version/channel from the final URL, aware of docs.rs vs stdlib layout.
///
/// - docs.rs: `/{pkg}/{version}/…` — second path segment is the version
/// - doc.rust-lang.org: `/{channel}/{crate}/…` — first path segment is channel/version
pub(super) fn extract_resolved_version_for_crate(
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
/// Uses `scraper` + path-segment scan — **no** regex field extraction (SCRAPE-Q-002).
/// Production fetch paths use [`scrape_docs_rs_version_from_document`] after a single
/// parse; this wrapper remains for unit tests and offline callers.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn scrape_docs_rs_version_from_html(html: &str, crate_name: &str) -> Option<String> {
    let document = Html::parse_document(html);
    scrape_docs_rs_version_from_document(&document, crate_name)
}

/// Version scrape against an already-parsed document (SCRAPE-S-003: one DOM per body).
pub(super) fn scrape_docs_rs_version_from_document(
    document: &Html,
    crate_name: &str,
) -> Option<String> {
    let crate_name = crate_name.trim();
    if crate_name.is_empty() || is_stdlib_crate(crate_name) {
        return None;
    }
    for a in document.select(&SEL_A_HREF) {
        if let Some(href) = a.value().attr("href")
            && let Some(ver) = version_from_docs_rs_href(href, crate_name)
        {
            return Some(ver);
        }
    }
    // Title text fallback (rustdoc skins sometimes embed "crate version").
    if let Some(title) = document.select(&SEL_TITLE).next() {
        let text: String = title.text().collect();
        if let Some(ver) = version_token_near_crate_in_text(&text, crate_name) {
            return Some(ver);
        }
    }
    None
}

/// Extract `/{crate}/{semver}/` from an href (absolute or path-absolute/relative).
fn version_from_docs_rs_href(href: &str, crate_name: &str) -> Option<String> {
    let href = href.trim();
    if href.is_empty() {
        return None;
    }
    // Prefer Url path_segments when the href is absolute; otherwise scan path pieces.
    let path = if href.starts_with("http://") || href.starts_with("https://") {
        Url::parse(href).ok()?.path().to_string()
    } else {
        // Strip query/fragment for path-only hrefs.
        let path = href.split('#').next().unwrap_or(href);
        path.split('?').next().unwrap_or(path).to_string()
    };
    let segs: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    for i in 0..segs.len().saturating_sub(1) {
        if segs[i].eq_ignore_ascii_case(crate_name)
            && let Some(ver) = segs.get(i + 1)
            && looks_like_semver(ver)
        {
            return Some((*ver).to_string());
        }
    }
    None
}

/// True when `s` looks like SemVer core (`X.Y.Z` optional pre-release), no `v` prefix.
fn looks_like_semver(s: &str) -> bool {
    if s.is_empty() || s.starts_with('v') || s.starts_with('V') || s.contains('+') {
        return false;
    }
    let (core, pre) = match s.split_once('-') {
        Some((c, p)) => (c, Some(p)),
        None => (s, None),
    };
    let mut parts = core.split('.');
    let (Some(a), Some(b), Some(c), None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    if !a.bytes().all(|b| b.is_ascii_digit())
        || !b.bytes().all(|b| b.is_ascii_digit())
        || !c.bytes().all(|b| b.is_ascii_digit())
        || a.is_empty()
        || b.is_empty()
        || c.is_empty()
    {
        return false;
    }
    if let Some(pre) = pre {
        if pre.is_empty() {
            return false;
        }
        return pre
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-');
    }
    true
}

/// Scan title-like free text for `crate_name` then a nearby SemVer token.
fn version_token_near_crate_in_text(text: &str, crate_name: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let crate_l = crate_name.to_ascii_lowercase();
    let idx = lower.find(&crate_l)?;
    let after = &text[idx + crate_name.len()..];
    for token in after.split(|c: char| !(c.is_ascii_alphanumeric() || c == '.' || c == '-')) {
        if looks_like_semver(token) {
            return Some(token.to_string());
        }
    }
    None
}

/// Compiled once: strip `on*` event handlers from HTML fragments (hot path for every HTML→MD).
static RE_ON_HANDLERS: LazyLock<Regex> = LazyLock::new(|| {
    compile_bounded_regex(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
        .expect("hardcoded on* handler strip regex is valid by construction")
});

/// Compiled once: strip `javascript:` URLs from href/src attributes.
static RE_JS_URLS: LazyLock<Regex> = LazyLock::new(|| {
    compile_bounded_regex(r#"(?i)(href|src)\s*=\s*["']\s*javascript:[^"']*["']"#)
        .expect("hardcoded javascript: URL strip regex is valid by construction")
});

/// Remove script/style nodes and on* handlers before Markdown conversion.
///
/// Extraction of doc content uses CSS selectors elsewhere in this module.
/// Regexes here are **security scrub only** (`on*` / `javascript:`), compiled
/// once via process-static [`LazyLock`] — not used to scrape fields from HTML.
///
/// Precondition: `html` is already body-capped (`HARD_MAX_BODY_BYTES`) and resident in RAM.
///
/// When there are no script/style/noscript nodes, skips the multi-replace loop
/// and only runs the process-static hygiene regexes (SCRAPE-S-006).
pub fn sanitize_html_fragment(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    // Collect serialized drop-targets once, longest first so nested matches do not
    // leave residuals, then strip from a single working buffer (SCRAPE-Q-003).
    let mut drop_html: Vec<String> = fragment
        .select(&SEL_DROP_SCRIPT_STYLE)
        .map(|el| el.html())
        .collect();
    drop_html.sort_by_key(|s| std::cmp::Reverse(s.len()));
    // SCRAPE-S-006: no script/style → scrub hygiene only (no N× replace allocs).
    let mut out = if drop_html.is_empty() {
        html.to_string()
    } else {
        let mut buf = String::with_capacity(html.len());
        buf.push_str(html);
        for tag_html in drop_html {
            if !tag_html.is_empty() {
                buf = buf.replace(&tag_html, "");
            }
        }
        buf
    };
    // Strip event handlers: onload=, onclick=, etc.
    out = RE_ON_HANDLERS.replace_all(&out, "").into_owned();
    // Drop javascript: URLs in href/src
    out = RE_JS_URLS.replace_all(&out, "").into_owned();
    out
}

pub(super) fn html_to_markdown(html: &str) -> AppResult<String> {
    let cleaned = sanitize_html_fragment(html);
    // Cause stays in `Error::source` — never embed `{e}` in Display (ADR 0002 / ERR-O-001).
    let md = htmd::convert(&cleaned).map_err(|e| {
        AppError::with_source(ErrorKind::Parse, "html to markdown conversion failed", e)
    })?;
    Ok(scrub_rustdoc_chrome(&md))
}

/// Strip rustdoc UI chrome that pollutes LLM context (`§`, "Copy item path", …).
pub fn scrub_rustdoc_chrome(md: &str) -> String {
    let stripped = md.replace('§', "").replace("Copy item path", "");
    // Single-pass collapse of accidental double spaces (not full reflow).
    let mut out = String::with_capacity(stripped.len());
    let mut prev_space = false;
    for c in stripped.chars() {
        if c == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
            out.push(' ');
        } else {
            prev_space = false;
            out.push(c);
        }
    }
    out
}

/// First non-empty `inner_html` among precompiled selectors (SCRAPE-R-004).
pub(super) fn first_inner_html(document: &Html, selectors: &[&Selector]) -> Option<String> {
    for sel in selectors {
        if let Some(n) = document.select(sel).next() {
            let inner = n.inner_html();
            if !inner.trim().is_empty() {
                return Some(inner);
            }
        }
    }
    None
}

/// Extract readme markdown from an already-parsed document (SCRAPE-S-003).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_readme_markdown_from_document(document: &Html) -> AppResult<(String, bool)> {
    let content = first_inner_html(
        document,
        &[
            &SEL_RUSTDOC_DOCBLOCK,
            &SEL_MAIN_DOCBLOCK,
            &SEL_DOCBLOCK,
            &SEL_RUSTDOC_MAIN_ITEM_DECL,
            &SEL_ITEM_DECL,
        ],
    );
    match content {
        Some(h) => Ok((html_to_markdown(&h)?, false)),
        None => Ok((String::new(), true)),
    }
}

/// Extract readme markdown from raw HTML (offline tests / pure path).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_readme_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    extract_readme_markdown_from_document(&Html::parse_document(html))
}

/// Extract get-item markdown from an already-parsed document (SCRAPE-S-003).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_item_markdown_from_document(document: &Html) -> AppResult<(String, bool)> {
    let content = first_inner_html(
        document,
        &[&SEL_MAIN_CONTENT, &SEL_ITEM_DECL, &SEL_DOCBLOCK],
    )
    .unwrap_or_default();
    let empty = content.trim().is_empty();
    if empty {
        Ok((String::new(), true))
    } else {
        Ok((html_to_markdown(&content)?, false))
    }
}

/// Extract get-item markdown from raw HTML (offline tests / pure path).
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_item_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    extract_item_markdown_from_document(&Html::parse_document(html))
}

/// Extract markdown for a single associated method by rustdoc `id="method.X"`.
///
/// Locates the method anchor and prefers the enclosing `details.method-toggle`
/// (signature + docblock). **Fail-closed:** missing or empty anchors return
/// [`ErrorKind::NotFound`] — never the full parent page as a method success
/// (Camada Y / GAP-W-001).
///
/// # Errors
///
/// Returns [`ErrorKind::NotFound`] when the method anchor is absent or empty.
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
/// On success `scope` is always `"method"`. Missing anchors are errors, not
/// `item_page` success payloads.
///
/// # Errors
///
/// Returns [`ErrorKind::NotFound`] when the method anchor is absent or empty.
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_scoped(
    html: &str,
    method_name: &str,
) -> AppResult<(String, bool, &'static str)> {
    extract_method_markdown_scoped_from_document(&Html::parse_document(html), method_name)
}

/// Method extract against an already-parsed document (SCRAPE-S-002/S-003).
///
/// Matches `id="method.{name}"` via static `[id]` selector + attribute equality
/// — never builds a dynamic CSS selector string.
///
/// # Errors
///
/// Returns [`ErrorKind::NotFound`] when the method anchor is absent or empty.
/// Returns [`ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_scoped_from_document(
    document: &Html,
    method_name: &str,
) -> AppResult<(String, bool, &'static str)> {
    let want_id = format!("method.{method_name}");
    for el in document.select(&SEL_WITH_ID) {
        if el.value().attr("id") == Some(want_id.as_str()) {
            let frag_html = method_container_html(el);
            let md = html_to_markdown(&frag_html)?;
            let empty = md.trim().is_empty();
            if empty {
                return Err(AppError::new(
                    ErrorKind::NotFound,
                    format!("method anchor empty: method.{method_name}"),
                ));
            }
            return Ok((md, empty, "method"));
        }
    }
    // Fail-closed: parent page HTTP 200 is not a method hit (GAP-W-001 / X-001).
    Err(AppError::new(
        ErrorKind::NotFound,
        format!("method anchor not found: method.{method_name}"),
    ))
}

/// List rustdoc associated-method anchor names (`id="method.X"` → `X`).
///
/// Used by `--suggest` on method miss (one DOM pass, no network). Pre-sizes the
/// output vec from a first count-free push path with a modest capacity hint.
pub fn list_method_anchor_names(document: &Html) -> Vec<String> {
    let mut out = Vec::with_capacity(32);
    for el in document.select(&SEL_WITH_ID) {
        if let Some(id) = el.value().attr("id") {
            if let Some(name) = id.strip_prefix("method.") {
                if !name.is_empty() {
                    out.push(name.to_string());
                }
            }
        }
    }
    out
}

/// Parse HTML and list method anchor names (CPU-bound helper for ops recovery).
pub fn list_method_anchor_names_from_html(html: &str) -> Vec<String> {
    list_method_anchor_names(&Html::parse_document(html))
}

/// Prefer the enclosing rustdoc `details.method-toggle` (signature + docs).
pub(super) fn method_container_html(anchor: scraper::ElementRef<'_>) -> String {
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
