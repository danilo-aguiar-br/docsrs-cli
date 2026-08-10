//! Classification: which [`ErrorKind`] a failure is, and whether to retry it.
//!
//! Split out of [`super::detail`] when that file crossed the 500-line ceiling.
//! The division is by responsibility rather than by size: `detail.rs` is the
//! catalogue of *what can fail*, and this file is the single place deciding
//! *how each failure is reported*. Keeping them together meant every new
//! failure shape grew the same file as every new classification rule.
//!
//! Deriving the kind here, instead of letting each call site pass one, is why a
//! single detail can never surface as two different kinds from two different
//! places.

use super::detail::ErrorDetail;
use super::vocab::{InternalOp, IoCause, IoOp};
use super::{ErrorKind, ValueSource};

impl ErrorDetail {
    /// Canonical [`ErrorKind`] for this failure.
    ///
    /// Derived here rather than passed by the caller so one detail can never be
    /// reported as two different kinds from two different call sites.
    pub fn kind(&self) -> ErrorKind {
        match self {
            // Shape shared by command input and operator config: the subject
            // decides which, so the exit code names the layer that needs fixing.
            Self::Empty { subject }
            | Self::TooLong { subject, .. }
            | Self::Invalid { subject, .. }
            | Self::ControlCharacters { subject }
            | Self::ContainsWhitespace { subject, .. } => {
                if subject.is_operator_config() {
                    ErrorKind::Config
                } else {
                    ErrorKind::InvalidInput
                }
            }
            Self::MustBeAtLeastOneSecond { source, .. } | Self::AboveHardMaximum { source, .. } => {
                match source {
                    ValueSource::CommandLine => ErrorKind::InvalidInput,
                    ValueSource::ConfigFile => ErrorKind::Config,
                }
            }

            Self::ItemPathNoSegments
            | Self::ItemPathMissingItemName
            | Self::CrateRefMultipleAt
            | Self::CrateRefEmptyName
            | Self::CrateRefEmptyVersion
            | Self::VersionBuildMetadata
            | Self::VersionVPrefix
            | Self::UnknownMatchMode { .. }
            | Self::UnknownItemType { .. }
            | Self::UnsupportedLang { .. }
            | Self::InvalidFilterExpression { .. }
            | Self::ModuleFilterUnsupported
            | Self::PageBelowOne
            | Self::PerPageOutOfRange { .. }
            | Self::MemberKindNeedsParent { .. }
            | Self::ItemPathSegmentCharset { .. }
            | Self::ConflictingVersions { .. } => ErrorKind::InvalidInput,

            Self::NotVisibleAscii { .. }
            | Self::OriginBadScheme { .. }
            | Self::OriginMissingHost { .. }
            | Self::ConfigDirUnresolved
            | Self::CacheDirUnresolved
            | Self::ConfigTomlInvalid
            | Self::ConfigTomlNotUtf8
            | Self::ConfigTomlTooLarge { .. }
            | Self::UserAgentHeaderInvalid
            | Self::ConfigAlreadyExists { .. }
            | Self::OriginNotAllowlisted { .. } => ErrorKind::Config,

            Self::JsonFormatConflict
            | Self::UnknownSchemaCommand { .. }
            | Self::AmbientTargetRefused { .. }
            | Self::ClapUsage { .. } => ErrorKind::Usage,

            Self::HttpStatus { status, .. } => match status {
                400 => ErrorKind::InvalidInput,
                404 => ErrorKind::NotFound,
                408 => ErrorKind::Timeout,
                429 => ErrorKind::RateLimited,
                500 | 502 | 503 | 504 => ErrorKind::Unavailable,
                _ => ErrorKind::Network,
            },

            Self::HostNotAllowlisted { .. }
            | Self::RedirectLimitExceeded
            | Self::HttpClientBuild
            | Self::BodyRead
            | Self::BodyReserveFailed { .. } => ErrorKind::Network,

            Self::BodyOverBudget { .. }
            | Self::CachedBodyOverBudget { .. }
            | Self::OutputOverBudget => ErrorKind::Budget,

            Self::BodyNotUtf8
            | Self::UnexpectedContentType { .. }
            | Self::HtmlToMarkdown
            | Self::CratesIoJson
            | Self::HitJoinFailed
            | Self::HitBaseInvalid => ErrorKind::Parse,

            Self::AssocParentPageNotFound
            | Self::AssocAnchorMissing { .. }
            | Self::AssocAnchorEmpty { .. } => ErrorKind::NotFound,

            Self::HttpRequestFailed { kind, .. } => *kind,

            Self::Interrupted => ErrorKind::Interrupted,
            Self::Terminated => ErrorKind::Terminated,
            Self::BrokenPipe => ErrorKind::BrokenPipe,
            Self::WallClockTimeout { .. } => ErrorKind::Timeout,

            // A full disk or a read-only mount describes the machine, never a
            // defect in this binary, so it must not arrive as `internal`.
            Self::Io { .. } => ErrorKind::Io,
            Self::Internal { op } => match op {
                InternalOp::SyntheticParseFailure => ErrorKind::Parse,
                _ => ErrorKind::Internal,
            },

            // Suggestions never change what failed, only how it is explained.
            Self::WithSuggestions { base, .. } => base.kind(),
        }
    }

    /// Build a filesystem failure, classifying the OS cause at the call site.
    ///
    /// The classification has to happen here because this is the last place the
    /// [`std::io::Error`] exists. One layer up there is only an [`ErrorKind`],
    /// and `ENOSPC` and `EACCES` share it while deserving opposite answers.
    pub fn io(op: IoOp, path: Option<String>, err: &std::io::Error) -> Self {
        Self::Io {
            op,
            path,
            cause: IoCause::of(err),
        }
    }

    /// Whether an agent may retry the invocation that produced this failure.
    ///
    /// Every kind but [`crate::error::ErrorKind::Io`] answers from the kind alone. Filesystem
    /// failure is the one case where two opposite answers share a kind, so the
    /// cause captured at construction wins.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Io { cause, .. } => cause.retryable(),
            Self::WithSuggestions { base, .. } => base.retryable(),
            other => other.kind().retryable(),
        }
    }
}
