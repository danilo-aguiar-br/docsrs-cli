//! `readme` handler: crate overview docblock from docs.rs.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use super::OpCtx;
use crate::config::{PROGRESS_HINT_DELAY_SECS, docs_origin_for_crate};
use crate::docs_rs;
use crate::domain::CrateRef;
use crate::error::AppResult;
use crate::http::HttpClient;
use crate::meta_cmds;
use crate::output;
use crate::render::{apply_truncation_to_readme, render_readme_markdown, success_envelope};
use crate::shutdown::{ProgressGuard, duration_ms};

/// Fetch crate overview docblock from docs.rs (not git README).
///
/// # Errors
///
/// - [`crate::error::ErrorKind::InvalidInput`](crate::error::ErrorKind::InvalidInput) for crate/version validation
/// - Network/timeout/rate-limit/unavailable/not-found/parse/budget as for HTTP GETs
/// - Cancel kinds on cooperative interrupt
pub(crate) async fn readme<Out: Write>(
    ctx: &OpCtx<'_>,
    stdout: &mut Out,
    crate_name: &str,
    crate_version: &Option<String>,
) -> AppResult<ExitCode> {
    let (crate_name, version) =
        CrateRef::parse(crate_name)?.into_name_and_version(crate_version.as_deref())?;
    let origin = docs_origin_for_crate(ctx.cfg, &crate_name);
    let url = docs_rs::readme_url_on_origin(&origin, &crate_name, &version)?;
    if ctx.dry_run {
        return meta_cmds::emit_dry_run(
            "readme",
            url.as_str(),
            meta_cmds::ReadmeDryParams {
                crate_name: crate_name.as_str(),
                version: version.as_str(),
            },
            ctx.wants_json,
            ctx.start,
            stdout,
        );
    }
    let progress = ProgressGuard::start(
        ctx.cli.quiet,
        Duration::from_secs(PROGRESS_HINT_DELAY_SECS),
        ctx.locale.progress_fetching(crate_name.as_str()),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let data = docs_rs::fetch_readme_on_origin(&http, &origin, &crate_name, &version).await;
    progress.finish();
    let data = apply_truncation_to_readme(data?, ctx.cfg);
    if ctx.wants_json {
        output::write_json(
            stdout,
            &success_envelope(
                "readme",
                &data,
                duration_ms(ctx.start),
                Some(&data.source_url),
            ),
        )?;
    } else {
        write!(stdout, "{}", render_readme_markdown(&data)).map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
