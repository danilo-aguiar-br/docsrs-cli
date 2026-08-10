//! Unit tests for `all.html` hit scanning, href joining and same-origin policy.

use super::super::*;
use crate::domain::{CrateName, MatchMode, SearchQuery, VersionArg};
use crate::item_kind::ItemKind;
use url::Url;

#[test]
fn join_relative() {
    let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
    let j = join_href(&base, "de/trait.Deserialize.html").unwrap();
    assert!(j.contains("/serde/de/trait.Deserialize.html"));
}

#[test]
fn join_absolute_path_and_fragment() {
    let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
    let j = join_href(&base, "/serde/latest/serde/struct.Error.html").unwrap();
    assert!(j.starts_with("https://docs.rs/serde/"));
    let f = join_href(&base, "struct.Error.html#method.to_string").unwrap();
    assert!(f.contains("#method.to_string"));
    // Absolute-path join must preserve the base origin (stdlib / mock hosts).
    let std_base = Url::parse("https://doc.rust-lang.org/stable/std/").unwrap();
    let std_j = join_href(&std_base, "/stable/std/option/struct.Option.html").unwrap();
    assert!(
        std_j.starts_with("https://doc.rust-lang.org/"),
        "must not hardcode docs.rs for absolute paths: {std_j}"
    );
    assert!(std_j.contains("struct.Option.html"));
}

#[test]
fn all_html_order_stable() {
    let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="struct.A.html">A</a>
          <a href="trait.B.html">B</a>
          <a href="fn.c.html">c</a>
          <a href="union.U.html">U</a>
          <a href="attr.x.html">x</a>
          <a href="derive.D.html">D</a>
        </div></body></html>"#;
    let base = Url::parse("https://docs.rs/demo/1.0.0/demo/all.html").unwrap();
    let hits = parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("", true).unwrap(),
        None,
        100,
        crate::domain::MatchMode::Prefix,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(hits.len(), 6);
    assert_eq!(hits[0].name, "A");
    assert_eq!(hits[0].kind, "struct");
    assert_eq!(hits[3].kind, "union");
    assert_eq!(hits[4].kind, "attribute");
    assert_eq!(hits[5].kind, "derive");
}

#[test]
fn parse_all_html_hits_honors_cancel_flag() {
    let html = include_str!("../../../tests/fixtures/docs_rs/all_html_sample.html");
    let base = url::Url::parse("https://docs.rs/demo/1.0.0/demo/all.html").unwrap();
    let cancel = crate::shutdown::CancelFlag::new();
    cancel.cancel();
    let err = parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("", true).unwrap(),
        None,
        100,
        MatchMode::Prefix,
        &cancel,
    )
    .expect_err("cancelled flag must abort scrape");
    assert!(
        matches!(
            err.kind(),
            crate::error::ErrorKind::Interrupted | crate::error::ErrorKind::Terminated
        ),
        "kind={:?}",
        err.kind()
    );
}

#[test]
fn search_in_crate_from_html_limit_and_filter() {
    let html = include_str!("../../../tests/fixtures/docs_rs/all_html_sample.html");
    let data = search_in_crate_from_html(
        html,
        &CrateName::parse("demo").unwrap(),
        &VersionArg::parse("1.0.0").unwrap(),
        &SearchQuery::parse("", true).unwrap(),
        Some(ItemKind::Struct),
        1,
        MatchMode::Prefix,
        "https://docs.rs/demo/1.0.0/demo/all.html",
        false,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(data.total, 1);
    assert_eq!(data.emitted, 1);
    assert_eq!(data.hits[0].kind, "struct");
    assert!(!data.truncated);
}

#[test]
fn search_in_crate_truncated_when_limit_cuts_hits() {
    let html = include_str!("../../../tests/fixtures/docs_rs/all_html_sample.html");
    let data = search_in_crate_from_html(
        html,
        &CrateName::parse("demo").unwrap(),
        &VersionArg::parse("1.0.0").unwrap(),
        &SearchQuery::parse("", true).unwrap(),
        None,
        2,
        MatchMode::Prefix,
        "https://docs.rs/demo/1.0.0/demo/all.html",
        false,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert!(data.total > 2, "fixture must have more than 2 hits");
    assert_eq!(data.emitted, 2);
    assert!(data.truncated);
    let full = search_in_crate_from_html(
        html,
        &CrateName::parse("demo").unwrap(),
        &VersionArg::parse("1.0.0").unwrap(),
        &SearchQuery::parse("", true).unwrap(),
        None,
        1000,
        MatchMode::Prefix,
        "https://docs.rs/demo/1.0.0/demo/all.html",
        false,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(full.total, full.emitted);
    assert!(!full.truncated);
}

#[test]
fn search_in_crate_limit_zero_emits_nothing() {
    let html = include_str!("../../../tests/fixtures/docs_rs/all_html_sample.html");
    let data = search_in_crate_from_html(
        html,
        &CrateName::parse("demo").unwrap(),
        &VersionArg::parse("1.0.0").unwrap(),
        &SearchQuery::parse("", true).unwrap(),
        None,
        0,
        MatchMode::Prefix,
        "https://docs.rs/demo/1.0.0/demo/all.html",
        false,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert!(data.total > 0, "fixture must have hits for total");
    assert_eq!(data.emitted, 0);
    assert!(data.hits.is_empty());
    assert!(data.truncated);
}

#[test]
fn serialize_exact_match_beats_deserializer() {
    let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="trait.Deserialize.html">Deserialize</a>
          <a href="trait.Serialize.html">Serialize</a>
          <a href="struct.Deserializer.html">Deserializer</a>
        </div></body></html>"#;
    let base = Url::parse("https://docs.rs/serde/1.0.0/serde/all.html").unwrap();
    let hits = parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("Serialize", false).unwrap(),
        Some(ItemKind::Trait),
        10,
        MatchMode::Prefix,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(hits[0].name, "Serialize");
    assert!(hits.iter().all(|h| h.name == "Serialize"));
}

#[test]
fn hit_urls_follow_source_url_host_stdlib_and_mock() {
    // SCRAPE-Q-001/008: never hardcode docs.rs when joining relative hrefs.
    let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="option/enum.Option.html">option::Option</a>
        </div></body></html>"#;
    let std_base = Url::parse("https://doc.rust-lang.org/stable/std/all.html").unwrap();
    let std_hits = parse_all_html_hits(
        html,
        &std_base,
        &SearchQuery::parse("Option", false).unwrap(),
        None,
        10,
        MatchMode::Prefix,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(std_hits.len(), 1);
    assert!(
        std_hits[0].url.starts_with("https://doc.rust-lang.org/"),
        "stdlib hit must stay on doc.rust-lang.org: {}",
        std_hits[0].url
    );
    assert!(
        !std_hits[0].url.contains("docs.rs"),
        "stdlib hit must not rewrite host to docs.rs: {}",
        std_hits[0].url
    );

    let mock_base = Url::parse("http://127.0.0.1:9/demo/1.0.0/demo/all.html").unwrap();
    let mock_html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="struct.Client.html">Client</a>
        </div></body></html>"#;
    let mock_hits = parse_all_html_hits(
        mock_html,
        &mock_base,
        &SearchQuery::parse("Client", false).unwrap(),
        None,
        10,
        MatchMode::Prefix,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(mock_hits.len(), 1);
    assert!(
        mock_hits[0].url.starts_with("http://127.0.0.1:9/"),
        "mock hit must preserve loopback origin: {}",
        mock_hits[0].url
    );
}

#[test]
fn join_same_origin_absolute_ok_off_origin_skipped() {
    // SCRAPE-S-001: absolute same-host OK; cross-origin soft-skipped (not passthrough).
    let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
    let full = "https://docs.rs/serde/latest/serde/struct.Error.html";
    assert_eq!(resolve_hit_url(&base, full).as_deref(), Some(full));
    assert_eq!(join_href(&base, full).unwrap(), full);
    let http = "http://example.invalid/x.html";
    assert!(resolve_hit_url(&base, http).is_none());
    assert!(join_href(&base, http).is_err());
    // https evil host also skipped
    assert!(resolve_hit_url(&base, "https://evil.example/struct.Foo.html").is_none());
    // scheme mismatch (http vs https) on same host label is off-origin
    assert!(
        resolve_hit_url(&base, "http://docs.rs/serde/latest/serde/struct.Error.html").is_none()
    );
}

#[test]
fn poison_absolute_off_origin_href_soft_skipped_in_search() {
    // SCRAPE-S-001: one poison absolute href must not fail the search or appear in hits.
    let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="https://evil.example/struct.Poison.html">Poison</a>
          <a href="struct.Client.html">Client</a>
        </div></body></html>"#;
    let base = Url::parse("https://docs.rs/demo/1.0.0/demo/all.html").unwrap();
    // Substring "i" matches both names so poison would be a hit without same-origin skip.
    let hits = parse_all_html_hits(
        html,
        &base,
        &SearchQuery::parse("i", false).unwrap(),
        None,
        10,
        MatchMode::Substring,
        &crate::shutdown::CancelFlag::new(),
    )
    .unwrap();
    assert_eq!(hits.len(), 1, "only same-origin Client: {hits:?}");
    assert_eq!(hits[0].name, "Client");
    assert!(
        hits[0].url.starts_with("https://docs.rs/"),
        "hit must stay on base host: {}",
        hits[0].url
    );
    assert!(
        hits.iter().all(|h| !h.url.contains("evil.example")),
        "poison host must not appear: {hits:?}"
    );
}
