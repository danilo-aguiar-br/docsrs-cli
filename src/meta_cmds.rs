//! Local meta commands: cache, config, schema, completions, command tree, dry-run/error emit.
//!
//! No product network I/O. One-shot helpers for agent discovery and local state.

use std::io::Write;
use std::process::ExitCode;

use clap::CommandFactory;
use serde::Serialize;
use tokio::time::Instant;

use crate::cache::DiskCache;
use crate::cli::{CacheAction, Cli, ConfigAction};
use crate::config::{
    APP_NAME, APP_VERSION, Config, MSRV, SCHEMA_VERSION, config_path_data, init_config_toml,
};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::i18n::Locale;
use crate::output;
use crate::render::{self, dry_run_envelope, error_envelope, success_envelope};
use crate::shutdown::{duration_ms, flush_stdio};

/// Cache path inventory for `cache path` (agent-readable).
#[derive(Debug, Clone, Serialize)]
struct CachePathData {
    root: Option<String>,
    source: crate::config::PathSource,
    no_cache: bool,
}

/// Inspect or clear the XDG HTTP disk cache (and report path).
///
/// # Errors
///
/// - [`ErrorKind::Config`] when the cache directory cannot be resolved
/// - [`ErrorKind::Internal`] / I/O kinds from cache clear/stats or stdout write
pub(crate) fn cache_cmd<Out: Write>(
    cfg: &Config,
    action: &CacheAction,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    if matches!(action, CacheAction::Path) {
        let data = CachePathData {
            root: cfg.cache_dir.as_ref().map(|p| p.display().to_string()),
            source: cfg.cache_path_source,
            no_cache: cfg.no_cache,
        };
        if wants_json {
            write_json(
                stdout,
                &success_envelope("cache-path", &data, duration_ms(start), None),
            )?;
        } else {
            writeln!(
                stdout,
                "cache_root={} source={} no_cache={}",
                data.root.as_deref().unwrap_or("<unresolved>"),
                data.source.as_str(),
                data.no_cache
            )
            .map_err(output::map_stdout_err)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let root = cfg.cache_dir.clone().ok_or_else(|| {
        AppError::new(
            ErrorKind::Config,
            "cache directory could not be resolved (set --cache-dir or ensure XDG cache home)",
        )
    })?;
    let cache = DiskCache::new(root, cfg.cache_ttl(), cfg.max_cache_bytes, cfg.allow_loopback);
    match action {
        CacheAction::Path => unreachable!("handled above"),
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
                .map_err(output::map_stdout_err)?;
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
                .map_err(output::map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// XDG config lifecycle: path inventory, effective show, init template.
///
/// # Errors
///
/// - [`ErrorKind::Config`] when init cannot resolve the config directory
/// - [`ErrorKind::Internal`] on serialize/stdout failures
pub(crate) fn config_cmd<Out: Write>(
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
                .map_err(output::map_stdout_err)?;
                writeln!(
                    stdout,
                    "config_file={} exists={} loaded={}",
                    data.config_file.as_deref().unwrap_or("<unresolved>"),
                    data.config_file_exists,
                    data.config_toml_loaded
                )
                .map_err(output::map_stdout_err)?;
                writeln!(
                    stdout,
                    "cache_dir={} source={}",
                    data.cache_dir.as_deref().unwrap_or("<unresolved>"),
                    data.cache_source.as_str()
                )
                .map_err(output::map_stdout_err)?;
                writeln!(
                    stdout,
                    "dotenv_runtime={} secrets={}",
                    data.dotenv_runtime, data.secrets_layers
                )
                .map_err(output::map_stdout_err)?;
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
                writeln!(stdout, "{pretty}").map_err(output::map_stdout_err)?;
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
                .map_err(output::map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Emit the full command tree for agent discovery (`commands` subcommand).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] / broken-pipe mapping on stdout write failure.
pub(crate) fn commands_cmd<Out: Write>(
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
        writeln!(stdout, "{} {} — command tree", data.name, data.version)
            .map_err(output::map_stdout_err)?;
        for c in data.commands {
            writeln!(stdout, "- {}: {}", c.name, c.about).map_err(output::map_stdout_err)?;
            for s in c.subcommands {
                writeln!(stdout, "  - {} {}: {}", c.name, s.name, s.about)
                    .map_err(output::map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// Typed `version` success data (matches `docs/schemas/version.schema.json`).
#[derive(Debug, Serialize)]
pub(crate) struct VersionData {
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) msrv: &'static str,
    pub(crate) os: &'static str,
    pub(crate) arch: &'static str,
}



/// Agent notes embedded in the `commands` tree.
#[derive(Debug, Serialize)]
pub(crate) struct AgentNotes {
    stdout: &'static str,
    stderr: &'static str,
    json_auto: &'static str,
    lifecycle: &'static str,
}

/// One nested subcommand entry under `cache`.
#[derive(Debug, Serialize)]
pub(crate) struct SubCommandNode {
    name: &'static str,
    about: &'static str,
}

/// One top-level command entry in the discovery tree.
#[derive(Debug, Serialize)]
pub(crate) struct CommandNode {
    name: &'static str,
    about: &'static str,
    args: &'static [&'static str],
    subcommands: &'static [SubCommandNode],
}

/// Stable, ordered command tree for agents (no HashMap iteration).
#[derive(Debug, Serialize)]
pub(crate) struct CommandTree {
    name: &'static str,
    version: &'static str,
    msrv: &'static str,
    schema_version: u32,
    agent_notes: AgentNotes,
    commands: &'static [CommandNode],
}

/// Schema command payload (`schema` stays `Value` — embedded JSON Schema document).
#[derive(Debug, Serialize)]
pub(crate) struct SchemaData<'a> {
    command: &'a str,
    schema: serde_json::Value,
    schema_version: u32,
}

/// Completions command payload.
#[derive(Debug, Serialize)]
pub(crate) struct CompletionsData {
    shell: &'static str,
    script: String,
}

/// Stable, ordered command tree for agents (no HashMap iteration).
pub(crate) fn command_tree_data() -> CommandTree {
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
                args: &["query", "--per-page", "--sort", "--page", "--page-token"],
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
                args: &[
                    "crate_name",
                    "item_type",
                    "item_path",
                    "--crate-version",
                    "--suggest",
                ],
                subcommands: &[],
            },
            CommandNode {
                name: "search-in-crate",
                about: "Search symbols in crate all.html index",
                args: &[
                    "crate_name",
                    "query",
                    "--crate-version",
                    "--item-type",
                    "--limit",
                    "--match",
                ],
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
                args: &["--online"],
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
                about: "Emit JSON Schema for a command envelope; pass --cmd all for the full bundle",
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
                        name: "path",
                        about: "Print resolved cache root and which layer won",
                    },
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

/// Canonical schema command names (single-cmd and `--cmd all`).
fn schema_command_names() -> &'static [&'static str] {
    &[
        "search-crates",
        "readme",
        "get-item",
        "search-in-crate",
        "version",
        "doctor",
        "commands",
        "schema",
        "completions",
        "error",
        "dry-run",
        "cache",
        "cache-path",
        "cache-clear",
        "cache-stats",
        "config",
        "config-path",
        "config-show",
        "config-init",
    ]
}

/// Embedded JSON Schema body for a known command name.
fn schema_json_for_cmd(cmd: &str) -> AppResult<&'static str> {
    Ok(match cmd {
        "search-crates" => include_str!("../docs/schemas/search-crates.schema.json"),
        "readme" => include_str!("../docs/schemas/readme.schema.json"),
        "get-item" => include_str!("../docs/schemas/get-item.schema.json"),
        "search-in-crate" => include_str!("../docs/schemas/search-in-crate.schema.json"),
        "version" => include_str!("../docs/schemas/version.schema.json"),
        "doctor" => include_str!("../docs/schemas/doctor.schema.json"),
        "commands" => include_str!("../docs/schemas/commands.schema.json"),
        "schema" => include_str!("../docs/schemas/schema.schema.json"),
        "completions" => include_str!("../docs/schemas/completions.schema.json"),
        "error" => include_str!("../docs/schemas/error.schema.json"),
        "dry-run" => include_str!("../docs/schemas/dry-run.schema.json"),
        "cache" | "cache-path" | "cache-clear" | "cache-stats" => {
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
    })
}

/// Schema bundle item for `schema --cmd all`.
#[derive(Debug, Serialize)]
struct SchemaAllItem {
    cmd: &'static str,
    schema: serde_json::Value,
}

/// Payload for `schema --cmd all`.
#[derive(Debug, Serialize)]
struct SchemaAllData {
    mode: &'static str,
    commands: &'static [&'static str],
    items: Vec<SchemaAllItem>,
    schema_version: u32,
}

/// Emit the JSON Schema (or human markdown/text) for a command envelope.
///
/// # Errors
///
/// - [`ErrorKind::Usage`] for an unknown schema command name
/// - [`ErrorKind::Internal`] when an embedded schema is invalid JSON or pretty-print fails
/// - stdout write / broken-pipe mapping via [`crate::output::map_stdout_err`]
pub(crate) fn schema_cmd<Out: Write>(
    cmd: &str,
    wants_json: bool,
    format: Option<crate::cli::OutputFormat>,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    if cmd == "all" {
        let mut items = Vec::with_capacity(schema_command_names().len());
        for name in schema_command_names() {
            let raw = schema_json_for_cmd(name)?;
            let schema_val: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "embedded schema is invalid JSON", e)
            })?;
            items.push(SchemaAllItem {
                cmd: name,
                schema: schema_val,
            });
        }
        if wants_json {
            let data = SchemaAllData {
                mode: "all",
                commands: schema_command_names(),
                items,
                schema_version: SCHEMA_VERSION,
            };
            write_json(
                stdout,
                &success_envelope("schema", &data, duration_ms(start), None),
            )?;
        } else {
            writeln!(stdout, "schema commands:").map_err(output::map_stdout_err)?;
            for name in schema_command_names() {
                writeln!(stdout, "  {name}").map_err(output::map_stdout_err)?;
            }
            writeln!(
                stdout,
                "use --json for the full schema bundle (schema --cmd all --json)"
            )
            .map_err(output::map_stdout_err)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let schema = schema_json_for_cmd(cmd)?;
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
        let md = render::render_schema_markdown(cmd, &schema_val)?;
        write!(stdout, "{md}").map_err(output::map_stdout_err)?;
    } else {
        let pretty = serde_json::to_string_pretty(&schema_val).map_err(|e| {
            AppError::with_source(ErrorKind::Internal, "json pretty-print failed", e)
        })?;
        writeln!(stdout, "{pretty}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Generate shell completion scripts.
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] / broken-pipe mapping on stdout write failure.
pub(crate) fn completions_cmd<Out: Write>(
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
        write!(stdout, "{script}").map_err(output::map_stdout_err)?;
        let _ = stdout.flush();
    }
    Ok(ExitCode::SUCCESS)
}

/// Typed dry-run params for `search-crates` (keys match crates.io query string).
#[derive(Debug, Serialize)]
pub(crate) struct SearchCratesDryParams<'a> {
    pub(crate) q: &'a str,
    pub(crate) per_page: u32,
    pub(crate) sort: &'a str,
    pub(crate) page: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) page_token: Option<&'a str>,
}

/// Typed dry-run params for `readme`.
#[derive(Debug, Serialize)]
pub(crate) struct ReadmeDryParams<'a> {
    pub(crate) crate_name: &'a str,
    pub(crate) version: &'a str,
}

/// Typed dry-run params for `get-item`.
#[derive(Debug, Serialize)]
pub(crate) struct GetItemDryParams<'a> {
    pub(crate) crate_name: &'a str,
    pub(crate) item_type: &'a str,
    pub(crate) item_path: &'a str,
    pub(crate) version: &'a str,
    /// Always `url_shape_only` — dry-run does not verify remote existence (X-007).
    pub(crate) validation: &'static str,
    /// First parent kind planned for associated methods (`struct`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) planned_parent_kind: Option<&'static str>,
    /// Full live probe order for method parents (struct→…→union).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent_kind_probe: Option<&'static [&'static str]>,
}

/// Typed dry-run params for `search-in-crate`.
#[derive(Debug, Serialize)]
pub(crate) struct SearchInCrateDryParams<'a> {
    pub(crate) crate_name: &'a str,
    pub(crate) query: &'a str,
    pub(crate) version: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) item_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) match_mode: Option<&'a str>,
    pub(crate) limit: u32,
}

/// Emit a dry-run success envelope (JSON) or human planned URL/params.
///
/// # Errors
///
/// - [`ErrorKind::Internal`] when JSON serialize/pretty-print fails
/// - stdout write / broken-pipe mapping
pub(crate) fn emit_dry_run<Out: Write, P: Serialize>(
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
        writeln!(stdout, "dry-run {command}").map_err(output::map_stdout_err)?;
        writeln!(stdout, "planned_url: {planned_url}").map_err(output::map_stdout_err)?;
        let pretty = serde_json::to_string_pretty(&planned_params).map_err(|e| {
            AppError::with_source(ErrorKind::Internal, "json pretty-print failed", e)
        })?;
        writeln!(stdout, "planned_params: {pretty}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
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
    stdout.write_all(&buf).map_err(output::map_stdout_err)?;
    // Best-effort: payload already written; flush failure must not change exit code.
    let _ = stdout.flush();
    Ok(())
}

#[allow(clippy::too_many_arguments)] // I/O pair + locale + wire command/duration (one-shot emit)
pub(crate) fn emit_error<Out: Write, ErrW: Write>(
    cli: &Cli,
    err: &AppError,
    locale: Locale,
    stdout: &mut Out,
    stderr: &mut ErrW,
    stdout_is_terminal: bool,
    command: &str,
    duration_ms: u64,
) -> ExitCode {
    if cli.wants_json(stdout_is_terminal) {
        // Best-effort: preserve the original error exit code even if stdout is gone.
        let _ = write_json(stdout, &error_envelope(err, command, duration_ms));
    } else {
        // Human path: error on stderr; stdout stays empty.
        // Force with --format text|markdown even when stdout is a pipe.
        let _ = writeln!(stderr, "{}", locale.format_error(err.message()));
        let _ = stderr.flush();
    }
    flush_stdio();
    err.exit_code()
}
