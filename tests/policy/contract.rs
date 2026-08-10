//! Agent-contract gates: the documented surface must equal the built surface.

use super::{assert_clean, current_docs, files_under, numbered, read, rel_of};
use regex::Regex;
use std::fs;

/// Documents an agent reads instead of `--help`.
const AGENT_DOCS: &[&str] = &["docs/AGENTS.md", "docs/AGENTS.pt-BR.md"];

/// Documents that teach the reduction layer.
///
/// This list is hand-kept because "which files teach the contract" is not
/// derivable from the tree. The *flag* list stays derived from the struct, so the
/// pair cannot go stale in the direction that matters. `MIGRATION.md` is
/// deliberately absent: it narrates a version change, it does not teach a surface.
///
/// The five entries after the skills were added on 2026-08-10, when an audit
/// measured `llms.txt`, `llms.pt-BR.txt` and `llms-full.txt` at **zero** of the
/// eight reduction flags and `INTEGRATIONS*` at two. `CATALOG_DOCS` below had
/// listed all five for the command surface the whole time, so the two sibling
/// constants disagreed about which files teach a contract — and the undefended
/// half is where the drift showed up, exactly as GAP-DOC-CONFIG-001 predicted.
///
/// The `llms` bundles matter most here: they exist so an agent can learn the
/// surface without reading the tree, so a flag missing there is a flag that does
/// not exist for the reader they were written for.
const CONTRACT_DOCS: &[&str] = &[
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "docs/COOKBOOK.md",
    "docs/COOKBOOK.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
    "README.md",
    "README.pt-BR.md",
    "skills/docsrs-cli-en/SKILL.md",
    "skills/docsrs-cli-pt/SKILL.md",
    "INTEGRATIONS.md",
    "INTEGRATIONS.pt-BR.md",
    "llms.txt",
    "llms.pt-BR.txt",
    "llms-full.txt",
];

/// Every field of `AgentArgs` is a flag the binary accepts, so every document
/// teaching the reduction layer must name it.
///
/// The first version watched `AGENTS.md` alone, fixed the two files it measured
/// and left the same drift alive everywhere else: four of the six remaining
/// documents named `--max-output-bytes` in the pipeline-order line while never
/// introducing the flag, so the reader learned there was a final stage and could
/// not learn how to reach it.
///
/// The match is the bare flag, not a backticked one. Requiring `` `--flag ``
/// tied the gate to one typographic habit, and `HOW_TO_USE.md` teaches through
/// whole command lines where the backtick opens at `docsrs-cli`.
#[test]
fn every_agent_flag_appears_in_every_contract_document() {
    let src = read("src/cli/agent_args.rs");
    let field = Regex::new(r"(?m)^ {4}pub ([a-z_]+):").expect("static regex");
    let flags: Vec<String> = field
        .captures_iter(&src)
        .map(|c| format!("--{}", c[1].replace('_', "-")))
        .collect();
    assert!(!flags.is_empty(), "AgentArgs exposes no fields");

    let mut problems = Vec::new();
    for doc in CONTRACT_DOCS {
        let text = read(doc);
        for flag in &flags {
            if !text.contains(flag.as_str()) {
                problems.push(format!("{doc}: missing {flag}"));
            }
        }
    }
    assert_clean("agent flags in contract docs", &problems);
}

/// Documents that promise the reader the whole command surface.
///
/// `CONFIGURATION.md` is deliberately absent: it is a flag and key reference, not
/// a command catalog, and demanding every command there would teach the gate that
/// scope does not matter.
const CATALOG_DOCS: &[&str] = &[
    "README.md",
    "README.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "INTEGRATIONS.md",
    "INTEGRATIONS.pt-BR.md",
    "llms.txt",
    "llms.pt-BR.txt",
    "llms-full.txt",
    "skills/docsrs-cli-en/SKILL.md",
    "skills/docsrs-cli-pt/SKILL.md",
];

/// Every invocable command path, derived from `wire_name()`.
///
/// `wire_name` is the single place the binary names its own surface, so a new
/// subcommand is in scope the moment it can appear in an envelope — nobody has to
/// remember to update a list here.
fn command_paths() -> Vec<String> {
    let body = wire_name_block();
    let lit = Regex::new(r#"=>\s*"([a-z][a-z-]*)""#).expect("static regex");
    let mut out: Vec<String> = lit.captures_iter(&body).map(|c| c[1].to_string()).collect();
    out.sort();
    out.dedup();
    out
}

/// The body of `wire_name`, which is where the binary names its own surface.
fn wire_name_block() -> String {
    let src = read("src/cli/mod.rs");
    src.split_once("pub fn wire_name")
        .and_then(|(_, rest)| rest.split_once("impl Cli"))
        .map(|(body, _)| body.to_string())
        .expect("wire_name block not found in src/cli/mod.rs")
}

/// Commands that dispatch to a nested action, derived from the `match action` arms.
///
/// Derived rather than listed, so a third nested command is in scope on the day it
/// is written. The names come back lowercased to match the wire spelling.
fn nested_parents() -> Vec<String> {
    let block = wire_name_block();
    let arm = Regex::new(r"Self::([A-Za-z]+)\s*\{\s*action\s*\}").expect("static regex");
    let mut out: Vec<String> = arm
        .captures_iter(&block)
        .map(|c| c[1].to_lowercase())
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Every command the binary can run must be named in every catalog document.
///
/// Measured on 2026-08-10: `cache path` existed, answered, owned a published
/// schema and a `CHANGELOG` entry, and appeared in **no** user-facing document —
/// not the README section that opens with "Full surface has 11 top-level
/// commands", not `HOW_TO_USE`, not `AGENTS`, not `COOKBOOK`, not either `llms`
/// file. The sibling gate watched `AgentArgs`, which is the reduction layer, so
/// the eight reduction flags were defended while the command surface was not.
///
/// A gate that guards one of two surfaces leaves the other free to drift, and the
/// undefended one is where the drift shows up.
#[test]
fn every_command_path_is_named_in_every_catalog_document() {
    let paths = command_paths();
    assert!(
        paths.len() >= 15,
        "wire_name yielded only {} names; the extraction is broken, not the docs",
        paths.len()
    );

    let nested = nested_parents();
    let mut problems = Vec::new();
    for doc in CATALOG_DOCS {
        let text = read(doc);
        for p in &paths {
            // A nested path must appear as the invocation `cache path`, never as the
            // wire alias `cache-path`. Accepting the alias let the schema-alias line
            // — `plus aliases (cache-path, cache-clear, …)` — satisfy this gate while
            // the README taught the command nowhere. Naming a schema is not teaching
            // a command.
            let is_nested = nested
                .iter()
                .any(|parent| p.starts_with(&format!("{parent}-")));
            let found = if is_nested {
                text.contains(&p.replace('-', " "))
            } else {
                text.contains(p.as_str())
            };
            if !found {
                problems.push(format!("{doc}: never names `{p}` as an invocation"));
            }
        }
    }
    assert_clean("command paths in catalog docs", &problems);
}

/// Every versioned schema must be reachable from the agent contract.
///
/// The meta block listed 13 `schema --cmd` invocations while `docs/schemas/` held
/// 20: an agent reading only `AGENTS.md` could not learn that `agent-surface` or
/// `config-show` were servable at all.
#[test]
fn every_schema_is_named_in_the_agent_contract() {
    let mut problems = Vec::new();
    for path in files_under("docs/schemas", ".schema.json") {
        let name = rel_of(&path)
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .trim_end_matches(".schema.json")
            .to_string();
        for doc in AGENT_DOCS {
            if !read(doc).contains(&format!("schema --cmd {name} ")) {
                problems.push(format!("{doc}: schema {name} is never shown"));
            }
        }
    }
    assert_clean("schemas in agent contract", &problems);
}

/// A prose count of the schema bundle must equal the number of files on disk.
///
/// `agent-surface` became the twentieth schema and three documents kept saying
/// nineteen — including `llms.txt` and `llms-full.txt`, which exist precisely so
/// an agent can learn the surface without reading the tree. Every audit before
/// this one verified that `schema --cmd all` returned as many entries as
/// `docs/schemas/` held, and that check passed every time: both sides were 20.
/// Nobody compared either against the sentence describing them.
///
/// Spelled-out numerals are included because two of the four wrong counts were
/// words, and a digits-only scan would have reported green.
/// The separator is a character class, not a space, because `19-name bundle`
/// walked past a `\s+` scan on 2026-08-10 while three documents said 19 and the
/// tree held 20. A hyphen is not a different claim, only a different typography,
/// and a gate that reads typography is counting the wrong thing.
///
/// The noun list is broad and the false-positive guard is the caller requiring
/// `schema` on the line, which is cheaper to keep true than an ever-growing list
/// of the ways a bundle size can be spelled.
pub(super) fn count_pattern() -> Regex {
    Regex::new(
        r"(?i)\b(\d{1,3}|nineteen|twenty|twenty-one|dezenove|vinte e um|vinte)[-\s]+(?:wire\s+)?(?:names?|nomes?|schemas?|files?|arquivos?)\b",
    )
    .expect("static regex")
}

/// Markdown emphasis is typography, not content.
///
/// Measured on 2026-08-10: `docs/MIGRATION.md` said ``Offline schemas: **19**
/// wire names`` and the gate reported green, because `[-\s]+` cannot cross the
/// `**` that closes the bold run. A count does not stop being a count by being
/// bold, exactly as `19-name` did not stop being a count by being hyphenated.
pub(super) fn strip_emphasis(line: &str) -> String {
    line.replace(['*', '_'], " ")
}

/// A heading naming a version other than the manifest's opens a historical section.
///
/// `MIGRATION.md` documents each past release under its own `##` heading, and the
/// bundle really did hold nineteen schemas at 1.2.0. Widening the pattern without
/// this exemption would turn a truthful migration note into a gate failure, which
/// is how gates get deleted instead of fixed.
fn heading_is_historical(heading: &str, current: &str) -> bool {
    let semver = Regex::new(r"\b\d+\.\d+(?:\.\d+|\.x)\b").expect("static regex");
    match semver.find(heading) {
        Some(m) => m.as_str() != current,
        None => false,
    }
}

#[test]
fn no_document_miscounts_the_schema_bundle() {
    let on_disk = files_under("docs/schemas", ".schema.json").len();
    let counted = count_pattern();
    let current = super::manifest_version();

    let mut problems = Vec::new();
    for (rel, text) in current_docs() {
        let mut heading = String::new();
        for (n, line) in numbered(&text) {
            if line.starts_with('#') {
                heading = line.to_string();
            }
            if heading_is_historical(&heading, &current) {
                continue;
            }
            if !line.to_lowercase().contains("schema") {
                continue;
            }
            let flat = strip_emphasis(line);
            for cap in counted.captures_iter(&flat) {
                let tok = cap[1].to_lowercase();
                let got = match tok.as_str() {
                    "nineteen" | "dezenove" => 19,
                    "twenty" | "vinte" => 20,
                    "twenty-one" | "vinte e um" => 21,
                    d => match d.parse::<usize>() {
                        Ok(v) => v,
                        Err(_) => continue,
                    },
                };
                if got != on_disk {
                    problems.push(format!(
                        "{rel}:{n}: counts {tok} schemas but docs/schemas holds {on_disk}"
                    ));
                }
            }
        }
    }
    assert_clean("schema bundle count", &problems);
}

/// The count pattern must match every spelling that has appeared in this repo.
///
/// Written after the pattern was widened to accept `19-name` and the widened
/// gate still reported green against a document that literally said
/// `19-name offline/runtime bundle`. The reason is worth keeping: the pattern
/// was assembled with a backslash line-continuation inside a **raw** string,
/// where `\` is not an escape — so the regex carried a literal backslash and a
/// newline and matched nothing at all. It had been silently matching nothing
/// since the day it was ported.
///
/// A gate that matches nothing passes every run. This test is the only thing
/// that can tell that apart from a clean repository.
#[test]
fn the_count_pattern_matches_every_spelling_seen_here() {
    let re = count_pattern();
    for s in [
        "the full 19-name offline/runtime bundle",
        "the whole bundle: 20 schemas in one call",
        "MUST know the twenty schema names",
        "DEVE conhecer os vinte nomes de schema",
        "(20 nomes wire incluindo aliases)",
        "for the 20-name wire bundle",
    ] {
        assert!(re.is_match(s), "count pattern misses a real spelling: {s}");
    }
    // Bold is typography. Without `strip_emphasis` the gate is blind to it, and a
    // wrong count written in bold passes forever.
    assert!(
        re.is_match(&strip_emphasis("Offline schemas: **19** wire names")),
        "count pattern misses a bold count"
    );
    assert!(
        heading_is_historical("## 1.2.0 breaking (from 1.1.x)", "1.3.0"),
        "a heading naming an older release must open a historical section"
    );
    assert!(
        !heading_is_historical("## Commands", "1.3.0"),
        "a heading naming no version must never be historical"
    );
    assert!(
        !re.is_match("no bundle size on this line"),
        "count pattern matches a line with no count"
    );
}

/// Every `--test <name>` a document tells the reader to run must exist.
///
/// Measured on 2026-08-10: `docs/TESTING.md` and its pt-BR pair listed
/// `cargo test --locked --test cli_smoke` and `--test http_integration` in the
/// "How to Run" block. Both files had been removed from `tests/`, so the two
/// commands failed on the spot — the first two commands a new contributor copies
/// out of the testing guide.
///
/// Same class as every other finding this round: a document asserting something
/// about the tree without reading it. The expectation is derived from `tests/`,
/// so renaming a suite is what puts it in scope.
#[test]
fn every_documented_test_target_exists() {
    let existing: Vec<String> = files_under("tests", ".rs")
        .iter()
        .filter_map(|p| {
            let rel = rel_of(p);
            let name = rel.strip_prefix("tests/")?.strip_suffix(".rs")?;
            (!name.contains('/')).then(|| name.to_string())
        })
        .collect();

    let invocation = Regex::new(r"--test\s+([a-z0-9_]+)").expect("static regex");
    let mut problems = Vec::new();
    for (rel, text) in current_docs() {
        for (n, line) in numbered(&text) {
            for cap in invocation.captures_iter(line) {
                let want = &cap[1];
                if !existing.iter().any(|e| e == want) {
                    problems.push(format!(
                        "{rel}:{n}: tells the reader to run --test {want}, which does not exist"
                    ));
                }
            }
        }
    }
    assert_clean("documented test targets", &problems);
}

/// Every gate under `scripts/` must be named in the maintainer checklists.
///
/// `scripts/` held four gates while the two documents that say what to run named
/// exactly one of them. With no CI by product policy, a gate nobody invokes
/// enforces nothing — verbatim the defect GAP-SUPPLY-001 was opened for,
/// reproduced one level up by the gate written to fix it.
///
/// The expectation derives from the directory, so adding a script is what puts it
/// in scope.
#[test]
fn every_gate_script_is_named_in_the_checklists() {
    const CHECKLISTS: &[(&str, &[&str])] = &[
        ("en", &["CONTRIBUTING.md", "docs/TESTING.md"]),
        ("pt-BR", &["CONTRIBUTING.pt-BR.md", "docs/TESTING.pt-BR.md"]),
    ];
    let scripts: Vec<String> = files_under("scripts", ".sh")
        .iter()
        .map(|p| rel_of(p).rsplit('/').next().unwrap_or_default().to_string())
        .collect();

    let mut problems = Vec::new();
    for (lang, files) in CHECKLISTS {
        let blob: String = files
            .iter()
            .map(|f| fs::read_to_string(super::repo_root().join(f)).unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        for s in &scripts {
            if !blob.contains(s.as_str()) {
                problems.push(format!(
                    "scripts/{s}: named in no {lang} checklist ({})",
                    files.join(", ")
                ));
            }
        }
    }
    assert_clean("gate scripts in checklists", &problems);
}

/// The Rust policy suite must be named in the checklists too.
///
/// Porting the gates out of `scripts/` would otherwise have removed them from the
/// very checklist rule that exists to keep gates invokable — closing a gap by
/// making its subject invisible is not closing it.
#[test]
fn the_policy_suite_is_named_in_the_checklists() {
    const FILES: &[&str] = &[
        "CONTRIBUTING.md",
        "docs/TESTING.md",
        "CONTRIBUTING.pt-BR.md",
        "docs/TESTING.pt-BR.md",
    ];
    let mut problems = Vec::new();
    for f in FILES {
        if !read(f).contains("policy_gates") {
            problems.push(format!("{f}: never names `cargo test --test policy_gates`"));
        }
    }
    assert_clean("policy suite in checklists", &problems);
}
