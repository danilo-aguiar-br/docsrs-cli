//! HTML sanitize, version scrape, and markdown extraction (CPU-bound pure paths).
//!
//! Runs under `spawn_blocking` via fetch/search callers — no network here.
//!
//! Field extraction uses `scraper` CSS selectors + typed path segments — never
//! regex over the document for content fields. Process-static regexes below are
//! **security scrub only** (`on*` handlers / `javascript:` URLs).
//!
//! Fixed CSS selectors are compiled once via [`std::sync::LazyLock`] (SCRAPE-R-004)
//! so the one-shot CPU path does not re-parse the same selector strings on every
//! page. Each selector lives in the submodule that uses it.
//!
//! Layout (SRP): `version` (resolved version / SemVer scrape) · `sanitize`
//! (security scrub + HTML→Markdown) · `extract` (docblock / item / method).

mod extract;
mod sanitize;
mod version;

pub use extract::{
    MethodExtract, extract_assoc_markdown_scoped_from_document,
    extract_item_markdown_from_document, extract_item_markdown_from_html,
    extract_method_markdown_from_html, extract_method_markdown_scoped,
    extract_readme_markdown_from_document, extract_readme_markdown_from_html,
    list_assoc_anchor_names_from_html, list_method_anchor_names,
    list_method_anchor_names_from_html, strip_method_anchor_prefix,
};
pub use sanitize::{sanitize_html_fragment, scrub_rustdoc_chrome};
pub(super) use version::{
    extract_resolved_version_for_crate, scrape_docs_rs_version_from_document,
};
// Raw-HTML wrappers of the two above: no production caller parses twice, so
// they exist for the offline suite in `docs_rs::tests` only.
#[cfg(test)]
pub(super) use version::{extract_resolved_version, scrape_docs_rs_version_from_html};

#[cfg(test)]
mod tests;
