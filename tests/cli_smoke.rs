//! Offline smoke tests for CLI surface.

mod common;

use std::process::Command;

fn bin() -> Command {
    // Product under test only (absolute CARGO_BIN_EXE). Stdio + env via common.
    common::docsrs_cli_cmd()
}

#[test]
fn version_json() {
    let out = bin().args(["version", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "docsrs-cli");
    assert_eq!(v["data"]["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(v["data"]["msrv"], "1.88.0");
    assert!(v.get("schema_version").is_some());
    assert!(v.get("duration_ms").is_some());
    assert!(v.get("timestamp").is_none());
}

#[test]
fn version_text() {
    // `.output()` pipes stdout (non-TTY) so JSON is auto; force human with --format text.
    let out = bin()
        .args(["--format", "text", "version"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains(concat!("docsrs-cli ", env!("CARGO_PKG_VERSION"))));
}

#[test]
fn version_auto_json_on_pipe() {
    // Subprocess `.output()` is non-TTY → agent-first auto JSON without --json.
    let out = bin().args(["version"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "version");
    assert_eq!(v["data"]["name"], "docsrs-cli");
}

#[test]
fn commands_tree_json() {
    let out = bin().args(["commands", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "commands");
    let names: Vec<&str> = v["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    assert!(names.contains(&"search-crates"));
    assert!(names.contains(&"commands"));
    assert!(names.contains(&"schema"));
    assert!(names.contains(&"config"));
    let config = v["data"]["commands"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "config")
        .expect("config command");
    let subs: Vec<&str> = config["subcommands"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|s| s["name"].as_str())
        .collect();
    assert!(subs.contains(&"path"));
    assert!(subs.contains(&"show"));
    assert!(subs.contains(&"init"));
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
fn schema_format_markdown_documents_fields() {
    let out = bin()
        .args(["schema", "--cmd", "search-crates", "--format", "markdown"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("# Schema:"), "{s}");
    assert!(s.contains("## Required fields"), "{s}");
    assert!(s.contains("## Properties"), "{s}");
    assert!(s.contains("`hits`"), "{s}");
    assert!(s.contains("`meta`"), "{s}");
    assert!(s.contains("## JSON Schema"), "{s}");
    assert!(s.contains("```json"), "{s}");
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
fn doctor_ok() {
    let out = bin().args(["doctor", "--json"]).output().unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["data"]["ok"], true);
    let checks = v["data"]["checks"].as_array().unwrap();
    for name in [
        "config_source",
        "config_file",
        "cache_source",
        "dotenv_runtime",
        "secrets_layers",
    ] {
        let c = checks
            .iter()
            .find(|c| c["name"] == name)
            .unwrap_or_else(|| panic!("missing doctor check {name}"));
        assert_eq!(c["ok"], true, "check {name} detail={}", c["detail"]);
    }
    let dotenv = checks
        .iter()
        .find(|c| c["name"] == "dotenv_runtime")
        .unwrap();
    assert!(
        dotenv["detail"]
            .as_str()
            .unwrap()
            .contains("no .env required"),
        "detail={}",
        dotenv["detail"]
    );
}

#[test]
fn config_path_show_init_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_dir = dir.path().join("cfg");
    let cache_dir = dir.path().join("cache");
    std::fs::create_dir_all(&cfg_dir).unwrap();
    std::fs::create_dir_all(&cache_dir).unwrap();

    let path_out = bin()
        .args([
            "--config-dir",
            cfg_dir.to_str().unwrap(),
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "config",
            "path",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        path_out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&path_out.stderr)
    );
    let path_v: serde_json::Value = serde_json::from_slice(&path_out.stdout).unwrap();
    assert_eq!(path_v["command"], "config-path");
    assert_eq!(path_v["data"]["config_source"], "cli");
    assert_eq!(path_v["data"]["cache_source"], "cli");
    assert_eq!(path_v["data"]["config_file_exists"], false);
    assert_eq!(path_v["data"]["dotenv_runtime"], false);

    let init_out = bin()
        .args([
            "--config-dir",
            cfg_dir.to_str().unwrap(),
            "config",
            "init",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(
        init_out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&init_out.stderr)
    );
    let init_v: serde_json::Value = serde_json::from_slice(&init_out.stdout).unwrap();
    assert_eq!(init_v["command"], "config-init");
    assert_eq!(init_v["data"]["created"], true);
    assert!(cfg_dir.join("config.toml").is_file());

    let show_out = bin()
        .args([
            "--config-dir",
            cfg_dir.to_str().unwrap(),
            "--cache-dir",
            cache_dir.to_str().unwrap(),
            "config",
            "show",
            "--json",
        ])
        .output()
        .unwrap();
    assert!(show_out.status.success());
    let show_v: serde_json::Value = serde_json::from_slice(&show_out.stdout).unwrap();
    assert_eq!(show_v["command"], "config-show");
    assert_eq!(show_v["data"]["config_toml_loaded"], true);
    assert_eq!(show_v["data"]["config_path_source"], "cli");

    let again = bin()
        .args([
            "--config-dir",
            cfg_dir.to_str().unwrap(),
            "config",
            "init",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(78));
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

#[test]
fn completions_json_envelope() {
    let out = bin()
        .args(["completions", "bash", "--json"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "completions");
    assert_eq!(v["data"]["shell"], "bash");
    let script = v["data"]["script"].as_str().unwrap_or("");
    assert!(!script.is_empty());
    assert!(
        script.contains("docsrs-cli") || script.contains("_docsrs"),
        "script should look like bash completion"
    );
}

#[test]
fn schema_format_markdown_wraps_json_schema() {
    let out = bin()
        .args(["schema", "--cmd", "version", "--format", "markdown"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("# Schema: `version`"), "out={s}");
    assert!(s.contains("## Required fields"), "out={s}");
    assert!(s.contains("## JSON Schema"), "out={s}");
    assert!(s.contains("```json"), "out={s}");
    assert!(s.contains("```"), "out={s}");
    assert!(s.contains("properties") || s.contains("$schema") || s.contains("{"));
}
