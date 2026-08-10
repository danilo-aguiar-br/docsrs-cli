use super::parse::truncate_chars;
use super::*;
use crate::cli::SortKind;
use crate::config::{MAX_CRATE_DESCRIPTION_CHARS, MAX_CRATE_NAME_CHARS, MAX_URL_FIELD_CHARS};
use crate::domain::{AllowedOrigin, SearchQuery};
use crate::error::ErrorKind;
use url::Url;

fn q(s: &str) -> SearchQuery {
    SearchQuery::parse(s, true).unwrap()
}

#[test]
fn planned_url_clamps_and_encodes() {
    let u = planned_url(&q("serde json"), 0, SortKind::Alphabetical, 0).unwrap();
    assert!(u.as_str().contains("per_page=1"));
    assert!(u.as_str().contains("page=1"));
    assert!(u.as_str().contains("sort=alphabetical"));
    assert!(u.as_str().contains("q=serde"));
    let u2 = planned_url(&q("x"), 500, SortKind::Relevance, 3).unwrap();
    assert!(u2.as_str().contains("per_page=100"));
    assert!(u2.as_str().contains("page=3"));
}

#[test]
fn clamp_search_pagination_page_and_per_page() {
    assert_eq!(clamp_search_pagination(0, 0), (1, 1));
    assert_eq!(clamp_search_pagination(10, 0), (10, 1));
    assert_eq!(clamp_search_pagination(500, 2), (100, 2));
    assert_eq!(clamp_search_pagination(25, 3), (25, 3));
}

#[test]
fn validate_search_pagination_rejects_invalid() {
    assert!(validate_search_pagination(10, 0).is_err());
    assert!(validate_search_pagination(0, 1).is_err());
    assert!(validate_search_pagination(200, 1).is_err());
    assert_eq!(validate_search_pagination(25, 3).unwrap(), (25, 3));
    assert_eq!(validate_search_pagination(100, 1).unwrap(), (100, 1));
}

#[test]
fn parse_fixture_serde() {
    let body = include_str!("../../tests/fixtures/crates_io/search_serde.json");
    let data = parse_search_body(body, "serde", 1, 10, "relevance", false).unwrap();
    assert_eq!(data.hits.len(), 2);
    assert_eq!(data.hits[0].name, "serde");
    assert_eq!(data.hits[0].exact_match, Some(true));
    assert_eq!(data.hits[0].version, "1.0.210");
    assert_eq!(data.hits[1].description, "No description available");
    assert_eq!(data.meta.total, 2);
    assert_eq!(data.meta.next_page.as_deref(), Some("page=2"));
}

#[test]
fn parse_fixture_seek_meta() {
    let body = include_str!("../../tests/fixtures/crates_io/search_seek_meta.json");
    let data = parse_search_body(body, "example", 1, 10, "downloads", false).unwrap();
    assert_eq!(
        data.meta.next_page.as_deref(),
        Some("seek=ABC123&per_page=10")
    );
    assert_eq!(data.meta.prev_page.as_deref(), Some("page=1"));
    assert_eq!(data.hits[0].version, "0.1.0");
}

#[test]
fn parse_invalid_json() {
    let err = parse_search_body("{", "q", 1, 10, "relevance", false).unwrap_err();
    assert_eq!(err.kind(), ErrorKind::Parse);
}

#[test]
fn parse_missing_versions_uses_unknown() {
    let body = r#"{"crates":[{"name":"x","downloads":1}],"meta":{"total":1}}"#;
    let data = parse_search_body(body, "x", 1, 10, "new", false).unwrap();
    assert_eq!(data.hits[0].version, "unknown");
    assert!(data.meta.next_page.is_none());
}

#[test]
fn planned_url_on_host_http_base() {
    let origin = AllowedOrigin::parse_with("http://127.0.0.1:9", true).unwrap();
    let u = planned_url_on_host(&origin, &q("q"), 10, SortKind::New, 1).unwrap();
    assert!(u.as_str().starts_with("http://127.0.0.1:9/api/v1/crates"));
}

#[test]
fn echo_params_from_page_token_url() {
    let fallback = SearchEcho::from_cli(&q(""), 1, 10, SortKind::Relevance);
    let origin = AllowedOrigin::crates_io_default();
    let url = planned_url_with_page_token(
        &origin,
        &q(""),
        10,
        SortKind::Relevance,
        "?q=serde&per_page=2&sort=relevance&page=2",
    )
    .unwrap();
    let echo = echo_params_from_url(&url, &fallback);
    assert_eq!(echo.query, "serde");
    assert_eq!(echo.page, 2);
    assert_eq!(echo.per_page, 2);
    assert_eq!(echo.sort, "relevance");
}

#[test]
fn echo_params_fallback_when_pairs_missing() {
    let fallback = SearchEcho::from_cli(&q("local"), 3, 25, SortKind::Downloads);
    let url = Url::parse("https://crates.io/api/v1/crates?seek=ABC").unwrap();
    let echo = echo_params_from_url(&url, &fallback);
    assert_eq!(echo.query, "local");
    assert_eq!(echo.page, 3);
    assert_eq!(echo.per_page, 25);
    assert_eq!(echo.sort, "downloads");
}

#[test]
fn parse_caps_oversized_api_strings() {
    let long_name = "a".repeat(MAX_CRATE_NAME_CHARS + 50);
    let long_desc = "d".repeat(MAX_CRATE_DESCRIPTION_CHARS + 100);
    let long_url = format!("https://example.com/{}", "p".repeat(MAX_URL_FIELD_CHARS));
    let long_token = "t".repeat(MAX_URL_FIELD_CHARS + 10);
    let body = format!(
        r#"{{"crates":[{{"name":"{long_name}","description":"{long_desc}","downloads":1,"newest_version":"1.0.0","documentation":"{long_url}","repository":"{long_url}","homepage":"{long_url}"}}],"meta":{{"total":1,"next_page":"{long_token}","prev_page":"{long_token}"}}}}"#
    );
    let data = parse_search_body(&body, "q", 1, 10, "relevance", false).unwrap();
    assert_eq!(data.hits[0].name.chars().count(), MAX_CRATE_NAME_CHARS);
    assert_eq!(
        data.hits[0].description.chars().count(),
        MAX_CRATE_DESCRIPTION_CHARS
    );
    assert_eq!(
        data.hits[0]
            .documentation
            .as_ref()
            .map(|s| s.chars().count()),
        Some(MAX_URL_FIELD_CHARS)
    );
    assert_eq!(
        data.meta.next_page.as_ref().map(|s| s.chars().count()),
        Some(MAX_URL_FIELD_CHARS)
    );
    assert_eq!(
        data.meta.prev_page.as_ref().map(|s| s.chars().count()),
        Some(MAX_URL_FIELD_CHARS)
    );
}

#[test]
fn truncate_chars_preserves_short_and_utf8() {
    assert_eq!(truncate_chars("abc", 10), "abc");
    assert_eq!(truncate_chars("abcdef", 3), "abc");
    // Multi-byte: take by char, not byte.
    assert_eq!(truncate_chars("日本語abc", 2), "日本");
}
