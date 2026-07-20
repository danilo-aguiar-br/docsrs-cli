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
//! multi-thread Tokio for HTTPS, `spawn_blocking` under
//! [`concurrency::ConcurrencyBudget`] for HTML parse, and `rayon` for large
//! `all.html` hit scans. Bound via `--max-concurrency` / auto CPU+RAM formula.
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
//! default build. Optional network live tests are gated by environment variables
//! (`DOCSRS_CLI_NETWORK_TESTS`, `DOCSRS_CLI_STDLIB_NETWORK_TESTS`), not features.
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
#![warn(rustdoc::missing_crate_level_docs)]
#![warn(rustdoc::broken_intra_doc_links)]
#![warn(rustdoc::private_intra_doc_links)]
#![deny(rustdoc::invalid_html_tags)]
#![deny(rustdoc::invalid_rust_codeblocks)]
#![deny(rustdoc::bare_urls)]
#![warn(rustdoc::redundant_explicit_links)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod cache;
pub mod cli;
pub mod concurrency;
pub mod config;
pub mod crates_io;
pub mod diagnostics;
pub mod docs_rs;
mod doctor;
mod meta_cmds;
mod ops;
pub mod domain;
pub mod error;
pub mod http;
pub mod i18n;
pub mod item_kind;
mod output;
pub mod platform;
pub mod render;
pub mod retry;
pub mod shutdown;
mod suggest;

use std::io::{self, IsTerminal, Write};
use std::process::ExitCode;
use std::time::Duration;

use clap::Parser;
use tokio::time::Instant;

use crate::cli::{Cli, Commands};
use crate::config::{APP_NAME, APP_VERSION, Config, MSRV, validate_user_agent};
use crate::diagnostics::init_tracing;
use crate::error::{
    AppError, AppResult, EXIT_BROKEN_PIPE, EXIT_USAGE, ErrorKind,
};
use crate::i18n::Locale;
use crate::render::{error_envelope, success_envelope};
use crate::shutdown::{
    CancelFlag, duration_ms, flush_stdio, race_op_with_cancel_and_deadline,
    spawn_double_interrupt_force_exit,
};

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
                let err = AppError::new(ErrorKind::Usage, msg);
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
            let _ = writeln!(stderr, "error: {}", err.message());
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
    if let Err(e) = cli.validate_format_conflict() {
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

    init_tracing(&cli);

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
                Locale::En,
                stdout,
                stderr,
                stdout_is_terminal,
                wire,
                0,
            ));
        }
    };
    if let Err(e) = apply_cli_overrides(&cli, &mut cfg) {
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
    let start = Instant::now();
    let wants_json = cli.wants_json(stdout_is_terminal);
    let dry_run = cli.dry_run;
    let wall = Duration::from_secs(cfg.timeout_secs.max(1));
    let cancel = CancelFlag::new();
    // Rules Rust: first Ctrl-C is cooperative; second within 5s force-exits 130.
    let force_on_second = spawn_double_interrupt_force_exit();

    let cli_ref = &cli;
    let cfg_ref = &cfg;
    let result = race_op_with_cancel_and_deadline(wall, cancel.clone(), async {
        dispatch(
            cli_ref, cfg_ref, locale, dry_run, wants_json, start, cancel, stdout, stderr,
        )
        .await
    })
    .await;

    force_on_second.abort();

    match result {
        Ok(code) => Ok(code),
        Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(ExitCode::from(EXIT_BROKEN_PIPE)),
        Err(e) => Ok(meta_cmds::emit_error(
            &cli,
            &e,
            locale,
            stdout,
            stderr,
            stdout_is_terminal,
            wire,
            duration_ms(start),
        )),
    }
}

fn apply_cli_overrides(cli: &Cli, cfg: &mut Config) -> AppResult<()> {
    if let Some(t) = cli.timeout {
        if t == 0 {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "timeout must be >= 1 second (got 0)",
            ));
        }
        cfg.timeout_secs = t;
    }
    if let Some(t) = cli.connect_timeout {
        if t == 0 {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "connect_timeout must be >= 1 second (got 0)",
            ));
        }
        cfg.connect_timeout_secs = t;
    }
    if let Some(ref ua) = cli.user_agent {
        validate_user_agent(ua)?;
        cfg.user_agent.clone_from(ua);
    }
    if let Some(b) = cli.max_body_bytes {
        // Fail-closed: never silently clamp explicit CLI budgets (GAP-X-005).
        if b > crate::config::HARD_MAX_BODY_BYTES {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!(
                    "max_body_bytes exceeds hard maximum ({})",
                    crate::config::HARD_MAX_BODY_BYTES
                ),
            ));
        }
        cfg.max_body_bytes = b;
    }
    if let Some(b) = cli.max_output_bytes {
        if b > crate::config::HARD_MAX_OUTPUT_BYTES {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!(
                    "max_output_bytes exceeds hard maximum ({})",
                    crate::config::HARD_MAX_OUTPUT_BYTES
                ),
            ));
        }
        cfg.max_output_bytes = b;
    }
    if let Some(d) = cli.rate_limit_delay_ms {
        cfg.rate_limit_delay_ms = d;
    }
    if let Some(n) = cli.max_concurrency {
        cfg.max_concurrency = n;
    }
    if let Some(r) = cli.max_retries {
        cfg.max_retries = r;
    }
    if let Some(ms) = cli.retry_base_ms {
        cfg.retry_base_ms = ms;
    }
    if let Some(ms) = cli.retry_max_delay_ms {
        cfg.retry_max_delay_ms = ms;
    }
    if let Some(ms) = cli.retry_max_elapsed_ms {
        cfg.retry_max_elapsed_ms = ms;
    }
    if cli.disable_retry {
        cfg.disable_retry = true;
    }
    if let Some(ref l) = cli.lang {
        match &mut cfg.lang {
            Some(dst) => dst.clone_from(l),
            None => cfg.lang = Some(l.clone()),
        }
    }
    if cli.no_cache {
        cfg.no_cache = true;
    }
    if cli.allow_loopback {
        cfg.allow_loopback = true;
    }
    if let Some(ttl) = cli.cache_ttl_secs {
        cfg.cache_ttl_secs = ttl;
    }
    if let Some(max) = cli.max_cache_bytes {
        cfg.max_cache_bytes = max;
    }
    if let Some(ref dir) = cli.cache_dir {
        cfg.cache_dir = Some(dir.clone());
    }
    // After all CLI mutations: hard ceiling (cannot elevate above HARD_MAX_*).
    cfg.clamp_resource_limits();
    Ok(())
}


#[allow(clippy::too_many_arguments)]
async fn dispatch<Out: Write, ErrW: Write>(
    cli: &Cli,
    cfg: &Config,
    locale: Locale,
    dry_run: bool,
    wants_json: bool,
    start: Instant,
    cancel: CancelFlag,
    stdout: &mut Out,
    _stderr: &mut ErrW,
) -> AppResult<ExitCode> {
    match &cli.command {
        Commands::Version => {
            if wants_json {
                let data = meta_cmds::VersionData {
                    name: APP_NAME,
                    version: APP_VERSION,
                    msrv: MSRV,
                    os: std::env::consts::OS,
                    arch: std::env::consts::ARCH,
                };
                output::write_json(
                    stdout,
                    &success_envelope("version", &data, duration_ms(start), None),
                )?;
            } else {
                writeln!(stdout, "{APP_NAME} {APP_VERSION}").map_err(output::map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Doctor { online } => doctor::doctor(cfg, *online, wants_json, start, locale, stdout),
        Commands::Commands => meta_cmds::commands_cmd(wants_json, start, stdout),
        Commands::Schema { cmd } => meta_cmds::schema_cmd(cmd, wants_json, cli.format, start, stdout),
        Commands::Completions { shell } => {
            // Completions emit shell script by default (even on non-TTY). JSON only if --json/--format json.
            let explicit_json =
                cli.json || matches!(cli.format, Some(crate::cli::OutputFormat::Json));
            meta_cmds::completions_cmd(*shell, explicit_json, start, stdout)
        }
        Commands::Cache { action } => meta_cmds::cache_cmd(cfg, action, wants_json, start, stdout),
        Commands::Config { action } => meta_cmds::config_cmd(cli, cfg, action, wants_json, start, stdout),
        Commands::SearchCrates {
            query,
            per_page,
            sort,
            page,
            page_token,
        } => {
            let ctx = ops::OpCtx {
                cli,
                cfg,
                locale,
                dry_run,
                wants_json,
                start,
                cancel,
            };
            ops::search_crates(&ctx, stdout, query, *per_page, *sort, *page, page_token).await
        }
        Commands::Readme {
            crate_name,
            crate_version,
        } => {
            let ctx = ops::OpCtx {
                cli,
                cfg,
                locale,
                dry_run,
                wants_json,
                start,
                cancel,
            };
            ops::readme(&ctx, stdout, crate_name, crate_version).await
        }
        Commands::GetItem {
            crate_name,
            item_type,
            item_path,
            crate_version,
            suggest,
        } => {
            let ctx = ops::OpCtx {
                cli,
                cfg,
                locale,
                dry_run,
                wants_json,
                start,
                cancel,
            };
            ops::get_item(
                &ctx,
                stdout,
                crate_name,
                item_type,
                item_path,
                crate_version,
                *suggest,
            )
            .await
        }
        Commands::SearchInCrate {
            crate_name,
            query,
            crate_version,
            item_type,
            limit,
            r#match,
        } => {
            let ctx = ops::OpCtx {
                cli,
                cfg,
                locale,
                dry_run,
                wants_json,
                start,
                cancel,
            };
            ops::search_in_crate(
                &ctx,
                stdout,
                crate_name,
                query,
                crate_version,
                item_type,
                *limit,
                r#match,
            )
            .await
        }
    }
}

