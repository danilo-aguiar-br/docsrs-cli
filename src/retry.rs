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
//!
//! # Delay formula
//!
//! ```text
//! cap(n)  = min(base_ms * 2^min(n, 16), max_delay_ms)
//! delay   = uniform(0..=cap)   // full jitter (AWS architecture blog)
//! ```
//!
//! Relógio monotônico: sleeps usam `tokio::time::sleep` (não `SystemTime`).
//! `Retry-After` delta-seconds overrides the formula for 429/503 when present.
//! HTTP-date `Retry-After` is not parsed (no chrono dep); falls back to formula.
//!
//! # Out of scope (one-shot agent CLI)
//!
//! Circuit breaker, retry budget token-bucket, hedged requests, gRPC,
//! OAuth refresh, outbox/saga, multi-host sticky sessions.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use reqwest::header::HeaderMap;
use reqwest::StatusCode;

use crate::config::Config;

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

/// Named retry policy for the product HTTP client (one dependency class).
///
/// Built from [`Config`] so CLI / TOML / env share one struct. Clone is cheap
/// (`Copy`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryConfig {
    /// Retries after the first attempt (`0` = single try).
    pub max_retries: u32,
    /// Exponential base delay in milliseconds.
    pub base_ms: u64,
    /// Maximum single sleep in milliseconds.
    pub max_delay_ms: u64,
    /// Kill switch: when false, never retries.
    pub enabled: bool,
}

impl RetryConfig {
    /// Resolve policy from runtime config (clamped).
    pub fn from_config(cfg: &Config) -> Self {
        let max_retries = cfg.max_retries.min(HARD_MAX_RETRIES);
        let base_ms = if cfg.retry_base_ms == 0 {
            DEFAULT_RETRY_BASE_MS
        } else {
            cfg.retry_base_ms
        };
        let max_delay_ms = cfg
            .retry_max_delay_ms
            .min(HARD_MAX_DELAY_MS)
            .max(base_ms);
        let enabled = !cfg.disable_retry && max_retries > 0;
        Self {
            max_retries,
            base_ms,
            max_delay_ms,
            enabled,
        }
    }

    /// Total attempts including the first try.
    pub fn max_attempts(self) -> u32 {
        if !self.enabled {
            1
        } else {
            self.max_retries.saturating_add(1)
        }
    }

    /// Whether another attempt is allowed after `attempt` (1-based completed count).
    pub fn may_retry(self, attempt: u32) -> bool {
        self.enabled && attempt < self.max_attempts()
    }

    /// Full-jitter delay for this attempt number (1-based).
    pub fn delay_for_attempt(self, attempt: u32) -> Duration {
        backoff_full_jitter(self.base_ms, attempt, self.max_delay_ms)
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: DEFAULT_MAX_RETRIES,
            base_ms: DEFAULT_RETRY_BASE_MS,
            max_delay_ms: DEFAULT_RETRY_MAX_DELAY_MS,
            enabled: true,
        }
    }
}

/// Classification of an HTTP status for the retry gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// Do not retry (4xx client errors, success, unknown permanent).
    Permanent,
    /// Retry with exponential full-jitter backoff (5xx).
    Transient,
    /// Retry respecting optional `Retry-After` (429, and 503 with header).
    RateLimited {
        /// Parsed `Retry-After` delta-seconds when present.
        retry_after: Option<Duration>,
    },
}

/// Classify HTTP status for in-process retry (GET-only product).
pub fn classify_http_status(status: StatusCode, retry_after: Option<Duration>) -> RetryClass {
    match status.as_u16() {
        429 => RetryClass::RateLimited { retry_after },
        500 | 502 | 504 => RetryClass::Transient,
        // 503: prefer Retry-After when present (same as many CDNs).
        503 => {
            if retry_after.is_some() {
                RetryClass::RateLimited { retry_after }
            } else {
                RetryClass::Transient
            }
        }
        // Success and permanent client / other statuses: never retry.
        _ => RetryClass::Permanent,
    }
}

/// Parse `Retry-After` as **delta-seconds** only.
///
/// HTTP-date form is intentionally unsupported (no `chrono`/`httpdate` dep).
/// Callers fall back to exponential backoff when this returns `None`.
pub fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())?
        .trim();
    // Reject HTTP-date and empty: only pure non-negative integer seconds.
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let secs: u64 = raw.parse().ok()?;
    // Cap pathological values (e.g. years) to hard max delay.
    Some(Duration::from_secs(secs.min(HARD_MAX_DELAY_MS / 1000)))
}

/// Full jitter: `uniform(0..=min(base*2^attempt, max_delay))`.
///
/// Entropy from monotonic `Instant` + stack address (no `rand` dependency).
/// Not cryptographic; sufficient to desynchronize multi-agent thundering herds.
pub fn backoff_full_jitter(base_ms: u64, attempt: u32, max_delay_ms: u64) -> Duration {
    let base = base_ms.max(1);
    let max_delay = max_delay_ms.max(base);
    // Saturating shift: attempt capped so 2^n fits in u64 comfortably.
    let exp = base.saturating_mul(1u64 << attempt.min(16));
    let cap = exp.min(max_delay);
    let pick = mix_u64(attempt) % (cap.saturating_add(1));
    Duration::from_millis(pick)
}

/// Per-host politeness delay with additive jitter (never below the configured floor).
///
/// ```text
/// effective = base + uniform(0..=base/5)   // up to +20% jitter
/// ```
///
/// Keeps the operator-configured minimum interval (crawl-delay style floor) while
/// avoiding synchronized multi-process hits that a fixed sleep would create.
/// Zero base stays zero (tests / `--rate-limit-delay-ms 0`).
pub fn politeness_delay(base: Duration) -> Duration {
    let base_ms = base.as_millis() as u64;
    if base_ms == 0 {
        return Duration::ZERO;
    }
    let span = (base_ms / 5).max(1);
    let extra = mix_u64(base_ms as u32) % (span.saturating_add(1));
    Duration::from_millis(base_ms.saturating_add(extra))
}

/// Prefer server `Retry-After` when present; otherwise full-jitter formula.
pub fn wait_for_retry(
    policy: RetryConfig,
    attempt: u32,
    retry_after: Option<Duration>,
) -> Duration {
    if let Some(d) = retry_after {
        // Respect server hint but never sleep longer than hard ceiling.
        return d.min(Duration::from_millis(policy.max_delay_ms));
    }
    policy.delay_for_attempt(attempt)
}

fn mix_u64(attempt: u32) -> u64 {
    let mut h = DefaultHasher::new();
    Instant::now().hash(&mut h);
    attempt.hash(&mut h);
    std::thread::current().id().hash(&mut h);
    // Stack address mixes per-call entropy without a global RNG.
    let marker = &h as *const DefaultHasher as usize;
    marker.hash(&mut h);
    h.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderValue, RETRY_AFTER};

    #[test]
    fn max_attempts_respects_kill_switch() {
        let mut p = RetryConfig::default();
        assert_eq!(p.max_attempts(), 4);
        p.enabled = false;
        assert_eq!(p.max_attempts(), 1);
        assert!(!p.may_retry(1));
    }

    #[test]
    fn from_config_disable_and_clamp() {
        let cfg = Config {
            disable_retry: true,
            max_retries: 99,
            retry_max_delay_ms: 999_999,
            ..Config::default()
        };
        let p = RetryConfig::from_config(&cfg);
        assert!(!p.enabled);
        assert_eq!(p.max_retries, HARD_MAX_RETRIES);
        assert_eq!(p.max_delay_ms, HARD_MAX_DELAY_MS);
        assert_eq!(p.max_attempts(), 1);
    }

    #[test]
    fn backoff_respects_cap() {
        let d = backoff_full_jitter(200, 20, 1_000);
        assert!(d.as_millis() <= 1_000);
    }

    #[test]
    fn politeness_delay_zero_stays_zero() {
        assert_eq!(politeness_delay(Duration::ZERO), Duration::ZERO);
    }

    #[test]
    fn politeness_delay_never_below_floor() {
        let base = Duration::from_millis(1000);
        for _ in 0..32 {
            let d = politeness_delay(base);
            assert!(d >= base, "delay {d:?} below floor {base:?}");
            // Floor + up to 20% (span = base/5).
            assert!(
                d <= base + Duration::from_millis(200),
                "delay {d:?} above floor+20%"
            );
        }
    }

    #[test]
    fn classify_matrix() {
        assert_eq!(
            classify_http_status(StatusCode::OK, None),
            RetryClass::Permanent
        );
        assert_eq!(
            classify_http_status(StatusCode::NOT_FOUND, None),
            RetryClass::Permanent
        );
        assert_eq!(
            classify_http_status(StatusCode::BAD_REQUEST, None),
            RetryClass::Permanent
        );
        assert_eq!(
            classify_http_status(StatusCode::INTERNAL_SERVER_ERROR, None),
            RetryClass::Transient
        );
        assert_eq!(
            classify_http_status(StatusCode::TOO_MANY_REQUESTS, Some(Duration::from_secs(2))),
            RetryClass::RateLimited {
                retry_after: Some(Duration::from_secs(2))
            }
        );
        assert_eq!(
            classify_http_status(StatusCode::SERVICE_UNAVAILABLE, Some(Duration::from_secs(1))),
            RetryClass::RateLimited {
                retry_after: Some(Duration::from_secs(1))
            }
        );
        assert_eq!(
            classify_http_status(StatusCode::SERVICE_UNAVAILABLE, None),
            RetryClass::Transient
        );
    }

    #[test]
    fn parse_retry_after_seconds_only() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static("7"));
        assert_eq!(parse_retry_after(&h), Some(Duration::from_secs(7)));

        h.insert(RETRY_AFTER, HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"));
        assert_eq!(parse_retry_after(&h), None);

        h.insert(RETRY_AFTER, HeaderValue::from_static(""));
        assert_eq!(parse_retry_after(&h), None);
    }

    #[test]
    fn wait_prefers_retry_after() {
        let p = RetryConfig::default();
        let w = wait_for_retry(p, 1, Some(Duration::from_secs(3)));
        assert_eq!(w, Duration::from_secs(3));
    }
}
