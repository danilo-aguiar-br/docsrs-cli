//! `cache path|clear|stats` — inspect or clear the XDG HTTP disk cache.

use std::io::Write;
use std::process::ExitCode;

use serde::Serialize;
use tokio::time::Instant;

use crate::cache::DiskCache;
use crate::cli::CacheAction;
use crate::cli::destructive::CACHE_CLEAR;
use crate::config::Config;
use crate::error::{AppError, AppResult, ErrorDetail};
use crate::output::{self, write_json};
use crate::render::success_envelope;
use crate::shutdown::duration_ms;

/// Cache path inventory for `cache path` (agent-readable).
#[derive(Debug, Clone, Serialize)]
struct CachePathData {
    root: Option<String>,
    source: crate::config::PathSource,
    no_cache: bool,
}

/// `cache clear` outcome plus the provenance of the root it deleted.
///
/// `cache path` already reports `source`, so an operator could learn which layer
/// won for a read-only verb and could not learn it for the destructive one. The
/// same value is now carried where it matters most: on the envelope that says
/// something was deleted.
#[derive(Debug, Clone, Serialize)]
struct CacheClearData {
    #[serde(flatten)]
    result: crate::cache::CacheClearResult,
    target_source: crate::config::PathSource,
}

/// Inspect or clear the XDG HTTP disk cache (and report path).
///
/// # Errors
///
/// - [`crate::error::ErrorKind::Config`] when the cache directory cannot be resolved
/// - [`crate::error::ErrorKind::Internal`] / I/O kinds from cache clear/stats or stdout write
pub(crate) fn cache_cmd<Out: Write>(
    cfg: &Config,
    action: &CacheAction,
    wants_json: bool,
    start: Instant,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    if matches!(action, CacheAction::Path) {
        let data = CachePathData {
            root: cfg.cache_dir.as_ref().map(|p| p.display().to_string()),
            source: cfg.cache_path_source,
            no_cache: cfg.no_cache,
        };
        if wants_json {
            write_json(
                stdout,
                &success_envelope("cache-path", &data, duration_ms(start), None),
            )?;
        } else {
            writeln!(
                stdout,
                "cache_root={} source={} no_cache={}",
                data.root.as_deref().unwrap_or("<unresolved>"),
                data.source.as_str(),
                data.no_cache
            )
            .map_err(output::map_stdout_err)?;
        }
        return Ok(ExitCode::SUCCESS);
    }

    let root = cfg
        .cache_dir
        .clone()
        .ok_or_else(|| AppError::of(ErrorDetail::CacheDirUnresolved))?;
    let root_display = root.display().to_string();
    let cache = DiskCache::new(
        root,
        cfg.cache_ttl(),
        cfg.max_cache_bytes,
        cfg.allow_loopback,
    );
    match action {
        CacheAction::Path => unreachable!("handled above"),
        CacheAction::Clear { yes } => {
            // Explicit Target Designation: a verb with a side effect designates
            // its subject in argv and never inherits it. `--cache-dir` names the
            // root; `--yes` is the explicit waiver that accepts the XDG one.
            // Neither present means the caller never saw the path it was about
            // to empty, so nothing is deleted.
            if CACHE_CLEAR.must_refuse(*yes, cfg.cache_path_source) {
                return Err(AppError::of(CACHE_CLEAR.refuse(root_display)));
            }
            let data = CacheClearData {
                result: cache.clear()?,
                target_source: cfg.cache_path_source,
            };
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("cache-clear", &data, duration_ms(start), None),
                )?;
            } else {
                writeln!(
                    stdout,
                    "cache cleared: {} entries, {} bytes freed ({}) target_source={}",
                    data.result.removed_entries,
                    data.result.freed_bytes,
                    data.result.root,
                    data.target_source.as_str()
                )
                .map_err(output::map_stdout_err)?;
            }
        }
        CacheAction::Stats => {
            let stats = cache.stats();
            if wants_json {
                write_json(
                    stdout,
                    &success_envelope("cache-stats", &stats, duration_ms(start), None),
                )?;
            } else {
                let budget = if stats.max_bytes == 0 {
                    "unlimited".to_string()
                } else {
                    format!("{}B", stats.max_bytes)
                };
                writeln!(
                    stdout,
                    "cache root={} layout={} entries={} used={}B max={} ttl={}s parser={}",
                    stats.root,
                    stats.layout,
                    stats.entries,
                    stats.total_bytes,
                    budget,
                    stats.ttl_secs,
                    stats.parser_version
                )
                .map_err(output::map_stdout_err)?;
            }
        }
    }
    Ok(ExitCode::SUCCESS)
}
