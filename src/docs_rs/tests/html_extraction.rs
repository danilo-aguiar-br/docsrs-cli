//! Unit tests for HTML sanitization, markdown extraction and version scraping.

use super::super::html::{
    extract_assoc_markdown_scoped_from_document, extract_resolved_version,
    extract_resolved_version_for_crate, scrape_docs_rs_version_from_html,
};
use super::super::*;
use crate::error::ErrorKind;
use url::Url;

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
fn extract_item_from_fixture() {
    let html = include_str!("../../../tests/fixtures/docs_rs/get_item_main.html");
    let (md, empty) = extract_item_markdown_from_html(html).unwrap();
    assert!(!empty);
    assert!(md.contains("Runtime") || md.contains("Tokio") || md.contains("runtime"));
}

#[test]
fn extract_readme_primary_docblock_fixture() {
    let html = include_str!("../../../tests/fixtures/docs_rs/readme_docblock.html");
    let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
    assert!(!empty);
    assert!(!md.to_ascii_lowercase().contains("alert"));
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
fn empty_html_readme() {
    let (md, empty) = extract_readme_markdown_from_html("<html><body></body></html>").unwrap();
    assert!(empty);
    assert!(md.is_empty());
}

#[test]
fn empty_item_html_extract() {
    let (md, empty) = extract_item_markdown_from_html("<html><body></body></html>").unwrap();
    assert!(empty);
    assert!(md.is_empty());
}

#[test]
fn method_extract_scopes_to_details_not_full_page() {
    let html = include_str!("../../../tests/fixtures/docs_rs/method_runtime_new.html");
    let found = extract_method_markdown_scoped(html, "new").unwrap();
    let md = found.markdown;
    assert!(!found.empty);
    assert_eq!(found.scope, "method");
    assert_eq!(found.anchor_id, "method.new");
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

/// Trait page fixture carrying both anchor flavours at once.
const TRAIT_PAGE: &str = include_str!("../../../tests/fixtures/docs_rs/trait_required_method.html");

#[test]
fn required_trait_method_resolves_through_tymethod_anchor() {
    // BUG-01: rustdoc emits `tymethod.` for a body-less trait method. Probing only
    // `method.` turned every required method (Iterator::next, Display::fmt) into a
    // silent HTTP-200 not-found.
    let found = extract_method_markdown_scoped(TRAIT_PAGE, "next").unwrap();
    assert_eq!(found.anchor_id, "tymethod.next");
    assert_eq!(found.scope, "method");
    assert!(!found.empty);
    assert!(
        found.markdown.contains("Advances the iterator"),
        "expected required-method docblock: {}",
        found.markdown
    );
    assert!(
        !found.markdown.contains("parent page noise"),
        "must stay scoped to the anchor: {}",
        found.markdown
    );
}

#[test]
fn provided_trait_method_still_resolves_through_method_anchor() {
    // Control in the very same document: the fix must not trade one prefix for the
    // other. A single-format corpus is what let BUG-01 survive 374 green tests.
    let found = extract_method_markdown_scoped(TRAIT_PAGE, "map").unwrap();
    assert_eq!(found.anchor_id, "method.map");
    assert!(found.markdown.contains("Takes a closure"));
}

#[test]
fn trait_page_lists_required_and_provided_anchors_together() {
    let names = list_method_anchor_names_from_html(TRAIT_PAGE);
    for want in ["next", "map", "size_hint"] {
        assert!(names.iter().any(|n| n == want), "missing {want}: {names:?}");
    }
    // `associatedtype.Item` is not an associated function and must not leak in.
    assert!(!names.iter().any(|n| n == "Item"), "names={names:?}");
}

#[test]
fn unknown_method_on_trait_page_names_both_anchors_tried() {
    let err = extract_method_markdown_scoped(TRAIT_PAGE, "nope").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    // The sentinel prefix is a contract: fetch/recovery match on it to stop the
    // parent-kind probe instead of walking every remaining kind.
    assert!(
        err.message().starts_with(ASSOC_ANCHOR_MISS_PREFIX),
        "message={}",
        err.message()
    );
    assert!(err.message().contains("method.nope"), "{}", err.message());
    assert!(err.message().contains("tymethod.nope"), "{}", err.message());
}

/// Parse the shared trait fixture once per test that needs a document.
fn trait_document() -> scraper::Html {
    scraper::Html::parse_document(TRAIT_PAGE)
}

#[test]
fn associated_type_resolves_through_associatedtype_anchor() {
    // GAP-ASSOCITEM-001: `Iterator::Item` used to build std/iter/Iterator/type.Item.html,
    // a path rustdoc has never emitted. The member lives as an anchor on the parent.
    let found = extract_assoc_markdown_scoped_from_document(
        &trait_document(),
        AssocAnchorKind::Type.anchor_prefixes(),
        "Item",
    )
    .unwrap();
    assert_eq!(found.anchor_id, "associatedtype.Item");
    // `extraction` means "came from the member anchor", not "the member is a fn".
    assert_eq!(found.scope, "method");
    assert!(!found.empty);
    assert!(
        found.markdown.contains("type of the elements"),
        "expected associated-type docblock: {}",
        found.markdown
    );
    assert!(
        !found.markdown.contains("parent page noise"),
        "must stay scoped to the anchor: {}",
        found.markdown
    );
    assert!(
        !found.markdown.contains("impl-block duplicate"),
        "suffixed duplicate must not win: {}",
        found.markdown
    );
}

#[test]
fn associated_constant_resolves_through_associatedconstant_anchor() {
    let found = extract_assoc_markdown_scoped_from_document(
        &trait_document(),
        AssocAnchorKind::Constant.anchor_prefixes(),
        "LIMIT",
    )
    .unwrap();
    assert_eq!(found.anchor_id, "associatedconstant.LIMIT");
    assert!(found.markdown.contains("Upper bound"));
}

#[test]
fn anchor_families_never_bleed_into_each_other() {
    // Each family sees only its own members. A shared list would make
    // `--suggest` offer `next` when the user asked for an associated type.
    let types = list_assoc_anchor_names_from_html(TRAIT_PAGE, AssocAnchorKind::Type);
    assert!(types.iter().any(|n| n == "Item"), "types={types:?}");
    assert!(
        !types.iter().any(|n| n == "next" || n == "map"),
        "types={types:?}"
    );

    let consts = list_assoc_anchor_names_from_html(TRAIT_PAGE, AssocAnchorKind::Constant);
    assert_eq!(consts, vec!["LIMIT".to_string()], "consts={consts:?}");

    let methods = list_assoc_anchor_names_from_html(TRAIT_PAGE, AssocAnchorKind::Method);
    assert!(
        !methods.iter().any(|n| n == "Item" || n == "LIMIT"),
        "methods={methods:?}"
    );
}

#[test]
fn rustdoc_disambiguator_suffixes_collapse_into_one_suggestion() {
    // Rustdoc appends `-<n>` when a member name repeats across impl blocks.
    // Live on std::iter::Iterator this produced Item, Item-1, Item-10, Item-100
    // and Item-101 — four of five suggestions were unusable noise, and the
    // edit-distance ranking preferred them for being near-identical to the typo.
    // A Rust identifier cannot contain `-`, so truncating there is exact.
    let html = r#"<html><body>
        <section id="associatedtype.Item"></section>
        <section id="associatedtype.Item-1"></section>
        <section id="associatedtype.Item-10"></section>
        <section id="associatedtype.Output"></section>
    </body></html>"#;
    let types = list_assoc_anchor_names_from_html(html, AssocAnchorKind::Type);
    assert_eq!(
        types,
        vec!["Item".to_string(), "Output".to_string()],
        "types={types:?}"
    );
}

/// Enum page carrying variants, plus a struct page carrying fields.
///
/// Inline rather than a fixture file: the point is the *anchor id shape*, and a
/// full rustdoc page would bury it under kilobytes of unrelated chrome.
const MEMBER_PAGE: &str = r#"<html><body><div id="main-content">
    <section id="variant.Some"><div class="docblock">Some value of type T.</div></section>
    <section id="variant.None"><div class="docblock">No value.</div></section>
    <section id="structfield.start"><div class="docblock">The lower bound of the range.</div></section>
    <section id="structfield.end"><div class="docblock">The upper bound of the range.</div></section>
    <section id="method.map"><div class="docblock">Maps an Option.</div></section>
</div></body></html>"#;

#[test]
fn variant_resolves_through_its_own_anchor() {
    // GAP-ASSOCITEM-002: `Option::Some` had no route at all — the free-item
    // branch would have built `variant.Some.html`, a page rustdoc never emits.
    let doc = scraper::Html::parse_document(MEMBER_PAGE);
    let found = extract_assoc_markdown_scoped_from_document(
        &doc,
        AssocAnchorKind::Variant.anchor_prefixes(),
        "Some",
    )
    .unwrap();
    assert_eq!(found.anchor_id, "variant.Some");
    assert!(found.markdown.contains("Some value of type T"));
}

#[test]
fn struct_field_resolves_through_its_own_anchor() {
    let doc = scraper::Html::parse_document(MEMBER_PAGE);
    let found = extract_assoc_markdown_scoped_from_document(
        &doc,
        AssocAnchorKind::StructField.anchor_prefixes(),
        "start",
    )
    .unwrap();
    assert_eq!(found.anchor_id, "structfield.start");
    assert!(found.markdown.contains("lower bound"));
}

#[test]
fn variant_and_field_families_stay_isolated() {
    // A shared list would make `--suggest` offer `map` when the user asked for a
    // variant, sending them back into the same not-found.
    let variants = list_assoc_anchor_names_from_html(MEMBER_PAGE, AssocAnchorKind::Variant);
    assert_eq!(variants, vec!["Some".to_string(), "None".to_string()]);

    let fields = list_assoc_anchor_names_from_html(MEMBER_PAGE, AssocAnchorKind::StructField);
    assert_eq!(fields, vec!["start".to_string(), "end".to_string()]);

    let methods = list_assoc_anchor_names_from_html(MEMBER_PAGE, AssocAnchorKind::Method);
    assert_eq!(methods, vec!["map".to_string()]);
}

#[test]
fn unknown_variant_names_only_its_own_family() {
    let doc = scraper::Html::parse_document(MEMBER_PAGE);
    let err = extract_assoc_markdown_scoped_from_document(
        &doc,
        AssocAnchorKind::Variant.anchor_prefixes(),
        "Nope",
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert!(err.message().contains("variant.Nope"), "{}", err.message());
    assert!(!err.message().contains("structfield."), "{}", err.message());
}

#[test]
fn unknown_associated_type_names_the_family_anchor_tried() {
    let err = extract_assoc_markdown_scoped_from_document(
        &trait_document(),
        AssocAnchorKind::Type.anchor_prefixes(),
        "Nope",
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::NotFound);
    assert!(
        err.message().starts_with(ASSOC_ANCHOR_MISS_PREFIX),
        "message={}",
        err.message()
    );
    assert!(
        err.message().contains("associatedtype.Nope"),
        "{}",
        err.message()
    );
    // The method family must not be named in an associated-type miss.
    assert!(!err.message().contains("tymethod."), "{}", err.message());
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
        err.message().starts_with(ASSOC_ANCHOR_MISS_PREFIX),
        "message={}",
        err.message()
    );
}

#[test]
fn list_method_anchors_from_fixture() {
    let html = include_str!("../../../tests/fixtures/docs_rs/method_runtime_new.html");
    let names = list_method_anchor_names_from_html(html);
    assert!(
        names.iter().any(|n| n == "new"),
        "expected method.new in fixture: {names:?}"
    );
}
