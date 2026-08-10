//! Environment, i18n and build-surface gates.
//!
//! No product knob may come from the environment (ADR 0009), every user-facing
//! string must route through `src/i18n.rs`, and CI must not exist in-tree.

use super::{assert_clean, files_under, numbered, read, rel_of, repo_root};
use std::fs;

/// Lines that are entirely or partly a Rust comment.
///
/// The shell version filtered these with `rg -v '//!|///|// '`, and the filter is
/// load-bearing: the gates below are about what the binary *does*, while a
/// comment naming a forbidden variable is usually the comment explaining why it
/// is forbidden.
fn is_comment(line: &str) -> bool {
    line.contains("//!") || line.contains("///") || line.contains("// ")
}

/// Documents may not teach the removed path-sandbox environment variables as a
/// live feature. Saying they never worked, or were removed, is the truth.
#[test]
fn docs_do_not_teach_the_path_sandbox_env_as_live() {
    const LIVE_CLAIMS: &[&str] = &[
        "still allows",
        "ainda permite",
        "still work",
        "ainda funcionam",
        "Path sandbox still",
        "Path sandbox ainda",
    ];
    let mut problems = Vec::new();
    let mut targets: Vec<String> = vec![
        "README.md".into(),
        "README.pt-BR.md".into(),
        "CLAUDE.md".into(),
    ];
    targets.extend(files_under("docs", ".md").iter().map(|p| rel_of(p)));
    targets.extend(files_under("skills", ".md").iter().map(|p| rel_of(p)));

    for rel in targets {
        let Ok(text) = fs::read_to_string(repo_root().join(&rel)) else {
            continue;
        };
        for (n, line) in numbered(&text) {
            for claim in LIVE_CLAIMS {
                if line.contains(claim) {
                    problems.push(format!("{rel}:{n}: teaches removed env as live: {claim}"));
                }
            }
        }
    }
    assert_clean("path sandbox env", &problems);
}

/// No document may show a `DOCSRS_CLI_HOME=` invocation, unless the surrounding
/// sentence marks it as removed or forbidden.
#[test]
fn docs_carry_no_live_product_env_examples() {
    const EXEMPT: &[&str] = &[
        "NEVER",
        "removed",
        "nunca",
        "proibido",
        "forbidden",
        "historical",
    ];
    let mut problems = Vec::new();
    let mut targets: Vec<String> = vec!["CLAUDE.md".into()];
    targets.extend(files_under("docs", ".md").iter().map(|p| rel_of(p)));
    targets.extend(files_under("skills", ".md").iter().map(|p| rel_of(p)));

    for rel in targets {
        let Ok(text) = fs::read_to_string(repo_root().join(&rel)) else {
            continue;
        };
        for (n, line) in numbered(&text) {
            if line.contains("DOCSRS_CLI_HOME=") && !EXEMPT.iter().any(|e| line.contains(e)) {
                problems.push(format!("{rel}:{n}: live DOCSRS_CLI_HOME= example"));
            }
        }
    }
    assert_clean("product env examples", &problems);
}

/// Product source may name `DOCSRS_CLI_` only in comments explaining the ban.
#[test]
fn product_source_does_not_reference_product_env() {
    let mut problems = Vec::new();
    for path in files_under("src", ".rs") {
        let rel = rel_of(&path);
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in numbered(&text) {
            if line.contains("DOCSRS_CLI_") && !is_comment(line) {
                problems.push(format!(
                    "{rel}:{n}: references DOCSRS_CLI_ outside a comment"
                ));
            }
        }
    }
    assert_clean("product env reads", &problems);
}

/// CI is forbidden in-tree by product policy.
#[test]
fn no_ci_directory_exists() {
    assert!(
        !repo_root().join(".github").exists(),
        ".github must not exist: CI is forbidden in-tree"
    );
}

/// `eprintln!` with a literal first argument bypasses the i18n catalogue, so the
/// message ships English-only whatever `--lang` says.
///
/// The allowlist is deliberately empty. A new literal belongs in the catalogue,
/// never in a list here.
#[test]
fn no_literal_eprintln_bypasses_i18n() {
    let mut problems = Vec::new();
    for path in files_under("src", ".rs") {
        let rel = rel_of(&path);
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in numbered(&text) {
            if line.contains("eprintln!(\"") {
                problems.push(format!("{rel}:{n}: literal eprintln! bypasses src/i18n.rs"));
            }
        }
    }
    assert_clean("i18n bypass", &problems);
}

/// No product knob may be read from the environment.
///
/// The allowlist holds exactly one category: variables that describe the
/// *terminal device*, the way `isatty` does. They report what the output can
/// render and never carry configuration, and `--no-color` outranks all of them.
/// `CARGO_PKG_*` and `CARGO_BIN_EXE_*` are build metadata baked in at compile
/// time, not runtime reads.
///
/// `from_default_env` is matched by name because `EnvFilter::try_from_default_env`
/// reads `RUST_LOG` without ever naming it — a string allowlist alone would never
/// catch it, and an ambient value silently outranked an explicit `-q` until it
/// was removed.
///
/// The scan covers `tests/` as well as `src/`, and that width was bought with a
/// real miss: `tests/network_live.rs` gated nine live tests on
/// `DOCSRS_CLI_NETWORK_TESTS`, so `cargo test -- --ignored` returned early from
/// every one of them and counted them as passed without opening a socket.
#[test]
fn nothing_reads_the_environment_outside_the_allowlist() {
    const READS: &[&str] = &[
        "env::var",
        "env::var_os",
        "from_default_env",
        "option_env!",
        "env!(",
    ];
    let mut problems = Vec::new();
    let mut sources = files_under("src", ".rs");
    sources.extend(files_under("tests", ".rs"));

    for path in sources {
        let rel = rel_of(&path);
        // The gate module names every read pattern it hunts for, so scanning
        // itself is guaranteed self-indictment. Excluding it is not a loophole:
        // `tests/policy/` contains no product code and configures nothing.
        if rel.starts_with("tests/policy") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in numbered(&text) {
            if !READS.iter().any(|r| line.contains(r)) {
                continue;
            }
            if line.trim_start().starts_with("//") {
                continue;
            }
            if is_allowlisted_read(line) {
                continue;
            }
            problems.push(format!("{rel}:{n}: reads the environment: {}", line.trim()));
        }
    }
    assert_clean("environment reads", &problems);
}

/// True when the line names an allowlisted variable.
///
/// The test is "names an allowlisted variable", not "every quoted token on the
/// line is allowlisted". The stricter reading looks safer and is simply wrong:
/// `env::var_os("TERM").is_some_and(|t| t == "dumb")` also quotes `"dumb"`, and
/// `concat!("docsrs-cli ", env!("CARGO_PKG_VERSION"))` also quotes a prefix. Both
/// are legal reads, and a gate that flags them teaches maintainers to ignore it.
fn is_allowlisted_read(line: &str) -> bool {
    const EXACT: &[&str] = &["NO_COLOR", "TERM", "CLICOLOR_FORCE"];
    const PREFIX: &[&str] = &["CARGO_PKG_", "CARGO_BIN_EXE_", "CARGO_MANIFEST_DIR"];
    line.split('"')
        .skip(1)
        .step_by(2)
        .any(|q| EXACT.contains(&q) || PREFIX.iter().any(|p| q.starts_with(p)))
}

/// `sys_locale` consults `LC_ALL` / `LC_MESSAGES` / `LANG` to pick stderr prose
/// when no `--lang` is given. That is terminal-environment category, like `TERM`:
/// it steers human text and stdout stays English regardless.
///
/// Keeping the read in one module is what makes the claim auditable at all, so
/// the gate enforces the confinement rather than a (false) absence.
#[test]
fn locale_environment_reads_stay_inside_i18n() {
    let mut problems = Vec::new();
    for path in files_under("src", ".rs") {
        let rel = rel_of(&path);
        if rel.starts_with("src/i18n/") || rel == "src/i18n.rs" {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap_or_default();
        if text.contains("sys_locale") {
            problems.push(format!("{rel}: sys_locale must stay inside src/i18n/"));
        }
    }
    assert_clean("sys_locale confinement", &problems);
}

/// The gates themselves must not reintroduce the interpreters this port removed.
///
/// This is the gate the shell version could never carry: it was written in the
/// very languages it would have had to ban. `scripts/check-policy.sh` held 260
/// lines of inline Python while the product rule said full-stack Rust, and three
/// audits missed it because every audit pointed at `src/`, never at `scripts/`.
#[test]
fn no_shell_gate_depends_on_a_foreign_interpreter() {
    const BANNED: &[&str] = &["python3", "python ", "perl ", "ruby "];
    let mut problems = Vec::new();
    for path in files_under("scripts", ".sh") {
        let rel = rel_of(&path);
        let text = fs::read_to_string(&path).unwrap_or_default();
        for (n, line) in numbered(&text) {
            if line.trim_start().starts_with('#') {
                continue;
            }
            for banned in BANNED {
                if line.contains(banned) {
                    problems.push(format!(
                        "{rel}:{n}: invokes {}; gates are Rust (tests/policy_gates.rs)",
                        banned.trim()
                    ));
                }
            }
        }
    }
    assert_clean("foreign interpreters in gates", &problems);
}

/// Reading the manifest must never be a source of surprise for the gates that
/// derive expectations from it.
#[test]
fn manifest_version_is_readable() {
    let v = super::manifest_version();
    assert!(
        v.split('.').count() == 3,
        "Cargo.toml version is not a semver triple: {v}"
    );
    let _ = read("Cargo.toml");
}
