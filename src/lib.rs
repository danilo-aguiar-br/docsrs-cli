//! docsrs-cli library: one-shot run entrypoint for tests and the binary.
//!
//! Lifecycle: BORN → EXECUTE → FINALIZE → DIE (no daemon).

pub mod cache;
pub mod cli;
pub mod config;
pub mod crates_io;
pub mod docs_rs;
pub mod error;
pub mod http;
pub mod i18n;
pub mod item_kind;
pub mod render;
pub mod shutdown;

use std::io::{self, Write};
use std::process::ExitCode;
use std::time::Duration;

use clap::{CommandFactory, Parser};
use serde_json::json;
use tokio::time::Instant;
use tracing_subscriber::EnvFilter;

use crate::cache::DiskCache;
use crate::cli::{CacheAction, Cli, Commands};
use crate::config::{
    APP_NAME, APP_VERSION, Config, MSRV, SCHEMA_VERSION, resolve_version_arg, validate_crate_name,
    validate_item_path, validate_query,
};
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
};

/// Parse argv and execute one command. Returns process exit code.
///
/// This is the binary entrypoint wrapper (stdin/stdout/stderr are process defaults).
pub async fn run<I, T>(args: I) -> ExitCode
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    run_with_io(args, io::stdin(), io::stdout(), io::stderr()).await
}

/// Injectable IO entrypoint for tests (`run(args, stdin, stdout, stderr)`).
///
/// `stdin` is reserved for future payload flows and is unused by current ops.
pub async fn run_with_io<I, T, In, Out, ErrW>(
    args: I,
    _stdin: In,
    mut stdout: Out,
    mut stderr: ErrW,
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

    let code = match run_cli(cli, &mut stdout, &mut stderr).await {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(stderr, "internal: {err}");
            ExitCode::from(70)
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
) -> Result<ExitCode, AppError> {
    if let Err(e) = cli.validate_format_conflict() {
        return Ok(emit_error(&cli, &e, Locale::En, stdout, stderr));
    }

    init_tracing(&cli);

    let mut cfg = Config::load_with_cache_dir(cli.config_dir.clone(), cli.cache_dir.clone())?;
    apply_cli_overrides(&cli, &mut cfg);

    let locale = Locale::detect(cli.lang.as_deref().or(cfg.lang.as_deref()));
    let start = Instant::now();
    let wants_json = cli.wants_json();
    let dry_run = cli.dry_run;
    let wall = Duration::from_secs(cfg.timeout_secs.max(1));
    let cancel = CancelFlag::new();

    let cli_ref = &cli;
    let cfg_ref = &cfg;
    let result = race_op_with_cancel_and_deadline(wall, cancel.clone(), async {
        dispatch(
            cli_ref, cfg_ref, locale, dry_run, wants_json, start, cancel, stdout, stderr,
        )
        .await
    })
    .await;

    match result {
        Ok(code) => Ok(code),
        Err(e) => Ok(emit_error(&cli, &e, locale, stdout, stderr)),
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
        cfg.user_agent = ua.clone();
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
    if let Some(r) = cli.max_retries {
        cfg.max_retries = r;
    }
    if let Some(ref l) = cli.lang {
        cfg.lang = Some(l.clone());
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
}

fn init_tracing(cli: &Cli) {
    let level = if cli.quiet {
        "error"
    } else if cli.verbose >= 2 {
        "trace"
    } else if cli.verbose == 1 {
        "debug"
    } else {
        "error"
    };
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(io::stderr)
        .with_ansi(!cli.no_color && std::env::var_os("NO_COLOR").is_none())
        .try_init();
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
                let data = json!({
                    "name": APP_NAME,
                    "version": APP_VERSION,
                    "msrv": MSRV,
                    "os": std::env::consts::OS,
                });
                write_json(
                    stdout,
                    &success_envelope("version", &data, duration_ms(start), None),
                )?;
            } else {
                writeln!(stdout, "{APP_NAME} {APP_VERSION}")
                    .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Doctor => doctor(cfg, wants_json, start, locale, stdout),
        Commands::Schema { cmd } => schema_cmd(cmd, wants_json, start, stdout),
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell.to_clap_shell(), &mut cmd, APP_NAME, stdout);
            Ok(ExitCode::SUCCESS)
        }
        Commands::Cache { action } => cache_cmd(cfg, action, wants_json, start, stdout),
        Commands::SearchCrates {
            query,
            per_page,
            sort,
            page,
        } => {
            validate_query(query, false)?;
            let sort_s = sort.as_api_str();
            // Clamp once so dry-run planned_params matches the URL and live request.
            let (per_page, page) = crates_io::clamp_search_pagination(*per_page, *page);
            let url = crates_io::planned_url_on_host(
                &cfg.crates_io_origin,
                query,
                per_page,
                sort_s,
                page,
            )?;
            if dry_run {
                return emit_dry_run(
                    "search-crates",
                    url.as_str(),
                    json!({
                        "q": query,
                        "per_page": per_page,
                        "sort": sort_s,
                        "page": page,
                    }),
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
                query,
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
                write!(stdout, "{out}")
                    .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::Readme {
            crate_name,
            crate_version,
        } => {
            validate_crate_name(crate_name)?;
            let version = resolve_version_arg(crate_version.as_deref())?;
            let url = docs_rs::readme_url_on_origin(&cfg.docs_rs_origin, crate_name, &version)?;
            if dry_run {
                return emit_dry_run(
                    "readme",
                    url.as_str(),
                    json!({ "crate": crate_name, "version": version }),
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(crate_name),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data =
                docs_rs::fetch_readme_on_origin(&http, &cfg.docs_rs_origin, crate_name, &version)
                    .await;
            progress.finish();
            let data = apply_truncation_to_readme(data?, cfg);
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("readme", &data, duration_ms(start), Some(&data.source_url)),
                )?;
            } else {
                write!(stdout, "{}", render_readme_markdown(&data))
                    .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
            }
            Ok(ExitCode::SUCCESS)
        }
        Commands::GetItem {
            crate_name,
            item_type,
            item_path,
            crate_version,
        } => {
            validate_crate_name(crate_name)?;
            let kind = ItemKind::parse(item_type)?;
            let segs = validate_item_path(item_path)?;
            if let Some(first) = segs.first() {
                let rustc = item_kind::rustc_crate_name(crate_name);
                if first != crate_name
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
            let version = resolve_version_arg(crate_version.as_deref())?;
            let url = docs_rs::get_item_url_on_origin(
                &cfg.docs_rs_origin,
                crate_name,
                &version,
                kind,
                &segs,
            )?;
            if dry_run {
                return emit_dry_run(
                    "get-item",
                    url.as_str(),
                    json!({
                        "crate": crate_name,
                        "item_type": kind.as_str(),
                        "item_path": item_path,
                        "version": version,
                    }),
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(item_path),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = docs_rs::fetch_item_on_origin(
                &http,
                &cfg.docs_rs_origin,
                crate_name,
                &version,
                kind,
                &segs,
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
                write!(stdout, "{}", render_item_markdown(&data))
                    .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
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
            validate_crate_name(crate_name)?;
            validate_query(query, true)?;
            let kind_filter = match item_type {
                Some(t) if !t.is_empty() => Some(ItemKind::parse(t)?),
                _ => None,
            };
            let version = resolve_version_arg(crate_version.as_deref())?;
            let url = docs_rs::all_html_url_on_origin(&cfg.docs_rs_origin, crate_name, &version)?;
            if dry_run {
                return emit_dry_run(
                    "search-in-crate",
                    url.as_str(),
                    json!({
                        "crate": crate_name,
                        "query": query,
                        "version": version,
                        "item_type": kind_filter.map(|k| k.as_str()),
                        "limit": limit,
                    }),
                    wants_json,
                    start,
                    stdout,
                );
            }
            let progress = ProgressGuard::start(
                cli.quiet,
                Duration::from_secs(2),
                locale.progress_fetching(&format!("{crate_name} all.html")),
            );
            let http = HttpClient::new(cfg.clone(), cancel)?;
            let data = docs_rs::search_in_crate_on_origin(
                &http,
                &cfg.docs_rs_origin,
                crate_name,
                &version,
                query,
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
                write!(stdout, "{out}")
                    .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
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

    checks.push(json!({
        "name": "rustls_client",
        "ok": true,
        "detail": "reqwest built with rustls-tls",
    }));
    checks.push(json!({
        "name": "user_agent",
        "ok": !cfg.user_agent.is_empty() && cfg.user_agent.contains(APP_NAME),
        "detail": cfg.user_agent,
    }));
    checks.push(json!({
        "name": "config_dir",
        "ok": true,
        "detail": cfg.config_dir.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "default-not-resolved".into()),
    }));
    checks.push(json!({
        "name": "timeouts",
        "ok": cfg.timeout_secs > 0 && cfg.connect_timeout_secs > 0,
        "detail": format!("timeout={}s connect={}s", cfg.timeout_secs, cfg.connect_timeout_secs),
    }));
    checks.push(json!({
        "name": "disk_cache",
        "ok": true,
        "detail": if cfg.no_cache {
            "disabled (--no-cache)".to_string()
        } else {
            match &cfg.cache_dir {
                Some(p) => {
                    let stats = DiskCache::new(
                        p.clone(),
                        cfg.cache_ttl(),
                        cfg.max_cache_bytes,
                    )
                    .stats();
                    let budget = if cfg.max_cache_bytes == 0 {
                        "unlimited".to_string()
                    } else {
                        format!("{}B", cfg.max_cache_bytes)
                    };
                    format!(
                        "enabled dir={} ttl={}s max={} entries={} used={}B parser={}",
                        p.display(),
                        cfg.cache_ttl_secs,
                        budget,
                        stats.entries,
                        stats.total_bytes,
                        crate::cache::CACHE_PARSER_VERSION
                    )
                }
                None => "no cache dir resolved".to_string(),
            }
        },
    }));

    let ok = checks.iter().all(|c| c["ok"].as_bool() == Some(true));
    let data = json!({ "ok": ok, "checks": checks });
    if wants_json {
        write_json(
            stdout,
            &success_envelope("doctor", &data, duration_ms(start), None),
        )?;
    } else {
        writeln!(stdout, "{}", locale.doctor_ok(ok))
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
        for c in &checks {
            writeln!(
                stdout,
                "- {} [{}] {}",
                c["name"].as_str().unwrap_or("?"),
                if c["ok"].as_bool() == Some(true) {
                    "ok"
                } else {
                    "fail"
                },
                c["detail"].as_str().unwrap_or("")
            )
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(78)
    })
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
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
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
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn schema_cmd<Out: Write>(
    cmd: &str,
    wants_json: bool,
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
        "cache" | "cache-clear" | "cache-stats" => {
            include_str!("../docs/schemas/cache.schema.json")
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
    let data = json!({ "command": cmd, "schema": schema_val, "schema_version": SCHEMA_VERSION });
    if wants_json {
        write_json(
            stdout,
            &success_envelope("schema", &data, duration_ms(start), None),
        )?;
    } else {
        writeln!(
            stdout,
            "{}",
            serde_json::to_string_pretty(&schema_val).unwrap_or_default()
        )
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn emit_dry_run<Out: Write>(
    command: &str,
    planned_url: &str,
    planned_params: serde_json::Value,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    if wants_json {
        write_json(
            stdout,
            &dry_run_envelope(command, planned_url, planned_params, duration_ms(start)),
        )?;
    } else {
        writeln!(stdout, "dry-run {command}")
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
        writeln!(stdout, "planned_url: {planned_url}")
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
        writeln!(
            stdout,
            "planned_params: {}",
            serde_json::to_string_pretty(&planned_params).unwrap_or_default()
        )
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
    }
    Ok(ExitCode::SUCCESS)
}

fn write_json<Out: Write>(stdout: &mut Out, v: &serde_json::Value) -> AppResult<()> {
    let s = serde_json::to_string(v)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "json serialize failed", e))?;
    writeln!(stdout, "{s}")
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "stdout write", e))?;
    let _ = stdout.flush();
    Ok(())
}

fn emit_error<Out: Write, ErrW: Write>(
    cli: &Cli,
    err: &AppError,
    locale: Locale,
    stdout: &mut Out,
    stderr: &mut ErrW,
) -> ExitCode {
    if cli.wants_json() {
        let env = error_envelope(err);
        if let Ok(s) = serde_json::to_string(&env) {
            let _ = writeln!(stdout, "{s}");
            let _ = stdout.flush();
        }
    } else {
        // PRD: without --json, human error on stderr; stdout stays empty.
        let _ = writeln!(stderr, "{}", locale.format_error(err.message()));
        let _ = stderr.flush();
    }
    flush_stdio();
    err.exit_code()
}
