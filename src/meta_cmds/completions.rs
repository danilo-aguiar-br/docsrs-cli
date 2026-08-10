//! `completions` — generate shell completion scripts.

use std::io::Write;
use std::process::ExitCode;

use clap::CommandFactory;
use serde::Serialize;
use tokio::time::Instant;

use crate::cli::Cli;
use crate::config::APP_NAME;
use crate::error::AppResult;
use crate::output::{self, write_json};
use crate::render::success_envelope;
use crate::shutdown::duration_ms;

/// Completions command payload.
#[derive(Debug, Serialize)]
pub(crate) struct CompletionsData {
    shell: &'static str,
    script: String,
}

/// Generate shell completion scripts.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] / broken-pipe mapping on stdout write failure.
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
