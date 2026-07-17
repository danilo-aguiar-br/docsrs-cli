//! Wall-clock timeout, signal awareness, and cancel racing for one-shot runs.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

use tokio::time::{Instant, timeout};

use crate::error::{AppError, AppResult, ErrorKind};

/// Which OS signal cancelled the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelKind {
    Interrupt,
    Terminate,
}

/// Shared cancel flag checked by HTTP retries between attempts.
#[derive(Debug, Clone, Default)]
pub struct CancelFlag {
    inner: Arc<AtomicBool>,
}

impl CancelFlag {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn cancel(&self) {
        self.inner.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.load(Ordering::SeqCst)
    }

    pub fn check(&self) -> AppResult<()> {
        if self.is_cancelled() {
            // Default to terminated; callers that know the signal override the kind.
            Err(AppError::terminated())
        } else {
            Ok(())
        }
    }
}

/// Run a future with a wall-clock deadline.
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
                cancel_flag.cancel();
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
            cancel_flag.cancel();
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

/// Future that completes when SIGINT or SIGTERM (Unix) / Ctrl-C (Windows) arrives.
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
        tokio::select! {
            _ = sigint.recv() => CancelKind::Interrupt,
            _ = sigterm.recv() => CancelKind::Terminate,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        CancelKind::Interrupt
    }
}

/// Progress reporter: after `delay`, emit one localized line to stderr if still running.
pub struct ProgressGuard {
    done: Arc<AtomicBool>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl ProgressGuard {
    pub fn start(quiet: bool, delay: Duration, message: String) -> Self {
        let done = Arc::new(AtomicBool::new(false));
        if quiet {
            return Self { done, handle: None };
        }
        let flag = Arc::clone(&done);
        let handle = tokio::spawn(async move {
            tokio::time::sleep(delay).await;
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

    pub fn finish(self) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(h) = self.handle {
            h.abort();
        }
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
    async fn cancel_flag_check() {
        let f = CancelFlag::new();
        assert!(f.check().is_ok());
        f.cancel();
        assert_eq!(f.check().unwrap_err().kind(), ErrorKind::Terminated);
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

    #[test]
    fn flush_stdio_does_not_panic() {
        flush_stdio();
    }
}
