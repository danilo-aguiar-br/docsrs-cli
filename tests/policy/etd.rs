//! Explicit Target Designation, gated by class instead of by instance.
//!
//! `cache clear` learned to demand a designated target on 2026-08-10, and four
//! tests in `tests/etd_target_designation.rs` proved it. The same approved plan
//! said `config init --force` would learn it too, and it did not. Nothing
//! noticed: every gate was green, because the rule existed only inside one
//! `match` arm and no test could ask the tree how many verbs destroy things.
//!
//! So the rule became data, in `src/cli/destructive.rs`, and these gates read
//! it. Two of them check that each registered verb keeps its promises. The third
//! is the one that matters: it walks every subcommand clap knows about, reads
//! what each one says it does, and fails when a verb that promises destruction
//! is missing from the register. That is the difference between catching this
//! defect and catching its class.

use super::{assert_clean, read};
use clap::CommandFactory;
use docsrs_cli::cli::Cli;
use docsrs_cli::cli::destructive::destructive_verbs;
use docsrs_cli::config::PathSource;
use docsrs_cli::error::DestructiveEffect;

/// Words that promise a caller something will not survive the call.
///
/// Derived into one constant so the class gate and its canary cannot drift, and
/// so widening the vocabulary is one edit rather than a search.
const DESTRUCTIVE_WORDS: &[&str] = &[
    "delete",
    "deletes",
    "overwrite",
    "overwrites",
    "remove",
    "removes",
    "erase",
    "erases",
    "purge",
    "purges",
    "replace",
    "replaces",
    "truncate",
    "truncates",
    "wipe",
    "wipes",
];

/// True when a sentence promises destruction rather than merely mentioning it.
fn promises_destruction(about: &str) -> bool {
    let lower = about.to_lowercase();
    DESTRUCTIVE_WORDS
        .iter()
        .any(|w| lower.split(|c: char| !c.is_alphanumeric()).any(|t| t == *w))
}

/// A leaf of the parser: its invocation path, what it says, and its override flags.
#[derive(Debug, Clone)]
struct Leaf {
    /// Invocation path in wire spelling (`cache clear`).
    wire: String,
    /// `about` and `long_about` joined, which is everything the leaf claims.
    prose: String,
    /// Long flags this leaf declares, minus the ones clap generates.
    flags: Vec<String>,
}

/// Flags whose only job is to override a safety check.
///
/// A leaf declaring one of these is destructive whatever its prose says, and
/// that is the signal that would have caught `config init --force` on the tree
/// where this gate was missing: its `about` read "Create a default config.toml"
/// and promised nothing, while `--force` sat right beneath it.
const OVERRIDE_FLAGS: &[&str] = &["force", "yes"];

/// Every invocable leaf of the parser, asked of clap rather than of the source.
///
/// Reading `get_long_about` as well as `get_about` matters: the short line is
/// written for `--help` column width and routinely omits the dangerous half.
fn command_leaves() -> Vec<Leaf> {
    fn walk(cmd: &clap::Command, prefix: &str, out: &mut Vec<Leaf>) {
        let subs: Vec<&clap::Command> = cmd.get_subcommands().collect();
        if subs.is_empty() {
            let about = cmd.get_about().map(ToString::to_string).unwrap_or_default();
            let long = cmd
                .get_long_about()
                .map(ToString::to_string)
                .unwrap_or_default();
            out.push(Leaf {
                wire: prefix.trim().to_string(),
                prose: format!("{about} {long}"),
                flags: cmd
                    .get_arguments()
                    .filter_map(clap::Arg::get_long)
                    .map(ToString::to_string)
                    .collect(),
            });
            return;
        }
        for sub in subs {
            let name = format!("{prefix} {}", sub.get_name());
            walk(sub, &name, out);
        }
    }
    let binding = Cli::command();
    let mut out = Vec::new();
    for sub in binding.get_subcommands() {
        walk(sub, sub.get_name(), &mut out);
    }
    out.sort_by(|a, b| a.wire.cmp(&b.wire));
    out
}

impl Leaf {
    /// Whether this leaf destroys, by either signal.
    fn destroys(&self) -> bool {
        promises_destruction(&self.prose)
            || self
                .flags
                .iter()
                .any(|f| OVERRIDE_FLAGS.contains(&f.as_str()))
    }
}

/// No verb that destroys may be absent from the register.
///
/// This is the class gate. `config init --force` shipped destructive, with no
/// waiver and no `target_source`, while its sibling had both — and the only
/// thing between the tree and that state was whether someone remembered.
///
/// Two signals, because either alone has a hole. Prose alone misses the verb
/// whose `about` reads "Create a default config.toml" and whose danger lives one
/// flag below; an override flag alone misses a verb that deletes with no flag at
/// all, which is exactly what `cache clear` was.
#[test]
fn no_verb_that_destroys_escapes_the_register() {
    let leaves = command_leaves();
    assert!(
        leaves.len() >= 10,
        "only {} command leaves came back from the parser; the walk is broken, not the tree",
        leaves.len()
    );

    let registered: Vec<&str> = destructive_verbs().iter().map(|v| v.wire).collect();
    let mut problems = Vec::new();
    for leaf in &leaves {
        if !leaf.destroys() {
            continue;
        }
        // A registered verb may carry a flag in its wire spelling
        // (`config init --force`), so compare on the leaf path prefix.
        if !registered.iter().any(|r| r.starts_with(leaf.wire.as_str())) {
            problems.push(format!(
                "{}: destroys (prose {:?}, flags {:?}) but is absent from destructive_verbs()",
                leaf.wire, leaf.prose, leaf.flags
            ));
        }
    }
    assert_clean("destructive verbs outside the register", &problems);
}

/// The class gate must fire on the tree that shipped the defect.
///
/// Reconstructed rather than asserted: a leaf whose description promises nothing
/// and whose only tell is `--force`. Without this, the two-signal predicate
/// could lose one signal and still report green on every real tree.
#[test]
fn the_class_gate_sees_a_verb_whose_only_tell_is_a_force_flag() {
    let silent = Leaf {
        wire: "config init".into(),
        prose: "Create a default config.toml under the resolved config directory".into(),
        flags: vec!["force".into()],
    };
    assert!(
        !promises_destruction(&silent.prose),
        "the bait must be silent in prose, or it proves the wrong signal"
    );
    assert!(
        silent.destroys(),
        "the override-flag signal is dead: this is the exact leaf that escaped"
    );

    let harmless = Leaf {
        wire: "cache stats".into(),
        prose: "Report entry count, total bytes, and budget".into(),
        flags: vec!["json".into()],
    };
    assert!(
        !harmless.destroys(),
        "the predicate condemns a read verb, which is the mirror mistake"
    );
}

/// The class predicate must condemn a destructive sentence and spare a read one.
///
/// Without this, a tokenizer bug would make the gate agree with every tree it
/// ever sees, which is the failure mode this repository has shipped twice.
#[test]
fn the_destruction_predicate_separates_a_read_verb_from_a_write_one() {
    let bait = "Delete all cached HTTP bodies under the cache dir";
    let read_only = "Print resolved cache root and which layer won";
    assert!(
        promises_destruction(bait),
        "the predicate would excuse the sentence it exists for"
    );
    assert!(
        !promises_destruction(read_only),
        "the predicate would condemn a read verb, which is the mirror mistake"
    );
    // "removed" is past tense in a result field, not a promise to remove.
    assert!(
        !promises_destruction("Report entry count, total bytes, and budget"),
        "the predicate fires on a verb that reports rather than acts"
    );
}

/// Every registered verb exposes its waiver flag and reports its target source.
///
/// The register is a claim about the binary and about `docs/schemas/`. Left
/// unchecked it would be a third copy of the truth, which is what the register
/// exists to prevent.
#[test]
fn every_destructive_verb_demands_a_designated_target() {
    let leaves = command_leaves();
    let mut problems = Vec::new();

    for verb in destructive_verbs() {
        let leaf = verb.wire.split_whitespace().take(2).collect::<Vec<_>>();
        let leaf_path = leaf.join(" ");
        if !leaves.iter().any(|l| l.wire == leaf_path) {
            problems.push(format!(
                "{}: registered, but the parser has no such command leaf",
                verb.wire
            ));
        }

        let schema = read(&format!("docs/schemas/{}.schema.json", verb.schema_stem));
        if !schema.contains("target_source") {
            problems.push(format!(
                "docs/schemas/{}.schema.json: destructive verb {} declares no target_source",
                verb.schema_stem, verb.wire
            ));
        }

        // The waiver must exist as a flag on that leaf, not merely as a string
        // in the register: a register naming a flag the parser rejects is worse
        // than no register, because it reads as coverage.
        let waiver = verb.waiver_flag.trim_start_matches('-');
        let found = Cli::command()
            .get_subcommands()
            .flat_map(clap::Command::get_subcommands)
            .filter(|c| leaf_path.ends_with(c.get_name()))
            .any(|c| c.get_arguments().any(|a| a.get_long() == Some(waiver)));
        if !found {
            problems.push(format!(
                "{}: register names {} as the waiver, and the parser has no such flag",
                verb.wire, verb.waiver_flag
            ));
        }
    }
    assert_clean("destructive verb obligations", &problems);
}

/// Both configuration references must teach every destructive verb as destructive.
///
/// `config init --force` was documented as "`--force` overwrites" and nothing
/// else: no target, no waiver, no exit code. A reader learned the flag existed
/// and never that it could take a file they had not named.
#[test]
fn every_destructive_verb_is_documented_as_destructive() {
    const REFERENCE: &[&str] = &["docs/CONFIGURATION.md", "docs/CONFIGURATION.pt-BR.md"];
    let mut problems = Vec::new();
    for doc in REFERENCE {
        let text = read(doc);
        for verb in destructive_verbs() {
            for needed in [verb.wire, verb.target_flag, verb.waiver_flag] {
                if !text.contains(needed) {
                    problems.push(format!("{doc}: {} is taught without {needed}", verb.wire));
                }
            }
        }
    }
    assert_clean(
        "destructive verbs in the configuration reference",
        &problems,
    );
}

/// Designation is `--config-dir`/`--cache-dir`, and nothing else counts.
///
/// `Unresolved` must refuse too: it is not a weaker form of designation, it is
/// the absence of any target at all, so acting would be acting somewhere unknown.
#[test]
fn only_a_flag_named_target_counts_as_designation() {
    for verb in destructive_verbs() {
        assert!(
            !verb.must_refuse(false, PathSource::CliFlag),
            "{}: a target named in argv must be accepted",
            verb.wire
        );
        assert!(
            verb.must_refuse(false, PathSource::Xdg),
            "{}: the ambient layer must be refused without a waiver",
            verb.wire
        );
        assert!(
            verb.must_refuse(false, PathSource::Unresolved),
            "{}: an unresolved target must be refused, not treated as designated",
            verb.wire
        );
        assert!(
            !verb.must_refuse(true, PathSource::Xdg),
            "{}: the waiver must accept the ambient target on purpose",
            verb.wire
        );
    }
}

/// Each registered effect must be the one that verb actually has.
///
/// The refusal message reads "would delete" or "would overwrite" from this
/// field. A verb registered with the wrong effect would ship a sentence that is
/// grammatical, translated, and false.
#[test]
fn each_registered_effect_matches_what_the_verb_does() {
    for verb in destructive_verbs() {
        let expected = if verb.wire.starts_with("cache clear") {
            DestructiveEffect::Delete
        } else {
            DestructiveEffect::Overwrite
        };
        assert_eq!(
            verb.effect, expected,
            "{}: registered effect contradicts what the verb does",
            verb.wire
        );
    }
}

/// Every destructive verb must be named in the migration guide, in both languages.
///
/// Measured on 2026-08-10: `docs/MIGRATION.md` opened its 1.3.0 section with the
/// heading `additive` and the sentence "Nothing in the JSON contract was removed
/// or renamed", and both were false. `cache clear` and `config init --force` had
/// just learned to REFUSE — exit 64, nothing deleted — so a 1.2.x script running
/// either one stopped working, and the `config init` envelope had renamed
/// `source` to `target_source`. A caller reading the guide would conclude no
/// action was needed and find out in production.
///
/// The sibling ETD gates cover the runtime, the schema and both configuration
/// references, which is why every one of them was green. None of them reads the
/// migration guide, and gaining a waiver is precisely the event a migration
/// guide exists to announce: a verb that refuses where it used to act is a break,
/// whatever the release notes call the release.
///
/// The guide is cumulative and keeps its historical sections, so naming a verb
/// once is enough forever — this asks for presence, not for a fresh entry.
#[test]
fn the_migration_guide_names_every_verb_that_learned_to_refuse() {
    const GUIDES: &[&str] = &["docs/MIGRATION.md", "docs/MIGRATION.pt-BR.md"];
    let verbs = destructive_verbs();
    assert!(
        !verbs.is_empty(),
        "the register came back empty; the extraction is broken, not the guide"
    );

    let mut problems = Vec::new();
    for guide in GUIDES {
        let text = read(guide);
        for verb in verbs {
            if !text.contains(verb.wire) {
                problems.push(format!("{guide}: never names `{}`", verb.wire));
            }
            if !text.contains(verb.waiver_flag) {
                problems.push(format!(
                    "{guide}: names `{}` without its waiver `{}`, so the reader cannot act",
                    verb.wire, verb.waiver_flag
                ));
            }
        }
    }
    assert_clean("destructive verbs in the migration guide", &problems);
}

/// No document may teach an invocation of a destructive verb that would refuse.
///
/// Measured on 2026-08-10: twenty-two lines across twelve files taught
/// `docsrs-cli cache clear --json` and `docsrs-cli config init --force --json`,
/// and both exit 64 on this binary while acting on nothing. The recipes were
/// correct when they were written and became wrong the day the verbs learned
/// Explicit Target Designation; `--yes` appeared in no recipe document at all,
/// so a reader could not even discover the way out.
///
/// The sibling gate above asks whether the migration guide *announces* the
/// break. This asks whether any document still *teaches* it, which is the
/// direction a reader actually meets: nobody consults a migration guide before
/// copying a one-line recipe.
///
/// The predicate reads the whole invocation up to the end of the line, because
/// designation may follow the verb at any position, and it accepts either flag —
/// naming the target and waiving it are both designation.
#[test]
fn no_document_teaches_a_destructive_invocation_that_would_refuse() {
    let verbs = destructive_verbs();
    let mut problems = Vec::new();

    for (rel, text) in super::prose_docs()
        .iter()
        .map(|p| (super::rel_of(p), read(&super::rel_of(p))))
    {
        // `gaps.md` and the changelogs quote the defective argv as the record of
        // what was wrong, which is their job. `CLAUDE.md` is the operator's own
        // instruction file: it is excluded from the package by the `include`
        // allowlist and teaches no reader of this crate, so demanding product
        // ergonomics of it would be a gate reaching outside its subject.
        if super::is_historical(&rel)
            || rel == "gaps.md"
            || rel == "CLAUDE.md"
            || rel.starts_with("CHANGELOG")
        {
            continue;
        }
        for (n, line) in super::numbered(&text) {
            for verb in verbs {
                let Some(at) = line.find(&format!("docsrs-cli {}", verb.wire)) else {
                    continue;
                };
                let invocation = &line[at..];
                if invocation.contains(verb.waiver_flag) || invocation.contains(verb.target_flag) {
                    continue;
                }
                problems.push(format!(
                    "{rel}:{n}: teaches `{}` with neither `{}` nor `{}`, which exits 64",
                    verb.wire, verb.target_flag, verb.waiver_flag
                ));
            }
        }
    }
    assert_clean("destructive invocations in documents", &problems);
}
