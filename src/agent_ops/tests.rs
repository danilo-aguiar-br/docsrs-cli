//! Behaviour tests for the agent-native reduction pipeline.
//!
//! Split out of `mod.rs` so the module stays under the 500-physical-line ceiling
//! enforced by `no_source_file_exceeds_the_line_ceiling` in `tests/policy/`,
//! matching `crate::cli::tests` and `crate::error::tests`.

use super::*;
use crate::error::ErrorKind;

fn envelope() -> Value {
    serde_json::json!({
        "schema_version": 1,
        "ok": true,
        "command": "search-crates",
        "duration_ms": 1,
        "data": {
            "query": "serde",
            "hits": [
                {"name": "serde", "version": "1.0.0", "downloads": 10},
                {"name": "serde_json", "version": "1.0.1", "downloads": 5},
                {"name": "serde", "version": "1.0.0", "downloads": 1},
                {"version": "9.9.9"}
            ]
        }
    })
}

/// Build a plan from the knobs a test cares about, defaulting the rest.
fn plan(f: impl FnOnce(&mut AgentArgs)) -> AgentOps {
    let mut args = AgentArgs::default();
    f(&mut args);
    AgentOps::from_args(&args).expect("valid knobs")
}

fn ops(select: &[&str], filter: &[&str]) -> AgentOps {
    plan(|a| {
        a.select = select.iter().map(ToString::to_string).collect();
        a.filter = filter.iter().map(ToString::to_string).collect();
    })
}

#[test]
fn select_projects_and_skips_missing() {
    let mut env = envelope();
    ops(&["name", "version"], &[]).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(
        hits[0],
        serde_json::json!({"name": "serde", "version": "1.0.0"})
    );
    // Element without `name` yields only `version`, never `"name": null`.
    assert_eq!(hits[3], serde_json::json!({"version": "9.9.9"}));
    assert!(hits[3].get("name").is_none());
}

#[test]
fn filter_substring_and_equality() {
    let mut env = envelope();
    ops(&[], &["name~serde_"]).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0]["name"], "serde_json");

    let mut env = envelope();
    ops(&[], &["downloads=10"]).apply_to_envelope(&mut env);
    assert_eq!(env["data"]["hits"].as_array().expect("hits").len(), 1);
}

#[test]
fn filter_conjunction_is_and() {
    let mut env = envelope();
    ops(&[], &["name=serde", "downloads=10"]).apply_to_envelope(&mut env);
    assert_eq!(env["data"]["hits"].as_array().expect("hits").len(), 1);
}

#[test]
fn filter_missing_key_never_matches_even_for_ne() {
    let mut env = envelope();
    ops(&[], &["name!=serde"]).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1, "only serde_json survives; keyless row drops");
    assert_eq!(hits[0]["name"], "serde_json");
}

#[test]
fn malformed_filter_is_invalid_input() {
    let args = AgentArgs {
        filter: vec!["nope".to_string()],
        ..AgentArgs::default()
    };
    let bad = AgentOps::from_args(&args).expect_err("must reject");
    assert_eq!(bad.kind(), ErrorKind::InvalidInput);
    assert_eq!(bad.kind().exit_code(), 65);
}

#[test]
fn dedupe_keeps_keyless_elements() {
    let mut env = envelope();
    plan(|a| a.dedupe_by = Some("name".to_string())).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 3, "one duplicate serde removed, keyless kept");
}

#[test]
fn sort_by_compares_numbers_numerically_not_lexically() {
    // The trap this guards: rendered as text, "10" sorts before "5".
    let mut env = envelope();
    plan(|a| a.sort_by = Some("downloads".to_string())).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    let order: Vec<u64> = hits
        .iter()
        .filter_map(|h| h.get("downloads").and_then(Value::as_u64))
        .collect();
    assert_eq!(order, vec![1, 5, 10]);
}

#[test]
fn sort_by_puts_elements_without_the_key_last() {
    let mut env = envelope();
    plan(|a| a.sort_by = Some("downloads".to_string())).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert!(
        hits.last().expect("non-empty").get("downloads").is_none(),
        "an unknown value is not the smallest value"
    );
}

#[test]
fn sort_by_is_stable_for_ties() {
    let mut env = serde_json::json!({
        "ok": true,
        "data": {"hits": [
            {"k": 1, "id": "a"},
            {"k": 1, "id": "b"},
            {"k": 0, "id": "c"},
            {"k": 1, "id": "d"}
        ]}
    });
    plan(|a| a.sort_by = Some("k".to_string())).apply_to_envelope(&mut env);
    let ids: Vec<&str> = env["data"]["hits"]
        .as_array()
        .expect("hits")
        .iter()
        .filter_map(|h| h["id"].as_str())
        .collect();
    assert_eq!(ids, vec!["c", "a", "b", "d"], "ties keep upstream order");
}

#[test]
fn sort_by_absent_key_is_not_an_error_and_preserves_order() {
    let mut env = envelope();
    let before = env["data"]["hits"].clone();
    plan(|a| a.sort_by = Some("no_such_key".to_string())).apply_to_envelope(&mut env);
    assert_eq!(env["data"]["hits"], before);
}

#[test]
fn max_items_cuts_after_filter_and_dedupe_never_before() {
    let mut env = envelope();
    plan(|a| {
        a.filter = vec!["name~serde".to_string()];
        a.dedupe_by = Some("name".to_string());
        a.max_items = Some(1);
    })
    .apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 1);
    // Filter left 3, dedupe left 2, limit left 1: cutting first would have
    // discarded serde_json before it could ever match.
    assert_eq!(env["agent_surface"]["input_count"], 4);
    assert_eq!(env["agent_surface"]["output_count"], 1);
    assert_eq!(env["agent_surface"]["limited"], true);
}

#[test]
fn max_items_above_the_set_reports_no_limiting() {
    let mut env = envelope();
    plan(|a| a.max_items = Some(99)).apply_to_envelope(&mut env);
    assert_eq!(env["data"]["hits"].as_array().expect("hits").len(), 4);
    assert_eq!(
        env["agent_surface"]["limited"], false,
        "a small set must not look like a cut one"
    );
}

#[test]
fn max_items_zero_is_an_empty_list_not_an_error() {
    let mut env = envelope();
    plan(|a| a.max_items = Some(0)).apply_to_envelope(&mut env);
    assert!(env["data"]["hits"].as_array().expect("hits").is_empty());
    assert_eq!(env["ok"], true);
    assert_eq!(env["agent_surface"]["output_count"], 0);
}

#[test]
fn count_only_counts_after_filter() {
    let mut env = envelope();
    plan(|a| {
        a.filter = vec!["name~serde".to_string()];
        a.count_only = true;
    })
    .apply_to_envelope(&mut env);
    assert_eq!(env["data"], serde_json::json!({"count": 3}));
    assert_eq!(env["agent_surface"]["input_count"], 4);
    assert_eq!(env["agent_surface"]["output_count"], 3);
}

#[test]
fn count_only_counts_the_slice_not_the_filtered_set() {
    let mut env = envelope();
    plan(|a| {
        a.filter = vec!["name~serde".to_string()];
        a.max_items = Some(2);
        a.count_only = true;
    })
    .apply_to_envelope(&mut env);
    assert_eq!(
        env["data"],
        serde_json::json!({"count": 2}),
        "the count must describe what was emitted"
    );
}

#[test]
fn emitted_follows_the_array_but_total_does_not() {
    // GAP-COUNT-001. `emitted` is documented as hits actually emitted, so it has to
    // track the array. `total` counts the upstream index and must survive intact.
    let mut env = serde_json::json!({
        "ok": true,
        "command": "search-in-crate",
        "data": {
            "total": 4,
            "emitted": 4,
            "hits": [
                {"name": "A", "kind": "struct"},
                {"name": "B", "kind": "trait"},
                {"name": "C", "kind": "struct"},
                {"name": "D", "kind": "struct"}
            ]
        }
    });
    plan(|a| a.filter = vec!["kind=struct".to_string()]).apply_to_envelope(&mut env);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert_eq!(hits.len(), 3);
    assert_eq!(env["data"]["emitted"], 3, "emitted follows the array");
    assert_eq!(env["data"]["total"], 4, "total describes upstream, not us");
}

#[test]
fn truncate_content_respects_char_boundaries() {
    let mut env = serde_json::json!({
        "ok": true,
        "data": {"hits": [{"name": "áéíóú-emoji-🦀-tail"}]}
    });
    plan(|a| a.truncate_content = Some(5)).apply_to_envelope(&mut env);
    let name = env["data"]["hits"][0]["name"].as_str().expect("string");
    assert_eq!(name, "áéíóú");
    assert_eq!(name.chars().count(), 5);
    assert_eq!(env["agent_surface"]["content_truncated"], true);
}

#[test]
fn error_envelope_is_never_reduced() {
    let raw = br#"{"schema_version":1,"ok":false,"command":"get-item","duration_ms":1,"error":{"code":66,"kind":"not_found","message":"missing","retryable":false}}"#;
    let out = plan(|a| {
        a.select = vec!["name".to_string()];
        a.filter = vec!["name=zzz".to_string()];
        a.sort_by = Some("name".to_string());
        a.max_items = Some(1);
    })
    .apply_to_bytes(raw);
    assert_eq!(out, raw.to_vec(), "failures always reach the caller");
}

#[test]
fn non_json_payload_passes_through() {
    let raw = b"#!/usr/bin/env bash\ncomplete -F _docsrs docsrs-cli\n";
    assert_eq!(ops(&["name"], &[]).apply_to_bytes(raw), raw.to_vec());
}

#[test]
fn max_output_bytes_drops_elements_not_bytes() {
    // Self-calibrating: measure the unbudgeted envelope, then ask for one byte
    // less so at least one element must go, without hardcoding a fragile size.
    let mut full = envelope();
    plan(|a| a.select = vec!["name".to_string()]).apply_to_envelope(&mut full);
    let budget = serde_json::to_vec(&full).expect("serialize").len() as u64 - 1;

    let mut env = envelope();
    plan(|a| {
        a.select = vec!["name".to_string()];
        a.max_output_bytes = Some(budget);
    })
    .apply_to_envelope(&mut env);
    let len = serde_json::to_vec(&env).expect("serialize").len() as u64;
    assert!(len <= budget, "envelope len {len} > {budget}");
    assert_eq!(env["agent_surface"]["output_truncated"], true);
    let hits = env["data"]["hits"].as_array().expect("hits");
    assert!(!hits.is_empty(), "budget must not empty a satisfiable list");
    assert!(hits.len() < 4, "trailing elements dropped");
    assert_eq!(
        env["agent_surface"]["output_count"]
            .as_u64()
            .expect("count") as usize,
        hits.len(),
        "surface reflects the post-budget list"
    );
    // Whole-element drops keep the document parseable; a byte slice would not.
    serde_json::to_string(&env).expect("still valid JSON");
}

#[test]
fn budget_below_the_envelope_floor_empties_the_list() {
    let mut env = envelope();
    plan(|a| a.max_output_bytes = Some(10)).apply_to_envelope(&mut env);
    // Unreachable budget: emit the floor payload flagged as truncated rather
    // than an unparseable fragment. Agents raise the cap on this signal.
    assert!(env["data"]["hits"].as_array().expect("hits").is_empty());
    assert_eq!(env["agent_surface"]["output_truncated"], true);
    assert_eq!(env["agent_surface"]["output_count"], 0);
    serde_json::to_string(&env).expect("still valid JSON");
}

#[test]
fn dotted_paths_tolerate_data_and_array_prefixes() {
    for path in ["name", "hits.name", "data.hits.name"] {
        let mut env = envelope();
        ops(&[path], &[]).apply_to_envelope(&mut env);
        assert_eq!(env["data"]["hits"][0]["name"], "serde", "path {path}");
    }
}

#[test]
fn object_payload_without_array_is_projected() {
    let mut env = serde_json::json!({
        "ok": true,
        "data": {"name": "docsrs-cli", "version": "1.2.1", "msrv": "1.88.0"}
    });
    ops(&["name", "msrv"], &[]).apply_to_envelope(&mut env);
    assert_eq!(
        env["data"],
        serde_json::json!({"name": "docsrs-cli", "msrv": "1.88.0"})
    );
}

#[test]
fn inactive_plan_reports_no_knobs() {
    let plan = plan(|a| a.max_output_bytes = Some(10));
    assert!(!plan.is_active(), "max-output-bytes alone is not a knob");
}
