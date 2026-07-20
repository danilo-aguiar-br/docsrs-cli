//! docs.rs / doc.rust-lang.org URL builders and path helpers.
//!
//! Pure URL construction — no HTTP, no HTML parse (one-shot safe).
//! Callers pass domain newtypes so validity is proven by the type (ADR 0006).

use url::Url;

use crate::domain::{AllowedOrigin, CrateName, VersionArg};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::item_kind::{ItemKind, rustc_crate_name};

use super::hits::{leaf_eq_ignore_ascii, unique_best_score_hit};
use super::types::SearchInCrateHit;

fn default_docs_origin(crate_name: &CrateName) -> AllowedOrigin {
    if crate_name.is_stdlib() {
        AllowedOrigin::stdlib_docs_default()
    } else {
        AllowedOrigin::docs_rs_default()
    }
}

/// Build crate index URL with rustc hyphen→underscore segment.
///
/// # Errors
///
/// Propagates [`ErrorKind::Internal`] from [`readme_url_on_origin`] when the URL is invalid.
pub fn readme_url(crate_name: &CrateName, version: &VersionArg) -> AppResult<Url> {
    readme_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build readme URL against an allowlisted origin (production or wiremock).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
pub fn readme_url_on_origin(
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
) -> AppResult<Url> {
    let origin = origin.as_str().trim_end_matches('/');
    let s = if crate_name.is_stdlib() {
        // doc.rust-lang.org/{channel}/{crate}/index.html
        let channel = version.stdlib_channel();
        format!("{origin}/{channel}/{crate_name}/index.html")
    } else {
        let rustc = rustc_crate_name(crate_name.as_str());
        format!("{origin}/{crate_name}/{version}/{rustc}/index.html")
    };
    Url::parse(&s).map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid readme URL", e))
}

/// Build rustdoc item or module URL.
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] or [`ErrorKind::Internal`] from
/// [`get_item_url_on_origin`].
pub fn get_item_url(
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    get_item_url_on_origin(
        &default_docs_origin(crate_name),
        crate_name,
        version,
        kind,
        segments,
    )
}

/// Detect associated method / inherent method path: `Type::method` where Type is
/// UpperCamel and method starts lowercase (or kind forced as method via parse alias).
///
/// `segs` must already have the crate-root prefix stripped (see
/// [`strip_crate_prefix_segments`]).
pub fn is_method_path(kind: ItemKind, segs: &[String]) -> bool {
    if segs.len() < 2 {
        return false;
    }
    if kind != ItemKind::Fn {
        return false;
    }
    let parent = segs[segs.len() - 2].as_str();
    let method = segs[segs.len() - 1].as_str();
    let parent_type = parent
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase());
    let method_fn = method
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c == '_');
    parent_type && method_fn
}

/// Strip leading crate name / rustc root from item path segments (same rules as URL builder).
pub fn strip_crate_prefix_segments(
    crate_name: &CrateName,
    kind: ItemKind,
    segments: &[String],
) -> Vec<String> {
    let rustc_root = rustc_crate_name(crate_name.as_str());
    let mut s = segments.to_vec();
    if let Some(first) = s.first() {
        let f = first.as_str();
        let is_crate_prefix = f == crate_name.as_str() || f == rustc_root.as_str();
        if is_crate_prefix && (s.len() >= 2 || kind == ItemKind::Module) {
            s.remove(0);
        }
    }
    s
}

/// Parent type kinds accepted when resolving a short method path via all.html.
pub const METHOD_PARENT_KIND_NAMES: &[&str] = &["struct", "enum", "trait", "type", "union"];

/// Pick a unique parent type path for `Type::method` short forms (e.g. `Runtime` → `runtime::Runtime`).
///
/// Exact leaf match among struct/enum/trait/type/union only. When several exact
/// leaves exist, prefers a unique best `score` (lower is better).
pub fn pick_unique_type_path(leaf: &str, hits: &[SearchInCrateHit]) -> Option<String> {
    let needle = leaf.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return None;
    }
    let exact: Vec<&SearchInCrateHit> = hits
        .iter()
        .filter(|h| {
            METHOD_PARENT_KIND_NAMES.contains(&h.kind.as_str())
                && leaf_eq_ignore_ascii(&h.name, &needle)
        })
        .collect();
    if exact.len() == 1 {
        return Some(exact[0].name.clone());
    }
    if exact.is_empty() {
        return None;
    }
    // Prefer struct when multiple kinds share the same leaf (rare).
    let structs: Vec<_> = exact.iter().filter(|h| h.kind == "struct").copied().collect();
    if structs.len() == 1 {
        return Some(structs[0].name.clone());
    }
    unique_best_score_hit(exact.into_iter()).map(|h| h.name.clone())
}

/// Build method path segments from a resolved parent type path + method leaf.
pub fn method_segments_from_parent(parent_path: &str, method_leaf: &str) -> Vec<String> {
    let mut segs: Vec<String> = parent_path
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    segs.push(method_leaf.to_string());
    segs
}

/// Parent type kinds tried when resolving a method URL (struct first — common case).
pub(super) const METHOD_PARENT_KINDS: &[ItemKind] = &[
    ItemKind::Struct,
    ItemKind::Enum,
    ItemKind::Trait,
    ItemKind::Type,
    ItemKind::Union,
];

/// Build get-item URL against a custom origin (wiremock tests).
///
/// Associated methods (`Runtime::new`) resolve to the parent type page plus
/// `#method.{name}` (rustdoc layout). Free functions keep `{kind}.{name}.html`.
///
/// # Errors
///
/// Returns [`ErrorKind::InvalidInput`] when a non-module path lacks an item name.
/// Returns [`ErrorKind::Internal`] when the assembled path is not a valid URL.
pub fn get_item_url_on_origin(
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    get_item_url_on_origin_with_parent_kind(origin, crate_name, version, kind, segments, None)
}

/// Like [`get_item_url_on_origin`] but forces the parent type kind for methods.
pub fn get_item_url_on_origin_with_parent_kind(
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
    parent_kind_override: Option<ItemKind>,
) -> AppResult<Url> {
    let origin = origin.as_str().trim_end_matches('/');
    let rustc_root = rustc_crate_name(crate_name.as_str());
    let segs: Vec<String> = {
        let mut s = segments.to_vec();
        if let Some(first) = s.first() {
            let f = first.as_str();
            let is_crate_prefix = f == crate_name.as_str() || f == rustc_root.as_str();
            if is_crate_prefix && (s.len() >= 2 || kind == ItemKind::Module) {
                s.remove(0);
            }
        }
        s
    };

    // Associated method: Type::method → parent page + #method.name
    if is_method_path(kind, &segs) {
        // is_method_path requires ≥2 segments; fail closed if invariant breaks.
        let (parent_name, method_name) = match segs.as_slice() {
            [.., parent, method] => (parent.clone(), method.clone()),
            _ => {
                return Err(AppError::new(
                    ErrorKind::Internal,
                    "method path invariant broken: expected at least 2 segments",
                ));
            }
        };
        let parent_kind = parent_kind_override.unwrap_or(ItemKind::Struct);
        let mod_parts: Vec<String> = if segs.len() == 2 {
            if crate_name.is_stdlib() {
                Vec::new()
            } else {
                vec![rustc_root.clone()]
            }
        } else {
            let mut m = if crate_name.is_stdlib() {
                Vec::new()
            } else {
                vec![rustc_root.clone()]
            };
            for p in &segs[..segs.len() - 2] {
                m.push(rustc_crate_name(p));
            }
            m
        };
        let url_str = if crate_name.is_stdlib() {
            let channel = version.stdlib_channel();
            if mod_parts.is_empty() {
                format!(
                    "{origin}/{channel}/{crate_name}/{}.{}.html#method.{method_name}",
                    parent_kind.file_prefix(),
                    parent_name
                )
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html#method.{method_name}",
                    mod_parts.join("/"),
                    parent_kind.file_prefix(),
                    parent_name
                )
            }
        } else if mod_parts.is_empty() {
            format!(
                "{origin}/{crate_name}/{version}/{}/{}.{}.html#method.{method_name}",
                rustc_root,
                parent_kind.file_prefix(),
                parent_name
            )
        } else {
            format!(
                "{origin}/{crate_name}/{version}/{}/{}.{}.html#method.{method_name}",
                mod_parts.join("/"),
                parent_kind.file_prefix(),
                parent_name
            )
        };
        return Url::parse(&url_str)
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid get-item URL", e));
    }

    let url_str = if crate_name.is_stdlib() {
        let channel = version.stdlib_channel();
        if kind == ItemKind::Module {
            let mut parts: Vec<String> = Vec::new();
            for p in &segs {
                parts.push(rustc_crate_name(p));
            }
            if parts.is_empty() {
                format!("{origin}/{channel}/{crate_name}/index.html")
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/index.html",
                    parts.join("/")
                )
            }
        } else {
            if segs.is_empty() {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "item path missing item name",
                ));
            }
            let item_name = segs.last().ok_or_else(|| {
                AppError::new(ErrorKind::InvalidInput, "item path missing item name")
            })?;
            let mod_parts: Vec<String> = segs[..segs.len().saturating_sub(1)]
                .iter()
                .map(|p| rustc_crate_name(p))
                .collect();
            if mod_parts.is_empty() {
                format!(
                    "{origin}/{channel}/{crate_name}/{}.{}.html",
                    kind.file_prefix(),
                    item_name
                )
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html",
                    mod_parts.join("/"),
                    kind.file_prefix(),
                    item_name
                )
            }
        }
    } else if kind == ItemKind::Module {
        let mut parts: Vec<String> = vec![rustc_root];
        for p in &segs {
            parts.push(rustc_crate_name(p));
        }
        format!(
            "{origin}/{crate_name}/{version}/{}/index.html",
            parts.join("/")
        )
    } else {
        if segs.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "item path missing item name",
            ));
        }
        let item_name = segs
            .last()
            .ok_or_else(|| AppError::new(ErrorKind::InvalidInput, "item path missing item name"))?;
        let mod_parts: Vec<String> = if segs.len() == 1 {
            vec![rustc_root]
        } else {
            let mut m = vec![rustc_root];
            for p in &segs[..segs.len() - 1] {
                m.push(rustc_crate_name(p));
            }
            m
        };
        format!(
            "{origin}/{crate_name}/{version}/{}/{}.{}.html",
            mod_parts.join("/"),
            kind.file_prefix(),
            item_name
        )
    };

    Url::parse(&url_str)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid get-item URL", e))
}

/// Build all.html index URL.
///
/// # Errors
///
/// Propagates [`ErrorKind::Internal`] from [`all_html_url_on_origin`].
pub fn all_html_url(crate_name: &CrateName, version: &VersionArg) -> AppResult<Url> {
    all_html_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build all.html URL against an allowlisted origin (production or wiremock).
///
/// # Errors
///
/// Returns [`ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
pub fn all_html_url_on_origin(
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
) -> AppResult<Url> {
    let origin = origin.as_str().trim_end_matches('/');
    let s = if crate_name.is_stdlib() {
        let channel = version.stdlib_channel();
        format!("{origin}/{channel}/{crate_name}/all.html")
    } else {
        let rustc = rustc_crate_name(crate_name.as_str());
        format!("{origin}/{crate_name}/{version}/{rustc}/all.html")
    };
    Url::parse(&s)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid all.html URL", e))
}
