//! What may enter the published crate, and what may only pretend to say so.
//!
//! The manifest used to govern packaging with `exclude`, and it failed the only
//! way a denylist can: eight audit scratch files shipped inside the crate because
//! none of their names was on the list. The remedy at the time was to add eight
//! more names — the same list, one sample longer (GAP-PKG-JUNK-001).
//!
//! An allowlist inverts the failure mode from quiet to loud, and these gates make
//! the loud version arrive at `cargo test` rather than at publish time. That
//! matters because the noisy failure is real: twenty JSON schemas are compiled
//! into the binary with `include_str!`, so dropping `/docs/**` from `include`
//! breaks the packaged build — a fact no other test in this tree could notice.

use super::{assert_clean, product_source, read};
use regex::Regex;

/// Every pattern of the manifest's `include` array, leading slash stripped.
///
/// Parsed from the text rather than through a TOML crate on purpose: this suite
/// has no dependency on one, and adding a parser to read a flat array of string
/// literals would be a dependency bought to avoid four lines.
fn include_patterns() -> Vec<String> {
    let manifest = read("Cargo.toml");
    let start = manifest
        .find("\ninclude = [")
        .expect("Cargo.toml declares no top-level `include` array");
    let rest = &manifest[start + "\ninclude = [".len()..];
    let end = rest.find(']').expect("the `include` array is never closed");
    let quoted = Regex::new(r#""([^"]+)""#).expect("static regex");
    quoted
        .captures_iter(&rest[..end])
        .map(|c| c[1].trim_start_matches('/').to_string())
        .collect()
}

/// Does the allowlist cover this repo-relative path?
///
/// Only the two grammars this manifest actually uses are understood: a directory
/// glob ending in `/**`, and an exact file name. Anything else returns `Err`, so
/// an unrecognised pattern fails the gate instead of being waved through — a
/// matcher that silently answers "covered" for syntax it cannot read is how a
/// gate reports green while guarding nothing.
fn covered_by(patterns: &[String], rel: &str) -> Result<bool, String> {
    let mut covered = false;
    for pattern in patterns {
        if let Some(dir) = pattern.strip_suffix("/**") {
            if rel.starts_with(&format!("{dir}/")) {
                covered = true;
            }
        } else if pattern.contains('*') {
            return Err(format!(
                "include pattern `{pattern}` uses a glob this gate cannot read; \
                 teach `covered_by` the shape or spell the entry out"
            ));
        } else if pattern == rel {
            covered = true;
        }
    }
    Ok(covered)
}

/// Repo-relative paths that `src/` pulls in at compile time.
///
/// Derived from the source, so a new `include_str!` enters scope by existing.
/// The path in the macro is relative to the *file* that writes it, which is why
/// this resolves against each source file's directory instead of the repo root.
fn embedded_paths() -> Vec<(String, String)> {
    let macro_call =
        Regex::new(r#"include_(?:str|bytes)!\s*\(\s*"([^"]+)""#).expect("static regex");
    let mut out = Vec::new();
    for (rel, text) in product_source() {
        for cap in macro_call.captures_iter(&text) {
            if let Some(resolved) = resolve_relative(&rel, &cap[1]) {
                out.push((rel.clone(), resolved));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Resolve `target` against the directory of `from`, collapsing `..` logically.
///
/// Logical and not `canonicalize`, because the answer must stay repo-relative and
/// must not depend on symlinks the filesystem happens to have.
fn resolve_relative(from: &str, target: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop()?;
    for segment in target.split('/') {
        match segment {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// Packaging must be an allowlist, and `exclude` must be gone rather than idle.
///
/// Keeping both is worse than keeping the wrong one: Cargo ignores `exclude`
/// entirely once `include` exists, so the abandoned rules would still read as
/// policy while enforcing nothing — a second copy of the truth, and the copy is
/// always the one that goes stale.
#[test]
fn the_manifest_uses_an_allowlist_and_not_a_denylist() {
    let manifest = read("Cargo.toml");
    let mut problems = Vec::new();
    if !manifest.contains("\ninclude = [") {
        problems.push("Cargo.toml has no `include` array; packaging is a denylist".to_string());
    }
    if manifest.contains("\nexclude = [") {
        problems.push(
            "Cargo.toml declares `exclude` beside `include`; Cargo discards it in silence"
                .to_string(),
        );
    }
    assert_clean("package allowlist", &problems);
}

/// No file compiled into the binary may be missing from the published crate.
///
/// This is the class gate. `src/meta_cmds/schema.rs` embeds all twenty schemas
/// from `docs/schemas/`, so an allowlist that forgets `/docs/**` produces a crate
/// that cannot build from the tarball — visible only to whoever runs
/// `cargo publish --dry-run`, and invisible to every test here until now.
#[test]
fn no_embedded_file_escapes_the_package_allowlist() {
    let patterns = include_patterns();
    assert!(
        !patterns.is_empty(),
        "no include patterns parsed; the extraction is broken, not the manifest"
    );
    let embedded = embedded_paths();
    assert!(
        !embedded.is_empty(),
        "no include_str!/include_bytes! found under src/; the extraction is broken"
    );

    let mut problems = Vec::new();
    for (source, path) in &embedded {
        match covered_by(&patterns, path) {
            Ok(true) => {}
            Ok(false) => problems.push(format!(
                "{source}: embeds `{path}`, which no `include` pattern covers"
            )),
            Err(e) => problems.push(e),
        }
    }
    assert_clean("embedded files inside the allowlist", &problems);
}

/// The extraction must find the dependency this gate exists for.
///
/// Two gates in this repository shipped matching nothing and reported green for
/// months. A scan that can return an empty set gets a canary by default.
#[test]
fn the_embedded_scan_finds_the_schemas_it_was_written_for() {
    let embedded = embedded_paths();
    assert!(
        embedded.iter().any(|(_, p)| p.starts_with("docs/schemas/")),
        "no embedded path under docs/schemas/: {embedded:?}"
    );
    // `..` must actually climb, or every path would resolve inside `src/` and the
    // gate would be asking whether `/src/**` covers files it never names.
    assert_eq!(
        resolve_relative("src/meta_cmds/schema.rs", "../../docs/schemas/x.json").as_deref(),
        Some("docs/schemas/x.json"),
        "relative resolution no longer climbs out of src/"
    );

    let patterns = include_patterns();
    assert_eq!(
        covered_by(&patterns, "docs/naoexiste/inventado.json"),
        Ok(true),
        "the contra-canary assumes /docs/** covers any docs path"
    );
    // The integration suite must stay out while its fixtures stay in: the
    // fixtures are compile-time inputs of `src/`, the suite is not.
    assert_eq!(
        covered_by(&patterns, "tests/policy/contract.rs"),
        Ok(false),
        "the allowlist covers the policy suite, which answers nothing in a tarball"
    );
    assert_eq!(
        covered_by(&patterns, "tests/fixtures/crates_io/search_serde.json"),
        Ok(true),
        "the allowlist lost the fixtures that src/ embeds at compile time"
    );
    assert!(
        covered_by(&["*.json".to_string()], "a.json").is_err(),
        "an unreadable glob is being waved through instead of failing the gate"
    );
}

/// `.cargoignore` may carry an explanation and never a rule.
///
/// The file mirrored `.gitignore` byte for byte and shipped inside the crate,
/// looking exactly like packaging policy while Cargo never read a line of it.
/// Emptying it fixes today; this gate stops the rules from creeping back.
#[test]
fn the_cargoignore_carries_no_rules() {
    let text = read(".cargoignore");
    let problems: Vec<String> = super::numbered(&text)
        .filter(|(_, line)| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .map(|(n, line)| {
            format!(".cargoignore:{n}: `{line}` reads as a rule Cargo will never apply")
        })
        .collect();
    assert_clean("cargoignore rules", &problems);

    // The warning is the file's only reason to exist; losing it turns an
    // explanation back into an empty file somebody will helpfully fill in.
    assert!(
        text.contains("include") && text.contains("Cargo.toml"),
        ".cargoignore no longer points the reader at the `include` allowlist"
    );
}
