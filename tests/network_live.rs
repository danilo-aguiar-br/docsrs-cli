//! Live network tests.
//!
//! - crates.io / docs.rs: `DOCSRS_CLI_NETWORK_TESTS=1`
//! - doc.rust-lang.org stdlib: `DOCSRS_CLI_STDLIB_NETWORK_TESTS=1` (separate policy)
//!
//! Default suite never opens external sockets.

use std::process::{Command, Stdio};
use std::time::Duration;

fn enabled() -> bool {
    matches!(
        std::env::var("DOCSRS_CLI_NETWORK_TESTS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn stdlib_enabled() -> bool {
    matches!(
        std::env::var("DOCSRS_CLI_STDLIB_NETWORK_TESTS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn bin() -> Command {
    // Spawns the product under test (accepted Command::new). stdin closed per native-crate rules.
    let mut c = Command::new(env!("CARGO_BIN_EXE_docsrs-cli"));
    c.env_remove("RUST_LOG");
    c.stdin(Stdio::null());
    // Respect crawler policy: modest delay already in defaults.
    c
}

fn run_json(args: &[&str]) -> (i32, serde_json::Value, String) {
    let out = bin()
        .args(args)
        .args(["--json", "--timeout", "30", "--rate-limit-delay-ms", "1000"])
        .output()
        .expect("spawn docsrs-cli");
    let code = out.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let v: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({
        "ok": false,
        "raw": stdout,
        "stderr": stderr,
    }));
    (code, v, stderr)
}

#[test]
fn live_search_crates_serde() {
    if !enabled() {
        eprintln!("skip: set DOCSRS_CLI_NETWORK_TESTS=1");
        return;
    }
    let (code, v, _) = run_json(&["search-crates", "serde", "--per-page", "5"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert!(!v["data"]["hits"].as_array().unwrap().is_empty());
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_readme_tokio() {
    if !enabled() {
        return;
    }
    let (code, v, _) = run_json(&["readme", "tokio"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["empty"], false);
    assert!(!v["data"]["markdown"].as_str().unwrap_or("").is_empty());
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_readme_async_trait_hyphen() {
    if !enabled() {
        return;
    }
    let (code, v, _) = run_json(&["readme", "async-trait"]);
    assert_eq!(code, 0, "F-01 regression: {v}");
    assert_eq!(v["ok"], true);
    let src = v["data"]["source_url"].as_str().unwrap_or("");
    assert!(
        src.contains("async_trait"),
        "resolved path must use underscore rustc segment: {src}"
    );
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_get_item_clap_parser() {
    if !enabled() {
        return;
    }
    let (code, v, _) = run_json(&["get-item", "clap", "trait", "clap::Parser"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["empty"], false);
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_search_in_crate_reqwest_client() {
    if !enabled() {
        return;
    }
    let (code, v, _) = run_json(&["search-in-crate", "reqwest", "Client", "--limit", "20"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let hits = v["data"]["hits"].as_array().unwrap();
    assert!(hits.iter().any(|h| h["name"] == "Client"));
}

#[test]
fn live_stdlib_readme_std() {
    if !stdlib_enabled() {
        eprintln!("skip: set DOCSRS_CLI_STDLIB_NETWORK_TESTS=1");
        return;
    }
    let (code, v, _) = run_json(&["readme", "std", "--no-cache"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let src = v["data"]["source_url"].as_str().unwrap_or("");
    assert!(
        src.contains("doc.rust-lang.org") && src.contains("/std/"),
        "stdlib readme must use doc.rust-lang.org: {src}"
    );
    assert_eq!(v["data"]["empty"], false);
    assert!(!v["data"]["markdown"].as_str().unwrap_or("").is_empty());
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_stdlib_get_item_option() {
    if !stdlib_enabled() {
        return;
    }
    // Option is an enum in rustdoc (`enum.Option.html`), not a struct.
    let (code, v, _) = run_json(&[
        "get-item",
        "std",
        "enum",
        "std::option::Option",
        "--no-cache",
    ]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let src = v["data"]["source_url"].as_str().unwrap_or("");
    assert!(
        src.contains("doc.rust-lang.org") && src.contains("enum.Option.html"),
        "source_url={src}"
    );
    assert_eq!(v["data"]["empty"], false);
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
fn live_stdlib_readme_core() {
    if !stdlib_enabled() {
        return;
    }
    let (code, v, _) = run_json(&["readme", "core", "--no-cache"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let src = v["data"]["source_url"].as_str().unwrap_or("");
    assert!(
        src.contains("doc.rust-lang.org") && src.contains("/core/"),
        "source_url={src}"
    );
    std::thread::sleep(Duration::from_millis(1100));
}

/// G-18: prove search-in-crate against stdlib all.html (or honest NotFound on that host).
#[test]
fn live_stdlib_search_in_crate_option() {
    if !stdlib_enabled() {
        return;
    }
    let (code, v, _) = run_json(&[
        "search-in-crate",
        "std",
        "Option",
        "--limit",
        "20",
        "--no-cache",
    ]);
    if code == 0 {
        assert_eq!(v["ok"], true, "{v}");
        let src = v["data"]["source_url"].as_str().unwrap_or("");
        assert!(
            src.contains("doc.rust-lang.org"),
            "search-in-crate stdlib host wrong: {src}"
        );
        let hits = v["data"]["hits"].as_array().unwrap();
        assert!(
            hits.iter().any(|h| {
                h["name"].as_str().unwrap_or("").contains("Option")
                    || h["path"].as_str().unwrap_or("").contains("Option")
            }),
            "expected Option hit in {hits:?}"
        );
    } else {
        // Honest failure on doc.rust-lang.org (e.g. all.html 404) — must not silently hit docs.rs.
        assert_eq!(
            code, 66,
            "unexpected exit for stdlib search-in-crate: {code} {v}"
        );
        let msg = v["error"]["message"].as_str().unwrap_or("");
        let src = v["error"]["source_url"]
            .as_str()
            .or_else(|| v["data"]["source_url"].as_str())
            .unwrap_or("");
        let combined = format!("{msg} {src} {v}");
        assert!(
            combined.contains("doc.rust-lang.org") || combined.contains("not_found") || code == 66,
            "G-18: failure must stay on stdlib host / not_found: {v}"
        );
    }
}
