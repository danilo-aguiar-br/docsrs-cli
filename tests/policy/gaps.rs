//! Gates over `gaps.md`, the audit trail.

use super::{assert_clean, read};
use regex::Regex;

/// One `## GAP-…` block: its heading, the line it starts on, and its `### `
/// section headings in order.
struct Block {
    id: String,
    sections: Vec<(usize, String)>,
}

/// Split `gaps.md` into blocks.
///
/// The split is on `## GAP-` rather than on an ID shape. An earlier ad-hoc check
/// used `GAP-[A-Z]+-\d+` and silently skipped four real gaps — `GAP-I18N-001/002/003`
/// because `I18N` carries a digit, and `GAP-DOC-RAYON-001` because it has two name
/// segments. A gate that under-counts reports green.
fn blocks(text: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let n = i + 1;
        if let Some(rest) = line.strip_prefix("## GAP-") {
            out.push(Block {
                id: format!("GAP-{}", rest.split_whitespace().next().unwrap_or_default()),
                sections: Vec::new(),
            });
        } else if let Some(rest) = line.strip_prefix("### ") {
            if let Some(b) = out.last_mut() {
                b.sections.push((n, rest.trim().to_string()));
            }
        }
    }
    out
}

fn is_state(section: &str) -> bool {
    let lower = section.to_lowercase();
    lower.starts_with("estado") || lower.starts_with("state")
}

/// Every gap must say where it stands.
///
/// A block without `### Estado` cannot be closed mechanically: the reader has to
/// re-derive the outcome from prose, which is the failure the file exists to
/// prevent.
#[test]
fn every_gap_block_declares_its_state() {
    let text = read("gaps.md");
    let problems: Vec<String> = blocks(&text)
        .iter()
        .filter(|b| !b.sections.iter().any(|(_, s)| is_state(s)))
        .map(|b| format!("{}: no '### Estado' section", b.id))
        .collect();
    assert_clean("gap state presence", &problems);
}

/// `### Estado` must be the LAST section of its block.
///
/// Presence was never enough. `GAP-SIZE-002` closed with `### Estado` on one line
/// and a `### Risco declarado fechado` section seven lines below it, so a reader
/// who stopped at the Estado concluded the risk was open with the evidence of the
/// contrary just underneath. That block diagnosed the trap in its own prose on
/// 2026-08-09 — "seção de fechamento vem depois do Estado é uma armadilha de
/// leitura" — and on 2026-08-10 `GAP-ADR-001` did exactly the same thing: Estado
/// said "Resolvido, provado nos dois sentidos" with "ela foi PARCIAL" six lines
/// below.
///
/// The document described the disease and caught it again the next day, because
/// the gate watched presence and never order.
#[test]
fn the_state_section_is_the_last_section_of_its_block() {
    let text = read("gaps.md");
    let mut problems = Vec::new();
    for b in blocks(&text) {
        let Some(last_state) = b.sections.iter().rposition(|(_, s)| is_state(s)) else {
            continue;
        };
        let trailing: Vec<String> = b.sections[last_state + 1..]
            .iter()
            .map(|(n, s)| format!("L{n} '{s}'"))
            .collect();
        if !trailing.is_empty() {
            let (n, _) = &b.sections[last_state];
            problems.push(format!(
                "{}: '### Estado' at L{n} is followed by {}; a reader who stops at the state reads a stale verdict",
                b.id,
                trailing.join(", ")
            ));
        }
    }
    assert_clean("gap state ordering", &problems);
}

/// A written count about a file belongs to the gate that reads that file.
///
/// `GAP-TRACE-001` closed by writing "29 blocos" into its own prose, and the
/// number was wrong the moment two gaps were added — the very next round.
/// Correcting the digit only resets the clock.
#[test]
fn no_written_gap_count_disagrees_with_the_file() {
    let text = read("gaps.md");
    let real = blocks(&text).len();
    let ids: std::collections::BTreeSet<String> = blocks(&text).into_iter().map(|b| b.id).collect();

    let claim =
        Regex::new(r"Medido hoje: (\d+) blocos, (\d+) identificadores").expect("static regex");
    let mut problems = Vec::new();
    for cap in claim.captures_iter(&text) {
        let (b, i) = (
            cap[1].parse::<usize>().unwrap_or_default(),
            cap[2].parse::<usize>().unwrap_or_default(),
        );
        if b != real || i != ids.len() {
            problems.push(format!(
                "claims {b} blocks / {i} ids; measured {real} / {}",
                ids.len()
            ));
        }
    }
    if ids.len() != real {
        problems.push(format!(
            "{real} blocks but {} unique identifiers (duplicate heading)",
            ids.len()
        ));
    }
    assert_clean("gap count claims", &problems);
}
