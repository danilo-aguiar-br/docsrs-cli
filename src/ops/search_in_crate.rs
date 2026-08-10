//! `search-in-crate` handler: symbol lookup over a crate `all.html` index.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use super::OpCtx;
use crate::config::{MAX_SEARCH_IN_CRATE_LIMIT, PROGRESS_HINT_DELAY_SECS, docs_origin_for_crate};
use crate::docs_rs;
use crate::domain::{CrateRef, MatchMode, SearchQuery};
use crate::error::{AppError, AppResult, ErrorDetail};
use crate::http::HttpClient;
use crate::item_kind::ItemKind;
use crate::meta_cmds;
use crate::output;
use crate::render::{
    self, apply_output_budget_search_in_crate, render_search_in_crate_markdown, success_envelope,
};
use crate::shutdown::{ProgressGuard, duration_ms};

/// Search symbols in a crate `all.html` index.
///
/// # Errors
///
/// - [`crate::error::ErrorKind::InvalidInput`] for name/query/match/type validation
/// - Network/timeout/rate-limit/unavailable/not-found/parse/budget as for HTTP GETs
/// - Cancel kinds on cooperative interrupt
#[allow(clippy::too_many_arguments)] // clap frontier args after OpCtx (item filter/limit/match)
pub(crate) async fn search_in_crate<Out: Write>(
    ctx: &OpCtx<'_>,
    stdout: &mut Out,
    crate_name: &str,
    query: &str,
    crate_version: &Option<String>,
    item_type: &Option<String>,
    limit: u32,
    r#match: &str,
) -> AppResult<ExitCode> {
    let (crate_name, version) =
        CrateRef::parse(crate_name)?.into_name_and_version(crate_version.as_deref())?;
    let query = SearchQuery::parse(query, true)?;
    let kind_filter = match item_type {
        Some(t) if !t.is_empty() => {
            let k = ItemKind::parse(t)?;
            if k == ItemKind::Module {
                return Err(AppError::of(ErrorDetail::ModuleFilterUnsupported));
            }
            Some(k)
        }
        _ => None,
    };
    let match_mode = MatchMode::parse(r#match)?;
    let origin = docs_origin_for_crate(ctx.cfg, &crate_name);
    let url = docs_rs::all_html_url_on_origin(&origin, &crate_name, &version)?;
    let limit = limit.min(MAX_SEARCH_IN_CRATE_LIMIT);
    if ctx.dry_run {
        return meta_cmds::emit_dry_run(
            "search-in-crate",
            url.as_str(),
            meta_cmds::SearchInCrateDryParams {
                crate_name: crate_name.as_str(),
                query: query.as_str(),
                version: version.as_str(),
                item_type: kind_filter.map(|k| k.as_str()),
                match_mode: Some(match_mode.as_str()),
                limit,
            },
            ctx.wants_json,
            ctx.start,
            stdout,
        );
    }
    let progress = ProgressGuard::start(
        ctx.cli.quiet,
        Duration::from_secs(PROGRESS_HINT_DELAY_SECS),
        ctx.locale
            .progress_fetching(&format!("{} all.html", crate_name.as_str())),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let data = docs_rs::search_in_crate_on_origin(
        &http,
        &origin,
        &crate_name,
        &version,
        &query,
        kind_filter,
        limit,
        match_mode,
    )
    .await;
    progress.finish();
    let data = data?;
    if ctx.wants_json {
        let ms = duration_ms(ctx.start);
        let src = data.source_url.clone();
        let data = apply_output_budget_search_in_crate(
            data,
            ctx.cfg.max_output_bytes,
            ms,
            Some(src.as_str()),
        );
        output::write_json(
            stdout,
            &success_envelope("search-in-crate", &data, ms, Some(src.as_str())),
        )?;
    } else {
        let md = render_search_in_crate_markdown(&data);
        let (out, _) = render::truncate_output(&md, ctx.cfg.max_output_bytes);
        write!(stdout, "{out}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
