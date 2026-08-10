//! `schema` — emit the embedded JSON Schema for one command or the full bundle.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use tokio::time::Instant;

use crate::config::SCHEMA_VERSION;
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};
use crate::output::{self, write_json};
use crate::render::{self, success_envelope};
use crate::shutdown::duration_ms;

/// Schema command payload (`schema` stays `Value` — embedded JSON Schema document).
#[derive(Debug, Serialize)]
pub(crate) struct SchemaData<'a> {
    command: &'a str,
    schema: serde_json::Value,
    schema_version: u32,
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
        "agent-surface",
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
        "search-crates" => include_str!("../../docs/schemas/search-crates.schema.json"),
        "readme" => include_str!("../../docs/schemas/readme.schema.json"),
        "get-item" => include_str!("../../docs/schemas/get-item.schema.json"),
        "search-in-crate" => include_str!("../../docs/schemas/search-in-crate.schema.json"),
        "version" => include_str!("../../docs/schemas/version.schema.json"),
        "doctor" => include_str!("../../docs/schemas/doctor.schema.json"),
        "commands" => include_str!("../../docs/schemas/commands.schema.json"),
        "schema" => include_str!("../../docs/schemas/schema.schema.json"),
        "completions" => include_str!("../../docs/schemas/completions.schema.json"),
        "error" => include_str!("../../docs/schemas/error.schema.json"),
        "dry-run" => include_str!("../../docs/schemas/dry-run.schema.json"),
        "agent-surface" => include_str!("../../docs/schemas/agent-surface.schema.json"),
        // Each subcommand serves its own file. The six specific names used to
        // fall through to the umbrella schema, so `schema --cmd config-show`
        // answered with the shape of `config path|show|init` in general — which
        // does not carry `log_directive`, a key `config show` does emit. An
        // agent validating against that reply would reject a valid payload, or
        // never learn the field exists. The files were versioned all along; only
        // the dispatch pointed elsewhere.
        "cache" => include_str!("../../docs/schemas/cache.schema.json"),
        "cache-path" => include_str!("../../docs/schemas/cache-path.schema.json"),
        "cache-clear" => include_str!("../../docs/schemas/cache-clear.schema.json"),
        "cache-stats" => include_str!("../../docs/schemas/cache-stats.schema.json"),
        "config" => include_str!("../../docs/schemas/config.schema.json"),
        "config-path" => include_str!("../../docs/schemas/config-path.schema.json"),
        "config-show" => include_str!("../../docs/schemas/config-show.schema.json"),
        "config-init" => include_str!("../../docs/schemas/config-init.schema.json"),
        other => {
            return Err(AppError::of(ErrorDetail::UnknownSchemaCommand {
                value: other.to_string(),
            }));
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
/// - [`crate::error::ErrorKind::Usage`] for an unknown schema command name
/// - [`crate::error::ErrorKind::Internal`] when an embedded schema is invalid JSON or pretty-print fails
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
                AppError::of_with_source(
                    ErrorDetail::Internal {
                        op: InternalOp::EmbeddedSchemaInvalid,
                    },
                    e,
                )
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
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::EmbeddedSchemaInvalid,
            },
            e,
        )
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
            AppError::of_with_source(
                ErrorDetail::Internal {
                    op: InternalOp::JsonPrettyPrint,
                },
                e,
            )
        })?;
        writeln!(stdout, "{pretty}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
