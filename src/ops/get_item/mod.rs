//! `get-item` handler: typed rustdoc item fetch with not-found recovery.
//!
//! This module owns the happy path: parse the coordinates, build the URL, issue a
//! single GET, emit. When docs.rs answers 404 it hands off to [`recover`], which
//! owns the catalog-backed recovery ladder and its ranked suggestions.

use std::io::Write;
use std::process::ExitCode;
use std::time::Duration;

use url::Url;

mod recover;

use recover::recover_not_found;

use super::OpCtx;
use crate::config::{Config, PROGRESS_HINT_DELAY_SECS, docs_origin_for_crate};
use crate::docs_rs::{self, GetItemData};
use crate::domain::{AllowedOrigin, CrateName, CrateRef, ItemPath, VersionArg};
use crate::error::{AppResult, ErrorKind};
use crate::http::HttpClient;
use crate::item_kind::{self, ItemKind};
use crate::meta_cmds;
use crate::output;
use crate::render::{apply_truncation_to_item, render_item_markdown, success_envelope};
use crate::shutdown::{ProgressGuard, duration_ms};

/// Catalog probe breadth when resolving a not-found item through `all.html`.
const CATALOG_PROBE_LIMIT: u32 = 1000;

/// Resolved coordinates of the requested item (no network handle).
///
/// Groups the values that every recovery step needs so helper signatures stay
/// under clippy's argument budget without collapsing distinct domain types.
struct ItemTarget<'a> {
    origin: &'a AllowedOrigin,
    crate_name: &'a CrateName,
    version: &'a VersionArg,
    kind: ItemKind,
    kind_echo: &'static str,
    item_path: &'a ItemPath,
}

/// An [`ItemTarget`] plus the handles the not-found recovery needs.
struct ItemRequest<'a> {
    target: ItemTarget<'a>,
    http: &'a HttpClient,
    cfg: &'a Config,
    suggest: bool,
}

/// Fetch documentation for a typed item (with optional `--suggest` recovery).
///
/// # Errors
///
/// - [`crate::error::ErrorKind::InvalidInput`] for name/type/path/version validation
/// - [`crate::error::ErrorKind::NotFound`] when the item page is missing after recovery
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
    warn_on_crate_prefix_mismatch(&crate_name, segs);
    let origin = docs_origin_for_crate(ctx.cfg, &crate_name);
    let url = docs_rs::get_item_url_on_origin(&origin, &crate_name, &version, kind, segs)?;
    let target = ItemTarget {
        origin: &origin,
        crate_name: &crate_name,
        version: &version,
        kind,
        kind_echo,
        item_path: &item_path,
    };
    if ctx.dry_run {
        return emit_dry_run(ctx, stdout, &url, &target, segs);
    }
    let progress = ProgressGuard::start(
        ctx.cli.quiet,
        Duration::from_secs(PROGRESS_HINT_DELAY_SECS),
        ctx.locale.progress_fetching(item_path.as_str()),
    );
    let http = HttpClient::new(ctx.cfg.clone(), ctx.cancel.clone())?;
    let fetched = docs_rs::fetch_item_on_origin_with_echo(
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
    let req = ItemRequest {
        target,
        http: &http,
        cfg: ctx.cfg,
        suggest,
    };
    let data = match fetched {
        Ok(d) => apply_truncation_to_item(d, ctx.cfg),
        Err(e) if e.kind() == ErrorKind::NotFound => recover_not_found(&req, segs, e).await?,
        Err(e) => return Err(e),
    };
    emit(ctx, stdout, &data)
}

/// Warn when the leading path segment names a different crate than `--crate`.
///
/// Advisory only: docs.rs still resolves the URL, so this never fails the run.
fn warn_on_crate_prefix_mismatch(crate_name: &CrateName, segs: &[String]) {
    let Some(first) = segs.first() else {
        return;
    };
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

/// Emit the planned URL without touching the network.
fn emit_dry_run<Out: Write>(
    ctx: &OpCtx<'_>,
    stdout: &mut Out,
    url: &Url,
    target: &ItemTarget<'_>,
    segs: &[String],
) -> AppResult<ExitCode> {
    let segs_detect = docs_rs::strip_crate_prefix_segments(target.crate_name, target.kind, segs);
    let assoc = docs_rs::associated_item_path(target.kind, &segs_detect);
    let planned_method_anchors = assoc
        .zip(segs_detect.last())
        .map(|(family, leaf)| family.anchor_ids(leaf));
    meta_cmds::emit_dry_run(
        "get-item",
        url.as_str(),
        meta_cmds::GetItemDryParams {
            crate_name: target.crate_name.as_str(),
            item_type: target.kind_echo,
            item_path: target.item_path.as_str(),
            version: target.version.as_str(),
            // Offline shape only: fragment / parent kind are not verified (X-007).
            validation: "url_shape_only",
            // Lead with the kind the live probe tries first, per anchor family.
            planned_parent_kind: assoc.and_then(|f| f.parent_kind_probe_names().next()),
            parent_kind_probe: assoc.map(|f| f.parent_kind_probe_names().collect()),
            planned_method_anchors,
        },
        ctx.wants_json,
        ctx.start,
        stdout,
    )
}

/// Write the fetched item as JSON envelope or rendered markdown.
fn emit<Out: Write>(ctx: &OpCtx<'_>, stdout: &mut Out, data: &GetItemData) -> AppResult<ExitCode> {
    if ctx.wants_json {
        output::write_json(
            stdout,
            &success_envelope(
                "get-item",
                data,
                duration_ms(ctx.start),
                Some(&data.source_url),
            ),
        )?;
    } else {
        write!(stdout, "{}", render_item_markdown(data)).map_err(output::map_stdout_err)?;
    }
    Ok(ExitCode::SUCCESS)
}
