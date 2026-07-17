//! Markdown and JSON rendering with output truncation.

use serde::Serialize;
use serde_json::json;

use crate::config::{Config, SCHEMA_VERSION};
use crate::crates_io::SearchCratesData;
use crate::docs_rs::{GetItemData, ReadmeData, SearchInCrateData};
use crate::error::{AppError, ErrorKind};

/// JSON error envelope for agents.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub schema_version: u32,
    pub ok: bool,
    pub error: ErrorBody,
}

/// Error body fields inside the envelope.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: u8,
    pub kind: String,
    pub message: String,
    pub retryable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

/// Build a success envelope (`schema_version`, `ok`, `command`, `data`, `duration_ms`).
pub fn success_envelope<T: Serialize>(
    command: &str,
    data: &T,
    duration_ms: u64,
    source_url: Option<&str>,
) -> serde_json::Value {
    let mut v = json!({
        "schema_version": SCHEMA_VERSION,
        "ok": true,
        "command": command,
        "data": data,
        "duration_ms": duration_ms,
    });
    if let Some(u) = source_url
        && let Some(obj) = v.as_object_mut()
    {
        obj.insert("source_url".into(), json!(u));
    }
    v
}

/// Build a dry-run success envelope with planned URL/params.
pub fn dry_run_envelope(
    command: &str,
    planned_url: &str,
    planned_params: serde_json::Value,
    duration_ms: u64,
) -> serde_json::Value {
    json!({
        "schema_version": SCHEMA_VERSION,
        "ok": true,
        "command": command,
        "dry_run": true,
        "data": {
            "planned_url": planned_url,
            "planned_params": planned_params,
        },
        "duration_ms": duration_ms,
    })
}

/// Build an error envelope. SIGINT/SIGTERM surface as kind `canceled` with exit 130/143.
pub fn error_envelope(err: &AppError) -> ErrorEnvelope {
    let kind = err.kind();
    let kind_str = match kind {
        ErrorKind::Interrupted | ErrorKind::Terminated => "canceled",
        other => other.as_str(),
    };
    ErrorEnvelope {
        schema_version: SCHEMA_VERSION,
        ok: false,
        error: ErrorBody {
            code: kind.exit_code(),
            kind: kind_str.to_string(),
            message: err.message().to_string(),
            retryable: kind.retryable(),
            retry_after_secs: err.retry_after_secs(),
        },
    }
}

/// Truncate UTF-8 text on a char boundary; returns (text, truncated).
pub fn truncate_output(text: &str, max_bytes: u64) -> (String, bool) {
    if max_bytes == 0 {
        return (String::new(), true);
    }
    let max = max_bytes as usize;
    if text.len() <= max {
        return (text.to_string(), false);
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    (text[..end].to_string(), true)
}

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
        out.push_str(&format!(
            "**Documentation:** {}\n\n---\n\n",
            h.documentation.as_deref().unwrap_or("N/A")
        ));
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
    let term = if data.query.is_empty() {
        "all items".to_string()
    } else {
        data.query.clone()
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

/// Apply max_output_bytes truncation to readme payload.
pub fn apply_truncation_to_readme(mut data: ReadmeData, cfg: &Config) -> ReadmeData {
    let (md, trunc) = truncate_output(&data.markdown, cfg.max_output_bytes);
    data.markdown = md;
    data.truncated = trunc;
    data
}

/// Apply max_output_bytes truncation to get-item payload.
pub fn apply_truncation_to_item(mut data: GetItemData, cfg: &Config) -> GetItemData {
    let (md, trunc) = truncate_output(&data.markdown, cfg.max_output_bytes);
    data.markdown = md;
    data.truncated = trunc;
    data
}

/// Map clap parse failures to usage errors.
pub fn usage_error(msg: impl Into<String>) -> AppError {
    AppError::new(ErrorKind::Usage, msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crates_io::{SearchCratesData, SearchMeta};
    use crate::docs_rs::{GetItemData, ReadmeData};
    use crate::error::AppError;

    #[test]
    fn truncate_marks_flag() {
        let (out, trunc) = truncate_output("abcdef", 3);
        assert!(trunc);
        assert_eq!(out, "abc");
    }

    #[test]
    fn cancel_kinds_in_envelope() {
        let e = error_envelope(&AppError::terminated());
        assert_eq!(e.error.code, 143);
        assert_eq!(e.error.kind, "canceled");
        let e = error_envelope(&AppError::interrupted());
        assert_eq!(e.error.code, 130);
        assert_eq!(e.error.kind, "canceled");
    }

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
        };
        assert!(render_readme_markdown(&r).contains("No documentation"));
        let i = GetItemData {
            crate_name: "c".into(),
            item_type: "fn".into(),
            item_path: "c::f".into(),
            version: "latest".into(),
            resolved_version: None,
            markdown: String::new(),
            empty: true,
            truncated: false,
            source_url: "u".into(),
            title: "c::f (fn)".into(),
        };
        assert!(render_item_markdown(&i).contains("No documentation"));
    }

    #[test]
    fn success_and_dry_run_envelopes() {
        let v = success_envelope("version", &serde_json::json!({"n":1}), 3, Some("https://x"));
        assert_eq!(v["ok"], true);
        assert_eq!(v["source_url"], "https://x");
        let d = dry_run_envelope("readme", "https://u", serde_json::json!({}), 1);
        assert_eq!(d["dry_run"], true);
        assert!(usage_error("x").kind() == ErrorKind::Usage);
    }

    #[test]
    fn apply_truncation_sets_flags() {
        let cfg = Config {
            max_output_bytes: 4,
            ..Config::default()
        };
        let r = apply_truncation_to_readme(
            ReadmeData {
                crate_name: "c".into(),
                version: "1".into(),
                resolved_version: None,
                markdown: "hello world".into(),
                empty: false,
                truncated: false,
                source_url: "u".into(),
            },
            &cfg,
        );
        assert!(r.truncated);
        assert!(r.markdown.len() <= 4);
        let i = apply_truncation_to_item(
            GetItemData {
                crate_name: "c".into(),
                item_type: "fn".into(),
                item_path: "c::f".into(),
                version: "1".into(),
                resolved_version: None,
                markdown: "abcdef".into(),
                empty: false,
                truncated: false,
                source_url: "u".into(),
                title: "t".into(),
            },
            &cfg,
        );
        assert!(i.truncated);
        assert_eq!(i.markdown, "abcd");
    }

    #[test]
    fn truncate_zero_and_utf8_boundary() {
        let (out, trunc) = truncate_output("abc", 0);
        assert!(trunc);
        assert!(out.is_empty());
        let s = "áéí"; // multi-byte
        let (out, trunc) = truncate_output(s, 2);
        assert!(trunc);
        assert!(out.is_char_boundary(out.len()));
    }

    #[test]
    fn empty_search_in_crate_markdown() {
        use crate::docs_rs::SearchInCrateData;
        let data = SearchInCrateData {
            crate_name: "c".into(),
            query: String::new(),
            version: "1".into(),
            total: 0,
            emitted: 0,
            hits: vec![],
            truncated: false,
            source_url: "u".into(),
        };
        let md = render_search_in_crate_markdown(&data);
        assert!(md.contains("all items"));
        assert!(md.contains("No matching items"));
    }
}
