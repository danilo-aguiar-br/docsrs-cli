//! The `--suggest` alternative, as data rather than as a sentence.
//!
//! Split out of [`super::detail`] because the type has a life of its own: it is
//! produced by the ranker in [`crate::suggest`], carried by
//! [`super::ErrorDetail::WithSuggestions`], published by
//! [`crate::render::error_envelope`] and printed by both renderers. Four callers
//! across three layers is not a detail of the error catalogue.

/// One ranked `--suggest` alternative.
///
/// The ranker used to hand back `Vec<String>` already formatted as
/// `"option::Option::unwrap (method)"`, and the only place those strings ever
/// landed was inside `error.message`. An agent that wanted the alternatives had
/// to split on `"; suggestions: "`, split again on `", "` and then regex the
/// trailing `"(kind)"` off each entry — parsing prose the CLI had just built out
/// of data it already held. Keeping the pair typed lets
/// [`crate::render::error_envelope`] publish `error.suggestions` while the two
/// renderers keep composing the same sentence for humans, from one source.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Suggestion {
    /// Item path to retype, without the crate prefix (`option::Option::unwrap`).
    pub path: String,
    /// CLI kind the caller would pass alongside `path` (`method`, `variant`, …).
    ///
    /// This is the kind the *caller* would retype, not the rustdoc anchor family:
    /// labelling an associated type `(method)` would send them straight back into
    /// the same not-found.
    pub kind: String,
}

impl Suggestion {
    /// Build one suggestion from its two parts.
    pub fn new(path: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            kind: kind.into(),
        }
    }
}

/// Render a suggestion list the way both languages print it.
///
/// Shared so the English and pt-BR renderers cannot drift into two orderings or
/// two separators; only the lead-in word differs between them.
pub(super) fn join_suggestions(items: &[Suggestion]) -> String {
    if items.is_empty() {
        return "(none)".to_string();
    }
    items
        .iter()
        .map(|s| format!("{} ({})", s.path, s.kind))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_list_reads_as_none_in_the_sentence() {
        // `--suggest` that ranked nothing still says so out loud; the wire field
        // is omitted instead, and that asymmetry is deliberate.
        assert_eq!(join_suggestions(&[]), "(none)");
    }

    #[test]
    fn entries_keep_rank_order_and_the_kind_label() {
        let out = join_suggestions(&[
            Suggestion::new("option::Option::unwrap", "method"),
            Suggestion::new("option::Option::unzip", "method"),
        ]);
        assert_eq!(
            out,
            "option::Option::unwrap (method), option::Option::unzip (method)"
        );
    }
}
