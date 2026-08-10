//! OS signal capture and the double-interrupt force-exit escalation.
//!
//! Split from the cancellation primitives in the parent module: deciding *that*
//! work should stop is portable state machinery, while learning that the OS
//! asked for it is platform-specific and carries every `cfg(unix)` /
//! `cfg(windows)` branch in the crate.

use std::time::Duration;

use super::{CancelKind, flush_stdio};
use crate::error::EXIT_INTERRUPTED;

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
/// First SIGINT is handled cooperatively by [`wait_for_cancel`] / [`crate::shutdown::race_op_with_cancel_and_deadline`].
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
            // The force window is enforced by `tokio::time::timeout` below, which
            // owns its own deadline. A separate `Instant::now()` used to be taken
            // here and then discarded with `let _ = first;` in the force branch —
            // a stamp nothing read, kept alive only by the discard that silenced
            // the unused warning it would otherwise have raised.
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
            portable_double_interrupt().await;
        }
    })
}

/// Portable double-interrupt watcher for targets without POSIX signals.
///
/// Compiled on every target (including Unix, where it is never called) so the
/// body stays type-checked on the host. Attribute `cfg` is resolved during macro
/// expansion, before type checking, so a body that only ever compiles elsewhere
/// can carry errors into a release unnoticed.
#[cfg_attr(unix, allow(dead_code))]
async fn portable_double_interrupt() {
    // Portable: first ctrl_c is cooperative (other task); second within window forces.
    let _ = tokio::signal::ctrl_c().await;
    // `timeout` returning `Ok` already proves the second interrupt landed inside
    // the force window; an elapsed-time re-check would be vacuously true.
    if let Ok(Ok(())) = tokio::time::timeout(
        Duration::from_secs(DOUBLE_INTERRUPT_FORCE_SECS),
        tokio::signal::ctrl_c(),
    )
    .await
    {
        let _ = writeln_force_hint();
        flush_stdio();
        // Force path only: cooperative cleanup already had a chance on first ctrl_c.
        std::process::exit(i32::from(EXIT_INTERRUPTED));
    }
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
