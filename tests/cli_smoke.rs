//! Offline smoke tests for CLI surface.

use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_docsrs-cli"));
    c.env_remove("RUST_LOG");
    c
}

#[test]
fn version_json() {
    let out = bin().args(["version", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "docsrs-cli");
    assert_eq!(v["data"]["version"], "0.1.0");
    assert_eq!(v["data"]["msrv"], "1.88.0");
    assert!(v.get("schema_version").is_some());
    assert!(v.get("duration_ms").is_some());
    assert!(v.get("timestamp").is_none());
}

#[test]
fn version_text() {
    let out = bin().args(["version"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("docsrs-cli 0.1.0"));
}

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
fn dry_run_std_core_alloc() {
    for crate_name in ["std", "core", "alloc"] {
        let out = bin()
            .args(["readme", crate_name, "--dry-run", "--json"])
            .output()
            .unwrap();
        assert!(out.status.success(), "{crate_name}");
        let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        let url = v["data"]["planned_url"].as_str().unwrap();
        assert!(url.contains(&format!("/{crate_name}/latest/{crate_name}/index.html")));
    }
}

#[test]
fn json_format_conflict_exit_64() {
    let out = bin()
        .args(["search-crates", "serde", "--json", "--format", "text"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "usage");
}

#[test]
fn human_error_keeps_stdout_empty() {
    let out = bin()
        .args(["readme", "serde", "--crate-version", "v1.0.0"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    assert!(out.stdout.is_empty(), "stdout must be empty without --json");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("error:") || err.contains("erro:"));
}

#[test]
fn invalid_version_prefix() {
    let out = bin()
        .args(["readme", "serde", "--crate-version", "v1.0.0", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
}

#[test]
fn invalid_item_type_exit_65() {
    let out = bin()
        .args(["get-item", "clap", "notakind", "Parser", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "invalid_input");
}

#[test]
fn empty_item_path_exit_65() {
    let out = bin()
        .args(["get-item", "clap", "trait", "::", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
}

#[test]
fn doctor_ok() {
    let out = bin().args(["doctor", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["ok"], true);
}

#[test]
fn schema_get_item_not_stub() {
    let out = bin()
        .args(["schema", "--cmd", "get-item", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let schema = &v["data"]["schema"];
    assert!(schema["properties"]["crate_name"].is_object());
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .iter()
            .any(|x| x == "title")
    );
}

#[test]
fn schema_search_crates_has_sort_enum() {
    let out = bin()
        .args(["schema", "--cmd", "search-crates"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("alphabetical"));
    assert!(s.contains("recent-downloads"));
}

#[test]
fn completions_bash() {
    let out = bin().args(["completions", "bash"]).output().unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("docsrs-cli") || s.contains("_docsrs"));
}

#[test]
fn cache_stats_and_clear_json() {
    let dir = tempfile::tempdir().unwrap();
    let cache = dir.path().join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let out = bin()
        .args([
            "cache",
            "stats",
            "--json",
            "--cache-dir",
            cache.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "cache-stats");
    assert_eq!(v["data"]["entries"], 0);

    let out = bin()
        .args([
            "cache",
            "clear",
            "--json",
            "--cache-dir",
            cache.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "cache-clear");
    assert_eq!(v["data"]["removed_entries"], 0);
}

#[test]
fn completions_powershell_alias_and_canonical() {
    for shell in ["powershell", "power-shell"] {
        let out = bin().args(["completions", shell]).output().unwrap();
        assert!(
            out.status.success(),
            "completions {shell} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let s = String::from_utf8_lossy(&out.stdout);
        assert!(
            s.contains("docsrs-cli") || s.contains("Register-ArgumentCompleter"),
            "empty or unexpected completions for {shell}"
        );
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
fn dry_run_search_crates_page_zero_clamps_params_and_url() {
    let out = bin()
        .args([
            "search-crates",
            "serde",
            "--page",
            "0",
            "--per-page",
            "0",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["planned_params"]["page"], 1);
    assert_eq!(v["data"]["planned_params"]["per_page"], 1);
    let url = v["data"]["planned_url"].as_str().unwrap_or("");
    assert!(url.contains("page=1"), "url={url}");
    assert!(url.contains("per_page=1"), "url={url}");
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
