//! Prose for [`ErrorDetail`], in both product languages.
//!
//! # The exhaustiveness contract
//!
//! Neither `match` below has a `_` arm. That is the whole point: adding a
//! variant to [`ErrorDetail`] without writing both sentences is a **compile
//! error**, not a message that silently ships in English. The previous design
//! matched on English prose and fell back to returning it unchanged, so an
//! untranslated message was indistinguishable from a translated one.
//!
//! # English is the wire contract
//!
//! [`render_en`] produces the `message` field of the JSON envelope. Agents match
//! on that text (`ASSOC_ANCHOR_MISS_PREFIX`, for one), so its wording is a
//! compatibility surface: change it deliberately, never incidentally.
//! [`render_pt_br`] is stderr prose for a human and carries no such contract.

mod en;
mod pt_br;

pub(super) use en::render_en;
pub(super) use pt_br::render_pt_br;

/// Sentinel prefix of the "anchor absent on the parent page" message.
///
/// Both languages embed it, and the live probe plus the recovery path match on
/// the English form to stop walking parent kinds. Shared here so the two
/// renderers cannot drift apart on a string that is a wire contract.
pub(super) const ASSOC_ANCHOR_MISS: &str = "associated item anchor ";
