//! Human-readable Markdown rendering for each command payload.
//!
//! Used when stdout is a TTY or `--format markdown|text` forces the human path.
//! Agents receive the JSON envelope instead (see [`super::envelope`]).

use crate::config::{HOST_DOCS_RS, SCHEME_HTTPS};
use crate::crates_io::SearchCratesData;
use crate::docs_rs::{GetItemData, ReadmeData, SearchInCrateData};

/// Human Markdown for search-crates.
pub fn render_search_markdown(data: &SearchCratesData) -> String {
    let mut out = format!(
        "# Crate Search Results for \"{}\" (page {})\n\n",
        data.query, data.page
    );
    if data.hits.is_empty() {
        out.push_str("No crates found.\n");
        return out;
    }
    for h in &data.hits {
        out.push_str(&format!("## {} ({})\n\n", h.name, h.version));
        out.push_str(&format!("**Description:** {}\n\n", h.description));
        out.push_str(&format!("**Downloads:** {}\n\n", h.downloads));
        let docs = h
            .documentation
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}/{}", h.name));
        out.push_str(&format!("**Documentation:** {docs}\n\n---\n\n"));
    }
    out
}

/// Human Markdown for readme.
pub fn render_readme_markdown(data: &ReadmeData) -> String {
    let mut out = format!("# {} Documentation\n\n", data.crate_name);
    if data.empty {
        out.push_str("No documentation content found.\n");
    } else {
        out.push_str(&data.markdown);
        out.push('\n');
    }
    out
}

/// Human Markdown for get-item.
pub fn render_item_markdown(data: &GetItemData) -> String {
    let mut out = format!("# {}\n\n", data.title);
    out.push_str(&format!("**Documentation URL:** {}\n\n", data.source_url));
    if data.empty {
        out.push_str("No documentation content found.\n");
    } else {
        out.push_str(&data.markdown);
        out.push('\n');
    }
    out
}

/// Human Markdown for search-in-crate.
pub fn render_search_in_crate_markdown(data: &SearchInCrateData) -> String {
    // Borrow query text — no String clone for the heading.
    let term = if data.query.is_empty() {
        "all items"
    } else {
        data.query.as_str()
    };
    let mut out = format!(
        "# Search Results for \"{}\" in {}\n\nFound {} items (emitted {})\n\n",
        term, data.crate_name, data.total, data.emitted
    );
    if data.hits.is_empty() {
        out.push_str("No matching items found.\n");
        return out;
    }
    for h in &data.hits {
        out.push_str(&format!("## {} ({})\n\n", h.name, h.kind));
        out.push_str(&format!("**Link:** {}\n\n---\n\n", h.url));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crates_io::{CrateSearchHit, SearchMeta};

    #[test]
    fn empty_search_markdown() {
        let data = SearchCratesData {
            query: "z".into(),
            page: 1,
            per_page: 10,
            sort: "relevance".into(),
            hits: vec![],
            meta: SearchMeta {
                total: 0,
                next_page: None,
                prev_page: None,
            },
            cache_hit: false,
            truncated: false,
        };
        let md = render_search_markdown(&data);
        assert!(md.contains("No crates found"));
    }

    #[test]
    fn empty_readme_and_item() {
        let r = ReadmeData {
            crate_name: "c".into(),
            version: "latest".into(),
            resolved_version: None,
            markdown: String::new(),
            empty: true,
            truncated: false,
            source_url: "u".into(),
            cache_hit: false,
        };
        assert!(render_readme_markdown(&r).contains("No documentation"));
        let i = GetItemData {
            crate_name: "c".into(),
            item_type: "fn".into(),
            item_path: "c::f".into(),
            item_name: "f".into(),
            version: "latest".into(),
            resolved_version: None,
            markdown: String::new(),
            empty: true,
            truncated: false,
            source_url: "u".into(),
            title: "c::f (fn)".into(),
            cache_hit: false,
            extraction: None,
            anchor_family: None,
            resolved_item_path: None,
        };
        assert!(render_item_markdown(&i).contains("No documentation"));
    }

    #[test]
    fn empty_search_in_crate_markdown() {
        let data = SearchInCrateData {
            crate_name: "c".into(),
            query: String::new(),
            version: "1".into(),
            item_type: None,
            match_mode: "prefix".into(),
            total: 0,
            emitted: 0,
            hits: vec![],
            truncated: false,
            source_url: "u".into(),
            cache_hit: false,
        };
        let md = render_search_in_crate_markdown(&data);
        assert!(md.contains("all items"));
        assert!(md.contains("No matching items"));
    }

    #[test]
    fn search_docs_fallback_docs_rs_url() {
        let data = SearchCratesData {
            query: "x".into(),
            page: 1,
            per_page: 10,
            sort: "relevance".into(),
            hits: vec![CrateSearchHit {
                name: "foo".into(),
                description: "d".into(),
                downloads: 1,
                version: "1.0.0".into(),
                documentation: None,
                max_version: None,
                max_stable_version: None,
                default_version: None,
                recent_downloads: None,
                exact_match: None,
                yanked: None,
                repository: None,
                homepage: None,
            }],
            meta: SearchMeta {
                total: 1,
                next_page: None,
                prev_page: None,
            },
            cache_hit: false,
            truncated: false,
        };
        let md = render_search_markdown(&data);
        assert!(md.contains("https://docs.rs/foo"));
        assert!(!md.contains("N/A"));
    }
}
