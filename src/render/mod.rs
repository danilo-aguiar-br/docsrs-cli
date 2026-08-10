//! Markdown and JSON rendering with output truncation.
//!
//! Split by responsibility, with the public API re-exported unchanged so every
//! consumer keeps importing from `crate::render::*`:
//!
//! - [`envelope`] — typed success / dry-run / error JSON envelopes
//! - [`markdown`] — human Markdown per command payload
//! - [`budget`] — `max_output_bytes` truncation and hit-list shrinking
//! - [`schema_md`] — Markdown rendering of the embedded JSON Schemas

pub mod budget;
pub mod envelope;
pub mod markdown;
pub mod schema_md;

pub use budget::{
    apply_output_budget_search_crates, apply_output_budget_search_in_crate,
    apply_truncation_to_item, apply_truncation_to_readme, truncate_output,
};
pub use envelope::{
    DryRunData, DryRunEnvelope, ErrorBody, ErrorEnvelope, SuccessEnvelope, dry_run_envelope,
    error_envelope, success_envelope, success_envelope_with_ok, usage_error,
};
pub use markdown::{
    render_item_markdown, render_readme_markdown, render_search_in_crate_markdown,
    render_search_markdown,
};
pub use schema_md::render_schema_markdown;
