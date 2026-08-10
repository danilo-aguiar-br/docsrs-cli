//! Gates over `docs/schemas/`, where a stale file is a wrong contract rather
//! than wrong prose.
//!
//! `src/meta_cmds/schema.rs` embeds all twenty schemas with `include_str!`, so
//! `docsrs-cli schema --cmd <name>` hands the caller whatever these files say.
//! The sibling gates elsewhere check that every schema is *named* in the agent
//! contract and that the *count* written in prose matches the disk. Both passed
//! on 2026-08-10 while three files in the `cache` family and three in the
//! `config` family declared a shape the binary had stopped emitting.
//!
//! The dispatch had recently moved from one umbrella file per family to one file
//! per wire name. That was the right fix, and it left the four members of each
//! family free to drift apart while every existing gate kept agreeing.

use std::collections::{BTreeMap, BTreeSet};

use super::{assert_clean, files_under, read, rel_of};

/// The `oneOf` variant titles a schema declares, as a set.
///
/// Two schemas that describe the same set of variants describe the same family,
/// which is what makes this derivable: nothing here lists which files are
/// siblings, the files say so themselves.
fn variant_titles(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Some((_, rest)) = text.split_once("\"oneOf\"") else {
        return out;
    };
    for chunk in rest.split("\"title\"").skip(1) {
        let Some((_, after)) = chunk.split_once('"') else {
            continue;
        };
        if let Some((title, _)) = after.split_once('"') {
            out.insert(title.to_string());
        }
    }
    out
}

/// Every field name any schema declares under any `properties` block.
fn declared_fields(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for chunk in text.split("\"properties\"").skip(1) {
        // Take the keys at the head of this block, which are the field names.
        // Stop at the first nested `properties`, because that block is handled
        // by the next iteration of this same loop.
        for line in chunk.lines() {
            let trimmed = line.trim();
            let Some(rest) = trimmed.strip_prefix('"') else {
                continue;
            };
            let Some((name, tail)) = rest.split_once('"') else {
                continue;
            };
            if tail.trim_start().starts_with(':')
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
            {
                out.insert(name.to_string());
            }
        }
    }
    out
}

/// Schemas describing the same `oneOf` family must carry the same body.
///
/// `docs/schemas/README.md` states the members of each family share one body,
/// and the dispatch serves each member for its own wire name. Both are only
/// true while the files agree, and on 2026-08-10 neither was: `cache-clear`
/// carried `target_source` and its three siblings did not, so `schema --cmd
/// cache` answered with a shape that rejects a real `cache clear` envelope.
#[test]
fn no_schema_drifts_from_the_siblings_that_share_its_shape() {
    let mut families: BTreeMap<Vec<String>, Vec<(String, String)>> = BTreeMap::new();
    for path in files_under("docs/schemas", ".schema.json") {
        let rel = rel_of(&path);
        let text = read(&rel);
        let titles = variant_titles(&text);
        if titles.len() < 2 {
            continue;
        }
        families
            .entry(titles.into_iter().collect())
            .or_default()
            .push((rel, text));
    }

    assert!(
        families.len() >= 2,
        "expected at least the cache and config families, found {}",
        families.len()
    );

    let mut problems = Vec::new();
    for (titles, members) in &families {
        let (first_rel, first_text) = &members[0];
        for (rel, text) in members.iter().skip(1) {
            if text != first_text {
                problems.push(format!(
                    "{rel} differs from {first_rel}, though both describe the variants [{}]",
                    titles.join(", ")
                ));
            }
        }
    }
    assert_clean("sibling schema drift", &problems);
}

/// The family grouping must actually find the families it was written for.
///
/// A `variant_titles` that returned nothing would make the gate above pass on
/// an empty set of families, which is the failure mode of every gate that
/// derives its own scope.
#[test]
fn the_family_extraction_finds_both_known_families() {
    let cache = variant_titles(&read("docs/schemas/cache-clear.schema.json"));
    let config = variant_titles(&read("docs/schemas/config-init.schema.json"));
    assert!(
        cache.contains("cache-clear") && cache.contains("cache-stats"),
        "cache family titles not extracted: {cache:?}"
    );
    assert!(
        config.contains("config-init") && config.contains("config-show"),
        "config family titles not extracted: {config:?}"
    );
    assert!(
        variant_titles(&read("docs/schemas/version.schema.json")).is_empty(),
        "version.schema.json has no oneOf and must yield no titles"
    );
}

/// Every field the wire contract declares must be taught by some prose document.
///
/// A field that exists in an envelope and in a schema, and in no document, is a
/// field an agent meets without an explanation anywhere to find. Measured on
/// 2026-08-10: 128 field names across the twenty schemas, of which `json_auto`
/// — a key of `commands.agent_notes` — appeared in no prose document at all.
#[test]
fn no_schema_field_goes_untaught() {
    let mut fields = BTreeSet::new();
    for path in files_under("docs/schemas", ".schema.json") {
        fields.extend(declared_fields(&read(&rel_of(&path))));
    }
    assert!(
        fields.len() >= 100,
        "field extraction collapsed: only {} names found",
        fields.len()
    );

    let prose: Vec<String> = super::prose_docs()
        .iter()
        .map(|p| read(&rel_of(p)))
        .collect();

    let mut problems = Vec::new();
    for field in &fields {
        if !prose.iter().any(|text| text.contains(field.as_str())) {
            problems.push(format!(
                "schema field `{field}` is declared on the wire and taught by no document"
            ));
        }
    }
    assert_clean("untaught schema fields", &problems);
}

/// The field extraction must reach the nested blocks, not just the top level.
///
/// `json_auto` lives two levels down, inside `agent_notes`. An extractor that
/// only read the outermost `properties` would have reported the gate green for
/// the one field that was actually missing.
#[test]
fn the_field_extraction_reaches_nested_property_blocks() {
    let commands = declared_fields(&read("docs/schemas/commands.schema.json"));
    assert!(
        commands.contains("json_auto"),
        "nested agent_notes fields not reached: {commands:?}"
    );
    assert!(
        commands.contains("subcommands"),
        "twice-nested command node fields not reached: {commands:?}"
    );
    let dry_run = declared_fields(&read("docs/schemas/dry-run.schema.json"));
    assert!(
        dry_run.contains("planned_method_anchors"),
        "planned_params fields not reached: {dry_run:?}"
    );
}
