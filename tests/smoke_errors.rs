//! Offline smoke tests for argument validation, flag conflicts and exit codes.

mod common;

// Product under test only (absolute CARGO_BIN_EXE). Stdio + env via common.
use common::docsrs_cli_cmd as bin;

#[test]
fn get_item_path_with_space_exit_65() {
    let out = bin()
        .args(["get-item", "clap", "trait", "has space", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "invalid_input");
}

#[test]
fn doctor_missing_config_dir_fails() {
    let out = bin()
        .args([
            "--config-dir",
            "/proc/docsrs-cli-doctor-missing-xyz",
            "--cache-dir",
            "/tmp/docsrs-cli-doctor-cache-ok",
            "doctor",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(78),
        "stdout={}",
        String::from_utf8_lossy(&out.stdout)
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    // GAP-004: top-level ok mirrors data.ok for agent-first health checks.
    assert_eq!(v["ok"], false);
    assert_eq!(v["data"]["ok"], false);
    let checks = v["data"]["checks"].as_array().unwrap();
    let config = checks
        .iter()
        .find(|c| c["name"] == "config_dir")
        .expect("config_dir check");
    assert_eq!(config["ok"], false);
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
    // Force human path on a pipe with --format text (auto-JSON would emit envelope).
    let out = bin()
        .args([
            "--format",
            "text",
            "readme",
            "serde",
            "--crate-version",
            "v1.0.0",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    assert!(
        out.stdout.is_empty(),
        "stdout must be empty on forced human path"
    );
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("error:") || err.contains("erro:"));
}

#[test]
fn auto_json_error_on_pipe() {
    // Non-TTY without --format → JSON error envelope on stdout.
    let out = bin()
        .args(["readme", "serde", "--crate-version", "v1.0.0"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "invalid_input");
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
fn timeout_zero_is_invalid_input_exit_65() {
    // Explicit --timeout 0 is fail-closed (GAP-015); no silent clamp to 1s.
    let out = bin()
        .args(["--timeout", "0", "doctor", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "invalid_input");
    assert_eq!(v["error"]["retryable"], false);
}

#[test]
fn connect_timeout_zero_is_invalid_input_exit_65() {
    let out = bin()
        .args(["--connect-timeout", "0", "version", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "invalid_input");
}

#[test]
fn dry_run_search_crates_page_zero_is_invalid_input() {
    let out = bin()
        .args([
            "search-crates",
            "serde",
            "--page",
            "0",
            "--per-page",
            "10",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "invalid_input");
}

#[test]
fn dry_run_search_crates_per_page_over_max_is_invalid_input() {
    let out = bin()
        .args([
            "search-crates",
            "serde",
            "--per-page",
            "200",
            "--dry-run",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "invalid_input");
}

#[test]
fn clap_invalid_timeout_json_exit_64() {
    let out = bin()
        .args(["--timeout", "abc", "--json", "version"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "usage");
    assert_eq!(v["error"]["code"], 64);
}

#[test]
fn search_in_crate_module_filter_rejected() {
    let out = bin()
        .args([
            "search-in-crate",
            "serde",
            "de",
            "--item-type",
            "module",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "invalid_input");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("get-item"), "msg={msg}");
}

#[test]
fn invalid_lang_fail_closed_exit_65() {
    let out = bin()
        .args(["--lang", "fr", "version", "--json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(65));
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "invalid_input");
    let msg = v["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("unsupported lang"), "msg={msg}");
}
