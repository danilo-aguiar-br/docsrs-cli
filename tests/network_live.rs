//! Live network tests — opt-in via DOCSRS_CLI_NETWORK_TESTS=1.
//!
//! Default suite never opens external sockets.

use std::process::Command;
use std::time::Duration;

fn enabled() -> bool {
    matches!(
        std::env::var("DOCSRS_CLI_NETWORK_TESTS").as_deref(),
        Ok("1") | Ok("true") | Ok("yes")
    )
}

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_docsrs-cli"));
    c.env_remove("RUST_LOG");
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
