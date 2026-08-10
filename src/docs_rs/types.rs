//! Wire payloads for docs.rs operations (JSON data contracts).

use serde::{Deserialize, Serialize};

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
    /// How markdown was scoped for associated methods. Success path is `method` only (1.2.0+ fail-closed; missing anchors are errors).
    ///
    /// Read this as "the markdown came from the member anchor", never as "the
    /// member is a function". It reports `method` for every anchor family,
    /// including `variant.` and `structfield.`, which is why
    /// [`Self::anchor_family`] exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extraction: Option<String>,
    /// Rustdoc anchor family the markdown was actually scoped to.
    ///
    /// One of `method`, `tymethod`, `associatedtype`, `associatedconstant`,
    /// `variant` or `structfield`, derived from the anchor id that matched on
    /// the page rather than from the requested kind.
    ///
    /// Added instead of correcting [`Self::extraction`] because agents already
    /// assert `extraction == "method"` to reject a parent-page false success
    /// (see `src/docs_rs/html/extract.rs`); changing that value would break them
    /// silently, while an absent field is a shape every reader already handles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_family: Option<String>,
    /// Canonical rustdoc path when a reexport/root path was resolved via all.html.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_item_path: Option<String>,
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

/// search-in-crate result set (`truncated` is true when hits were cut by `--limit`
/// and/or by `--max-output-bytes` output budget).
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
    /// Hits the upstream index classified, before `--limit`.
    ///
    /// Describes docs.rs, not this envelope, so agent-native reduction leaves it
    /// alone: it is the only count that survives a filter intact.
    pub total: usize,
    /// Length of the emitted `hits` array.
    ///
    /// Listed in [`crate::agent_ops::EMITTED_COUNT_KEYS`], so reduction rewrites it
    /// whenever it shrinks the array. A field that names the array has to follow it.
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
