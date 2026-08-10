//! Offline HTTP integration tests for the docs.rs surface (wiremock).
//!
//! Covers readme, get-item and search-in-crate fetches plus URL builders.

mod common;

use std::time::Duration;

use common::{origin_of, test_cfg};
use docsrs_cli::docs_rs;
use docsrs_cli::domain::{CrateName, SearchQuery, VersionArg};
use docsrs_cli::error::ErrorKind;
use docsrs_cli::http::HttpClient;
use docsrs_cli::item_kind::ItemKind;
use docsrs_cli::shutdown::CancelFlag;
use wiremock::matchers::{method, path};
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

#[tokio::test]
async fn docs_rs_readme_fixture_via_mock() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/latest/demo/index.html"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/readme_docblock.html"),
            "text/html; charset=utf-8",
        ))
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/readme_docblock.html"),
            "text/html; charset=utf-8",
        ))
        .mount(&server)
        .await;

    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("latest"))
        .unwrap();
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
    let url =
        docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("missing"), &ver("latest"))
            .unwrap();
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/get_item_main.html"),
            "text/html",
        ))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
    let url = docs_rs::get_item_url_on_origin(
        &origin_of(&server.uri()),
        &cn("tokio"),
        &ver("latest"),
        ItemKind::Struct,
        &segs,
    )
    .unwrap();
    let data = docs_rs::fetch_item_at(
        &http,
        &cn("tokio"),
        &ver("latest"),
        ItemKind::Struct,
        &segs,
        &url,
    )
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
    let url = docs_rs::get_item_url_on_origin(
        &origin_of(&server.uri()),
        &cn("demo"),
        &ver("1.0.0"),
        ItemKind::Struct,
        &segs,
    )
    .unwrap();
    let err = docs_rs::fetch_item_at(
        &http,
        &cn("demo"),
        &ver("1.0.0"),
        ItemKind::Struct,
        &segs,
        &url,
    )
    .await
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
}

#[tokio::test]
async fn search_in_crate_at_end_to_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/all_html_sample.html"),
            "text/html",
        ))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url =
        docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0"))
            .unwrap();
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
    let url =
        docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("gone"), &ver("latest"))
            .unwrap();
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
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("latest"))
        .unwrap();
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            "<html><body><main id=\"main-content\"></main></body></html>",
            "text/html",
        ))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url = docs_rs::readme_url_on_origin(&origin_of(&server.uri()), &cn("empty"), &ver("1.0.0"))
        .unwrap();
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
    let g = docs_rs::get_item_url_on_origin(
        &origin,
        &cn("clap"),
        &ver("latest"),
        ItemKind::Trait,
        &segs,
    )
    .unwrap();
    assert!(g.as_str().contains("/clap/latest/clap/trait.Parser.html"));
}

#[tokio::test]
async fn search_in_crate_truncated_true_when_limit_cuts() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/all_html_sample.html"),
            "text/html",
        ))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url =
        docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0"))
            .unwrap();
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
        .respond_with(ResponseTemplate::new(200).set_body_raw(
            include_str!("fixtures/docs_rs/all_html_sample.html"),
            "text/html",
        ))
        .mount(&server)
        .await;
    let cfg = test_cfg(&server.uri());
    let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
    let url =
        docs_rs::all_html_url_on_origin(&origin_of(&server.uri()), &cn("demo"), &ver("1.0.0"))
            .unwrap();
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
