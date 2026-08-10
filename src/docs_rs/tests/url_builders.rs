//! Unit tests for docs.rs URL construction and method-path resolution.

use super::super::*;
use crate::domain::{AllowedOrigin, CrateName, VersionArg};
use crate::error::ErrorKind;
use crate::item_kind::ItemKind;

#[test]
fn readme_url_hyphen() {
    let u = readme_url(
        &CrateName::parse("async-trait").unwrap(),
        &VersionArg::parse("latest").unwrap(),
    )
    .unwrap();
    assert_eq!(
        u.as_str(),
        "https://docs.rs/async-trait/latest/async_trait/index.html"
    );
}

#[test]
fn get_item_nested() {
    let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
    let u = get_item_url(
        &CrateName::parse("tokio").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Struct,
        &segs,
    )
    .unwrap();
    assert!(u.as_str().contains("/tokio/runtime/struct.Runtime.html"));
}

#[test]
fn get_item_without_crate_prefix() {
    let segs = vec!["Parser".into()];
    let u = get_item_url(
        &CrateName::parse("clap").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Trait,
        &segs,
    )
    .unwrap();
    assert!(
        u.as_str().ends_with("/clap/trait.Parser.html")
            || u.path().ends_with("/clap/trait.Parser.html")
    );
}

#[test]
fn get_item_keeps_single_segment_equal_to_crate_name() {
    // Attribute/macro items often share the rustc crate name.
    let segs = vec!["async_trait".into()];
    let u = get_item_url(
        &CrateName::parse("async-trait").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Attribute,
        &segs,
    )
    .unwrap();
    assert!(
        u.as_str()
            .ends_with("/async-trait/latest/async_trait/attr.async_trait.html"),
        "url={u}"
    );
}

#[test]
fn get_item_module_single_segment_crate_name_is_root() {
    let segs = vec!["async_trait".into()];
    let u = get_item_url(
        &CrateName::parse("async-trait").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Module,
        &segs,
    )
    .unwrap();
    assert!(
        u.as_str()
            .ends_with("/async-trait/latest/async_trait/index.html"),
        "url={u}"
    );
}

#[test]
fn std_core_alloc_urls() {
    let u = readme_url(
        &CrateName::parse("std").unwrap(),
        &VersionArg::parse("latest").unwrap(),
    )
    .unwrap();
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
    let a = all_html_url(
        &CrateName::parse("core").unwrap(),
        &VersionArg::parse("nightly").unwrap(),
    )
    .unwrap();
    assert_eq!(
        a.as_str(),
        "https://doc.rust-lang.org/nightly/core/all.html"
    );
}

#[test]
fn method_url_uses_anchor() {
    let segs = vec!["runtime".into(), "Runtime".into(), "new".into()];
    let u = get_item_url(
        &CrateName::parse("tokio").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Fn,
        &segs,
    )
    .unwrap();
    assert!(
        u.as_str().contains("struct.Runtime.html#method.new"),
        "url={u}"
    );
}

#[test]
fn module_url_template() {
    let segs = vec!["serde".into(), "de".into()];
    let u = get_item_url(
        &CrateName::parse("serde").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Module,
        &segs,
    )
    .unwrap();
    assert!(u.as_str().ends_with("/serde/de/index.html"));
}

#[test]
fn get_item_empty_segments_errors() {
    let err = get_item_url(
        &CrateName::parse("clap").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Struct,
        &[],
    )
    .unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
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
        vec![
            "runtime".to_string(),
            "Runtime".to_string(),
            "new".to_string()
        ]
    );
    assert!(is_method_path(
        ItemKind::Fn,
        &["Runtime".into(), "new".into()]
    ));
    assert!(!is_method_path(ItemKind::Fn, &["spawn".into()]));
}

/// Build a `std` item URL from a `::`-separated path.
fn std_url(kind: ItemKind, path: &str) -> String {
    let segs: Vec<String> = path.split("::").map(str::to_string).collect();
    get_item_url(
        &CrateName::parse("std").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        kind,
        &segs,
    )
    .unwrap()
    .to_string()
}

#[test]
fn associated_type_targets_the_parent_trait_anchor() {
    // GAP-ASSOCITEM-001: this used to build std/iter/Iterator/type.Item.html,
    // a path rustdoc has never emitted in any version.
    assert_eq!(
        std_url(ItemKind::Type, "iter::Iterator::Item"),
        "https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#associatedtype.Item"
    );
    assert_eq!(
        std_url(ItemKind::Type, "ops::Deref::Target"),
        "https://doc.rust-lang.org/stable/std/ops/trait.Deref.html#associatedtype.Target"
    );
}

#[test]
fn lowercase_leaf_still_reaches_the_parent_page() {
    // GAP-ASSOCLEAF-001: a leaf-case guard sent this to the free-item branch and
    // built std/iter/Iterator/type.item.html — the same impossible shape the
    // family fix was supposed to retire. The parent is uppercase, so the member
    // belongs on the parent page and the miss can list the real members.
    assert_eq!(
        std_url(ItemKind::Type, "iter::Iterator::item"),
        "https://doc.rust-lang.org/stable/std/iter/trait.Iterator.html#associatedtype.item"
    );
    assert_eq!(
        std_url(ItemKind::Constant, "time::Duration::max"),
        "https://doc.rust-lang.org/stable/std/time/struct.Duration.html#associatedconstant.max"
    );
}

#[test]
fn associated_constant_targets_the_parent_struct_anchor() {
    assert_eq!(
        std_url(ItemKind::Constant, "time::Duration::MAX"),
        "https://doc.rust-lang.org/stable/std/time/struct.Duration.html#associatedconstant.MAX"
    );
}

#[test]
fn primitive_constants_keep_the_legacy_free_item_page() {
    // REGRESSION GUARD: both of these resolve today. `u32` and `f32` are
    // lowercase primitives, so the uppercase-parent rule keeps them off the
    // associated-item path and on the module page std still serves.
    assert_eq!(
        std_url(ItemKind::Constant, "u32::MAX"),
        "https://doc.rust-lang.org/stable/std/u32/constant.MAX.html"
    );
    assert_eq!(
        std_url(ItemKind::Constant, "f32::EPSILON"),
        "https://doc.rust-lang.org/stable/std/f32/constant.EPSILON.html"
    );
}

#[test]
fn method_anchor_is_unchanged_by_the_family_generalization() {
    // Control: widening detection to types and constants must not move methods.
    assert_eq!(
        std_url(ItemKind::Fn, "iter::Iterator::next"),
        "https://doc.rust-lang.org/stable/std/iter/struct.Iterator.html#method.next"
    );
}

#[test]
fn associated_item_url_on_docs_rs_origin() {
    let segs: Vec<String> = vec!["de".into(), "Deserializer".into(), "Error".into()];
    let u = get_item_url_on_origin(
        &AllowedOrigin::docs_rs_default(),
        &CrateName::parse("serde").unwrap(),
        &VersionArg::parse("latest").unwrap(),
        ItemKind::Type,
        &segs,
    )
    .unwrap();
    assert_eq!(
        u.as_str(),
        "https://docs.rs/serde/latest/serde/de/trait.Deserializer.html#associatedtype.Error"
    );
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
