//! Validated crates.io / stdlib crate name.

use std::fmt;
use std::str::FromStr;
use std::sync::LazyLock;

use regex::Regex;

use crate::config::MAX_CRATE_NAME_CHARS;
use crate::error::{AppError, AppResult, ErrorKind};

use super::regex::compile_bounded_regex;

/// Canonical stdlib crate names documented on `doc.rust-lang.org` (single source).
pub(crate) const STDLIB_NAMES: &[&str] = &["std", "core", "alloc"];

/// True when `name` is a Rust standard library crate (`std` / `core` / `alloc`).
///
/// Accepts raw strings for HTML scrape / allowlist helpers. Prefer
/// [`CrateName::is_stdlib`] when a validated name is already in hand.
#[must_use]
pub fn is_stdlib_name(name: &str) -> bool {
    STDLIB_NAMES.contains(&name)
}

/// Validated crates.io / stdlib crate name.
#[repr(transparent)]
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
            compile_bounded_regex(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
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
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True for `std` / `core` / `alloc` (docs on doc.rust-lang.org).
    #[must_use]
    pub fn is_stdlib(&self) -> bool {
        is_stdlib_name(self.as_str())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn crate_name_ok_and_stdlib() {
        let s = CrateName::parse("serde").unwrap();
        assert_eq!(s.as_str(), "serde");
        assert!(!s.is_stdlib());
        assert!(CrateName::parse("async-trait").unwrap().as_str() == "async-trait");
        for n in STDLIB_NAMES {
            let c = CrateName::parse(n).unwrap();
            assert!(c.is_stdlib(), "{n}");
            assert!(is_stdlib_name(n));
        }
        assert!(!is_stdlib_name("serde"));
    }

    #[test]
    fn crate_name_err() {
        assert!(CrateName::parse("").is_err());
        assert!(CrateName::parse("1bad").is_err());
        assert!(CrateName::parse(&"a".repeat(MAX_CRATE_NAME_CHARS + 1)).is_err());
    }

    #[test]
    fn crate_name_transparent_layout() {
        assert_eq!(size_of::<CrateName>(), size_of::<String>());
    }
}
