//! Offline HTTP integration tests for the crates.io search surface (wiremock).

mod common;

use common::{origin_of, test_cfg};
use docsrs_cli::cli::SortKind;
use docsrs_cli::crates_io;
use docsrs_cli::domain::SearchQuery;
use docsrs_cli::http::HttpClient;
use docsrs_cli::shutdown::CancelFlag;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn search_crates_fixture_meta_and_fallback_description() {
    let server = MockServer::start().await;
    let body = include_str!("fixtures/crates_io/search_serde.json");
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .and(query_param("q", "serde"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(body, "application/json"))
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let query = SearchQuery::parse("example", false).unwrap();
    let url = crates_io::planned_url_on_host(
        &origin_of(&server.uri()),
        &query,
        10,
        SortKind::Downloads,
        1,
    )
    .unwrap();
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
async fn disk_cache_serves_second_request_without_network() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/crates_io/search_serde.json"),
            "application/json",
        ))
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
