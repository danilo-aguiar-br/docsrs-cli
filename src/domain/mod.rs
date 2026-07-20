//! Domain newtypes: parse at the boundary, carry validity in the type.
//!
//! Follows parse-don't-validate: only fallible constructors build values.
//! Core path APIs take these types by reference so the compiler keeps the proof
//! (see `docs/decisions/0006-type-system-posture.md` and
//! `docs/decisions/0008-domain-types-posture.md` for url-only of the four
//! generic domain crates — no chrono/uuid/rust_decimal).
//!
//! # Naming note
//!
//! Product commands such as `get-item` and HTTP helpers such as `get_json` /
//! `get_html` use the verb **get** for the operation, not as a field getter
//! (`name()` not `get_name()`).

mod crate_name;
mod crate_ref;
mod item_path;
mod match_mode;
mod origin;
mod regex;
mod search_query;
mod version;

pub use crate_name::{CrateName, is_stdlib_name};
pub use crate_ref::CrateRef;
pub use item_path::ItemPath;
pub use match_mode::MatchMode;
pub use origin::AllowedOrigin;
pub(crate) use regex::compile_bounded_regex;
pub use search_query::SearchQuery;
pub use version::VersionArg;
