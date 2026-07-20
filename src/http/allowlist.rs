//! SSRF host allowlist gate for request and cached final URLs.

use reqwest::Url;

/// Product SSRF gate for request URLs (and cached `final_url` provenance).
///
/// Delegates host membership to [`crate::config::is_allowlisted_host`] and
/// requires `https` for production hosts (loopback may use `http` when
/// `allow_loopback` is true — CLI/XDG only).
pub(crate) fn is_allowed_host(url: &Url, allow_loopback: bool) -> bool {
    let Some(host) = url.host_str() else {
        return false;
    };
    crate::config::is_allowed_origin_scheme_host(url.scheme(), host, allow_loopback)
}
