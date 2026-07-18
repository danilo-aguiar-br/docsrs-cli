//! Domain newtypes: parse at the boundary, carry validity in the type.
//!
//! Follows parse-don't-validate: only fallible constructors build values.

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;

use crate::config::{
    MAX_CRATE_NAME_CHARS, MAX_ITEM_PATH_CHARS, MAX_QUERY_CHARS, MAX_VERSION_CHARS,
};
use crate::error::{AppError, AppResult, ErrorKind};

/// Validated crates.io / stdlib crate name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CrateName(String);

impl CrateName {
    /// Parses and validates a crate name (trim, length, charset).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when the name is empty, too long, or
    /// contains characters outside the crates.io charset.
    ///
    /// # Examples
    ///
    /// ```
    /// use docsrs_cli::domain::CrateName;
    ///
    /// let name = CrateName::parse("tokio").expect("valid");
    /// assert_eq!(name.as_str(), "tokio");
    /// assert!(CrateName::parse("").is_err());
    /// ```
    pub fn parse(name: &str) -> AppResult<Self> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "crate name is empty",
            ));
        }
        if name.chars().count() > MAX_CRATE_NAME_CHARS {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("crate name exceeds {MAX_CRATE_NAME_CHARS} characters"),
            ));
        }
        static RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
                .expect("hardcoded crate-name regex is valid by construction")
        });
        if !RE.is_match(name) {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("invalid crate name '{name}'"),
            ));
        }
        Ok(Self(name.to_string()))
    }

    /// Borrow the validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for `std` / `core` / `alloc` (docs on doc.rust-lang.org).
    pub fn is_stdlib(&self) -> bool {
        matches!(self.0.as_str(), "std" | "core" | "alloc")
    }
}

impl AsRef<str> for CrateName {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for CrateName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrateName {
    type Err = AppError;

    /// Parses via [`CrateName::parse`].
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for empty, oversized, or illegal names.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl TryFrom<&str> for CrateName {
    type Error = AppError;

    /// Converts via [`CrateName::parse`].
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for empty, oversized, or illegal names.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

/// Validated rustdoc item path (`tokio::runtime::Runtime` or `runtime/Runtime`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ItemPath {
    /// Original trimmed input (preserves `::` vs `/` only for Display of raw when needed).
    raw: String,
    /// Normalized path segments.
    segments: Vec<String>,
}

impl ItemPath {
    /// Parses an item path with `::` or `/` separators.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when the path is empty, too long, or
    /// contains `.` / `..` segments.
    pub fn parse(path: &str) -> AppResult<Self> {
        let path = path.trim();
        if path.is_empty() || path == "::" || path == "/" {
            return Err(AppError::new(ErrorKind::InvalidInput, "item path is empty"));
        }
        if path.chars().count() > MAX_ITEM_PATH_CHARS {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("item path exceeds {MAX_ITEM_PATH_CHARS} characters"),
            ));
        }
        let normalized = path.replace('/', "::");
        let parts: Vec<String> = normalized
            .split("::")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if parts.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "item path has no segments",
            ));
        }
        for seg in &parts {
            if seg == "." || seg == ".." {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    format!("invalid item path segment '{seg}'"),
                ));
            }
            if seg.chars().any(|c| c.is_whitespace()) {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    format!("item path segment contains whitespace: '{seg}'"),
                ));
            }
            if !seg.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "invalid item path segment '{seg}' (use letters, digits, underscore; separate with :: or /)"
                    ),
                ));
            }
        }
        Ok(Self {
            raw: path.to_string(),
            segments: parts,
        })
    }

    /// Path segments after normalization.
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Owned segment list (for APIs that still take `Vec<String>`).
    pub fn into_segments(self) -> Vec<String> {
        self.segments
    }

    /// Original trimmed input string.
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl AsRef<str> for ItemPath {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for ItemPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.segments.join("::"))
    }
}

impl FromStr for ItemPath {
    type Err = AppError;

    /// Parses via [`ItemPath::parse`].
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for empty, oversized, or illegal paths.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Validated search query string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchQuery(String);

impl SearchQuery {
    /// Parses a search query.
    ///
    /// When `allow_empty` is true, whitespace-only becomes an empty query (list-all).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when the query is empty without
    /// `allow_empty`, or when it exceeds the maximum length.
    pub fn parse(query: &str, allow_empty: bool) -> AppResult<Self> {
        let q = query.trim();
        if !allow_empty && q.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "search query is empty",
            ));
        }
        if q.chars().count() > MAX_QUERY_CHARS {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("query exceeds {MAX_QUERY_CHARS} characters"),
            ));
        }
        Ok(Self(q.to_string()))
    }

    /// Borrow the validated query.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SearchQuery {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for SearchQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Validated crate version token (`latest`, channel, or SemVer without `v` prefix).
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
            Regex::new(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")
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
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Map to rustdoc channel for stdlib paths (`latest` → `stable`).
    pub fn stdlib_channel(&self) -> &str {
        crate::config::stdlib_channel(self.as_str())
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

    #[test]
    fn crate_name_ok_and_stdlib() {
        let s = CrateName::parse("serde").unwrap();
        assert_eq!(s.as_str(), "serde");
        assert!(!s.is_stdlib());
        assert!(CrateName::parse("async-trait").unwrap().as_str() == "async-trait");
        for n in ["std", "core", "alloc"] {
            let c = CrateName::parse(n).unwrap();
            assert!(c.is_stdlib(), "{n}");
        }
    }

    #[test]
    fn crate_name_err() {
        assert!(CrateName::parse("").is_err());
        assert!(CrateName::parse("1bad").is_err());
        assert!(CrateName::parse(&"a".repeat(MAX_CRATE_NAME_CHARS + 1)).is_err());
    }

    #[test]
    fn item_path_segments_and_display() {
        let p = ItemPath::parse("clap::Parser").unwrap();
        assert_eq!(p.segments(), &["clap".to_string(), "Parser".to_string()]);
        assert_eq!(p.to_string(), "clap::Parser");
        let slash = ItemPath::parse("runtime/Runtime").unwrap();
        assert_eq!(
            slash.segments(),
            &["runtime".to_string(), "Runtime".to_string()]
        );
        assert_eq!(
            ItemPath::parse("async_trait").unwrap().segments(),
            &["async_trait".to_string()]
        );
    }

    #[test]
    fn item_path_err() {
        assert!(ItemPath::parse("").is_err());
        assert!(ItemPath::parse("::").is_err());
        assert!(ItemPath::parse("has space").is_err());
        assert!(ItemPath::parse("foo.bar").is_err());
        assert!(ItemPath::parse("..").is_err());
        assert!(ItemPath::parse("a/../b").is_err());
    }

    #[test]
    fn search_query_rules() {
        assert!(SearchQuery::parse("", false).is_err());
        assert_eq!(SearchQuery::parse("  ", true).unwrap().as_str(), "");
        assert_eq!(
            SearchQuery::parse("serde", false).unwrap().as_str(),
            "serde"
        );
        assert!(SearchQuery::parse(&"q".repeat(MAX_QUERY_CHARS + 1), false).is_err());
    }

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
}
