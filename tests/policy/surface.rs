//! Vocabulary the binary invents about itself, held against the documents.
//!
//! Both gates here answer the same question in two places: when the product
//! coins a token an agent will read off the wire, does every document that
//! claims to teach that vocabulary actually name it?
//!
//! Nothing asked this before. `tests/policy/contract.rs` derives flags, wire
//! names and schemas, and stops there — so `ErrorKind` could gain a variant and
//! the thirteen kinds listed in `error.schema.json` would quietly become
//! fourteen-minus-one, with no test able to notice. The anchor families have the
//! same shape: five constants in `src/docs_rs/assoc.rs` decide which rustdoc
//! items resolve, and the documents teaching item resolution list them by hand.

use super::{assert_clean, read};
use docsrs_cli::error::ErrorKind;
use regex::Regex;

/// The one document that promises the whole `kind` catalogue.
///
/// Scoped to a single file on purpose, and the first draft of this gate got it
/// wrong: it demanded all thirteen kinds from `COOKBOOK` and `AGENTS` too, and
/// reported fourteen problems that were not defects. Those documents teach kinds
/// *by example* inside recipes — `error.kind=budget` beside the recipe that hits
/// a budget — and a gate that demands a catalogue where none was promised is the
/// mirror of the defect it was written to prevent. The inverse direction below
/// is what covers them, and it is falsifiable everywhere.
const KIND_CATALOGUE: &str = "docs/schemas/error.schema.json";

/// Every document that names a kind at all, catalogue or example.
fn documents_naming_kinds() -> &'static [&'static str] {
    &[
        "docs/schemas/error.schema.json",
        "docs/AGENTS.md",
        "docs/AGENTS.pt-BR.md",
        "docs/COOKBOOK.md",
        "docs/COOKBOOK.pt-BR.md",
        "docs/HOW_TO_USE.md",
        "docs/HOW_TO_USE.pt-BR.md",
        "skills/docsrs-cli-en/SKILL.md",
        "skills/docsrs-cli-pt/SKILL.md",
    ]
}

/// Every wire spelling the binary can put in `error.kind`.
///
/// Derived from `as_str`, the one function the product uses to name itself, so
/// the list cannot be one variant behind the enum. `interrupted` and
/// `terminated` are internal spellings that reach the wire as `canceled`; both
/// are included because a document may legitimately teach either.
fn wire_kinds() -> Vec<&'static str> {
    [
        ErrorKind::Usage,
        ErrorKind::InvalidInput,
        ErrorKind::NotFound,
        ErrorKind::RateLimited,
        ErrorKind::Unavailable,
        ErrorKind::Timeout,
        ErrorKind::Network,
        ErrorKind::Budget,
        ErrorKind::Parse,
        ErrorKind::Config,
        ErrorKind::Io,
        ErrorKind::Internal,
        ErrorKind::BrokenPipe,
    ]
    .into_iter()
    .map(ErrorKind::as_str)
    .collect()
}

/// Every kind the binary can emit must be named where kinds are taught.
///
/// The new `io` kind is what made this necessary. Before it, filesystem failure
/// arrived as `internal` — telling an agent to report a bug in a CLI that had
/// behaved correctly, and `retryable: false` about a full disk that clears the
/// moment an operator frees a block. Adding the kind fixes the instance; this
/// fixes the class, so the fourteenth kind cannot ship undocumented.
#[test]
fn every_error_kind_is_named_in_the_kind_catalogue() {
    let kinds = wire_kinds();
    assert!(
        kinds.len() >= 13,
        "only {} kinds derived; the extraction is broken, not the docs",
        kinds.len()
    );

    let text = read(KIND_CATALOGUE);
    // Word boundaries, not `contains`: the shortest kind is `io`, and a naive
    // substring search finds it inside `description` on every JSON Schema ever
    // written. The first draft of this gate passed for exactly that reason.
    let problems: Vec<String> = kinds
        .iter()
        .filter(|k| {
            !Regex::new(&format!(r"\b{}\b", regex::escape(k)))
                .expect("kind names are literal")
                .is_match(&text)
        })
        .map(|k| format!("{KIND_CATALOGUE}: missing kind `{k}`"))
        .collect();
    assert_clean("error kinds in the catalogue", &problems);
}

/// No document may teach a kind the binary cannot emit.
///
/// This is the direction that covers every document, because it needs no claim
/// about scope: whatever a file chooses to teach, it must be true. It catches
/// the rename and the removal, which are the ways a kind list actually rots —
/// nobody deletes a kind from a document, they change it in the enum and the
/// prose keeps naming yesterday's spelling.
#[test]
fn no_document_teaches_a_kind_the_binary_cannot_emit() {
    let known = wire_kinds();
    // Only the explicit `kind=<token>` shape, so ordinary prose about "the kind
    // of item" is never mistaken for a claim about the wire.
    let mention = Regex::new(r"kind=`?([a-z_]+)").expect("static regex");
    let mut problems = Vec::new();

    for doc in documents_naming_kinds() {
        let text = read(doc);
        for cap in mention.captures_iter(&text) {
            let named = cap[1].to_string();
            // `kind=struct` and friends are `--filter` examples over item kinds,
            // a different vocabulary that happens to share the word.
            if docsrs_cli::item_kind::ItemKind::parse(&named).is_ok() {
                continue;
            }
            if !known.contains(&named.as_str()) {
                problems.push(format!(
                    "{doc}: teaches `kind={named}`, which no ErrorKind spells"
                ));
            }
        }
    }
    assert_clean("kinds taught that the binary cannot emit", &problems);
}

/// Both directions must be able to fail, or neither proves anything.
///
/// Two gates in this repository shipped matching nothing and reported green for
/// months, so a `contains` sweep gets a canary by default.
#[test]
fn the_kind_gates_would_notice_a_kind_that_is_wrong() {
    assert!(
        !read(KIND_CATALOGUE).contains("storage_quota_exceeded"),
        "an invented kind was found in the catalogue; the scan is matching noise"
    );
    // The trap that made the first draft pass: `io` inside `description`.
    assert!(
        read(KIND_CATALOGUE).contains("description"),
        "the catalogue lost the word that hides the shortest kind from a substring scan"
    );
    assert!(
        !Regex::new(r"\bio\b")
            .expect("static regex")
            .is_match("a description field"),
        "the word-boundary match is not actually bounded"
    );
    assert!(
        wire_kinds().contains(&"io"),
        "the derived catalogue lost the kind this gate was written for"
    );
    let mention = Regex::new(r"kind=`?([a-z_]+)").expect("static regex");
    let bait = "fails closed with exit 74 (`kind=filesystem`)";
    let seen: Vec<String> = mention
        .captures_iter(bait)
        .map(|c| c[1].to_string())
        .collect();
    assert_eq!(
        seen,
        vec!["filesystem".to_string()],
        "the mention pattern no longer reads the shape it exists for"
    );
    assert!(
        !wire_kinds().contains(&"filesystem"),
        "the bait names a real kind, so it proves nothing"
    );
}

/// Documents that teach a caller how to reach an associated item.
const ANCHOR_DOCS: &[&str] = &[
    "docs/AGENTS.md",
    "docs/AGENTS.pt-BR.md",
    "docs/COOKBOOK.md",
    "docs/COOKBOOK.pt-BR.md",
    "docs/HOW_TO_USE.md",
    "docs/HOW_TO_USE.pt-BR.md",
];

/// The rustdoc anchor prefixes this binary knows how to resolve.
///
/// Read out of `src/docs_rs/assoc.rs` rather than listed, because that file is
/// where adding support for a seventh family would happen, and a hand-kept copy
/// here would be the second truth this repository keeps deleting.
fn anchor_families() -> Vec<String> {
    let src = read("src/docs_rs/assoc.rs");
    let re = Regex::new(r#"ANCHOR_PREFIXES: &\[&str\] = &\[([^\]]*)\]"#).expect("static regex");
    let quoted = Regex::new(r#""([a-z]+)\.""#).expect("static regex");
    let mut out: Vec<String> = re
        .captures_iter(&src)
        .flat_map(|c| {
            quoted
                .captures_iter(&c[1])
                .map(|q| q[1].to_string())
                .collect::<Vec<_>>()
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Every anchor family the binary resolves must be taught where items are taught.
///
/// A live audit of docs.rs on 2026-08-10 read 275 anchors across a trait, an
/// enum and a struct of the standard library and found no family outside these.
/// That number describes three pages on one day and belongs in no document; what
/// belongs is this gate, which fails the day a seventh family is supported and
/// the documents still teach six.
#[test]
fn every_supported_anchor_family_is_taught_where_items_are_taught() {
    let families = anchor_families();
    assert!(
        families.len() >= 5,
        "only {} anchor families extracted from src/docs_rs/assoc.rs: {families:?}",
        families.len()
    );

    let mut problems = Vec::new();
    for doc in ANCHOR_DOCS {
        let text = read(doc);
        for family in &families {
            if !text.contains(family.as_str()) {
                problems.push(format!("{doc}: missing anchor family `{family}.`"));
            }
        }
    }
    assert_clean("anchor families in the item documents", &problems);
}

/// The extraction must find the families the tree actually declares.
///
/// `tymethod` is the one a naive `method.` scan swallows, and `structfield` is
/// the one a scan keyed on `assoc` misses entirely.
#[test]
fn the_anchor_extraction_finds_the_families_that_are_easy_to_miss() {
    let families = anchor_families();
    for expected in [
        "method",
        "tymethod",
        "associatedtype",
        "variant",
        "structfield",
    ] {
        assert!(
            families.contains(&expected.to_string()),
            "extraction lost `{expected}.`: {families:?}"
        );
    }
    assert!(
        !families.contains(&"impl".to_string()),
        "extraction is matching prose rather than the constants: {families:?}"
    );
}

/// Every spelling `ItemKind::parse` accepts, taken from the match it lives in.
///
/// Extracted from the source rather than listed, following the precedent of
/// `wire_name` in `contract.rs`: the match arms are where the binary decides
/// what it answers to, so a spelling added there is in scope the day it is
/// written.
fn accepted_item_kinds() -> Vec<String> {
    let src = read("src/item_kind.rs");
    let body = src
        .split_once("pub fn parse_with_echo")
        .and_then(|(_, rest)| rest.split_once("other =>"))
        .map(|(body, _)| body.to_string())
        .expect("the parse match is no longer where this gate looks");
    let arm = Regex::new(r#""([a-z]+)""#).expect("static regex");
    let mut out: Vec<String> = arm.captures_iter(&body).map(|c| c[1].to_string()).collect();
    out.sort();
    out.dedup();
    out
}

/// The help string and the README must name every kind the binary accepts.
///
/// Measured on 2026-08-10: `ItemKind::parse` accepted `variant`, `structfield`
/// and `field`, and all three were absent from `src/cli/mod.rs`, whose `/// Kind:`
/// doc comment is a hand-written pipe-separated copy of that match, and from
/// `README.md` line 137, which mirrors the same string. `variant Option::Some`
/// resolved with exit 0 while every list a caller could read said it would not.
///
/// The sibling gates in this module cover `ErrorKind`, which is the vocabulary a
/// caller reads *off* the wire. Nothing covered `ItemKind`, which is the
/// vocabulary a caller must write *into* argv — the direction where being
/// unlisted means the feature may as well not exist.
#[test]
fn every_accepted_item_kind_is_named_where_the_kinds_are_listed() {
    const LISTS: &[&str] = &["src/cli/mod.rs", "README.md", "README.pt-BR.md"];
    let kinds = accepted_item_kinds();
    assert!(
        kinds.len() >= 18,
        "only {} spellings parsed from the match; the extraction is broken, not the lists",
        kinds.len()
    );

    let mut problems = Vec::new();
    for list in LISTS {
        let text = read(list);
        for kind in &kinds {
            if !text.contains(kind.as_str()) {
                problems.push(format!("{list}: never names the accepted kind `{kind}`"));
            }
        }
    }
    assert_clean("accepted item kinds in the lists", &problems);
}

/// The extraction must reach the arms that were missing, and stop at the error arm.
#[test]
fn the_item_kind_extraction_reaches_the_arms_that_went_unlisted() {
    let kinds = accepted_item_kinds();
    for expected in ["variant", "structfield", "field", "module", "derive"] {
        assert!(
            kinds.iter().any(|k| k == expected),
            "extraction missed `{expected}`: {kinds:?}"
        );
    }
    assert!(
        !kinds.iter().any(|k| k == "other"),
        "the extraction ran past the error arm and is reading the wrong thing"
    );
}
