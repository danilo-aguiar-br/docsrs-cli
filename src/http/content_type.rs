//! Content-Type sniffing and fail-closed require helpers.

use crate::error::{AppError, AppResult, ErrorKind};

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
/// Returns [`ErrorKind::Parse`] when `ct` is present and does not look like JSON.
pub fn require_content_type_json(ct: Option<&str>) -> AppResult<()> {
    match ct {
        None => Ok(()),
        Some(c) if content_type_looks_json(Some(c)) => Ok(()),
        Some(c) => Err(AppError::new(
            ErrorKind::Parse,
            format!("unexpected Content-Type for JSON response: {c}"),
        )),
    }
}

/// Fail closed when a Content-Type is present but not HTML/XHTML.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when `ct` is present and does not look like HTML.
pub fn require_content_type_html(ct: Option<&str>) -> AppResult<()> {
    match ct {
        None => Ok(()),
        Some(c) if content_type_looks_html(Some(c)) => Ok(()),
        Some(c) => Err(AppError::new(
            ErrorKind::Parse,
            format!("unexpected Content-Type for HTML response: {c}"),
        )),
    }
}

/// Floor for the direct `rustls` dependency pin (ADR 0007; RUSTSEC-2024-0399 era).
pub const RUSTLS_VERSION_FLOOR: &str = "0.23.18";

/// Crypto provider installed by the binary and selected by reqwest `rustls-tls`.
pub const TLS_CRYPTO_PROVIDER: &str = "ring";

/// Machine-readable summary of the compiled HTTP client posture (doctor / docs).
///
/// Single source for agent audits — README/SECURITY cite this string via `doctor`.
pub fn client_posture_detail() -> String {
    format!(
        "rustls≥{RUSTLS_VERSION_FLOOR} provider={TLS_CRYPTO_PROVIDER} TLS≥1.2 webpki-roots http2 system-proxy tcp_nodelay keepalive={TCP_KEEPALIVE_IDLE_SECS}s/{TCP_KEEPALIVE_INTERVAL_SECS}s/r{TCP_KEEPALIVE_RETRIES} pool_idle={POOL_MAX_IDLE_PER_HOST}@{POOL_IDLE_TIMEOUT_SECS}s host-allowlist no-danger-tls"
    )
}
