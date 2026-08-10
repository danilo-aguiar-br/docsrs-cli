//! Agent-native payload reduction flags, shared by every subcommand.
//!
//! # Why these live in their own struct
//!
//! Declaring *how much of the payload reaches the caller* is a different concern from
//! declaring *how the request is made*. The transport knobs ([`crate::cli::Cli`]:
//! timeouts, retries, cache) describe the fetch; these describe the emission. Keeping
//! them apart means a reduction knob can be added without touching the transport
//! surface, and [`crate::agent_ops`] takes one value instead of a growing argument list.
//!
//! # Query limits versus emission limits
//!
//! `search-in-crate --limit` and `search-crates --per-page` bound the **query**: they
//! decide how much upstream work happens. [`AgentArgs::max_items`] bounds the
//! **emission**: it decides how much of the already-computed result is written to
//! stdout. The names differ because the things differ — and because a second `--limit`
//! would collide, which clap rejects outright with "Long option names must be unique
//! for each argument".

use clap::Args;

/// Payload reduction knobs applied to the JSON envelope before it is written.
///
/// Every field is `global = true`, so the flags are accepted before or after the
/// subcommand. See [`crate::agent_ops`] for the fixed application order.
#[derive(Debug, Clone, Default, Args)]
pub struct AgentArgs {
    /// Project only these dotted keys (CSV or repeated). Missing keys are skipped, never null
    #[arg(long, visible_alias = "fields", global = true, value_delimiter = ',')]
    pub select: Vec<String>,

    /// Keep only matching elements: `key=value`, `key!=value`, `key~substring` (repeat = AND)
    ///
    /// A malformed expression fails with exit 65 instead of returning an empty set.
    #[arg(long, global = true)]
    pub filter: Vec<String>,

    /// Sort elements ascending by this dotted key (stable; elements without it go last)
    #[arg(long, global = true)]
    pub sort_by: Option<String>,

    /// Drop later elements repeating this key's value (elements without the key are kept)
    #[arg(long, global = true)]
    pub dedupe_by: Option<String>,

    /// Emit at most N elements, counted after --filter and --dedupe-by (0 emits none)
    ///
    /// Bounds the emission, not the query: `search-in-crate --limit` is the query bound.
    #[arg(long, global = true)]
    pub max_items: Option<usize>,

    /// Replace the payload with `{"count": N}`, counted after --filter, --dedupe-by and --max-items
    #[arg(long, global = true)]
    pub count_only: bool,

    /// Shorten every string above N characters (never bytes; UTF-8 is never split)
    #[arg(long, global = true)]
    pub truncate_content: Option<usize>,

    /// Cap emitted payload size in bytes (hard max 2097152 = 2 MiB; cannot raise above)
    #[arg(long, global = true)]
    pub max_output_bytes: Option<u64>,
}
