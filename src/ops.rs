//! Network-facing command handlers (crates.io / docs.rs).
//!
//! One-shot: each handler performs bounded HTTP then returns. CPU HTML work stays
//! inside `docs_rs` / `spawn_blocking`. No process-global locks across `.await`.
//!
//! Shared handler context is [`OpCtx`] (TYPE-L-011) so call sites stay under
//! clippy's argument budget without discarding domain types.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use tokio::time::Instant;

use crate::cli::{Cli, SortKind};
use crate::config::{Config, PROGRESS_HINT_DELAY_SECS, docs_origin_for_crate};
use crate::domain::{CrateRef, ItemPath, SearchQuery};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::HttpClient;
use crate::i18n::Locale;
use crate::item_kind::{self, ItemKind};
use crate::meta_cmds;
use crate::output;
use crate::render::{
    self, apply_output_budget_search_crates, apply_output_budget_search_in_crate,
    apply_truncation_to_item, apply_truncation_to_readme, render_item_markdown,
    render_readme_markdown, render_search_in_crate_markdown, render_search_markdown,
    success_envelope,
};
use crate::shutdown::{CancelFlag, ProgressGuard, duration_ms};
use crate::{crates_io, docs_rs, suggest};

/// Shared one-shot handler context (cli/cfg/locale/flags/cancel).
///
/// Domain command args stay as separate parameters so signatures stay explicit.
#[derive(Clone)]
pub(crate) struct OpCtx<'a> {
    pub cli: &'a Cli,
    pub cfg: &'a Config,
    pub locale: Locale,
    pub dry_run: bool,
    pub wants_json: bool,
    pub start: Instant,
    pub cancel: CancelFlag,
}

/// Search crates on crates.io (or dry-run the planned URL).
///
/// # Errors
///
/// - [`ErrorKind::InvalidInput`] for query/pagination validation
/// - [`ErrorKind::Network`] / [`ErrorKind::Timeout`] / [`ErrorKind::RateLimited`] / [`ErrorKind::Unavailable`] from HTTP
/// - [`ErrorKind::Parse`] for unexpected response bodies
/// - [`ErrorKind::Budget`] when body/output caps are exceeded
/// - [`ErrorKind::Interrupted`] / [`ErrorKind::Terminated`] on cooperative cancel
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
        ctx.locale.progress_fetching("crates.io"),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let data =
        crates_io::search_crates_at(&http, &url, &query, echo.per_page, sort, echo.page).await;
    progress.finish();
    let data = data?;
    if ctx.wants_json {
        let ms = duration_ms(ctx.start);
        let data =
            apply_output_budget_search_crates(data, ctx.cfg.max_output_bytes, ms, Some(url.as_str()));
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

/// Fetch crate overview docblock from docs.rs (not git README).
///
/// # Errors
///
/// - [`ErrorKind::InvalidInput`] for crate/version validation
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

/// Fetch documentation for a typed item (with optional `--suggest` recovery).
///
/// # Errors
///
/// - [`ErrorKind::InvalidInput`] for name/type/path/version validation
/// - [`ErrorKind::NotFound`] when the item page is missing after recovery
/// - Network/timeout/rate-limit/unavailable/parse/budget as for HTTP GETs
/// - Cancel kinds on cooperative interrupt
pub(crate) async fn get_item<Out: Write>(
    ctx: &OpCtx<'_>,
    stdout: &mut Out,
    crate_name: &str,
    item_type: &str,
    item_path: &str,
    crate_version: &Option<String>,
    suggest: bool,
) -> AppResult<ExitCode> {
    let (crate_name, version) =
        CrateRef::parse(crate_name)?.into_name_and_version(crate_version.as_deref())?;
    let (kind, kind_echo) = ItemKind::parse_with_echo(item_type)?;
    let item_path = ItemPath::parse(item_path)?;
    let segs = item_path.segments();
    if let Some(first) = segs.first() {
        let rustc = item_kind::rustc_crate_name(crate_name.as_str());
        if first.as_str() != crate_name.as_str()
            && first.as_str() != rustc.as_str()
            && (first.contains('-') || first.contains('_'))
        {
            tracing::warn!(
                path_crate = %first,
                expected = %crate_name,
                "item_path crate prefix differs from crate_name"
            );
        }
    }
    let origin = docs_origin_for_crate(ctx.cfg, &crate_name);
    let url = docs_rs::get_item_url_on_origin(&origin, &crate_name, &version, kind, segs)?;
    if ctx.dry_run {
        let segs_detect = docs_rs::strip_crate_prefix_segments(&crate_name, kind, segs);
        let is_method = docs_rs::is_method_path(kind, &segs_detect);
        return meta_cmds::emit_dry_run(
            "get-item",
            url.as_str(),
            meta_cmds::GetItemDryParams {
                crate_name: crate_name.as_str(),
                item_type: kind_echo,
                item_path: item_path.as_str(),
                version: version.as_str(),
                // Offline shape only: fragment / parent kind are not verified (X-007).
                validation: "url_shape_only",
                planned_parent_kind: if is_method { Some("struct") } else { None },
                parent_kind_probe: if is_method {
                    Some(docs_rs::METHOD_PARENT_KIND_NAMES)
                } else {
                    None
                },
            },
            ctx.wants_json,
            ctx.start,
            stdout,
        );
    }
    let progress = ProgressGuard::start(
        ctx.cli.quiet,
        Duration::from_secs(PROGRESS_HINT_DELAY_SECS),
        ctx.locale.progress_fetching(item_path.as_str()),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let data = docs_rs::fetch_item_on_origin_with_echo(
        &http,
        &origin,
        &crate_name,
        &version,
        kind,
        kind_echo,
        segs,
    )
    .await;
    progress.finish();
    let data = match data {
        Ok(d) => apply_truncation_to_item(d, ctx.cfg),
        Err(e) if e.kind() == ErrorKind::NotFound => {
            // Catalog-backed resolution: free-fn reexports OR short Type::method paths.
            let segs_detect = docs_rs::strip_crate_prefix_segments(&crate_name, kind, segs);
            let all_url = docs_rs::all_html_url_on_origin(&origin, &crate_name, &version)?;

            // R1: associated method short path (Runtime::new → runtime::Runtime::new).
            if docs_rs::is_method_path(kind, &segs_detect) {
                let parent_leaf = segs_detect[segs_detect.len() - 2].as_str();
                let method_leaf = segs_detect[segs_detect.len() - 1].as_str();
                let method_anchor_miss = e.message().starts_with("method anchor ");
                let parent_q = SearchQuery::parse(parent_leaf, false)?;
                let catalog = docs_rs::search_in_crate_at(
                    &http,
                    &crate_name,
                    &version,
                    &parent_q,
                    None,
                    1000,
                    crate::domain::MatchMode::Prefix,
                    &all_url,
                )
                .await;
                match catalog {
                    Ok(cat) => {
                        if let Some(parent_path) =
                            docs_rs::pick_unique_type_path(parent_leaf, &cat.hits)
                        {
                            let resolved_segs =
                                docs_rs::method_segments_from_parent(&parent_path, method_leaf);
                            let resolved_label = format!("{parent_path}::{method_leaf}");
                            match docs_rs::fetch_item_on_origin_with_echo(
                                &http,
                                &origin,
                                &crate_name,
                                &version,
                                kind,
                                kind_echo,
                                &resolved_segs,
                            )
                            .await
                            {
                                Ok(mut resolved_data) => {
                                    resolved_data.item_path = item_path.as_str().to_string();
                                    resolved_data.item_name = method_leaf.to_string();
                                    resolved_data.resolved_item_path = Some(resolved_label);
                                    apply_truncation_to_item(resolved_data, ctx.cfg)
                                }
                                // Parent path ok but method leaf missing → suggest methods (X-002).
                                Err(re)
                                    if re.kind() == ErrorKind::NotFound
                                        && (method_anchor_miss
                                            || re.message().starts_with("method anchor ")) =>
                                {
                                    let names = method_name_suggestions(
                                        &http,
                                        &origin,
                                        &crate_name,
                                        &version,
                                        kind,
                                        &resolved_segs,
                                        method_leaf,
                                        &parent_path,
                                    )
                                    .await;
                                    if suggest || !names.is_empty() {
                                        return Err(AppError::new(
                                            ErrorKind::NotFound,
                                            format!(
                                                "{}; suggestions: {}",
                                                re.message(),
                                                if names.is_empty() {
                                                    "(none)".into()
                                                } else {
                                                    names.join(", ")
                                                }
                                            ),
                                        ));
                                    }
                                    return Err(re);
                                }
                                Err(re) => return Err(re),
                            }
                        } else {
                            let names = suggest::rank_suggestions(parent_leaf, &cat.hits, 5);
                            if suggest || !names.is_empty() {
                                return Err(AppError::new(
                                    ErrorKind::NotFound,
                                    format!(
                                        "{}; suggestions: {}",
                                        e.message(),
                                        if names.is_empty() {
                                            "(none)".into()
                                        } else {
                                            names.join(", ")
                                        }
                                    ),
                                ));
                            }
                            return Err(e);
                        }
                    }
                    Err(_) if method_anchor_miss => {
                        // Catalog unavailable but parent page existed: still try method-leaf suggest.
                        let parent_path = segs_detect[..segs_detect.len() - 1].join("::");
                        let names = method_name_suggestions(
                            &http,
                            &origin,
                            &crate_name,
                            &version,
                            kind,
                            segs,
                            method_leaf,
                            &parent_path,
                        )
                        .await;
                        if suggest || !names.is_empty() {
                            return Err(AppError::new(
                                ErrorKind::NotFound,
                                format!(
                                    "{}; suggestions: {}",
                                    e.message(),
                                    if names.is_empty() {
                                        "(none)".into()
                                    } else {
                                        names.join(", ")
                                    }
                                ),
                            ));
                        }
                        return Err(e);
                    }
                    Err(_) => return Err(e),
                }
            } else {
                // Free item / reexport resolution via all.html.
                let leaf = segs
                    .last()
                    .map(|s| s.as_str())
                    .unwrap_or(item_path.as_str());
                let leaf_q = SearchQuery::parse(leaf, false)?;
                let catalog = docs_rs::search_in_crate_at(
                    &http,
                    &crate_name,
                    &version,
                    &leaf_q,
                    Some(kind),
                    1000,
                    crate::domain::MatchMode::Prefix,
                    &all_url,
                )
                .await;
                match catalog {
                    Ok(cat) => {
                        if let Some(resolved) =
                            suggest::pick_unique_reexport_path(leaf, kind, &cat.hits)
                        {
                            let resolved_segs: Vec<String> = resolved
                                .split("::")
                                .filter(|s| !s.is_empty())
                                .map(str::to_string)
                                .collect();
                            let mut resolved_data = docs_rs::fetch_item_on_origin_with_echo(
                                &http,
                                &origin,
                                &crate_name,
                                &version,
                                kind,
                                kind_echo,
                                &resolved_segs,
                            )
                            .await?;
                            resolved_data.item_path = item_path.as_str().to_string();
                            resolved_data.item_name = leaf.to_string();
                            resolved_data.resolved_item_path = Some(resolved);
                            apply_truncation_to_item(resolved_data, ctx.cfg)
                        } else {
                            let names = suggest::rank_suggestions(leaf, &cat.hits, 5);
                            if suggest || !names.is_empty() {
                                return Err(AppError::new(
                                    ErrorKind::NotFound,
                                    format!(
                                        "{}; suggestions: {}",
                                        e.message(),
                                        if names.is_empty() {
                                            "(none)".into()
                                        } else {
                                            names.join(", ")
                                        }
                                    ),
                                ));
                            }
                            return Err(e);
                        }
                    }
                    Err(_) => return Err(e),
                }
            }
        }
        Err(e) => return Err(e),
    };
    if ctx.wants_json {
        output::write_json(
            stdout,
            &success_envelope(
                "get-item",
                &data,
                duration_ms(ctx.start),
                Some(&data.source_url),
            ),
        )?;
    } else {
        write!(stdout, "{}", render_item_markdown(&data)).map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}

/// Fetch parent type HTML (struct→union probe) and rank nearby method names.
///
/// One extra GET on the method-miss path only (one-shot recovery). Cancel is
/// honored via [`HttpClient`]. Failures yield an empty suggestion list.
///
/// Important: parent pages are fetched as **type** kinds (`struct`/`enum`/…),
/// never as `method`/`fn` paths — otherwise `is_method_path` misreads
/// `runtime::Runtime` as a free-fn URL and suggestions stay empty (X-002).
#[allow(clippy::too_many_arguments)]
async fn method_name_suggestions(
    http: &HttpClient,
    origin: &crate::domain::AllowedOrigin,
    crate_name: &crate::domain::CrateName,
    version: &crate::domain::VersionArg,
    _kind: ItemKind,
    method_segs: &[String],
    method_leaf: &str,
    parent_path: &str,
) -> Vec<String> {
    let parent_segs: Vec<String> = if method_segs.len() >= 2 {
        method_segs[..method_segs.len() - 1].to_vec()
    } else {
        return Vec::new();
    };
    // Same parent-kind order as live method probe (struct first).
    const PARENT_KINDS: &[ItemKind] = &[
        ItemKind::Struct,
        ItemKind::Enum,
        ItemKind::Trait,
        ItemKind::Type,
        ItemKind::Union,
    ];
    for parent_kind in PARENT_KINDS {
        let Ok(url) = docs_rs::get_item_url_on_origin(
            origin,
            crate_name,
            version,
            *parent_kind,
            &parent_segs,
        ) else {
            continue;
        };
        let mut fetch_url = url;
        fetch_url.set_fragment(None);
        let Ok(resp) = http.get_html(&fetch_url).await else {
            continue;
        };
        if resp.status.as_u16() == 404 {
            continue;
        }
        if !resp.status.is_success() {
            continue;
        }
        let Ok(text) = crate::http::decode_utf8(&resp.body) else {
            continue;
        };
        let names = docs_rs::list_method_anchor_names_from_html(&text);
        if names.is_empty() {
            continue;
        }
        return suggest::rank_method_names(method_leaf, parent_path, &names, 5);
    }
    Vec::new()
}

/// Search symbols in a crate `all.html` index.
///
/// # Errors
///
/// - [`ErrorKind::InvalidInput`] for name/query/match/type validation
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
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "search-in-crate --item-type module is not supported (all.html has no module index); use get-item <crate> module <path>",
                ));
            }
            Some(k)
        }
        _ => None,
    };
    let match_mode = crate::domain::MatchMode::parse(r#match)?;
    let origin = docs_origin_for_crate(ctx.cfg, &crate_name);
    let url = docs_rs::all_html_url_on_origin(&origin, &crate_name, &version)?;
    let limit = limit.min(crate::config::MAX_SEARCH_IN_CRATE_LIMIT);
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
        let data =
            apply_output_budget_search_in_crate(data, ctx.cfg.max_output_bytes, ms, Some(src.as_str()));
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
