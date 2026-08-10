//! Wall-clock timeout, signal awareness, and cancel racing for one-shot runs.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **Class:** signal/deadline coordination on the multi-thread one-shot runtime
//!   (see `src/main.rs` and [`crate::concurrency`]).
//! - **Fan-out bound:** fixed auxiliary tasks — at most one double-interrupt
//!   watcher and one [`ProgressGuard`] timer per command (abort-on-drop). Product
//!   CPU/HTTP admission lives in [`crate::concurrency::ConcurrencyBudget`].
//! - **Shared state:** `CancelFlag` and progress `done` use `Arc<Atomic*>` (SeqCst),
//!   never `Mutex` across `.await`.
//!
//! # Shutdown model (CLI one-shot)
//!
//! Product policy is **minimum + pipeline-critical** shutdown, not daemon coordination:
//! detect (signals / deadline) → signal (`CancelFlag`) → await (race + flush) → DIE.
//!
//! - Unix: SIGINT → interrupt (130); SIGTERM / SIGHUP → terminate (143)
//! - Windows: Ctrl+C → interrupt; Ctrl+Break / Ctrl+Close → terminate
//! - Second SIGINT / Ctrl+C within [`DOUBLE_INTERRUPT_FORCE_SECS`] force-exits 130 after flush
//! - stdout `BrokenPipe` is mapped by callers to exit 141 (Rust ignores SIGPIPE by default)

mod signals;

#[cfg(test)]
mod tests;

pub use signals::{
    DOUBLE_INTERRUPT_FORCE_SECS, spawn_double_interrupt_force_exit, wait_for_cancel,
};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::{Instant, timeout};

use crate::error::{AppError, AppResult, ErrorDetail};

/// Which OS signal cancelled the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelKind {
    /// SIGINT / Ctrl-C.
    Interrupt,
    /// SIGTERM, SIGHUP, Ctrl+Break, or Ctrl+Close (one-shot terminate).
    Terminate,
}

const FLAG_NONE: u8 = 0;
const FLAG_INTERRUPT: u8 = 1;
const FLAG_TERMINATE: u8 = 2;

/// Shared cancel flag checked by HTTP retries between attempts.
///
/// Stores the first observed [`CancelKind`] so cooperative checks can surface
/// exit 130 vs 143 correctly (first cancel wins).
///
/// # Interior mutability
///
/// Cross-task flag (`Arc<AtomicU8>`): signal waiter, wall-clock path, and HTTP
/// pollers share one flag. An atomic is the right primitive (not `Mutex<u8>` /
/// `Cell` — multi-task, single primitive, no compound invariant).
///
/// Ordering is [`Ordering::SeqCst`] on every load/store/CAS so first-writer-wins
/// is totally ordered against cooperative `check` without a separate fence story.
#[derive(Debug, Clone, Default)]
pub struct CancelFlag {
    state: Arc<AtomicU8>,
}

impl CancelFlag {
    /// Creates a fresh, non-cancelled flag.
    pub fn new() -> Self {
        Self {
            state: Arc::new(AtomicU8::new(FLAG_NONE)),
        }
    }

    /// Marks the flag as cancelled with [`CancelKind::Terminate`] (generic cancel).
    pub fn cancel(&self) {
        self.cancel_with(CancelKind::Terminate);
    }

    /// Marks the flag as cancelled with an explicit signal kind.
    ///
    /// First writer wins; subsequent calls are no-ops (idempotent).
    /// CAS uses `SeqCst` success/failure orderings (see type docs).
    pub fn cancel_with(&self, kind: CancelKind) {
        let v = match kind {
            CancelKind::Interrupt => FLAG_INTERRUPT,
            CancelKind::Terminate => FLAG_TERMINATE,
        };
        // SeqCst: publish first cancel kind with a total order vs loads in `check`.
        let _ = self
            .state
            .compare_exchange(FLAG_NONE, v, Ordering::SeqCst, Ordering::SeqCst);
    }

    /// Returns whether cancellation was requested.
    pub fn is_cancelled(&self) -> bool {
        // SeqCst: observe any prior `cancel_with` from another task.
        self.state.load(Ordering::SeqCst) != FLAG_NONE
    }

    /// Returns the stored kind when cancelled.
    pub fn kind(&self) -> Option<CancelKind> {
        // SeqCst: pair with `cancel_with` CAS (see type docs).
        match self.state.load(Ordering::SeqCst) {
            FLAG_INTERRUPT => Some(CancelKind::Interrupt),
            FLAG_TERMINATE => Some(CancelKind::Terminate),
            _ => None,
        }
    }

    /// Returns [`AppError::interrupted`] or [`AppError::terminated`] when cancelled.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Interrupted`] or [`crate::error::ErrorKind::Terminated`] when the flag is set.
    pub fn check(&self) -> AppResult<()> {
        match self.kind() {
            None => Ok(()),
            Some(CancelKind::Interrupt) => Err(AppError::interrupted()),
            Some(CancelKind::Terminate) => Err(AppError::terminated()),
        }
    }
}

/// Run a future with a wall-clock deadline.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Timeout`] when `deadline` elapses before `fut` completes.
/// Propagates any error returned by `fut` itself.
pub async fn with_wall_clock<T, F, Fut>(deadline: Duration, fut: F) -> AppResult<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    match timeout(deadline, fut()).await {
        Ok(inner) => inner,
        Err(_) => Err(AppError::of(ErrorDetail::WallClockTimeout {
            secs: deadline.as_secs(),
        })),
    }
}

/// Race an operation against cancel signals and an optional wall-clock deadline.
///
/// # Cancel safety (`select!`)
///
/// - Cancel arm: `wait_for_cancel` only awaits signal `recv` / `ctrl_c` (Tokio
///   signal futures are cancel-safe; losing a poll does not drop OS registration).
/// - Op arm: dropping `op` mid-flight aborts the in-flight HTTP future; partial
///   body buffers are dropped with the task (one-shot CLI — no shared mutable
///   request state beyond `CancelFlag`, which is written only on cancel/timeout).
/// - `biased` prefers the cancel arm so a ready signal is not starved by a ready `op`.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Timeout`] when `deadline` elapses first.
/// Returns [`crate::error::ErrorKind::Interrupted`] on SIGINT / Ctrl-C.
/// Returns [`crate::error::ErrorKind::Terminated`] on SIGTERM / SIGHUP / Ctrl+Break / Ctrl+Close.
/// Propagates any error returned by `op`.
pub async fn race_op_with_cancel_and_deadline<T, Fut>(
    deadline: Duration,
    cancel_flag: CancelFlag,
    op: Fut,
) -> AppResult<T>
where
    Fut: Future<Output = AppResult<T>>,
{
    tokio::pin!(op);
    let cancel = wait_for_cancel();
    tokio::pin!(cancel);

    let timed = timeout(deadline, async {
        tokio::select! {
            biased;
            kind = &mut cancel => {
                cancel_flag.cancel_with(kind);
                match kind {
                    CancelKind::Interrupt => Err(AppError::interrupted()),
                    CancelKind::Terminate => Err(AppError::terminated()),
                }
            }
            res = &mut op => res,
        }
    });

    match timed.await {
        Ok(inner) => inner,
        Err(_) => {
            cancel_flag.cancel_with(CancelKind::Terminate);
            Err(AppError::of(ErrorDetail::WallClockTimeout {
                secs: deadline.as_secs(),
            }))
        }
    }
}

/// Elapsed milliseconds since start.
pub fn duration_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}

/// Flush stdout and stderr best-effort (FINALIZE).
pub fn flush_stdio() {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
}

/// Progress reporter: after `delay`, emit one localized line to stderr if still running.
///
/// `done` is an [`AtomicBool`](std::sync::atomic::AtomicBool) shared with a background task (not `Mutex<bool>`).
/// Stores/loads use [`Ordering::SeqCst`] so `finish` is visible before the delayed
/// write without relying on abort alone.
///
/// # Drop / non-orphan tasks
///
/// Tokio's [`JoinHandle`](tokio::task::JoinHandle) **detaches** the task on drop (does not abort). This guard
/// implements [`Drop`] so early `?` returns and cancel races still abort the timer
/// (Rules Rust: no orphan critical tasks; fixed fan-out of at most one progress task).
pub struct ProgressGuard {
    done: Arc<std::sync::atomic::AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl ProgressGuard {
    /// Starts a delayed stderr progress line unless `quiet` is set.
    pub fn start(quiet: bool, delay: Duration, message: String) -> Self {
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        if quiet {
            return Self { done, handle: None };
        }
        let flag = Arc::clone(&done);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            // SeqCst: see `finish` / `Drop` store from the owner task.
            if !flag.load(Ordering::SeqCst) {
                use std::io::Write;
                let _ = writeln!(std::io::stderr(), "{message}");
                let _ = std::io::stderr().flush();
            }
        });
        Self {
            done,
            handle: Some(handle),
        }
    }

    /// Cancels the pending progress line and aborts the background task.
    pub fn finish(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        // SeqCst: publish "done" before abort so a racing timer load never prints.
        self.done.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle.take() {
            h.abort();
        }
    }
}

impl Drop for ProgressGuard {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Helper future that polls cancel between retries (used by tests).
pub struct CancelPoll<'a> {
    flag: &'a CancelFlag,
}

impl Future for CancelPoll<'_> {
    type Output = AppResult<()>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(self.flag.check())
    }
}
