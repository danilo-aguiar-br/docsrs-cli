//! Shared process-spawn helpers for integration tests.
//!
//! # Scope (Rules Rust — processos externos)
//!
//! Product `src/` never spawns external binaries (HTTP-only agent CLI).
//! Integration tests are the **only** `std::process::Command` sites, and they
//! always target the under-test absolute path from `CARGO_BIN_EXE_docsrs-cli`
//! (never a shell, never PATH lookup of a third-party tool).
//!
//! # Stdio policy
//!
//! Every `Command` sets `stdin` / `stdout` / `stderr` explicitly:
//! - capture mode: `null` / `piped` / `piped` (for `.output()` assertions)
//! - silent mode: all `null` (signal harnesses that only need exit codes)
//!
//! # Environment hygiene
//!
//! Removes noisy log env and common injection vectors (`LD_PRELOAD`,
//! `DYLD_INSERT_LIBRARIES`, …) so child runs are closer to a clean spawn.
//!
//! Each integration test file is a separate crate that may use only a subset of
//! these helpers — `dead_code` is therefore expected and allowed here.

#![allow(dead_code)]

use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

/// Absolute path to the product binary under test.
pub fn docsrs_cli_bin() -> &'static str {
    env!("CARGO_BIN_EXE_docsrs-cli")
}

/// Build a `Command` for the product binary with **capture** Stdio policy.
///
/// - `stdin` → [`Stdio::null`]: tests never feed stdin; avoid inherit races
/// - `stdout` / `stderr` → [`Stdio::piped`]: capture for assertions via `.output()`
///
/// Prefer this over ad-hoc `Command::new` so Stdio/env policy stays in one place.
pub fn docsrs_cli_cmd() -> Command {
    let mut c = Command::new(docsrs_cli_bin());
    // Explicit Stdio — do not rely on `.output()` / spawn defaults alone.
    c.stdin(Stdio::null());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());
    sanitize_child_env(&mut c);
    c
}

/// Build a `Command` with all streams discarded (exit-code-only harnesses).
pub fn docsrs_cli_cmd_silent() -> Command {
    let mut c = Command::new(docsrs_cli_bin());
    c.stdin(Stdio::null());
    c.stdout(Stdio::null());
    c.stderr(Stdio::null());
    sanitize_child_env(&mut c);
    c
}

fn sanitize_child_env(c: &mut Command) {
    c.env_remove("RUST_LOG");
    // Injection / preload surfaces (Rules Rust — ambiente e contexto).
    c.env_remove("LD_PRELOAD");
    c.env_remove("LD_LIBRARY_PATH");
    c.env_remove("LD_AUDIT");
    c.env_remove("DYLD_INSERT_LIBRARIES");
    c.env_remove("DYLD_LIBRARY_PATH");
}

/// Wait for `child` up to `max`, then `kill` + `wait` to avoid hung tests / zombies.
///
/// Always reaps the child (Rules Rust: every `Child` must be `wait`ed).
pub fn wait_with_timeout(child: &mut Child, max: Duration) -> std::io::Result<ExitStatus> {
    let start = Instant::now();
    loop {
        match child.try_wait()? {
            Some(status) => return Ok(status),
            None => {
                if start.elapsed() >= max {
                    // Last resort: SIGKILL on Unix / TerminateProcess on Windows.
                    let _ = child.kill();
                    return child.wait();
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
    }
}
