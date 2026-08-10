//! docs.rs / doc.rust-lang.org URL builders and path helpers.
//!
//! Pure URL construction — no HTTP, no HTML parse (one-shot safe).
//! Callers pass domain newtypes so validity is proven by the type (ADR 0006).

use url::Url;

use crate::domain::{AllowedOrigin, CrateName, VersionArg};
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};
use crate::item_kind::{ItemKind, rustc_crate_name};

use super::assoc::{AssocAnchorKind, associated_item_path, member_only_family};
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
/// Propagates [`crate::error::ErrorKind::Internal`] from [`readme_url_on_origin`] when the URL is invalid.
pub fn readme_url(crate_name: &CrateName, version: &VersionArg) -> AppResult<Url> {
    readme_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build readme URL against an allowlisted origin (production or wiremock).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
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
    Url::parse(&s).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::UrlBuild,
            },
            e,
        )
    })
}

/// Build rustdoc item or module URL.
///
/// # Errors
///
/// Propagates [`crate::error::ErrorKind::InvalidInput`] or [`crate::error::ErrorKind::Internal`] from
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
///
/// Narrow view of [`associated_item_path`], kept because callers that only care
/// about functions read better without matching on the family enum.
pub fn is_method_path(kind: ItemKind, segs: &[String]) -> bool {
    associated_item_path(kind, segs) == Some(AssocAnchorKind::Method)
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
            // Same list the live fetch probes, read from the family instead of a
            // local copy: a parent kind reachable there must be reachable here.
            AssocAnchorKind::Method
                .parent_kind_probe()
                .iter()
                .any(|k| k.as_str() == h.kind.as_str())
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
    let structs: Vec<_> = exact
        .iter()
        .filter(|h| h.kind == "struct")
        .copied()
        .collect();
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

/// Build get-item URL against a custom origin (wiremock tests).
///
/// Associated items (`Runtime::new`, `Iterator::Item`, `Duration::MAX`) resolve
/// to the parent page plus the family's anchor fragment (rustdoc layout). Free
/// items keep `{kind}.{name}.html`.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::InvalidInput`] when a non-module path lacks an item name.
/// Returns [`crate::error::ErrorKind::Internal`] when the assembled path is not a valid URL.
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
///
/// # Errors
/// Same as [`get_item_url_on_origin`].
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
    let segs = strip_crate_prefix_segments(crate_name, kind, segments);

    // Associated item: Parent::member → parent page + #{prefix}{member}
    if let Some(assoc) = associated_item_path(kind, &segs) {
        // associated_item_path requires ≥2 segments; fail closed if that breaks.
        let (parent_name, member_name) = match segs.as_slice() {
            [.., parent, member] => (parent.clone(), member.clone()),
            _ => {
                return Err(AppError::of(ErrorDetail::Internal {
                    op: InternalOp::AssocPathTooShort,
                }));
            }
        };
        // Only one fragment fits in a URL. The extractor still probes every
        // prefix of the family against the page it fetches, so a required trait
        // method planned as `#method.X` still resolves through `#tymethod.X`.
        let anchor = format!("{}{member_name}", assoc.primary_prefix());
        // Probe order leads with the kind that hosts this family most often:
        // traits for associated types, structs for methods and constants.
        let parent_kind = parent_kind_override.unwrap_or(assoc.parent_kind_probe()[0]);
        // Every kind in a `parent_kind_probe` list owns a page by construction;
        // a `None` here would mean the probe tables named a member as a host.
        let parent_prefix = parent_kind.file_prefix().ok_or_else(|| {
            AppError::of(ErrorDetail::Internal {
                op: InternalOp::AssocParentOwnsNoPage,
            })
        })?;
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
                    "{origin}/{channel}/{crate_name}/{parent_prefix}.{parent_name}.html#{anchor}"
                )
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html#{anchor}",
                    mod_parts.join("/"),
                    parent_prefix,
                    parent_name
                )
            }
        } else if mod_parts.is_empty() {
            format!(
                "{origin}/{crate_name}/{version}/{rustc_root}/{parent_prefix}.{parent_name}.html#{anchor}"
            )
        } else {
            format!(
                "{origin}/{crate_name}/{version}/{}/{}.{}.html#{anchor}",
                mod_parts.join("/"),
                parent_prefix,
                parent_name
            )
        };
        return Url::parse(&url_str).map_err(|e| {
            AppError::of_with_source(
                ErrorDetail::Internal {
                    op: InternalOp::UrlBuild,
                },
                e,
            )
        });
    }

    // Fail closed before the free-item branch. An enum variant and a struct
    // field exist only as anchors on their parent, so reaching here means the
    // path had no `Parent::member` shape. Building `variant.Some.html` would be
    // an HTTP 404 dressed up as a plan; naming the missing parent is actionable.
    if let Some(family) = member_only_family(kind) {
        return Err(AppError::of(ErrorDetail::MemberKindNeedsParent {
            kind: kind.as_str(),
            member: segs.last().map_or("member", String::as_str).to_string(),
            parent_kinds: family
                .parent_kind_probe_names()
                .collect::<Vec<_>>()
                .join(", "),
        }));
    }

    // Safe after the guard above: every remaining kind owns a page.
    let file_prefix = kind.file_prefix().unwrap_or(kind.as_str());

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
                return Err(AppError::of(ErrorDetail::ItemPathMissingItemName));
            }
            let item_name = segs
                .last()
                .ok_or_else(|| AppError::of(ErrorDetail::ItemPathMissingItemName))?;
            let mod_parts: Vec<String> = segs[..segs.len().saturating_sub(1)]
                .iter()
                .map(|p| rustc_crate_name(p))
                .collect();
            if mod_parts.is_empty() {
                format!("{origin}/{channel}/{crate_name}/{file_prefix}.{item_name}.html")
            } else {
                format!(
                    "{origin}/{channel}/{crate_name}/{}/{}.{}.html",
                    mod_parts.join("/"),
                    file_prefix,
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
            return Err(AppError::of(ErrorDetail::ItemPathMissingItemName));
        }
        let item_name = segs
            .last()
            .ok_or_else(|| AppError::of(ErrorDetail::ItemPathMissingItemName))?;
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
            file_prefix,
            item_name
        )
    };

    Url::parse(&url_str).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::UrlBuild,
            },
            e,
        )
    })
}

/// Build all.html index URL.
///
/// # Errors
///
/// Propagates [`crate::error::ErrorKind::Internal`] from [`all_html_url_on_origin`].
pub fn all_html_url(crate_name: &CrateName, version: &VersionArg) -> AppResult<Url> {
    all_html_url_on_origin(&default_docs_origin(crate_name), crate_name, version)
}

/// Build all.html URL against an allowlisted origin (production or wiremock).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] when `origin` / path segments do not form a valid URL.
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
    Url::parse(&s).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::UrlBuild,
            },
            e,
        )
    })
}
