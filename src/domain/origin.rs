//! Allowlisted HTTP(S) origin (SSRF gate proof in the type).

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use url::Url;

use crate::config::{
    HOST_CRATES_IO, HOST_DOC_RUST_LANG_ORG, HOST_DOCS_RS, HOST_STATIC_DOCS_RS, SCHEME_HTTPS,
    is_allowed_origin_scheme_host, normalize_origin,
};
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp, Subject};

/// HTTPS (or gated loopback) origin that passed the product SSRF allowlist.
///
/// Construct only via [`AllowedOrigin::parse`] / [`AllowedOrigin::parse_with`].
/// Field is private so callers cannot assign an arbitrary host without
/// re-validation (TYPE-L-004 / ADR 0006).
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AllowedOrigin(String);

impl AllowedOrigin {
    /// Parse and normalize an HTTP(S) origin with production allowlist
    /// (`allow_loopback = false`).
    ///
    /// For offline wiremock / loopback origins use [`Self::parse_with`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when the string is not a valid `http`/`https` URL
    /// with a host (empty, bare path, unsupported scheme, or non-allowlisted host).
    pub fn parse(raw: &str) -> AppResult<Self> {
        Self::parse_with(raw, false)
    }

    /// Parse and normalize an HTTP(S) origin (scheme + host, no path required).
    ///
    /// Hosts must pass the product SSRF allowlist. When `allow_loopback` is true,
    /// `127.0.0.1` / `localhost` with `http` or `https` are accepted (CLI
    /// `--allow-loopback` or XDG TOML `allow_loopback = true` — never env).
    /// Arbitrary origins such as `https://evil.example` are always rejected.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when the string is not a valid `http`/`https` URL
    /// with a host (empty, bare path, unsupported scheme, or non-allowlisted host).
    pub fn parse_with(raw: &str, allow_loopback: bool) -> AppResult<Self> {
        let normalized = normalize_origin(raw);
        if normalized.is_empty() {
            return Err(AppError::of(ErrorDetail::Empty {
                subject: Subject::Origin,
            }));
        }
        let url = url::Url::parse(&normalized).map_err(|e| {
            AppError::of_with_source(
                ErrorDetail::Invalid {
                    subject: Subject::Origin,
                    value: normalized.to_string(),
                },
                e,
            )
        })?;
        let scheme = url.scheme();
        if scheme != "http" && scheme != "https" {
            return Err(AppError::of(ErrorDetail::OriginBadScheme {
                scheme: scheme.to_string(),
            }));
        }
        let Some(host) = url.host_str() else {
            return Err(AppError::of(ErrorDetail::OriginMissingHost {
                url: normalized.to_string(),
            }));
        };
        if !is_allowed_origin_scheme_host(scheme, host, allow_loopback) {
            return Err(AppError::of(ErrorDetail::OriginNotAllowlisted {
                host: host.to_string(),
                allowed: format!(
                    "{HOST_CRATES_IO}, {HOST_DOCS_RS}, {HOST_STATIC_DOCS_RS}, {HOST_DOC_RUST_LANG_ORG}"
                ),
            }));
        }
        // Drop path/query/fragment so origins stay host-level bases.
        let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
        Ok(Self(format!("{scheme}://{host}{port}")))
    }

    /// Default crates.io API origin (valid by construction).
    #[must_use]
    pub fn crates_io_default() -> Self {
        Self(format!("{SCHEME_HTTPS}://{HOST_CRATES_IO}"))
    }

    /// Default docs.rs origin (valid by construction).
    #[must_use]
    pub fn docs_rs_default() -> Self {
        Self(format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}"))
    }

    /// Default stdlib docs origin on doc.rust-lang.org (valid by construction).
    #[must_use]
    pub fn stdlib_docs_default() -> Self {
        Self(format!("{SCHEME_HTTPS}://{HOST_DOC_RUST_LANG_ORG}"))
    }

    /// Borrow the origin string (no trailing slash).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Re-parse this validated origin into a WHATWG [`Url`] (fail-closed).
    ///
    /// Used by URL builders that need a typed base. The stored string was produced
    /// by [`Self::parse`] (or a default constructor), so failure is treated as an
    /// internal invariant break rather than user input (ADR 0008).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Internal`] if the stored origin is not a valid URL
    /// (should not happen for values constructed via this type).
    pub fn to_url(&self) -> AppResult<Url> {
        Url::parse(self.as_str()).map_err(|e| {
            AppError::of_with_source(
                ErrorDetail::Internal {
                    op: InternalOp::AllowedOriginUnparseable,
                },
                e,
            )
        })
    }
}

impl AsRef<str> for AllowedOrigin {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for AllowedOrigin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for AllowedOrigin {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for AllowedOrigin {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_https_and_rejects_garbage() {
        assert_eq!(
            AllowedOrigin::parse("https://docs.rs/").unwrap().as_str(),
            format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}")
        );
        assert!(AllowedOrigin::parse("https://evil.example").is_err());
        assert!(AllowedOrigin::parse("not-a-url").is_err());
        assert!(AllowedOrigin::parse("").is_err());
        assert!(AllowedOrigin::parse("http://127.0.0.1:9").is_err());
        assert_eq!(
            AllowedOrigin::parse_with("http://127.0.0.1:9/", true)
                .unwrap()
                .as_str(),
            "http://127.0.0.1:9"
        );
    }

    #[test]
    fn defaults_round_trip() {
        assert_eq!(
            AllowedOrigin::crates_io_default().as_str(),
            format!("{SCHEME_HTTPS}://{HOST_CRATES_IO}")
        );
        assert_eq!(
            AllowedOrigin::docs_rs_default().as_str(),
            format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}")
        );
    }

    #[test]
    fn to_url_round_trips_defaults() {
        let o = AllowedOrigin::docs_rs_default();
        let u = o.to_url().unwrap();
        assert_eq!(u.scheme(), SCHEME_HTTPS);
        assert_eq!(u.host_str(), Some(HOST_DOCS_RS));
        assert_eq!(u.as_str().trim_end_matches('/'), o.as_str());
    }
}
