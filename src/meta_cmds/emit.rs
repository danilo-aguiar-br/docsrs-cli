//! Error envelope emission and the typed `version` payload.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;

use crate::cli::Cli;
use crate::error::AppError;
use crate::i18n::Locale;
use crate::output::write_json;
use crate::render::error_envelope;
use crate::shutdown::flush_stdio;

/// Typed `version` success data (matches `docs/schemas/version.schema.json`).
#[derive(Debug, Serialize)]
pub(crate) struct VersionData {
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) msrv: &'static str,
    pub(crate) os: &'static str,
    pub(crate) arch: &'static str,
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
        let _ = writeln!(stderr, "{}", locale.format_error(err));
        let _ = stderr.flush();
    }
    flush_stdio();
    err.exit_code()
}
