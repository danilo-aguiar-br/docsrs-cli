//! ItemKind normalization for rustdoc URLs and all.html filters.

use crate::error::{AppError, AppResult, ErrorKind};

/// Canonical rustdoc item kinds used by docs.rs HTML.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Module page.
    Module,
    /// Struct page.
    Struct,
    /// Trait page.
    Trait,
    /// Enum page.
    Enum,
    /// Union page.
    Union,
    /// Function or method page.
    Fn,
    /// Type alias page.
    Type,
    /// Constant page.
    Constant,
    /// Static page.
    Static,
    /// Macro page.
    Macro,
    /// Attribute macro page.
    Attribute,
    /// Derive macro page.
    Derive,
}

impl ItemKind {
    /// Stable JSON / filter string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Struct => "struct",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::Union => "union",
            Self::Fn => "fn",
            Self::Type => "type",
            Self::Constant => "constant",
            Self::Static => "static",
            Self::Macro => "macro",
            Self::Attribute => "attribute",
            Self::Derive => "derive",
        }
    }

    /// Filename prefix in rustdoc HTML (e.g. `struct`, `fn`, `attr`, `constant`).
    ///
    /// Modern rustdoc/docs.rs uses `constant.NAME.html` (not legacy `const.NAME.html`).
    pub fn file_prefix(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Struct => "struct",
            Self::Trait => "trait",
            Self::Enum => "enum",
            Self::Union => "union",
            Self::Fn => "fn",
            Self::Type => "type",
            Self::Constant => "constant",
            Self::Static => "static",
            Self::Macro => "macro",
            Self::Attribute => "attr",
            Self::Derive => "derive",
        }
    }

    /// Parse CLI / filter aliases into canonical kind.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `input` is not a known kind alias.
    pub fn parse(input: &str) -> AppResult<Self> {
        Ok(Self::parse_with_echo(input)?.0)
    }

    /// Parse kind for URL building plus the wire echo string for `item_type`.
    ///
    /// `method` maps to [`ItemKind::Fn`] for rustdoc URLs but echoes as `"method"`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when `input` is not a known kind alias.
    pub fn parse_with_echo(input: &str) -> AppResult<(Self, &'static str)> {
        let s = input.trim().to_ascii_lowercase();
        match s.as_str() {
            "module" | "mod" => Ok((Self::Module, "module")),
            "struct" => Ok((Self::Struct, "struct")),
            "trait" => Ok((Self::Trait, "trait")),
            "enum" => Ok((Self::Enum, "enum")),
            "union" => Ok((Self::Union, "union")),
            "fn" | "function" => Ok((Self::Fn, "fn")),
            "method" => Ok((Self::Fn, "method")),
            "type" => Ok((Self::Type, "type")),
            "const" | "constant" => Ok((Self::Constant, "constant")),
            "static" => Ok((Self::Static, "static")),
            "macro" => Ok((Self::Macro, "macro")),
            "attr" | "attribute" => Ok((Self::Attribute, "attribute")),
            "derive" => Ok((Self::Derive, "derive")),
            other => Err(AppError::new(
                ErrorKind::InvalidInput,
                format!("unknown item type '{other}'"),
            )),
        }
    }

    /// Infer kind from all.html href (e.g. `struct.Name.html`, `attr.foo.html`).
    ///
    /// Accepts modern `constant.` and legacy `const.` prefixes for Constant.
    pub fn from_href(href: &str) -> Option<Self> {
        let file = href.rsplit('/').next().unwrap_or(href);
        let file = file.split('#').next().unwrap_or(file);
        let prefix = file.split('.').next().unwrap_or("");
        match prefix {
            "struct" => Some(Self::Struct),
            "trait" => Some(Self::Trait),
            "fn" => Some(Self::Fn),
            "enum" => Some(Self::Enum),
            "type" => Some(Self::Type),
            // Modern rustdoc: constant.NAME.html; legacy: const.NAME.html
            "constant" | "const" => Some(Self::Constant),
            "static" => Some(Self::Static),
            "macro" => Some(Self::Macro),
            "union" => Some(Self::Union),
            "attr" => Some(Self::Attribute),
            "derive" => Some(Self::Derive),
            // Module pages use index.html (no typed prefix).
            "index" => Some(Self::Module),
            _ => {
                // Associated method anchors: `struct.Foo.html#method.bar`
                if href.contains("#method.") {
                    Some(Self::Fn)
                } else {
                    None
                }
            }
        }
    }
}

/// Replace hyphens with underscores for rustc crate/module path segments.
pub fn rustc_crate_name(pkg: &str) -> String {
    pkg.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hyphen_to_underscore() {
        assert_eq!(rustc_crate_name("async-trait"), "async_trait");
        assert_eq!(rustc_crate_name("serde"), "serde");
    }

    #[test]
    fn fn_aliases() {
        assert_eq!(ItemKind::parse("fn").unwrap(), ItemKind::Fn);
        assert_eq!(ItemKind::parse("function").unwrap(), ItemKind::Fn);
        assert_eq!(ItemKind::parse("method").unwrap(), ItemKind::Fn);
        let (k, echo) = ItemKind::parse_with_echo("method").unwrap();
        assert_eq!(k, ItemKind::Fn);
        assert_eq!(echo, "method");
        let (_, echo) = ItemKind::parse_with_echo("fn").unwrap();
        assert_eq!(echo, "fn");
    }

    #[test]
    fn href_classifier() {
        assert_eq!(ItemKind::from_href("fn.foo.html"), Some(ItemKind::Fn));
        assert_eq!(ItemKind::from_href("union.U.html"), Some(ItemKind::Union));
        assert_eq!(
            ItemKind::from_href("attr.async_trait.html"),
            Some(ItemKind::Attribute)
        );
        assert_eq!(
            ItemKind::from_href("derive.Serialize.html"),
            Some(ItemKind::Derive)
        );
        assert_eq!(
            ItemKind::from_href("de/trait.Deserialize.html"),
            Some(ItemKind::Trait)
        );
        assert_eq!(ItemKind::from_href("unknown.html"), None);
    }

    #[test]
    fn constant_prefix_modern_and_legacy() {
        assert_eq!(ItemKind::file_prefix(ItemKind::Constant), "constant");
        assert_eq!(
            ItemKind::from_href("constant.MAX.html"),
            Some(ItemKind::Constant)
        );
        assert_eq!(
            ItemKind::from_href("constant._SC_OPEN_MAX.html"),
            Some(ItemKind::Constant)
        );
        // Legacy rustdoc href still classifies as Constant.
        assert_eq!(
            ItemKind::from_href("const.MAX.html"),
            Some(ItemKind::Constant)
        );
        assert_eq!(ItemKind::parse("const").unwrap(), ItemKind::Constant);
        assert_eq!(ItemKind::parse("constant").unwrap(), ItemKind::Constant);
        assert_eq!(ItemKind::Constant.as_str(), "constant");
    }
}
