//! Cross-target matrix and file-size gates.

use super::{assert_clean, files_under, read, rel_of};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;

/// The targets the cross gate actually checks, read from its own array.
fn gate_targets() -> BTreeSet<String> {
    let script = read("scripts/check-targets.sh");
    let block = Regex::new(r"(?ms)^targets=\((.*?)^\)")
        .expect("static regex")
        .captures(&script)
        .map(|c| c[1].to_string())
        .expect("scripts/check-targets.sh has no targets=( ... ) array");
    block
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// The support matrix and the cross-target gate must list the same targets.
///
/// They did not. `docs/CROSS_PLATFORM.md` claimed `aarch64-unknown-linux-gnu` as
/// supported while `scripts/check-targets.sh` never looked at it, and the gate
/// checked `x86_64-pc-windows-gnu`, which the matrix never mentioned. Two lists
/// answering "which targets does this product support", never compared, so each
/// was free to be wrong exactly where the other would have caught it.
#[test]
fn the_support_matrix_and_the_cross_gate_agree() {
    let gate = gate_targets();
    let row = Regex::new(r"(?m)^\| `([a-z0-9_]+-[a-z0-9-]+)`").expect("static regex");
    let mut problems = Vec::new();

    for matrix in ["docs/CROSS_PLATFORM.md", "docs/CROSS_PLATFORM.pt-BR.md"] {
        let text = read(matrix);
        let rows: BTreeSet<String> = row.captures_iter(&text).map(|c| c[1].to_string()).collect();
        for missing in gate.difference(&rows) {
            problems.push(format!(
                "{matrix}: check-targets.sh checks {missing}, the matrix omits it"
            ));
        }
        for extra in rows.difference(&gate) {
            problems.push(format!(
                "{matrix}: the matrix claims {extra}, check-targets.sh never checks it"
            ));
        }
    }
    assert_clean("cross-target matrix", &problems);
}

/// Every host tool the cross gate requires, read from `cross_tools_for`.
///
/// `cc` is dropped: it is the catch-all arm for a target that needs nothing
/// special, not a tool a reader has to go install.
fn required_host_tools() -> BTreeSet<String> {
    let script = read("scripts/check-targets.sh");
    let block = Regex::new(r"(?ms)^cross_tools_for\(\)\s*\{(.*?)^\}")
        .expect("static regex")
        .captures(&script)
        .map(|c| c[1].to_string())
        .expect("scripts/check-targets.sh has no cross_tools_for() function");

    let arm = Regex::new(r#"echo\s+"?([a-z0-9 _-]+)"?\s*;;"#).expect("static regex");
    arm.captures_iter(&block)
        .flat_map(|c| {
            c[1].split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|t| t != "cc")
        .collect()
}

/// The support matrix must name every host tool that decides target coverage.
///
/// Measured on 2026-08-10: `gaps.md` recorded `checked=4 skipped=2
/// cross_checked=2` in the `### Estado` of GAP-APPLE-CAUSE-001, and the same
/// host produced `checked=2 skipped=4 cross_checked=1` the next day. Nothing
/// regressed in this crate. `zig` had left the machine, and `cargo-zigbuild`
/// alone does not build anything.
///
/// The written counters were never a property of the repository: they describe
/// which toolchains happened to be installed when someone ran the script. The
/// durable fact is the dependency itself, and it belonged in the matrix a reader
/// consults — where, until this gate, not one of the five tools was named.
///
/// Same class as GAP-TRACE-001: a number written by hand that ages in silence.
/// The remedy is the same one that closed it — stop writing the number, derive
/// the thing the number was standing in for.
#[test]
fn the_support_matrix_names_every_host_tool_the_cross_gate_requires() {
    let tools = required_host_tools();
    assert!(
        tools.len() >= 3,
        "only {} host tools extracted from cross_tools_for(); the regex is broken, not the docs",
        tools.len()
    );

    let mut problems = Vec::new();
    for matrix in ["docs/CROSS_PLATFORM.md", "docs/CROSS_PLATFORM.pt-BR.md"] {
        let text = read(matrix);
        for tool in &tools {
            if !text.contains(tool.as_str()) {
                problems.push(format!(
                    "{matrix}: check-targets.sh requires {tool} on the host, the matrix never names it"
                ));
            }
        }
    }
    assert_clean("host tools in the support matrix", &problems);
}

/// Single-responsibility file-size ceiling, in PHYSICAL lines.
///
/// The metric used to be `tokei` (code + comments + blanks). tokei re-classifies
/// `///` and `//!` blocks as embedded Markdown and reports them on a separate
/// language row, so reading only the Rust row measured each file with its rustdoc
/// deleted. The stated intent was "documenting an item must never push its file
/// over the limit", but the effect was an inverted incentive: the better a file
/// was documented, the more of it became invisible. Three files were over the
/// ceiling while the gate returned 0.
///
/// A file that only exceeds the ceiling because of rustdoc is still a file that
/// does too much: the fix is to split by responsibility, never to hide the lines.
#[test]
fn no_source_file_exceeds_the_line_ceiling() {
    const MAX: usize = 500;
    let mut over: Vec<(usize, String)> = Vec::new();
    let mut sources = files_under("src", ".rs");
    sources.extend(files_under("tests", ".rs"));

    for path in sources {
        let n = fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .count();
        if n > MAX {
            over.push((n, rel_of(&path)));
        }
    }
    over.sort_by(|a, b| b.0.cmp(&a.0));
    let problems: Vec<String> = over
        .iter()
        .map(|(n, f)| format!("{f}: {n} lines (ceiling {MAX}); split by responsibility"))
        .collect();
    assert_clean("file size ceiling", &problems);
}

/// The gate scripts that remain must stay small enough to read.
///
/// `scripts/check-policy.sh` reached 540 lines — over the ceiling the product
/// applies to its own source, and the largest file in the tree. It is gone now,
/// but nothing stopped the next script from growing the same way.
#[test]
fn no_gate_script_exceeds_the_line_ceiling() {
    const MAX: usize = 500;
    let problems: Vec<String> = files_under("scripts", ".sh")
        .iter()
        .filter_map(|p| {
            let n = fs::read_to_string(p).unwrap_or_default().lines().count();
            (n > MAX).then(|| format!("{}: {n} lines (ceiling {MAX})", rel_of(p)))
        })
        .collect();
    assert_clean("gate script size", &problems);
}

/// A production path may not `unwrap` a fallible result.
///
/// Measured on 2026-08-10: 22 `expect` calls live outside `#[cfg(test)]`, and
/// every one of them parses a hardcoded selector or regex, with a message saying
/// so. None can fail on a value the network, the disk or the caller supplies. So
/// the tree is clean, and nothing was keeping it that way.
///
/// The exemption is the message, not the position: a call that declares which
/// literal it is parsing is auditable, and a bare `unwrap()` in the same place is
/// not — a reader cannot tell an infallible parse from a swallowed failure.
/// Rustdoc examples are excluded because they are prose about the API, and
/// `unwrap` is how a doctest keeps a three-line example three lines long.
#[test]
fn no_production_path_unwraps_a_fallible_result() {
    /// Phrases that mark a call as infallible by construction.
    const DECLARED_STATIC: &[&str] = &[
        "by construction",
        "static regex",
        "static test",
        "is valid",
        "infallible",
    ];

    // Test code is derived, not guessed by filename: a module declared under
    // `#[cfg(test)]` is test code wherever it lives, and `src/cli/tests.rs`
    // carries no `#[cfg(test)]` of its own because its parent carries it.
    let gated = Regex::new(r"(?m)#\[cfg\(test\)\]\s*\n\s*mod\s+([a-z_]+);").expect("static regex");
    let mut test_roots: Vec<String> = Vec::new();
    for path in files_under("src", ".rs") {
        let text = fs::read_to_string(&path).unwrap_or_default();
        let parent = rel_of(&path)
            .rsplit_once('/')
            .map_or(String::new(), |(d, _)| d.to_string());
        for cap in gated.captures_iter(&text) {
            test_roots.push(format!("{parent}/{}", &cap[1]));
        }
    }
    assert!(
        !test_roots.is_empty(),
        "no #[cfg(test)] module declarations found; the derivation is broken, not the tree"
    );

    let mut problems = Vec::new();
    for path in files_under("src", ".rs") {
        let rel = rel_of(&path);
        if test_roots
            .iter()
            .any(|r| rel == format!("{r}.rs") || rel.starts_with(&format!("{r}/")))
        {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        // Inline `#[cfg(test)] mod tests { … }` blocks end the production half.
        let production: Vec<&str> = text
            .lines()
            .take_while(|l| !l.contains("#[cfg(test)]"))
            .collect();
        for (i, line) in production.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("///") || trimmed.starts_with("//!") || trimmed.starts_with("//")
            {
                continue;
            }
            if !line.contains(".unwrap()") {
                continue;
            }
            let lower = line.to_lowercase();
            if DECLARED_STATIC.iter().any(|m| lower.contains(m)) {
                continue;
            }
            problems.push(format!(
                "{rel}:{}: bare unwrap() on a production path; use ? or expect() naming why it cannot fail",
                i + 1
            ));
        }
    }
    assert_clean("bare unwrap on production paths", &problems);
}
