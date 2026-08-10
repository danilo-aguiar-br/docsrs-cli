//! Docblock, item, and associated-item markdown extraction from rustdoc HTML.

use std::sync::LazyLock;

use scraper::{Html, Selector};

use crate::docs_rs::assoc::{AssocAnchorKind, METHOD_ANCHOR_PREFIXES, strip_assoc_anchor_prefix};
use crate::error::{AppError, AppResult, ErrorDetail};

use super::sanitize::html_to_markdown;

/// Process-static CSS selectors (valid by construction — panic on init is a bug).
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

/// First non-empty `inner_html` among precompiled selectors (SCRAPE-R-004).
fn first_inner_html(document: &Html, selectors: &[&Selector]) -> Option<String> {
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
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
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
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_readme_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    extract_readme_markdown_from_document(&Html::parse_document(html))
}

/// Extract get-item markdown from an already-parsed document (SCRAPE-S-003).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
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
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_item_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    extract_item_markdown_from_document(&Html::parse_document(html))
}

/// Extract markdown for a single associated method by rustdoc `id="method.X"`.
///
/// Locates the method anchor and prefers the enclosing `details.method-toggle`
/// (signature + docblock). **Fail-closed:** missing or empty anchors return
/// [`crate::error::ErrorKind::NotFound`] — never the full parent page as a method success
/// (Camada Y / GAP-W-001).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::NotFound`] when the method anchor is absent or empty.
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_from_html(
    html: &str,
    method_name: &str,
) -> AppResult<(String, bool)> {
    let found = extract_method_markdown_scoped(html, method_name)?;
    Ok((found.markdown, found.empty))
}

/// A located associated-item anchor plus its rendered markdown.
///
/// `anchor_id` is the id that actually matched, so callers can echo a fragment
/// that exists on the page instead of the one they guessed.
#[derive(Debug, Clone)]
pub struct MethodExtract {
    /// Markdown for the member signature plus its docblock.
    pub markdown: String,
    /// Whether the rendered markdown is blank (always `false` on success).
    pub empty: bool,
    /// Extraction scope reported on the wire; always `"method"`.
    ///
    /// The value means "the markdown came from the member anchor", never "the
    /// member is a function". Associated types and constants report it too:
    /// agents have asserted `extraction == "method"` to reject parent-page
    /// fallback success since 1.2.0, and that fail-closed guarantee holds for
    /// every anchor family.
    pub scope: &'static str,
    /// Anchor id that matched, e.g. `tymethod.next` or `associatedtype.Item`.
    pub anchor_id: String,
}

/// Like [`extract_method_markdown_from_html`] but reports scope and anchor id.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::NotFound`] when no anchor variant is present, or when
/// the matched anchor renders empty.
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_method_markdown_scoped(html: &str, method_name: &str) -> AppResult<MethodExtract> {
    extract_assoc_markdown_scoped_from_document(
        &Html::parse_document(html),
        METHOD_ANCHOR_PREFIXES,
        method_name,
    )
}

/// Associated-item extract against an already-parsed document (SCRAPE-S-002/S-003).
///
/// Matches every id built from `prefixes` via the static `[id]` selector plus
/// attribute equality — never builds a dynamic CSS selector string.
///
/// Walks the DOM **once**: a second full pass per prefix would double the CPU
/// cost of the hot member path, and a trait page carries hundreds of ids.
/// Document order does not decide the winner — prefix order does, so `method.`
/// still beats `tymethod.` when a page somehow carries both for one name. The
/// scan short-circuits on a rank-0 hit.
///
/// Suffixed duplicates rustdoc emits for `impl` blocks (`associatedtype.Item-3`)
/// never match, because comparison is full-id equality rather than prefix test.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::NotFound`] when no anchor variant is present, or when
/// the matched anchor renders empty.
/// Returns [`crate::error::ErrorKind::Parse`] when HTML→Markdown conversion fails.
pub fn extract_assoc_markdown_scoped_from_document(
    document: &Html,
    prefixes: &[&str],
    member_name: &str,
) -> AppResult<MethodExtract> {
    let candidates: Vec<String> = prefixes
        .iter()
        .map(|prefix| format!("{prefix}{member_name}"))
        .collect();
    let mut best: Option<(usize, scraper::ElementRef<'_>)> = None;
    for el in document.select(&SEL_WITH_ID) {
        let Some(id) = el.value().attr("id") else {
            continue;
        };
        let Some(rank) = candidates.iter().position(|c| c == id) else {
            continue;
        };
        if best.as_ref().is_none_or(|(seen, _)| rank < *seen) {
            best = Some((rank, el));
            if rank == 0 {
                break;
            }
        }
    }
    let Some((rank, el)) = best else {
        // Fail-closed: parent page HTTP 200 is not a member hit (GAP-W-001 / X-001).
        // Naming every id tried keeps the miss diagnosable without a second fetch.
        return Err(AppError::of(ErrorDetail::AssocAnchorMissing {
            anchors: candidates.join(", "),
        }));
    };
    let anchor_id = candidates[rank].clone();
    let frag_html = method_container_html(el);
    let markdown = html_to_markdown(&frag_html)?;
    if markdown.trim().is_empty() {
        return Err(AppError::of(ErrorDetail::AssocAnchorEmpty {
            anchor_id: anchor_id.to_string(),
        }));
    }
    Ok(MethodExtract {
        markdown,
        empty: false,
        scope: "method",
        anchor_id,
    })
}

/// Strip any [`METHOD_ANCHOR_PREFIXES`] entry from a rustdoc anchor id.
///
/// Returns the bare method name, or `None` when the id names something else
/// (`associatedtype.Item`, `variant.Some`, a section heading, …).
pub fn strip_method_anchor_prefix(id: &str) -> Option<&str> {
    strip_assoc_anchor_prefix(id, METHOD_ANCHOR_PREFIXES)
}

/// List rustdoc associated-method anchor names (`method.X` / `tymethod.X` → `X`).
///
/// Used by `--suggest` on method miss (one DOM pass, no network). Required trait
/// methods are included: omitting them made `--suggest` blind on trait pages.
pub fn list_method_anchor_names(document: &Html) -> Vec<String> {
    list_assoc_anchor_names(document, METHOD_ANCHOR_PREFIXES)
}

/// List rustdoc anchor names of one family (`associatedtype.X` → `X`).
///
/// Backs `--suggest` for every member category. Suggestions come from the
/// **parent page already in hand**, never from `all.html` — which is why they
/// work at all: the crate index lists the trait but none of its members.
///
/// Rustdoc appends a `-<n>` disambiguator when the same member name appears in
/// several impl blocks on one page (`associatedtype.Item-1`, `Item-10`, …).
/// A Rust identifier can never contain `-`, so any hyphen here is that suffix
/// and nothing else — the name is truncated at it and duplicates collapse.
/// Without this, `Iterator::item` answered with `Item`, `Item-1`, `Item-10`,
/// `Item-100` and `Item-101`: four of five suggestions were noise, and the
/// edit-distance ranking favours them precisely because they are near-identical.
pub fn list_assoc_anchor_names(document: &Html, prefixes: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(32);
    let mut seen = std::collections::HashSet::with_capacity(32);
    for el in document.select(&SEL_WITH_ID) {
        if let Some(name) = el
            .value()
            .attr("id")
            .and_then(|id| strip_assoc_anchor_prefix(id, prefixes))
        {
            let canonical = name.split('-').next().unwrap_or(name);
            if !canonical.is_empty() && seen.insert(canonical.to_string()) {
                out.push(canonical.to_string());
            }
        }
    }
    out
}

/// Parse HTML and list method anchor names (CPU-bound helper for ops recovery).
pub fn list_method_anchor_names_from_html(html: &str) -> Vec<String> {
    list_method_anchor_names(&Html::parse_document(html))
}

/// Parse HTML and list one family's anchor names (CPU-bound helper for ops recovery).
pub fn list_assoc_anchor_names_from_html(html: &str, family: AssocAnchorKind) -> Vec<String> {
    list_assoc_anchor_names(&Html::parse_document(html), family.anchor_prefixes())
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
