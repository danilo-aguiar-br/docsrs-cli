//! End-to-end offline dispatch: wiremock origins + run_with_io (covers lib.rs live paths).

use std::io::Cursor;
use std::process::ExitCode;

use docsrs_cli::run_with_io;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn allow_localhost() {
    unsafe {
        std::env::set_var("DOCSRS_CLI_ALLOW_LOCALHOST", "1");
    }
}

async fn run_with_config(
    config_dir: &std::path::Path,
    args: &[&str],
) -> (ExitCode, String, String) {
    let mut argv: Vec<String> = vec![
        "docsrs-cli".into(),
        "--config-dir".into(),
        config_dir.display().to_string(),
        "--rate-limit-delay-ms".into(),
        "0".into(),
        "--timeout".into(),
        "10".into(),
        "--quiet".into(),
    ];
    argv.extend(args.iter().map(|s| (*s).to_string()));
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(argv, Cursor::new(Vec::new()), &mut out, &mut err).await;
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn write_origin_config(dir: &std::path::Path, origin: &str) {
    let body = format!(
        "crates_io_origin = \"{origin}\"\ndocs_rs_origin = \"{origin}\"\nrate_limit_delay_ms = 0\nmax_retries = 1\n"
    );
    std::fs::write(dir.join("config.toml"), body).unwrap();
}

#[tokio::test]
async fn e2e_search_crates_json_and_markdown() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .and(query_param("q", "serde"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(include_str!("fixtures/crates_io/search_serde.json")),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(dir.path(), &["search-crates", "serde", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["hits"][0]["name"], "serde");

    let (code, out, _) = run_with_config(dir.path(), &["search-crates", "serde"]).await;
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(out.contains("serde") || out.contains("Crate Search"));
}

#[tokio::test]
async fn e2e_readme_json_and_markdown() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/latest/demo/index.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(include_str!("fixtures/docs_rs/readme_docblock.html")),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(dir.path(), &["readme", "demo", "--json"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["crate_name"], "demo");
    assert_eq!(v["data"]["empty"], false);

    let (code, out, _) = run_with_config(dir.path(), &["readme", "demo"]).await;
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(out.contains("demo") || out.contains("Documentation"));
}

#[tokio::test]
async fn e2e_get_item_json() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/tokio/latest/tokio/runtime/struct.Runtime.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(include_str!("fixtures/docs_rs/get_item_main.html")),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(
        dir.path(),
        &[
            "get-item",
            "tokio",
            "struct",
            "tokio::runtime::Runtime",
            "--json",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["item_type"], "struct");
    assert_eq!(v["data"]["item_path"], "tokio::runtime::Runtime");
}

#[tokio::test]
async fn e2e_search_in_crate_json_and_markdown() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/demo/1.0.0/demo/all.html"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(include_str!("fixtures/docs_rs/all_html_sample.html")),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(
        dir.path(),
        &[
            "search-in-crate",
            "demo",
            "Client",
            "--crate-version",
            "1.0.0",
            "--item-type",
            "struct",
            "--json",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["hits"][0]["name"], "Client");
    assert_eq!(v["data"]["truncated"], false);

    let (code, out, _) = run_with_config(
        dir.path(),
        &[
            "search-in-crate",
            "demo",
            "",
            "--crate-version",
            "1.0.0",
            "--limit",
            "2",
            "--json",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["emitted"], 2);
    assert_eq!(v["data"]["truncated"], true);
    assert!(v["data"]["total"].as_u64().unwrap() > 2);

    let (code, out, _) = run_with_config(
        dir.path(),
        &[
            "search-in-crate",
            "demo",
            "Client",
            "--crate-version",
            "1.0.0",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS);
    assert!(out.contains("Client"));
}

#[tokio::test]
async fn e2e_readme_404_json_error() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing/latest/missing/index.html"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(dir.path(), &["readme", "missing", "--json"]).await;
    assert_eq!(code, ExitCode::from(66), "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "not_found");
}

#[tokio::test]
async fn e2e_get_item_mismatched_prefix_still_works() {
    // Covers warn branch for path_crate differing from crate_name with underscore/hyphen.
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/async-trait/latest/async_trait/trait.AsyncTrait.html",
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/html")
                .set_body_string(include_str!("fixtures/docs_rs/get_item_main.html")),
        )
        .mount(&server)
        .await;

    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());

    let (code, out, _) = run_with_config(
        dir.path(),
        &[
            "get-item",
            "async-trait",
            "trait",
            "async_trait::AsyncTrait",
            "--json",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["ok"], true);
}

#[tokio::test]
async fn e2e_lang_pt_br_human_error() {
    allow_localhost();
    let dir = tempfile::tempdir().unwrap();
    // Invalid version triggers human error without network.
    let (code, out, err) = run_with_config(
        dir.path(),
        &[
            "--lang",
            "pt-BR",
            "readme",
            "serde",
            "--crate-version",
            "v1",
        ],
    )
    .await;
    assert_eq!(code, ExitCode::from(65));
    assert!(out.is_empty());
    assert!(err.contains("erro:"), "err={err}");
}

#[tokio::test]
async fn e2e_cli_overrides_and_doctor_fail() {
    allow_localhost();
    let dir = tempfile::tempdir().unwrap();
    // Doctor fails when user_agent lacks APP_NAME
    std::fs::write(
        dir.path().join("config.toml"),
        "user_agent = \"not-the-product\"\n",
    )
    .unwrap();
    let (code, out, _) = run_with_config(dir.path(), &["doctor", "--json"]).await;
    assert_eq!(code, ExitCode::from(78), "out={out}");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["data"]["ok"], false);

    let (code, out, _) = run_with_config(dir.path(), &["doctor"]).await;
    assert_eq!(code, ExitCode::from(78));
    assert!(out.contains("fail") || out.contains("doctor"));
}

#[tokio::test]
async fn e2e_full_cli_overrides_version() {
    allow_localhost();
    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), "https://docs.rs");
    let argv = vec![
        "docsrs-cli".into(),
        "--config-dir".into(),
        dir.path().display().to_string(),
        "--user-agent".into(),
        "docsrs-cli/0.1.0 (test@example.com)".into(),
        "--max-body-bytes".into(),
        "1000000".into(),
        "--max-output-bytes".into(),
        "500000".into(),
        "--max-retries".into(),
        "1".into(),
        "--connect-timeout".into(),
        "3".into(),
        "--verbose".into(),
        "--no-color".into(),
        "version".into(),
        "--json".into(),
    ];
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(argv, Cursor::new(Vec::new()), &mut out, &mut err).await;
    assert_eq!(code, ExitCode::SUCCESS);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("docsrs-cli"));
}

#[tokio::test]
async fn e2e_schema_text_and_all_cmds() {
    allow_localhost();
    let dir = tempfile::tempdir().unwrap();
    for cmd in ["readme", "get-item", "search-in-crate", "doctor"] {
        let (code, out, _) = run_with_config(dir.path(), &["schema", "--cmd", cmd]).await;
        assert_eq!(code, ExitCode::SUCCESS, "cmd={cmd}");
        assert!(out.contains("properties") || out.contains("$schema") || out.contains("{"));
    }
}

#[tokio::test]
async fn e2e_search_empty_hits_markdown() {
    allow_localhost();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v1/crates"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(r#"{"crates":[],"meta":{"total":0}}"#),
        )
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    write_origin_config(dir.path(), &server.uri());
    let (code, out, _) = run_with_config(dir.path(), &["search-crates", "zzzznope"]).await;
    assert_eq!(code, ExitCode::SUCCESS, "out={out}");
    assert!(out.contains("No crates found") || out.contains("Search"));
}

#[tokio::test]
async fn e2e_vv_verbose_dry_run() {
    allow_localhost();
    let dir = tempfile::tempdir().unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run_with_io(
        [
            "docsrs-cli",
            "--config-dir",
            &dir.path().display().to_string(),
            "-vv",
            "readme",
            "tokio",
            "--dry-run",
            "--json",
        ],
        Cursor::new(Vec::new()),
        &mut out,
        &mut err,
    )
    .await;
    assert_eq!(code, ExitCode::SUCCESS);
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("dry_run") || s.contains("planned_url"));
}
