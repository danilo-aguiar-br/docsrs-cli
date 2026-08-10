//! Documentation must agree with the manifest it describes.
//!
//! The shell predecessor of this gate matched three literal phrases:
//! `current product line`, `linha de produto atual` and
//! `version --json ... reports`. Those were the phrases that existed on the day
//! it was written, so it protected exactly them.
//!
//! Measured on 2026-08-10: `docs/AGENTS.md:10` said
//! ``Product line is `1.2.x` (`version` reports `1.2.0`)`` while the crate was at
//! 1.3.0, and line 52 of the same file told the reader to confirm `1.3.0`. The
//! document contradicted itself and the gate reported green, because
//! `Product line is` is not `current product line` and `` `version` reports `` is
//! not `version --json ... reports`.
//!
//! This version matches the *class*: any line that ties the product to a version
//! is a claim, unless it is explicitly historical.

use super::{assert_clean, files_under, is_historical, manifest_version, numbered, read, rel_of};
use regex::Regex;
use std::fs;

/// Markers that make a line a statement about the version in force *now*.
const CLAIM_MARKERS: &[&str] = &[
    "product line",
    "linha de produto",
    "current release",
    "release atual",
    "versão atual",
    "current version",
    "version --json",
    "dogfood",
    "version` reports",
    "version` reporta",
    "version reports",
    "version reporta",
];

/// Markers that make the same line a statement about the past.
///
/// These are what keep `fail-closed since 1.2.0` and `removed in 1.2.0` legal:
/// they are true sentences about history, and rewriting them to today's number
/// would turn correct documentation into a lie.
/// The series check below is deny-by-default, so this list is what keeps true
/// sentences about the past legal: migration entries, retained sections and
/// "earlier highlights" all name an old series on purpose.
const HISTORICAL_MARKERS: &[&str] = &[
    "since",
    "desde",
    "removed",
    "removido",
    "introduced",
    "introduzido",
    "added in",
    "legacy",
    "legado",
    "up to",
    "até a",
    "prior to",
    "anterior",
    "was ",
    "era ",
    "upgrad",
    "atualiz",
    "migrat",
    "migra",
    "earlier",
    "retain",
    "retid",
    "adicionad",
    "quebras",
    "breaking",
    "histór",
    "histor",
];

/// Phrases that assert a version is in force *now*, overriding a historical
/// marker on the same line.
///
/// `## Flags Added in 1.1.x (still current on 1.2.x)` is half history and half
/// claim, and the half that was wrong is the claim. `added in` alone exempted
/// the whole line, so the English heading passed while its pt-BR pair — which
/// says `adicionadas em`, matching no marker — failed. A gate whose verdict
/// depends on which language a sentence was written in is not checking the
/// sentence.
const CURRENCY_MARKERS: &[&str] = &[
    "still current",
    "ainda válid",
    "ainda valid",
    "ainda vale",
    "current line",
    "linha atual",
    "in force",
    "em vigor",
];

fn is_claim(lower: &str) -> bool {
    CLAIM_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_currency(lower: &str) -> bool {
    CURRENCY_MARKERS.iter().any(|m| lower.contains(m))
}

fn is_history(lower: &str) -> bool {
    HISTORICAL_MARKERS.iter().any(|m| lower.contains(m)) && !is_currency(lower)
}

/// Direct dependency names, read from the manifest.
///
/// A version series is only a claim about *this* product. `rustls 0.23.x` in ADR
/// 0007 is a true statement about a dependency, and deny-by-default flagged it
/// the moment the series check stopped requiring a claim phrase. The exemption is
/// derived from `Cargo.toml` rather than listed here, so adding a dependency
/// never needs a gate edit.
fn dependency_names() -> Vec<String> {
    let manifest = read("Cargo.toml");
    let mut names = Vec::new();
    let mut inside = false;
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            inside = trimmed.contains("dependencies");
            continue;
        }
        if !inside || trimmed.starts_with('#') {
            continue;
        }
        if let Some(name) = trimmed.split('=').next() {
            let name = name.trim();
            if !name.is_empty() && !name.contains(' ') {
                names.push(name.to_lowercase());
            }
        }
    }
    names
}

/// Files a reader may take as current fact about the release.
///
/// `docs/schemas/*.json` is included because those files are the machine-readable
/// contract: `schema.schema.json` described the product line in a `description`
/// field, which no markdown-only scan would ever have opened.
fn version_scoped_files() -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = super::current_docs();
    for path in files_under("docs/schemas", ".json") {
        let rel = rel_of(&path);
        if is_historical(&rel) {
            continue;
        }
        out.push((rel, fs::read_to_string(&path).unwrap_or_default()));
    }
    out
}

/// A line that ties the product to a full semver must carry the manifest version.
#[test]
fn no_document_claims_a_stale_release_version() {
    let version = manifest_version();
    let semver = Regex::new(r"\b\d+\.\d+\.\d+\b").expect("static regex");
    let mut problems = Vec::new();

    for (rel, text) in version_scoped_files() {
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            if !is_claim(&lower) || is_history(&lower) {
                continue;
            }
            let found: Vec<&str> = semver.find_iter(line).map(|m| m.as_str()).collect();
            if !found.is_empty() && !found.contains(&version.as_str()) {
                problems.push(format!(
                    "{rel}:{n}: claims {found:?} but Cargo.toml is {version}"
                ));
            }
        }
    }
    assert_clean("release version claims", &problems);
}

/// A line that ties the product to a `MAJOR.MINOR.x` series must carry the
/// manifest's series.
///
/// Separate from the check above because `1.2.x` is not a semver triple: the
/// digits-only scan walked straight past it, which is how the same sentence
/// managed to be wrong twice in two different ways.
///
/// **Deny-by-default, unlike the triple check.** The module header above claims
/// this gate "matches the class: any line that ties the product to a version is
/// a claim, unless it is explicitly historical" — and the first implementation
/// did no such thing. It required a phrase from `CLAIM_MARKERS`, which is an
/// allowlist of the ways a claim had been *worded so far*. So the rustdoc
/// described the class while the code enforced a list, and on 2026-08-10
/// `docs/schemas/README.md:11` said `Release 1.2.x network schemas require
/// cache_hit` at 1.3.0 and the gate reported green for the third time in this
/// family.
///
/// The asymmetry is the point: ways to *assert* something are open-ended and no
/// list can close them, while ways to *mark the past* are few and stable. A gate
/// belongs on the finite side. `MAJOR.MINOR.x` is safe to invert because, unlike
/// a semver triple, it is never a dependency pin or an MSRV.
#[test]
fn no_document_claims_a_stale_release_series() {
    let version = manifest_version();
    let mut parts = version.split('.');
    let series = format!(
        "{}.{}",
        parts.next().unwrap_or_default(),
        parts.next().unwrap_or_default()
    );
    let series_pat = Regex::new(r"\b(\d+\.\d+)\.x\b").expect("static regex");
    let deps = dependency_names();
    let mut problems = Vec::new();

    for (rel, text) in version_scoped_files() {
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            if is_history(&lower) {
                continue;
            }
            if deps.iter().any(|d| lower.contains(d.as_str())) {
                continue;
            }
            for cap in series_pat.captures_iter(line) {
                let got = &cap[1];
                if got != series {
                    problems.push(format!(
                        "{rel}:{n}: claims series {got}.x but Cargo.toml is {version}"
                    ));
                }
            }
        }
    }
    assert_clean("release series claims", &problems);
}

/// A positive `docs.rs build` cell contradicts an empty `targets` list.
///
/// Saying `no (ring builds C)` in that column is the truth the manifest states,
/// so banning the phrase outright would forbid documenting reality. Only a
/// positive claim is a defect.
///
/// The claim lives in the table *body*, never in the header naming the column, so
/// the column index is taken from the header and applied to the rows beneath it.
/// A first version scanned one line at a time and proved nothing, because
/// `docs.rs build` and `yes` are never on the same line.
#[test]
fn no_document_claims_a_docs_rs_build_the_manifest_disables() {
    if !read("Cargo.toml")
        .lines()
        .any(|l| l.trim() == "targets = []")
    {
        return;
    }
    let mut problems = Vec::new();
    for (rel, text) in super::current_docs() {
        let mut col: Option<usize> = None;
        for (n, line) in numbered(&text) {
            let cells: Vec<String> = line.split('|').map(|c| c.trim().to_lowercase()).collect();
            if let Some(i) = cells.iter().position(|c| c == "docs.rs build") {
                col = Some(i);
                continue;
            }
            let Some(i) = col else { continue };
            if !line.contains('|') {
                continue;
            }
            if cells.get(i).is_some_and(|c| c == "yes" || c == "sim") {
                problems.push(format!(
                    "{rel}:{n}: claims a docs.rs build while Cargo.toml has targets = []"
                ));
            }
        }
    }
    assert_clean("docs.rs build claims", &problems);
}

/// The gate must reject the exact shapes that escaped the shell version.
///
/// Without this, "we fixed the regex" is an assertion about a regex nobody
/// re-tested. These are the two real sentences, verbatim.
#[test]
fn the_claim_predicate_catches_the_shapes_that_escaped() {
    let escaped = [
        "- Product line is `1.2.x` (`version` reports `1.2.0`)",
        "- A linha de produto é `1.2.x` (`version` reporta `1.2.0`)",
        "> **Dogfood 1.2.0:** always invoke ./target/release/docsrs-cli",
    ];
    for line in escaped {
        let lower = line.to_lowercase();
        assert!(
            is_claim(&lower) && !is_history(&lower),
            "predicate misses a real claim: {line}"
        );
    }

    let history = [
        "- MUST reject method success when `extraction` is legacy (fail-closed since 1.2.0)",
        "# never treat extraction=item_page as method success (removed in 1.2.0)",
    ];
    for line in history {
        let lower = line.to_lowercase();
        assert!(
            is_history(&lower),
            "predicate would rewrite true history: {line}"
        );
    }
}
