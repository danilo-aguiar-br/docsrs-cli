//! docs.rs operations: readme, get-item, search-in-crate.
//!
//! # Extraction posture (Rules Rust — web scraping / data extraction)
//!
//! - **Transport** is HTTP GET ([`HttpClient`]); this module only parses bytes.
//! - **Structure** uses `scraper` (html5ever) + CSS selectors — never regex for
//!   field extraction from HTML (version scrape uses path segments on `a[href]`).
//! - **Sanitization** may use process-static regex only to strip `on*` handlers
//!   and `javascript:` URLs after DOM script/style removal (XSS hygiene, not
//!   content discovery).
//! - **Markdown** conversion via `htmd` on the sanitized fragment.
//! - **Provenance:** every success payload includes `source_url` (final URL).
//!   Hit URLs from `all.html` join against that `source_url` base (stdlib /
//!   mock / docs.rs) — never a hardcoded docs.rs host. Absolute hit hrefs must
//!   stay **same-origin** as `source_url` (SCRAPE-S-001); off-origin are skipped.
//! - **Encoding:** UTF-8 only after BOM strip ([`crate::http::decode_utf8`]);
//!   docs.rs / doc.rust-lang.org ship UTF-8. `encoding_rs` multi-charset is OOS.
//! - **Not a crawl:** no link frontier, no sitemap, no RSS/Atom.
//! - **robots.txt:** **PROIBIDO** — this product never fetches, parses, or
//!   enforces REP (operator mandate + ADR 0003). One-shot allowlisted GETs only.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **I/O stage:** HTTPS GET via [`HttpClient`] on multi-thread Tokio workers.
//! - **CPU stage:** HTML scrape + markdown conversion via
//!   [`ConcurrencyBudget::run_cpu_bound`] (`spawn_blocking` + Semaphore).
//! - **Hit scan:** large `all.html` candidate lists use `rayon` after a size
//!   threshold; small lists stay sequential to avoid pool overhead.


mod types;
pub(crate) mod hits;
mod urls;
mod html;
mod fetch;
mod search;

pub use types::{GetItemData, ReadmeData, SearchInCrateData, SearchInCrateHit};
pub use urls::{
    METHOD_PARENT_KIND_NAMES, all_html_url, all_html_url_on_origin, get_item_url,
    get_item_url_on_origin, get_item_url_on_origin_with_parent_kind, is_method_path,
    method_segments_from_parent, pick_unique_type_path, readme_url, readme_url_on_origin,
    strip_crate_prefix_segments,
};
pub use html::{
    extract_item_markdown_from_html, extract_method_markdown_from_html,
    extract_method_markdown_scoped, extract_readme_markdown_from_html,
    list_method_anchor_names, list_method_anchor_names_from_html, sanitize_html_fragment,
    scrub_rustdoc_chrome,
};
pub use fetch::{
    fetch_item, fetch_item_at, fetch_item_at_with_echo, fetch_item_on_origin,
    fetch_item_on_origin_with_echo, fetch_readme, fetch_readme_at, fetch_readme_on_origin,
};
pub use search::{
    join_href, parse_all_html_hits, resolve_hit_url, search_in_crate, search_in_crate_at,
    search_in_crate_from_html, search_in_crate_on_origin,
};

// Test-only reexports of internal helpers used by unit tests below.
#[cfg(test)]
#[allow(unused_imports)]
use html::{
    extract_resolved_version, extract_resolved_version_for_crate, scrape_docs_rs_version_from_html,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{AllowedOrigin, CrateName, MatchMode, SearchQuery, VersionArg};
    use crate::error::ErrorKind;
    use crate::item_kind::ItemKind;
    use url::Url;

    #[test]
    fn readme_url_hyphen() {
        let u = readme_url(&CrateName::parse("async-trait").unwrap(), &VersionArg::parse("latest").unwrap()).unwrap();
        assert_eq!(
            u.as_str(),
            "https://docs.rs/async-trait/latest/async_trait/index.html"
        );
    }

    #[test]
    fn get_item_nested() {
        let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
        let u = get_item_url(&CrateName::parse("tokio").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Struct, &segs).unwrap();
        assert!(u.as_str().contains("/tokio/runtime/struct.Runtime.html"));
    }

    #[test]
    fn get_item_without_crate_prefix() {
        let segs = vec!["Parser".into()];
        let u = get_item_url(&CrateName::parse("clap").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Trait, &segs).unwrap();
        assert!(
            u.as_str().ends_with("/clap/trait.Parser.html")
                || u.path().ends_with("/clap/trait.Parser.html")
        );
    }

    #[test]
    fn get_item_keeps_single_segment_equal_to_crate_name() {
        // Attribute/macro items often share the rustc crate name.
        let segs = vec!["async_trait".into()];
        let u = get_item_url(&CrateName::parse("async-trait").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Attribute, &segs).unwrap();
        assert!(
            u.as_str()
                .ends_with("/async-trait/latest/async_trait/attr.async_trait.html"),
            "url={u}"
        );
    }

    #[test]
    fn get_item_module_single_segment_crate_name_is_root() {
        let segs = vec!["async_trait".into()];
        let u = get_item_url(&CrateName::parse("async-trait").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Module, &segs).unwrap();
        assert!(
            u.as_str()
                .ends_with("/async-trait/latest/async_trait/index.html"),
            "url={u}"
        );
    }

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
    fn sanitize_strips_script() {
        let raw = r#"<div>ok<script>alert(1)</script><p onclick="x()">hi</p></div>"#;
        let s = sanitize_html_fragment(raw);
        assert!(!s.contains("script"));
        assert!(!s.contains("onclick"));
        assert!(s.contains("hi") || s.contains("ok"));
    }

    #[test]
    fn readme_fallback_item_decl_converts() {
        let html = r#"<!DOCTYPE html><html><body class="rustdoc-main">
        <div class="item-decl"><pre>pub use foo;</pre></div>
        </body></html>"#;
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.trim().is_empty());
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
    fn std_core_alloc_urls() {
        let u = readme_url(&CrateName::parse("std").unwrap(), &VersionArg::parse("latest").unwrap()).unwrap();
        assert_eq!(
            u.as_str(),
            "https://doc.rust-lang.org/stable/std/index.html"
        );
        let u = get_item_url(
            &CrateName::parse("std").unwrap(),
            &VersionArg::parse("latest").unwrap(),
            ItemKind::Struct,
            &["option".into(), "Option".into()],
        )
        .unwrap();
        assert_eq!(
            u.as_str(),
            "https://doc.rust-lang.org/stable/std/option/struct.Option.html"
        );
        let a = all_html_url(&CrateName::parse("core").unwrap(), &VersionArg::parse("nightly").unwrap()).unwrap();
        assert_eq!(
            a.as_str(),
            "https://doc.rust-lang.org/nightly/core/all.html"
        );
    }

    #[test]
    fn extract_item_from_fixture() {
        let html = include_str!("../../tests/fixtures/docs_rs/get_item_main.html");
        let (md, empty) = extract_item_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(md.contains("Runtime") || md.contains("Tokio") || md.contains("runtime"));
    }

    #[test]
    fn extract_readme_primary_docblock_fixture() {
        let html = include_str!("../../tests/fixtures/docs_rs/readme_docblock.html");
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.to_ascii_lowercase().contains("alert"));
    }

    #[test]
    fn parse_all_html_hits_honors_cancel_flag() {
        let html = include_str!("../../tests/fixtures/docs_rs/all_html_sample.html");
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
        let html = include_str!("../../tests/fixtures/docs_rs/all_html_sample.html");
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
        let html = include_str!("../../tests/fixtures/docs_rs/all_html_sample.html");
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
        let html = include_str!("../../tests/fixtures/docs_rs/all_html_sample.html");
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
        let std_base =
            Url::parse("https://doc.rust-lang.org/stable/std/all.html").unwrap();
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

        let mock_base =
            Url::parse("http://127.0.0.1:9/demo/1.0.0/demo/all.html").unwrap();
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
    fn method_url_uses_anchor() {
        let segs = vec!["runtime".into(), "Runtime".into(), "new".into()];
        let u = get_item_url(&CrateName::parse("tokio").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Fn, &segs).unwrap();
        assert!(
            u.as_str().contains("struct.Runtime.html#method.new"),
            "url={u}"
        );
    }

    #[test]
    fn stdlib_resolved_version_is_channel() {
        let u = Url::parse("https://doc.rust-lang.org/stable/std/index.html").unwrap();
        assert_eq!(
            extract_resolved_version_for_crate(&u, "latest", Some("std")).as_deref(),
            Some("stable")
        );
    }

    #[test]
    fn resolved_version_from_url() {
        let u = Url::parse("https://docs.rs/serde/1.0.210/serde/index.html").unwrap();
        assert_eq!(
            extract_resolved_version(&u, "latest").as_deref(),
            Some("1.0.210")
        );
        let u2 = Url::parse("https://docs.rs/serde/latest/serde/index.html").unwrap();
        assert!(extract_resolved_version(&u2, "latest").is_none());
    }

    #[test]
    fn scrape_version_prefers_target_crate_not_deps() {
        let html = r#"
        <title>tokio - Rust</title>
        <a href="https://docs.rs/bytes/1.2.0/bytes/">bytes</a>
        <a href="/tokio/1.53.0/tokio/runtime/index.html">runtime</a>
        <a href="https://docs.rs/mio/0.8.0/mio/">mio</a>
        "#;
        assert_eq!(
            scrape_docs_rs_version_from_html(html, "tokio").as_deref(),
            Some("1.53.0")
        );
        // Must not pick dependency when crate path missing
        let html2 = r#"<title>x</title><a href="https://docs.rs/bytes/1.2.0/bytes/">b</a>"#;
        assert_eq!(scrape_docs_rs_version_from_html(html2, "tokio"), None);
    }

    #[test]
    fn module_url_template() {
        let segs = vec!["serde".into(), "de".into()];
        let u = get_item_url(&CrateName::parse("serde").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Module, &segs).unwrap();
        assert!(u.as_str().ends_with("/serde/de/index.html"));
    }

    #[test]
    fn empty_html_readme() {
        let (md, empty) = extract_readme_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }

    #[test]
    fn get_item_empty_segments_errors() {
        let err = get_item_url(&CrateName::parse("clap").unwrap(), &VersionArg::parse("latest").unwrap(), ItemKind::Struct, &[]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
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
        assert!(resolve_hit_url(&base, "http://docs.rs/serde/latest/serde/struct.Error.html").is_none());
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

    #[test]
    fn origin_builders_trim_trailing_slash() {
        let origin = AllowedOrigin::parse("https://docs.rs/").unwrap();
        let u = readme_url_on_origin(
            &origin,
            &CrateName::parse("demo").unwrap(),
            &VersionArg::parse("1.0.0").unwrap(),
        )
        .unwrap();
        assert_eq!(u.as_str(), "https://docs.rs/demo/1.0.0/demo/index.html");
        let a = all_html_url_on_origin(
            &origin,
            &CrateName::parse("demo").unwrap(),
            &VersionArg::parse("1.0.0").unwrap(),
        )
        .unwrap();
        assert_eq!(a.as_str(), "https://docs.rs/demo/1.0.0/demo/all.html");
    }

    #[test]
    fn empty_item_html_extract() {
        let (md, empty) = extract_item_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }

    #[test]
    fn method_extract_scopes_to_details_not_full_page() {
        let html = include_str!("../../tests/fixtures/docs_rs/method_runtime_new.html");
        let (md, empty, scope) = extract_method_markdown_scoped(html, "new").unwrap();
        assert!(!empty);
        assert_eq!(scope, "method");
        assert!(
            !md.contains("parent page noise"),
            "must not dump full parent page: {md}"
        );
        assert!(
            md.to_ascii_lowercase().contains("new") || md.contains("Creates a new runtime"),
            "expected method content: {md}"
        );
        assert!(!md.contains('§'));
        assert!(!md.contains("Copy item path"));
    }

    #[test]
    fn scrub_rustdoc_chrome_strips_section_sign() {
        let dirty = "## [§](#serde)Serde\nCopy item path\nbody";
        let clean = scrub_rustdoc_chrome(dirty);
        assert!(!clean.contains('§'));
        assert!(!clean.contains("Copy item path"));
        assert!(clean.contains("Serde"));
    }

    #[test]
    fn method_missing_anchor_is_not_found() {
        // GAP-X-006 inverted: missing method anchor must fail-closed (not item_page success).
        let html = r#"<html><body><div id="main-content"><h1>Struct Runtime</h1><p>only parent</p></div></body></html>"#;
        let err = extract_method_markdown_scoped(html, "new").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::NotFound);
        assert!(
            err.message().contains("method anchor not found"),
            "message={}",
            err.message()
        );
    }

    #[test]
    fn list_method_anchors_from_fixture() {
        let html = include_str!("../../tests/fixtures/docs_rs/method_runtime_new.html");
        let names = list_method_anchor_names_from_html(html);
        assert!(
            names.iter().any(|n| n == "new"),
            "expected method.new in fixture: {names:?}"
        );
    }

    #[test]
    fn pick_unique_type_path_resolves_runtime() {
        let hits = vec![
            SearchInCrateHit {
                name: "runtime::LocalRuntime".into(),
                kind: "struct".into(),
                url: "u1".into(),
                score: Some(1),
            },
            SearchInCrateHit {
                name: "runtime::Runtime".into(),
                kind: "struct".into(),
                url: "u2".into(),
                score: Some(0),
            },
            SearchInCrateHit {
                name: "runtime::RuntimeFlavor".into(),
                kind: "enum".into(),
                url: "u3".into(),
                score: Some(2),
            },
            SearchInCrateHit {
                name: "task::spawn".into(),
                kind: "fn".into(),
                url: "u4".into(),
                score: Some(0),
            },
        ];
        assert_eq!(
            pick_unique_type_path("Runtime", &hits).as_deref(),
            Some("runtime::Runtime")
        );
        assert_eq!(
            method_segments_from_parent("runtime::Runtime", "new"),
            vec!["runtime".to_string(), "Runtime".to_string(), "new".to_string()]
        );
        assert!(is_method_path(
            ItemKind::Fn,
            &["Runtime".into(), "new".into()]
        ));
        assert!(!is_method_path(ItemKind::Fn, &["spawn".into()]));
    }

    #[test]
    fn pick_unique_type_path_ambiguous_returns_none() {
        let hits = vec![
            SearchInCrateHit {
                name: "a::Client".into(),
                kind: "struct".into(),
                url: "u1".into(),
                score: Some(0),
            },
            SearchInCrateHit {
                name: "b::Client".into(),
                kind: "struct".into(),
                url: "u2".into(),
                score: Some(0),
            },
        ];
        assert!(pick_unique_type_path("Client", &hits).is_none());
    }
}
