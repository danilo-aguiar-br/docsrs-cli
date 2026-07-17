//! docs.rs operations: readme, get-item, search-in-crate.

use std::collections::HashSet;

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::{HOST_DOCS_RS, MAX_SEARCH_IN_CRATE_LIMIT};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::{HttpClient, content_type_looks_html, decode_utf8};
use crate::item_kind::{ItemKind, rustc_crate_name};

/// Crate overview (rustdoc crate docblock, not git README).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadmeData {
    pub crate_name: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_version: Option<String>,
    pub markdown: String,
    pub empty: bool,
    pub truncated: bool,
    pub source_url: String,
}

/// Typed item documentation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetItemData {
    pub crate_name: String,
    pub item_type: String,
    pub item_path: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_version: Option<String>,
    pub markdown: String,
    pub empty: bool,
    pub truncated: bool,
    pub source_url: String,
    pub title: String,
}

/// Single all.html hit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchInCrateHit {
    pub name: String,
    pub kind: String,
    pub url: String,
}

/// search-in-crate result set (`truncated` is true when `total > emitted`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchInCrateData {
    pub crate_name: String,
    pub query: String,
    pub version: String,
    pub total: usize,
    pub emitted: usize,
    pub hits: Vec<SearchInCrateHit>,
    /// True when the hit list was cut by `--limit` (`total > emitted`).
    pub truncated: bool,
    pub source_url: String,
}

/// Build crate index URL with rustc hyphen→underscore segment.
pub fn readme_url(crate_name: &str, version: &str) -> AppResult<Url> {
    readme_url_on_origin(&format!("https://{HOST_DOCS_RS}"), crate_name, version)
}

/// Build readme URL against a custom origin (wiremock tests).
pub fn readme_url_on_origin(origin: &str, crate_name: &str, version: &str) -> AppResult<Url> {
    let rustc = rustc_crate_name(crate_name);
    let origin = origin.trim_end_matches('/');
    let s = format!("{origin}/{crate_name}/{version}/{rustc}/index.html");
    Url::parse(&s).map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid readme URL", e))
}

/// Build rustdoc item or module URL.
pub fn get_item_url(
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    get_item_url_on_origin(
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
        kind,
        segments,
    )
}

/// Build get-item URL against a custom origin (wiremock tests).
pub fn get_item_url_on_origin(
    origin: &str,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<Url> {
    let origin = origin.trim_end_matches('/');
    let rustc_root = rustc_crate_name(crate_name);
    let segs: Vec<String> = {
        let mut s = segments.to_vec();
        if let Some(first) = s.first() {
            let f = first.as_str();
            if f == crate_name || f == rustc_root.as_str() {
                s.remove(0);
            }
        }
        s
    };

    let url_str = if kind == ItemKind::Module {
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
pub fn all_html_url(crate_name: &str, version: &str) -> AppResult<Url> {
    all_html_url_on_origin(&format!("https://{HOST_DOCS_RS}"), crate_name, version)
}

/// Build all.html URL against a custom origin (wiremock tests).
pub fn all_html_url_on_origin(origin: &str, crate_name: &str, version: &str) -> AppResult<Url> {
    let rustc = rustc_crate_name(crate_name);
    let origin = origin.trim_end_matches('/');
    let s = format!("{origin}/{crate_name}/{version}/{rustc}/all.html");
    Url::parse(&s)
        .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid all.html URL", e))
}

fn extract_resolved_version(final_url: &Url, requested: &str) -> Option<String> {
    let mut segs = final_url.path_segments()?.filter(|s| !s.is_empty());
    let _pkg = segs.next()?;
    let ver = segs.next()?;
    if ver == "latest" {
        return None;
    }
    if requested == "latest" || ver != requested {
        return Some(ver.to_string());
    }
    Some(ver.to_string())
}

/// Remove script/style nodes and on* handlers before Markdown conversion.
pub fn sanitize_html_fragment(html: &str) -> String {
    let fragment = Html::parse_fragment(html);
    let drop_sel = Selector::parse("script, style, noscript").expect("static selector");
    let mut cleaned = String::new();
    // Prefer reconstructing from root children excluding dropped tags.
    if let Some(root) = fragment.tree.root().children().next() {
        let _ = root;
    }
    // Collect HTML of non-script/style nodes by stripping matches from a working copy.
    let mut out = html.to_string();
    for el in fragment.select(&drop_sel) {
        let tag_html = el.html();
        out = out.replace(&tag_html, "");
    }
    // Strip event handlers: onload=, onclick=, etc.
    if let Ok(re) = regex::Regex::new(r#"(?i)\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)"#) {
        out = re.replace_all(&out, "").to_string();
    }
    // Drop javascript: URLs in href/src
    if let Ok(re) = regex::Regex::new(r#"(?i)(href|src)\s*=\s*["']\s*javascript:[^"']*["']"#) {
        out = re.replace_all(&out, "").to_string();
    }
    cleaned.push_str(&out);
    cleaned
}

fn html_to_markdown(html: &str) -> AppResult<String> {
    let cleaned = sanitize_html_fragment(html);
    htmd::convert(&cleaned).map_err(|e| {
        AppError::new(
            ErrorKind::Parse,
            format!("HTML to Markdown conversion failed: {e}"),
        )
    })
}

fn first_inner_html(document: &Html, selectors: &[&str]) -> Option<String> {
    for sel in selectors {
        if let Ok(s) = Selector::parse(sel)
            && let Some(n) = document.select(&s).next()
        {
            let inner = n.inner_html();
            if !inner.trim().is_empty() {
                return Some(inner);
            }
        }
    }
    None
}

/// Fetch and convert crate overview docblock (production docs.rs).
pub async fn fetch_readme(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
) -> AppResult<ReadmeData> {
    fetch_readme_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
    )
    .await
}

/// Fetch readme against a configurable origin (offline mocks).
pub async fn fetch_readme_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
) -> AppResult<ReadmeData> {
    let url = readme_url_on_origin(origin, crate_name, version)?;
    fetch_readme_at(http, crate_name, version, &url).await
}

/// Fetch readme from a prebuilt URL (wiremock).
pub async fn fetch_readme_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
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
            "docs.rs readme",
        ));
    }
    if resp.content_type.is_some() && !content_type_looks_html(resp.content_type.as_deref()) {
        tracing::warn!(content_type = ?resp.content_type, "unexpected Content-Type from docs.rs");
    }

    let text = decode_utf8(&resp.body)?;
    let (markdown, empty) = extract_readme_markdown_from_html(&text)?;
    let resolved = extract_resolved_version(&resp.final_url, version);
    Ok(ReadmeData {
        crate_name: crate_name.to_string(),
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
    })
}

/// Fetch and convert a typed rustdoc item page (production docs.rs).
pub async fn fetch_item(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    fetch_item_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
        kind,
        segments,
    )
    .await
}

/// Fetch item against a configurable origin (offline mocks).
pub async fn fetch_item_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
) -> AppResult<GetItemData> {
    let url = get_item_url_on_origin(origin, crate_name, version, kind, segments)?;
    fetch_item_at(http, crate_name, version, kind, segments, &url).await
}

/// Fetch item from a prebuilt URL (wiremock).
pub async fn fetch_item_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    kind: ItemKind,
    segments: &[String],
    url: &Url,
) -> AppResult<GetItemData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("item not found at {url}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "docs.rs get-item",
        ));
    }

    let text = decode_utf8(&resp.body)?;
    let (markdown, empty) = extract_item_markdown_from_html(&text)?;
    let item_path = segments.join("::");
    let title = format!("{item_path} ({})", kind.as_str());
    let resolved = extract_resolved_version(&resp.final_url, version);

    Ok(GetItemData {
        crate_name: crate_name.to_string(),
        item_type: kind.as_str().to_string(),
        item_path,
        version: version.to_string(),
        resolved_version: resolved,
        markdown,
        empty,
        truncated: false,
        source_url: resp.final_url.to_string(),
        title,
    })
}

/// Parse all.html and filter symbols (production docs.rs).
pub async fn search_in_crate(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
) -> AppResult<SearchInCrateData> {
    search_in_crate_on_origin(
        http,
        &format!("https://{HOST_DOCS_RS}"),
        crate_name,
        version,
        query,
        item_type,
        limit,
    )
    .await
}

/// search-in-crate against a configurable origin (offline mocks).
pub async fn search_in_crate_on_origin(
    http: &HttpClient,
    origin: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
) -> AppResult<SearchInCrateData> {
    let url = all_html_url_on_origin(origin, crate_name, version)?;
    search_in_crate_at(http, crate_name, version, query, item_type, limit, &url).await
}

/// search-in-crate against a prebuilt all.html URL (wiremock).
pub async fn search_in_crate_at(
    http: &HttpClient,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
    url: &Url,
) -> AppResult<SearchInCrateData> {
    let resp = http.get_html(url).await?;
    if resp.status.as_u16() == 404 {
        return Err(AppError::from_http_status(
            404,
            &format!("all.html not found for {crate_name}@{version}"),
        ));
    }
    if !resp.status.is_success() {
        return Err(AppError::from_http_status(
            resp.status.as_u16(),
            "docs.rs all.html",
        ));
    }

    let text = decode_utf8(&resp.body)?;
    search_in_crate_from_html(
        &text,
        crate_name,
        version,
        query,
        item_type,
        limit,
        resp.final_url.as_str(),
    )
}

/// Join relative/absolute/full href against crate rustdoc base.
pub fn join_href(base: &Url, href: &str) -> AppResult<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Ok(href.to_string());
    }
    if href.starts_with('/') {
        let joined = Url::parse(&format!("https://docs.rs{href}")).map_err(|e| {
            AppError::with_source(ErrorKind::Parse, "failed to join absolute path href", e)
        })?;
        return Ok(joined.to_string());
    }
    base.join(href)
        .map(|u| u.to_string())
        .map_err(|e| AppError::with_source(ErrorKind::Parse, "failed to join relative href", e))
}

/// Pure parse of all.html body for offline tests.
pub fn parse_all_html_hits(
    html: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: usize,
) -> AppResult<Vec<SearchInCrateHit>> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let document = Html::parse_document(html);
    let a_sel = Selector::parse("#main-content a").expect("static selector");
    let rustc = rustc_crate_name(crate_name);
    let base = Url::parse(&format!(
        "https://{HOST_DOCS_RS}/{crate_name}/{version}/{rustc}/"
    ))
    .map_err(|e| AppError::with_source(ErrorKind::Internal, "invalid all.html base", e))?;
    let q = query.trim().to_ascii_lowercase();
    let mut hits = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    for a in document.select(&a_sel) {
        let name = a.text().collect::<String>().trim().to_string();
        let href = match a.value().attr("href") {
            Some(h) if !h.is_empty() => h,
            _ => continue,
        };
        if name.is_empty() {
            continue;
        }
        let Some(kind) = ItemKind::from_href(href) else {
            continue;
        };
        if let Some(filter) = item_type
            && kind != filter
        {
            continue;
        }
        if !q.is_empty() && !name.to_ascii_lowercase().contains(&q) {
            continue;
        }
        let abs = join_href(&base, href)?;
        let key = (name.clone(), kind.as_str().to_string());
        if !seen.insert(key) {
            continue;
        }
        hits.push(SearchInCrateHit {
            name,
            kind: kind.as_str().to_string(),
            url: abs,
        });
        if hits.len() >= limit {
            break;
        }
    }
    Ok(hits)
}

/// Extract readme markdown from raw HTML (offline tests / pure path).
pub fn extract_readme_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    let document = Html::parse_document(html);
    let content = first_inner_html(
        &document,
        &[
            ".rustdoc .docblock",
            "#main-content .docblock",
            ".docblock",
            ".rustdoc-main .item-decl",
            ".item-decl",
        ],
    );
    match content {
        Some(h) => Ok((html_to_markdown(&h)?, false)),
        None => Ok((String::new(), true)),
    }
}

/// Extract get-item markdown from raw HTML (offline tests / pure path).
pub fn extract_item_markdown_from_html(html: &str) -> AppResult<(String, bool)> {
    let document = Html::parse_document(html);
    let content = first_inner_html(&document, &["#main-content", ".item-decl", ".docblock"])
        .unwrap_or_default();
    let empty = content.trim().is_empty();
    if empty {
        Ok((String::new(), true))
    } else {
        Ok((html_to_markdown(&content)?, false))
    }
}

/// Build SearchInCrateData from HTML body without network (offline tests).
///
/// `--limit 0` is honoured: `emitted = 0`, `hits = []`, and `truncated` is true
/// when the unfiltered total is greater than zero. Values above
/// [`MAX_SEARCH_IN_CRATE_LIMIT`] are capped.
pub fn search_in_crate_from_html(
    html: &str,
    crate_name: &str,
    version: &str,
    query: &str,
    item_type: Option<ItemKind>,
    limit: u32,
    source_url: &str,
) -> AppResult<SearchInCrateData> {
    let limit = (limit as usize).min(MAX_SEARCH_IN_CRATE_LIMIT as usize);
    let all = parse_all_html_hits(html, crate_name, version, query, item_type, usize::MAX)?;
    let total = all.len();
    let hits: Vec<_> = all.into_iter().take(limit).collect();
    let emitted = hits.len();
    Ok(SearchInCrateData {
        crate_name: crate_name.to_string(),
        query: query.to_string(),
        version: version.to_string(),
        total,
        emitted,
        hits,
        truncated: total > emitted,
        source_url: source_url.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_url_hyphen() {
        let u = readme_url("async-trait", "latest").unwrap();
        assert_eq!(
            u.as_str(),
            "https://docs.rs/async-trait/latest/async_trait/index.html"
        );
    }

    #[test]
    fn get_item_nested() {
        let segs = vec!["tokio".into(), "runtime".into(), "Runtime".into()];
        let u = get_item_url("tokio", "latest", ItemKind::Struct, &segs).unwrap();
        assert!(u.as_str().contains("/tokio/runtime/struct.Runtime.html"));
    }

    #[test]
    fn get_item_without_crate_prefix() {
        let segs = vec!["Parser".into()];
        let u = get_item_url("clap", "latest", ItemKind::Trait, &segs).unwrap();
        assert!(
            u.as_str().ends_with("/clap/trait.Parser.html")
                || u.path().ends_with("/clap/trait.Parser.html")
        );
    }

    #[test]
    fn join_relative() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let j = join_href(&base, "de/trait.Deserialize.html").unwrap();
        assert!(j.contains("/serde/de/trait.Deserialize.html"));
    }

    #[test]
    fn join_absolute_path_and_fragment() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let j = join_href(&base, "/serde/latest/serde/struct.Error.html").unwrap();
        assert!(j.starts_with("https://docs.rs/serde/"));
        let f = join_href(&base, "struct.Error.html#method.to_string").unwrap();
        assert!(f.contains("#method.to_string"));
    }

    #[test]
    fn sanitize_strips_script() {
        let raw = r#"<div>ok<script>alert(1)</script><p onclick="x()">hi</p></div>"#;
        let s = sanitize_html_fragment(raw);
        assert!(!s.contains("script"));
        assert!(!s.contains("onclick"));
        assert!(s.contains("hi") || s.contains("ok"));
    }

    #[test]
    fn readme_fallback_item_decl_converts() {
        let html = r#"<!DOCTYPE html><html><body class="rustdoc-main">
        <div class="item-decl"><pre>pub use foo;</pre></div>
        </body></html>"#;
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.trim().is_empty());
    }

    #[test]
    fn all_html_order_stable() {
        let html = r#"<!DOCTYPE html><html><body>
        <div id="main-content">
          <a href="struct.A.html">A</a>
          <a href="trait.B.html">B</a>
          <a href="fn.c.html">c</a>
          <a href="union.U.html">U</a>
          <a href="attr.x.html">x</a>
          <a href="derive.D.html">D</a>
        </div></body></html>"#;
        let hits = parse_all_html_hits(html, "demo", "1.0.0", "", None, 100).unwrap();
        assert_eq!(hits.len(), 6);
        assert_eq!(hits[0].name, "A");
        assert_eq!(hits[0].kind, "struct");
        assert_eq!(hits[3].kind, "union");
        assert_eq!(hits[4].kind, "attribute");
        assert_eq!(hits[5].kind, "derive");
    }

    #[test]
    fn std_core_alloc_urls() {
        let u = readme_url("std", "latest").unwrap();
        assert!(u.as_str().contains("/std/latest/std/index.html"));
        let u = get_item_url("core", "latest", ItemKind::Struct, &["Option".into()]).unwrap();
        assert!(u.as_str().contains("/core/struct.Option.html"));
    }

    #[test]
    fn extract_item_from_fixture() {
        let html = include_str!("../tests/fixtures/docs_rs/get_item_main.html");
        let (md, empty) = extract_item_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(md.contains("Runtime") || md.contains("Tokio") || md.contains("runtime"));
    }

    #[test]
    fn extract_readme_primary_docblock_fixture() {
        let html = include_str!("../tests/fixtures/docs_rs/readme_docblock.html");
        let (md, empty) = extract_readme_markdown_from_html(html).unwrap();
        assert!(!empty);
        assert!(!md.to_ascii_lowercase().contains("alert"));
    }

    #[test]
    fn search_in_crate_from_html_limit_and_filter() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            Some(ItemKind::Struct),
            1,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert_eq!(data.total, 1);
        assert_eq!(data.emitted, 1);
        assert_eq!(data.hits[0].kind, "struct");
        assert!(!data.truncated);
    }

    #[test]
    fn search_in_crate_truncated_when_limit_cuts_hits() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            2,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert!(data.total > 2, "fixture must have more than 2 hits");
        assert_eq!(data.emitted, 2);
        assert!(data.truncated);
        let full = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            1000,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert_eq!(full.total, full.emitted);
        assert!(!full.truncated);
    }

    #[test]
    fn search_in_crate_limit_zero_emits_nothing() {
        let html = include_str!("../tests/fixtures/docs_rs/all_html_sample.html");
        let data = search_in_crate_from_html(
            html,
            "demo",
            "1.0.0",
            "",
            None,
            0,
            "https://docs.rs/demo/1.0.0/demo/all.html",
        )
        .unwrap();
        assert!(data.total > 0, "fixture must have hits for total");
        assert_eq!(data.emitted, 0);
        assert!(data.hits.is_empty());
        assert!(data.truncated);
        assert!(
            parse_all_html_hits(html, "demo", "1.0.0", "", None, 0)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn resolved_version_from_url() {
        let u = Url::parse("https://docs.rs/serde/1.0.210/serde/index.html").unwrap();
        assert_eq!(
            extract_resolved_version(&u, "latest").as_deref(),
            Some("1.0.210")
        );
        let u2 = Url::parse("https://docs.rs/serde/latest/serde/index.html").unwrap();
        assert!(extract_resolved_version(&u2, "latest").is_none());
    }

    #[test]
    fn module_url_template() {
        let segs = vec!["serde".into(), "de".into()];
        let u = get_item_url("serde", "latest", ItemKind::Module, &segs).unwrap();
        assert!(u.as_str().ends_with("/serde/de/index.html"));
    }

    #[test]
    fn empty_html_readme() {
        let (md, empty) = extract_readme_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }

    #[test]
    fn get_item_empty_segments_errors() {
        let err = get_item_url("clap", "latest", ItemKind::Struct, &[]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn join_full_http_url_passthrough() {
        let base = Url::parse("https://docs.rs/serde/latest/serde/").unwrap();
        let full = "https://docs.rs/serde/latest/serde/struct.Error.html";
        assert_eq!(join_href(&base, full).unwrap(), full);
        let http = "http://example.invalid/x.html";
        assert_eq!(join_href(&base, http).unwrap(), http);
    }

    #[test]
    fn origin_builders_trim_trailing_slash() {
        let u = readme_url_on_origin("https://docs.rs/", "demo", "1.0.0").unwrap();
        assert_eq!(u.as_str(), "https://docs.rs/demo/1.0.0/demo/index.html");
        let a = all_html_url_on_origin("https://docs.rs/", "demo", "1.0.0").unwrap();
        assert_eq!(a.as_str(), "https://docs.rs/demo/1.0.0/demo/all.html");
    }

    #[test]
    fn empty_item_html_extract() {
        let (md, empty) = extract_item_markdown_from_html("<html><body></body></html>").unwrap();
        assert!(empty);
        assert!(md.is_empty());
    }
}
