//! SSRF host/origin allowlist helpers (no domain dependency — used by `AllowedOrigin`).

use super::constants::{
    HOST_CRATES_IO, HOST_DOC_RUST_LANG_ORG, HOST_DOCS_RS, HOST_LOCALHOST, HOST_LOOPBACK_IPV4,
    HOST_STATIC_DOCS_RS, SCHEME_HTTPS,
};

/// Strip trailing slash from an origin base URL.
pub fn normalize_origin(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

/// True when `host` is a product HTTP allowlist entry (SSRF gate).
///
/// Production: `crates.io`, `docs.rs`, `static.docs.rs`, `doc.rust-lang.org`.
/// Loopback (`127.0.0.1` / `localhost`) only when `allow_loopback` is true
/// (CLI `--allow-loopback` or XDG TOML `allow_loopback = true` — never env).
pub fn is_allowlisted_host(host: &str, allow_loopback: bool) -> bool {
    matches!(
        host,
        HOST_CRATES_IO | HOST_DOCS_RS | HOST_STATIC_DOCS_RS | HOST_DOC_RUST_LANG_ORG
    ) || (allow_loopback && (host == HOST_LOOPBACK_IPV4 || host == HOST_LOCALHOST))
}

/// True when scheme+host is a legal origin for product config (defense in depth).
///
/// Production allowlisted hosts require `https`. Loopback (when `allow_loopback`)
/// allows `http` or `https` for offline mocks.
pub fn is_allowed_origin_scheme_host(scheme: &str, host: &str, allow_loopback: bool) -> bool {
    if !is_allowlisted_host(host, allow_loopback) {
        return false;
    }
    if host == HOST_LOOPBACK_IPV4 || host == HOST_LOCALHOST {
        return scheme == "http" || scheme == "https";
    }
    scheme == SCHEME_HTTPS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_requires_explicit_flag() {
        assert!(!is_allowlisted_host(HOST_LOOPBACK_IPV4, false));
        assert!(!is_allowlisted_host(HOST_LOCALHOST, false));
        assert!(is_allowlisted_host(HOST_LOOPBACK_IPV4, true));
        assert!(is_allowlisted_host(HOST_LOCALHOST, true));
        assert!(is_allowlisted_host(HOST_DOCS_RS, false));
        assert!(!is_allowlisted_host("evil.example", true));
    }

    #[test]
    fn production_hosts_require_https() {
        assert!(is_allowed_origin_scheme_host("https", HOST_DOCS_RS, false));
        assert!(!is_allowed_origin_scheme_host("http", HOST_DOCS_RS, false));
        assert!(is_allowed_origin_scheme_host(
            "http",
            HOST_LOOPBACK_IPV4,
            true
        ));
        assert!(!is_allowed_origin_scheme_host(
            "http",
            HOST_LOOPBACK_IPV4,
            false
        ));
    }
}
