//! Typed catalogue of every product error, carrying data instead of prose.
//!
//! # Why a catalogue
//!
//! Localization used to work by matching the *English text* of a message with
//! `starts_with`. That coupling is invisible to the compiler: rewording an
//! English message silently dropped its pt-BR translation, and a brand-new
//! message fell through to a fallback that returned English with no signal at
//! all. Thirty-two match arms guarded well over a hundred construction sites.
//!
//! Here the data is the message. [`ErrorDetail`] carries the *values*; the two
//! renderers in [`super::render`] own the words. Because their `match` has no
//! `_` arm, a variant added without a translation is a **compile error** rather
//! than a silent English leak. The compiler is the coverage gate.
//!
//! # Shape, not sentence
//!
//! Variants describe the *shape* of a failure ("this subject is empty", "this
//! subject exceeds a limit") and name the subject through [`Subject`]. Eleven
//! "X is empty" messages therefore share one variant instead of eleven.
//!
//! # Upstream text
//!
//! Failures from `reqwest`, `std::io`, `serde` and `toml` carry text we did not
//! write. [`ErrorDetail::Io`] and [`ErrorDetail::Internal`] translate the
//! *operation* and leave any upstream detail on the `source` chain, untouched.
//! Inventing a translation for someone else's sentence would be worse than
//! leaving it be.

use super::suggestion::Suggestion;
use super::vocab::{
    AllowlistStage, ContentKind, DestructiveEffect, InternalOp, IoCause, IoOp, Subject, ValueSource,
};
use crate::error::ErrorKind;

/// Every failure the product can report, as data.
///
/// Construct errors through [`super::AppError::of`] so the [`ErrorKind`] and the
/// English wire message are both derived from one place.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ErrorDetail {
    // ── Generic domain shapes ────────────────────────────────────────────────
    /// `subject` was empty or blank.
    Empty {
        /// What was empty.
        subject: Subject,
    },
    /// `subject` was longer than `limit` characters.
    TooLong {
        /// What was too long.
        subject: Subject,
        /// Maximum accepted length in characters.
        limit: usize,
    },
    /// `subject` did not satisfy its format rule.
    Invalid {
        /// What was invalid.
        subject: Subject,
        /// The offending value, echoed back so the caller can see it.
        value: String,
    },
    /// `subject` contained control or invisible characters.
    ControlCharacters {
        /// What carried the characters.
        subject: Subject,
    },
    /// `subject` must be printable ASCII only.
    NotVisibleAscii {
        /// What must be ASCII.
        subject: Subject,
    },
    /// `subject` contained whitespace where none is allowed.
    ContainsWhitespace {
        /// What carried the whitespace.
        subject: Subject,
        /// The offending value.
        value: String,
    },
    /// A duration knob was set to zero.
    MustBeAtLeastOneSecond {
        /// Which duration.
        subject: Subject,
        /// Layer the value came from (decides `invalid_input` vs `config`).
        source: ValueSource,
    },
    /// A byte budget was raised above the product's hard ceiling.
    AboveHardMaximum {
        /// Which budget.
        subject: Subject,
        /// The ceiling that cannot be raised.
        hard_max: u64,
        /// Layer the value came from (decides `invalid_input` vs `config`).
        source: ValueSource,
    },

    // ── Path and reference shapes ────────────────────────────────────────────
    /// An item path yielded no usable segments.
    ItemPathNoSegments,
    /// A non-module item path ended without an item name.
    ItemPathMissingItemName,
    /// A crate reference carried more than one `@`.
    CrateRefMultipleAt,
    /// A crate reference had nothing before its `@`.
    CrateRefEmptyName,
    /// A crate reference had nothing after its `@`.
    CrateRefEmptyVersion,
    /// A version carried `+build` metadata, which docs.rs does not resolve.
    VersionBuildMetadata,
    /// A version was written with the `v` prefix.
    VersionVPrefix,

    // ── Enumerated argument values ───────────────────────────────────────────
    /// `--match` received a value outside the accepted set.
    UnknownMatchMode {
        /// What was passed.
        value: String,
    },
    /// A kind alias was not recognized.
    UnknownItemType {
        /// What was passed.
        value: String,
    },
    /// `--lang` received an unsupported BCP-47 tag.
    UnsupportedLang {
        /// What was passed.
        tag: String,
    },
    /// `schema --cmd` named a document that does not exist.
    UnknownSchemaCommand {
        /// What was passed.
        value: String,
    },
    /// `--json` was combined with a human output format.
    JsonFormatConflict,
    /// Argv failed `clap` parsing.
    ///
    /// The text is `clap`'s own rendering, not ours, so it is carried verbatim
    /// and not translated — inventing a pt-BR version of someone else's usage
    /// help would drift from what `--help` actually prints.
    ClapUsage {
        /// `clap`'s rendered message.
        message: String,
    },
    /// `--filter` could not be parsed.
    InvalidFilterExpression {
        /// The expression as written.
        expr: String,
        /// Why it was rejected.
        reason: String,
    },
    /// `search-in-crate --item-type module` has no catalogue to read.
    ModuleFilterUnsupported,
    /// A destructive verb resolved its target from ambient state, not from argv.
    ///
    /// This is the confused-deputy shape: the caller names the verb, the
    /// environment names the victim, and the two are never compared. Refusing
    /// here costs one flag; not refusing costs whichever cache root the process
    /// happened to resolve.
    AmbientTargetRefused {
        /// The invocation that was refused, in wire spelling.
        verb: String,
        /// The target the environment resolved, so the caller sees the victim.
        target: String,
        /// The flag that names the target in argv instead.
        target_flag: String,
        /// The flag that explicitly accepts the ambient target.
        waiver_flag: String,
        /// What would have happened to the target.
        ///
        /// Carried as data because the harm differs by verb: the first version
        /// of this message said "would delete" for every caller, which was
        /// simply false once a verb that overwrites started using it.
        effect: DestructiveEffect,
    },

    // ── Pagination ───────────────────────────────────────────────────────────
    /// `--page` was zero or absent where one is required.
    PageBelowOne,
    /// `--per-page` fell outside its accepted range.
    PerPageOutOfRange {
        /// Highest accepted value.
        max: u32,
        /// What was passed.
        got: u32,
    },

    // ── Origins and configuration ────────────────────────────────────────────
    /// An origin URL used a scheme other than http or https.
    OriginBadScheme {
        /// The scheme as written.
        scheme: String,
    },
    /// An origin URL carried no host component.
    OriginMissingHost {
        /// The URL as written.
        url: String,
    },
    /// An origin host was outside the SSRF allowlist.
    OriginNotAllowlisted {
        /// The rejected host.
        host: String,
        /// Comma-separated allowlist, so the operator sees the accepted set.
        allowed: String,
    },
    /// An item path segment used characters outside the accepted set.
    ItemPathSegmentCharset {
        /// The offending segment.
        segment: String,
    },
    /// `crate@version` and `--crate-version` disagreed.
    ConflictingVersions {
        /// Version taken from the `crate@version` sugar.
        from_ref: String,
        /// Version taken from the explicit flag.
        from_flag: String,
    },
    /// The XDG config directory could not be resolved.
    ConfigDirUnresolved,
    /// The XDG cache directory could not be resolved.
    CacheDirUnresolved,
    /// `config.toml` failed to parse as TOML.
    ConfigTomlInvalid,
    /// `config.toml` was not valid UTF-8.
    ConfigTomlNotUtf8,
    /// `config init` found an existing file and `--force` was absent.
    ConfigAlreadyExists {
        /// Absolute path that would have been overwritten.
        path: String,
    },
    /// `config.toml` was larger than the read cap.
    ConfigTomlTooLarge {
        /// The cap in bytes.
        max_bytes: u64,
    },
    /// The configured User-Agent was rejected as an HTTP header value.
    UserAgentHeaderInvalid,

    // ── HTTP transport and budgets ───────────────────────────────────────────
    /// A remote returned a status that maps to a domain failure.
    HttpStatus {
        /// The HTTP status code.
        status: u16,
        /// Short non-sensitive context label (endpoint name, never a body).
        hint: String,
    },
    /// A request or redirect targeted a host outside the allowlist.
    HostNotAllowlisted {
        /// The rejected host.
        host: String,
        /// Which stage rejected it, so the operator knows where to look.
        stage: AllowlistStage,
    },
    /// The transport failed before any status was seen.
    ///
    /// Carries its own [`ErrorKind`] because the classification comes from
    /// `reqwest` (timeout vs connect vs body), not from the shape of the call.
    HttpRequestFailed {
        /// The URL that was being fetched.
        url: String,
        /// Classification supplied by the transport layer.
        kind: ErrorKind,
    },
    /// The redirect chain went past `max_redirects`.
    RedirectLimitExceeded,
    /// Building the HTTP client failed.
    HttpClientBuild,
    /// Reading the response body failed mid-stream.
    BodyRead,
    /// The response body passed the configured byte budget.
    BodyOverBudget {
        /// The budget in bytes.
        max_bytes: u64,
    },
    /// A cached body no longer fits the current byte budget.
    CachedBodyOverBudget {
        /// The budget in bytes.
        max_bytes: u64,
    },
    /// Reserving body capacity failed (allocation refused).
    BodyReserveFailed {
        /// Bytes that could not be reserved.
        bytes: usize,
    },
    /// The response body was not valid UTF-8.
    BodyNotUtf8,
    /// The response advertised a content type the endpoint cannot use.
    UnexpectedContentType {
        /// Flavour the caller required.
        expected: ContentKind,
        /// What the remote advertised.
        got: String,
    },
    /// The emitted payload passed `max_output_bytes`.
    OutputOverBudget,

    // ── Docs.rs extraction ───────────────────────────────────────────────────
    /// HTML could not be converted to Markdown.
    HtmlToMarkdown,
    /// crates.io returned JSON the parser rejected.
    CratesIoJson,
    /// No parent page existed for an associated item.
    AssocParentPageNotFound,
    /// The parent page was found but carried no anchor for the member.
    ///
    /// The rendered text must keep the sentinel prefix, since both the live
    /// probe and the recovery path match on it to stop walking parent kinds.
    AssocAnchorMissing {
        /// Comma-separated anchor ids that were tried.
        anchors: String,
    },
    /// The member anchor existed but its docblock was empty.
    AssocAnchorEmpty {
        /// The anchor id that was found.
        anchor_id: String,
    },
    /// A member-only kind was requested without a parent to anchor against.
    MemberKindNeedsParent {
        /// The kind as the user typed it.
        kind: &'static str,
        /// The leaf they supplied, echoed into the suggested rewrite.
        member: String,
        /// Comma-separated parent kinds that can host this member.
        parent_kinds: String,
    },
    /// A hit href could not be joined against its origin.
    HitJoinFailed,
    /// A hit's `source_url` was not a usable join base.
    HitBaseInvalid,

    // ── Cancellation and deadlines ───────────────────────────────────────────
    /// SIGINT / Ctrl-C.
    Interrupted,
    /// SIGTERM.
    Terminated,
    /// stdout was closed by the reader.
    BrokenPipe,
    /// The wall-clock deadline elapsed.
    WallClockTimeout {
        /// Deadline in seconds.
        secs: u64,
    },

    // ── Infrastructure ───────────────────────────────────────────────────────
    /// A filesystem operation failed; the cause is on the source chain.
    ///
    /// Build this with [`Self::io`] rather than by hand: the constructor is what
    /// classifies the OS cause while the [`std::io::Error`] is still available,
    /// and a hand-written `cause` is a guess about a value the caller already had.
    Io {
        /// What was being attempted.
        op: IoOp,
        /// Path involved, when one is known and safe to show.
        path: Option<String>,
        /// Whether waiting could plausibly change the outcome.
        cause: IoCause,
    },
    /// An internal invariant or host facility failed.
    Internal {
        /// What was being attempted.
        op: InternalOp,
    },

    // ── Composition ──────────────────────────────────────────────────────────
    /// Another failure, with ranked `--suggest` alternatives appended.
    WithSuggestions {
        /// The failure being explained.
        base: Box<ErrorDetail>,
        /// Ranked alternatives, best first.
        suggestions: Vec<Suggestion>,
    },
}
