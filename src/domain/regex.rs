//! Bounded regex compilation (Rules Rust — ReDoS / size_limit).

use regex::{Regex, RegexBuilder};

/// Max compiled regex heap for product patterns (NFA/bytecode budget).
///
/// Product patterns are fixed or escape-bounded (`regex::escape` + crate-name
/// charset). 256 KiB is far above those patterns and far below default 10 MiB
/// so a future dynamic pattern cannot balloon compile cost unnoticed.
pub(crate) const REGEX_SIZE_LIMIT_BYTES: usize = 256 * 1024;

/// Max DFA cache for product patterns (mirror of [`REGEX_SIZE_LIMIT_BYTES`]).
pub(crate) const REGEX_DFA_SIZE_LIMIT_BYTES: usize = 256 * 1024;

/// Compile a regex with explicit `size_limit` / `dfa_size_limit` (ReDoS posture).
///
/// Prefer this over bare [`Regex::new`] everywhere in product code.
///
/// # Errors
///
/// Returns the underlying [`regex::Error`] when the pattern is invalid or
/// exceeds the size budgets.
pub(crate) fn compile_bounded_regex(pattern: &str) -> Result<Regex, regex::Error> {
    RegexBuilder::new(pattern)
        .size_limit(REGEX_SIZE_LIMIT_BYTES)
        .dfa_size_limit(REGEX_DFA_SIZE_LIMIT_BYTES)
        .build()
}

/// True when `c` is a C0/C1 control **or** a format/invisible character that
/// must not appear in free-text CLI input (bidi overrides, zero-width, BOM).
///
/// Domain identifiers (crate name, item path, version) already restrict to
/// ASCII alphanumerics; this gate is for [`super::SearchQuery`] free text.
pub(crate) fn is_hostile_text_char(c: char) -> bool {
    if c.is_control() {
        return true;
    }
    matches!(
        c,
        // Zero-width / word joiner / BOM
        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
        // Bidi embeddings / overrides / isolates (U+202A..=U+202E, U+2066..=U+2069)
        | '\u{202A}'..='\u{202E}'
        | '\u{2066}'..='\u{2069}'
    )
}
