//! Offline HTTP integration tests for transport policy (wiremock).
//!
//! Covers body-size budget, host allowlist, retry/backoff, rate limiting and
//! cancellation — everything that is independent of the parsed payload.

mod common;

use std::time::Duration;

use common::test_cfg;
use docsrs_cli::error::ErrorKind;
use docsrs_cli::http::HttpClient;
use docsrs_cli::shutdown::CancelFlag;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn retry_on_503_then_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let resp = http.get_json(&url).await.unwrap();
    assert!(resp.status.is_success());
}

#[tokio::test]
async fn body_cap_returns_budget_not_retryable() {
    let server = MockServer::start().await;
    let big = "x".repeat(1024);
    Mock::given(method("GET"))
        .and(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(big, "text/html"))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_body_bytes = 64;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/big", server.uri())).unwrap();
    let err = http.get_html(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Budget);
    assert!(!err.retryable());
    assert!(err.message().contains("max_body_bytes"));
}

#[tokio::test]
async fn host_not_allowlisted() {
    let cfg = test_cfg("http://example.com");
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse("https://evil.example/api").unwrap();
    let err = http.get_json(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Network);
}

#[tokio::test]
async fn retry_on_429_then_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 2;
    cfg.retry_base_ms = 50;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let resp = http.get_json(&url).await.unwrap();
    assert!(resp.status.is_success());
}

#[tokio::test]
async fn retry_429_exhausted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "1"))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 0; // single attempt
    cfg.retry_base_ms = 1;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=x", server.uri())).unwrap();
    let err = http.get_json(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::RateLimited);
}

#[tokio::test]
async fn disable_retry_kill_switch_no_second_attempt() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(503))
        .expect(1)
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 5;
    cfg.disable_retry = true;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    assert!(!http.retry_policy().enabled);
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=x", server.uri())).unwrap();
    let err = http.get_json(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unavailable);
}

#[tokio::test]
async fn permanent_404_not_retried() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404).set_body_string("gone"))
        .expect(1)
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 5;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/missing", server.uri())).unwrap();
    let resp = http.get_html(&url).await.unwrap();
    assert_eq!(resp.status.as_u16(), 404);
}

#[tokio::test]
async fn retry_503_with_retry_after_then_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "0"))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 2;
    cfg.retry_base_ms = 50;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let resp = http.get_json(&url).await.unwrap();
    assert!(resp.status.is_success());
}

#[tokio::test]
async fn retry_429_with_http_date_retry_after_then_success() {
    let server = MockServer::start().await;
    // Near-future HTTP-date (≤1s) — parser accepts IMF-fixdate form.
    let when = std::time::SystemTime::now() + Duration::from_millis(200);
    let http_date = httpdate::fmt_http_date(when);
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", http_date.as_str()))
        .up_to_n_times(1)
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 2;
    cfg.retry_base_ms = 50;
    cfg.retry_max_elapsed_ms = 5_000;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let resp = http.get_json(&url).await.unwrap();
    assert!(resp.status.is_success());
}

#[tokio::test]
async fn retry_max_elapsed_budget_blocks_second_attempt() {
    let server = MockServer::start().await;
    // Always 503 with a multi-second Retry-After — budget of 100ms cannot afford the wait.
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(503).insert_header("retry-after", "5"))
        .expect(1)
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 5;
    cfg.retry_base_ms = 50;
    cfg.retry_max_delay_ms = 50;
    // Server Retry-After=5s > 100ms budget → no second attempt (hint not shrunk).
    cfg.retry_max_elapsed_ms = 100;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    assert_eq!(http.retry_policy().max_elapsed_ms, 100);
    let url = url::Url::parse(&format!("{}/api/v1/crates?q=x", server.uri())).unwrap();
    let err = http.get_json(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Unavailable);
    assert!(err.is_retryable());
}

#[tokio::test]
async fn cancel_before_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let cancel = CancelFlag::new();
    cancel.cancel();
    let http = HttpClient::new(cfg, cancel).unwrap();
    let url = url::Url::parse(&format!("{}/api/v1/crates", server.uri())).unwrap();
    let err = http.get_json(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Terminated);
}

#[tokio::test]
async fn rate_limit_delay_between_hits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("ok", "text/html"))
        .expect(2)
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.rate_limit_delay_ms = 30;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/ping", server.uri())).unwrap();
    let t0 = std::time::Instant::now();
    let _ = http.get_html(&url).await.unwrap();
    let _ = http.get_html(&url).await.unwrap();
    assert!(
        t0.elapsed() >= Duration::from_millis(20),
        "expected rate limit delay between hits"
    );
}

#[tokio::test]
async fn rate_limit_cross_process_stamp_with_cache_dir() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("ok", "text/html"))
        .expect(2)
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = test_cfg(&server.uri());
    cfg.rate_limit_delay_ms = 80;
    cfg.cache_dir = Some(dir.path().to_path_buf());
    cfg.no_cache = true; // only use cache_dir for rate-limit lock/stamp

    let http_a = HttpClient::new(cfg.clone(), CancelFlag::new()).unwrap();
    let http_b = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/ping", server.uri())).unwrap();
    let t0 = std::time::Instant::now();
    let _ = http_a.get_html(&url).await.unwrap();
    // Second client shares stamp via exclusive lock + stamp file.
    let _ = http_b.get_html(&url).await.unwrap();
    assert!(
        t0.elapsed() >= Duration::from_millis(60),
        "cross-process stamp should enforce delay, elapsed={:?}",
        t0.elapsed()
    );
}

#[tokio::test]
async fn rate_limit_cancel_during_sleep_propagates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/ping"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("ok", "text/html"))
        .mount(&server)
        .await;

    let mut cfg = test_cfg(&server.uri());
    cfg.rate_limit_delay_ms = 5_000;
    let cancel = CancelFlag::new();
    let http = HttpClient::new(cfg, cancel.clone()).unwrap();
    let url = url::Url::parse(&format!("{}/ping", server.uri())).unwrap();
    // First hit records in-process clock.
    let _ = http.get_html(&url).await.unwrap();
    cancel.cancel();
    let err = http.get_html(&url).await.unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Terminated);
}
