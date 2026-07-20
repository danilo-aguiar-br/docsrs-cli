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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant, SystemTime};

use reqwest::StatusCode;
use reqwest::header::HeaderMap;

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
/// Minimum allowed base delay (Rules Rust: never below 50ms in production config).
pub const MIN_RETRY_BASE_MS: u64 = 50;
/// Hard ceiling for total retry wall time including sleeps (milliseconds).
pub const HARD_MAX_ELAPSED_MS: u64 = 300_000;
/// Default total retry budget when not overridden (`0` in config = derive from timeout).
/// Used only when `timeout_secs` is unavailable; production derives from wall timeout.
pub const DEFAULT_RETRY_MAX_ELAPSED_MS: u64 = 30_000;
/// Clock-skew tolerance for past HTTP-date `Retry-After` values.
const RETRY_AFTER_SKEW_SECS: u64 = 1;

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
    /// Total wall budget for the retry loop (attempts + sleeps), milliseconds.
    pub max_elapsed_ms: u64,
    /// Kill switch: when false, never retries.
    pub enabled: bool,
}

impl RetryConfig {
    /// Resolve policy from runtime config (clamped).
    ///
    /// Coherence: `base_ms ≥ MIN_RETRY_BASE_MS`, `max_delay ≥ base`,
    /// `max_elapsed ≥ max_delay` (or wall timeout when config field is `0`).
    pub fn from_config(cfg: &Config) -> Self {
        let max_retries = cfg.max_retries.min(HARD_MAX_RETRIES);
        let base_ms = if cfg.retry_base_ms == 0 {
            DEFAULT_RETRY_BASE_MS
        } else {
            cfg.retry_base_ms.max(MIN_RETRY_BASE_MS)
        };
        let max_delay_ms = cfg.retry_max_delay_ms.min(HARD_MAX_DELAY_MS).max(base_ms);
        // `0` means derive from wall-clock timeout (one-shot: retries must not
        // outlive the operation deadline).
        let derived = cfg.timeout_secs.saturating_mul(1000).max(1_000);
        let max_elapsed_ms = if cfg.retry_max_elapsed_ms == 0 {
            derived.min(HARD_MAX_ELAPSED_MS)
        } else {
            cfg.retry_max_elapsed_ms
                .min(HARD_MAX_ELAPSED_MS)
                .max(max_delay_ms)
        };
        let enabled = !cfg.disable_retry && max_retries > 0;
        Self {
            max_retries,
            base_ms,
            max_delay_ms,
            max_elapsed_ms,
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

    /// Whether another attempt is allowed given attempt count **and** elapsed budget.
    ///
    /// Rules: combine `max_attempts` and `max_elapsed_time`. A planned wait that
    /// would exceed the remaining budget aborts retry (no sleep past budget).
    pub fn may_retry_within_budget(
        self,
        attempt: u32,
        elapsed: Duration,
        planned_wait: Duration,
    ) -> bool {
        if !self.may_retry(attempt) {
            return false;
        }
        let budget = Duration::from_millis(self.max_elapsed_ms);
        if elapsed >= budget {
            return false;
        }
        let remaining = budget.saturating_sub(elapsed);
        planned_wait <= remaining
    }

    /// Remaining time in the elapsed budget (zero when exhausted).
    pub fn remaining_budget(self, elapsed: Duration) -> Duration {
        Duration::from_millis(self.max_elapsed_ms).saturating_sub(elapsed)
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
            max_elapsed_ms: DEFAULT_RETRY_MAX_ELAPSED_MS,
            enabled: true,
        }
    }
}

/// Classification of an HTTP status for the retry gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// Do not retry (4xx client errors, success, unknown permanent).
    Permanent,
    /// Retry with exponential full-jitter backoff (5xx / 408).
    Transient,
    /// Retry respecting optional `Retry-After` (429, and 503 with header).
    RateLimited {
        /// Parsed `Retry-After` delta-seconds or HTTP-date delta when present.
        retry_after: Option<Duration>,
    },
}

/// Detailed retry category exposed on errors (Rules checklist `retry_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryKind {
    /// Never auto-retry (validation, 4xx, parse, budget, cancel).
    Permanent,
    /// Transport / connect failure (may retry).
    TransientNetwork,
    /// Remote 5xx / temporary unavailability (may retry).
    TransientServer,
    /// HTTP 429 with optional `Retry-After` (may retry).
    RateLimited,
    /// Wall or request timeout (may retry).
    Timeout,
}

/// Transport / protocol layer of a failure (Rules checklist `ErrorLayer`).
///
/// Reqwest/hyper does not always separate DNS from TCP or TLS handshake from
/// connect; `TcpConnect` covers DNS+TCP+TLS establishment when only
/// `is_connect()` is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLayer {
    /// Name resolution (when distinguishable; else folded into [`Self::TcpConnect`]).
    Dns,
    /// TCP connect / peer unreachable (includes DNS when not separated).
    TcpConnect,
    /// TLS handshake failure (when distinguishable).
    Tls,
    /// HTTP/1.1 or HTTP/2 protocol error after connect.
    HttpProtocol,
    /// HTTP status from a completed response.
    HttpStatus,
    /// Application / parse / budget after a successful body read.
    Application,
    /// Unclassified.
    Unknown,
}

impl ErrorLayer {
    /// Stable snake_case label for logs and doctor detail.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dns => "dns",
            Self::TcpConnect => "tcp_connect",
            Self::Tls => "tls",
            Self::HttpProtocol => "http_protocol",
            Self::HttpStatus => "http_status",
            Self::Application => "application",
            Self::Unknown => "unknown",
        }
    }
}

impl RetryKind {
    /// Stable snake_case label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::TransientNetwork => "transient_network",
            Self::TransientServer => "transient_server",
            Self::RateLimited => "rate_limited",
            Self::Timeout => "timeout",
        }
    }

    /// Whether this kind is eligible for in-process or agent retry.
    pub fn is_retryable(self) -> bool {
        !matches!(self, Self::Permanent)
    }
}

/// Classify HTTP status for in-process retry (GET-only product).
pub fn classify_http_status(status: StatusCode, retry_after: Option<Duration>) -> RetryClass {
    match status.as_u16() {
        429 => RetryClass::RateLimited { retry_after },
        // 408 Request Timeout — transient by design (client may retry).
        408 | 500 | 502 | 504 => RetryClass::Transient,
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

/// True when any error in the source chain is a [`rustls::Error`].
///
/// Does not string-match messages (Rules Rust — classification). When hyper
/// folds TLS into a plain connect I/O error without a rustls source, the
/// caller still reports [`ErrorLayer::TcpConnect`] (documented fold).
pub(crate) fn source_chain_has_rustls(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur {
        if e.downcast_ref::<rustls::Error>().is_some() {
            return true;
        }
        cur = e.source();
    }
    false
}

/// Classify a reqwest transport error without string-matching messages.
///
/// Returns `(app ErrorKind, layer, in_process_retryable)`.
///
/// When a [`rustls::Error`] appears in the source chain, the layer is
/// [`ErrorLayer::Tls`] even if reqwest also flags timeout/connect/request
/// (prefer precise TLS labeling for agent diagnostics).
pub fn classify_reqwest_error(err: &reqwest::Error) -> (crate::error::ErrorKind, ErrorLayer, bool) {
    use crate::error::ErrorKind;

    let tls = source_chain_has_rustls(err as &(dyn std::error::Error + 'static));

    if err.is_timeout() {
        // Connect or total timeout — both are transient for GET.
        let layer = if tls {
            ErrorLayer::Tls
        } else {
            ErrorLayer::TcpConnect
        };
        return (ErrorKind::Timeout, layer, true);
    }
    if tls {
        // Handshake / cert validation failures when discriminable via rustls.
        return (ErrorKind::Network, ErrorLayer::Tls, true);
    }
    if err.is_connect() {
        // Hyper folds DNS + TCP (+ often TLS without rustls source) into connect.
        return (ErrorKind::Network, ErrorLayer::TcpConnect, true);
    }
    if err.is_request() {
        // Protocol / request construction / HTTP/2 stream issues after dial.
        return (ErrorKind::Network, ErrorLayer::HttpProtocol, true);
    }
    if err.is_body() || err.is_decode() {
        // Incomplete body after headers — rare; treat as network, non-retry in
        // send path (body path is separate). Keep retryable=false here.
        return (ErrorKind::Network, ErrorLayer::HttpProtocol, false);
    }
    (ErrorKind::Network, ErrorLayer::Unknown, false)
}

/// Parse `Retry-After` as **delta-seconds** or **HTTP-date** (RFC 7231).
///
/// - Pure non-negative integer → seconds (capped at [`HARD_MAX_DELAY_MS`]).
/// - IMF-fixdate via `httpdate` → duration until that instant.
/// - Past date within [`RETRY_AFTER_SKEW_SECS`] → `Duration::ZERO`.
/// - Older past date or unparseable → `None` (caller falls back to formula).
pub fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())?
        .trim();
    if raw.is_empty() {
        return None;
    }
    // Delta-seconds (preferred when the value is purely numeric).
    if raw.bytes().all(|b| b.is_ascii_digit()) {
        let secs: u64 = raw.parse().ok()?;
        return Some(Duration::from_secs(secs.min(HARD_MAX_DELAY_MS / 1000)));
    }
    // HTTP-date form.
    let when = httpdate::parse_http_date(raw).ok()?;
    let now = SystemTime::now();
    match when.duration_since(now) {
        Ok(d) => Some(d.min(Duration::from_millis(HARD_MAX_DELAY_MS))),
        Err(e) => {
            // Clock skew: slightly past → treat as immediate; deep past → formula.
            if e.duration() <= Duration::from_secs(RETRY_AFTER_SKEW_SECS) {
                Some(Duration::ZERO)
            } else {
                None
            }
        }
    }
}

/// Full jitter: `uniform(0..=min(base*2^attempt, max_delay))`.
///
/// Entropy from monotonic `Instant` + stack address (no `rand` dependency).
/// Not cryptographic; sufficient to desynchronize multi-agent thundering herds.
/// For deterministic tests use [`backoff_full_jitter_seeded`].
pub fn backoff_full_jitter(base_ms: u64, attempt: u32, max_delay_ms: u64) -> Duration {
    let seed = mix_u64(attempt);
    backoff_full_jitter_seeded(base_ms, attempt, max_delay_ms, seed)
}

/// Full jitter with explicit seed (tests / property checks).
pub fn backoff_full_jitter_seeded(
    base_ms: u64,
    attempt: u32,
    max_delay_ms: u64,
    seed: u64,
) -> Duration {
    let base = base_ms.max(1);
    let max_delay = max_delay_ms.max(base);
    // Saturating shift: attempt capped so 2^n fits in u64 comfortably.
    let exp = base.saturating_mul(1u64 << attempt.min(16));
    let cap = exp.min(max_delay);
    let pick = seed % (cap.saturating_add(1));
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
///
/// Server hint is honored without additional jitter and **without** shrinking
/// below the server value via `max_delay_ms` (Rules: never impose a shorter
/// backoff than `Retry-After`). Only the hard delay ceiling applies; the
/// elapsed budget gate in the HTTP loop refuses waits that would overrun
/// `max_elapsed_ms`.
pub fn wait_for_retry(
    policy: RetryConfig,
    attempt: u32,
    retry_after: Option<Duration>,
) -> Duration {
    if let Some(d) = retry_after {
        return d.min(Duration::from_millis(HARD_MAX_DELAY_MS));
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
            retry_base_ms: 10, // below MIN → raised
            retry_max_elapsed_ms: 0, // derive from timeout
            timeout_secs: 12,
            ..Config::default()
        };
        let p = RetryConfig::from_config(&cfg);
        assert!(!p.enabled);
        assert_eq!(p.max_retries, HARD_MAX_RETRIES);
        assert_eq!(p.max_delay_ms, HARD_MAX_DELAY_MS);
        assert_eq!(p.base_ms, MIN_RETRY_BASE_MS);
        assert_eq!(p.max_elapsed_ms, 12_000);
        assert_eq!(p.max_attempts(), 1);
    }

    #[test]
    fn from_config_explicit_max_elapsed() {
        let cfg = Config {
            retry_max_elapsed_ms: 5_000,
            retry_base_ms: 200,
            retry_max_delay_ms: 1_000,
            ..Config::default()
        };
        let p = RetryConfig::from_config(&cfg);
        assert_eq!(p.max_elapsed_ms, 5_000);
    }

    #[test]
    fn may_retry_within_budget_blocks_oversleep() {
        let p = RetryConfig {
            max_retries: 5,
            base_ms: 100,
            max_delay_ms: 1_000,
            max_elapsed_ms: 500,
            enabled: true,
        };
        assert!(p.may_retry_within_budget(1, Duration::from_millis(0), Duration::from_millis(400)));
        assert!(!p.may_retry_within_budget(1, Duration::from_millis(0), Duration::from_millis(600)));
        assert!(!p.may_retry_within_budget(1, Duration::from_millis(500), Duration::ZERO));
    }

    #[test]
    fn backoff_respects_cap() {
        let d = backoff_full_jitter(200, 20, 1_000);
        assert!(d.as_millis() <= 1_000);
    }

    #[test]
    fn backoff_seeded_is_deterministic() {
        let a = backoff_full_jitter_seeded(200, 3, 10_000, 42);
        let b = backoff_full_jitter_seeded(200, 3, 10_000, 42);
        assert_eq!(a, b);
        // Property: always ≤ cap for any seed.
        for seed in 0..64u64 {
            for attempt in 0..8u32 {
                let d = backoff_full_jitter_seeded(100, attempt, 500, seed);
                assert!(d.as_millis() <= 500);
            }
        }
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
            classify_http_status(StatusCode::REQUEST_TIMEOUT, None),
            RetryClass::Transient
        );
        assert_eq!(
            classify_http_status(StatusCode::TOO_MANY_REQUESTS, Some(Duration::from_secs(2))),
            RetryClass::RateLimited {
                retry_after: Some(Duration::from_secs(2))
            }
        );
        assert_eq!(
            classify_http_status(
                StatusCode::SERVICE_UNAVAILABLE,
                Some(Duration::from_secs(1))
            ),
            RetryClass::RateLimited {
                retry_after: Some(Duration::from_secs(1))
            }
        );
        assert_eq!(
            classify_http_status(StatusCode::SERVICE_UNAVAILABLE, None),
            RetryClass::Transient
        );
        // Permanent auth / client errors never retry.
        assert_eq!(
            classify_http_status(StatusCode::UNAUTHORIZED, None),
            RetryClass::Permanent
        );
        assert_eq!(
            classify_http_status(StatusCode::FORBIDDEN, None),
            RetryClass::Permanent
        );
        assert_eq!(
            classify_http_status(StatusCode::UNPROCESSABLE_ENTITY, None),
            RetryClass::Permanent
        );
    }

    #[test]
    fn parse_retry_after_seconds_and_http_date() {
        let mut h = HeaderMap::new();
        h.insert(RETRY_AFTER, HeaderValue::from_static("7"));
        assert_eq!(parse_retry_after(&h), Some(Duration::from_secs(7)));

        h.insert(RETRY_AFTER, HeaderValue::from_static(""));
        assert_eq!(parse_retry_after(&h), None);

        // Future HTTP-date (~2s) must parse to a short positive duration.
        let future = SystemTime::now() + Duration::from_secs(2);
        let http_date = httpdate::fmt_http_date(future);
        h.insert(
            RETRY_AFTER,
            HeaderValue::from_str(&http_date).expect("http-date header"),
        );
        let d = parse_retry_after(&h).expect("HTTP-date Retry-After");
        assert!(d <= Duration::from_secs(3), "got {d:?}");
        assert!(d >= Duration::from_millis(500), "got {d:?}");

        // Deep past HTTP-date → None (fall back to formula).
        let past = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let past_s = httpdate::fmt_http_date(past);
        h.insert(
            RETRY_AFTER,
            HeaderValue::from_str(&past_s).expect("past http-date"),
        );
        assert_eq!(parse_retry_after(&h), None);
    }

    #[test]
    fn wait_prefers_retry_after() {
        let p = RetryConfig::default();
        let w = wait_for_retry(p, 1, Some(Duration::from_secs(3)));
        assert_eq!(w, Duration::from_secs(3));
    }

    #[test]
    fn source_chain_detects_rustls_and_ignores_plain_io() {
        let tls = rustls::Error::General("unit-test".into());
        assert!(source_chain_has_rustls(&tls));
        let io = std::io::Error::other("not tls");
        assert!(!source_chain_has_rustls(&io));
        assert_eq!(ErrorLayer::Tls.as_str(), "tls");
    }

    #[test]
    fn retry_kind_labels() {
        assert_eq!(RetryKind::Timeout.as_str(), "timeout");
        assert!(RetryKind::TransientNetwork.is_retryable());
        assert!(!RetryKind::Permanent.is_retryable());
        assert_eq!(ErrorLayer::TcpConnect.as_str(), "tcp_connect");
    }
}
