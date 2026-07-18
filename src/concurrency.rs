//! Bounded concurrency budget for the multi-thread one-shot runtime.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **Class:** mixed — I/O-bound HTTPS on the Tokio multi-thread runtime plus
//!   CPU-bound HTML/JSON parse on the `spawn_blocking` pool.
//! - **Stages:** (1) network/cache GET on async workers → (2) body decode +
//!   scrape/markdown on blocking workers gated by this budget → (3) emit.
//! - **Bound:** every fan-out of CPU work acquires a permit from
//!   [`ConcurrencyBudget`]'s `Arc<Semaphore>`. Drop of the permit (RAII) returns
//!   capacity. HTTP remains one in-flight request per command today; the same
//!   budget is ready for multi-GET fan-out if a batch command is added.
//! - **Formula (auto):** `min(cpus, floor((free_ram_mib / 2) / RAM_PER_TASK_MIB))`
//!   clamped to `[1, MAX_AUTO_CONCURRENCY]`. Free RAM uses `MemAvailable` on
//!   Linux (`/proc/meminfo`); other platforms fall back to CPU count only.
//! - **RSS ground truth:** `RAM_PER_TASK_MIB` rounds up a single HTML parse of a
//!   body near `HARD_MAX_BODY_BYTES` (10 MiB body + DOM + markdown ≈ 2–3× peak).
//!   Re-measure with `/usr/bin/time -v docsrs-cli search-in-crate …` when the
//!   body cap or scraper stack changes materially.
//! - **Override:** CLI `--max-concurrency=N`, env `DOCSRS_CLI_MAX_CONCURRENCY`,
//!   or `max_concurrency` in XDG `config.toml`. `0` / omit = auto formula.

use std::fs;
use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::{AppError, AppResult, ErrorKind};

/// Estimated peak RSS (MiB) for one CPU-bound parse task.
///
/// Derived from body hard cap (10 MiB) plus scraper DOM and markdown conversion
/// headroom; rounded up with margin. Not a live measurement — revalidate with
/// `/usr/bin/time -v` when parse deps or body caps change.
pub const RAM_PER_TASK_MIB: u64 = 48;

/// Hard ceiling for auto-computed concurrency (CLI, not a server farm).
pub const MAX_AUTO_CONCURRENCY: usize = 32;

/// Hard ceiling for explicit `--max-concurrency` / config override.
pub const MAX_EXPLICIT_CONCURRENCY: usize = 256;

/// Shared admission gate for CPU-bound (and future multi-GET) work.
#[derive(Debug, Clone)]
pub struct ConcurrencyBudget {
    max: usize,
    semaphore: Arc<Semaphore>,
}

impl ConcurrencyBudget {
    /// Build a budget from a resolved permit count (`0` is invalid; use
    /// [`resolve_max_concurrency`] first).
    pub fn new(max: usize) -> Self {
        let max = max.max(1);
        Self {
            max,
            semaphore: Arc::new(Semaphore::new(max)),
        }
    }

    /// Resolve auto/override then build.
    pub fn from_configured(configured: u32) -> Self {
        Self::new(resolve_max_concurrency(configured))
    }

    /// Configured / auto-resolved maximum concurrent CPU tasks.
    pub fn max(&self) -> usize {
        self.max
    }

    /// Permits currently free (observability / doctor).
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Clone of the inner semaphore for `acquire_owned` at spawn sites.
    pub fn semaphore(&self) -> Arc<Semaphore> {
        Arc::clone(&self.semaphore)
    }

    /// Acquire one permit (RAII). Prefer [`Self::run_cpu_bound`] for spawn paths.
    pub async fn acquire_owned(&self) -> AppResult<OwnedSemaphorePermit> {
        self.semaphore
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| AppError::new(ErrorKind::Internal, "concurrency semaphore closed"))
    }

    /// Run CPU / blocking work on Tokio's blocking pool under this budget.
    ///
    /// Acquires a permit before `spawn_blocking`; the permit is held until the
    /// closure returns (dropped inside the blocking thread).
    ///
    /// # Errors
    ///
    /// Propagates closure errors. Maps worker panic to [`ErrorKind::Internal`]
    /// and abort/cancel to [`ErrorKind::Interrupted`].
    pub async fn run_cpu_bound<F, T>(&self, f: F) -> AppResult<T>
    where
        F: FnOnce() -> AppResult<T> + Send + 'static,
        T: Send + 'static,
    {
        let permit = self.acquire_owned().await?;
        let join = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            f()
        });
        match join.await {
            Ok(inner) => inner,
            Err(e) if e.is_cancelled() => Err(AppError::interrupted()),
            Err(e) if e.is_panic() => Err(AppError::with_source(
                ErrorKind::Internal,
                "CPU worker panicked during parse",
                e,
            )),
            Err(e) => Err(AppError::with_source(
                ErrorKind::Internal,
                "CPU worker join failed",
                e,
            )),
        }
    }
}

/// Resolve permit count from config override (`0` = auto).
///
/// Auto formula (documented at module root):
/// `min(cpus, max(1, (free_ram_mib / 2) / RAM_PER_TASK_MIB))` then clamp
/// to `[1, MAX_AUTO_CONCURRENCY]`.
pub fn resolve_max_concurrency(configured: u32) -> usize {
    if configured > 0 {
        return (configured as usize).clamp(1, MAX_EXPLICIT_CONCURRENCY);
    }
    auto_max_concurrency()
}

/// Auto permits from CPUs and free RAM (50% free-RAM safety margin).
pub fn auto_max_concurrency() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .max(1);
    let ram_based = match free_ram_mib() {
        Some(free) => {
            let usable = free / 2; // 50% safety margin on free RAM
            ((usable / RAM_PER_TASK_MIB).max(1) as usize).max(1)
        }
        None => cpus,
    };
    cpus.min(ram_based).clamp(1, MAX_AUTO_CONCURRENCY)
}

/// Worker thread hint for the multi-thread runtime (same as auto concurrency
/// floor on CPUs, independent of RAM clamp for the blocking pool).
pub fn runtime_worker_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .clamp(1, MAX_AUTO_CONCURRENCY)
}

/// Linux `MemAvailable` in MiB; `None` when unavailable.
fn free_ram_mib() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let text = fs::read_to_string("/proc/meminfo").ok()?;
        for line in text.lines() {
            if let Some(rest) = line.strip_prefix("MemAvailable:") {
                let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
                return Some(kb / 1024);
            }
        }
        None
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = fs::metadata;
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_override_respected() {
        assert_eq!(resolve_max_concurrency(4), 4);
        assert_eq!(resolve_max_concurrency(1), 1);
    }

    #[test]
    fn auto_is_at_least_one() {
        assert!(auto_max_concurrency() >= 1);
        assert!(auto_max_concurrency() <= MAX_AUTO_CONCURRENCY);
    }

    #[test]
    fn budget_reports_max_and_available() {
        let b = ConcurrencyBudget::new(3);
        assert_eq!(b.max(), 3);
        assert_eq!(b.available_permits(), 3);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_cpu_bound_returns_value_and_releases_permit() {
        let b = ConcurrencyBudget::new(2);
        let out = b.run_cpu_bound(|| Ok::<_, AppError>(41 + 1)).await.unwrap();
        assert_eq!(out, 42);
        assert_eq!(b.available_permits(), 2);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_cpu_bound_propagates_closure_error() {
        let b = ConcurrencyBudget::new(1);
        let err = b
            .run_cpu_bound(|| -> AppResult<()> {
                Err(AppError::new(ErrorKind::Parse, "synthetic parse failure"))
            })
            .await
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Parse);
        assert_eq!(b.available_permits(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn peak_concurrency_never_exceeds_max() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let b = ConcurrencyBudget::new(2);
        let peak = Arc::new(AtomicUsize::new(0));
        let current = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();
        for _ in 0..8 {
            let b = b.clone();
            let peak = Arc::clone(&peak);
            let current = Arc::clone(&current);
            handles.push(tokio::spawn(async move {
                b.run_cpu_bound(move || {
                    let now = current.fetch_add(1, Ordering::SeqCst) + 1;
                    peak.fetch_max(now, Ordering::SeqCst);
                    std::thread::sleep(std::time::Duration::from_millis(15));
                    current.fetch_sub(1, Ordering::SeqCst);
                    Ok::<_, AppError>(())
                })
                .await
            }));
        }
        for h in handles {
            h.await.unwrap().unwrap();
        }
        assert!(
            peak.load(Ordering::SeqCst) <= 2,
            "peak {} exceeded budget 2",
            peak.load(Ordering::SeqCst)
        );
        assert_eq!(b.available_permits(), 2);
    }
}
