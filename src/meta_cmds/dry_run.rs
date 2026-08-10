//! Typed dry-run parameter payloads and the dry-run envelope emitter.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use tokio::time::Instant;

use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};
use crate::output::{self, write_json};
use crate::render::dry_run_envelope;
use crate::shutdown::duration_ms;

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
    ///
    /// Owned rather than borrowed because the names are now derived from the
    /// kind list instead of being kept as a parallel `static` array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parent_kind_probe: Option<Vec<&'static str>>,
    /// Anchor ids the live run will try, in order (`method.X`, then `tymethod.X`).
    ///
    /// `planned_url` can only carry one fragment; a required trait method resolves
    /// through the second id, so planning just the first would understate the run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) planned_method_anchors: Option<Vec<String>>,
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
/// - [`crate::error::ErrorKind::Internal`] when JSON serialize/pretty-print fails
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
            AppError::of_with_source(
                ErrorDetail::Internal {
                    op: InternalOp::JsonPrettyPrint,
                },
                e,
            )
        })?;
        writeln!(stdout, "planned_params: {pretty}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
