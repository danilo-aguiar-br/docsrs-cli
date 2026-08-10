//! Local meta commands: cache, config, schema, completions, command tree, dry-run/error emit.
//!
//! No product network I/O. One-shot helpers for agent discovery and local state.
//!
//! One submodule per command family (single responsibility). This module re-exports
//! the crate-internal surface so callers keep using `meta_cmds::*` paths unchanged.

mod cache;
mod commands;
mod completions;
mod config;
mod dry_run;
mod emit;
mod schema;

pub(crate) use cache::cache_cmd;
pub(crate) use commands::commands_cmd;
pub(crate) use completions::completions_cmd;
pub(crate) use config::config_cmd;
pub(crate) use dry_run::{
    GetItemDryParams, ReadmeDryParams, SearchCratesDryParams, SearchInCrateDryParams, emit_dry_run,
};
pub(crate) use emit::{VersionData, emit_error};
pub(crate) use schema::schema_cmd;
