//! Security scrub (`on*` handlers, `javascript:` URLs, script/style) + HTML→Markdown.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;
use scraper::{Html, Selector};

use crate::domain::compile_bounded_regex;
use crate::error::{AppError, AppResult, ErrorDetail};

static SEL_DROP_SCRIPT_STYLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse("script, style, noscript")
        .expect("hardcoded scraper selector 'script, style, noscript' is valid by construction")
});

/// Compiled once: strip `on*` event handlers from HTML fragments (hot path for every HTML→MD).
pub(super) static RE_ON_HANDLERS: LazyLock<Regex> = LazyLock::new(|| {
    compile_bounded_regex(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#)
        .expect("hardcoded on* handler strip regex is valid by construction")
});

/// Compiled once: strip `javascript:` URLs from href/src attributes.
pub(super) static RE_JS_URLS: LazyLock<Regex> = LazyLock::new(|| {
    compile_bounded_regex(r#"(?i)(href|src)\s*=\s*["']\s*javascript:[^"']*["']"#)
        .expect("hardcoded javascript: URL strip regex is valid by construction")
});

/// Compiled once: literal prescan for the tags [`SEL_DROP_SCRIPT_STYLE`] removes.
///
/// Deliberately looser than the selector (no end-of-name anchor) so it can only
/// over-report, never miss a real start tag.
static RE_DROP_TAG_PROBE: LazyLock<Regex> = LazyLock::new(|| {
    compile_bounded_regex(r"(?i)<(?:script|style|noscript)")
        .expect("hardcoded drop-tag probe regex is valid by construction")
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
    // Cheap literal prescan before the DOM build: html5ever never invents a
    // script/style/noscript element, so no start-tag substring means no drop
    // target and the whole `parse_fragment` pass is dead work. A false positive
    // (the literal inside text or an attribute) only costs the parse we already
    // paid for unconditionally, so the fast path is fail-safe (SCRAPE-S-006).
    let mut drop_html: Vec<String> = if RE_DROP_TAG_PROBE.is_match(html) {
        let fragment = Html::parse_fragment(html);
        // Collect serialized drop-targets once, longest first so nested matches do
        // not leave residuals, then strip from a single working buffer (SCRAPE-Q-003).
        fragment
            .select(&SEL_DROP_SCRIPT_STYLE)
            .map(|el| el.html())
            .collect()
    } else {
        Vec::new()
    };
    drop_html.sort_by_key(|s| std::cmp::Reverse(s.len()));
    // SCRAPE-S-006: no script/style → scrub hygiene only (no N× replace allocs).
    let out: Cow<'_, str> = if drop_html.is_empty() {
        Cow::Borrowed(html)
    } else {
        let mut buf = String::with_capacity(html.len());
        buf.push_str(html);
        for tag_html in drop_html {
            if !tag_html.is_empty() {
                buf = buf.replace(&tag_html, "");
            }
        }
        Cow::Owned(buf)
    };
    // Strip event handlers: onload=, onclick=, etc.
    let out = scrub_pattern(&RE_ON_HANDLERS, out);
    // Drop javascript: URLs in href/src
    let out = scrub_pattern(&RE_JS_URLS, out);
    out.into_owned()
}

/// Delete every `re` match from `input`, keeping the borrow when nothing matches.
///
/// `Regex::replace_all(..).into_owned()` copies the whole body even on a clean
/// document, so a script-free 10 MiB page paid one full copy per hygiene regex.
/// Probing with `is_match` first keeps the untouched case allocation-free; the
/// extra scan only runs when a rewrite was going to allocate anyway.
pub(super) fn scrub_pattern<'a>(re: &Regex, input: Cow<'a, str>) -> Cow<'a, str> {
    if re.is_match(&input) {
        Cow::Owned(re.replace_all(&input, "").into_owned())
    } else {
        input
    }
}

pub(super) fn html_to_markdown(html: &str) -> AppResult<String> {
    let cleaned = sanitize_html_fragment(html);
    // Cause stays in `Error::source` — never embed `{e}` in Display (ADR 0002 / ERR-O-001).
    let md = htmd::convert(&cleaned)
        .map_err(|e| AppError::of_with_source(ErrorDetail::HtmlToMarkdown, e))?;
    Ok(scrub_rustdoc_chrome(&md))
}

/// Strip rustdoc UI chrome that pollutes LLM context (`§`, "Copy item path", …).
pub fn scrub_rustdoc_chrome(md: &str) -> String {
    // `str::replace` allocates even when the needle is absent. Probe first and
    // keep the borrow, preserving the original strip order (`§` before the
    // label) so a `Copy§ item path` artefact still collapses exactly as before.
    let stripped = strip_literal(strip_literal(Cow::Borrowed(md), "§"), "Copy item path");
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

/// Delete every occurrence of `needle`, keeping the borrow when it is absent.
fn strip_literal<'a>(input: Cow<'a, str>, needle: &str) -> Cow<'a, str> {
    if input.contains(needle) {
        Cow::Owned(input.replace(needle, ""))
    } else {
        input
    }
}
