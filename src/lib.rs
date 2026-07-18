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
//! `Retry-After` delta-seconds on 429/503, no retry on permanent 4xx/parse.
//! Kill switch: `--disable-retry` / `DOCSRS_CLI_DISABLE_RETRY`. Policy is
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
//! - Product code in `src/` contains no `unsafe` blocks.
//! - docs.rs builds enable `doc_cfg` via `#![cfg_attr(docsrs, feature(doc_cfg))]`
//!   (post-2025 merge of `doc_auto_cfg` into `doc_cfg`). Local `cargo +stable doc`
//!   does not require the nightly feature gate.
//! - TLS is rustls-only; HTTP product methods are GET-only against an allowlist.
//! - Mermaid diagrams for the agent lifecycle live on [`run`] via `aquamarine`
//!   (attribute macros apply to items, not the crate root).
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
pub mod docs_rs;
pub mod domain;
pub mod error;
pub mod http;
pub mod i18n;
pub mod item_kind;
pub mod platform;
pub mod render;
pub mod retry;
pub mod shutdown;
pub mod telemetry;

use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use serde::Serialize;
use tokio::time::Instant;

use crate::cache::DiskCache;
use crate::cli::{CacheAction, Cli, Commands, ConfigAction};
use crate::config::{
    APP_NAME, APP_VERSION, Config, MSRV, SCHEMA_VERSION, config_path_data, docs_origin_for_crate,
    init_config_toml,
};
use crate::domain::{CrateName, ItemPath, SearchQuery, VersionArg};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::HttpClient;
use crate::i18n::Locale;
use crate::item_kind::ItemKind;
use crate::render::{
    apply_truncation_to_item, apply_truncation_to_readme, dry_run_envelope, error_envelope,
    render_item_markdown, render_readme_markdown, render_search_in_crate_markdown,
    render_search_markdown, success_envelope,
};
use crate::shutdown::{
    CancelFlag, ProgressGuard, duration_ms, flush_stdio, race_op_with_cancel_and_deadline,
    spawn_double_interrupt_force_exit,
};
use crate::telemetry::init_tracing;

/// Parse argv and execute one command. Returns process exit code.
///
/// This is the binary entrypoint wrapper (stdin/stdout/stderr are process defaults).
/// Agent-first: when stdout is not a TTY, JSON is auto-selected unless `--format` forces human.
///
/// # Lifecycle
///
/// ```mermaid
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
#[cfg_attr(doc, aquamarine::aquamarine)]
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
    let cli = match Cli::try_parse_from(args) {
        Ok(c) => c,
        Err(e) => {
            // clap prints help/usage itself to its preferred stream
            let _ = e.print();
            flush_stdio();
            return if e.use_stderr() {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            };
        }
    };

    let code = match run_cli(cli, &mut stdout, &mut stderr, stdout_is_terminal).await {
        Ok(code) => code,
        Err(err) if err.kind() == ErrorKind::BrokenPipe => ExitCode::from(141),
        Err(err) => {
            // Domain errors should already have been turned into ExitCode via
            // `emit_error` inside `run_cli`. This path is a last-resort safety net
            // (preserves kind exit code; never hardcodes 70).
            tracing::error!(
                target: "docsrs_cli::telemetry",
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
    if let Err(e) = cli.validate_format_conflict() {
        return Ok(emit_error(
            &cli,
            &e,
            Locale::En,
            stdout,
            stderr,
            stdout_is_terminal,
        ));
    }

    init_tracing(&cli);

    // Config load must go through `emit_error` so exit 78 / JSON envelope are correct.
    // A bare `?` would bubble to `run_with_io` and historically forced exit 70.
    let mut cfg = match Config::load_with_cache_dir(cli.config_dir.clone(), cli.cache_dir.clone())
    {
        Ok(c) => c,
        Err(e) => {
            return Ok(emit_error(
                &cli,
                &e,
                Locale::En,
                stdout,
                stderr,
                stdout_is_terminal,
            ));
        }
    };
    apply_cli_overrides(&cli, &mut cfg);

    // Fail-closed on explicit/config/env lang before any network work.
    let locale = match Locale::resolve(cli.lang.as_deref().or(cfg.lang.as_deref())) {
        Ok(l) => l,
        Err(e) => {
            return Ok(emit_error(
                &cli,
                &e,
                Locale::En,
                stdout,
                stderr,
                stdout_is_terminal,
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
        Err(e) if e.kind() == ErrorKind::BrokenPipe => Ok(ExitCode::from(141)),
        Err(e) => Ok(emit_error(
            &cli,
            &e,
            locale,
            stdout,
            stderr,
            stdout_is_terminal,
        )),
    }
}

fn apply_cli_overrides(cli: &Cli, cfg: &mut Config) {
    if let Some(t) = cli.timeout {
        cfg.timeout_secs = t;
    }
    if let Some(t) = cli.connect_timeout {
        cfg.connect_timeout_secs = t;
    }
    if let Some(ref ua) = cli.user_agent {
        cfg.user_agent.clone_from(ua);
    }
    if let Some(b) = cli.max_body_bytes {
        cfg.max_body_bytes = b;
    }
    if let Some(b) = cli.max_output_bytes {
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
                let data = VersionData {
                    name: APP_NAME,
                    version: APP_VERSION,
                    msrv: MSRV,
                    os: std::env::consts::OS,
                    arch: std::env::consts::ARCH,
                };
                write_json(
                    stdout,
                    &success_envelope("version", &data, duration_ms(start), None),
                )?;
            } else {
                writeln!(stdout, "{APP_NAME} {APP_VERSION}").map_err(map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Doctor => doctor(cfg, wants_json, start, locale, stdout),
        Commands::Commands => commands_cmd(wants_json, start, stdout),
        Commands::Schema { cmd } => schema_cmd(cmd, wants_json, cli.format, start, stdout),
        Commands::Completions { shell } => completions_cmd(*shell, wants_json, start, stdout),
        Commands::Cache { action } => cache_cmd(cfg, action, wants_json, start, stdout),
        Commands::Config { action } => config_cmd(cli, cfg, action, wants_json, start, stdout),
        Commands::SearchCrates {
            query,
            per_page,
            sort,
            page,
        } => {
            let query = SearchQuery::parse(query, false)?;
            let sort_s = sort.as_api_str();
            // Clamp once so dry-run planned_params matches the URL and live request.
            let (per_page, page) = crates_io::clamp_search_pagination(*per_page, *page);
            let url = crates_io::planned_url_on_host(
                &cfg.crates_io_origin,
                query.as_str(),
                per_page,
                sort_s,
                page,
            )?;
            if dry_run {
                return emit_dry_run(
                    "search-crates",
                    url.as_str(),
                    SearchCratesDryParams {
                        q: query.as_str(),
                        per_page,
                        sort: sort_s,
                        page,
                    },
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching("crates.io"),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = crates_io::search_crates_on_origin(
                &http,
                &cfg.crates_io_origin,
                query.as_str(),
                per_page,
                sort_s,
                page,
            )
            .await;
            progress.finish();
            let data = data?;
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope(
                        "search-crates",
                        &data,
                        duration_ms(start),
                        Some(url.as_str()),
                    ),
                )?;
            } else {
                let md = render_search_markdown(&data);
                let (out, _) = render::truncate_output(&md, cfg.max_output_bytes);
                write!(stdout, "{out}").map_err(map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Readme {
            crate_name,
            crate_version,
        } => {
            let crate_name = CrateName::parse(crate_name)?;
            let version = VersionArg::parse_opt(crate_version.as_deref())?;
            let origin = docs_origin_for_crate(cfg, crate_name.as_str());
            let url =
                docs_rs::readme_url_on_origin(&origin, crate_name.as_str(), version.as_str())?;
            if dry_run {
                return emit_dry_run(
                    "readme",
                    url.as_str(),
                    ReadmeDryParams {
                        crate_name: crate_name.as_str(),
                        version: version.as_str(),
                    },
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(crate_name.as_str()),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = docs_rs::fetch_readme_on_origin(
                &http,
                &origin,
                crate_name.as_str(),
                version.as_str(),
            )
            .await;
            progress.finish();
            let data = apply_truncation_to_readme(data?, cfg);
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("readme", &data, duration_ms(start), Some(&data.source_url)),
                )?;
            } else {
                write!(stdout, "{}", render_readme_markdown(&data)).map_err(map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::GetItem {
            crate_name,
            item_type,
            item_path,
            crate_version,
        } => {
            let crate_name = CrateName::parse(crate_name)?;
            let kind = ItemKind::parse(item_type)?;
            let item_path = ItemPath::parse(item_path)?;
            let segs = item_path.segments();
            if let Some(first) = segs.first() {
                let rustc = item_kind::rustc_crate_name(crate_name.as_str());
                if first.as_str() != crate_name.as_str()
                    && first.as_str() != rustc.as_str()
                    && (first.contains('-') || first.contains('_'))
                {
                    tracing::warn!(
                        path_crate = %first,
                        expected = %crate_name,
                        "item_path crate prefix differs from crate_name"
                    );
                }
            }
            let version = VersionArg::parse_opt(crate_version.as_deref())?;
            let origin = docs_origin_for_crate(cfg, crate_name.as_str());
            let url = docs_rs::get_item_url_on_origin(
                &origin,
                crate_name.as_str(),
                version.as_str(),
                kind,
                segs,
            )?;
            if dry_run {
                return emit_dry_run(
                    "get-item",
                    url.as_str(),
                    GetItemDryParams {
                        crate_name: crate_name.as_str(),
                        item_type: kind.as_str(),
                        item_path: item_path.as_str(),
                        version: version.as_str(),
                    },
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(item_path.as_str()),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = docs_rs::fetch_item_on_origin(
                &http,
                &origin,
                crate_name.as_str(),
                version.as_str(),
                kind,
                segs,
            )
            .await;
            progress.finish();
            let data = apply_truncation_to_item(data?, cfg);
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope(
                        "get-item",
                        &data,
                        duration_ms(start),
                        Some(&data.source_url),
                    ),
                )?;
            } else {
                write!(stdout, "{}", render_item_markdown(&data)).map_err(map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::SearchInCrate {
            crate_name,
            query,
            crate_version,
            item_type,
            limit,
        } => {
            let crate_name = CrateName::parse(crate_name)?;
            let query = SearchQuery::parse(query, true)?;
            let kind_filter = match item_type {
                Some(t) if !t.is_empty() => Some(ItemKind::parse(t)?),
                _ => None,
            };
            let version = VersionArg::parse_opt(crate_version.as_deref())?;
            let origin = docs_origin_for_crate(cfg, crate_name.as_str());
            let url =
                docs_rs::all_html_url_on_origin(&origin, crate_name.as_str(), version.as_str())?;
            if dry_run {
                return emit_dry_run(
                    "search-in-crate",
                    url.as_str(),
                    SearchInCrateDryParams {
                        crate_name: crate_name.as_str(),
                        query: query.as_str(),
                        version: version.as_str(),
                        item_type: kind_filter.map(|k| k.as_str()),
                        limit: *limit,
                    },
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(&format!("{} all.html", crate_name.as_str())),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = docs_rs::search_in_crate_on_origin(
                &http,
                &origin,
                crate_name.as_str(),
                version.as_str(),
                query.as_str(),
                kind_filter,
                *limit,
            )
            .await;
            progress.finish();
            let data = data?;
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope(
                        "search-in-crate",
                        &data,
                        duration_ms(start),
                        Some(&data.source_url),
                    ),
                )?;
            } else {
                let md = render_search_in_crate_markdown(&data);
                let (out, _) = render::truncate_output(&md, cfg.max_output_bytes);
                write!(stdout, "{out}").map_err(map_stdout_err)?;
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn doctor<Out: Write>(
    cfg: &Config,
    wants_json: bool,
    start: Instant,
    locale: Locale,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    let mut checks = Vec::new();

    checks.push(DoctorCheck {
        name: "platform",
        ok: true,
        detail: crate::platform::platform_detail(),
    });
    checks.push(DoctorCheck {
        name: "rustls_client",
        ok: true,
        detail: "reqwest built with rustls-tls".into(),
    });
    checks.push(DoctorCheck {
        name: "user_agent",
        ok: !cfg.user_agent.is_empty() && cfg.user_agent.contains(APP_NAME),
        detail: cfg.user_agent.clone(),
    });

    let (config_ok, config_detail) = match &cfg.config_dir {
        Some(p) => dir_ready_check(p),
        None => (false, "default-not-resolved".into()),
    };
    checks.push(DoctorCheck {
        name: "config_dir",
        ok: config_ok,
        detail: config_detail,
    });

    // XDG storage layer diagnostics (no .env runtime; no secret layers).
    checks.push(DoctorCheck {
        name: "config_source",
        ok: cfg.config_path_source != crate::config::PathSource::Unresolved,
        detail: cfg.config_path_source.as_str().into(),
    });
    let (cfg_file_ok, cfg_file_detail) = match cfg.config_file_path() {
        Some(p) => {
            if p.is_file() {
                (
                    true,
                    format!(
                        "present loaded={} path={}",
                        cfg.config_toml_loaded,
                        p.display()
                    ),
                )
            } else {
                (
                    true,
                    format!(
                        "absent (optional; run config init) path={}",
                        p.display()
                    ),
                )
            }
        }
        None => (false, "config file path unresolved".into()),
    };
    checks.push(DoctorCheck {
        name: "config_file",
        ok: cfg_file_ok,
        detail: cfg_file_detail,
    });
    checks.push(DoctorCheck {
        name: "cache_source",
        ok: cfg.no_cache || cfg.cache_path_source != crate::config::PathSource::Unresolved,
        detail: if cfg.no_cache {
            "n/a (--no-cache)".into()
        } else {
            cfg.cache_path_source.as_str().into()
        },
    });
    checks.push(DoctorCheck {
        name: "dotenv_runtime",
        ok: true,
        detail: "disabled (XDG + env allowlist only; no .env required after install)".into(),
    });
    checks.push(DoctorCheck {
        name: "secrets_layers",
        ok: true,
        detail: "none (public HTTP only; keyring/cloud secret managers out of scope)".into(),
    });

    // Runtime clamps both to min 1s (`Config::timeout` / `connect_timeout`); report effective values.
    let eff_timeout = cfg.timeout_secs.max(1);
    let eff_connect = cfg.connect_timeout_secs.max(1);
    checks.push(DoctorCheck {
        name: "timeouts",
        ok: true,
        detail: format!("timeout={eff_timeout}s connect={eff_connect}s"),
    });

    let resolved_conc = crate::concurrency::resolve_max_concurrency(cfg.max_concurrency);
    let workers = crate::concurrency::runtime_worker_threads();
    checks.push(DoctorCheck {
        name: "concurrency",
        ok: true,
        detail: format!(
            "max_concurrency={} (configured={}; 0=auto) runtime_workers≈{} formula=min(cpus,free_ram/2/{}MiB) cap={}",
            resolved_conc,
            cfg.max_concurrency,
            workers,
            crate::concurrency::RAM_PER_TASK_MIB,
            crate::concurrency::MAX_AUTO_CONCURRENCY
        ),
    });

    let retry = crate::retry::RetryConfig::from_config(cfg);
    checks.push(DoctorCheck {
        name: "retry_policy",
        ok: true,
        detail: format!(
            "enabled={} max_retries={} base_ms={} max_delay_ms={} max_attempts={} formula=full_jitter(min(base*2^n,max_delay)) kill_switch=--disable-retry",
            retry.enabled,
            retry.max_retries,
            retry.base_ms,
            retry.max_delay_ms,
            retry.max_attempts()
        ),
    });

    let (cache_ok, cache_detail) = if cfg.no_cache {
        (true, "disabled (--no-cache)".to_string())
    } else {
        match &cfg.cache_dir {
            Some(p) => {
                let (ready, ready_detail) = dir_ready_check(p);
                if !ready {
                    (false, ready_detail)
                } else {
                    let stats =
                        DiskCache::new(p.clone(), cfg.cache_ttl(), cfg.max_cache_bytes).stats();
                    let budget = if cfg.max_cache_bytes == 0 {
                        "unlimited".to_string()
                    } else {
                        format!("{}B", cfg.max_cache_bytes)
                    };
                    (
                        true,
                        format!(
                            "enabled dir={} ttl={}s max={} entries={} used={}B parser={}",
                            p.display(),
                            cfg.cache_ttl_secs,
                            budget,
                            stats.entries,
                            stats.total_bytes,
                            crate::cache::CACHE_PARSER_VERSION
                        ),
                    )
                }
            }
            None => (false, "no cache dir resolved".to_string()),
        }
    };
    checks.push(DoctorCheck {
        name: "disk_cache",
        ok: cache_ok,
        detail: cache_detail,
    });

    // Cross-process rate-limit lock+stamp directory (same root as disk cache).
    let (rl_ok, rl_detail) = if cfg.no_cache {
        (
            true,
            "n/a (--no-cache; in-process rate limit only)".to_string(),
        )
    } else {
        match &cfg.cache_dir {
            Some(p) => {
                let rl = p.join("rate-limit");
                dir_ready_check(&rl)
            }
            None => (
                false,
                "no cache dir resolved (cross-process rate limit unavailable)".to_string(),
            ),
        }
    };
    checks.push(DoctorCheck {
        name: "rate_limit_dir",
        ok: rl_ok,
        detail: rl_detail,
    });

    // Resolved human locale for stderr prose (JSON messages stay English).
    checks.push(DoctorCheck {
        name: "lang",
        ok: true,
        detail: locale.as_bcp47().into(),
    });

    let ok = checks.iter().all(|c| c.ok);
    let data = DoctorData { ok, checks };
    if wants_json {
        write_json(
            stdout,
            &success_envelope("doctor", &data, duration_ms(start), None),
        )?;
    } else {
        writeln!(stdout, "{}", locale.doctor_ok(ok)).map_err(map_stdout_err)?;
        for c in &data.checks {
            writeln!(
                stdout,
                "- {} [{}] {}",
                c.name,
                if c.ok { "ok" } else { "fail" },
                c.detail
            )
            .map_err(map_stdout_err)?;
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(78)
    })
}

/// Check that `path` is an existing writable directory, or can be created under a
/// writable ancestor. Does not leave directories behind when only probing parents.
fn dir_ready_check(path: &Path) -> (bool, String) {
    if path.is_dir() {
        let probe = path.join(".docsrs-cli-doctor-write-probe");
        return match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&probe)
        {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                (true, path.display().to_string())
            }
            Err(e) => (false, format!("{} (not writable: {e})", path.display())),
        };
    }
    if path.exists() {
        return (
            false,
            format!("{} (exists but is not a directory)", path.display()),
        );
    }
    let mut ancestor = path.parent();
    while let Some(a) = ancestor {
        if a.as_os_str().is_empty() {
            break;
        }
        if a.is_dir() {
            let probe = a.join(".docsrs-cli-doctor-write-probe");
            return match std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&probe)
            {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                    (
                        true,
                        format!("{} (creatable under {})", path.display(), a.display()),
                    )
                }
                Err(e) => (
                    false,
                    format!("{} (ancestor not writable: {e})", path.display()),
                ),
            };
        }
        if a.exists() {
            return (
                false,
                format!("{} (ancestor is not a directory)", path.display()),
            );
        }
        ancestor = a.parent();
    }
    (
        false,
        format!("{} (missing; no writable ancestor)", path.display()),
    )
}

fn cache_cmd<Out: Write>(
    cfg: &Config,
    action: &CacheAction,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    let root = cfg.cache_dir.clone().ok_or_else(|| {
        AppError::new(
            ErrorKind::Config,
            "cache directory could not be resolved (set --cache-dir or DOCSRS_CLI_CACHE_DIR)",
        )
    })?;
    let cache = DiskCache::new(root, cfg.cache_ttl(), cfg.max_cache_bytes);
    match action {
        CacheAction::Clear => {
            let result = cache.clear()?;
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("cache-clear", &result, duration_ms(start), None),
                )?;
            } else {
                writeln!(
                    stdout,
                    "cache cleared: {} entries, {} bytes freed ({})",
                    result.removed_entries, result.freed_bytes, result.root
                )
                .map_err(map_stdout_err)?;
            }
        }
        CacheAction::Stats => {
            let stats = cache.stats();
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("cache-stats", &stats, duration_ms(start), None),
                )?;
            } else {
                let budget = if stats.max_bytes == 0 {
                    "unlimited".to_string()
                } else {
                    format!("{}B", stats.max_bytes)
                };
                writeln!(
                    stdout,
                    "cache root={} layout={} entries={} used={}B max={} ttl={}s parser={}",
                    stats.root,
                    stats.layout,
                    stats.entries,
                    stats.total_bytes,
                    budget,
                    stats.ttl_secs,
                    stats.parser_version
                )
                .map_err(map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// XDG config lifecycle: path inventory, effective show, init template.
fn config_cmd<Out: Write>(
    cli: &Cli,
    cfg: &Config,
    action: &ConfigAction,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    match action {
        ConfigAction::Path => {
            let data = config_path_data(cfg);
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("config-path", &data, duration_ms(start), None),
                )?;
            } else {
                writeln!(
                    stdout,
                    "config_dir={} source={}",
                    data.config_dir.as_deref().unwrap_or("<unresolved>"),
                    data.config_source.as_str()
                )
                .map_err(map_stdout_err)?;
                writeln!(
                    stdout,
                    "config_file={} exists={} loaded={}",
                    data.config_file.as_deref().unwrap_or("<unresolved>"),
                    data.config_file_exists,
                    data.config_toml_loaded
                )
                .map_err(map_stdout_err)?;
                writeln!(
                    stdout,
                    "cache_dir={} source={}",
                    data.cache_dir.as_deref().unwrap_or("<unresolved>"),
                    data.cache_source.as_str()
                )
                .map_err(map_stdout_err)?;
                writeln!(
                    stdout,
                    "dotenv_runtime={} secrets={}",
                    data.dotenv_runtime, data.secrets_layers
                )
                .map_err(map_stdout_err)?;
            }
        }
        ConfigAction::Show => {
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("config-show", cfg, duration_ms(start), None),
                )?;
            } else {
                let pretty = serde_json::to_string_pretty(cfg).map_err(|e| {
                    AppError::with_source(ErrorKind::Internal, "failed to serialize config", e)
                })?;
                writeln!(stdout, "{pretty}").map_err(map_stdout_err)?;
            }
        }
        ConfigAction::Init { force } => {
            let result = init_config_toml(cli.config_dir.clone(), *force)?;
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("config-init", &result, duration_ms(start), None),
                )?;
            } else {
                let verb = if result.overwritten {
                    "overwrote"
                } else {
                    "created"
                };
                writeln!(
                    stdout,
                    "config init: {verb} {} (source={})",
                    result.path,
                    result.source.as_str()
                )
                .map_err(map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Emit the full command tree for agent discovery (`commands` subcommand).
fn commands_cmd<Out: Write>(
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    let data = command_tree_data();
    if wants_json {
        write_json(
            stdout,
            &success_envelope("commands", &data, duration_ms(start), None),
        )?;
    } else {
        writeln!(
            stdout,
            "{} {} — command tree",
            data.name, data.version
        )
        .map_err(map_stdout_err)?;
        for c in data.commands {
            writeln!(stdout, "- {}: {}", c.name, c.about).map_err(map_stdout_err)?;
            for s in c.subcommands {
                writeln!(stdout, "  - {} {}: {}", c.name, s.name, s.about)
                    .map_err(map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Typed `version` success data (matches `docs/schemas/version.schema.json`).
#[derive(Debug, Serialize)]
struct VersionData {
    name: &'static str,
    version: &'static str,
    msrv: &'static str,
    os: &'static str,
    arch: &'static str,
}

/// One doctor health check row.
#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: &'static str,
    ok: bool,
    detail: String,
}

/// Typed `doctor` success data (matches `docs/schemas/doctor.schema.json`).
#[derive(Debug, Serialize)]
struct DoctorData {
    ok: bool,
    checks: Vec<DoctorCheck>,
}

/// Agent notes embedded in the `commands` tree.
#[derive(Debug, Serialize)]
struct AgentNotes {
    stdout: &'static str,
    stderr: &'static str,
    json_auto: &'static str,
    lifecycle: &'static str,
}

/// One nested subcommand entry under `cache`.
#[derive(Debug, Serialize)]
struct SubCommandNode {
    name: &'static str,
    about: &'static str,
}

/// One top-level command entry in the discovery tree.
#[derive(Debug, Serialize)]
struct CommandNode {
    name: &'static str,
    about: &'static str,
    args: &'static [&'static str],
    subcommands: &'static [SubCommandNode],
}

/// Stable, ordered command tree for agents (no HashMap iteration).
#[derive(Debug, Serialize)]
struct CommandTree {
    name: &'static str,
    version: &'static str,
    msrv: &'static str,
    schema_version: u32,
    agent_notes: AgentNotes,
    commands: &'static [CommandNode],
}

/// Schema command payload (`schema` stays `Value` — embedded JSON Schema document).
#[derive(Debug, Serialize)]
struct SchemaData<'a> {
    command: &'a str,
    schema: serde_json::Value,
    schema_version: u32,
}

/// Completions command payload.
#[derive(Debug, Serialize)]
struct CompletionsData {
    shell: &'static str,
    script: String,
}

/// Stable, ordered command tree for agents (no HashMap iteration).
fn command_tree_data() -> CommandTree {
    CommandTree {
        name: APP_NAME,
        version: APP_VERSION,
        msrv: MSRV,
        schema_version: SCHEMA_VERSION,
        agent_notes: AgentNotes {
            stdout: "data only (JSON envelope or markdown)",
            stderr: "diagnostics only",
            json_auto: "JSON is selected automatically when stdout is not a TTY unless --format markdown|text",
            lifecycle: "BORN → EXECUTE → FINALIZE → DIE (one-shot, no daemon)",
        },
        commands: &[
            CommandNode {
                name: "search-crates",
                about: "Search crates on crates.io",
                args: &["query", "--per-page", "--sort", "--page"],
                subcommands: &[],
            },
            CommandNode {
                name: "readme",
                about: "Fetch crate overview docblock from docs.rs (not git README)",
                args: &["crate_name", "--crate-version"],
                subcommands: &[],
            },
            CommandNode {
                name: "get-item",
                about: "Fetch documentation for a typed item",
                args: &["crate_name", "item_type", "item_path", "--crate-version"],
                subcommands: &[],
            },
            CommandNode {
                name: "search-in-crate",
                about: "Search symbols in crate all.html index",
                args: &["crate_name", "query", "--crate-version", "--item-type", "--limit"],
                subcommands: &[],
            },
            CommandNode {
                name: "version",
                about: "Print binary version",
                args: &[],
                subcommands: &[],
            },
            CommandNode {
                name: "doctor",
                about: "Validate local TLS/config readiness",
                args: &[],
                subcommands: &[],
            },
            CommandNode {
                name: "commands",
                about: "List the full command tree for agent discovery",
                args: &[],
                subcommands: &[],
            },
            CommandNode {
                name: "schema",
                about: "Emit JSON Schema for a command",
                args: &["--cmd"],
                subcommands: &[],
            },
            CommandNode {
                name: "completions",
                about: "Generate shell completions",
                args: &["shell"],
                subcommands: &[],
            },
            CommandNode {
                name: "cache",
                about: "Inspect or clear the XDG HTTP disk cache",
                args: &[],
                subcommands: &[
                    SubCommandNode {
                        name: "clear",
                        about: "Delete all cached HTTP bodies under the cache dir",
                    },
                    SubCommandNode {
                        name: "stats",
                        about: "Report entry count, total bytes, and budget",
                    },
                ],
            },
            CommandNode {
                name: "config",
                about: "Manage XDG config paths and optional config.toml (no secrets / no .env)",
                args: &[],
                subcommands: &[
                    SubCommandNode {
                        name: "path",
                        about: "Print resolved config/cache directories and which layer won",
                    },
                    SubCommandNode {
                        name: "show",
                        about: "Print effective runtime configuration",
                    },
                    SubCommandNode {
                        name: "init",
                        about: "Create default config.toml under the resolved config directory",
                    },
                ],
            },
        ],
    }
}

fn schema_cmd<Out: Write>(
    cmd: &str,
    wants_json: bool,
    format: Option<crate::cli::OutputFormat>,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    let schema = match cmd {
        "search-crates" => include_str!("../docs/schemas/search-crates.schema.json"),
        "readme" => include_str!("../docs/schemas/readme.schema.json"),
        "get-item" => include_str!("../docs/schemas/get-item.schema.json"),
        "search-in-crate" => include_str!("../docs/schemas/search-in-crate.schema.json"),
        "version" => include_str!("../docs/schemas/version.schema.json"),
        "doctor" => include_str!("../docs/schemas/doctor.schema.json"),
        "commands" => include_str!("../docs/schemas/commands.schema.json"),
        "cache" | "cache-clear" | "cache-stats" => {
            include_str!("../docs/schemas/cache.schema.json")
        }
        "config" | "config-path" | "config-show" | "config-init" => {
            include_str!("../docs/schemas/config.schema.json")
        }
        other => {
            return Err(AppError::new(
                ErrorKind::Usage,
                format!("unknown schema command '{other}'"),
            ));
        }
    };
    let schema_val: serde_json::Value = serde_json::from_str(schema).map_err(|e| {
        AppError::with_source(ErrorKind::Internal, "embedded schema is invalid JSON", e)
    })?;
    // Branch first so JSON path can move `schema_val` into the envelope (no clone).
    if wants_json {
        let data = SchemaData {
            command: cmd,
            schema: schema_val,
            schema_version: SCHEMA_VERSION,
        };
        write_json(
            stdout,
            &success_envelope("schema", &data, duration_ms(start), None),
        )?;
    } else if matches!(format, Some(crate::cli::OutputFormat::Markdown)) {
        let md = render::render_schema_markdown(cmd, &schema_val);
        write!(stdout, "{md}").map_err(map_stdout_err)?;
    } else {
        writeln!(
            stdout,
            "{}",
            serde_json::to_string_pretty(&schema_val).unwrap_or_default()
        )
        .map_err(map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn completions_cmd<Out: Write>(
    shell: crate::cli::Shell,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    let mut buf = Vec::new();
    let mut cmd = Cli::command();
    clap_complete::generate(shell.to_clap_shell(), &mut cmd, APP_NAME, &mut buf);
    let script = String::from_utf8_lossy(&buf).into_owned();
    if wants_json {
        let data = CompletionsData {
            shell: shell.as_str(),
            script,
        };
        write_json(
            stdout,
            &success_envelope("completions", &data, duration_ms(start), None),
        )?;
    } else {
        write!(stdout, "{script}").map_err(map_stdout_err)?;
        let _ = stdout.flush();
    }
    Ok(ExitCode::SUCCESS)
}

/// Typed dry-run params for `search-crates` (keys match crates.io query string).
#[derive(Debug, Serialize)]
struct SearchCratesDryParams<'a> {
    q: &'a str,
    per_page: u32,
    sort: &'a str,
    page: u32,
}

/// Typed dry-run params for `readme`.
#[derive(Debug, Serialize)]
struct ReadmeDryParams<'a> {
    #[serde(rename = "crate")]
    crate_name: &'a str,
    version: &'a str,
}

/// Typed dry-run params for `get-item`.
#[derive(Debug, Serialize)]
struct GetItemDryParams<'a> {
    #[serde(rename = "crate")]
    crate_name: &'a str,
    item_type: &'a str,
    item_path: &'a str,
    version: &'a str,
}

/// Typed dry-run params for `search-in-crate`.
#[derive(Debug, Serialize)]
struct SearchInCrateDryParams<'a> {
    #[serde(rename = "crate")]
    crate_name: &'a str,
    query: &'a str,
    version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_type: Option<&'a str>,
    limit: u32,
}

fn emit_dry_run<Out: Write, P: Serialize>(
    command: &str,
    planned_url: &str,
    planned_params: P,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    if wants_json {
        write_json(
            stdout,
            &dry_run_envelope(command, planned_url, &planned_params, duration_ms(start)),
        )?;
    } else {
        writeln!(stdout, "dry-run {command}").map_err(map_stdout_err)?;
        writeln!(stdout, "planned_url: {planned_url}").map_err(map_stdout_err)?;
        writeln!(
            stdout,
            "planned_params: {}",
            serde_json::to_string_pretty(&planned_params).unwrap_or_default()
        )
        .map_err(map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn map_stdout_err(e: io::Error) -> AppError {
    if e.kind() == io::ErrorKind::BrokenPipe {
        AppError::broken_pipe()
    } else {
        AppError::with_source(ErrorKind::Internal, "stdout write", e)
    }
}

/// Serialize one RFC 8259 JSON object (compact, single line) + trailing `\n`.
///
/// Product stdout is a single JSON document per invocation (not NDJSON streams).
/// Optional fields are omitted when `None` (`skip_serializing_if`), never `NaN`/`Infinity`.
///
/// Serialize to an intermediate buffer first so broken-pipe on write is mapped to
/// exit 141 without conflating pure serde failures with I/O.
fn write_json<Out: Write, T: Serialize>(stdout: &mut Out, v: &T) -> AppResult<()> {
    let mut buf = Vec::with_capacity(256);
    serde_json::to_writer(&mut buf, v)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "json serialize failed", e))?;
    buf.push(b'\n');
    stdout.write_all(&buf).map_err(map_stdout_err)?;
    let _ = stdout.flush();
    Ok(())
}

fn emit_error<Out: Write, ErrW: Write>(
    cli: &Cli,
    err: &AppError,
    locale: Locale,
    stdout: &mut Out,
    stderr: &mut ErrW,
    stdout_is_terminal: bool,
) -> ExitCode {
    if cli.wants_json(stdout_is_terminal) {
        // Best-effort: preserve the original error exit code even if stdout is gone.
        let _ = write_json(stdout, &error_envelope(err));
    } else {
        // Human path: error on stderr; stdout stays empty.
        // Force with --format text|markdown even when stdout is a pipe.
        let _ = writeln!(stderr, "{}", locale.format_error(err.message()));
        let _ = stderr.flush();
    }
    flush_stdio();
    err.exit_code()
}
