//! Typed JSON envelopes for the agent contract (success, dry-run, error).
//!
//! Every envelope serializes directly from a typed struct — no intermediate
//! `serde_json::Value` on the write path. Optional fields are omitted on `None`
//! (`skip_serializing_if`) rather than emitted as JSON `null`.

use serde::Serialize;

use crate::config::SCHEMA_VERSION;
use crate::error::{AppError, ErrorDetail, ErrorKind, Suggestion};

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
///
/// Parity with success envelopes: agents always get `command` + `duration_ms`
/// (Camada Y / GAP-X-004) so failures correlate with the attempted wire command.
#[derive(Debug, Serialize)]
pub struct ErrorEnvelope<'a> {
    /// Envelope schema version.
    pub schema_version: u32,
    /// Always `false` for error envelopes.
    pub ok: bool,
    /// Wire command name (`get-item`, `usage`, `cache-path`, …).
    pub command: &'a str,
    /// Wall-clock duration of the attempt in milliseconds.
    pub duration_ms: u64,
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
    /// Ranked `--suggest` alternatives, best first.
    ///
    /// Present only when the recovery ladder produced at least one candidate, so
    /// an ordinary not-found keeps the shape it always had. The same list is also
    /// spelled out inside [`Self::message`] for humans; agents read it here
    /// instead of splitting that sentence apart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestions: Option<Vec<Suggestion>>,
}

/// Build a typed success envelope (`schema_version`, `ok`, `command`, `data`, `duration_ms`).
pub fn success_envelope<'a, T: Serialize>(
    command: &'a str,
    data: T,
    duration_ms: u64,
    source_url: Option<&'a str>,
) -> SuccessEnvelope<'a, T> {
    success_envelope_with_ok(command, data, duration_ms, source_url, true)
}

/// Build a success-shaped envelope with an explicit top-level `ok` (used by `doctor`).
pub fn success_envelope_with_ok<'a, T: Serialize>(
    command: &'a str,
    data: T,
    duration_ms: u64,
    source_url: Option<&'a str>,
    ok: bool,
) -> SuccessEnvelope<'a, T> {
    SuccessEnvelope {
        schema_version: SCHEMA_VERSION,
        ok,
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
///
/// `command` is the wire name of the attempted command (`usage` when clap fails
/// before dispatch). `duration_ms` is wall time since process start for that
/// attempt (may be `0` for early parse failures).
pub fn error_envelope<'a>(err: &AppError, command: &'a str, duration_ms: u64) -> ErrorEnvelope<'a> {
    let kind = err.kind();
    let kind_str = match kind {
        ErrorKind::Interrupted | ErrorKind::Terminated => "canceled",
        ErrorKind::BrokenPipe => "broken_pipe",
        other => other.as_str(),
    };
    ErrorEnvelope {
        schema_version: SCHEMA_VERSION,
        ok: false,
        command,
        duration_ms,
        error: ErrorBody {
            code: kind.exit_code(),
            kind: kind_str.to_string(),
            message: err.message().to_string(),
            retryable: kind.retryable(),
            retry_after_secs: err.retry_after_secs(),
            // Empty stays absent: `--suggest` on a leaf close to nothing renders
            // "(none)" in the sentence, and an empty array would read as a
            // different claim than "no suggestion field at all".
            suggestions: err
                .suggestions()
                .filter(|s| !s.is_empty())
                .map(<[Suggestion]>::to_vec),
        },
    }
}

/// Map clap parse failures to usage errors.
pub fn usage_error(msg: impl Into<String>) -> AppError {
    AppError::of(ErrorDetail::ClapUsage {
        message: msg.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crates_io::CrateSearchHit;
    use crate::docs_rs::ReadmeData;

    #[test]
    fn cancel_kinds_in_envelope() {
        let e = error_envelope(&AppError::terminated(), "get-item", 0);
        assert_eq!(e.error.code, 143);
        assert_eq!(e.command, "get-item");
        assert_eq!(e.duration_ms, 0);
        assert_eq!(e.error.kind, "canceled");
        let e = error_envelope(&AppError::interrupted(), "get-item", 1);
        assert_eq!(e.error.code, 130);
        assert_eq!(e.error.kind, "canceled");
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
        assert!(!v.as_object().expect("object").values().any(|x| x.is_null()));

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
        assert!(!r.as_object().expect("object").values().any(|x| x.is_null()));
    }

    #[test]
    fn budget_error_is_not_retryable() {
        let e = AppError::of(ErrorDetail::OutputOverBudget);
        let env = error_envelope(&e, "get-item", 12);
        assert_eq!(env.command, "get-item");
        assert_eq!(env.duration_ms, 12);
        assert_eq!(env.error.kind, "budget");
        assert_eq!(env.error.code, 74);
        assert!(!env.error.retryable);
    }

    #[test]
    fn suggestions_reach_the_wire_as_data_not_only_as_prose() {
        // `--suggest` computed a correct ranking and then buried it inside
        // `error.message`, so the only way to consume it was to split on
        // "; suggestions: ", split again on ", " and regex the "(kind)" off each
        // entry. The sentence still carries the list for humans; this locks the
        // machine-readable half beside it.
        let e = AppError::of(ErrorDetail::WithSuggestions {
            base: Box::new(ErrorDetail::AssocAnchorMissing {
                anchors: "method.unwrp".into(),
            }),
            suggestions: vec![
                Suggestion::new("option::Option::unwrap", "method"),
                Suggestion::new("option::Option::unzip", "method"),
            ],
        });
        let env = error_envelope(&e, "get-item", 3);
        let got = env.error.suggestions.as_deref().expect("suggestions field");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].path, "option::Option::unwrap");
        assert_eq!(got[0].kind, "method");
        assert!(env.error.message.contains("suggestions:"), "prose kept");
    }

    #[test]
    fn a_plain_failure_omits_the_suggestions_field_entirely() {
        // Absent, never `null` and never `[]`: an empty array would claim "we
        // looked and found nothing", which is a different fact from "no ranking
        // ran". `--suggest` on a leaf close to nothing still renders "(none)" in
        // the sentence, and that path is covered by the error-render tests.
        let e = AppError::of(ErrorDetail::OutputOverBudget);
        let env = error_envelope(&e, "get-item", 1);
        assert!(env.error.suggestions.is_none());
        let v = serde_json::to_value(&env).expect("serialize");
        assert!(v["error"].get("suggestions").is_none());
    }
}
