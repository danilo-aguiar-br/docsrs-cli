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
//! `docsrs_cli::cache`, `docsrs_cli::diagnostics`). Steer them with `-v` / `-q`
//! or with the XDG key `log_directive`
//! (e.g. `log_directive = "docsrs_cli=debug,docsrs_cli::http=trace"`).
//!
//! # No product environment variable
//!
//! The filter is built from CLI flags and `config.toml` only. `RUST_LOG` is
//! **not** read (ADR 0009).
//!
//! This module used to call `EnvFilter::try_from_default_env()`, which let
//! `RUST_LOG` override `-q` — an ambient value silently outranking an explicit
//! flag. The practical harm is not the rule violation: it made the product
//! unconfigurable *by the product*, because no `docsrs-cli` command could turn
//! that stderr noise off, while `doctor` kept reporting `(no env)`.
//!
//! Terminal-capability variables (`NO_COLOR`, `TERM`, `CLICOLOR_FORCE`) are a
//! different category and stay: they describe the *device*, like `isatty`, and
//! never carry product configuration. See [`crate::platform`].

use std::io;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::cli::Cli;
use crate::config::DEFAULT_LOG_DIRECTIVE;

/// Install the global `tracing` subscriber once for this process.
///
/// Writes to **stderr** with an `EnvFilter` built from CLI verbosity and the
/// XDG `log_directive` key — never from the environment. Idempotent: a second
/// call (integration tests re-enter `run_with_io`) is a no-op so the first
/// install wins for the process lifetime.
///
/// `toml_directive` comes from `config.toml`; an unparseable value falls back to
/// [`DEFAULT_LOG_DIRECTIVE`] instead of aborting, because a typo in an optional
/// diagnostics knob must never take the command down with it.
///
/// Does **not** return a `WorkerGuard`: the writer is stderr (no background
/// appender thread). FINALIZE still flushes stdio via [`crate::shutdown::flush_stdio`].
pub fn init_tracing(cli: &Cli, toml_directive: Option<&str>) {
    let directive = resolve_directive(cli, toml_directive);
    let filter =
        EnvFilter::try_new(directive).unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_DIRECTIVE));
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

/// Resolve the effective filter directive: CLI flags, then TOML, then default.
///
/// An explicit `-q` / `-v` always outranks `config.toml`, so the flag the
/// operator typed in this invocation wins over the file they wrote once.
pub(crate) fn resolve_directive<'a>(cli: &Cli, toml_directive: Option<&'a str>) -> &'a str {
    if let Some(explicit) = cli_directive(cli) {
        return explicit;
    }
    toml_directive
        .map(str::trim)
        .filter(|d| !d.is_empty())
        .unwrap_or(DEFAULT_LOG_DIRECTIVE)
}

/// Directive demanded by `-q` / `-v`, or `None` when neither was passed.
///
/// Returning `None` (not the default) is what lets `log_directive` apply: a
/// default folded in here would be indistinguishable from an explicit request
/// and would silently outrank the file.
fn cli_directive(cli: &Cli) -> Option<&'static str> {
    if cli.quiet {
        Some("error")
    } else if cli.verbose >= 2 {
        Some("trace")
    } else if cli.verbose == 1 {
        Some("debug")
    } else {
        None
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
    fn no_flags_and_no_toml_key_is_error() {
        let cli = cli_with(&["docsrs-cli", "version"]);
        assert_eq!(resolve_directive(&cli, None), "error");
    }

    #[test]
    fn verbose_maps_to_debug_and_trace() {
        let v1 = cli_with(&["docsrs-cli", "-v", "version"]);
        assert_eq!(resolve_directive(&v1, None), "debug");
        let v2 = cli_with(&["docsrs-cli", "-vv", "version"]);
        assert_eq!(resolve_directive(&v2, None), "trace");
    }

    #[test]
    fn quiet_wins_over_verbose() {
        let cli = cli_with(&["docsrs-cli", "-q", "-vv", "version"]);
        assert_eq!(resolve_directive(&cli, None), "error");
        let _ = cli.command;
    }

    #[test]
    fn init_tracing_is_idempotent() {
        let cli = cli_with(&["docsrs-cli", "version"]);
        init_tracing(&cli, None);
        init_tracing(&cli, None);
        // Second call must not panic; command still parseable.
        assert!(matches!(cli.command, Commands::Version));
    }

    #[test]
    fn toml_directive_applies_only_without_explicit_flags() {
        // GAP-ENV-001: the XDG key replaces `RUST_LOG`, so it must behave the way
        // a config file should — yielding to the flag typed in this invocation.
        let bare = cli_with(&["docsrs-cli", "version"]);
        assert_eq!(
            resolve_directive(&bare, Some("docsrs_cli=debug")),
            "docsrs_cli=debug"
        );

        let quiet = cli_with(&["docsrs-cli", "-q", "version"]);
        assert_eq!(resolve_directive(&quiet, Some("docsrs_cli=trace")), "error");

        let verbose = cli_with(&["docsrs-cli", "-v", "version"]);
        assert_eq!(
            resolve_directive(&verbose, Some("docsrs_cli=trace")),
            "debug"
        );
    }

    #[test]
    fn blank_toml_directive_falls_back_to_default() {
        // A key present but empty is an operator typo, not a request for silence
        // of an unparseable kind; treat it as absent rather than as a directive.
        let cli = cli_with(&["docsrs-cli", "version"]);
        assert_eq!(resolve_directive(&cli, Some("   ")), DEFAULT_LOG_DIRECTIVE);
        assert_eq!(resolve_directive(&cli, None), DEFAULT_LOG_DIRECTIVE);
    }
}
