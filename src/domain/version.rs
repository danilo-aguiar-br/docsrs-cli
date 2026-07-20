//! Validated crate version token.

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;

use crate::config::MAX_VERSION_CHARS;
use crate::error::{AppError, AppResult, ErrorKind};

use super::regex::compile_bounded_regex;

/// Validated crate version token (`latest`, channel, or SemVer without `v` prefix).
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VersionArg(String);

impl VersionArg {
    /// Parse optional CLI version; empty/`None` → `latest`.
    ///
    /// # Errors
    ///
    /// Propagates [`ErrorKind::InvalidInput`] from [`Self::parse`] when the
    /// provided token is present and invalid.
    pub fn parse_opt(raw: Option<&str>) -> AppResult<Self> {
        let v = raw
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("latest");
        Self::parse(v)
    }

    /// Parses a non-optional version token.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for unknown channels, `v`-prefixed
    /// SemVer, build metadata, or malformed versions.
    pub fn parse(v: &str) -> AppResult<Self> {
        let v = v.trim();
        if v.chars().count() > MAX_VERSION_CHARS {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("version exceeds {MAX_VERSION_CHARS} characters"),
            ));
        }
        if v == "latest" {
            return Ok(Self(v.to_string()));
        }
        if matches!(v, "stable" | "beta" | "nightly") {
            return Ok(Self(v.to_string()));
        }
        if v.starts_with('v') || v.starts_with('V') {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "version must not start with 'v' prefix",
            ));
        }
        if v.contains('+') {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "version build metadata is not accepted",
            ));
        }
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            compile_bounded_regex(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")
                .expect("hardcoded semver-ish version regex is valid by construction")
        });
        if !RE.is_match(v) {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("invalid version '{v}'"),
            ));
        }
        Ok(Self(v.to_string()))
    }

    /// Borrow the version token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Map to rustdoc channel for stdlib paths (`latest` → `stable`).
    ///
    /// Single source of truth for channel mapping (TYPE-L-010).
    #[must_use]
    pub fn stdlib_channel(&self) -> &str {
        match self.as_str() {
            "latest" => "stable",
            other => other,
        }
    }
}

impl AsRef<str> for VersionArg {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for VersionArg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for VersionArg {
    type Err = AppError;

    /// Parses via [`VersionArg::parse`].
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for unknown channels or malformed versions.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn version_arg_policy() {
        assert_eq!(VersionArg::parse_opt(None).unwrap().as_str(), "latest");
        assert_eq!(VersionArg::parse_opt(Some("")).unwrap().as_str(), "latest");
        assert_eq!(VersionArg::parse("1.0.0").unwrap().as_str(), "1.0.0");
        assert_eq!(
            VersionArg::parse("stable").unwrap().stdlib_channel(),
            "stable"
        );
        assert_eq!(
            VersionArg::parse("latest").unwrap().stdlib_channel(),
            "stable"
        );
        assert!(VersionArg::parse("v1.0.0").is_err());
        assert!(VersionArg::parse("1.0.0+build").is_err());
        assert!(VersionArg::parse("not-a-version").is_err());
    }

    #[test]
    fn version_arg_transparent_layout() {
        assert_eq!(size_of::<VersionArg>(), size_of::<String>());
    }
}
