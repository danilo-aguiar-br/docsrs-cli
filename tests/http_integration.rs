//! Offline HTTP integration tests using wiremock (no external network).

use std::time::Duration;

use docsrs_cli::cli::SortKind;
use docsrs_cli::config::Config;
use docsrs_cli::crates_io;
use docsrs_cli::docs_rs;
use docsrs_cli::domain::{AllowedOrigin, CrateName, SearchQuery, VersionArg};
use docsrs_cli::error::ErrorKind;
use docsrs_cli::http::HttpClient;
use docsrs_cli::item_kind::ItemKind;
use docsrs_cli::shutdown::CancelFlag;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cn(s: &str) -> CrateName {
    CrateName::parse(s).unwrap()
}
fn ver(s: &str) -> VersionArg {
    VersionArg::parse(s).unwrap()
}
fn sq(s: &str) -> SearchQuery {
    SearchQuery::parse(s, true).unwrap()
}

/// Parse wiremock `server.uri()` into an allowlisted origin (requires `allow_loopback`).
fn origin_of(uri: &str) -> AllowedOrigin {
    AllowedOrigin::parse_with(uri, true).expect("wiremock origin must pass allowlist")
}

fn test_cfg(base: &str) -> Config {
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

#[tokio::test]
async fn search_crates_fixture_meta_and_fallback_description() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/crates_io/search_serde.json");
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .and(query_param("q", "serde"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body, "application/json"),
        )
        .mount(&server)
        .await;

    let cfg = test_cfg(server.uri().as_str());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!(
        "{}/api/v1/crates?q=serde&per_page=10&sort=relevance&page=1",
        server.uri()
    ))
    .unwrap();
    let resp = http.get_json(&url).await.unwrap();
    assert!(resp.status.is_success());
    let text = docsrs_cli::http::decode_utf8(&resp.body).unwrap();
    assert!(text.contains("\"exact_match\": true"));
    assert!(text.contains("No description") || text.contains("serde_json"));
}

#[tokio::test]
async fn search_crates_at_end_to_end_with_seek_meta() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/crates_io/search_seek_meta.json");
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(body, "application/json"),
        )
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let query = SearchQuery::parse("example", false).unwrap();
    let url =
        crates_io::planned_url_on_host(&origin_of(&server.uri()), &query, 10, SortKind::Downloads, 1).unwrap();
    let data = crates_io::search_crates_at(&http, &url, &query, 10, SortKind::Downloads, 1)
        .await
        .unwrap();
    assert_eq!(data.hits[0].name, "example");
    assert_eq!(
        data.meta.next_page.as_deref(),
        Some("seek=ABC123&per_page=10")
    );
    assert_eq!(data.meta.prev_page.as_deref(), Some("page=1"));
}

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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(big, "text/html"),
        )
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
async fn docs_rs_readme_fixture_via_mock() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/latest/demo/index.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/readme_docblock.html"), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = url::Url::parse(&format!("{}/demo/latest/demo/index.html", server.uri())).unwrap();
    let resp = http.get_html(&url).await.unwrap();
    let html = docsrs_cli::http::decode_utf8(&resp.body).unwrap();
    let (md, empty) = docs_rs::extract_readme_markdown_from_html(&html).unwrap();
    assert!(!empty);
    assert!(md.to_lowercase().contains("demo") || md.contains("main"));
    assert!(!md.contains("alert"));
}

#[tokio::test]
async fn all_html_fixture_kinds() {
    let html = include_str!("fixtures/docs_rs/all_html_sample.html");
    let base = url::Url::parse("https://docs.rs/demo/1.0.0/demo/all.html").unwrap();
    let hits = docs_rs::parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("", true).unwrap(),
        None,
        100,
        docsrs_cli::domain::MatchMode::Prefix,
        &docsrs_cli::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert!(
        hits.iter()
            .any(|h| h.kind == "struct" && h.name == "Client")
    );
    assert!(hits.iter().any(|h| h.kind == "attribute"));
    assert!(hits.iter().any(|h| h.kind == "derive"));
    assert!(hits.iter().any(|h| h.kind == "union"));
    assert!(hits.iter().any(|h| h.name == "de::Deserialize"));
    assert!(!hits.iter().any(|h| h.name == "skipme"));
    let limited = docs_rs::parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("", true).unwrap(),
        None,
        2,
        docsrs_cli::domain::MatchMode::Prefix,
        &docsrs_cli::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(limited.len(), 2);
    assert_eq!(limited[0].name, "Client");
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
async fn item_kind_function_alias_url() {
    let segs = vec!["get".into()];
    let u = docs_rs::get_item_url(
        &CrateName::parse("reqwest").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::parse("function").unwrap(),
        &segs,
    )
    .unwrap();
    assert!(u.as_str().contains("/fn.get.html"));
    let _ = Duration::from_millis(1);
}

#[tokio::test]
async fn fetch_readme_at_end_to_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/latest/demo/index.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/readme_docblock.html"), "text/html; charset=utf-8"),
        )
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("latest")).unwrap();
    let data = docs_rs::fetch_readme_at(&http, &cn("demo"), &ver("latest"), &url)
        .await
        .unwrap();
    assert_eq!(data.crate_name, "demo");
    assert_eq!(data.version, "latest");
    assert!(!data.empty);
    assert!(!data.truncated);
    assert!(data.markdown.to_lowercase().contains("demo") || data.markdown.contains("main"));
    assert!(!data.markdown.to_ascii_lowercase().contains("alert"));
    assert!(data.source_url.contains("/demo/latest/demo/index.html"));
}

#[tokio::test]
async fn fetch_readme_at_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing/latest/missing/index.html"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("missing"), &ver("latest")).unwrap();
    let err = docs_rs::fetch_readme_at(&http, &cn("missing"), &ver("latest"), &url)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn fetch_item_at_end_to_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tokio/latest/tokio/runtime/struct.Runtime.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/get_item_main.html"), "text/html"),
        )
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
    let url =
        docs_rs::get_item_url_on_origin(&origin_of(&server.uri()), &cn("tokio"), &ver("latest"), ItemKind::Struct, &segs)
            .unwrap();
    let data = docs_rs::fetch_item_at(&http, &cn("tokio"), &ver("latest"), ItemKind::Struct, &segs, &url)
        .await
        .unwrap();
    assert_eq!(data.item_type, "struct");
    assert_eq!(data.item_path, "tokio::runtime::Runtime");
    assert!(!data.empty);
    assert!(
        data.markdown.contains("Runtime")
            || data.markdown.contains("Tokio")
            || data.markdown.contains("runtime")
    );
    assert!(data.title.contains("struct"));
}

#[tokio::test]
async fn fetch_item_at_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let segs = vec!["Missing".into()];
    let url =
        docs_rs::get_item_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0"), ItemKind::Struct, &segs)
            .unwrap();
    let err = docs_rs::fetch_item_at(&http, &cn("demo"), &ver("1.0.0"), ItemKind::Struct, &segs, &url)
        .await
        .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn search_in_crate_at_end_to_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/all_html_sample.html"), "text/html"),
        )
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0")).unwrap();
    let data = docs_rs::search_in_crate_at(
        &http,
        &cn("demo"),
        &ver("1.0.0"),
        &sq("Client"),
        Some(ItemKind::Struct),
        10,
        docsrs_cli::domain::MatchMode::Prefix,
        &url,
    )
    .await
    .unwrap();
    assert_eq!(data.total, 1);
    assert_eq!(data.emitted, 1);
    assert_eq!(data.hits[0].name, "Client");
    assert_eq!(data.hits[0].kind, "struct");
    assert!(!data.truncated);

    let constants = docs_rs::search_in_crate_at(
        &http,
        &cn("demo"),
        &ver("1.0.0"),
        &sq(""),
        Some(ItemKind::Constant),
        10,
        docsrs_cli::domain::MatchMode::Prefix,
        &url,
    )
    .await
    .unwrap();
    assert_eq!(constants.total, 1, "constant.MAX must be indexed");
    assert_eq!(constants.hits[0].name, "MAX");
    assert_eq!(constants.hits[0].kind, "constant");
}

#[tokio::test]
async fn search_in_crate_at_404() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gone/latest/gone/all.html"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("gone"), &ver("latest")).unwrap();
    let err = docs_rs::search_in_crate_at(
        &http,
        &cn("gone"),
        &ver("latest"),
        &sq("x"),
        None,
        10,
        docsrs_cli::domain::MatchMode::Prefix,
        &url,
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("ok", "text/html"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("ok", "text/html"),
        )
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
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("ok", "text/html"),
        )
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

#[tokio::test]
async fn non_success_status_on_readme_returns_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/latest/demo/index.html"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    let mut cfg = test_cfg(&server.uri());
    cfg.max_retries = 0;
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("latest")).unwrap();
    let err = docs_rs::fetch_readme_at(&http, &cn("demo"), &ver("latest"), &url)
        .await
        .unwrap_err();
    assert!(matches!(
        err.kind(),
        ErrorKind::Unavailable | ErrorKind::Network | ErrorKind::Internal
    ));
}

#[tokio::test]
async fn empty_readme_fixture_via_fetch() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/empty/1.0.0/empty/index.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw("<html><body><main id=\"main-content\"></main></body></html>", "text/html"),
        )
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("empty"), &ver("1.0.0")).unwrap();
    let data = docs_rs::fetch_readme_at(&http, &cn("empty"), &ver("1.0.0"), &url)
        .await
        .unwrap();
    assert!(data.empty);
    assert!(data.markdown.is_empty());
}

#[tokio::test]
async fn origin_url_builders_hyphen_crate() {
    let origin = origin_of("http://127.0.0.1:9");
    let r = docs_rs::readme_url_on_origin(&origin, &cn("async-trait"), &ver("latest")).unwrap();
    assert!(
        r.as_str()
            .contains("/async-trait/latest/async_trait/index.html")
    );
    let a = docs_rs::all_html_url_on_origin(&origin, &cn("async-trait"), &ver("1.0.0")).unwrap();
    assert!(
        a.as_str()
            .contains("/async-trait/1.0.0/async_trait/all.html")
    );
    let segs = vec!["Parser".into()];
    let g =
        docs_rs::get_item_url_on_origin(&origin, &cn("clap"), &ver("latest"), ItemKind::Trait, &segs).unwrap();
    assert!(g.as_str().contains("/clap/latest/clap/trait.Parser.html"));
}

#[tokio::test]
async fn disk_cache_serves_second_request_without_network() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
        .expect(1)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    let mut cfg = test_cfg(&server.uri());
    cfg.cache_dir = Some(cache_dir.path().to_path_buf());
    cfg.cache_ttl_secs = 3600;
    cfg.no_cache = false;

    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let http1 = HttpClient::new(cfg.clone(), CancelFlag::new()).unwrap();
    let first = http1.get_json(&url).await.unwrap();
    assert!(first.status.is_success());

    // New process-equivalent client: same cache dir, mock allows only 1 request total.
    let http2 = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let second = http2.get_json(&url).await.unwrap();
    assert!(second.status.is_success());
    assert_eq!(first.body, second.body);
}

#[tokio::test]
async fn no_cache_flag_bypasses_disk() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/crates_io/search_serde.json"), "application/json"),
        )
        .expect(2)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    let mut cfg = test_cfg(&server.uri());
    cfg.cache_dir = Some(cache_dir.path().to_path_buf());
    cfg.no_cache = true;

    let url = url::Url::parse(&format!("{}/api/v1/crates?q=serde", server.uri())).unwrap();
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let _ = http.get_json(&url).await.unwrap();
    let _ = http.get_json(&url).await.unwrap();
}

#[tokio::test]
async fn search_in_crate_truncated_true_when_limit_cuts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/all_html_sample.html"), "text/html"),
        )
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0")).unwrap();
    let data = docs_rs::search_in_crate_at(
        &http,
        &cn("demo"),
        &ver("1.0.0"),
        &sq(""),
        None,
        2,
        docsrs_cli::domain::MatchMode::Prefix,
        &url,
    )
    .await
    .unwrap();
    assert!(data.total > 2);
    assert_eq!(data.emitted, 2);
    assert!(data.truncated);
}

#[tokio::test]
async fn search_in_crate_limit_zero_emits_empty_hits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(include_str!("fixtures/docs_rs/all_html_sample.html"), "text/html"),
        )
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0")).unwrap();
    let data = docs_rs::search_in_crate_at(
        &http,
        &cn("demo"),
        &ver("1.0.0"),
        &sq(""),
        None,
        0,
        docsrs_cli::domain::MatchMode::Prefix,
        &url,
    )
    .await
    .unwrap();
    assert!(data.total > 0);
    assert_eq!(data.emitted, 0);
    assert!(data.hits.is_empty());
    assert!(data.truncated);
}
