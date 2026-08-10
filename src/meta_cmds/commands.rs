//! `commands` — stable, ordered command tree for agent discovery.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use tokio::time::Instant;

use crate::config::{APP_NAME, APP_VERSION, MSRV, SCHEMA_VERSION};
use crate::error::AppResult;
use crate::output::{self, write_json};
use crate::render::success_envelope;
use crate::shutdown::duration_ms;

/// Emit the full command tree for agent discovery (`commands` subcommand).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] / broken-pipe mapping on stdout write failure.
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
