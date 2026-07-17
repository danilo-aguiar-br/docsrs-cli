//! Error types and exit-code mapping for docsrs-cli.
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
//! | timeout | 124 | yes |
//! | parse | 65 | no |
//! | config | 78 | no |
//! | internal | 70 | no |
//! | interrupted (SIGINT) | 130 | no |
//! | terminated (SIGTERM) | 143 | no |

use std::process::ExitCode;

use thiserror::Error;

/// Canonical error kinds exposed in JSON envelopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Usage,
    InvalidInput,
    NotFound,
    RateLimited,
    Unavailable,
    Timeout,
    Network,
    Parse,
    Config,
    Internal,
    /// SIGINT / Ctrl-C
    Interrupted,
    /// SIGTERM graceful cancel
    Terminated,
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
            Self::Parse => "parse",
            Self::Config => "config",
            Self::Internal => "internal",
            Self::Interrupted => "interrupted",
            Self::Terminated => "terminated",
        }
    }

    /// Process exit code for this kind.
    pub fn exit_code(self) -> u8 {
        match self {
            Self::Usage => 64,
            Self::InvalidInput => 65,
            Self::NotFound => 66,
            Self::RateLimited | Self::Unavailable => 69,
            Self::Network => 74,
            Self::Timeout => 124,
            Self::Parse => 65,
            Self::Config => 78,
            Self::Internal => 70,
            Self::Interrupted => 130,
            Self::Terminated => 143,
        }
    }

    /// Whether an agent may retry the same invocation after backoff.
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Unavailable | Self::Timeout | Self::Network
        )
    }
}

/// Application error with structured kind for agents.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{message}")]
    Structured {
        kind: ErrorKind,
        message: String,
        retry_after_secs: Option<u64>,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl AppError {
    /// Build a structured error without a source chain.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self::Structured {
            kind,
            message: message.into(),
            retry_after_secs: None,
            source: None,
        }
    }

    /// Build a structured error with a source chain (cause preserved).
    pub fn with_source(
        kind: ErrorKind,
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        Self::Structured {
            kind,
            message: message.into(),
            retry_after_secs: None,
            source: Some(Box::new(source)),
        }
    }

    /// Attach optional Retry-After seconds (HTTP 429).
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

    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Structured { kind, .. } => *kind,
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Structured { message, .. } => message,
        }
    }

    pub fn retry_after_secs(&self) -> Option<u64> {
        match self {
            Self::Structured {
                retry_after_secs, ..
            } => *retry_after_secs,
        }
    }

    pub fn exit_code(&self) -> ExitCode {
        ExitCode::from(self.kind().exit_code())
    }

    /// Map remote HTTP status to a domain error.
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
        assert_eq!(ErrorKind::Timeout.exit_code(), 124);
        assert_eq!(ErrorKind::Interrupted.exit_code(), 130);
        assert_eq!(ErrorKind::Terminated.exit_code(), 143);
        assert_eq!(ErrorKind::Config.exit_code(), 78);
        assert_eq!(ErrorKind::Internal.exit_code(), 70);
    }

    #[test]
    fn cancel_helpers() {
        assert_eq!(AppError::interrupted().kind(), ErrorKind::Interrupted);
        assert_eq!(AppError::terminated().kind(), ErrorKind::Terminated);
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
        assert!(!ErrorKind::NotFound.retryable());
        assert_eq!(ErrorKind::Parse.as_str(), "parse");
        assert_eq!(ErrorKind::Config.exit_code(), 78);
    }

    #[test]
    fn with_source_preserves_kind() {
        let e = AppError::with_source(ErrorKind::Parse, "bad", std::io::Error::other("inner"));
        assert_eq!(e.kind(), ErrorKind::Parse);
        assert_eq!(e.message(), "bad");
    }
}
