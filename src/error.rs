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
            Self::Internal => "internal",
            Self::Interrupted => "interrupted",
            Self::Terminated => "terminated",
            Self::BrokenPipe => "broken_pipe",
        }
    }

    /// Process exit code for this kind.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Usage => 64,
            Self::InvalidInput => 65,
            Self::NotFound => 66,
            Self::RateLimited | Self::Unavailable => 69,
            Self::Network | Self::Budget => 74,
            Self::Timeout => 124,
            Self::Parse => 65,
            Self::Config => 78,
            Self::Internal => 70,
            Self::Interrupted => 130,
            Self::Terminated => 143,
            Self::BrokenPipe => 141,
        }
    }

    /// Whether an agent may retry the same invocation after backoff.
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
        /// Technical English message (never localized).
        message: String,
        /// Optional Retry-After hint from HTTP 429 / 503.
        retry_after_secs: Option<u64>,
        /// Optional lower-level cause (shared via [`Arc`] for [`Clone`]).
        #[source]
        source: Option<Arc<dyn std::error::Error + Send + Sync>>,
    },
}

impl AppError {
    /// Build a structured error without a source chain.
    ///
    /// # Errors
    ///
    /// This constructor never fails; the returned value is the error itself.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::Structured {
            kind,
            message: message.into(),
            retry_after_secs: None,
            source: None,
        }
    }

    /// Build a structured error with a source chain (cause preserved).
    ///
    /// The cause is available via [`std::error::Error::source`] and is not
    /// duplicated in `message`.
    pub fn with_source(
        kind: ErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Structured {
            kind,
            message: message.into(),
            retry_after_secs: None,
            source: Some(Arc::new(source)),
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
        Self::new(ErrorKind::Interrupted, "interrupted by SIGINT")
    }

    /// SIGTERM cancel.
    pub fn terminated() -> Self {
        Self::new(ErrorKind::Terminated, "terminated by SIGTERM")
    }

    /// stdout closed by the reader (pipe consumer exited).
    pub fn broken_pipe() -> Self {
        Self::new(ErrorKind::BrokenPipe, "broken pipe writing to stdout")
    }

    /// Returns the canonical error kind.
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Structured { kind, .. } => *kind,
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
    pub fn retryable(&self) -> bool {
        self.kind().retryable()
    }

    /// Alias of [`Self::retryable`] (Rules Rust checklist name `is_retryable`).
    pub fn is_retryable(&self) -> bool {
        self.retryable()
    }

    /// Permanent for agent auto-retry (complement of [`Self::is_retryable`]).
    pub fn is_permanent(&self) -> bool {
        self.kind().is_permanent()
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
        match status {
            400 => Self::new(
                ErrorKind::InvalidInput,
                format!("bad request from remote: {body_hint}"),
            ),
            404 => Self::new(
                ErrorKind::NotFound,
                format!("resource not found: {body_hint}"),
            ),
            429 => Self::new(
                ErrorKind::RateLimited,
                format!("rate limited by remote: {body_hint}"),
            ),
            500 | 502 | 503 | 504 => Self::new(
                ErrorKind::Unavailable,
                format!("remote unavailable (HTTP {status}): {body_hint}"),
            ),
            other => Self::new(
                ErrorKind::Network,
                format!("unexpected HTTP status {other}: {body_hint}"),
            ),
        }
    }
}

/// Result alias for library operations.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_canonical() {
        assert_eq!(ErrorKind::Usage.exit_code(), 64);
        assert_eq!(ErrorKind::InvalidInput.exit_code(), 65);
        assert_eq!(ErrorKind::NotFound.exit_code(), 66);
        assert_eq!(ErrorKind::RateLimited.exit_code(), 69);
        assert_eq!(ErrorKind::Network.exit_code(), 74);
        assert_eq!(ErrorKind::Budget.exit_code(), 74);
        assert_eq!(ErrorKind::Budget.as_str(), "budget");
        assert!(!ErrorKind::Budget.retryable());
        assert!(ErrorKind::Budget.is_permanent());
        assert_eq!(ErrorKind::Timeout.exit_code(), 124);
        assert_eq!(ErrorKind::Interrupted.exit_code(), 130);
        assert_eq!(ErrorKind::Terminated.exit_code(), 143);
        assert_eq!(ErrorKind::BrokenPipe.exit_code(), 141);
        assert_eq!(ErrorKind::Config.exit_code(), 78);
        assert_eq!(ErrorKind::Internal.exit_code(), 70);
        assert_eq!(ErrorKind::BrokenPipe.as_str(), "broken_pipe");
        assert!(!ErrorKind::BrokenPipe.retryable());
    }

    #[test]
    fn cancel_helpers() {
        assert_eq!(AppError::interrupted().kind(), ErrorKind::Interrupted);
        assert_eq!(AppError::terminated().kind(), ErrorKind::Terminated);
        assert_eq!(AppError::broken_pipe().kind(), ErrorKind::BrokenPipe);
        assert_eq!(AppError::broken_pipe().exit_code(), ExitCode::from(141));
    }

    #[test]
    fn from_http_status_matrix() {
        assert_eq!(
            AppError::from_http_status(400, "x").kind(),
            ErrorKind::InvalidInput
        );
        assert_eq!(
            AppError::from_http_status(404, "x").kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            AppError::from_http_status(429, "x").kind(),
            ErrorKind::RateLimited
        );
        assert_eq!(
            AppError::from_http_status(503, "x").kind(),
            ErrorKind::Unavailable
        );
        assert_eq!(
            AppError::from_http_status(418, "x").kind(),
            ErrorKind::Network
        );
        let e = AppError::from_http_status(429, "slow").with_retry_after(7);
        assert_eq!(e.retry_after_secs(), Some(7));
        assert!(e.kind().retryable());
        assert!(e.kind().is_retryable());
        assert!(!ErrorKind::NotFound.retryable());
        assert!(ErrorKind::NotFound.is_permanent());
        assert!(ErrorKind::Interrupted.is_permanent());
        assert_eq!(ErrorKind::Parse.as_str(), "parse");
        assert_eq!(ErrorKind::Config.exit_code(), 78);
    }

    #[test]
    fn with_source_preserves_kind() {
        let e = AppError::with_source(ErrorKind::Parse, "bad", std::io::Error::other("inner"));
        assert_eq!(e.kind(), ErrorKind::Parse);
        assert_eq!(e.message(), "bad");
    }

    #[test]
    fn source_chain_via_std_error() {
        use std::error::Error as _;
        let e = AppError::with_source(ErrorKind::Parse, "bad", std::io::Error::other("inner"));
        let src = e.source().expect("source present");
        assert!(src.to_string().contains("inner"));
        // Display is message only — cause is not duplicated.
        assert_eq!(e.to_string(), "bad");
        assert!(!e.to_string().contains("inner"));
    }

    #[test]
    fn app_error_clone_shares_source() {
        use std::error::Error as _;
        let e = AppError::with_source(
            ErrorKind::Network,
            "req failed",
            std::io::Error::other("eof"),
        );
        let c = e.clone();
        assert_eq!(c.kind(), ErrorKind::Network);
        assert_eq!(c.message(), "req failed");
        assert!(c.source().is_some());
        assert!(c.is_retryable());
        assert!(!c.is_permanent());
        assert!(AppError::new(ErrorKind::NotFound, "missing").is_permanent());
    }

    #[test]
    fn display_message_style() {
        let e = AppError::new(ErrorKind::Config, "invalid config.toml");
        let s = e.to_string();
        assert_eq!(s, "invalid config.toml");
        assert!(!s.ends_with('.'));
        assert_eq!(s, s.to_ascii_lowercase());
    }
}
