//! Human Markdown rendering for the embedded JSON Schemas (`schema --format markdown`).

use crate::error::{AppError, AppResult, ErrorDetail, InternalOp};

/// Render a human-readable markdown document for an embedded JSON Schema.
///
/// Includes title, description, required fields, a property table, and the
/// raw JSON Schema fenced for agents that still want the machine schema.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Internal`] when pretty-printing the embedded schema fails
/// (should not happen for valid `serde_json::Value` trees).
pub fn render_schema_markdown(cmd: &str, schema: &serde_json::Value) -> AppResult<String> {
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
    let pretty = serde_json::to_string_pretty(schema).map_err(|e| {
        AppError::of_with_source(
            ErrorDetail::Internal {
                op: InternalOp::JsonPrettyPrint,
            },
            e,
        )
    })?;
    out.push_str(&pretty);
    out.push_str("\n```\n");
    Ok(out)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_label_covers_scalar_union_and_containers() {
        assert_eq!(
            schema_type_label(&serde_json::json!({"type": "string"})),
            "string"
        );
        assert_eq!(
            schema_type_label(&serde_json::json!({"type": ["string", "null"]})),
            "string | null"
        );
        assert_eq!(
            schema_type_label(&serde_json::json!({"properties": {}})),
            "object"
        );
        assert_eq!(
            schema_type_label(&serde_json::json!({"items": {}})),
            "array"
        );
        assert_eq!(schema_type_label(&serde_json::json!({})), "any");
    }

    #[test]
    fn markdown_lists_required_and_properties() {
        let schema = serde_json::json!({
            "title": "demo",
            "description": "a demo schema",
            "required": ["a"],
            "properties": {
                "a": {"type": "string", "description": "pipe | escaped"},
                "b": {"type": "integer"}
            }
        });
        let md = render_schema_markdown("demo", &schema).expect("render");
        assert!(md.contains("# Schema: `demo`"));
        assert!(md.contains("**Title:** demo"));
        assert!(md.contains("| `a` | string | yes |"));
        assert!(md.contains("| `b` | integer | no |"));
        assert!(md.contains("pipe \\| escaped"), "table pipes are escaped");
        assert!(md.contains("```json"));
    }

    #[test]
    fn markdown_without_properties_emits_placeholder_row() {
        let md = render_schema_markdown("empty", &serde_json::json!({})).expect("render");
        assert!(md.contains("_(none)_"));
    }
}
