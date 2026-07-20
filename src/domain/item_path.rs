//! Validated rustdoc item path.

use std::fmt;
use std::str::FromStr;

use crate::config::MAX_ITEM_PATH_CHARS;
use crate::error::{AppError, AppResult, ErrorKind};

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
        let mut segments = Vec::with_capacity(parts.len());
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
            // Accept hyphen (crates.io style) and normalize to underscore (rustc paths).
            if !seg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "invalid item path segment '{seg}' (use letters, digits, underscore or hyphen; hyphens normalize to underscore; separate with :: or /)"
                    ),
                ));
            }
            segments.push(crate::item_kind::rustc_crate_name(seg));
        }
        Ok(Self {
            raw: path.to_string(),
            segments,
        })
    }

    /// Path segments after normalization.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Owned segment list (for APIs that still take `Vec<String>`).
    #[must_use]
    pub fn into_segments(self) -> Vec<String> {
        self.segments
    }

    /// Original trimmed input string.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MAX_ITEM_PATH_CHARS;

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
        // Hyphen normalizes to underscore (rustc path).
        assert_eq!(
            ItemPath::parse("async-trait").unwrap().segments(),
            &["async_trait".to_string()]
        );
        assert_eq!(
            ItemPath::parse("foo-bar::Baz-Quux").unwrap().segments(),
            &["foo_bar".to_string(), "Baz_Quux".to_string()]
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
        assert!(ItemPath::parse(&format!("a::{}", "b".repeat(MAX_ITEM_PATH_CHARS))).is_err());
    }
}
