//! Offline integration tests for the agent-native payload reduction knobs.
//!
//! Every case uses a meta command (`doctor`, `commands`, `version`, `schema`) so the
//! suite never opens a socket. `doctor` (without `--online`) exposes `data.checks`,
//! an array of objects — the shape the reduction pipeline targets.

mod common;

use std::process::Command;

fn bin() -> Command {
    common::docsrs_cli_cmd()
}

/// Run argv offline and return `(exit_code, parsed_stdout)`.
fn run_json(args: &[&str]) -> (i32, serde_json::Value) {
    let out = bin().args(args).output().expect("spawn product binary");
    let code = out.status.code().expect("exit code");
    let value = serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null);
    (code, value)
}

fn checks(v: &serde_json::Value) -> &Vec<serde_json::Value> {
    v["data"]["checks"].as_array().expect("data.checks array")
}

#[test]
fn select_projects_only_requested_keys() {
    let (_, v) = run_json(&["doctor", "--json", "--select", "name,ok"]);
    assert_eq!(v["ok"], true);
    for c in checks(&v) {
        let obj = c.as_object().expect("check object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["name", "ok"], "detail must be projected away");
    }
}

#[test]
fn fields_is_an_alias_of_select() {
    let (_, a) = run_json(&["doctor", "--json", "--select", "name"]);
    let (_, b) = run_json(&["doctor", "--json", "--fields", "name"]);
    assert_eq!(checks(&a).len(), checks(&b).len());
    assert_eq!(checks(&b)[0].as_object().expect("object").len(), 1);
}

#[test]
fn select_skips_missing_key_instead_of_emitting_null() {
    let (_, v) = run_json(&["doctor", "--json", "--select", "name,no_such_field"]);
    for c in checks(&v) {
        let obj = c.as_object().expect("check object");
        assert!(obj.contains_key("name"));
        assert!(
            !obj.contains_key("no_such_field"),
            "absent key must be skipped, never emitted as null"
        );
        assert!(obj.values().all(|x| !x.is_null()));
    }
}

#[test]
fn filter_substring_narrows_the_list() {
    let (_, all) = run_json(&["doctor", "--json"]);
    let (_, filtered) = run_json(&["doctor", "--json", "--filter", "name~platform"]);
    let hits = checks(&filtered);
    assert!(!hits.is_empty(), "at least the `platform` check must match");
    assert!(hits.len() < checks(&all).len(), "filter must reduce");
    assert!(
        hits.iter()
            .all(|c| c["name"].as_str().expect("name").contains("platform"))
    );
}

#[test]
fn filter_equality_and_negation() {
    let (_, eq) = run_json(&["doctor", "--json", "--filter", "name=platform"]);
    assert_eq!(checks(&eq).len(), 1);
    assert_eq!(checks(&eq)[0]["name"], "platform");

    let (_, ne) = run_json(&["doctor", "--json", "--filter", "name!=platform"]);
    assert!(
        checks(&ne)
            .iter()
            .all(|c| c["name"].as_str() != Some("platform"))
    );
}

#[test]
fn repeated_filters_conjoin_with_and() {
    let (_, v) = run_json(&[
        "doctor",
        "--json",
        "--filter",
        "name~platform",
        "--filter",
        "ok=true",
    ]);
    for c in checks(&v) {
        assert!(c["name"].as_str().expect("name").contains("platform"));
        assert_eq!(c["ok"], true);
    }
}

#[test]
fn malformed_filter_exits_65() {
    let out = bin()
        .args(["doctor", "--json", "--filter", "no_operator_here"])
        .output()
        .expect("spawn");
    assert_eq!(
        out.status.code(),
        Some(65),
        "invalid_input, never a silent empty set"
    );
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("error envelope json");
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["kind"], "invalid_input");
    assert_eq!(v["error"]["code"], 65);
}

#[test]
fn dedupe_by_removes_repeated_values() {
    // Every doctor check has a unique name, so dedupe must be a no-op there…
    let (_, all) = run_json(&["doctor", "--json"]);
    let (_, deduped) = run_json(&["doctor", "--json", "--dedupe-by", "name"]);
    assert_eq!(checks(&deduped).len(), checks(&all).len());

    // …while deduping on a low-cardinality field collapses the list.
    let (_, by_ok) = run_json(&["doctor", "--json", "--dedupe-by", "ok"]);
    assert!(checks(&by_ok).len() <= 2, "ok is boolean: at most two rows");
    assert!(!checks(&by_ok).is_empty());
}

#[test]
fn dedupe_keeps_elements_missing_the_key() {
    let (_, v) = run_json(&["doctor", "--json", "--dedupe-by", "no_such_field"]);
    let (_, all) = run_json(&["doctor", "--json"]);
    assert_eq!(
        checks(&v).len(),
        checks(&all).len(),
        "rows without the key are kept, never silently dropped"
    );
}

#[test]
fn count_only_replaces_the_payload() {
    let (_, all) = run_json(&["doctor", "--json"]);
    let expected = checks(&all).len();
    let (_, v) = run_json(&["doctor", "--json", "--count-only"]);
    assert_eq!(v["data"], serde_json::json!({ "count": expected }));
    assert!(v["data"].get("checks").is_none());
}

#[test]
fn count_only_counts_after_filter() {
    let (_, filtered) = run_json(&["doctor", "--json", "--filter", "name~platform"]);
    let expected = checks(&filtered).len();
    let (_, counted) = run_json(&[
        "doctor",
        "--json",
        "--filter",
        "name~platform",
        "--count-only",
    ]);
    assert_eq!(counted["data"]["count"], expected);
    assert!(
        expected < checks(&run_json(&["doctor", "--json"]).1).len(),
        "filter must run before the count"
    );
}

#[test]
fn truncate_content_cuts_chars_without_breaking_utf8() {
    // `commands` carries prose (`about`) plus non-ASCII-safe arrow glyphs in help.
    let (_, v) = run_json(&["commands", "--json", "--truncate-content", "4"]);
    let cmds = v["data"]["commands"].as_array().expect("commands array");
    assert!(!cmds.is_empty());
    for c in cmds {
        let about = c["about"].as_str().expect("about string");
        assert!(about.chars().count() <= 4, "got {about:?}");
    }
    // Round-tripping proves no multi-byte sequence was split mid-character.
    let raw = serde_json::to_string(&v).expect("re-serialize");
    let _: serde_json::Value = serde_json::from_str(&raw).expect("still valid UTF-8 JSON");
}

#[test]
fn truncate_content_preserves_multibyte_boundaries() {
    let out = bin()
        .args([
            "--lang",
            "pt-BR",
            "doctor",
            "--json",
            "--truncate-content",
            "3",
        ])
        .output()
        .expect("spawn");
    let text = String::from_utf8(out.stdout).expect("stdout is valid UTF-8");
    let v: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
    for c in checks(&v) {
        assert!(c["detail"].as_str().expect("detail").chars().count() <= 3);
    }
}

#[test]
fn agent_surface_reports_counters_when_a_knob_is_active() {
    let (_, v) = run_json(&["doctor", "--json", "--filter", "name~platform"]);
    let s = &v["agent_surface"];
    assert!(s.is_object(), "agent_surface must be present");
    assert!(s["input_count"].as_u64().expect("input_count") > 0);
    assert_eq!(
        s["output_count"].as_u64().expect("output_count") as usize,
        checks(&v).len()
    );
    assert_eq!(s["content_truncated"], false);
    assert_eq!(s["output_truncated"], false);
}

#[test]
fn agent_surface_is_absent_without_knobs() {
    let (_, v) = run_json(&["doctor", "--json"]);
    assert!(
        v.get("agent_surface").is_none(),
        "a dead field costs the agent tokens for nothing"
    );
}

#[test]
fn error_envelope_is_not_silenced_by_filter() {
    let out = bin()
        .args([
            "schema",
            "--cmd",
            "no-such-command",
            "--json",
            "--filter",
            "name=impossible",
            "--select",
            "name",
        ])
        .output()
        .expect("spawn");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).expect("error envelope json");
    assert_eq!(v["ok"], false, "failures always reach the caller");
    assert!(v["error"]["kind"].is_string());
    assert!(v.get("agent_surface").is_none());
    assert_ne!(out.status.code(), Some(0));
}

#[test]
fn select_projects_a_payload_without_an_array() {
    let (code, v) = run_json(&["version", "--json", "--select", "name,msrv"]);
    assert_eq!(code, 0);
    let data = v["data"].as_object().expect("data object");
    let mut keys: Vec<&str> = data.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["msrv", "name"]);
    assert_eq!(v["data"]["name"], "docsrs-cli");
}

#[test]
fn dotted_path_prefixes_are_tolerated() {
    for path in ["name", "checks.name", "data.checks.name"] {
        let (_, v) = run_json(&["doctor", "--json", "--select", path]);
        assert!(
            checks(&v)[0].get("name").is_some(),
            "path {path} must resolve"
        );
    }
}

#[test]
fn knobs_do_not_alter_human_output() {
    let out = bin()
        .args(["--format", "text", "version", "--count-only"])
        .output()
        .expect("spawn");
    assert!(out.status.success());
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("docsrs-cli"), "human path stays untouched: {s}");
    assert!(!s.contains("count"));
}

#[test]
fn dry_run_envelope_is_reducible() {
    let (code, v) = run_json(&[
        "--dry-run",
        "readme",
        "serde",
        "--json",
        "--select",
        "planned_url",
    ]);
    assert_eq!(code, 0);
    assert_eq!(v["dry_run"], true);
    let data = v["data"].as_object().expect("data object");
    assert!(data.contains_key("planned_url"));
    assert!(
        !data.contains_key("planned_params"),
        "projection applies to the dry-run plan too"
    );
}

#[test]
fn max_items_caps_the_emitted_list() {
    let (_, v) = run_json(&["doctor", "--json", "--max-items", "2"]);
    assert_eq!(v["ok"], true);
    assert_eq!(checks(&v).len(), 2);
    assert_eq!(v["agent_surface"]["output_count"], 2);
    assert_eq!(v["agent_surface"]["limited"], true);
}

#[test]
fn max_items_above_the_set_is_not_reported_as_limited() {
    // A caller must be able to tell "this is everything" from "this was cut",
    // otherwise raising the cap looks worthwhile when nothing more exists.
    let (_, v) = run_json(&["doctor", "--json", "--max-items", "999"]);
    assert_eq!(v["agent_surface"]["limited"], false);
}

#[test]
fn max_items_zero_emits_an_empty_list_with_ok_true() {
    let (code, v) = run_json(&["doctor", "--json", "--max-items", "0"]);
    assert_eq!(code, 0, "an empty selection is not a failure");
    assert_eq!(v["ok"], true);
    assert!(checks(&v).is_empty());
}

#[test]
fn sort_by_orders_ascending_and_is_a_no_op_for_absent_keys() {
    let (_, sorted) = run_json(&["doctor", "--json", "--sort-by", "name", "--select", "name"]);
    let names: Vec<&str> = checks(&sorted)
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    let mut expected = names.clone();
    expected.sort_unstable();
    assert_eq!(names, expected, "ascending by name");

    let (_, plain) = run_json(&["doctor", "--json", "--select", "name"]);
    let (_, absent) = run_json(&[
        "doctor",
        "--json",
        "--sort-by",
        "no_such_key",
        "--select",
        "name",
    ]);
    assert_eq!(
        checks(&plain),
        checks(&absent),
        "a key nobody carries must not reorder anything"
    );
}

#[test]
fn count_only_counts_the_slice_not_the_whole_list() {
    let (_, all) = run_json(&["doctor", "--json", "--count-only"]);
    let total = all["data"]["count"].as_u64().expect("count");
    assert!(total > 1, "doctor must publish more than one check");

    let (_, capped) = run_json(&["doctor", "--json", "--max-items", "1", "--count-only"]);
    assert_eq!(
        capped["data"]["count"], 1,
        "the count describes what was emitted"
    );
}

#[test]
fn sort_by_and_max_items_never_touch_a_failure_envelope() {
    let (code, v) = run_json(&[
        "get-item",
        "serde",
        "widget",
        "Foo",
        "--json",
        "--sort-by",
        "name",
        "--max-items",
        "1",
    ]);
    assert_eq!(code, 65);
    assert_eq!(v["ok"], false);
    assert!(
        v.get("agent_surface").is_none(),
        "reduction must not annotate a failure it did not touch"
    );
    assert!(
        v["error"]["message"]
            .as_str()
            .expect("message")
            .contains("unknown item type"),
        "the original diagnosis reaches the caller intact"
    );
}
