//! Explicit Target Designation: a destructive verb names its subject in argv.
//!
//! `cache clear` deletes every cached body under the resolved cache root. Until
//! 2026-08-10 it took that root from ambient state — the XDG layer — and the
//! caller never had to name, or even see, the directory being emptied. That is
//! the confused-deputy shape: the caller names the verb, the environment names
//! the victim, and nothing compares the two.
//!
//! The ambient branch is the one that can surprise a caller, so it is the one
//! that must be covered. A branch that reads ambient state and has no test is a
//! branch nobody has ever seen fail.

use std::io::Cursor;
use std::process::ExitCode;

use docsrs_cli::run_with_io;

/// Run one invocation with a private config dir, capturing both streams.
///
/// `--cache-dir` is deliberately NOT injected here: whether the target is named
/// in argv is exactly what each test below varies.
async fn run(args: &[&str]) -> (ExitCode, String, String) {
    let mut argv: Vec<String> = vec!["docsrs-cli".into(), "--quiet".into()];
    argv.extend(args.iter().map(|s| (*s).to_string()));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(argv, Cursor::new(Vec::new()), &mut out, &mut err, false).await;
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

/// An ambient target, unwaived, must fail with usage and delete nothing.
///
/// This is the token-displacement test the ETD rule requires: the environment
/// resolves a root, argv names none, and the parser must refuse rather than act
/// somewhere the caller never pointed at.
#[tokio::test]
async fn cache_clear_refuses_an_ambient_target() {
    let (code, out, _err) = run(&["cache", "clear", "--json"]).await;
    assert_eq!(code, ExitCode::from(64), "out={out}");

    let v: serde_json::Value = serde_json::from_str(&out).expect("error envelope is JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "usage");
    assert_eq!(v["error"]["code"], 64);
    assert_eq!(
        v["error"]["retryable"], false,
        "a refusal to guess is permanent, not transient"
    );

    // The message must name the victim. A refusal that hides which directory was
    // about to be emptied teaches the caller nothing about what to pass.
    let msg = v["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("--cache-dir") && msg.contains("--yes"),
        "the refusal must name both the designation flag and the waiver: {msg}"
    );
}

/// A target named in argv is designated, so the verb runs and says so.
#[tokio::test]
async fn cache_clear_accepts_a_target_named_in_argv() {
    let dir = std::env::temp_dir().join("docsrs-cli-etd-argv");
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.display().to_string();

    let (code, out, _err) = run(&["--cache-dir", &target, "cache", "clear", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");

    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert_eq!(
        v["data"]["target_source"], "cli",
        "argv designated the root, so the envelope must say so"
    );
    assert_eq!(v["data"]["root"], target);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The waiver is explicit and is reported, never silent.
///
/// `--yes` accepts an ambient target on purpose. It must still be visible in the
/// envelope, because an operator auditing a deletion needs to know whether the
/// path came from the command line or from the environment.
#[tokio::test]
async fn the_waiver_is_recorded_in_the_envelope() {
    let dir = std::env::temp_dir().join("docsrs-cli-etd-waived");
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.display().to_string();

    let (code, out, _err) =
        run(&["--cache-dir", &target, "cache", "clear", "--yes", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");

    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert!(
        v["data"]["target_source"].is_string(),
        "every clear envelope carries the provenance of what it deleted"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `config init --force` refuses an ambient target, and writes nothing.
///
/// This is the branch that damaged a real machine. On 2026-08-10 an audit ran
/// `config init --force` with no `--config-dir` to measure the gap and replaced
/// the operator's own `config.toml` with the default template, exit 0, no
/// backup, no waiver, no field in the envelope recording which layer chose the
/// path. The approved plan had named this verb; only its sibling was built.
#[tokio::test]
async fn config_init_force_refuses_an_ambient_target() {
    let (code, out, _err) = run(&["config", "init", "--force", "--json"]).await;
    assert_eq!(code, ExitCode::from(64), "out={out}");

    let v: serde_json::Value = serde_json::from_str(&out).expect("error envelope is JSON");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "usage");
    assert_eq!(v["error"]["code"], 64);

    let msg = v["error"]["message"].as_str().unwrap_or_default();
    assert!(
        msg.contains("--config-dir") && msg.contains("--yes"),
        "the refusal must name both the designation flag and the waiver: {msg}"
    );
    // The harm word is data, not phrasing: this verb replaces a file, and the
    // first version of this message said "delete" for every caller.
    assert!(
        msg.contains("overwrite"),
        "the refusal must name the harm this verb actually does: {msg}"
    );
}

/// A config directory named in argv is designated, so `--force` runs.
#[tokio::test]
async fn config_init_force_accepts_a_target_named_in_argv() {
    let dir = std::env::temp_dir().join("docsrs-cli-etd-config-argv");
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.display().to_string();

    let (code, out, _err) = run(&[
        "--config-dir",
        &target,
        "config",
        "init",
        "--force",
        "--json",
    ])
    .await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");

    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert_eq!(
        v["data"]["target_source"], "cli",
        "argv designated the directory, so the envelope must say so"
    );
    assert_eq!(v["data"]["created"], true);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Without `--force` nothing is destroyed, so no waiver is demanded.
///
/// Copying the ceremony onto the harmless path would teach callers that the
/// waiver is noise, and a waiver treated as noise stops protecting anything.
#[tokio::test]
async fn config_init_without_force_asks_for_no_waiver() {
    let dir = std::env::temp_dir().join("docsrs-cli-etd-config-plain");
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    let target = dir.display().to_string();

    let (code, out, _err) = run(&["--config-dir", &target, "config", "init", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert_eq!(v["data"]["overwritten"], false);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The read-only siblings keep inheriting the ambient root, on purpose.
///
/// ETD binds verbs with side effects. Forcing `--cache-dir` onto `cache stats`
/// would be ceremony with no victim, and copying destructive ergonomics onto a
/// read verb is the mirror of the mistake this suite exists to prevent.
#[tokio::test]
async fn read_only_cache_verbs_still_inherit_the_ambient_root() {
    let (code, out, _err) = run(&["cache", "path", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert!(
        v["data"]["source"].is_string(),
        "cache path reports which layer won without any waiver"
    );

    let (code, out, _err) = run(&["config", "path", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).expect("success envelope is JSON");
    assert!(
        v["data"]["config_source"].is_string(),
        "config path reports which layer won without any waiver"
    );
}
