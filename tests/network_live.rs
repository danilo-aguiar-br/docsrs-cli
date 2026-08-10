//! Live network tests (opt-in; **ignored** by default so `cargo test` is honest).
//!
//! Run with:
//! ```text
//! cargo test --locked --test network_live -- --ignored
//! ```
//!
//! `#[ignore]` is the only gate, and that is deliberate. These tests used to
//! carry a second gate reading `DOCSRS_CLI_NETWORK_TESTS`, which made
//! `cargo test -- --ignored` return early from every test and report them as
//! passed without opening a single socket — a suite that proved nothing while
//! looking green. It also put an environment read in the repository, which the
//! product forbids; the anti-env gate in `scripts/check-policy.sh` never saw it
//! because that scan only walked `src/`.
//!
//! The default suite still never opens external sockets: `cargo test` skips
//! everything here.

mod common;

use std::process::Command;
use std::time::Duration;

fn bin() -> Command {
    // Product under test only (absolute CARGO_BIN_EXE). Stdio + env via common.
    // Respect crawler policy: modest delay already in defaults / run_json flags.
    common::docsrs_cli_cmd()
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
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_search_crates_serde() {
    let (code, v, _) = run_json(&["search-crates", "serde", "--per-page", "5"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert!(!v["data"]["hits"].as_array().unwrap().is_empty());
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_readme_tokio() {
    let (code, v, _) = run_json(&["readme", "tokio"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["empty"], false);
    assert!(!v["data"]["markdown"].as_str().unwrap_or("").is_empty());
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_readme_async_trait_hyphen() {
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
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_get_item_clap_parser() {
    let (code, v, _) = run_json(&["get-item", "clap", "trait", "clap::Parser"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["empty"], false);
    std::thread::sleep(Duration::from_millis(1100));
}

#[test]
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_search_in_crate_reqwest_client() {
    let (code, v, _) = run_json(&["search-in-crate", "reqwest", "Client", "--limit", "20"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["ok"], true);
    let hits = v["data"]["hits"].as_array().unwrap();
    assert!(hits.iter().any(|h| h["name"] == "Client"));
}

#[test]
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_stdlib_readme_std() {
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
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_stdlib_get_item_option() {
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
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_stdlib_readme_core() {
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
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_stdlib_search_in_crate_option() {
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
        // SCRAPE-R-003: hit URLs must join against source_url (stdlib host), not rewrite to docs.rs.
        for h in hits {
            let url = h["url"].as_str().unwrap_or("");
            assert!(
                url.starts_with("https://doc.rust-lang.org/"),
                "stdlib hit must stay on doc.rust-lang.org: {url}"
            );
            assert!(
                !url.contains("docs.rs"),
                "stdlib hit must not rewrite host to docs.rs: {url}"
            );
        }
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

/// Every anchor family reports itself in `anchor_family`, while `extraction`
/// keeps its documented value.
///
/// The pair is the point. `extraction` answers "did the markdown come from the
/// member anchor?" and says `method` for all six families, which is why an
/// agent cannot use it to tell a variant from a function. `Iterator::next` is
/// the sharpest case in the table: it is a *required trait method*, so the page
/// defines `#tymethod.next` and never `#method.next` — `extraction` still reads
/// `method` while the family is `tymethod`.
///
/// An item with its own page (`struct.Runtime.html`) has no member anchor, so
/// both fields are absent rather than null.
#[test]
#[ignore = "live network: opens external sockets; run with `cargo test -- --ignored`"]
fn live_anchor_family_matches_the_matched_anchor() {
    let cases = [
        (
            ["get-item", "std", "variant", "option::Option::Some"],
            "variant",
        ),
        (
            ["get-item", "std", "structfield", "ops::Range::start"],
            "structfield",
        ),
        (
            ["get-item", "std", "type", "iter::Iterator::Item"],
            "associatedtype",
        ),
        (
            ["get-item", "std", "const", "time::Duration::MAX"],
            "associatedconstant",
        ),
        (
            ["get-item", "std", "method", "iter::Iterator::next"],
            "tymethod",
        ),
    ];
    for (args, expected_family) in cases {
        let (code, v, _) = run_json(&args);
        assert_eq!(code, 0, "{args:?} -> {v}");
        assert_eq!(
            v["data"]["anchor_family"], expected_family,
            "{args:?} must report anchor_family={expected_family}: {v}"
        );
        assert_eq!(
            v["data"]["extraction"], "method",
            "{args:?}: extraction stays `method` for every family (wire contract)"
        );
        std::thread::sleep(Duration::from_millis(1100));
    }

    // An item page carries no member anchor: both fields are omitted.
    let (code, v, _) = run_json(&["get-item", "tokio", "struct", "runtime::Runtime"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        v["data"]["anchor_family"].is_null(),
        "item pages must omit anchor_family: {v}"
    );
    assert!(
        v["data"]["extraction"].is_null(),
        "item pages must omit extraction: {v}"
    );
}
