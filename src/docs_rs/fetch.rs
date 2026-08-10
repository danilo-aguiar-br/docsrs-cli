//! Async HTTP fetch for readme / get-item (I/O on Tokio; parse on spawn_blocking).

use url::Url;

use crate::domain::{AllowedOrigin, CrateName, VersionArg};
use crate::error::{AppError, AppResult, ErrorDetail, ErrorKind};
use crate::http::{HttpClient, decode_utf8, require_content_type_html};
use crate::item_kind::ItemKind;

use scraper::Html;

use super::assoc::{ASSOC_ANCHOR_MISS_PREFIX, assoc_from_fragment, associated_item_path};
use super::html::{
    extract_assoc_markdown_scoped_from_document, extract_item_markdown_from_document,
    extract_readme_markdown_from_document, extract_resolved_version_for_crate,
    scrape_docs_rs_version_from_document,
};
use super::types::{GetItemData, ReadmeData};
use super::urls::{
    get_item_url_on_origin, get_item_url_on_origin_with_parent_kind, readme_url_on_origin,
    strip_crate_prefix_segments,
};

/// Fetch and convert crate overview docblock (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and HTML conversion failures from
/// [`fetch_readme_on_origin`].
pub async fn fetch_readme(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
) -> AppResult<ReadmeData> {
    fetch_readme_on_origin(http, &AllowedOrigin::docs_rs_default(), crate_name, version).await
}

/// Fetch readme against a configurable origin (offline mocks).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] when the origin URL is invalid.
/// Propagates fetch failures from [`fetch_readme_at`].
pub async fn fetch_readme_on_origin(
    http: &HttpClient,
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
) -> AppResult<ReadmeData> {
    let url = readme_url_on_origin(origin, crate_name, version)?;
    fetch_readme_at(http, crate_name, version, &url).await
}

/// Fetch readme from a prebuilt URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`crate::error::ErrorKind::Parse`] on non-UTF-8 bodies or HTML→Markdown failure.
pub async fn fetch_readme_at(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    url: &Url,
) -> AppResult<ReadmeData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("crate or version not found: {crate_name}@{version}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "rustdoc readme",
        ));
    }
    // Fail closed when origin advertises a non-HTML type (MIME confusion).
    require_content_type_html(resp.content_type.as_deref())?;

    // One decode + one DOM parse for extract + version scrape (SCRAPE-Q-004/009, S-003).
    let body = resp.body;
    let crate_for_scrape = crate_name.as_str().to_string();
    let (markdown, empty, scraped_ver) = http
        .budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            let doc = Html::parse_document(&text);
            let (md, empty) = extract_readme_markdown_from_document(&doc)?;
            let scraped = scrape_docs_rs_version_from_document(&doc, &crate_for_scrape);
            Ok((md, empty, scraped))
        })
        .await?;
    let resolved = extract_resolved_version_for_crate(
        &resp.final_url,
        version.as_str(),
        Some(crate_name.as_str()),
    )
    .or(scraped_ver);
    Ok(ReadmeData {
        crate_name: crate_name.as_str().to_string(),
        version: version.as_str().to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
        cache_hit: resp.cache_hit,
    })
}

/// Fetch and convert a typed rustdoc item page (production docs.rs).
///
/// # Errors
///
/// Propagates URL, HTTP, decode, and HTML conversion failures from
/// [`fetch_item_on_origin`].
pub async fn fetch_item(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    fetch_item_on_origin(
        http,
        &AllowedOrigin::docs_rs_default(),
        crate_name,
        version,
        kind,
        segments,
    )
    .await
}

/// Fetch item against a configurable origin (offline mocks).
///
/// For associated methods, tries parent type pages (struct → enum → trait → …)
/// until one returns HTTP 200.
///
/// # Errors
///
/// Propagates URL build errors from [`get_item_url_on_origin`] and fetch errors
/// from [`fetch_item_at`].
pub async fn fetch_item_on_origin(
    http: &HttpClient,
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    fetch_item_on_origin_with_echo(
        http,
        origin,
        crate_name,
        version,
        kind,
        kind.as_str(),
        segments,
    )
    .await
}

/// Like [`fetch_item_on_origin`] with a wire `item_type` echo (`method` vs `fn`).
///
/// # Errors
/// Same as [`fetch_item_on_origin`].
pub async fn fetch_item_on_origin_with_echo(
    http: &HttpClient,
    origin: &AllowedOrigin,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    item_type_echo: &str,
    segments: &[String],
) -> AppResult<GetItemData> {
    // Shared with the URL builder on purpose. This block used to be a hand copy
    // carrying the comment "the same way URL builder does" — a promise the
    // compiler could not keep. Detection and URL construction reading different
    // rules is the exact shape of GAP-ASSOCITEM-001.
    let segs_for_detect = strip_crate_prefix_segments(crate_name, kind, segments);

    if let Some(assoc) = associated_item_path(kind, &segs_for_detect) {
        // Keep the *first* parent-page NotFound for diagnostics (X-010): dry-run
        // plans the family's leading kind first; reporting the last probe
        // (`union.*`) misleads agents.
        //
        // Sequential on purpose, reconfirmed by measurement in 2026-08 after the
        // decision was reopened (GAP-PROBE-001).
        //
        // The original rejection cited a 12.6x regression without naming the
        // mechanism, which is why it kept getting reopened. The mechanism is the
        // politeness delay: `rate_limit_delay_ms` defaults to 1000 and is keyed
        // by host, while every kind in `parent_kind_probe` — up to five — resolves
        // against that same host. Measured on this host: `Duration::MAX`, which
        // hits on its first probe, completes in 104 ms; `Iterator::Item`, which
        // needs a second request, takes 1083 ms. The extra second is the throttle,
        // not the network.
        //
        // So the floor for N sequential probes is (N-1) x delay whether they are
        // issued serially or concurrently: a fan-out would queue behind the same
        // limiter and add coordination on top. Concurrency can only pay off once
        // the probes stop sharing a rate-limited host, which is not a property of
        // rustdoc URLs.
        let mut first_err: Option<AppError> = None;
        for parent_kind in assoc.parent_kind_probe() {
            let url = get_item_url_on_origin_with_parent_kind(
                origin,
                crate_name,
                version,
                kind,
                segments,
                Some(*parent_kind),
            )?;
            match fetch_item_at_with_echo(
                http,
                crate_name,
                version,
                kind,
                item_type_echo,
                segments,
                &url,
            )
            .await
            {
                Ok(data) => return Ok(data),
                // Parent page found but member anchor missing → stop probing kinds.
                Err(e)
                    if e.kind() == ErrorKind::NotFound
                        && e.message().starts_with(ASSOC_ANCHOR_MISS_PREFIX) =>
                {
                    return Err(e);
                }
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                    continue;
                }
                Err(e) => return Err(e),
            }
        }
        return Err(first_err.unwrap_or_else(|| AppError::of(ErrorDetail::AssocParentPageNotFound)));
    }

    let url = get_item_url_on_origin(origin, crate_name, version, kind, segments)?;
    fetch_item_at_with_echo(
        http,
        crate_name,
        version,
        kind,
        item_type_echo,
        segments,
        &url,
    )
    .await
}

/// Fetch item from a prebuilt URL (wiremock).
///
/// # Errors
///
/// Propagates HTTP errors from [`HttpClient::get_html`].
/// Maps non-success statuses via [`AppError::from_http_status`].
/// Returns [`crate::error::ErrorKind::Parse`] on non-UTF-8 bodies or HTML→Markdown failure.
pub async fn fetch_item_at(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    segments: &[String],
    url: &Url,
) -> AppResult<GetItemData> {
    fetch_item_at_with_echo(
        http,
        crate_name,
        version,
        kind,
        kind.as_str(),
        segments,
        url,
    )
    .await
}

/// Like [`fetch_item_at`] but echoes a requested kind string (e.g. `method` vs `fn`).
///
/// # Errors
/// Same as [`fetch_item_at`].
pub async fn fetch_item_at_with_echo(
    http: &HttpClient,
    crate_name: &CrateName,
    version: &VersionArg,
    kind: ItemKind,
    item_type_echo: &str,
    segments: &[String],
    url: &Url,
) -> AppResult<GetItemData> {
    let _ = kind; // URL kind already baked into `url`; echo drives item_type/title.
    // HTTP does not send fragments; strip for request, keep for source_url.
    let mut fetch_url = url.clone();
    let fragment = fetch_url.fragment().map(str::to_string);
    fetch_url.set_fragment(None);

    let resp = http.get_html(&fetch_url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("item not found at {url}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "rustdoc get-item",
        ));
    }
    require_content_type_html(resp.content_type.as_deref())?;

    // A fragment names its own anchor family without ambiguity, so the family is
    // read back off the URL already in hand instead of threading an extra
    // parameter through every layer. The extractor still decides which prefix of
    // that family the page actually carries (`method.` vs `tymethod.`).
    let assoc_anchor = fragment
        .as_deref()
        .and_then(assoc_from_fragment)
        .map(|(family, name)| (family, name.to_string()));
    // Move body once: one DOM for markdown + version scrape (SCRAPE-S-003).
    let body = resp.body;
    let assoc_anchor_cpu = assoc_anchor;
    let crate_for_scrape = crate_name.as_str().to_string();
    let ((markdown, empty), extraction, matched_anchor, scraped_ver) = http
        .budget()
        .run_cpu_bound(move || {
            let text = decode_utf8(&body)?;
            let doc = Html::parse_document(&text);
            let scraped = scrape_docs_rs_version_from_document(&doc, &crate_for_scrape);
            if let Some((family, ref m)) = assoc_anchor_cpu {
                let found =
                    extract_assoc_markdown_scoped_from_document(&doc, family.anchor_prefixes(), m)?;
                Ok((
                    (found.markdown, found.empty),
                    Some(found.scope.to_string()),
                    Some(found.anchor_id),
                    scraped,
                ))
            } else {
                let (md, empty) = extract_item_markdown_from_document(&doc)?;
                Ok(((md, empty), None, None, scraped))
            }
        })
        .await?;
    let item_path = segments.join("::");
    let item_name = segments
        .last()
        .cloned()
        .unwrap_or_else(|| item_path.clone());
    let title = format!("{item_path} ({item_type_echo})");
    let resolved = extract_resolved_version_for_crate(
        &resp.final_url,
        version.as_str(),
        Some(crate_name.as_str()),
    )
    .or(scraped_ver);

    // The anchor id carries the family in its prefix (`variant.Some`), so the
    // real family is already in hand here and never has to be re-derived from
    // the requested kind. Split before the id is moved into the fragment below.
    let anchor_family = matched_anchor
        .as_deref()
        .and_then(|a| a.split_once('.'))
        .map(|(family, _)| family.to_string());

    let mut source_url = resp.final_url.clone();
    // Echo the anchor that exists on the page, not the one the URL builder
    // guessed: a required trait method lives at `#tymethod.X`, and pointing an
    // agent at `#method.X` would send it to a fragment the page never defines.
    if let Some(anchor) = matched_anchor {
        source_url.set_fragment(Some(&anchor));
    } else if let Some(f) = fragment {
        source_url.set_fragment(Some(&f));
    }

    Ok(GetItemData {
        crate_name: crate_name.as_str().to_string(),
        item_type: item_type_echo.to_string(),
        item_path,
        item_name,
        version: version.as_str().to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: source_url.to_string(),
        title,
        cache_hit: resp.cache_hit,
        extraction,
        anchor_family,
        resolved_item_path: None,
    })
}
