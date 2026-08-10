//! docs.rs operations: readme, get-item, search-in-crate.
//!
//! # Extraction posture (Rules Rust — web scraping / data extraction)
//!
//! - **Transport** is HTTP GET ([`HttpClient`](crate::http::HttpClient)); this module only parses bytes.
//! - **Structure** uses `scraper` (html5ever) + CSS selectors — never regex for
//!   field extraction from HTML (version scrape uses path segments on `a[href]`).
//! - **Sanitization** may use process-static regex only to strip `on*` handlers
//!   and `javascript:` URLs after DOM script/style removal (XSS hygiene, not
//!   content discovery).
//! - **Markdown** conversion via `htmd` on the sanitized fragment.
//! - **Provenance:** every success payload includes `source_url` (final URL).
//!   Hit URLs from `all.html` join against that `source_url` base (stdlib /
//!   mock / docs.rs) — never a hardcoded docs.rs host. Absolute hit hrefs must
//!   stay **same-origin** as `source_url` (SCRAPE-S-001); off-origin are skipped.
//! - **Encoding:** UTF-8 only after BOM strip ([`crate::http::decode_utf8`]);
//!   docs.rs / doc.rust-lang.org ship UTF-8. `encoding_rs` multi-charset is OOS.
//! - **Not a crawl:** no link frontier, no sitemap, no RSS/Atom.
//! - **robots.txt:** **PROIBIDO** — this product never fetches, parses, or
//!   enforces REP (operator mandate + ADR 0003). One-shot allowlisted GETs only.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **I/O stage:** HTTPS GET via [`HttpClient`](crate::http::HttpClient) on multi-thread Tokio workers.
//! - **CPU stage:** HTML scrape + markdown conversion via
//!   [`ConcurrencyBudget::run_cpu_bound`](crate::concurrency::ConcurrencyBudget::run_cpu_bound)
//!   (`spawn_blocking` + Semaphore).
//! - **Hit scan:** `all.html` candidate lists are scanned sequentially at every
//!   size. Per-item work is one URL resolve plus a string match — too small to
//!   amortize thread fan-out and join. A `rayon` path was benchmarked from 16 to
//!   32768 candidates, lost at every size (best case 0.85x), and was removed.

pub(crate) mod assoc;
mod fetch;
pub(crate) mod hits;
mod html;
mod search;
mod types;
mod urls;

pub use assoc::{
    ASSOC_ANCHOR_MISS_PREFIX, AssocAnchorKind, METHOD_ANCHOR_PREFIXES, assoc_from_fragment,
    associated_item_path, strip_assoc_anchor_prefix,
};
pub use fetch::{
    fetch_item, fetch_item_at, fetch_item_at_with_echo, fetch_item_on_origin,
    fetch_item_on_origin_with_echo, fetch_readme, fetch_readme_at, fetch_readme_on_origin,
};
pub use html::{
    MethodExtract, extract_item_markdown_from_html, extract_method_markdown_from_html,
    extract_method_markdown_scoped, extract_readme_markdown_from_html,
    list_assoc_anchor_names_from_html, list_method_anchor_names,
    list_method_anchor_names_from_html, sanitize_html_fragment, scrub_rustdoc_chrome,
    strip_method_anchor_prefix,
};
pub use search::{
    join_href, parse_all_html_hits, resolve_hit_url, search_in_crate, search_in_crate_at,
    search_in_crate_from_html, search_in_crate_on_origin,
};
pub use types::{GetItemData, ReadmeData, SearchInCrateData, SearchInCrateHit};
pub use urls::{
    all_html_url, all_html_url_on_origin, get_item_url, get_item_url_on_origin,
    get_item_url_on_origin_with_parent_kind, is_method_path, method_segments_from_parent,
    pick_unique_type_path, readme_url, readme_url_on_origin, strip_crate_prefix_segments,
};

#[cfg(test)]
mod tests;
