//! Vocabulary the error catalogue is written in.
//!
//! These enums name *what* an error is about ([`Subject`]), *which* operation
//! failed ([`IoOp`], [`InternalOp`]), and *where* a value came from
//! ([`ValueSource`], [`AllowlistStage`]). They change rarely; the catalogue in
//! [`super::detail`] grows with every new failure, so the two live apart.

/// The thing an error is about, when the failure shape is generic.
///
/// Separating subject from shape is what collapses "crate name is empty",
/// "search query is empty" and "item path is empty" into one variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    /// A crate name (`serde`).
    CrateName,
    /// A crate reference (`serde@1.0.0`).
    CrateRef,
    /// A search query string.
    SearchQuery,
    /// A rustdoc item path (`runtime::Runtime::new`).
    ItemPath,
    /// One `::`-separated segment of an item path.
    ItemPathSegment,
    /// A semver version argument.
    Version,
    /// The `--page-token` opaque pagination cursor.
    PageToken,
    /// The HTTP `User-Agent` header value.
    UserAgent,
    /// The contact URL or address embedded in the User-Agent.
    Contact,
    /// An allowlisted origin URL.
    Origin,
    /// The wall-clock request timeout.
    Timeout,
    /// The TCP connect timeout.
    ConnectTimeout,
    /// The downloaded-body byte budget.
    MaxBodyBytes,
    /// The emitted-stdout byte budget.
    MaxOutputBytes,
    /// The optional XDG `config.toml`.
    ConfigFile,
    /// The `log_directive` tracing filter from `config.toml`.
    LogDirective,
}

/// Grammatical gender of a [`Subject`] noun in Portuguese.
///
/// Adjectives agree with their noun in pt-BR, so "consulta … vazia" but "nome …
/// vazio". Without this the renderer produced "consulta de busca está vazio",
/// which reads as broken Portuguese — the kind of defect a translation table
/// keyed on English prose can never surface, because English has no agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenderPtBr {
    /// Masculine noun (`nome`, `caminho`, `contato`).
    Masculine,
    /// Feminine noun (`consulta`, `versão`, `referência`).
    Feminine,
}

impl Subject {
    /// Gender of this subject's Portuguese noun, for adjective agreement.
    pub(crate) fn gender_pt_br(self) -> GenderPtBr {
        match self {
            // "referência", "consulta", "versão", "URL"
            Self::CrateRef | Self::SearchQuery | Self::Version | Self::Origin => {
                GenderPtBr::Feminine
            }
            // "nome", "caminho", "segmento", "contato", plus knob identifiers
            // that stay verbatim and read as masculine tokens.
            Self::CrateName
            | Self::ItemPath
            | Self::ItemPathSegment
            | Self::PageToken
            | Self::UserAgent
            | Self::Contact
            | Self::Timeout
            | Self::ConnectTimeout
            | Self::MaxBodyBytes
            | Self::MaxOutputBytes
            | Self::ConfigFile
            | Self::LogDirective => GenderPtBr::Masculine,
        }
    }

    /// Whether this subject is operator configuration rather than command input.
    ///
    /// Drives the [`ErrorKind`] split: a bad crate name is `invalid_input`
    /// (exit 65) because the caller typed it this run, while a bad origin is
    /// `config` (exit 78) because it lives in `config.toml`. Same failure
    /// *shape*, different remedy — and agents branch on the exit code.
    pub(super) fn is_operator_config(self) -> bool {
        matches!(
            self,
            Self::Origin | Self::UserAgent | Self::Contact | Self::ConfigFile | Self::LogDirective
        )
    }
}

/// Which layer supplied a value that failed validation.
///
/// The same limit is reachable from a flag and from `config.toml`, and the two
/// carry different exit codes (65 vs 78) because they need different fixes.
/// Recording the layer keeps one variant serving both without guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// Passed as a CLI flag in this invocation.
    CommandLine,
    /// Read from the XDG `config.toml`.
    ConfigFile,
}

/// A filesystem or process operation that failed, for [`crate::error::ErrorDetail::Io`].
///
/// Named rather than free text so both languages describe the same act and the
/// underlying `std::io::Error` stays on the source chain where it belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOp {
    /// Reading a file.
    Read,
    /// Writing a file.
    Write,
    /// Creating a directory.
    CreateDir,
    /// Creating a temporary file.
    CreateTemp,
    /// Renaming a path into place.
    Rename,
    /// Flushing data to durable storage.
    Sync,
    /// Installing a generated file at its destination.
    Install,
    /// Opening a lock file.
    OpenLock,
    /// Taking an exclusive lock.
    Lock,
    /// Listing a directory.
    ReadDir,
    /// Removing an entry.
    Remove,
}

/// An internal invariant or infrastructure failure, for [`crate::error::ErrorDetail::Internal`].
///
/// These are bugs or host problems, never user input. They are still localized:
/// the operator reading stderr deserves their own language even when the cause
/// on the source chain is upstream English.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternalOp {
    /// Serializing a value to JSON.
    JsonSerialize,
    /// Pretty-printing JSON.
    JsonPrettyPrint,
    /// Writing to stdout.
    StdoutWrite,
    /// An embedded schema document is not valid JSON.
    EmbeddedSchemaInvalid,
    /// A built URL was rejected by the parser.
    UrlBuild,
    /// The concurrency semaphore was closed.
    SemaphoreClosed,
    /// A CPU worker task failed to join.
    WorkerJoin,
    /// A CPU worker panicked mid-parse.
    WorkerPanic,
    /// The system clock is before the UNIX epoch.
    ClockBeforeEpoch,
    /// A cache key did not satisfy its own format invariant.
    CacheKeyMalformed,
    /// An associated-item path reached the builder with too few segments.
    AssocPathTooShort,
    /// A probe table named a member kind as a parent page host.
    AssocParentOwnsNoPage,
    /// An already-validated origin failed to re-parse.
    AllowedOriginUnparseable,
    /// Injected failure used by tests to exercise the CPU-bound error path.
    SyntheticParseFailure,
}

/// Where an allowlist rejection happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowlistStage {
    /// The URL the caller asked for.
    Request,
    /// A hop inside the redirect chain.
    Redirect,
    /// The URL the chain finally landed on.
    FinalUrl,
}

/// Which payload flavour a response was expected to carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentKind {
    /// `text/html`
    Html,
    /// `application/json`
    Json,
}

/// What a destructive verb would have done to the target it was refused.
///
/// The refusal message has to name the harm, and "would delete" is false for a
/// verb that overwrites. Carrying the effect as an enum rather than a phrase
/// keeps both locales honest: a third kind of harm cannot ship until the
/// exhaustive match in each renderer is taught the word for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DestructiveEffect {
    /// The target and its contents would be removed (`cache clear`).
    Delete,
    /// The target would be replaced in place (`config init --force`).
    Overwrite,
}

/// Why an I/O failure is or is not worth another attempt.
///
/// Retryability is a property of the operating-system cause, never of the call
/// site: `ENOSPC` on the cache directory clears when the operator frees space,
/// and `EACCES` on the same path does not. Classifying once, where the
/// [`std::io::Error`] is still in hand, is what stops the envelope from
/// promising an agent a retry that can only ever fail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoCause {
    /// The condition can clear without the caller changing anything.
    Transient,
    /// The condition needs operator action; retrying repeats the failure.
    Permanent,
}

impl IoCause {
    /// Classify an OS error by whether waiting could plausibly help.
    ///
    /// Unknown kinds classify as [`Self::Permanent`]. An agent that stops early
    /// on a transient fault wastes one run; an agent that retries a permanent
    /// one spins until its budget is gone, so the safe default is to stop.
    pub fn of(err: &std::io::Error) -> Self {
        use std::io::ErrorKind as K;
        match err.kind() {
            K::StorageFull
            | K::Interrupted
            | K::WouldBlock
            | K::TimedOut
            | K::ResourceBusy
            | K::QuotaExceeded => Self::Transient,
            _ => Self::Permanent,
        }
    }

    /// True when another attempt is worth making.
    pub fn retryable(self) -> bool {
        matches!(self, Self::Transient)
    }
}
