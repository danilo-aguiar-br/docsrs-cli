//! Unit tests for cancellation primitives and signal mapping.

use super::*;
use crate::error::ErrorKind;

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
