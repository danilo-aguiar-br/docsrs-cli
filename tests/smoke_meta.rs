//! Offline smoke tests for the meta CLI surface.
//!
//! Covers `version`, `commands`, `schema`, `completions`, `doctor`, `config`
//! and `cache` — everything that inspects the binary instead of the network.

mod common;

// Product under test only (absolute CARGO_BIN_EXE). Stdio + env via common.
use common::docsrs_cli_cmd as bin;

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
