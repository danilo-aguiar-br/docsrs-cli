//! One-shot library and CLI for crates.io search and docs.rs documentation.
//!
//! `docsrs-cli` is agent-first: structured data on stdout, diagnostics on stderr,
//! deterministic exit codes, and a BORN → EXECUTE → FINALIZE → DIE lifecycle with
//! no daemon or sticky session state.
//!
//! This crate **consumes** published documentation from crates.io and docs.rs
//! (and stdlib pages on `doc.rust-lang.org`). It does **not** replace docs.rs
//! or re-implement `cargo doc` as a proprietary host.
//!
//! # Concurrency model (Rules Rust — parallelism)
//!
//! Workload is **mixed I/O + CPU** (see binary `main`, [`concurrency`], [`http::HttpClient`]):
//! multi-thread Tokio for HTTPS and `spawn_blocking` under
//! [`concurrency::ConcurrencyBudget`] for HTML parse. The `all.html` hit scan is
//! sequential by measurement: a `rayon` path lost at every size from 16 to 32768
//! candidates and was removed, so the crate has no data-parallel dependency.
//! Bound via `--max-concurrency` / auto CPU+RAM formula.
//! Fixed auxiliary tasks (double-interrupt, [`shutdown::ProgressGuard`]) still
//! abort-on-drop. Shared cancel uses `Arc<AtomicU8>`, not `Mutex` across `.await`.
//!
//! # HTTP retry (Rules Rust — retry/backoff)
//!
//! Product GETs use [`retry::RetryConfig`]: exponential full-jitter backoff,
//! `Retry-After` delta-seconds **or** HTTP-date on 429/503, dual budget
//! (`max_attempts` + `max_elapsed_ms`), no retry on permanent 4xx/parse/budget.
//! Kill switch: `--disable-retry` / TOML `disable_retry` / `max_retries=0`. Policy is
//! single-layer inside [`http::HttpClient`] only.
//!
//! # Features
//!
//! This package has no Cargo feature flags. All product capabilities ship in the
//! default build. Optional network live tests are gated by `#[ignore]` and run
//! with `cargo test -- --ignored` — no environment variable and no feature. They
//! used to carry a second gate reading two `DOCSRS_CLI_*` variables, which made
//! `--ignored` return early from all of them and still report them as passed.
//!
//! # Safety
//!
//! - Product code in `src/` contains no `unsafe` blocks (`#![forbid(unsafe_code)]`).
//! - External input is validated at boundaries (domain newtypes, origin allowlist, body caps).
//! - Threat model (STRIDE / accepted risks): `docs/decisions/0004-threat-model.md`.
//! - Regexes compile with bounded `size_limit` / `dfa_size_limit` (ReDoS posture).
//! - docs.rs builds enable `doc_cfg` via `#![cfg_attr(docsrs, feature(doc_cfg))]`
//!   (post-2025 merge of `doc_auto_cfg` into `doc_cfg`). Local `cargo +stable doc`
//!   does not require the nightly feature gate.
//! - TLS is rustls-only; HTTP product methods are GET-only against an allowlist.
//! - Agent lifecycle is documented as a Mermaid sequence fence on [`run`]
//!   (plain rustdoc code block — no proc-macro renderer).
//!
//! # Examples
//!
//! Parse a crate name at the domain boundary:
//!
//! ```
//! use docsrs_cli::domain::CrateName;
//!
//! let name = CrateName::parse("serde").expect("valid crate name");
//! assert_eq!(name.as_str(), "serde");
//! ```
//!
//! Map error kinds to process exit codes:
//!
//! ```
//! use docsrs_cli::error::ErrorKind;
//!
//! assert_eq!(ErrorKind::NotFound.exit_code(), 66);
//! assert!(ErrorKind::Timeout.retryable());
//! ```

// Defensive security (Rules Rust / ADR 0009): product has no FFI; forbid all `unsafe`.
// Integration tests may use `unsafe` only for the Unix signal harness (`libc::kill`).
// Loopback mocks use CLI/XDG `allow_loopback` — never `env::set_var`.
#![forbid(unsafe_code)]
#![warn(missing_docs)]
// Gate for the `# Errors` / `# Panics` sections. Without these two lints the
// sections regress silently: `missing_docs` only checks that an item is
// documented, never that a fallible item documents *how* it fails.
#![warn(clippy::missing_errors_doc)]
#![warn(clippy::missing_panics_doc)]
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
#![warn(rustdoc::redundant_explicit_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod agent_ops;
pub mod cache;
pub mod cli;
pub mod concurrency;
pub mod config;
pub mod crates_io;
pub mod diagnostics;
mod dispatch;
pub mod docs_rs;
mod doctor;
pub mod domain;
pub mod error;
pub mod http;
pub mod i18n;
pub mod item_kind;
mod meta_cmds;
mod ops;
mod output;
pub mod platform;
pub mod render;
pub mod retry;
pub mod shutdown;
mod suggest;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;

use clap::Parser;

use crate::agent_ops::AgentOps;
use crate::cli::Cli;
use crate::cli::overrides::apply_cli_overrides;
use crate::config::Config;
use crate::diagnostics::init_tracing;
use crate::dispatch::execute;
use crate::error::{AppError, EXIT_BROKEN_PIPE, EXIT_USAGE, ErrorDetail, ErrorKind};
use crate::i18n::Locale;
use crate::render::error_envelope;
use crate::shutdown::flush_stdio;

/// True when argv requests JSON or stdout is non-TTY (agent mode) — used before full parse.
fn argv_wants_json_mode(args: &[std::ffi::OsString], stdout_is_terminal: bool) -> bool {
    let has_json = args.iter().any(|a| a == "--json");
    let has_human_format = args.windows(2).any(|w| {
        w[0] == "--format"
            && w[1]
                .to_str()
                .is_some_and(|v| v == "text" || v == "markdown" || v == "md")
    }) || args.iter().any(|a| {
        a.to_str().is_some_and(|s| {
            s.starts_with("--format=")
                && (s.contains("text") || s.contains("markdown") || s.contains("md"))
        })
    });
    if has_json {
        return true;
    }
    if has_human_format {
        return false;
    }
    !stdout_is_terminal
}

/// Parse argv and execute one command. Returns process exit code.
///
/// This is the binary entrypoint wrapper (stdin/stdout/stderr are process defaults).
/// Agent-first: when stdout is not a TTY, JSON is auto-selected unless `--format` forces human.
///
/// # Lifecycle
///
/// Lifecycle (BORN → EXECUTE → FINALIZE → DIE), Mermaid form for agent readers:
///
/// ```text
/// sequenceDiagram
///   participant A as Agent
///   participant C as docsrs-cli
///   participant N as crates.io / docs.rs
///   A->>C: BORN spawn
///   C->>N: EXECUTE GET
///   N-->>C: body
///   C-->>A: stdout envelope
///   C->>C: FINALIZE flush
///   C->>C: DIE exit code
/// ```
pub async fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let stdout_is_terminal = io::stdout().is_terminal();
    run_with_io(
        args,
        io::stdin(),
        io::stdout(),
        io::stderr(),
        stdout_is_terminal,
    )
    .await
}

/// Injectable IO entrypoint for tests (`run(args, stdin, stdout, stderr, stdout_is_terminal)`).
///
/// `stdin` is reserved for future payload flows and is unused by current ops.
/// Pass `stdout_is_terminal = true` to keep human markdown default in unit tests;
/// the binary path passes the real `IsTerminal` of process stdout.
pub async fn run_with_io<I, T, In, Out, ErrW>(
    args: I,
    _stdin: In,
    mut stdout: Out,
    mut stderr: ErrW,
    stdout_is_terminal: bool,
) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
    In: io::Read,
    Out: Write,
    ErrW: Write,
{
    // Collect args once so we can detect --json / agent mode before parse fails.
    let args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let agent_json = argv_wants_json_mode(&args, stdout_is_terminal);

    let cli = match Cli::try_parse_from(&args) {
        Ok(c) => c,
        Err(e) => {
            // Help / version display: human text, success.
            if !e.use_stderr() {
                let _ = e.print();
                flush_stdio();
                return ExitCode::SUCCESS;
            }
            // Contract: usage failures are exit EXIT_USAGE with JSON envelope in agent/JSON mode.
            let msg = e.to_string();
            if agent_json {
                let err = AppError::of(ErrorDetail::ClapUsage { message: msg });
                let _ = output::write_json(&mut stdout, &error_envelope(&err, "usage", 0));
                flush_stdio();
                return ExitCode::from(EXIT_USAGE);
            }
            // Human TTY: print clap rendering to stderr, still EXIT_USAGE (unified contract).
            let _ = e.print();
            flush_stdio();
            return ExitCode::from(EXIT_USAGE);
        }
    };

    let code = match run_cli(cli, &mut stdout, &mut stderr, stdout_is_terminal).await {
        Ok(code) => code,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => ExitCode::from(EXIT_BROKEN_PIPE),
        Err(err) => {
            // Domain errors should already have been turned into ExitCode via
            // `emit_error` inside `run_cli`. This path is a last-resort safety net
            // (preserves kind exit code; never hardcodes 70).
            tracing::error!(
                target: "docsrs_cli::diagnostics",
                error = %err,
                kind = ?err.kind(),
                "error escaped structured emit path"
            );
            // Localized like every other error path. The `cli` was moved into
            // `run_cli`, so the locale is re-read from argv — the same source the
            // bootstrap failures in `main` use, and the only one still available
            // here. A hardcoded `error:` prefix used to ship English whatever
            // `--lang` said, precisely because this branch is rare enough to be
            // forgotten.
            let locale = Locale::from_argv_for_bootstrap(&args);
            let _ = writeln!(stderr, "{}", locale.format_error(&err));
            err.exit_code()
        }
    };
    flush_stdio();
    let _ = stdout.flush();
    let _ = stderr.flush();
    code
}

async fn run_cli<Out: Write, ErrW: Write>(
    cli: Cli,
    stdout: &mut Out,
    stderr: &mut ErrW,
    stdout_is_terminal: bool,
) -> Result<ExitCode, AppError> {
    let wire = cli.wire_command();

    // Provisional locale for errors raised before the strict `--lang` resolution
    // below. Soft mapping never fails, so an unsupported explicit tag still
    // reaches its own fail-closed check and is reported in English there.
    let early_locale = cli
        .lang
        .as_deref()
        .map(Locale::soft_from_tag)
        .unwrap_or_else(Locale::from_system);

    if let Err(e) = cli.validate_format_conflict() {
        return Ok(meta_cmds::emit_error(
            &cli,
            &e,
            early_locale,
            stdout,
            stderr,
            stdout_is_terminal,
            wire,
            0,
        ));
    }

    // Config load must go through `emit_error` so exit 78 / JSON envelope are correct.
    // A bare `?` would bubble to `run_with_io` and historically forced exit 70.
    let mut cfg = match Config::load_with_options(
        cli.config_dir.clone(),
        cli.cache_dir.clone(),
        cli.allow_loopback,
    ) {
        Ok(c) => c,
        Err(e) => {
            return Ok(meta_cmds::emit_error(
                &cli,
                &e,
                early_locale,
                stdout,
                stderr,
                stdout_is_terminal,
                wire,
                0,
            ));
        }
    };

    // Tracing installs *after* the config load so the XDG `log_directive` can
    // steer it. Nothing is lost: the load emits no diagnostics of its own, and
    // its failure already travels through `emit_error` above.
    init_tracing(&cli, cfg.log_directive.as_deref());

    // `cfg` is loaded now, so a `lang` written in config.toml can also steer this
    // error. CLI still wins; soft mapping keeps the strict check below authoritative.
    let cfg_locale = cli
        .lang
        .as_deref()
        .or(cfg.lang.as_deref())
        .map_or(early_locale, Locale::soft_from_tag);
    if let Err(e) = apply_cli_overrides(&cli, &mut cfg) {
        return Ok(meta_cmds::emit_error(
            &cli,
            &e,
            cfg_locale,
            stdout,
            stderr,
            stdout_is_terminal,
            wire,
            0,
        ));
    }

    // Fail-closed on explicit/config lang before any network work (no product env).
    let locale = match Locale::resolve(cli.lang.as_deref().or(cfg.lang.as_deref())) {
        Ok(l) => l,
        Err(e) => {
            return Ok(meta_cmds::emit_error(
                &cli,
                &e,
                Locale::En,
                stdout,
                stderr,
                stdout_is_terminal,
                wire,
                0,
            ));
        }
    };
    // Reduction knobs are validated before any network work so a malformed
    // `--filter` fails fast (exit 65) instead of after a paid round-trip.
    let ops = match AgentOps::from_cli(&cli) {
        Ok(o) => o,
        Err(e) => {
            return Ok(meta_cmds::emit_error(
                &cli,
                &e,
                locale,
                stdout,
                stderr,
                stdout_is_terminal,
                wire,
                0,
            ));
        }
    };

    if !(ops.is_active() && cli.wants_json(stdout_is_terminal)) {
        return Ok(execute(&cli, &cfg, locale, stdout, stderr, stdout_is_terminal, wire).await);
    }

    // Agent-native reduction: capture the envelope, cut it, then emit. The buffer is
    // bounded by the same `max_output_bytes` budget the direct path already enforces.
    let mut captured: Vec<u8> = Vec::new();
    let code = execute(
        &cli,
        &cfg,
        locale,
        &mut captured,
        stderr,
        stdout_is_terminal,
        wire,
    )
    .await;
    let reduced = ops.apply_to_bytes(&captured);
    if !reduced.is_empty() && stdout.write_all(&reduced).is_err() {
        return Ok(ExitCode::from(EXIT_BROKEN_PIPE));
    }
    let _ = stdout.flush();
    Ok(code)
}
