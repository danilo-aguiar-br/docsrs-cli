//! Error types and exit-code mapping for docsrs-cli.
//!
//! # Model (Rules Rust — error handling)
//!
//! - Recoverable failures are [`AppError`] / [`AppResult`] (I/O, network, parse, validation).
//! - [`ErrorKind`] is the stable discriminant for exit codes and JSON `error.kind`.
//! - [`std::error::Error::source`] preserves the lower-level cause when present.
//! - Display messages are technical English: lowercase, no trailing period, no secrets.
//! - Panic is reserved for static invariants (hardcoded regex / CSS selectors).
//! - Library API: typed `thiserror` enum, not `anyhow` / `Box<dyn Error>` as the public `E`.
//!
//! See `docs/decisions/0002-error-model.md`.
//!
//! # Errors
//!
//! | Kind | Exit | Retryable |
//! |------|------|-----------|
//! | usage | 64 | no |
//! | invalid_input | 65 | no |
//! | not_found | 66 | no |
//! | rate_limited / unavailable | 69 | yes |
//! | network | 74 | yes |
//! | budget | 74 | no |
//! | timeout | 124 | yes |
//! | parse | 65 | no |
//! | config | 78 | no |
//! | internal | 70 | no |
//! | interrupted (SIGINT) | 130 | no |
//! | terminated (SIGTERM) | 143 | no |
//! | broken_pipe (stdout EPIPE) | 141 | no |

use std::process::ExitCode;
use std::sync::Arc;

use thiserror::Error;

// ── Named process exit codes (sysexits-inspired + signal/convention) ─────────
// Every `ExitCode::from(N)` / `process::exit(N)` in product code must use these
// (or [`crate::error::ErrorKind::exit_code`]) — never raw numeric literals for exit status.

/// Clap / argv usage error (sysexits `EX_USAGE`).
pub const EXIT_USAGE: u8 = 64;
/// Domain validation failure (invalid crate name, path, pagination).
pub const EXIT_INVALID_INPUT: u8 = 65;
/// Remote resource missing (HTTP 404).
pub const EXIT_NOT_FOUND: u8 = 66;
/// Rate limit or temporary remote unavailability (HTTP 429/5xx).
pub const EXIT_UNAVAILABLE: u8 = 69;
/// Unexpected internal failure.
pub const EXIT_INTERNAL: u8 = 70;
/// Transport / network failure or local body budget.
pub const EXIT_NETWORK: u8 = 74;
/// Local filesystem failure (sysexits `EX_IOERR`, the original meaning of 74).
///
/// Three kinds share this code and the contract has always said so: 74 is
/// ambiguous until `error.kind` is read. `network` is retryable, `budget` never
/// is, and `io` depends on the OS cause. Giving filesystem failure its own code
/// would buy nothing an agent cannot already read, and would add a number to a
/// contract that promises stable exit codes.
pub const EXIT_IO: u8 = EXIT_NETWORK;
/// Local configuration error (XDG TOML, origins).
pub const EXIT_CONFIG: u8 = 78;
/// Wall-clock deadline exceeded.
pub const EXIT_TIMEOUT: u8 = 124;
/// SIGINT / Ctrl-C cooperative cancel.
pub const EXIT_INTERRUPTED: u8 = 130;
/// stdout EPIPE / broken pipe.
pub const EXIT_BROKEN_PIPE: u8 = 141;
/// SIGTERM / SIGHUP terminate.
pub const EXIT_TERMINATED: u8 = 143;

/// Map a filesystem failure onto the catalogue, keeping the path that failed.
///
/// Sixteen call sites across the cache and the rate limiter used to build
/// [`ErrorDetail::Io`] inline with `path: None`, every one of them holding the
/// path it had just tried to open. An operator whose disk filled learned which
/// *operation* failed and never which *file*, which is the one fact needed to
/// act. Routing them through one closure fixes that and puts OS-cause
/// classification in a single place at the same time.
///
/// ```no_run
/// # use std::path::Path;
/// # use docsrs_cli::error::{io_at, IoOp};
/// std::fs::create_dir_all("/tmp/x").map_err(io_at(IoOp::CreateDir, Path::new("/tmp/x")))?;
/// # Ok::<(), docsrs_cli::error::AppError>(())
/// ```
pub fn io_at(op: IoOp, path: &std::path::Path) -> impl FnOnce(std::io::Error) -> AppError {
    let path = path.display().to_string();
    move |e| {
        let detail = ErrorDetail::io(op, Some(path), &e);
        AppError::of_with_source(detail, e)
    }
}

/// Canonical error kinds exposed in JSON envelopes.
///
/// Marked `#[non_exhaustive]` so new kinds can ship without a SemVer major break
/// for external `match` consumers of this library.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ErrorKind {
    /// Clap usage / argv error (exit 64).
    Usage,
    /// Invalid domain input such as crate name or path (exit 65).
    InvalidInput,
    /// Remote resource missing (exit 66).
    NotFound,
    /// HTTP 429 rate limit (exit 69, retryable).
    RateLimited,
    /// Remote 5xx or temporary outage (exit 69, retryable).
    Unavailable,
    /// Wall-clock deadline exceeded (exit 124, retryable).
    Timeout,
    /// Transport / unexpected HTTP failure (exit 74, retryable).
    Network,
    /// Local body/output budget exceeded (exit 74, **not** retryable).
    ///
    /// Permanent for the same config: raising `--max-body-bytes` (or lowering
    /// the remote payload) is required — agents must not auto-retry.
    Budget,
    /// Body parse failure (exit 65).
    Parse,
    /// Local configuration error (exit 78).
    Config,
    /// Local filesystem failure caused by the environment (exit 74).
    ///
    /// A full disk, a read-only mount or a directory the process cannot enter
    /// are facts about the machine, not defects in this binary. They used to
    /// arrive as [`Self::Internal`], which tells an agent to report a bug in a
    /// CLI that behaved correctly, and as `retryable: false`, which is wrong the
    /// moment the operator frees a block. Retryability here comes from the OS
    /// cause carried in the detail, so read [`AppError::retryable`] rather than
    /// [`Self::retryable`], which can only answer for the kind.
    Io,
    /// Unexpected internal failure (exit 70).
    Internal,
    /// SIGINT / Ctrl-C
    Interrupted,
    /// SIGTERM graceful cancel
    Terminated,
    /// stdout write hit a closed pipe (EPIPE / SIGPIPE)
    BrokenPipe,
}

impl ErrorKind {
    /// Stable snake_case identifier for JSON `error.kind`.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Usage => "usage",
            Self::InvalidInput => "invalid_input",
            Self::NotFound => "not_found",
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::Timeout => "timeout",
            Self::Network => "network",
            Self::Budget => "budget",
            Self::Parse => "parse",
            Self::Config => "config",
            Self::Io => "io",
            Self::Internal => "internal",
            Self::Interrupted => "interrupted",
            Self::Terminated => "terminated",
            Self::BrokenPipe => "broken_pipe",
        }
    }

    /// Process exit code for this kind.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Usage => EXIT_USAGE,
            Self::InvalidInput => EXIT_INVALID_INPUT,
            Self::NotFound => EXIT_NOT_FOUND,
            Self::RateLimited | Self::Unavailable => EXIT_UNAVAILABLE,
            Self::Network | Self::Budget => EXIT_NETWORK,
            Self::Timeout => EXIT_TIMEOUT,
            Self::Parse => EXIT_INVALID_INPUT,
            Self::Config => EXIT_CONFIG,
            Self::Io => EXIT_IO,
            Self::Internal => EXIT_INTERNAL,
            Self::Interrupted => EXIT_INTERRUPTED,
            Self::Terminated => EXIT_TERMINATED,
            Self::BrokenPipe => EXIT_BROKEN_PIPE,
        }
    }

    /// Whether an agent may retry the same invocation after backoff.
    ///
    /// [`Self::Io`] answers `false` here and means "unknown, ask the detail":
    /// the kind alone cannot tell a full disk from a permission denial. The wire
    /// field comes from [`AppError::retryable`], which reads the cause, so this
    /// conservative answer never reaches an envelope.
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Unavailable | Self::Timeout | Self::Network
        )
    }

    /// Alias of [`Self::retryable`] (Rules Rust checklist name `is_retryable`).
    pub fn is_retryable(self) -> bool {
        self.retryable()
    }

    /// Permanent for agent auto-retry (complement of [`Self::is_retryable`]).
    ///
    /// Cancel / broken-pipe kinds are permanent: do not re-issue the same command
    /// as if the remote failed.
    pub fn is_permanent(self) -> bool {
        !self.retryable()
    }

    /// Detailed retry category (Rules Rust checklist `retry_kind`).
    pub fn retry_kind(self) -> crate::retry::RetryKind {
        use crate::retry::RetryKind;
        match self {
            Self::RateLimited => RetryKind::RateLimited,
            Self::Unavailable => RetryKind::TransientServer,
            Self::Timeout => RetryKind::Timeout,
            Self::Network => RetryKind::TransientNetwork,
            _ => RetryKind::Permanent,
        }
    }
}

/// Application error with structured kind for agents.
///
/// # Display
///
/// Prints only the technical English `message` field (lowercase, no trailing
/// period). Walk [`std::error::Error::source`] for the cause chain; do not
/// duplicate the cause text in `message`.
///
/// # Clone
///
/// Source is held in [`Arc`] so clones share the cause without re-boxing.
#[derive(Debug, Error, Clone)]
#[non_exhaustive]
pub enum AppError {
    /// Structured error used by all library and CLI paths.
    #[error("{message}")]
    Structured {
        /// Canonical error kind for exit codes and JSON.
        kind: ErrorKind,
        /// Technical English message (never localized) — the JSON wire field.
        message: String,
        /// Typed catalogue entry this error was built from.
        ///
        /// Not optional: the string constructors were removed, so every error
        /// carries its data and therefore both renderings. Boxed to keep
        /// `AppError` small, since most values are moved around as `Result`
        /// payloads rather than inspected.
        detail: Box<ErrorDetail>,
        /// Optional Retry-After hint from HTTP 429 / 503.
        retry_after_secs: Option<u64>,
        /// Optional lower-level cause (shared via [`Arc`] for [`Clone`]).
        #[source]
        source: Option<Arc<dyn std::error::Error + Send + Sync>>,
    },
}

impl AppError {
    /// Build an error from the typed catalogue.
    ///
    /// This is the constructor product code should use. Both the [`ErrorKind`]
    /// and the English wire message are derived from `detail`, so one failure
    /// can never be reported as two different kinds from two call sites, and it
    /// can never ship without a pt-BR rendering — the renderer's `match` has no
    /// `_` arm, so a missing translation fails to compile.
    pub fn of(detail: ErrorDetail) -> Self {
        Self::Structured {
            kind: detail.kind(),
            message: render::render_en(&detail),
            detail: Box::new(detail),
            retry_after_secs: None,
            source: None,
        }
    }

    /// [`Self::of`] with a preserved lower-level cause.
    pub fn of_with_source(
        detail: ErrorDetail,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Structured {
            kind: detail.kind(),
            message: render::render_en(&detail),
            detail: Box::new(detail),
            retry_after_secs: None,
            source: Some(Arc::new(source)),
        }
    }

    /// Render this error in `locale`, for human stderr.
    ///
    /// Always available in both languages: the catalogue is the only way to
    /// build an `AppError`, and its pt-BR renderer is exhaustive.
    pub fn localized(&self, locale: crate::i18n::Locale) -> String {
        match self {
            Self::Structured {
                detail, message, ..
            } => match locale {
                crate::i18n::Locale::PtBr => render::render_pt_br(detail),
                crate::i18n::Locale::En => message.clone(),
            },
        }
    }

    /// Attach optional Retry-After seconds (HTTP 429 / 503).
    pub fn with_retry_after(mut self, secs: u64) -> Self {
        let Self::Structured {
            retry_after_secs, ..
        } = &mut self;
        *retry_after_secs = Some(secs);
        self
    }

    /// SIGINT cancel.
    pub fn interrupted() -> Self {
        Self::of(ErrorDetail::Interrupted)
    }

    /// SIGTERM cancel.
    pub fn terminated() -> Self {
        Self::of(ErrorDetail::Terminated)
    }

    /// stdout closed by the reader (pipe consumer exited).
    pub fn broken_pipe() -> Self {
        Self::of(ErrorDetail::BrokenPipe)
    }

    /// Returns the canonical error kind.
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Structured { kind, .. } => *kind,
        }
    }

    /// Typed catalogue entry this error carries.
    ///
    /// Lets a caller wrap one failure inside another (suggestions, context)
    /// without flattening it to a string and losing the pt-BR rendering.
    pub fn detail(&self) -> &ErrorDetail {
        match self {
            Self::Structured { detail, .. } => detail,
        }
    }

    /// Returns the technical English message.
    pub fn message(&self) -> &str {
        match self {
            Self::Structured { message, .. } => message,
        }
    }

    /// Returns optional Retry-After seconds when present.
    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::Structured {
                retry_after_secs, ..
            } => *retry_after_secs,
        }
    }

    /// Whether an agent may retry the same invocation after backoff.
    ///
    /// This is the value that reaches `error.retryable` on the wire. For
    /// filesystem failures it comes from the OS cause captured where the
    /// [`std::io::Error`] was still in hand, because `ENOSPC` and `EACCES` are
    /// the same [`ErrorKind`] and opposite answers.
    pub fn retryable(&self) -> bool {
        self.detail().retryable()
    }

    /// Ranked `--suggest` alternatives carried by this failure, if any.
    ///
    /// Only the outermost detail is inspected: the recovery ladder wraps a
    /// not-found in exactly one [`ErrorDetail::WithSuggestions`], and a nested
    /// one would mean two competing lists with no rule for which wins.
    pub fn suggestions(&self) -> Option<&[Suggestion]> {
        match self.detail() {
            ErrorDetail::WithSuggestions { suggestions, .. } => Some(suggestions),
            _ => None,
        }
    }

    /// Alias of [`Self::retryable`] (Rules Rust checklist name `is_retryable`).
    pub fn is_retryable(&self) -> bool {
        self.retryable()
    }

    /// Permanent for agent auto-retry (complement of [`Self::is_retryable`]).
    pub fn is_permanent(&self) -> bool {
        !self.retryable()
    }

    /// Detailed retry category (Rules Rust checklist `retry_kind`).
    pub fn retry_kind(&self) -> crate::retry::RetryKind {
        self.kind().retry_kind()
    }

    /// Maps this error to a process [`ExitCode`].
    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.kind().exit_code())
    }

    /// Map remote HTTP status to a domain error.
    ///
    /// `body_hint` must be a short, non-sensitive context label (e.g. endpoint
    /// name or `"retries exhausted"`), never a raw response body or credential.
    pub fn from_http_status(status: u16, body_hint: &str) -> Self {
        // Status-to-kind mapping lives in `ErrorDetail::kind`, so the wire text
        // and the exit code can never disagree about what a 429 means.
        Self::of(ErrorDetail::HttpStatus {
            status,
            hint: body_hint.to_string(),
        })
    }
}

/// Result alias for library operations.
pub type AppResult<T> = Result<T, AppError>;

mod classify;
mod detail;
mod render;
mod suggestion;
mod vocab;

#[cfg(test)]
mod tests;

pub use detail::ErrorDetail;
pub use suggestion::Suggestion;
pub use vocab::{
    AllowlistStage, ContentKind, DestructiveEffect, InternalOp, IoCause, IoOp, Subject, ValueSource,
};
