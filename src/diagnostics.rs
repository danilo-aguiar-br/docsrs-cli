//! Process-local diagnostic subscriber for the CLI entry path.
//!
//! # Binary vs library
//!
//! Product modules (`http`, `cache`, `docs_rs`, …) only **emit** events/spans.
//! Global subscriber installation happens once on the process entry path
//! ([`crate::run`] / [`crate::run_with_io`]), not from pure library helpers.
//!
//! # Agent-first sinks
//!
//! - **stdout**: structured command output (JSON/Markdown) — never diagnostics
//! - **stderr**: tracing events, progress lines, human errors
//!
//! File rotation (`tracing-appender`), OpenTelemetry/OTLP, `reload::Layer` admin
//! endpoints, and `tokio-console` are **out of scope** for this one-shot CLI:
//! there is no long-lived process, no admin surface, and no product telemetry
//! export. Diagnostics stay on stderr for agent capture.
//!
//! # Targets
//!
//! Events use module paths under `docsrs_cli::…` (e.g. `docsrs_cli::http`,
//! `docsrs_cli::cache`, `docsrs_cli::diagnostics`). Override with `RUST_LOG`
//! (e.g. `RUST_LOG=docsrs_cli=debug,docsrs_cli::http=trace`).
//!
//! # CLI vs env
//!
//! `-v` / `-vv` / `-q` set the default directive when `RUST_LOG` is unset.
//! When `RUST_LOG` is set, it wins over CLI verbosity (standard EnvFilter).

use std::io;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::Cli;

/// Install the global `tracing` subscriber once for this process.
///
/// Writes to **stderr** with `EnvFilter` from `RUST_LOG` or CLI verbosity.
/// Idempotent: a second call (integration tests re-enter `run_with_io`) is a
/// no-op so the first install wins for the process lifetime.
///
/// Does **not** return a `WorkerGuard`: the writer is stderr (no background
/// appender thread). FINALIZE still flushes stdio via [`crate::shutdown::flush_stdio`].
pub fn init_tracing(cli: &Cli) {
    let default_level = default_directive(cli);
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let filter_display = filter.to_string();
    // Color policy: --no-color, NO_COLOR, TERM=dumb, CLICOLOR_FORCE (see platform).
    let ansi = crate::platform::ansi_colors_enabled(cli.no_color);

    // Registry + fmt layer: composable base; log crate bridge via feature
    // `tracing-log` on tracing-subscriber (reqwest/html5ever emit via `log`).
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_writer(io::stderr)
        .with_ansi(ansi)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false)
        .with_thread_names(false);

    let result = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .try_init();

    match result {
        Ok(()) => {
            tracing::debug!(
                target: "docsrs_cli::diagnostics",
                filter = %filter_display,
                default_level,
                ansi,
                "tracing subscriber installed (stderr diagnostics)"
            );
        }
        Err(_) => {
            // Already installed in this process (tests call run_with_io many times).
            // Do not panic; first successful install owns the global default.
        }
    }
}

/// Default EnvFilter directive from `-q` / `-v` when `RUST_LOG` is unset.
///
/// Agent-first default is `error` (quiet stderr unless the operator asks).
pub(crate) fn default_directive(cli: &Cli) -> &'static str {
    if cli.quiet {
        "error"
    } else if cli.verbose >= 2 {
        "trace"
    } else if cli.verbose == 1 {
        "debug"
    } else {
        "error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{Cli, Commands};
    use clap::Parser;

    fn cli_with(args: &[&str]) -> Cli {
        Cli::try_parse_from(args).expect("parse test argv")
    }

    #[test]
    fn default_directive_agent_first_is_error() {
        let cli = cli_with(&["docsrs-cli", "version"]);
        assert_eq!(default_directive(&cli), "error");
    }

    #[test]
    fn default_directive_verbose_debug_and_trace() {
        let v1 = cli_with(&["docsrs-cli", "-v", "version"]);
        assert_eq!(default_directive(&v1), "debug");
        let v2 = cli_with(&["docsrs-cli", "-vv", "version"]);
        assert_eq!(default_directive(&v2), "trace");
    }

    #[test]
    fn default_directive_quiet_wins_over_verbose() {
        let cli = cli_with(&["docsrs-cli", "-q", "-vv", "version"]);
        assert_eq!(default_directive(&cli), "error");
        let _ = cli.command;
    }

    #[test]
    fn init_tracing_is_idempotent() {
        let cli = cli_with(&["docsrs-cli", "version"]);
        init_tracing(&cli);
        init_tracing(&cli);
        // Second call must not panic; command still parseable.
        assert!(matches!(cli.command, Commands::Version));
    }
}
