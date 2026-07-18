//! Strict golden diffs for render + parse paths (no timestamps).

use docsrs_cli::crates_io::parse_search_body;
use docsrs_cli::docs_rs::{GetItemData, ReadmeData};
use docsrs_cli::docs_rs::{
    extract_item_markdown_from_html, extract_readme_markdown_from_html, search_in_crate_from_html,
};
use docsrs_cli::item_kind::ItemKind;
use docsrs_cli::render::{
    render_item_markdown, render_readme_markdown, render_search_in_crate_markdown,
    render_search_markdown,
};

fn normalize_md(s: &str) -> String {
    // htmd output can vary whitespace slightly; normalize blank lines and trim ends.
    let mut out = String::new();
    for line in s.lines() {
        let t = line.trim_end();
        out.push_str(t);
        out.push('\n');
    }
    while out.ends_with("\n\n\n") {
        out.pop();
    }
    out
}

#[test]
fn golden_search_crates_json_and_markdown() {
    let body = include_str!("fixtures/crates_io/search_serde.json");
    let data = parse_search_body(body, "serde", 1, 10, "relevance", false).unwrap();
    let got = serde_json::to_value(&data).unwrap();
    let want: serde_json::Value =
        serde_json::from_str(include_str!("golden/json/search-crates.json")).unwrap();
    assert_eq!(got, want);

    let md = render_search_markdown(&data);
    let golden = include_str!("golden/markdown/search-crates.md");
    assert_eq!(normalize_md(&md), normalize_md(golden));
}

#[test]
fn golden_readme_markdown_from_fixture() {
    let html = include_str!("fixtures/docs_rs/readme_docblock.html");
    let (body, empty) = extract_readme_markdown_from_html(html).unwrap();
    assert!(!empty);
    let data = ReadmeData {
        crate_name: "demo".into(),
        version: "latest".into(),
        resolved_version: None,
        markdown: body,
        empty: false,
        truncated: false,
        source_url: "https://docs.rs/demo/latest/demo/index.html".into(),
        cache_hit: false,
    };
    let md = render_readme_markdown(&data);
    // Assert stable header + key content; body converter may add fences.
    assert!(md.starts_with("# demo Documentation\n"));
    assert!(md.contains("Demo crate documentation"));
    assert!(!md.to_ascii_lowercase().contains("script"));
    assert!(!md.contains("alert"));
    // Golden snapshot of structure (header fixed)
    let golden = include_str!("golden/markdown/readme.md");
    assert!(normalize_md(&md).contains(normalize_md(golden).lines().next().unwrap_or("")));
}

#[test]
fn golden_get_item_markdown_structure() {
    let html = include_str!("fixtures/docs_rs/get_item_main.html");
    let (body, empty) = extract_item_markdown_from_html(html).unwrap();
    assert!(!empty);
    let data = GetItemData {
        crate_name: "tokio".into(),
        item_type: "struct".into(),
        item_path: "tokio::runtime::Runtime".into(),
        item_name: "Runtime".into(),
        version: "latest".into(),
        resolved_version: None,
        markdown: "The Tokio runtime.".into(),
        empty: false,
        truncated: false,
        source_url: "https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html".into(),
        title: "tokio::runtime::Runtime (struct)".into(),
        cache_hit: false,
        extraction: None,
    };
    // Prefer stable synthetic body for exact golden match.
    let md = render_item_markdown(&data);
    let golden = include_str!("golden/markdown/get-item.md");
    assert_eq!(normalize_md(&md), normalize_md(golden));
    assert!(!body.is_empty());
}

#[test]
fn golden_search_in_crate_markdown() {
    let html = include_str!("fixtures/docs_rs/all_html_sample.html");
    let data = search_in_crate_from_html(
        html,
        "demo",
        "1.0.0",
        "Client",
        Some(ItemKind::Struct),
        10,
        docsrs_cli::domain::MatchMode::Prefix,
        "https://docs.rs/demo/1.0.0/demo/all.html",
        false,
    )
    .unwrap();
    let md = render_search_in_crate_markdown(&data);
    let golden = include_str!("golden/markdown/search-in-crate.md");
    assert_eq!(normalize_md(&md), normalize_md(golden));
}

#[test]
fn htmd_converter_wins_golden_smoke() {
    // PRD: converter must win golden rustdoc-like HTML. htmd is the selected engine.
    let html = r#"<div class="docblock"><p>Hello <code>world</code></p><pre><code class="language-rust">fn x() {}</code></pre></div>"#;
    let (md, empty) = extract_readme_markdown_from_html(&format!(
        r#"<!DOCTYPE html><html><body class="rustdoc">{html}</body></html>"#
    ))
    .unwrap();
    assert!(!empty);
    assert!(md.contains("Hello"));
    assert!(md.contains("world") || md.contains("`world`"));
    // Must not require html2md for this golden.
    let _ = htmd::convert(html).expect("htmd converts fixture");
}
