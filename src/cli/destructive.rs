//! The register of verbs that destroy, and the one rule they all obey.
//!
//! # Why a register instead of two guards
//!
//! `cache clear` learned Explicit Target Designation on 2026-08-10 and the same
//! plan said `config init --force` would learn it too. Only the first was built.
//! The gap was not the missing guard — it was that nothing could tell the guard
//! was missing, because the rule lived inside one `match` arm and could not be
//! asked a question. A second destructive verb shipped without a waiver and every
//! gate agreed with the tree.
//!
//! So the rule moved here. The runtime reads this list to decide whether to
//! refuse, and `tests/policy/etd.rs` reads the same list to demand a waiver flag,
//! a `target_source` in the schema, and a line in both configuration references.
//! A third verb that destroys is either in this list or the class gate fails; it
//! cannot be quietly correct the way `config init --force` was.
//!
//! # What counts as designation
//!
//! Naming the directory in argv designates it. Passing the waiver accepts an
//! ambient one on purpose. Passing neither means the caller never saw the path
//! about to be destroyed, which is the confused-deputy shape: the caller names
//! the verb, the environment names the victim, and nothing compares the two.

use crate::config::PathSource;
use crate::error::{DestructiveEffect, ErrorDetail};

/// One verb that destroys, with everything needed to refuse it safely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestructiveVerb {
    /// Invocation in wire spelling, as the refusal message names it.
    pub wire: &'static str,
    /// Flag that designates the target in argv.
    pub target_flag: &'static str,
    /// Flag that accepts an ambient target on purpose.
    pub waiver_flag: &'static str,
    /// What would happen to the target.
    pub effect: DestructiveEffect,
    /// Schema stem under `docs/schemas/`, which must carry `target_source`.
    pub schema_stem: &'static str,
}

impl DestructiveVerb {
    /// Whether this invocation must be refused for want of a designated target.
    ///
    /// Only [`PathSource::CliFlag`] counts as designation. `Xdg` is the ambient
    /// layer the caller never named, and `Unresolved` is worse: there is no
    /// target at all, so acting would be acting somewhere unknown.
    pub fn must_refuse(self, waived: bool, source: PathSource) -> bool {
        !waived && source != PathSource::CliFlag
    }

    /// The refusal, naming the victim and both ways out of it.
    ///
    /// The message names the target because a refusal that hides which path was
    /// about to be destroyed teaches the caller nothing about what to pass.
    pub fn refuse(self, target: String) -> ErrorDetail {
        ErrorDetail::AmbientTargetRefused {
            verb: self.wire.to_string(),
            target,
            target_flag: self.target_flag.to_string(),
            waiver_flag: self.waiver_flag.to_string(),
            effect: self.effect,
        }
    }
}

/// `cache clear` empties every cached body under the resolved cache root.
pub const CACHE_CLEAR: DestructiveVerb = DestructiveVerb {
    wire: "cache clear",
    target_flag: "--cache-dir",
    waiver_flag: "--yes",
    effect: DestructiveEffect::Delete,
    schema_stem: "cache-clear",
};

/// `config init --force` replaces an existing `config.toml` in place.
///
/// The waiver is required whenever the target is ambient, even where no file
/// exists yet. The caller cannot know in advance whether a directory they never
/// named holds a file, and that uncertainty is precisely the risk: a rule whose
/// answer depends on the state of the disk gives the same argv two behaviours.
pub const CONFIG_INIT_FORCE: DestructiveVerb = DestructiveVerb {
    wire: "config init --force",
    target_flag: "--config-dir",
    waiver_flag: "--yes",
    effect: DestructiveEffect::Overwrite,
    schema_stem: "config-init",
};

/// Every verb in this binary that destroys something it did not create.
///
/// This is the list the class gate reads. Adding a destructive verb without
/// adding it here fails `no_verb_that_destroys_escapes_the_register`.
pub const fn destructive_verbs() -> &'static [DestructiveVerb] {
    &[CACHE_CLEAR, CONFIG_INIT_FORCE]
}
