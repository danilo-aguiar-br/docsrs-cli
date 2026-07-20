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

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::{Instant, timeout};

use crate::error::{AppError, AppResult, EXIT_INTERRUPTED, ErrorKind};

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
    /// Returns [`ErrorKind::Interrupted`] or [`ErrorKind::Terminated`] when the flag is set.
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
/// Returns [`ErrorKind::Timeout`] when `deadline` elapses before `fut` completes.
/// Propagates any error returned by `fut` itself.
pub async fn with_wall_clock<T, F, Fut>(deadline: Duration, fut: F) -> AppResult<T>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = AppResult<T>>,
{
    match timeout(deadline, fut()).await {
        Ok(inner) => inner,
        Err(_) => Err(AppError::new(
            ErrorKind::Timeout,
            format!("wall-clock timeout after {}s", deadline.as_secs()),
        )),
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
/// Returns [`ErrorKind::Timeout`] when `deadline` elapses first.
/// Returns [`ErrorKind::Interrupted`] on SIGINT / Ctrl-C.
/// Returns [`ErrorKind::Terminated`] on SIGTERM / SIGHUP / Ctrl+Break / Ctrl+Close.
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
            Err(AppError::new(
                ErrorKind::Timeout,
                format!("wall-clock timeout after {}s", deadline.as_secs()),
            ))
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

/// Future that completes when a cancel signal arrives.
///
/// # Platform policy (one-shot)
///
/// - Unix: SIGINT → [`CancelKind::Interrupt`]; SIGTERM / SIGHUP → [`CancelKind::Terminate`]
/// - Windows: Ctrl+C → Interrupt; Ctrl+Break / Ctrl+Close → Terminate
/// - SIGHUP is terminate (no config reload on one-shot)
pub async fn wait_for_cancel() -> CancelKind {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut sigint = match signal(SignalKind::interrupt()) {
            Ok(s) => s,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return CancelKind::Interrupt;
            }
        };
        let mut sigterm = match signal(SignalKind::terminate()) {
            Ok(s) => s,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return CancelKind::Interrupt;
            }
        };
        let mut sighup = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(_) => {
                // Hangup unavailable: fall through without a third arm.
                tokio::select! {
                    _ = sigint.recv() => return CancelKind::Interrupt,
                    _ = sigterm.recv() => return CancelKind::Terminate,
                }
            }
        };
        tokio::select! {
            _ = sigint.recv() => CancelKind::Interrupt,
            _ = sigterm.recv() => CancelKind::Terminate,
            _ = sighup.recv() => CancelKind::Terminate,
        }
    }
    #[cfg(windows)]
    {
        use tokio::signal::windows::{ctrl_break, ctrl_c, ctrl_close};
        let mut c_c = match ctrl_c() {
            Ok(s) => s,
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
                return CancelKind::Interrupt;
            }
        };
        let mut c_break = match ctrl_break() {
            Ok(s) => s,
            Err(_) => {
                // Break unavailable: wait only on Ctrl+C.
                let _ = c_c.recv().await;
                return CancelKind::Interrupt;
            }
        };
        let mut c_close = match ctrl_close() {
            Ok(s) => s,
            Err(_) => {
                tokio::select! {
                    _ = c_c.recv() => return CancelKind::Interrupt,
                    _ = c_break.recv() => return CancelKind::Terminate,
                }
            }
        };
        tokio::select! {
            _ = c_c.recv() => CancelKind::Interrupt,
            _ = c_break.recv() => CancelKind::Terminate,
            _ = c_close.recv() => CancelKind::Terminate,
        }
    }
    #[cfg(not(any(unix, windows)))]
    {
        // Unsupported platforms: portable Ctrl+C only.
        let _ = tokio::signal::ctrl_c().await;
        CancelKind::Interrupt
    }
}

/// Window for second SIGINT to force-exit (Rules Rust: double Ctrl-C).
pub const DOUBLE_INTERRUPT_FORCE_SECS: u64 = 5;

/// Install a process-wide second-SIGINT force-exit watcher.
///
/// First SIGINT is handled cooperatively by [`wait_for_cancel`] / [`race_op_with_cancel_and_deadline`].
/// A second SIGINT within [`DOUBLE_INTERRUPT_FORCE_SECS`] flushes stdio and exits 130.
/// Returns a join handle so callers may abort it after a clean finish (optional).
pub fn spawn_double_interrupt_force_exit() -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};
            let mut sigint = match signal(SignalKind::interrupt()) {
                Ok(s) => s,
                Err(_) => return,
            };
            // Consume first SIGINT (cooperative path also receives it).
            let _ = sigint.recv().await;
            let first = Instant::now();
            // Wait for a second SIGINT inside the force window.
            let second = sigint.recv();
            tokio::pin!(second);
            match tokio::time::timeout(
                Duration::from_secs(DOUBLE_INTERRUPT_FORCE_SECS),
                &mut second,
            )
            .await
            {
                Ok(Some(())) => {
                    let _ = first;
                    let _ = writeln_force_hint();
                    flush_stdio();
                    // Force path only: cooperative cleanup already had a chance on first SIGINT.
                    std::process::exit(i32::from(EXIT_INTERRUPTED));
                }
                Ok(None) => {
                    // Signal stream ended; do not force-exit.
                }
                Err(_) => {
                    // Window expired: no force-exit for a late second interrupt.
                }
            }
        }
        #[cfg(not(unix))]
        {
            // Portable: first ctrl_c is cooperative (other task); second within window forces.
            let _ = tokio::signal::ctrl_c().await;
            let first = Instant::now();
            match tokio::time::timeout(
                Duration::from_secs(DOUBLE_INTERRUPT_FORCE_SECS),
                tokio::signal::ctrl_c(),
            )
            .await
            {
                Ok(Ok(())) => {
                    if first.elapsed() <= Duration::from_secs(DOUBLE_INTERRUPT_FORCE_SECS) {
                        let _ = writeln_force_hint();
                        flush_stdio();
                        std::process::exit(i32::from(EXIT_INTERRUPTED));
                    }
                }
                _ => {}
            }
        }
    })
}

fn writeln_force_hint() -> std::io::Result<()> {
    use std::io::Write;
    let mut err = std::io::stderr();
    writeln!(
        err,
        "docsrs-cli: second interrupt within {DOUBLE_INTERRUPT_FORCE_SECS}s; forcing exit {EXIT_INTERRUPTED}"
    )?;
    err.flush()
}

/// Progress reporter: after `delay`, emit one localized line to stderr if still running.
///
/// `done` is an [`AtomicBool`] shared with a background task (not `Mutex<bool>`).
/// Stores/loads use [`Ordering::SeqCst`] so `finish` is visible before the delayed
/// write without relying on abort alone.
///
/// # Drop / non-orphan tasks
///
/// Tokio's [`JoinHandle`] **detaches** the task on drop (does not abort). This guard
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn wall_clock_timeout() {
        let err = with_wall_clock(Duration::from_millis(20), || async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<(), AppError>(())
        })
        .await
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }

    #[tokio::test]
    async fn cancel_flag_check_defaults_to_terminated() {
        let f = CancelFlag::new();
        assert!(f.check().is_ok());
        f.cancel();
        assert_eq!(f.check().unwrap_err().kind(), ErrorKind::Terminated);
        assert_eq!(f.kind(), Some(CancelKind::Terminate));
    }

    #[tokio::test]
    async fn cancel_flag_preserves_interrupt_kind() {
        let f = CancelFlag::new();
        f.cancel_with(CancelKind::Interrupt);
        assert_eq!(f.check().unwrap_err().kind(), ErrorKind::Interrupted);
        // First cancel wins: later terminate must not overwrite interrupt.
        f.cancel_with(CancelKind::Terminate);
        assert_eq!(f.check().unwrap_err().kind(), ErrorKind::Interrupted);
    }

    #[test]
    fn duration_ms_non_zero_after_sleep() {
        let start = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        assert!(duration_ms(start) >= 1);
    }

    #[tokio::test]
    async fn race_deadline_wins() {
        let flag = CancelFlag::new();
        let err = race_op_with_cancel_and_deadline(Duration::from_millis(15), flag, async {
            tokio::time::sleep(Duration::from_millis(200)).await;
            Ok::<(), AppError>(())
        })
        .await
        .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Timeout);
    }

    #[tokio::test]
    async fn progress_guard_quiet_is_noop() {
        let g = ProgressGuard::start(true, Duration::from_millis(1), "x".into());
        g.finish();
    }

    #[tokio::test]
    async fn progress_guard_drop_aborts_without_finish() {
        // Drop must abort (JoinHandle drop alone would detach the task).
        let g = ProgressGuard::start(false, Duration::from_secs(60), "must-not-print".into());
        drop(g);
    }

    #[test]
    fn flush_stdio_does_not_panic() {
        flush_stdio();
    }

    #[test]
    fn double_interrupt_window_is_five_seconds() {
        assert_eq!(DOUBLE_INTERRUPT_FORCE_SECS, 5);
    }

    #[tokio::test]
    async fn spawn_double_interrupt_handle_aborts_cleanly() {
        let h = spawn_double_interrupt_force_exit();
        h.abort();
        let _ = h.await;
    }
}
