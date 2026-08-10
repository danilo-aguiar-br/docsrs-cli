//! Explicit HTTP retry policy for crates.io / docs.rs (Rules Rust — retry/backoff).
//!
//! # Workload and policy scope
//!
//! - **Dependency:** public HTTPS hosts only (`crates.io`, `docs.rs`,
//!   `static.docs.rs`, `doc.rust-lang.org`).
//! - **Operations:** product surface is **GET-only** (idempotent by HTTP
//!   semantics). No POST/PUT/DELETE; no Idempotency-Key required.
//! - **Default:** retries **on** for transient failures (agent reliability).
//!   Kill switch: `--disable-retry` / TOML `disable_retry = true` / `max_retries = 0`.
//! - **Layer:** single retry loop inside [`crate::http::HttpClient`] only —
//!   no middleware stack, no nested client retries, no agent-level auto-retry
//!   of permanent kinds.
//! - **Budget:** `max_attempts` **and** `max_elapsed_ms` are enforced together;
//!   a sleep that would exceed the elapsed budget aborts retry (no partial
//!   sleep then fail).
//!
//! # Delay formula
//!
//! ```text
//! cap(n)  = min(base_ms * 2^min(n, 16), max_delay_ms)
//! delay   = uniform(0..=cap)   // full jitter (AWS Architecture Blog)
//! ```
//!
//! Monotonic clock: sleeps use `tokio::time::sleep` (not `SystemTime`).
//! `Retry-After` (delta-seconds **or** HTTP-date via `httpdate`) overrides the
//! formula for 429/503 when present. Past HTTP-dates within 1s skew → zero wait;
//! older past dates fall back to the formula.
//!
//! # Out of scope (one-shot agent CLI)
//!
//! Circuit breaker, retry budget token-bucket, hedged requests, gRPC,
//! OAuth refresh, outbox/saga, multi-host sticky sessions.
//!
//! Layout (SRP): `config` (policy struct) · `classify` (status / transport /
//! `Retry-After`) · `backoff` (jitter formulas and wait selection).

mod backoff;
mod classify;
mod config;

pub use backoff::{
    backoff_full_jitter, backoff_full_jitter_seeded, politeness_delay, wait_for_retry,
};
pub use classify::{
    ErrorLayer, RetryClass, RetryKind, classify_http_status, classify_reqwest_error,
    parse_retry_after,
};
pub use config::RetryConfig;

/// Default maximum retries after the first attempt (total attempts = N+1).
pub const DEFAULT_MAX_RETRIES: u32 = 3;
/// Default base backoff in milliseconds (also minimum floor for formula).
pub const DEFAULT_RETRY_BASE_MS: u64 = 200;
/// Default ceiling for a single backoff sleep (milliseconds).
pub const DEFAULT_RETRY_MAX_DELAY_MS: u64 = 30_000;
/// Hard cap on configured retries (prevents accidental retry storms).
pub const HARD_MAX_RETRIES: u32 = 10;
/// Hard ceiling for a single delay (milliseconds).
pub const HARD_MAX_DELAY_MS: u64 = 60_000;
/// Minimum allowed base delay (Rules Rust: never below 50ms in production config).
pub const MIN_RETRY_BASE_MS: u64 = 50;
/// Hard ceiling for total retry wall time including sleeps (milliseconds).
pub const HARD_MAX_ELAPSED_MS: u64 = 300_000;
/// Seconds-to-milliseconds factor for the two places retry crosses units.
///
/// Named so the direction of the conversion is readable at the call site: a
/// bare `1000` reads the same whether it multiplies or divides.
pub const MILLIS_PER_SECOND: u64 = 1_000;
/// Floor for the retry budget derived from `timeout_secs` (milliseconds).
///
/// `timeout_secs` is a whole number, so any sub-second intent collapses to `0`
/// and would leave the budget exhausted before the first attempt could sleep.
/// One second is the smallest window in which a single retry is still possible.
pub const MIN_DERIVED_ELAPSED_MS: u64 = 1_000;
/// Default total retry budget when not overridden (`0` in config = derive from timeout).
/// Used only when `timeout_secs` is unavailable; production derives from wall timeout.
pub const DEFAULT_RETRY_MAX_ELAPSED_MS: u64 = 30_000;

#[cfg(test)]
mod tests;
