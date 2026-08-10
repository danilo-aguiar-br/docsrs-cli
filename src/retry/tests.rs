//! Unit tests for the retry policy, classification, and backoff formulas.

use std::time::{Duration, SystemTime};

use reqwest::StatusCode;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};

use super::classify::source_chain_has_rustls;
use super::*;
use crate::config::Config;

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
        retry_base_ms: 10,       // below MIN → raised
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
