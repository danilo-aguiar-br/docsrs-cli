//! Cross-process rate-limit stamp/lock path helpers.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::error::{AppError, AppResult, ErrorKind};

pub(super) fn safe_host_name(host: &str) -> String {
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "unknown".to_string()
    } else {
        safe
    }
}

/// Path for host rate-limit stamp: `{cache_dir}/rate-limit/{safe_host}.stamp`.
pub(super) fn rate_limit_stamp_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join("rate-limit")
        .join(format!("{}.stamp", safe_host_name(host)))
}

/// Path for host rate-limit exclusive lock: `{cache_dir}/rate-limit/{safe_host}.lock`.
pub(super) fn rate_limit_lock_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join("rate-limit")
        .join(format!("{}.lock", safe_host_name(host)))
}

pub(super) fn now_unix_ms() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
}

/// Remaining sleep from stamp file, if any.
pub(super) fn stamp_remaining(path: &Path, delay: Duration) -> Option<Duration> {
    let text = fs::read_to_string(path).ok()?;
    let last_ms: u128 = text.trim().parse().ok()?;
    let now = now_unix_ms()?;
    if now < last_ms {
        return Some(delay);
    }
    let elapsed = Duration::from_millis((now - last_ms) as u64);
    if elapsed < delay {
        Some(delay - elapsed)
    } else {
        None
    }
}

/// Write stamp under exclusive lock (temp + rename).
pub(super) fn write_stamp(path: &Path) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::with_source(ErrorKind::Network, "rate-limit stamp dir create failed", e)
        })?;
    }
    let now = now_unix_ms()
        .ok_or_else(|| AppError::new(ErrorKind::Internal, "system clock before UNIX epoch"))?;
    let tmp = path.with_extension("stamp.tmp");
    fs::write(&tmp, now.to_string()).map_err(|e| {
        AppError::with_source(ErrorKind::Network, "rate-limit stamp write failed", e)
    })?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::with_source(ErrorKind::Network, "rate-limit stamp rename failed", e)
    })?;
    crate::platform::restrict_private_file(path);
    Ok(())
}
