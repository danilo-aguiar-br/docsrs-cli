//! Network-facing command handlers (crates.io / docs.rs).
//!
//! One-shot: each handler performs bounded HTTP then returns. CPU HTML work stays
//! inside `docs_rs` / `spawn_blocking`. No process-global locks across `.await`.
//!
//! Shared handler context is [`OpCtx`] (TYPE-L-011) so call sites stay under
//! clippy's argument budget without discarding domain types.
//!
//! One handler per submodule (single responsibility). Each submodule re-exports
//! its handler here, so the crate-facing surface stays `ops::<command>`.

mod get_item;
mod readme;
mod search_crates;
mod search_in_crate;

pub(crate) use get_item::get_item;
pub(crate) use readme::readme;
pub(crate) use search_crates::search_crates;
pub(crate) use search_in_crate::search_in_crate;

use tokio::time::Instant;

use crate::cli::Cli;
use crate::config::Config;
use crate::i18n::Locale;
use crate::shutdown::CancelFlag;

/// Shared one-shot handler context (cli/cfg/locale/flags/cancel).
///
/// Domain command args stay as separate parameters so signatures stay explicit.
#[derive(Clone)]
pub(crate) struct OpCtx<'a> {
    pub cli: &'a Cli,
    pub cfg: &'a Config,
    pub locale: Locale,
    pub dry_run: bool,
    pub wants_json: bool,
    pub start: Instant,
    pub cancel: CancelFlag,
}
