//! Documentation gates whose subject is a *class* of drift, not one instance.
//!
//! Every gate here was written after an audit found a defect that an existing
//! gate walked past, and in each case the existing gate was correct about what
//! it declared and blind to what it was for. The rule these three share: derive
//! the expectation from the tree, and let the predicate describe the class.

use super::contract::{count_pattern, strip_emphasis};
use super::{assert_clean, files_under, numbered, prose_docs, read, rel_of, repo_root};
use clap::CommandFactory;
use docsrs_cli::cli::Cli;
use regex::Regex;
use std::collections::BTreeMap;
use std::fs;

/// Every prose document, historical ones included, paired with its path.
///
/// `current_docs` drops the historical set on purpose, because those files
/// record what a past release said. The release-agreement gate below needs them
/// precisely for that: a corrupted historical note is only detectable against
/// the other documents describing the same release.
fn all_docs() -> Vec<(String, String)> {
    prose_docs()
        .into_iter()
        .map(|p| (rel_of(&p), fs::read_to_string(&p).unwrap_or_default()))
        .collect()
}

/// Source and script files, paired with their repo-relative path.
fn source_and_scripts() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = files_under("src", ".rs")
        .into_iter()
        .chain(files_under("tests", ".rs"))
        .chain(files_under("scripts", ".sh"))
        .map(|p| (rel_of(&p), fs::read_to_string(&p).unwrap_or_default()))
        .collect();
    out.sort();
    out
}

/// Naming a removed file in the past tense is a record; in the present it is a lie.
///
/// These markers keep the audit trail legal. `tests/policy/mod.rs` opens with
/// "these gates used to live in `scripts/check-policy.sh`", which is true and
/// must stay; `src/agent_ops/tests.rs` said the ceiling was "enforced by
/// `scripts/check-policy.sh`", which shipped in published rustdoc pointing at a
/// file deleted by GAP-GATE-005.
const PAST_TENSE: &[&str] = &[
    "used to",
    "held",
    "reached",
    "never saw",
    "replace",
    "is gone",
    "was ",
    "were ",
    "removed",
    "gone",
    "shipped",
    "deleted",
    "não existe mais",
    "nao existe mais",
    "removido",
    "era ",
    "tinha ",
    "foi ",
];

/// No file may point the reader at a gate script that is not on disk.
///
/// Measured on 2026-08-10: `src/agent_ops/tests.rs:4` attributed the 500-line
/// ceiling to `scripts/check-policy.sh`, removed weeks earlier, in a `//!` block
/// that ships as rustdoc. The ceiling is real and the enforcer named was not.
///
/// No gate could catch it: the script gates read `scripts/`, the prose gates read
/// `.md`, and nothing compared a *mention* of a path against the filesystem.
#[test]
fn no_file_names_a_gate_script_that_does_not_exist() {
    let existing: Vec<String> = files_under("scripts", ".sh")
        .iter()
        .map(|p| rel_of(p).rsplit('/').next().unwrap_or_default().to_string())
        .collect();
    assert!(
        !existing.is_empty(),
        "scripts/ yielded no .sh files; the extraction is broken, not the tree"
    );

    let mention = Regex::new(r"scripts/([a-z0-9_-]+\.sh)").expect("static regex");
    let mut problems = Vec::new();
    let mut scanned = source_and_scripts();
    scanned.extend(all_docs().into_iter().filter(|(rel, _)| {
        // Historical documents narrate removals; that is their job.
        !super::is_historical(rel)
    }));

    for (rel, text) in scanned {
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            if PAST_TENSE.iter().any(|m| lower.contains(m)) {
                continue;
            }
            for cap in mention.captures_iter(line) {
                let name = cap[1].to_string();
                if !existing.contains(&name) {
                    problems.push(format!(
                        "{rel}:{n}: names scripts/{name} in the present tense, and no such file exists"
                    ));
                }
            }
        }
    }
    assert_clean("dangling script references", &problems);
}

/// The predicate must recognise the sentence it exists for, and spare the ones it does not.
///
/// A gate that matches nothing passes forever, and this repository has already
/// shipped two such gates. The bait is the exact line that was defective.
#[test]
fn the_dangling_script_predicate_separates_record_from_claim() {
    // The path is split across two literals on purpose. Written contiguously,
    // this canary would be a dangling reference in the very file that rejects
    // them, and the gate would have to exempt its own source to stay green — an
    // exemption is a hole, and a hole in the gate is worse than the defect.
    let claim = concat!(
        "//! enforced by `scripts/",
        "check-policy.sh`, matching `crate::cli::tests`"
    );
    let record = concat!(
        "//! the anti-env gate in `scripts/",
        "check-policy.sh` never saw it"
    );
    let lower_claim = claim.to_lowercase();
    let lower_record = record.to_lowercase();
    assert!(
        !PAST_TENSE.iter().any(|m| lower_claim.contains(m)),
        "the predicate would excuse the present-tense claim it exists for"
    );
    assert!(
        PAST_TENSE.iter().any(|m| lower_record.contains(m)),
        "the predicate would condemn a legitimate historical record"
    );
}

/// The two arguments clap generates on its own, which no reference must teach.
///
/// Kept at exactly two and asserted below: an exemption list that may grow
/// quietly is the hole this gate exists to close, only moved one level up.
const PARSER_GENERATED: &[&str] = &["help", "version"];

/// Every global flag the binary accepts, in long spelling, asked of clap itself.
///
/// This used to scan `src/cli/mod.rs` for `#[arg(… global = true …)]` paired with
/// the `pub` field beneath it, and its own rustdoc explained why that was enough:
/// `AgentArgs` arrives by `#[command(flatten)]` and has a gate in `contract.rs`.
/// The two claims never met. That sibling gate requires the reduction flags in
/// fifteen `CONTRACT_DOCS`, and `docs/CONFIGURATION.md` is in none of them — it is
/// the one document promising to teach every knob, and eight of its thirty-two
/// were unfalsifiable. Deleting the `--dedupe-by` line from it left the suite green.
///
/// Asking the parser removes the question. `Cli::command()` already merged every
/// flattened struct, so a flag declared in a file nobody thought to grep is in
/// scope on the day it is written, and no path appears in this function at all.
fn global_flags() -> Vec<String> {
    let mut out: Vec<String> = Cli::command()
        .get_arguments()
        .filter(|a| a.is_global_set())
        .filter_map(clap::Arg::get_long)
        .filter(|long| !PARSER_GENERATED.contains(long))
        .map(|long| format!("--{long}"))
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The derived list must reach past the file the old predicate read.
///
/// `--dedupe-by` lives in `src/cli/agent_args.rs`. If it is absent, the extraction
/// has silently narrowed back to one struct, and a narrowed extraction is
/// indistinguishable from a documented tree.
#[test]
fn the_global_flag_extraction_reaches_every_flattened_struct() {
    let flags = global_flags();
    assert!(
        flags.contains(&"--dedupe-by".to_string()),
        "extraction missed the reduction layer entirely: {flags:?}"
    );
    assert!(
        flags.contains(&"--timeout".to_string()),
        "extraction missed the transport layer entirely: {flags:?}"
    );
    assert_eq!(
        PARSER_GENERATED.len(),
        2,
        "the exemption list grew; every entry is a flag no document must teach"
    );
    for generated in PARSER_GENERATED {
        assert!(
            !flags.contains(&format!("--{generated}")),
            "--{generated} is clap's own and must not be demanded of the reference"
        );
    }
}

/// The configuration reference must name every global flag the binary accepts.
///
/// GAP-DOC-CONFIG-001 closed the documentation half and said so in its own
/// `### Estado`: "aberto quanto ao gate: nenhum teste ainda exige que uma flag
/// global nova apareça na referência". Eleven knobs had existed in no user
/// document at all, and the reason was structural — `AgentArgs` had a gate and
/// the transport knobs, living in `src/cli/mod.rs`, had none.
///
/// Writing the missing eleven fixed the instance. This fixes the class.
#[test]
fn every_global_flag_appears_in_the_configuration_reference() {
    const REFERENCE: &[&str] = &["docs/CONFIGURATION.md", "docs/CONFIGURATION.pt-BR.md"];
    let flags = global_flags();
    assert!(
        flags.len() >= 30,
        "only {} global flags came back from the parser; the extraction is broken, not the docs",
        flags.len()
    );

    let mut problems = Vec::new();
    for doc in REFERENCE {
        let text = read(doc);
        for flag in &flags {
            if !text.contains(flag.as_str()) {
                problems.push(format!("{doc}: missing {flag}"));
            }
        }
    }
    assert_clean("global flags in the configuration reference", &problems);
}

/// A `##` heading naming a release, or `None` when it names no version.
fn heading_release(heading: &str) -> Option<String> {
    Regex::new(r"\b(\d+\.\d+(?:\.\d+|\.x))\b")
        .expect("static regex")
        .captures(heading)
        .map(|c| c[1].to_string())
}

/// No two documents may attribute different schema counts to the same release.
///
/// GAP-DOC-HISTORY-001 measured the defect and could not close it: `README.md`
/// and `INTEGRATIONS.md` said 1.2.0 shipped twenty wire names while
/// `docs/MIGRATION.md` said nineteen, and nineteen was right — the changelog
/// records `agent-surface` arriving in 1.3.0. The historical lines had been
/// edited in place when the count turned, so the record of the past began
/// asserting the present.
///
/// The sibling gate cannot see this. It compares every count against
/// `docs/schemas/` on disk, and the disk only knows today. What a past release
/// contained survives in exactly one place: the agreement between the documents
/// that describe it. Disagreement is the signal, and it needs no oracle.
#[test]
fn no_two_documents_disagree_about_the_same_release() {
    let counted = count_pattern();
    let mut claims: BTreeMap<String, BTreeMap<usize, Vec<String>>> = BTreeMap::new();

    for (rel, text) in all_docs() {
        let mut heading = String::new();
        for (n, line) in numbered(&text) {
            if line.starts_with('#') {
                heading = line.to_string();
            }
            let Some(release) = heading_release(&heading) else {
                continue;
            };
            if !line.to_lowercase().contains("schema") {
                continue;
            }
            for cap in counted.captures_iter(&strip_emphasis(line)) {
                let Some(got) = spelled_count(&cap[1]) else {
                    continue;
                };
                claims
                    .entry(release.clone())
                    .or_default()
                    .entry(got)
                    .or_default()
                    .push(format!("{rel}:{n}"));
            }
        }
    }

    let problems: Vec<String> = claims
        .iter()
        .filter(|(_, by_count)| by_count.len() > 1)
        .map(|(release, by_count)| {
            let detail: Vec<String> = by_count
                .iter()
                .map(|(count, wheres)| format!("{count} at {}", wheres.join(", ")))
                .collect();
            format!(
                "release {release}: documents disagree — {}",
                detail.join("; ")
            )
        })
        .collect();
    assert_clean("cross-document release agreement", &problems);
}

/// Digits and the spelled-out numerals this repository has actually written.
fn spelled_count(token: &str) -> Option<usize> {
    match token.to_lowercase().as_str() {
        "nineteen" | "dezenove" => Some(19),
        "twenty" | "vinte" => Some(20),
        "twenty-one" | "vinte e um" => Some(21),
        d => d.parse::<usize>().ok(),
    }
}

/// The agreement gate must fire on the pair it was written for.
///
/// Reconstructed from the measured defect rather than asserted abstractly: two
/// files, one release heading each, two different counts. Without this, a
/// grouping bug would make the gate silently agree with everything.
#[test]
fn the_agreement_gate_fires_on_the_pair_it_was_written_for() {
    let counted = count_pattern();
    let readme = "## Earlier highlights (1.2.0)\n- Offline schemas: 20 wire names\n";
    let migration = "## 1.2.0 breaking (from 1.1.x)\n- Offline schemas: **19** wire names\n";

    let mut seen: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for text in [readme, migration] {
        let mut heading = String::new();
        for (_, line) in numbered(text) {
            if line.starts_with('#') {
                heading = line.to_string();
            }
            let Some(release) = heading_release(&heading) else {
                continue;
            };
            if !line.to_lowercase().contains("schema") {
                continue;
            }
            for cap in counted.captures_iter(&strip_emphasis(line)) {
                if let Some(v) = spelled_count(&cap[1]) {
                    seen.entry(release.clone()).or_default().push(v);
                }
            }
        }
    }
    let for_1_2_0 = seen.get("1.2.0").expect("both headings name 1.2.0");
    assert!(
        for_1_2_0.contains(&19) && for_1_2_0.contains(&20),
        "the agreement gate no longer sees the disagreement it exists for: {for_1_2_0:?}"
    );
}

/// The repository root must stay resolvable from the manifest directory.
///
/// Cheap, and it turns a confusing panic deep inside `read` into one line.
#[test]
fn the_repository_root_is_resolvable() {
    assert!(
        repo_root().join("Cargo.toml").is_file(),
        "CARGO_MANIFEST_DIR does not contain Cargo.toml"
    );
}
