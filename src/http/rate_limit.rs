//! Per-host politeness throttle: in-process clock plus cross-process lock+stamp.
//!
//! Policy and its stamp/lock I/O live together so the request loop in `client`
//! only calls [`HttpClient::rate_limit`] and never open-codes the throttle.

use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use fs4::FileExt as Fs4FileExt;
use fs4::TryLockError as Fs4TryLockError;
use reqwest::Url;
use tracing::warn;

use crate::config::CANCEL_POLL_INTERVAL_MS;
use crate::error::{AppError, AppResult, ErrorDetail, ErrorKind, InternalOp, IoOp, io_at};
use crate::retry::politeness_delay;

use super::client::HttpClient;

impl HttpClient {
    /// Sleep in cancel-poll slices so SIGINT/SIGTERM never waits out a full delay.
    ///
    /// # Errors
    ///
    /// Returns the cancel error when the process was interrupted or terminated.
    pub(super) async fn sleep_cancelable(&self, wait: Duration) -> AppResult<()> {
        let step = Duration::from_millis(CANCEL_POLL_INTERVAL_MS);
        let mut remaining = wait;
        while !remaining.is_zero() {
            self.cancel.check()?;
            let slice = remaining.min(step);
            tokio::time::sleep(slice).await;
            remaining = remaining.saturating_sub(slice);
        }
        self.cancel.check()
    }

    /// Enforce per-host delay: in-process clock + exclusive cross-process lock+stamp.
    ///
    /// Floor is `rate_limit_delay_ms`; each wait uses [`politeness_delay`] so the
    /// effective interval is never fixed (additive jitter up to +20%).
    ///
    /// In-process map uses a short [`std::sync::Mutex`] hold (no sleep under the
    /// lock). Cross-process flock is released on drop before return (never held
    /// across unrelated work).
    ///
    /// # Errors
    ///
    /// Returns the cancel error when interrupted; lock/stamp I/O failures degrade
    /// to the in-process clock instead of failing the request.
    pub(super) async fn rate_limit(&self, url: &Url) -> AppResult<()> {
        // Lookup borrows host; allocate only when inserting/updating the clock map.
        let host = url.host_str().unwrap_or("");
        let delay = politeness_delay(self.cfg.rate_limit_delay());
        if delay.is_zero() {
            return Ok(());
        }

        // Prefer exclusive cross-process section when cache_dir is available.
        if let Some(dir) = self.cfg.cache_dir.as_ref() {
            match self.rate_limit_cross_process(dir, host, delay).await {
                Ok(()) => {
                    self.touch_host_clock(host);
                    return Ok(());
                }
                Err(e)
                    if e.kind() == ErrorKind::Terminated || e.kind() == ErrorKind::Interrupted =>
                {
                    return Err(e);
                }
                Err(e) => {
                    // FS without flock or I/O failure: fall back to in-process only.
                    warn!(error = %e, host = %host, "cross-process rate-limit lock failed; using in-process only");
                }
            }
        }

        let in_proc = {
            let map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
            map.get(host).and_then(|prev| {
                let elapsed = prev.elapsed();
                if elapsed < delay {
                    Some(delay - elapsed)
                } else {
                    None
                }
            })
        };
        if let Some(d) = in_proc {
            self.sleep_cancelable(d).await?;
        }
        self.touch_host_clock(host);
        Ok(())
    }

    fn touch_host_clock(&self, host: &str) {
        let mut map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(host.to_string(), Instant::now());
    }

    /// Exclusive lock on `{cache_dir}/rate-limit/{host}.lock`, then stamp throttle.
    async fn rate_limit_cross_process(
        &self,
        cache_dir: &Path,
        host: &str,
        delay: Duration,
    ) -> AppResult<()> {
        let lock_path = rate_limit_lock_path(cache_dir, host);
        let stamp_path = rate_limit_stamp_path(cache_dir, host);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(io_at(IoOp::CreateDir, parent))?;
            crate::platform::restrict_private_dir(parent);
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(io_at(IoOp::OpenLock, &lock_path))?;
        crate::platform::restrict_private_file(&lock_path);

        // Cancel-aware exclusive acquisition (fs4 try_lock + short sleep).
        // Fully-qualified calls avoid collision with std::fs::File::try_lock (Rust 1.89+).
        loop {
            self.cancel.check()?;
            match Fs4FileExt::try_lock(&file) {
                Ok(()) => break,
                Err(Fs4TryLockError::WouldBlock) => {
                    self.sleep_cancelable(Duration::from_millis(CANCEL_POLL_INTERVAL_MS))
                        .await?;
                }
                Err(Fs4TryLockError::Error(e)) => {
                    return Err(io_at(IoOp::Lock, &lock_path)(e));
                }
            }
        }

        // Guard unlock on all exit paths.
        struct UnlockOnDrop<'a>(&'a File);
        impl Drop for UnlockOnDrop<'_> {
            fn drop(&mut self) {
                let _ = Fs4FileExt::unlock(self.0);
            }
        }
        let _guard = UnlockOnDrop(&file);

        let remaining = stamp_remaining(&stamp_path, delay);
        if let Some(d) = remaining {
            self.sleep_cancelable(d).await?;
        }
        write_stamp(&stamp_path)?;
        Ok(())
    }
}

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

/// Cache subdirectory holding per-host rate-limit stamps and locks.
///
/// `doctor` reports on this directory and the limiter creates it; a literal in
/// each would let the health check inspect a path the limiter no longer uses.
pub const RATE_LIMIT_DIR_NAME: &str = "rate-limit";

/// Path for host rate-limit stamp: `{cache_dir}/rate-limit/{safe_host}.stamp`.
pub(super) fn rate_limit_stamp_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join(RATE_LIMIT_DIR_NAME)
        .join(format!("{}.stamp", safe_host_name(host)))
}

/// Path for host rate-limit exclusive lock: `{cache_dir}/rate-limit/{safe_host}.lock`.
pub(super) fn rate_limit_lock_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join(RATE_LIMIT_DIR_NAME)
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
        fs::create_dir_all(parent).map_err(io_at(IoOp::CreateDir, parent))?;
    }
    let now = now_unix_ms().ok_or_else(|| {
        AppError::of(ErrorDetail::Internal {
            op: InternalOp::ClockBeforeEpoch,
        })
    })?;
    let tmp = path.with_extension("stamp.tmp");
    fs::write(&tmp, now.to_string()).map_err(io_at(IoOp::Write, &tmp))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        io_at(IoOp::Rename, path)(e)
    })?;
    crate::platform::restrict_private_file(path);
    Ok(())
}
