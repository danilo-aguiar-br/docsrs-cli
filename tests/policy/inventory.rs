//! Whether a document's own inventory still matches the tree it indexes.
//!
//! Both gates here were written after an audit found a defect that three
//! existing gates walked past, and in both cases those gates were correct about
//! what they declared. One counts the *number* of schemas written in prose and
//! compares it to the disk; the sentence said twenty and the list beneath it
//! held nineteen. Another requires every schema name in `AGENTS.md`, and the
//! file whose entire purpose is to be the inventory was in no gate's list at
//! all.
//!
//! A count is not an inventory, a sibling document is not an inventory, and a
//! translation that is shorter than its original is not a translation.

use super::{assert_clean, files_under, numbered, prose_docs, read, rel_of, repo_root};

/// The index file, split into its English half and its Portuguese half.
///
/// Split rather than searched whole, because a name present once satisfies a
/// whole-file scan while leaving one of the two readers unable to reach the
/// document. The marker is the language heading the file already uses.
fn index_halves() -> (String, String) {
    const INDEX: &str = "docs/schemas/README.md";
    const MARKER: &str = "## Português Brasileiro";
    let text = read(INDEX);
    let (en, pt) = text
        .split_once(MARKER)
        .expect("docs/schemas/README.md lost its Portuguese heading");
    (en.to_string(), pt.to_string())
}

/// Everything `docs/schemas/README.md` promises to index, taken from the disk.
///
/// Both directories are read, because the file indexes both and drifted in both.
fn indexed_files() -> Vec<String> {
    let mut out: Vec<String> = files_under("docs/schemas", ".schema.json")
        .iter()
        .chain(files_under("docs/decisions", ".md").iter())
        .map(|p| rel_of(p).rsplit('/').next().unwrap_or_default().to_string())
        .filter(|name| !name.ends_with(".pt-BR.md") && name != "README.md")
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The schema index must reach every file it claims to index, in both languages.
///
/// Measured on 2026-08-10: `docs/schemas/` held twenty schemas and the inventory
/// listed nineteen — `agent-surface` was absent from both language halves. Four
/// of the nine architecture decisions were reachable from no user-facing document
/// at all, and two of those four (`0005`, `0009`) were linked from nowhere in the
/// repository, source included.
///
/// Two gates already read this subject and both reported green. One counts the
/// *number* written in prose against the disk, and the sentence said twenty
/// correctly while the list under it was one short. The other requires every
/// schema name in `AGENT_DOCS`, and `docs/schemas/README.md` is not in that list
/// — the file whose entire purpose is to be the inventory was the one file no
/// gate read. A count is not an inventory, and neither is a sibling document.
#[test]
fn the_schema_index_reaches_every_file_it_indexes() {
    let files = indexed_files();
    assert!(
        files.len() >= 25,
        "only {} indexable files came back; the extraction is broken, not the index",
        files.len()
    );

    let (en, pt) = index_halves();
    let mut problems = Vec::new();
    for name in &files {
        if !en.contains(name.as_str()) {
            problems.push(format!("docs/schemas/README.md: English half omits {name}"));
        }
        if !pt.contains(name.as_str()) {
            problems.push(format!(
                "docs/schemas/README.md: Portuguese half omits {name}"
            ));
        }
    }
    assert_clean("schema index completeness", &problems);
}

/// The extraction must reach both directories and both halves must be real.
///
/// A canary, because this gate's failure mode is silence: an extraction that
/// narrows to one directory, or a split that yields an empty half, would report
/// green over an index that lists nothing.
#[test]
fn the_index_extraction_reaches_both_directories() {
    let files = indexed_files();
    assert!(
        files.iter().any(|f| f.ends_with(".schema.json")),
        "extraction found no schema at all: {files:?}"
    );
    assert!(
        files.iter().any(|f| f.starts_with("0009-")),
        "extraction missed the decisions directory: {files:?}"
    );
    assert!(
        !files.iter().any(|f| f.ends_with(".pt-BR.md")),
        "the pt-BR twin is not a separate record and must not be demanded twice"
    );

    let (en, pt) = index_halves();
    assert!(
        en.contains("### Inventory") && pt.contains("### Inventário"),
        "the split no longer separates the two inventories"
    );
}

/// Every document paired with its Portuguese twin, derived from the file names.
///
/// A twin that does not exist yields no pair rather than a failure: `llms-full.txt`
/// is a deliberate single-file bundle, and demanding a twin for it would teach the
/// gate that intent does not matter.
fn language_pairs() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in prose_docs() {
        let rel = rel_of(&path);
        if rel.contains(".pt-BR.") || rel.starts_with("tests/") {
            continue;
        }
        let Some((stem, ext)) = rel.rsplit_once('.') else {
            continue;
        };
        let twin = format!("{stem}.pt-BR.{ext}");
        if repo_root().join(&twin).is_file() {
            out.push((rel, twin));
        }
    }
    out.sort();
    out
}

/// Section headings, which is the granularity at which a translation goes missing.
fn heading_count(text: &str) -> usize {
    numbered(text)
        .filter(|(_, line)| line.starts_with("## ") || line.starts_with("### "))
        .count()
}

/// A translated document must carry every section its English original carries.
///
/// Measured on 2026-08-10 across every pair in the tree: thirty-odd pairs agreed
/// exactly, and one did not. `docs/decisions/0004-threat-model.pt-BR.md` was
/// missing `## CVSS usage` outright — the section that tells a reader how a
/// reported vulnerability gets prioritised, absent from the Portuguese half of a
/// security document while every other pair matched.
///
/// The invariant is not that translations must be literal. It is that a section
/// is a promise about coverage, and this product is bilingual by mandate, so a
/// reader who chose Portuguese must not be quietly served a shorter document.
/// Counting headings is a proxy, and it is the proxy that caught the real one.
#[test]
fn no_translation_drops_a_section_its_original_carries() {
    let pairs = language_pairs();
    assert!(
        pairs.len() >= 20,
        "only {} language pairs found; the pairing is broken, not the docs",
        pairs.len()
    );

    let problems: Vec<String> = pairs
        .iter()
        .filter_map(|(en, pt)| {
            let (a, b) = (heading_count(&read(en)), heading_count(&read(pt)));
            (a != b).then(|| format!("{en} has {a} sections, {pt} has {b}"))
        })
        .collect();
    assert_clean("bilingual section parity", &problems);
}

/// Every `src/` path an ADR points at must still exist.
///
/// The `Related` section is the pointer an auditor follows to check a decision
/// against the code, so a dead pointer costs exactly the reader who was doing
/// the right thing. Measured on 2026-08-10: ADR 0003 pointed at
/// `src/crates_io.rs` and `src/retry.rs`, and ADR 0004 at `src/http.rs`,
/// `src/config.rs`, `src/domain.rs` and `src/cache.rs`. All six had become
/// directories, and four of them sat in a list of five.
#[test]
fn no_decision_record_points_at_a_source_path_that_is_gone() {
    let cited = regex::Regex::new(r"`(src/[A-Za-z0-9_/]+(?:\.rs)?/?)`").expect("static regex");
    let mut seen = 0usize;
    let mut problems = Vec::new();

    for path in files_under("docs/decisions", ".md") {
        let rel = rel_of(&path);
        let text = read(&rel);
        for (n, line) in numbered(&text) {
            for caps in cited.captures_iter(line) {
                let cited_path = &caps[1];
                seen += 1;
                if !repo_root().join(cited_path.trim_end_matches('/')).exists() {
                    problems.push(format!(
                        "{rel}:{n}: cites `{cited_path}`, which does not exist"
                    ));
                }
            }
        }
    }

    assert!(
        seen >= 20,
        "the src-path extraction collapsed: only {seen} citations found across the decision records"
    );
    assert_clean("dead source pointers in decision records", &problems);
}

/// The extraction must reach both spellings an ADR uses for a source pointer.
///
/// Records name a module either as a file (`src/doctor.rs`) or as a directory
/// (`src/http/`). An extraction that matched only one form would have reported
/// green on the very lists that were wrong, because those lists mixed both.
#[test]
fn the_source_pointer_extraction_reads_files_and_directories_alike() {
    let cited = regex::Regex::new(r"`(src/[A-Za-z0-9_/]+(?:\.rs)?/?)`").expect("static regex");
    let hits: Vec<String> = cited
        .captures_iter("- `SECURITY.md`, `src/http/`, `src/doctor.rs`, `src/nope.rs`")
        .map(|c| c[1].to_string())
        .collect();
    assert_eq!(
        hits,
        vec!["src/http/", "src/doctor.rs", "src/nope.rs"],
        "extraction missed a spelling"
    );
    assert!(
        !repo_root().join("src/nope.rs").exists(),
        "the negative control must not exist, or the gate proves nothing"
    );
}
