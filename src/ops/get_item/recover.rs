//! Not-found recovery ladder for `get-item`.
//!
//! Fetching an item and recovering from a 404 are different jobs. The fetch is one
//! GET; recovery is a catalog round trip, a parent-page probe, a ranking pass and
//! two distinct resolution shapes. Splitting them keeps each file about one thing —
//! and recovery is the half that grows every time rustdoc gains an anchor family.
//!
//! Two arms, chosen by path shape:
//!
//! - [`recover_assoc_path`] — `Parent::member` (methods, associated types/constants,
//!   enum variants, struct fields): resolve the parent, then retry the anchor
//! - [`recover_reexport_path`] — free items: resolve the canonical path, then retry
//!
//! Both fall back to ranked suggestions instead of a bare not-found.

use url::Url;

use super::{CATALOG_PROBE_LIMIT, ItemRequest, ItemTarget};
use crate::docs_rs::{self, GetItemData, SearchInCrateHit};
use crate::domain::{MatchMode, SearchQuery};
use crate::error::{AppError, AppResult, ErrorDetail, ErrorKind, Suggestion};
use crate::http::HttpClient;
use crate::item_kind::ItemKind;
use crate::render::apply_truncation_to_item;
use crate::suggest;

/// Widen an empty catalog before ranking suggestions.
///
/// The recovery probe queries `all.html` with the **typed** leaf under
/// [`MatchMode::Prefix`]. A typo such as `Runtimee` prefix-matches nothing, so
/// the catalog collapses to zero rows and [`suggest::rank_suggestions`] — including
/// its edit-distance arm — never receives a candidate to score. That is why every
/// distance-1 typo answered `(none)` while the ranker itself was correct.
///
/// Re-reading the same `all.html` with an empty query restores the full index.
/// The URL was fetched moments ago in this same process, so this resolves against
/// the disk cache instead of opening a second connection.
async fn widen_catalog_if_empty(
    http: &HttpClient,
    target: &ItemTarget<'_>,
    item_type: Option<ItemKind>,
    all_url: &Url,
    hits: Vec<SearchInCrateHit>,
) -> Vec<SearchInCrateHit> {
    if !hits.is_empty() {
        return hits;
    }
    let Ok(list_all) = SearchQuery::parse("", true) else {
        return hits;
    };
    match docs_rs::search_in_crate_at(
        http,
        target.crate_name,
        target.version,
        &list_all,
        item_type,
        CATALOG_PROBE_LIMIT,
        MatchMode::Prefix,
        all_url,
    )
    .await
    {
        Ok(full) => full.hits,
        // A widening failure must never mask the original not-found.
        Err(_) => hits,
    }
}

/// Catalog-backed resolution after a 404: free-item reexports OR short
/// `Parent::member` paths (methods, associated types, associated constants).
pub(super) async fn recover_not_found(
    req: &ItemRequest<'_>,
    segs: &[String],
    base: AppError,
) -> AppResult<GetItemData> {
    let t = &req.target;
    let segs_detect = docs_rs::strip_crate_prefix_segments(t.crate_name, t.kind, segs);
    let all_url = docs_rs::all_html_url_on_origin(t.origin, t.crate_name, t.version)?;
    if let Some(assoc) = docs_rs::associated_item_path(t.kind, &segs_detect) {
        recover_assoc_path(req, assoc, segs, &segs_detect, &all_url, base).await
    } else {
        recover_reexport_path(req, segs, &all_url, base).await
    }
}

/// R1: associated member short path (`Runtime::new` → `runtime::Runtime::new`).
///
/// Works for every anchor family, so `Iterator::Item` resolves its parent trait
/// through `all.html` exactly like `Runtime::new` resolves its parent struct.
async fn recover_assoc_path(
    req: &ItemRequest<'_>,
    assoc: docs_rs::AssocAnchorKind,
    segs: &[String],
    segs_detect: &[String],
    all_url: &Url,
    base: AppError,
) -> AppResult<GetItemData> {
    let t = &req.target;
    let parent_leaf = segs_detect[segs_detect.len() - 2].as_str();
    let method_leaf = segs_detect[segs_detect.len() - 1].as_str();
    let method_anchor_miss = base
        .message()
        .starts_with(docs_rs::ASSOC_ANCHOR_MISS_PREFIX);

    // GAP-SUGGEST-003: an anchor miss means the parent page was FOUND and only
    // the member was absent. Re-resolving the parent through the catalog then
    // has nothing to discover, and it actively harms: when the parent's leaf is
    // ambiguous (`Range` exists in ops, collections::btree_map, …),
    // `pick_unique_type_path` gives up and the fallback suggests those
    // same-named types instead of the members the user asked about. Measured on
    // `std ops::Range::nxt`, which offered five structs and zero methods.
    // Going straight to the parent's anchors also skips an all.html download.
    if method_anchor_miss {
        let parent_path = segs_detect[..segs_detect.len() - 1].join("::");
        let names =
            member_name_suggestions(req.http, t, assoc, segs, method_leaf, &parent_path).await;
        if !names.is_empty() {
            return Err(with_suggestions(base, &names, req.suggest));
        }
        // Empty member list: fall through to the catalog, which can still spot a
        // reexport whose short path reaches a different parent.
    }

    let parent_q = SearchQuery::parse(parent_leaf, false)?;
    let catalog = docs_rs::search_in_crate_at(
        req.http,
        t.crate_name,
        t.version,
        &parent_q,
        None,
        CATALOG_PROBE_LIMIT,
        MatchMode::Prefix,
        all_url,
    )
    .await;
    let cat = match catalog {
        Ok(cat) => cat,
        Err(_) if method_anchor_miss => {
            // Catalog unavailable but parent page existed: still try member-leaf suggest.
            let parent_path = segs_detect[..segs_detect.len() - 1].join("::");
            let names =
                member_name_suggestions(req.http, t, assoc, segs, method_leaf, &parent_path).await;
            return Err(with_suggestions(base, &names, req.suggest));
        }
        Err(_) => return Err(base),
    };
    let Some(parent_path) = docs_rs::pick_unique_type_path(parent_leaf, &cat.hits) else {
        let hits = widen_catalog_if_empty(req.http, t, None, all_url, cat.hits).await;
        let names = suggest::rank_suggestions(parent_leaf, &hits, 5);
        return Err(with_suggestions(base, &names, req.suggest));
    };
    let resolved_segs = docs_rs::method_segments_from_parent(&parent_path, method_leaf);
    let resolved_label = format!("{parent_path}::{method_leaf}");
    match docs_rs::fetch_item_on_origin_with_echo(
        req.http,
        t.origin,
        t.crate_name,
        t.version,
        t.kind,
        t.kind_echo,
        &resolved_segs,
    )
    .await
    {
        Ok(mut resolved_data) => {
            resolved_data.item_path = t.item_path.as_str().to_string();
            resolved_data.item_name = method_leaf.to_string();
            resolved_data.resolved_item_path = Some(resolved_label);
            Ok(apply_truncation_to_item(resolved_data, req.cfg))
        }
        // Parent path ok but member leaf missing → suggest members (X-002).
        Err(re)
            if re.kind() == ErrorKind::NotFound
                && (method_anchor_miss
                    || re.message().starts_with(docs_rs::ASSOC_ANCHOR_MISS_PREFIX)) =>
        {
            let names = member_name_suggestions(
                req.http,
                t,
                assoc,
                &resolved_segs,
                method_leaf,
                &parent_path,
            )
            .await;
            Err(with_suggestions(re, &names, req.suggest))
        }
        Err(re) => Err(re),
    }
}

/// R2: free item / reexport resolution via `all.html`.
async fn recover_reexport_path(
    req: &ItemRequest<'_>,
    segs: &[String],
    all_url: &Url,
    base: AppError,
) -> AppResult<GetItemData> {
    let t = &req.target;
    let leaf = segs
        .last()
        .map(|s| s.as_str())
        .unwrap_or(t.item_path.as_str());
    let leaf_q = SearchQuery::parse(leaf, false)?;
    let catalog = docs_rs::search_in_crate_at(
        req.http,
        t.crate_name,
        t.version,
        &leaf_q,
        Some(t.kind),
        CATALOG_PROBE_LIMIT,
        MatchMode::Prefix,
        all_url,
    )
    .await;
    let Ok(cat) = catalog else {
        return Err(base);
    };
    let Some(resolved) = suggest::pick_unique_reexport_path(leaf, t.kind, &cat.hits) else {
        let hits = widen_catalog_if_empty(req.http, t, Some(t.kind), all_url, cat.hits).await;
        let names = suggest::rank_suggestions(leaf, &hits, 5);
        return Err(with_suggestions(base, &names, req.suggest));
    };
    let resolved_segs: Vec<String> = resolved
        .split("::")
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let mut resolved_data = docs_rs::fetch_item_on_origin_with_echo(
        req.http,
        t.origin,
        t.crate_name,
        t.version,
        t.kind,
        t.kind_echo,
        &resolved_segs,
    )
    .await?;
    resolved_data.item_path = t.item_path.as_str().to_string();
    resolved_data.item_name = leaf.to_string();
    resolved_data.resolved_item_path = Some(resolved);
    Ok(apply_truncation_to_item(resolved_data, req.cfg))
}

/// Attach ranked suggestions to a not-found error.
///
/// Returns `base` unchanged when the caller did not pass `--suggest` and nothing
/// was found, so a plain miss keeps its original message.
fn with_suggestions(base: AppError, names: &[Suggestion], suggest: bool) -> AppError {
    if !suggest && names.is_empty() {
        return base;
    }
    // Wrap the typed base rather than flattening it to a string, so the
    // suggestion line renders in both languages alongside the original failure —
    // and so `error.suggestions` can publish the same list as data.
    AppError::of(ErrorDetail::WithSuggestions {
        base: Box::new(base.detail().clone()),
        suggestions: names.to_vec(),
    })
}

/// Fetch parent page HTML and rank nearby member names of one anchor family.
///
/// One extra GET on the member-miss path only (one-shot recovery). Cancel is
/// honored via [`crate::http::HttpClient`]. Failures yield an empty suggestion list.
///
/// Suggestions read the **parent page already identified**, never `all.html`.
/// That distinction is what makes them possible at all: the crate index lists
/// `Iterator` but none of `next`, `map`, or `Item`.
///
/// Important: parent pages are fetched as **type** kinds (`struct`/`enum`/…),
/// never as `method`/`fn` paths — otherwise `associated_item_path` misreads
/// `runtime::Runtime` as a free-fn URL and suggestions stay empty (X-002).
async fn member_name_suggestions(
    http: &HttpClient,
    target: &ItemTarget<'_>,
    assoc: docs_rs::AssocAnchorKind,
    method_segs: &[String],
    method_leaf: &str,
    parent_path: &str,
) -> Vec<Suggestion> {
    let parent_segs: Vec<String> = if method_segs.len() >= 2 {
        method_segs[..method_segs.len() - 1].to_vec()
    } else {
        return Vec::new();
    };
    // Same order as the live probe — shared table, never a second copy: two
    // lists that must agree but can drift is exactly how a kind goes missing here
    // while staying reachable there.
    for parent_kind in assoc.parent_kind_probe() {
        let Ok(url) = docs_rs::get_item_url_on_origin(
            target.origin,
            target.crate_name,
            target.version,
            *parent_kind,
            &parent_segs,
        ) else {
            continue;
        };
        let mut fetch_url = url;
        fetch_url.set_fragment(None);
        let Ok(resp) = http.get_html(&fetch_url).await else {
            continue;
        };
        if resp.status.as_u16() == 404 {
            continue;
        }
        if !resp.status.is_success() {
            continue;
        }
        let Ok(text) = crate::http::decode_utf8(&resp.body) else {
            continue;
        };
        let names = docs_rs::list_assoc_anchor_names_from_html(&text, assoc);
        if names.is_empty() {
            continue;
        }
        // Label with the kind the caller typed, so the suggestion is copy-paste
        // ready instead of pointing back at the same not-found.
        return suggest::rank_member_names(method_leaf, parent_path, target.kind_echo, &names, 5);
    }
    Vec::new()
}
