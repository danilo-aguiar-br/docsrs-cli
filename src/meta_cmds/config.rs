//! `config path|show|init` — XDG config lifecycle (no secrets, no .env).

use std::io::Write;
use std::process::ExitCode;

use tokio::time::Instant;

use crate::cli::destructive::CONFIG_INIT_FORCE;
use crate::cli::{Cli, ConfigAction};
use crate::config::{CONFIG_FILE_NAME, Config, config_path_data, init_config_toml};
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};
use crate::output::{self, write_json};
use crate::render::success_envelope;
use crate::shutdown::duration_ms;

/// XDG config lifecycle: path inventory, effective show, init template.
///
/// # Errors
///
/// - [`crate::error::ErrorKind::Config`] when init cannot resolve the config directory
/// - [`crate::error::ErrorKind::Internal`] on serialize/stdout failures
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
                    AppError::of_with_source(
                        ErrorDetail::Internal {
                            op: InternalOp::JsonSerialize,
                        },
                        e,
                    )
                })?;
                writeln!(stdout, "{pretty}").map_err(output::map_stdout_err)?;
            }
        }
        ConfigAction::Init { force, yes } => {
            // Explicit Target Designation. Only `--force` destroys, and the
            // waiver is demanded whenever the target is ambient — including
            // where no file exists yet, because the caller cannot know that
            // about a directory they never named, and a rule answering from the
            // state of the disk would give the same argv two behaviours.
            if *force && CONFIG_INIT_FORCE.must_refuse(*yes, cfg.config_path_source) {
                // Name the file, not just the directory: the caller needs to
                // recognise what would have been replaced.
                let target = cfg.config_dir.as_ref().map_or_else(
                    || "<unresolved>".to_string(),
                    |d| d.join(CONFIG_FILE_NAME).display().to_string(),
                );
                return Err(AppError::of(CONFIG_INIT_FORCE.refuse(target)));
            }
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
                    "config init: {verb} {} (target_source={})",
                    result.path,
                    result.target_source.as_str()
                )
                .map_err(output::map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
