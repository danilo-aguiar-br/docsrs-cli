//! Offline smoke tests for `--dry-run` URL and parameter planning.

mod common;

// Product under test only (absolute CARGO_BIN_EXE). Stdio + env via common.
use common::docsrs_cli_cmd as bin;

#[test]
fn dry_run_async_trait_path() {
    let out = bin()
        .args(["readme", "async-trait", "--dry-run", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["dry_run"], true);
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert_eq!(
        url,
        "https://docs.rs/async-trait/latest/async_trait/index.html"
    );
}

#[test]
fn dry_run_get_item_nested() {
    let out = bin()
        .args([
            "get-item",
            "tokio",
            "struct",
            "tokio::runtime::Runtime",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(url.contains("/tokio/runtime/struct.Runtime.html"));
}

#[test]
fn dry_run_get_item_slash_path() {
    let out = bin()
        .args([
            "get-item",
            "tokio",
            "struct",
            "runtime/Runtime",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(
        url.contains("/tokio/runtime/struct.Runtime.html"),
        "url={url}"
    );
}

#[test]
fn dry_run_get_item_attr_same_as_crate_name() {
    let out = bin()
        .args([
            "get-item",
            "async-trait",
            "attribute",
            "async_trait",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(
        url.ends_with("/async_trait/attr.async_trait.html"),
        "url={url}"
    );
}

#[test]
fn dry_run_search_in_crate() {
    let out = bin()
        .args([
            "search-in-crate",
            "reqwest",
            "Client",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(url.ends_with("/reqwest/all.html") || url.contains("/reqwest/latest/reqwest/all.html"));
}

#[test]
fn dry_run_std_core_alloc_uses_doc_rust_lang_org() {
    // std/core/alloc are served from doc.rust-lang.org (not docs.rs).
    for crate_name in ["std", "core", "alloc"] {
        let out = bin()
            .args(["readme", crate_name, "--dry-run", "--json"])
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{crate_name} stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let url = v["data"]["planned_url"].as_str().unwrap_or("");
        assert!(
            url.contains("doc.rust-lang.org") && url.contains(crate_name),
            "{crate_name} url={url}"
        );
        assert!(url.contains("/stable/"), "{crate_name} url={url}");
    }
}

#[test]
fn dry_run_search_crates_alphabetical() {
    let out = bin()
        .args([
            "search-crates",
            "serde",
            "--sort",
            "alphabetical",
            "--page",
            "2",
            "--per-page",
            "5",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["planned_params"]["sort"], "alphabetical");
    assert_eq!(v["data"]["planned_params"]["page"], 2);
    assert_eq!(v["data"]["planned_params"]["per_page"], 5);
}

#[test]
fn function_alias_dry_run() {
    let out = bin()
        .args([
            "get-item",
            "reqwest",
            "function",
            "get",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["planned_params"]["item_type"], "fn");
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(url.contains("fn.get.html"));
}

#[test]
fn constant_item_type_uses_constant_file_prefix() {
    let out = bin()
        .args([
            "get-item",
            "libc",
            "constant",
            "_SC_OPEN_MAX",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["planned_params"]["item_type"], "constant");
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(
        url.contains("constant._SC_OPEN_MAX.html"),
        "expected modern constant. prefix, url={url}"
    );
    assert!(
        !url.contains("const._SC_OPEN_MAX.html"),
        "legacy const. prefix must not be planned, url={url}"
    );
}

#[test]
fn const_alias_also_plans_constant_prefix() {
    let out = bin()
        .args(["get-item", "libc", "const", "MAX", "--dry-run", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let url = v["data"]["planned_url"].as_str().unwrap();
    assert!(url.contains("constant.MAX.html"), "url={url}");
}

#[test]
fn stdlib_missing_item_still_exit_66_from_doc_rust_lang_org() {
    // Unknown item on stdlib host returns not_found (network path uses doc.rust-lang.org).
    let out = bin()
        .args([
            "get-item",
            "std",
            "struct",
            "ThisDoesNotExistAnywhere123",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let url = v["data"]["planned_url"].as_str().unwrap_or("");
    assert!(url.contains("doc.rust-lang.org/stable/std/"), "url={url}");
}
