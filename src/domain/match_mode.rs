//! Text match policy for `search-in-crate`.

use crate::error::{AppError, AppResult, ErrorKind};

/// Text match policy for `search-in-crate` (GAP-001 / GAP-019).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchMode {
    /// Leaf name equals query (case-insensitive).
    Exact,
    /// Exact leaf or leaf starts with query — product default.
    #[default]
    Prefix,
    /// Substring match on full path (legacy / opt-in).
    Substring,
}

impl MatchMode {
    /// Stable wire / CLI token.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Prefix => "prefix",
            Self::Substring => "substring",
        }
    }

    /// Parse CLI token (`exact` | `prefix` | `substring`).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for unknown tokens.
    pub fn parse(input: &str) -> AppResult<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "exact" => Ok(Self::Exact),
            "prefix" => Ok(Self::Prefix),
            "substring" | "contains" | "substr" => Ok(Self::Substring),
            other => Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("unknown match mode '{other}' (expected exact|prefix|substring)"),
            )),
        }
    }

    /// Score a hit name against the query (lower is better). `None` = no match.
    ///
    /// `query` accepts any [`AsRef<str>`] (validated [`super::SearchQuery`] or
    /// raw leaf text from HTML hits).
    #[must_use]
    pub fn score(self, name: &str, query: impl AsRef<str>) -> Option<u8> {
        let q = query.as_ref().trim();
        if q.is_empty() {
            return Some(0);
        }
        let q = q.to_ascii_lowercase();
        let name_l = name.to_ascii_lowercase();
        let leaf = name_l.rsplit("::").next().unwrap_or(name_l.as_str());
        let exact_leaf = leaf == q;
        let prefix_leaf = leaf.starts_with(&q);
        let exact_path = name_l == q;
        let contains = name_l.contains(&q) || leaf.contains(&q);
        match self {
            Self::Exact => {
                if exact_leaf {
                    Some(0)
                } else if exact_path {
                    Some(1)
                } else {
                    None
                }
            }
            Self::Prefix => {
                if exact_leaf {
                    Some(0)
                } else if prefix_leaf {
                    Some(1)
                } else if exact_path {
                    Some(2)
                } else {
                    None
                }
            }
            Self::Substring => {
                if exact_leaf {
                    Some(0)
                } else if prefix_leaf {
                    Some(1)
                } else if exact_path {
                    Some(2)
                } else if contains {
                    Some(3)
                } else {
                    None
                }
            }
        }
    }
}

impl std::str::FromStr for MatchMode {
    type Err = AppError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}
