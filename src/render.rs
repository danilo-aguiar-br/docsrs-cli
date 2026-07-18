//! Markdown and JSON rendering with output truncation.

use serde::Serialize;

use crate::config::{Config, SCHEMA_VERSION};
use crate::crates_io::SearchCratesData;
use crate::docs_rs::{GetItemData, ReadmeData, SearchInCrateData};
use crate::error::{AppError, ErrorKind};

/// JSON success envelope for agents (`schema_version`, `ok`, `command`, `data`, `duration_ms`).
///
/// Typed wire shape — serializes directly without an intermediate `serde_json::Value`.
/// Optional fields use omit-on-`None` (`skip_serializing_if`), not JSON `null`.
#[derive(Debug, Serialize)]
pub struct SuccessEnvelope<'a, T: Serialize> {
    /// Envelope schema version.
    pub schema_version: u32,
    /// Always `true` for success envelopes.
    pub ok: bool,
    /// Command name that produced this payload.
    pub command: &'a str,
    /// Command-specific typed data payload.
    pub data: T,
    /// Wall-clock duration of the command in milliseconds.
    pub duration_ms: u64,
    /// Final source URL when the command fetched remote content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<&'a str>,
}

/// JSON dry-run success envelope with planned URL and params.
///
/// `P` is a typed per-command params struct (`Serialize`). No intermediate
/// `serde_json::Value` on the agent write path.
#[derive(Debug, Serialize)]
pub struct DryRunEnvelope<'a, P: Serialize> {
    /// Envelope schema version.
    pub schema_version: u32,
    /// Always `true` for dry-run envelopes.
    pub ok: bool,
    /// Command name that would have run.
    pub command: &'a str,
    /// Always `true` — marks the response as a dry-run plan.
    pub dry_run: bool,
    /// Planned request details.
    pub data: DryRunData<'a, P>,
    /// Wall-clock duration until the plan was emitted.
    pub duration_ms: u64,
}

/// Dry-run `data` object (`planned_url` + typed `planned_params`).
#[derive(Debug, Serialize)]
pub struct DryRunData<'a, P: Serialize> {
    /// Absolute URL that would be requested.
    pub planned_url: &'a str,
    /// Command-specific planned query/body parameters (typed struct).
    pub planned_params: P,
}

/// JSON error envelope for agents.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    /// Envelope schema version.
    pub schema_version: u32,
    /// Always `false` for error envelopes.
    pub ok: bool,
    /// Structured error body.
    pub error: ErrorBody,
}

/// Error body fields inside the envelope.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Process exit code as an integer.
    pub code: u8,
    /// Snake_case error kind.
    pub kind: String,
    /// Technical English message.
    pub message: String,
    /// Whether an agent may retry after backoff.
    pub retryable: bool,
    /// Optional Retry-After seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_after_secs: Option<u64>,
}

/// Render a human-readable markdown document for an embedded JSON Schema.
///
/// Includes title, description, required fields, a property table, and the
/// raw JSON Schema fenced for agents that still want the machine schema.
pub fn render_schema_markdown(cmd: &str, schema: &serde_json::Value) -> String {
    // Schema markdown is small but non-trivial; avoid repeated small reallocs.
    let mut out = String::with_capacity(1024);
    let title = schema.get("title").and_then(|v| v.as_str()).unwrap_or(cmd);
    out.push_str(&format!("# Schema: `{cmd}`\n\n"));
    out.push_str(&format!("**Title:** {title}\n\n"));
    if let Some(desc) = schema.get("description").and_then(|v| v.as_str()) {
        out.push_str(&format!("{desc}\n\n"));
    }
    if let Some(req) = schema.get("required").and_then(|v| v.as_array()) {
        out.push_str("## Required fields\n\n");
        for r in req {
            if let Some(name) = r.as_str() {
                out.push_str(&format!("- `{name}`\n"));
            }
        }
        out.push('\n');
    }
    out.push_str("## Properties\n\n");
    out.push_str("| property | type | required | description |\n");
    out.push_str("| --- | --- | --- | --- |\n");
    let required: std::collections::HashSet<&str> = schema
        .get("required")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).collect())
        .unwrap_or_default();
    if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
        for (name, prop) in props {
            let ty = schema_type_label(prop);
            let is_req = if required.contains(name.as_str()) {
                "yes"
            } else {
                "no"
            };
            let desc = prop
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .replace('|', "\\|");
            out.push_str(&format!("| `{name}` | {ty} | {is_req} | {desc} |\n"));
        }
    } else {
        out.push_str("| _(none)_ | | | |\n");
    }
    out.push_str("\n## JSON Schema\n\n");
    out.push_str("```json\n");
    out.push_str(&serde_json::to_string_pretty(schema).unwrap_or_else(|_| "{}".to_string()));
    out.push_str("\n```\n");
    out
}

fn schema_type_label(prop: &serde_json::Value) -> String {
    if let Some(t) = prop.get("type") {
        if let Some(s) = t.as_str() {
            return s.to_string();
        }
        if let Some(arr) = t.as_array() {
            let parts: Vec<&str> = arr.iter().filter_map(|v| v.as_str()).collect();
            if !parts.is_empty() {
                return parts.join(" | ");
            }
        }
    }
    if prop.get("properties").is_some() {
        return "object".into();
    }
    if prop.get("items").is_some() {
        return "array".into();
    }
    "any".into()
}

/// Build a typed success envelope (`schema_version`, `ok`, `command`, `data`, `duration_ms`).
pub fn success_envelope<'a, T: Serialize>(
    command: &'a str,
    data: T,
    duration_ms: u64,
    source_url: Option<&'a str>,
) -> SuccessEnvelope<'a, T> {
    SuccessEnvelope {
        schema_version: SCHEMA_VERSION,
        ok: true,
        command,
        data,
        duration_ms,
        source_url,
    }
}

/// Build a typed dry-run success envelope with planned URL/params.
pub fn dry_run_envelope<'a, P: Serialize>(
    command: &'a str,
    planned_url: &'a str,
    planned_params: P,
    duration_ms: u64,
) -> DryRunEnvelope<'a, P> {
    DryRunEnvelope {
        schema_version: SCHEMA_VERSION,
        ok: true,
        command,
        dry_run: true,
        data: DryRunData {
            planned_url,
            planned_params,
        },
        duration_ms,
    }
}

/// Build an error envelope. SIGINT/SIGTERM surface as kind `canceled` with exit 130/143.
pub fn error_envelope(err: &AppError) -> ErrorEnvelope {
    let kind = err.kind();
    let kind_str = match kind {
        ErrorKind::Interrupted | ErrorKind::Terminated => "canceled",
        ErrorKind::BrokenPipe => "broken_pipe",
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
    use crate::crates_io::{CrateSearchHit, SearchCratesData, SearchMeta};
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
            cache_hit: false,
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
        };
        assert!(render_item_markdown(&i).contains("No documentation"));
    }

    #[test]
    fn success_and_dry_run_envelopes() {
        let payload = serde_json::json!({"n": 1});
        let v = success_envelope("version", &payload, 3, Some("https://x"));
        let v = serde_json::to_value(&v).expect("serialize success envelope");
        assert_eq!(v["ok"], true);
        assert_eq!(v["source_url"], "https://x");
        assert_eq!(v["schema_version"], SCHEMA_VERSION);
        // Optional source_url omitted when None (not JSON null).
        let no_url = success_envelope("version", &payload, 1, None);
        let no_url = serde_json::to_value(&no_url).expect("serialize");
        assert!(no_url.get("source_url").is_none());
        #[derive(serde::Serialize)]
        struct EmptyParams {}
        let d = dry_run_envelope("readme", "https://u", EmptyParams {}, 1);
        let d = serde_json::to_value(&d).expect("serialize dry-run envelope");
        assert_eq!(d["dry_run"], true);
        assert_eq!(d["data"]["planned_url"], "https://u");
        assert_eq!(d["data"]["planned_params"], serde_json::json!({}));
        assert!(usage_error("x").kind() == ErrorKind::Usage);
    }

    #[test]
    fn optional_fields_omitted_not_null() {
        let hit = CrateSearchHit {
            name: "x".into(),
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
        };
        let v = serde_json::to_value(&hit).expect("serialize hit");
        assert!(v.get("documentation").is_none());
        assert!(v.get("repository").is_none());
        assert!(!v
            .as_object()
            .expect("object")
            .values()
            .any(|x| x.is_null()));

        let readme = ReadmeData {
            crate_name: "c".into(),
            version: "latest".into(),
            resolved_version: None,
            markdown: "m".into(),
            empty: false,
            truncated: false,
            source_url: "https://example.com".into(),
            cache_hit: false,
        };
        let r = serde_json::to_value(&readme).expect("serialize readme");
        assert!(r.get("resolved_version").is_none());
        assert!(!r
            .as_object()
            .expect("object")
            .values()
            .any(|x| x.is_null()));
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
            cache_hit: false,
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
                item_name: "f".into(),
                version: "1".into(),
                resolved_version: None,
                markdown: "abcdef".into(),
                empty: false,
                truncated: false,
                source_url: "u".into(),
                title: "t".into(),
            cache_hit: false,
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
}
