//! Content-Type sniffing and fail-closed require helpers.

use crate::error::{AppError, AppResult, ContentKind, ErrorDetail};

use super::constants::{
    POOL_IDLE_TIMEOUT_SECS, POOL_MAX_IDLE_PER_HOST, TCP_KEEPALIVE_IDLE_SECS,
    TCP_KEEPALIVE_INTERVAL_SECS, TCP_KEEPALIVE_RETRIES,
};

/// ASCII case-insensitive substring check without heap allocation.
///
/// Used on Content-Type sniffing (hot path after every successful GET) so we
/// never build a lowercased temporary `String` for a few-byte header.
pub(super) fn ascii_contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let n = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(n.len())
        .any(|w| w.eq_ignore_ascii_case(n))
}

/// True when Content-Type looks like JSON.
pub fn content_type_looks_json(ct: Option<&str>) -> bool {
    ct.is_some_and(|c| ascii_contains_ignore_case(c, "json"))
}

/// True when Content-Type looks like HTML/XHTML.
pub fn content_type_looks_html(ct: Option<&str>) -> bool {
    ct.is_some_and(|c| {
        ascii_contains_ignore_case(c, "html") || ascii_contains_ignore_case(c, "xhtml")
    })
}

/// Fail closed when a Content-Type is present but not JSON.
///
/// Missing Content-Type is allowed (some caches/mocks omit it); body parse still
/// validates structure. Present-but-wrong types are rejected to reduce MIME confusion.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Parse`] when `ct` is present and does not look like JSON.
pub fn require_content_type_json(ct: Option<&str>) -> AppResult<()> {
    match ct {
        None => Ok(()),
        Some(c) if content_type_looks_json(Some(c)) => Ok(()),
        Some(c) => Err(AppError::of(ErrorDetail::UnexpectedContentType {
            expected: ContentKind::Json,
            got: c.to_string(),
        })),
    }
}

/// Fail closed when a Content-Type is present but not HTML/XHTML.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Parse`] when `ct` is present and does not look like HTML.
pub fn require_content_type_html(ct: Option<&str>) -> AppResult<()> {
    match ct {
        None => Ok(()),
        Some(c) if content_type_looks_html(Some(c)) => Ok(()),
        Some(c) => Err(AppError::of(ErrorDetail::UnexpectedContentType {
            expected: ContentKind::Html,
            got: c.to_string(),
        })),
    }
}

/// Floor for the direct `rustls` dependency pin (ADR 0007; RUSTSEC-2024-0399 era).
///
/// A policy gate compares this against the pin in `Cargo.toml`. It used to be a
/// bare literal sitting beside an equal literal in the manifest — equal by
/// coincidence, with nothing keeping them equal — and a unit test asserted the
/// constant against itself, which reads as coverage and verifies nothing.
pub const RUSTLS_VERSION_FLOOR: &str = "0.23.18";

/// Trust anchors the HTTP client validates against.
///
/// This is a constant rather than a word inside the posture string because the
/// word was wrong for a month. reqwest 0.13 removed every webpki-roots feature,
/// so upgrading from 0.12 moved the anchors to the operating system store while
/// this string kept printing `webpki-roots` — the binary misreported its own
/// trust anchors in the very line an operator reads to audit them
/// (GAP-TLS-ROOTS-001). A policy gate now derives the truth from
/// `src/http/tls.rs`, which is the module that builds the root store, and it can
/// only reach this fact if the fact is a named constant.
pub const TLS_TRUST_ANCHORS: &str = "webpki-roots";

/// Crypto provider installed by the binary through `rustls-no-provider`.
///
/// Not selected by a reqwest feature: `Cargo.toml` uses `rustls-no-provider`
/// precisely so no provider is chosen for us, and `src/main.rs` installs this one
/// as the process default.
///
/// `ring` compiles C through `cc-rs`, which contradicts the self-contained,
/// Rust-native product rule. Both pure-Rust replacements were evaluated in
/// 2026-08 and both were rejected on measurement; see [`TLS_PROVIDER_POSTURE`]
/// and ADR 0007. The non-conformance is tracked as GAP-TOOLCHAIN-001.
pub const TLS_CRYPTO_PROVIDER: &str = "ring";

/// Why the provider is not pure Rust, in one machine-readable token.
///
/// Surfaced through `doctor` rather than left to the source, because "this CLI
/// builds C" is a fact an operator auditing a supposedly Rust-native binary is
/// entitled to read from the binary itself. The two rejected candidates failed
/// for opposite reasons: `graviola` needs x86_64 `adx` (Broadwell, 2015+) and
/// aborts at the first handshake without it, while `rustls-rustcrypto` pins
/// `rustls-webpki ^0.102`, which carries four certificate-validation advisories
/// plus an unpatched `rsa` timing sidechannel.
pub const TLS_PROVIDER_POSTURE: &str =
    "c-build-dep(ring); pure-rust-blocked(graviola:needs-adx, rustcrypto:webpki-advisories)";

/// Machine-readable summary of the compiled HTTP client posture (doctor / docs).
///
/// Single source for agent audits — README/SECURITY cite this string via `doctor`.
pub fn client_posture_detail() -> String {
    format!(
        "rustls≥{RUSTLS_VERSION_FLOOR} provider={TLS_CRYPTO_PROVIDER} posture={TLS_PROVIDER_POSTURE} TLS≥1.2 {TLS_TRUST_ANCHORS} http2 system-proxy tcp_nodelay keepalive={TCP_KEEPALIVE_IDLE_SECS}s/{TCP_KEEPALIVE_INTERVAL_SECS}s/r{TCP_KEEPALIVE_RETRIES} pool_idle={POOL_MAX_IDLE_PER_HOST}@{POOL_IDLE_TIMEOUT_SECS}s host-allowlist no-danger-tls"
    )
}
