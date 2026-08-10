//! crates.io search-crates operation.
//!
//! Split by responsibility:
//! - `types` — agent-facing wire payloads and request echo
//! - `urls` — URL builders and pagination bounds
//! - `parse` — JSON body parsing and hostile-field capping
//! - `fetch` — HTTP execution against an allowlisted origin

mod fetch;
mod parse;
mod types;
mod urls;

pub use fetch::{search_crates, search_crates_at, search_crates_on_origin};
pub use parse::parse_search_body;
pub use types::{CrateSearchHit, SearchCratesData, SearchEcho, SearchMeta, echo_params_from_url};
pub use urls::{
    clamp_search_pagination, planned_url, planned_url_on_host, planned_url_with_page_token,
    validate_search_pagination,
};

#[cfg(test)]
mod tests;
