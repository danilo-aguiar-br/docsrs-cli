//! `search-crates` handler: crates.io search with URL-derived parameter echo.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use super::OpCtx;
use crate::cli::SortKind;
use crate::config::{HOST_CRATES_IO, PROGRESS_HINT_DELAY_SECS};
use crate::crates_io;
use crate::domain::SearchQuery;
use crate::error::AppResult;
use crate::http::HttpClient;
use crate::meta_cmds;
use crate::output;
use crate::render::{
    self, apply_output_budget_search_crates, render_search_markdown, success_envelope,
};
use crate::shutdown::{ProgressGuard, duration_ms};

/// Search crates on crates.io (or dry-run the planned URL).
///
/// # Errors
///
/// - [`crate::error::ErrorKind::InvalidInput`](crate::error::ErrorKind::InvalidInput) for query/pagination validation
/// - [`crate::error::ErrorKind::Network`](crate::error::ErrorKind::Network) / [`crate::error::ErrorKind::Timeout`](crate::error::ErrorKind::Timeout) / [`crate::error::ErrorKind::RateLimited`](crate::error::ErrorKind::RateLimited) / [`crate::error::ErrorKind::Unavailable`](crate::error::ErrorKind::Unavailable) from HTTP
/// - [`crate::error::ErrorKind::Parse`](crate::error::ErrorKind::Parse) for unexpected response bodies
/// - [`crate::error::ErrorKind::Budget`](crate::error::ErrorKind::Budget) when body/output caps are exceeded
/// - [`crate::error::ErrorKind::Interrupted`](crate::error::ErrorKind::Interrupted) / [`crate::error::ErrorKind::Terminated`](crate::error::ErrorKind::Terminated) on cooperative cancel
pub(crate) async fn search_crates<Out: Write>(
    ctx: &OpCtx<'_>,
    stdout: &mut Out,
    query: &str,
    per_page: u32,
    sort: SortKind,
    page: u32,
    page_token: &Option<String>,
) -> AppResult<ExitCode> {
    // Full `meta.next_page` tokens already embed `q=…`; allow empty positional query then.
    let token_carries_query = page_token.as_deref().is_some_and(|t| {
        let qs = t.trim().trim_start_matches('?');
        qs.contains('=')
    });
    let query = SearchQuery::parse(query, token_carries_query)?;
    // Explicit CLI flags fail closed (GAP-006); page-token path still clamps via URL echo.
    let (per_page, page) = if page_token.is_some() {
        crates_io::clamp_search_pagination(per_page, page.max(1))
    } else {
        crates_io::validate_search_pagination(per_page, page)?
    };
    let url = if let Some(token) = page_token.as_deref() {
        crates_io::planned_url_with_page_token(
            &ctx.cfg.crates_io_origin,
            &query,
            per_page,
            sort,
            token,
        )?
    } else {
        crates_io::planned_url_on_host(&ctx.cfg.crates_io_origin, &query, per_page, sort, page)?
    };
    // Single source of truth: echo params from the effective URL (GAP-001).
    let fallback = crates_io::SearchEcho::from_cli(&query, page, per_page, sort);
    let echo = crates_io::echo_params_from_url(&url, &fallback);
    if ctx.dry_run {
        return meta_cmds::emit_dry_run(
            "search-crates",
            url.as_str(),
            meta_cmds::SearchCratesDryParams {
                q: &echo.query,
                per_page: echo.per_page,
                sort: &echo.sort,
                page: echo.page,
                page_token: page_token.as_deref(),
            },
            ctx.wants_json,
            ctx.start,
            stdout,
        );
    }
    let progress = ProgressGuard::start(
        ctx.cli.quiet,
        Duration::from_secs(PROGRESS_HINT_DELAY_SECS),
        ctx.locale.progress_fetching(HOST_CRATES_IO),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let data =
        crates_io::search_crates_at(&http, &url, &query, echo.per_page, sort, echo.page).await;
    progress.finish();
    let data = data?;
    if ctx.wants_json {
        let ms = duration_ms(ctx.start);
        let data = apply_output_budget_search_crates(
            data,
            ctx.cfg.max_output_bytes,
            ms,
            Some(url.as_str()),
        );
        output::write_json(
            stdout,
            &success_envelope("search-crates", &data, ms, Some(url.as_str())),
        )?;
    } else {
        let md = render_search_markdown(&data);
        let (out, _) = render::truncate_output(&md, ctx.cfg.max_output_bytes);
        write!(stdout, "{out}").map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
