//! TLS posture gates: provider, trust anchors and version floor.
//!
//! Every expectation here is read out of the code that does the thing, never out
//! of prose. That is the whole lesson of GAP-ADR-001 and GAP-TLS-ROOTS-001: the
//! normative document and the binary drifted apart and the document was the one
//! that kept being believed.

use super::{assert_clean, current_docs, numbered, product_source, read};
use regex::Regex;

/// Prose that ships, wherever it ships: markdown, plus rustdoc in `src/`.
fn prose_and_source() -> Vec<(String, String)> {
    let mut out = current_docs();
    out.extend(product_source());
    out
}

/// Fold `rustls-rustcrypto`, `rustls_rustcrypto` and `rustcrypto` onto one
/// spelling, because providers do not agree on how to name themselves.
fn canon(name: &str) -> String {
    let n = name.to_lowercase().replace('_', "-");
    n.strip_prefix("rustls-").map_or(n.clone(), str::to_string)
}

/// Sentences that name a provider in order to reject, replace or qualify it.
///
/// A document naming `graviola` to explain why it was refused is the
/// documentation working, not drifting. Shared by the purity gate and the
/// active-provider gate below, because both ask the same question of a line:
/// is this an assertion about what ships, or a note about what does not?
const DISCLAIMS: &[&str] = &[
    "compiles c",
    "compila c",
    "c-build-dep",
    "alternative",
    "alternativa",
    "replacement",
    "substitut",
    "blocked",
    "bloquead",
    "rejected",
    "rejeitad",
    "abandon",
    "not met",
    "não é cumprida",
    "cc-rs",
    "candidate",
    "candidat",
    "would",
    "instead of",
    "em vez de",
    "não ",
    "nao ",
    "never",
    "nunca",
    "pure-rust-blocked",
    "recusad",
    "refused",
    "considered",
    "avaliad",
    "experiment",
    "reverted",
    "revertid",
];

fn known_providers() -> Vec<String> {
    [
        "ring",
        "aws-lc-rs",
        "graviola",
        "rustls-graviola",
        "rustls-rustcrypto",
    ]
    .iter()
    .map(|p| canon(p))
    .collect()
}

/// The provider the binary actually installs, read from the bootstrap call.
fn installed_provider() -> String {
    let main = read("src/main.rs");
    let re = Regex::new(r"rustls::crypto::(\w+)::default_provider").expect("static regex");
    re.captures(&main)
        .map(|c| c[1].to_string())
        .expect("src/main.rs installs no rustls crypto provider")
}

/// The provider constant the `doctor` surface reports to operators.
fn posture_provider() -> String {
    let src = read("src/http/content_type.rs");
    let re = Regex::new(r#"TLS_CRYPTO_PROVIDER: &str = "([^"]+)""#).expect("static regex");
    re.captures(&src)
        .map(|c| c[1].to_string())
        .expect("TLS_CRYPTO_PROVIDER not found")
}

/// The constant an operator reads must equal what `main` installs.
#[test]
fn the_reported_provider_is_the_installed_provider() {
    assert_eq!(
        canon(&posture_provider()),
        canon(&installed_provider()),
        "doctor reports a provider the binary does not install"
    );
}

/// No document may name a provider other than the one in the constant.
#[test]
fn no_document_names_a_foreign_provider() {
    let want = posture_provider().to_lowercase();
    let re = Regex::new(r#"(?i)provider\s*=\s*[`"']?([a-z0-9-]+)"#).expect("static regex");
    let mut problems = Vec::new();
    for (rel, text) in prose_and_source() {
        for (n, line) in numbered(&text) {
            for cap in re.captures_iter(line) {
                let got = cap[1].to_lowercase();
                if got != want {
                    problems.push(format!(
                        "{rel}:{n}: names provider={got} but the binary installs {want}"
                    ));
                }
            }
        }
    }
    assert_clean("provider naming", &problems);
}

/// No document may accept, or show installed, a provider the binary does not.
///
/// The gate above compares `provider = X` and reported green while ADR 0007 — the
/// normative TLS document — marked `rustls-rustcrypto` as **accepted** and showed
/// `rustls_rustcrypto::provider().install_default()` in a code span. None of those
/// sentences has the form `provider = ...`; the drift hid in a table cell.
///
/// Both bootstrap spellings are matched. The first version of this gate matched
/// `X::provider()` only, which catches `rustls_rustcrypto::provider()` and misses
/// `rustls_graviola::default_provider()` — the exact form the abandoned pure-Rust
/// plan proposed. A gate that only recognises the mistake already made is not a
/// gate, it is a memorial.
#[test]
fn no_document_accepts_or_installs_a_foreign_provider() {
    let want = canon(&installed_provider());
    let known = known_providers();
    let accepted =
        Regex::new(r"(?i)\*\*([a-z0-9_-]+)\*\*\s*\((?:accepted|aceito)\b").expect("static regex");
    let installs =
        Regex::new(r"(?i)([a-z0-9_]+)::(?:default_provider|provider)\(\)").expect("static regex");
    let mut problems = Vec::new();

    for (rel, text) in current_docs() {
        for (n, line) in numbered(&text) {
            if let Some(cap) = accepted.captures(line) {
                let got = canon(&cap[1]);
                if known.contains(&got) && got != want {
                    problems.push(format!(
                        "{rel}:{n}: accepts provider {} but src/main.rs installs {want}",
                        &cap[1]
                    ));
                }
            }
            for cap in installs.captures_iter(line) {
                let got = canon(&cap[1]);
                if known.contains(&got) && got != want {
                    problems.push(format!(
                        "{rel}:{n}: shows {} being installed but src/main.rs installs {want}",
                        &cap[1]
                    ));
                }
            }
        }
    }
    assert_clean("provider acceptance", &problems);
}

/// Words that make a line an assertion about the provider in force right now.
const ACTIVE_VOICE: &[&str] = &[
    "install",
    "instala",
    "bootstrap",
    "cryptoprovider",
    "in use",
    "em uso",
    "active",
    "ativo",
    "current",
    "atual",
    "ships",
];

/// No line may name a foreign provider as the one in force.
///
/// The two gates above compare *shapes*: one wants `provider = X`, the other
/// wants `**X** (accepted)` or `X::provider()`. Both reported green while
/// `src/main.rs:32` — the bootstrap comment sitting ten lines above the call —
/// read `install process CryptoProvider once (rustcrypto; …)` while line 42
/// installed `ring`. The word was loose inside parentheses, so it matched no
/// shape, and `canon` already folded `rustcrypto` onto one spelling without
/// anything ever asking it this question.
///
/// The aggravating detail is where the wrong name lived: `src/main.rs` is the
/// file `installed_provider()` reads to decide what the truth *is*. The source
/// of truth contradicted itself and every gate derived from it agreed.
///
/// This gate matches the bare name plus active voice, which is the class, and
/// exempts `DISCLAIMS`, because naming a refused candidate is the record working.
#[test]
fn no_line_names_a_foreign_provider_as_the_active_one() {
    let want = canon(&installed_provider());
    let foreign: Vec<String> = known_providers()
        .into_iter()
        .filter(|p| *p != want)
        .collect();
    assert!(
        !foreign.is_empty(),
        "every known provider equals the installed one; the vocabulary is broken"
    );

    let mut problems = Vec::new();
    for (rel, text) in prose_and_source() {
        // ADR 0007 exists to weigh candidates by name; that is its whole job.
        if rel.starts_with("docs/decisions/") {
            continue;
        }
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            if !ACTIVE_VOICE.iter().any(|v| lower.contains(v)) {
                continue;
            }
            if DISCLAIMS.iter().any(|d| lower.contains(d)) {
                continue;
            }
            for p in &foreign {
                let word = Regex::new(&format!(r"\b{}\b", regex::escape(p))).expect("static regex");
                if word.is_match(&lower) {
                    problems.push(format!(
                        "{rel}:{n}: names {p} as the provider in force, but the binary installs {want}"
                    ));
                }
            }
        }
    }
    assert_clean("active provider naming", &problems);

    // The sentence this gate was written for, kept as a canary. Without it, a
    // predicate that stopped matching would be indistinguishable from a clean
    // tree — which is exactly how `no_document_miscounts_the_schema_bundle`
    // passed for weeks with a regex that matched nothing at all.
    let bait = "// 1) install process CryptoProvider once (rustcrypto; reqwest rustls-no-provider)"
        .to_lowercase();
    assert!(
        ACTIVE_VOICE.iter().any(|v| bait.contains(v))
            && !DISCLAIMS.iter().any(|d| bait.contains(d))
            && foreign.iter().any(|p| bait.contains(p.as_str())),
        "the active-provider predicate no longer recognises the line it exists for"
    );
}

/// Trust-root sources the client could plausibly build from.
const ROOT_SOURCES: &[(&str, &str)] = &[
    ("webpki_roots", "webpki-roots"),
    ("rustls_platform_verifier", "rustls-platform-verifier"),
];

/// The active trust-root source, read from the module that builds the store.
///
/// `rustls-platform-verifier` is still in `Cargo.lock`, forced there by reqwest's
/// feature wiring, so "present in the lock" proves nothing about what is used.
fn active_trust_root() -> &'static str {
    let tls = read("src/http/tls.rs");
    let active: Vec<&str> = ROOT_SOURCES
        .iter()
        .filter(|(ident, _)| tls.contains(&format!("{ident}::")))
        .map(|(_, wire)| *wire)
        .collect();
    assert_eq!(
        active.len(),
        1,
        "src/http/tls.rs must build from exactly one trust-root source, found {active:?}"
    );
    active[0]
}

/// Nothing may name a trust-root source other than the one the client builds.
///
/// reqwest 0.13 removed every webpki-roots feature and made `rustls-no-provider`
/// pull `rustls-platform-verifier` unconditionally, so the 0.12 upgrade moved the
/// product's anchors from a compiled-in Mozilla set to the operating system store
/// and nothing said so. `http_client_posture` — the string an operator reads to
/// audit exactly this — printed `webpki-roots` from a hard-coded literal. The
/// binary misreported its own trust anchors.
///
/// The posture constant is scanned alongside the prose because it *is* prose; it
/// just happens to be compiled in and handed to operators as fact.
#[test]
fn nothing_names_a_stale_trust_root_source() {
    let want = active_trust_root();
    let stale: Vec<&str> = ROOT_SOURCES
        .iter()
        .map(|(_, wire)| *wire)
        .filter(|w| *w != want)
        .collect();

    let scoped = prose_and_source();

    // Naming the stale crate is only a defect when the sentence presents it as
    // where trust comes from. Both crates legitimately appear in other roles:
    // `rustls-platform-verifier` sits in `Cargo.lock` unconditionally and drags
    // Apple SDK frameworks into the link step, which is a fact about
    // cross-compilation, not about anchors. A gate that cannot tell the two apart
    // forces documents to omit a true cause in order to stay green.
    const TRUST_CONTEXT: &[&str] = &[
        "root", "raiz", "raíz", "anchor", "âncora", "ancora", "trust", "confian",
    ];
    let mut problems = Vec::new();
    for (rel, text) in scoped {
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            let negated = [
                "not ",
                "não ",
                "nao ",
                "never",
                "nunca",
                "nenhum",
                "no code path",
            ]
            .iter()
            .any(|m| lower.contains(m));
            let about_trust = TRUST_CONTEXT.iter().any(|m| lower.contains(m));
            for s in &stale {
                if line.contains(s) && about_trust && !negated {
                    problems.push(format!(
                        "{rel}:{n}: names {s} as a trust-root source but src/http/tls.rs builds from {want}"
                    ));
                }
            }
        }
    }
    assert_clean("trust-root naming", &problems);
}

/// The posture string must report the trust-root source the client builds from.
///
/// This is the half of GAP-TLS-ROOTS-001 that stayed open: the source moved into
/// `src/http/tls.rs` and got a gate, while the operator-facing string kept its
/// literal. A literal that describes the dependency graph without reading it is
/// correct only by coincidence, and the coincidence ends at the next upgrade.
#[test]
fn the_posture_string_reports_the_active_trust_root() {
    let want = active_trust_root();
    let src = read("src/http/content_type.rs");
    let re = Regex::new(r#"TLS_TRUST_ANCHORS: &str = "([^"]+)""#).expect("static regex");
    let declared = re
        .captures(&src)
        .map(|c| c[1].to_string())
        .expect("TLS_TRUST_ANCHORS constant not found in src/http/content_type.rs");
    assert_eq!(
        declared, want,
        "the posture constant names a trust-root source the client does not build from"
    );
    assert!(
        src.contains("{TLS_TRUST_ANCHORS}"),
        "client_posture_detail must interpolate TLS_TRUST_ANCHORS, not a literal"
    );
}

/// The declared rustls floor must equal the pin the manifest actually carries.
///
/// `RUSTLS_VERSION_FLOOR` was a literal `0.23.18` sitting beside a manifest pin of
/// `0.23.18`: equal by coincidence, with nothing keeping them equal. It is the
/// same class as the `webpki-roots` literal — a string asserting something about
/// `Cargo.toml` without reading it — and it was worse defended, because a unit
/// test asserted the literal against itself and looked like coverage.
#[test]
fn the_declared_rustls_floor_matches_the_manifest_pin() {
    let src = read("src/http/content_type.rs");
    let declared = Regex::new(r#"RUSTLS_VERSION_FLOOR: &str = "([^"]+)""#)
        .expect("static regex")
        .captures(&src)
        .map(|c| c[1].to_string())
        .expect("RUSTLS_VERSION_FLOOR not found");

    let manifest = read("Cargo.toml");
    let pinned = Regex::new(r#"rustls = \{ version = "([^"]+)""#)
        .expect("static regex")
        .captures(&manifest)
        .map(|c| c[1].to_string())
        .expect("no direct rustls pin in Cargo.toml");

    assert_eq!(
        declared, pinned,
        "the posture floor and the manifest pin disagree"
    );
}

/// Nothing may call the active provider pure Rust while it builds C.
///
/// Every provider gate above compares a *name*: `provider=ring` equals `ring`,
/// so all of them reported green while `docs/CROSS_PLATFORM.md:49` said
/// ``TLS is rustls only (`provider=ring`, pure Rust); no OpenSSL and no C
/// toolchain``. The name was right and the sentence was false, and it sat in the
/// **macOS** section — the one target this repository cannot cross-compile
/// precisely because of a toolchain. A reader following that line would conclude
/// the opposite of what the same file says fifteen lines above it.
///
/// Identifying an object is not characterising it. This gate reads the posture
/// constant, and if it declares a C build dependency, no sentence naming the
/// provider may also claim purity.
#[test]
fn nothing_calls_the_c_building_provider_pure_rust() {
    let posture = read("src/http/content_type.rs");
    let declared = Regex::new(r#"TLS_PROVIDER_POSTURE: &str =\s*\n?\s*"([^"]+)""#)
        .expect("static regex")
        .captures(&posture)
        .map(|c| c[1].to_string())
        .expect("TLS_PROVIDER_POSTURE not found");
    if !declared.contains("c-build-dep") {
        return;
    }

    let provider = installed_provider().to_lowercase();
    const PURITY: &[&str] = &[
        "pure rust",
        "pure-rust",
        "puro rust",
        "no c toolchain",
        "sem toolchain c",
        "no c compiler",
        "sem compilador c",
    ];
    let mut problems = Vec::new();
    for (rel, text) in prose_and_source() {
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            if !lower.contains(&provider) {
                continue;
            }
            if DISCLAIMS.iter().any(|d| lower.contains(d)) {
                continue;
            }
            if let Some(claim) = PURITY.iter().find(|p| lower.contains(**p)) {
                problems.push(format!(
                    "{rel}:{n}: calls {provider} \"{claim}\" while the posture declares {declared}"
                ));
            }
        }
    }
    assert_clean("provider purity claims", &problems);

    // A gate that matches nothing is indistinguishable from a clean repository,
    // and this file already shipped one such gate for weeks. The sentence below
    // is the real defect this gate was written for, kept as a canary.
    let bait = "- TLS is rustls only (`provider=ring`, pure Rust); no OpenSSL and no C toolchain";
    let lower = bait.to_lowercase();
    assert!(
        lower.contains(&provider)
            && PURITY.iter().any(|p| lower.contains(p))
            && !DISCLAIMS.iter().any(|d| lower.contains(d)),
        "the purity predicate no longer recognises the sentence it exists for"
    );
}

/// An ADR may not describe a builder call the client does not make.
///
/// ADR 0007 section 3 described `.use_rustls_tls()` and `.min_tls_version(...)`
/// for a month after the client moved to `.use_preconfigured_tls(...)`, which
/// makes `min_tls_version` inert. A reader following the ADR would have set a
/// floor that does nothing.
#[test]
fn no_adr_describes_a_builder_call_the_client_does_not_make() {
    const WATCHED: &[&str] = &[
        ".use_rustls_tls()",
        ".min_tls_version(",
        ".use_preconfigured_tls(",
    ];
    let client = read("src/http/client.rs");
    let mut problems = Vec::new();

    for (rel, text) in current_docs() {
        if !rel.starts_with("docs/decisions/") {
            continue;
        }
        for (n, line) in numbered(&text) {
            let lower = line.to_lowercase();
            let negated = ["not ", "não ", "nao ", "never", "nunca", "inert", "inerte"]
                .iter()
                .any(|m| lower.contains(m));
            for call in WATCHED {
                if line.contains(call) && !client.contains(call) && !negated {
                    problems.push(format!(
                        "{rel}:{n}: describes {call} but src/http/client.rs does not call it"
                    ));
                }
            }
        }
    }
    assert_clean("ADR builder calls", &problems);
}
