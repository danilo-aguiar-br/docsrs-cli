//! Golden render checks from offline fixtures (no network).

use docsrs_cli::crates_io::{CrateSearchHit, SearchCratesData, SearchMeta};
use docsrs_cli::docs_rs::{
    GetItemData, ReadmeData, SearchInCrateData, SearchInCrateHit,
    extract_readme_markdown_from_html, parse_all_html_hits,
};
use docsrs_cli::render::{
    render_item_markdown, render_readme_markdown, render_search_in_crate_markdown,
    render_search_markdown, truncate_output,
};

#[test]
fn golden_search_markdown_contains_sections() {
    let data = SearchCratesData {
        query: "serde".into(),
        page: 1,
        per_page: 10,
        sort: "relevance".into(),
        hits: vec![CrateSearchHit {
            name: "serde".into(),
            description: "A generic serialization framework".into(),
            downloads: 1,
            version: "1.0.0".into(),
            documentation: Some("https://docs.rs/serde".into()),
            max_version: Some("1.0.0".into()),
            max_stable_version: None,
            default_version: None,
            recent_downloads: None,
            exact_match: Some(true),
            yanked: Some(false),
            repository: None,
            homepage: None,
        }],
        meta: SearchMeta {
            total: 1,
            next_page: None,
            prev_page: None,
        },
    };
    let md = render_search_markdown(&data);
    assert!(md.contains("# Crate Search Results"));
    assert!(md.contains("## serde (1.0.0)"));
    assert!(md.contains("A generic serialization framework"));
}

#[test]
fn golden_readme_from_fixture() {
    let html = include_str!("fixtures/docs_rs/readme_docblock.html");
    let (md_body, empty) = extract_readme_markdown_from_html(html).unwrap();
    assert!(!empty);
    let data = ReadmeData {
        crate_name: "demo".into(),
        version: "latest".into(),
        resolved_version: None,
        markdown: md_body,
        empty: false,
        truncated: false,
        source_url: "https://docs.rs/demo/latest/demo/index.html".into(),
    };
    let md = render_readme_markdown(&data);
    assert!(md.contains("# demo Documentation"));
    assert!(!md.contains("<script>"));
}

#[test]
fn golden_item_markdown() {
    let data = GetItemData {
        crate_name: "tokio".into(),
        item_type: "struct".into(),
        item_path: "tokio::runtime::Runtime".into(),
        version: "latest".into(),
        resolved_version: Some("1.40.0".into()),
        markdown: "The Tokio runtime.".into(),
        empty: false,
        truncated: false,
        source_url: "https://docs.rs/tokio/latest/tokio/runtime/struct.Runtime.html".into(),
        title: "tokio::runtime::Runtime (struct)".into(),
    };
    let md = render_item_markdown(&data);
    assert!(md.contains("# tokio::runtime::Runtime (struct)"));
    assert!(md.contains("Documentation URL"));
}

#[test]
fn golden_search_in_crate_from_fixture() {
    let html = include_str!("fixtures/docs_rs/all_html_sample.html");
    let hits = parse_all_html_hits(html, "demo", "1.0.0", "Client", None, 10).unwrap();
    let data = SearchInCrateData {
        crate_name: "demo".into(),
        query: "Client".into(),
        version: "1.0.0".into(),
        total: hits.len(),
        emitted: hits.len(),
        hits: hits
            .into_iter()
            .map(|h| SearchInCrateHit {
                name: h.name,
                kind: h.kind,
                url: h.url,
            })
            .collect(),
        truncated: false,
        source_url: "https://docs.rs/demo/1.0.0/demo/all.html".into(),
    };
    let md = render_search_in_crate_markdown(&data);
    assert!(md.contains("Client"));
    assert!(md.contains("Found"));
}

#[test]
fn truncation_sets_flag_on_readme_render_path() {
    let long = "x".repeat(100);
    let (cut, trunc) = truncate_output(&long, 10);
    assert!(trunc);
    assert_eq!(cut.len(), 10);
}
