//! Shared helpers for the policy gates.
//!
//! These gates used to live in `scripts/check-policy.sh`: 540 lines of bash
//! wrapping 260 lines of inline Python. That violated the product's full-stack
//! Rust rule outright, and it carried a consequence nobody had written down —
//! the crate promises Linux, macOS and Windows, while its own policy was only
//! verifiable where `bash`, `python3`, `fd` and `rg` all happened to exist. A
//! Windows maintainer could not run a single gate.
//!
//! As Rust integration tests they run inside `cargo test`, which every
//! contributor already runs, on every platform the crate targets.
//!
//! The design rule carried over from the shell version is the one that matters:
//! **every expectation is derived from the tree, never from a list kept here.**
//! A hand-kept list is a second copy of the truth, and the copy is always the
//! one that goes stale.

mod contract;
mod docs;
mod env;
mod envelopes;
mod etd;
mod gaps;
mod inventory;
mod packaging;
mod surface;
mod targets;
mod tls;
mod version;

use std::fs;
use std::path::{Path, PathBuf};

/// Repository root, resolved at compile time so the gates do not depend on the
/// working directory a test runner happens to pick.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Read a repo-relative file, failing with the path when it is missing.
///
/// A gate that silently skips an unreadable file reports green for the wrong
/// reason, so this panics instead of returning an `Option`.
pub fn read(rel: &str) -> String {
    let path = repo_root().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// Repo-relative path in POSIX spelling, so gate messages read the same on
/// Windows as they do on Linux.
pub fn rel_of(path: &Path) -> String {
    path.strip_prefix(repo_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Recursively collect files under `dir` whose name ends with `suffix`.
///
/// Replaces the `fd` invocations the shell version relied on. Sorted, because a
/// gate that reports its findings in filesystem order is a gate whose output
/// changes between machines for no reason.
pub fn files_under(dir: &str, suffix: &str) -> Vec<PathBuf> {
    fn walk(dir: &Path, suffix: &str, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, suffix, out);
            } else if path.to_string_lossy().ends_with(suffix) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(&repo_root().join(dir), suffix, &mut out);
    out.sort();
    out
}

/// Historical records are exempt from every "must match today" gate.
///
/// They document what a past release said, and rewriting them to match the
/// present would destroy the audit trail they exist for. `gaps.md` belongs here
/// for the same reason as the changelogs: it records what was true when each gap
/// was opened, including the wrong version a document used to claim.
pub const HISTORICAL: &[&str] = &[
    "CHANGELOG.md",
    "CHANGELOG.pt-BR.md",
    "docs/MIGRATION.md",
    "docs/MIGRATION.pt-BR.md",
    "gaps.md",
];

pub fn is_historical(rel: &str) -> bool {
    HISTORICAL.contains(&rel)
}

/// Every prose document an agent or a human may read as current fact.
///
/// Root markdown, everything under `docs/` and `skills/`, plus the `llms*.txt`
/// bundles, which exist precisely so an agent can learn the surface without
/// reading the tree — and which therefore drift exactly like the docs do.
pub fn prose_docs() -> Vec<PathBuf> {
    let root = repo_root();
    let mut docs: Vec<PathBuf> = fs::read_dir(&root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            p.is_file() && (name.ends_with(".md") || name.starts_with("llms"))
        })
        .filter(|p| {
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            name.ends_with(".md") || name.ends_with(".txt")
        })
        .collect();
    docs.extend(files_under("docs", ".md"));
    docs.extend(files_under("skills", ".md"));
    docs.sort();
    docs.dedup();
    docs
}

/// Every product source file, paired with its repo-relative path.
///
/// Rustdoc is prose that ships. `src/http/mod.rs` opened with a module header
/// asserting `provider=ring`, `floor ≥0.23.18` and `webpki-roots` — the three
/// facts the TLS gates exist to defend — and no gate ever opened the file,
/// because the scan stopped at `.md`. A claim does not stop being a claim by
/// being written above a `mod` keyword.
pub fn product_source() -> Vec<(String, String)> {
    files_under("src", ".rs")
        .into_iter()
        .map(|p| (rel_of(&p), fs::read_to_string(&p).unwrap_or_default()))
        .collect()
}

/// Documents that are current fact, paired with their repo-relative path.
pub fn current_docs() -> Vec<(String, String)> {
    prose_docs()
        .into_iter()
        .map(|p| (rel_of(&p), fs::read_to_string(&p).unwrap_or_default()))
        .filter(|(rel, _)| !is_historical(rel))
        .collect()
}

/// Numbered lines, 1-based, the way every gate message quotes them.
pub fn numbered(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines().enumerate().map(|(i, l)| (i + 1, l))
}

/// Fail the calling gate with one line per problem.
///
/// Collecting every problem before failing is deliberate: a gate that stops at
/// the first finding turns one fix-and-rerun cycle into N of them.
pub fn assert_clean(gate: &str, problems: &[String]) {
    assert!(
        problems.is_empty(),
        "{gate}: {} problem(s)\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// The `version` field of the manifest, which is the single source for every
/// claim a document makes about the current release.
pub fn manifest_version() -> String {
    let manifest = read("Cargo.toml");
    for line in manifest.lines() {
        if let Some(rest) = line.strip_prefix("version = \"") {
            if let Some(v) = rest.split('"').next() {
                return v.to_string();
            }
        }
    }
    panic!("Cargo.toml has no top-level version");
}
