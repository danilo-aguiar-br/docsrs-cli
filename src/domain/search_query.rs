//! Validated search query string.

use std::fmt;

use crate::config::MAX_QUERY_CHARS;
use crate::error::{AppError, AppResult, ErrorDetail, Subject};

use super::regex::is_hostile_text_char;

/// Validated search query string.
#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SearchQuery(String);

impl SearchQuery {
    /// Parses a search query.
    ///
    /// When `allow_empty` is true, whitespace-only becomes an empty query (list-all).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] when the query is empty without
    /// `allow_empty`, or when it exceeds the maximum length.
    pub fn parse(query: &str, allow_empty: bool) -> AppResult<Self> {
        let q = query.trim();
        if !allow_empty && q.is_empty() {
            return Err(AppError::of(ErrorDetail::Empty {
                subject: Subject::SearchQuery,
            }));
        }
        if q.chars().count() > MAX_QUERY_CHARS {
            return Err(AppError::of(ErrorDetail::TooLong {
                subject: Subject::SearchQuery,
                limit: MAX_QUERY_CHARS,
            }));
        }
        // Hostile argv: reject C0/C1 controls and invisible/bidi format chars.
        // Crates.io query is free text but never needs ZWSP, BOM, or bidi overrides.
        // ASCII domain types (crate/item/version) do not need NFC; free-text search
        // is not an identity key — we fail closed on format chars instead of silent strip.
        if q.chars().any(is_hostile_text_char) {
            return Err(AppError::of(ErrorDetail::ControlCharacters {
                subject: Subject::SearchQuery,
            }));
        }
        Ok(Self(q.to_string()))
    }

    /// Borrow the validated query.
    #[must_use]
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn search_query_rules() {
        assert!(SearchQuery::parse("", false).is_err());
        assert_eq!(SearchQuery::parse("  ", true).unwrap().as_str(), "");
        assert_eq!(
            SearchQuery::parse("serde", false).unwrap().as_str(),
            "serde"
        );
        assert!(SearchQuery::parse(&"q".repeat(MAX_QUERY_CHARS + 1), false).is_err());
        assert!(SearchQuery::parse("evil\0null", false).is_err());
        assert!(SearchQuery::parse("bad\nline", false).is_err());
        // Zero-width / bidi format characters (not `char::is_control`).
        assert!(SearchQuery::parse("evil\u{200B}zwsp", false).is_err());
        assert!(SearchQuery::parse("evil\u{202E}bidi", false).is_err());
        assert!(SearchQuery::parse("evil\u{FEFF}bom", false).is_err());
    }

    #[test]
    fn search_query_transparent_layout() {
        assert_eq!(size_of::<SearchQuery>(), size_of::<String>());
    }
}
