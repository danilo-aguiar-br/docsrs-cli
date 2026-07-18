//! Offline dispatch coverage via run_with_io (no network).

use std::io::Cursor;
use std::process::ExitCode;

use docsrs_cli::run_with_io;

async fn run_args(args: &[&str]) -> (ExitCode, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    // stdout_is_terminal=true keeps human markdown default in unit tests.
    let code = run_with_io(
        std::iter::once("docsrs-cli").chain(args.iter().copied()),
        Cursor::new(Vec::new()),
        &mut out,
        &mut err,
        true,
    )
    .await;
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn assert_code(code: ExitCode, expected: u8) {
    assert_eq!(code, ExitCode::from(expected), "expected exit {expected}");
}

#[tokio::test]
async fn version_json_via_lib() {
    let (code, out, _) = run_args(&["version", "--json"]).await;
    assert_code(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["name"], "docsrs-cli");
}

#[tokio::test]
async fn version_text_via_lib() {
    let (code, out, _) = run_args(&["version"]).await;
    assert_code(code, 0);
    assert!(out.contains("docsrs-cli 1.1.0"));
}

#[tokio::test]
async fn doctor_json_and_text() {
    let (code, out, _) = run_args(&["doctor", "--json"]).await;
    assert_code(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["ok"], true);

    let (code, out, _) = run_args(&["doctor"]).await;
    assert_code(code, 0);
    assert!(!out.is_empty());
}

#[tokio::test]
async fn schema_all_commands() {
    for cmd in [
        "search-crates",
        "readme",
        "get-item",
        "search-in-crate",
        "version",
        "doctor",
        "commands",
        "cache",
        "cache-clear",
        "cache-stats",
        "config",
        "config-path",
        "config-show",
        "config-init",
    ] {
        let (code, out, _) = run_args(&["schema", "--cmd", cmd, "--json"]).await;
        assert_code(code, 0);
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["ok"], true, "schema {cmd}");
        assert!(v["data"]["schema"].is_object());
    }
    let (code, out, _) = run_args(&["schema", "--cmd", "version"]).await;
    assert_code(code, 0);
    assert!(out.contains("schema") || out.contains("$schema") || out.contains("properties"));
}

#[tokio::test]
async fn schema_unknown_command() {
    let (code, out, _) = run_args(&["schema", "--cmd", "nope", "--json"]).await;
    assert_code(code, 64);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], false);
}

#[tokio::test]
async fn completions_shells() {
    for shell in ["bash", "zsh", "fish", "elvish", "power-shell", "powershell"] {
        let (code, out, _) = run_args(&["completions", shell]).await;
        assert_code(code, 0);
        assert!(!out.is_empty(), "completions {shell}");
    }
}

#[tokio::test]
async fn completions_json_envelope() {
    let (code, out, _) = run_args(&["completions", "zsh", "--json"]).await;
    assert_code(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "completions");
    assert_eq!(v["data"]["shell"], "zsh");
    assert!(v["data"]["script"].as_str().unwrap().len() > 10);
}

#[tokio::test]
async fn schema_markdown_format() {
    let (code, out, _) = run_args(&["schema", "--cmd", "doctor", "--format", "markdown"]).await;
    assert_code(code, 0);
    assert!(out.contains("# Schema: `doctor`"));
    assert!(out.contains("## Required fields") || out.contains("## Properties"));
    assert!(out.contains("```json"));
}

#[tokio::test]
async fn dry_run_text_and_json_all_ops() {
    let cases: &[&[&str]] = &[
        &["search-crates", "serde", "--dry-run", "--json"],
        &["search-crates", "serde", "--dry-run"],
        &["readme", "tokio", "--dry-run", "--json"],
        &["readme", "tokio", "--dry-run"],
        &["get-item", "clap", "trait", "Parser", "--dry-run", "--json"],
        &["get-item", "clap", "trait", "Parser", "--dry-run"],
        &[
            "search-in-crate",
            "reqwest",
            "Client",
            "--dry-run",
            "--json",
        ],
        &["search-in-crate", "reqwest", "Client", "--dry-run"],
    ];
    for args in cases {
        let (code, out, _) = run_args(args).await;
        assert_code(code, 0);
        assert!(!out.is_empty(), "args={args:?}");
    }
}

#[tokio::test]
async fn invalid_input_json_and_human() {
    let (code, out, _) = run_args(&["readme", "serde", "--crate-version", "v1", "--json"]).await;
    assert_code(code, 65);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], false);

    let (code, out, err) = run_args(&["readme", "serde", "--crate-version", "v1"]).await;
    assert_code(code, 65);
    assert!(
        out.is_empty(),
        "stdout must stay empty on TTY human path without --json"
    );
    assert!(!err.is_empty());
}

#[tokio::test]
async fn commands_tree_json() {
    let (code, out, _) = run_args(&["commands", "--json"]).await;
    assert_code(code, 0);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "commands");
    let cmds = v["data"]["commands"].as_array().unwrap();
    assert!(cmds.iter().any(|c| c["name"] == "search-crates"));
    assert!(cmds.iter().any(|c| c["name"] == "commands"));
    assert!(cmds.iter().any(|c| c["name"] == "get-item"));
    assert!(cmds.iter().any(|c| c["name"] == "config"));
}

#[tokio::test]
async fn auto_json_when_non_tty() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        ["docsrs-cli", "version"],
        Cursor::new(Vec::new()),
        &mut out,
        &mut err,
        false,
    )
    .await;
    assert_code(code, 0);
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out)).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["command"], "version");
}

#[tokio::test]
async fn format_markdown_overrides_auto_json() {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        ["docsrs-cli", "--format", "markdown", "version"],
        Cursor::new(Vec::new()),
        &mut out,
        &mut err,
        false,
    )
    .await;
    assert_code(code, 0);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("docsrs-cli 1.1.0"));
    assert!(!s.trim_start().starts_with('{'));
}

#[tokio::test]
async fn format_conflict_json() {
    let (code, out, _) = run_args(&["search-crates", "serde", "--json", "--format", "text"]).await;
    assert_code(code, 64);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["error"]["kind"], "usage");
}

#[tokio::test]
async fn overrides_timeout_and_quiet() {
    let (code, out, _) = run_args(&[
        "version",
        "--json",
        "--timeout",
        "5",
        "--connect-timeout",
        "2",
        "--rate-limit-delay-ms",
        "0",
        "--max-retries",
        "1",
        "--max-output-bytes",
        "10000",
        "--quiet",
    ])
    .await;
    assert_code(code, 0);
    assert!(out.contains("docsrs-cli"));
}

#[tokio::test]
async fn clap_help_exit_success_or_2() {
    let (code, _out, _err) = run_args(&["--help"]).await;
    assert!(code == ExitCode::SUCCESS || code == ExitCode::from(2));
}

/// Writer that always fails with BrokenPipe (simulates `app | head` consumer exit).
struct BrokenPipeWriter;

impl std::io::Write for BrokenPipeWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "simulated closed pipe",
        ))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn broken_pipe_on_stdout_exits_141() {
    let mut err = Vec::new();
    let code = run_with_io(
        ["docsrs-cli", "version"],
        Cursor::new(Vec::new()),
        BrokenPipeWriter,
        &mut err,
        true,
    )
    .await;
    assert_eq!(
        code,
        ExitCode::from(141),
        "stdout BrokenPipe must map to exit 141 (no panic)"
    );
}
