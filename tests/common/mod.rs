//! Shared helpers for integration tests: process spawning and offline HTTP config.
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

use docsrs_cli::config::Config;
use docsrs_cli::domain::AllowedOrigin;

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

// ---------------------------------------------------------------------------
// Offline HTTP integration helpers (wiremock-backed suites).
// ---------------------------------------------------------------------------

/// Parse a wiremock `server.uri()` into an allowlisted origin (requires `allow_loopback`).
pub fn origin_of(uri: &str) -> AllowedOrigin {
    AllowedOrigin::parse_with(uri, true).expect("wiremock origin must pass allowlist")
}

/// Fast, loopback-friendly [`Config`] for wiremock-backed suites.
pub fn test_cfg(base: &str) -> Config {
    let _ = base;
    Config {
        rate_limit_delay_ms: 0,
        max_retries: 2,
        retry_base_ms: 50, // MIN_RETRY_BASE_MS floor
        retry_max_elapsed_ms: 10_000,
        timeout_secs: 5,
        connect_timeout_secs: 2,
        user_agent: "docsrs-cli/0.1.0 (test@example.com)".into(),
        allow_loopback: true,
        ..Config::default()
    }
}
