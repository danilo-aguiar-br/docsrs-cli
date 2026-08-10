//! Subcommand routing for the one-shot run.
//!
//! Split out of `lib.rs`, which is the crate façade: module declarations plus
//! the `run` / `run_with_io` entry points and the stdout/stderr wiring around
//! them. Deciding *which* operation a parsed `Commands` maps to is a separate
//! responsibility, and it is the one that grows with every new subcommand.
//!
//! Both functions stay crate-private: they are an internal seam, never API.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use tokio::time::Instant;

use crate::cli::{Cli, Commands};
use crate::config::{APP_NAME, APP_VERSION, Config, MSRV};
use crate::error::{AppResult, EXIT_BROKEN_PIPE, ErrorKind};
use crate::i18n::Locale;
use crate::render::success_envelope;
use crate::shutdown::{
    CancelFlag, duration_ms, race_op_with_cancel_and_deadline, spawn_double_interrupt_force_exit,
};
use crate::{doctor, meta_cmds, ops, output};

/// Run the dispatch race and turn any escaping error into an emitted envelope.
///
/// Split out of `run_cli` so the same body serves both the direct-to-stdout path and
/// the capture buffer used by agent-native reduction.
pub(crate) async fn execute<Out: Write, ErrW: Write>(
    cli: &Cli,
    cfg: &Config,
    locale: Locale,
    stdout: &mut Out,
    stderr: &mut ErrW,
    stdout_is_terminal: bool,
    wire: &'static str,
) -> ExitCode {
    let start = Instant::now();
    let wants_json = cli.wants_json(stdout_is_terminal);
    let dry_run = cli.dry_run;
    let wall = Duration::from_secs(cfg.timeout_secs.max(1));
    let cancel = CancelFlag::new();
    // Rules Rust: first Ctrl-C is cooperative; second within 5s force-exits 130.
    let force_on_second = spawn_double_interrupt_force_exit();

    let result = race_op_with_cancel_and_deadline(wall, cancel.clone(), async {
        dispatch(
            cli, cfg, locale, dry_run, wants_json, start, cancel, stdout, stderr,
        )
        .await
    })
    .await;

    force_on_second.abort();

    match result {
        Ok(code) => code,
        Err(e) if e.kind() == ErrorKind::BrokenPipe => ExitCode::from(EXIT_BROKEN_PIPE),
        Err(e) => meta_cmds::emit_error(
            cli,
            &e,
            locale,
            stdout,
            stderr,
            stdout_is_terminal,
            wire,
            duration_ms(start),
        ),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn dispatch<Out: Write, ErrW: Write>(
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
        Commands::Doctor { online } => {
            doctor::doctor(cfg, *online, wants_json, start, locale, stdout)
        }
        Commands::Commands => meta_cmds::commands_cmd(wants_json, start, stdout),
        Commands::Schema { cmd } => {
            meta_cmds::schema_cmd(cmd, wants_json, cli.format, start, stdout)
        }
        Commands::Completions { shell } => {
            // Completions emit shell script by default (even on non-TTY). JSON only if --json/--format json.
            let explicit_json =
                cli.json || matches!(cli.format, Some(crate::cli::OutputFormat::Json));
            meta_cmds::completions_cmd(*shell, explicit_json, start, stdout)
        }
        Commands::Cache { action } => meta_cmds::cache_cmd(cfg, action, wants_json, start, stdout),
        Commands::Config { action } => {
            meta_cmds::config_cmd(cli, cfg, action, wants_json, start, stdout)
        }
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
