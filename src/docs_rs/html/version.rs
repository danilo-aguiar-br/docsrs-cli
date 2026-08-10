//! Resolved version detection: URL layout, docs.rs hrefs, and title fallback.

use std::sync::LazyLock;

use scraper::{Html, Selector};
use url::Url;

use crate::config::{HOST_DOC_RUST_LANG_ORG, is_stdlib_crate};

/// Process-static CSS selectors (valid by construction — panic on init is a bug).
static SEL_A_HREF: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("a[href]")
        .expect("hardcoded scraper selector 'a[href]' is valid by construction")
});
static SEL_TITLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("title").expect("hardcoded scraper selector 'title' is valid by construction")
});

#[cfg_attr(not(test), allow(dead_code))]
pub(in crate::docs_rs) fn extract_resolved_version(
    final_url: &Url,
    requested: &str,
) -> Option<String> {
    extract_resolved_version_for_crate(final_url, requested, None)
}

/// Resolve version/channel from the final URL, aware of docs.rs vs stdlib layout.
///
/// - docs.rs: `/{pkg}/{version}/…` — second path segment is the version
/// - doc.rust-lang.org: `/{channel}/{crate}/…` — first path segment is channel/version
pub(in crate::docs_rs) fn extract_resolved_version_for_crate(
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
pub(in crate::docs_rs) fn scrape_docs_rs_version_from_html(
    html: &str,
    crate_name: &str,
) -> Option<String> {
    let document = Html::parse_document(html);
    scrape_docs_rs_version_from_document(&document, crate_name)
}

/// Version scrape against an already-parsed document (SCRAPE-S-003: one DOM per body).
pub(in crate::docs_rs) fn scrape_docs_rs_version_from_document(
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
    let (Some(a), Some(b), Some(c), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
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
