//! Offline dispatch coverage via run_with_io (no network).

use std::io::Cursor;
use std::process::ExitCode;

use docsrs_cli::run_with_io;

async fn run_args(args: &[&str]) -> (ExitCode, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        std::iter::once("docsrs-cli").chain(args.iter().copied()),
        Cursor::new(Vec::new()),
        &mut out,
        &mut err,
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
    assert!(out.contains("docsrs-cli 0.1.0"));
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
        "cache",
        "cache-clear",
        "cache-stats",
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
    assert!(out.is_empty(), "stdout must stay empty without --json");
    assert!(!err.is_empty());
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
