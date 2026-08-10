//! English renderings — the `message` field of the JSON envelope.
//!
//! Style contract (ADR 0002): lowercase opening, no trailing period, no secrets,
//! and never the cause text (that lives on the source chain). Agents match on
//! this wording, so changing it is a compatibility decision.

use super::ASSOC_ANCHOR_MISS;
use crate::error::detail::ErrorDetail;
use crate::error::vocab::{
    AllowlistStage, ContentKind, DestructiveEffect, InternalOp, IoCause, IoOp, Subject,
};

impl Subject {
    /// English name used inside a rendered sentence.
    pub(crate) fn en(self) -> &'static str {
        match self {
            Self::CrateName => "crate name",
            Self::CrateRef => "crate reference",
            Self::SearchQuery => "search query",
            Self::ItemPath => "item path",
            Self::ItemPathSegment => "item path segment",
            Self::Version => "version",
            Self::PageToken => "page-token",
            Self::UserAgent => "user-agent",
            Self::Contact => "contact",
            Self::Origin => "origin URL",
            Self::Timeout => "timeout",
            Self::ConnectTimeout => "connect_timeout",
            Self::MaxBodyBytes => "max_body_bytes",
            Self::MaxOutputBytes => "max_output_bytes",
            Self::ConfigFile => "config.toml",
            Self::LogDirective => "log_directive",
        }
    }
}

impl IoOp {
    pub(crate) fn en(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::CreateDir => "create directory",
            Self::CreateTemp => "create temporary file",
            Self::Rename => "rename",
            Self::Sync => "sync",
            Self::Install => "install",
            Self::OpenLock => "open lock file",
            Self::Lock => "acquire exclusive lock",
            Self::ReadDir => "list directory",
            Self::Remove => "remove",
        }
    }
}

impl InternalOp {
    pub(crate) fn en(self) -> &'static str {
        match self {
            Self::JsonSerialize => "json serialize failed",
            Self::JsonPrettyPrint => "json pretty-print failed",
            Self::StdoutWrite => "stdout write failed",
            Self::EmbeddedSchemaInvalid => "embedded schema is invalid JSON",
            Self::UrlBuild => "built an invalid URL",
            Self::SemaphoreClosed => "concurrency semaphore closed",
            Self::WorkerJoin => "cpu worker join failed",
            Self::WorkerPanic => "cpu worker panicked during parse",
            Self::ClockBeforeEpoch => "system clock before UNIX epoch",
            Self::CacheKeyMalformed => {
                "cache key is not valid sha256 hex (internal invariant broken)"
            }
            Self::AssocPathTooShort => {
                "associated item path invariant broken: expected at least 2 segments"
            }
            Self::AssocParentOwnsNoPage => "associated-item parent kind owns no rustdoc page",
            Self::AllowedOriginUnparseable => "allowed origin is not a valid URL",
            Self::SyntheticParseFailure => "synthetic parse failure",
        }
    }
}

impl AllowlistStage {
    pub(crate) fn en(self) -> &'static str {
        match self {
            Self::Request => "host",
            Self::Redirect => "redirect host",
            Self::FinalUrl => "final URL host",
        }
    }
}

impl ContentKind {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Html => "HTML",
            Self::Json => "JSON",
        }
    }
}

/// English rendering — this is the `message` field of the JSON envelope.
///
/// Style contract (ADR 0002): lowercase opening, no trailing period, no secrets,
/// and never the cause text (that lives on the source chain).
pub(crate) fn render_en(d: &ErrorDetail) -> String {
    match d {
        ErrorDetail::Empty { subject } => format!("{} is empty", subject.en()),
        ErrorDetail::TooLong { subject, limit } => {
            format!("{} exceeds {limit} characters", subject.en())
        }
        ErrorDetail::Invalid { subject, value } => {
            format!("invalid {} '{value}'", subject.en())
        }
        ErrorDetail::ControlCharacters { subject } => {
            format!("{} contains control or invisible characters", subject.en())
        }
        ErrorDetail::NotVisibleAscii { subject } => format!(
            "{} must be visible ASCII (no control characters or non-ASCII)",
            subject.en()
        ),
        ErrorDetail::ContainsWhitespace { subject, value } => {
            format!("{} contains whitespace: '{value}'", subject.en())
        }
        ErrorDetail::MustBeAtLeastOneSecond { subject, .. } => {
            format!("{} must be >= 1 second (got 0)", subject.en())
        }
        ErrorDetail::AboveHardMaximum {
            subject, hard_max, ..
        } => {
            format!("{} exceeds hard maximum ({hard_max})", subject.en())
        }

        ErrorDetail::ItemPathNoSegments => "item path has no segments".into(),
        ErrorDetail::ItemPathMissingItemName => "item path missing item name".into(),
        ErrorDetail::CrateRefMultipleAt => "crate reference must contain at most one '@'".into(),
        ErrorDetail::CrateRefEmptyName => "crate name is empty before '@'".into(),
        ErrorDetail::CrateRefEmptyVersion => "version is empty after '@'".into(),
        ErrorDetail::VersionBuildMetadata => "version build metadata is not accepted".into(),
        ErrorDetail::VersionVPrefix => "version must not start with 'v' prefix".into(),

        ErrorDetail::UnknownMatchMode { value } => {
            format!("unknown match mode '{value}' (expected exact|prefix|substring)")
        }
        ErrorDetail::UnknownItemType { value } => format!("unknown item type '{value}'"),
        ErrorDetail::UnsupportedLang { tag } => {
            format!("unsupported lang '{tag}'; expected en or pt-BR")
        }
        ErrorDetail::UnknownSchemaCommand { value } => format!("unknown schema command '{value}'"),
        ErrorDetail::JsonFormatConflict => {
            "cannot combine --json with --format text or --format markdown".into()
        }
        ErrorDetail::ClapUsage { message } => message.clone(),
        ErrorDetail::InvalidFilterExpression { expr, reason } => {
            format!("invalid --filter expression `{expr}`: {reason}")
        }
        ErrorDetail::ModuleFilterUnsupported => concat!(
            "search-in-crate --item-type module is not supported ",
            "(all.html has no module index); use get-item with kind module"
        )
        .into(),

        ErrorDetail::PageBelowOne => "page must be >= 1 (got 0 or missing)".into(),
        ErrorDetail::PerPageOutOfRange { max, got } => {
            format!("per_page must be 1..={max} (got {got})")
        }

        ErrorDetail::OriginBadScheme { scheme } => {
            format!("origin scheme must be http or https, got '{scheme}'")
        }
        ErrorDetail::OriginMissingHost { url } => {
            format!("origin URL must include a host: '{url}'")
        }
        ErrorDetail::OriginNotAllowlisted { host, allowed } => format!(
            "origin host not allowlisted: '{host}' (allowed: {allowed}; \
             loopback requires allow_loopback via CLI or config.toml)"
        ),
        ErrorDetail::ItemPathSegmentCharset { segment } => format!(
            "invalid item path segment '{segment}' (use letters, digits, underscore or hyphen; \
             hyphens normalize to underscore; separate with :: or /)"
        ),
        ErrorDetail::ConflictingVersions {
            from_ref,
            from_flag,
        } => format!("conflicting versions: crate@{from_ref} vs --crate-version {from_flag}"),
        ErrorDetail::ConfigDirUnresolved => {
            "config directory could not be resolved (set --config-dir or ensure XDG config home)"
                .into()
        }
        ErrorDetail::CacheDirUnresolved => {
            "cache directory could not be resolved (set --cache-dir or ensure XDG cache home)"
                .into()
        }
        ErrorDetail::AmbientTargetRefused {
            verb,
            target,
            target_flag,
            waiver_flag,
            effect,
        } => {
            let harm = match effect {
                DestructiveEffect::Delete => "delete",
                DestructiveEffect::Overwrite => "overwrite",
            };
            format!(
                "{verb} would {harm} an ambient target it was never given: {target}; \
                 name it in argv with {target_flag} <DIR>, or accept it with {waiver_flag}"
            )
        }
        ErrorDetail::ConfigTomlInvalid => "invalid config.toml".into(),
        ErrorDetail::ConfigTomlNotUtf8 => "config.toml is not valid UTF-8".into(),
        ErrorDetail::ConfigTomlTooLarge { max_bytes } => {
            format!("config.toml exceeds max size ({max_bytes} bytes)")
        }
        ErrorDetail::ConfigAlreadyExists { path } => {
            format!("config already exists: {path} (pass --force to overwrite)")
        }
        ErrorDetail::UserAgentHeaderInvalid => "invalid user-agent header".into(),

        ErrorDetail::HttpStatus { status, hint } => match status {
            400 => format!("bad request from remote: {hint}"),
            404 => format!("resource not found: {hint}"),
            408 => format!("remote request timeout (HTTP 408): {hint}"),
            429 => format!("rate limited by remote: {hint}"),
            500 | 502 | 503 | 504 => format!("remote unavailable (HTTP {status}): {hint}"),
            other => format!("unexpected HTTP status {other}: {hint}"),
        },
        ErrorDetail::HostNotAllowlisted { host, stage } => {
            format!("{} not allowlisted: {host}", stage.en())
        }
        ErrorDetail::HttpRequestFailed { url, .. } => format!("http request failed for {url}"),
        ErrorDetail::RedirectLimitExceeded => "redirect limit exceeded".into(),
        ErrorDetail::HttpClientBuild => "failed to build HTTP client".into(),
        ErrorDetail::BodyRead => "failed reading response body".into(),
        ErrorDetail::BodyOverBudget { max_bytes } => {
            format!("response body exceeds max_body_bytes ({max_bytes})")
        }
        ErrorDetail::CachedBodyOverBudget { max_bytes } => {
            format!("cached response body exceeds max_body_bytes ({max_bytes})")
        }
        ErrorDetail::BodyReserveFailed { bytes } => {
            format!("failed to reserve {bytes} bytes for response body")
        }
        ErrorDetail::BodyNotUtf8 => "response body is not valid UTF-8".into(),
        ErrorDetail::UnexpectedContentType { expected, got } => format!(
            "unexpected Content-Type for {} response: {got}",
            expected.label()
        ),
        ErrorDetail::OutputOverBudget => "body too large".into(),

        ErrorDetail::HtmlToMarkdown => "html to markdown conversion failed".into(),
        ErrorDetail::CratesIoJson => "failed to parse crates.io JSON".into(),
        ErrorDetail::AssocParentPageNotFound => "method parent type page not found".into(),
        ErrorDetail::AssocAnchorMissing { anchors } => {
            format!("{ASSOC_ANCHOR_MISS}not found: {anchors}")
        }
        ErrorDetail::AssocAnchorEmpty { anchor_id } => {
            format!("{ASSOC_ANCHOR_MISS}empty: {anchor_id}")
        }
        ErrorDetail::MemberKindNeedsParent {
            kind,
            member,
            parent_kinds,
        } => format!(
            "{kind} has no page of its own: qualify it as Parent::{member} (parent kinds: {parent_kinds})"
        ),
        ErrorDetail::HitJoinFailed => {
            "failed to join href (off-origin or invalid against source_url base)".into()
        }
        ErrorDetail::HitBaseInvalid => "invalid source_url for hit join base".into(),

        ErrorDetail::Interrupted => "interrupted by SIGINT".into(),
        ErrorDetail::Terminated => "terminated by SIGTERM".into(),
        ErrorDetail::BrokenPipe => "broken pipe writing to stdout".into(),
        ErrorDetail::WallClockTimeout { secs } => format!("wall-clock timeout after {secs}s"),

        ErrorDetail::Io { op, path, cause } => {
            // The cause is what tells the reader whether to free space and retry
            // or to fix a permission and stop; the wire says it in `retryable`,
            // and the human sentence must not be quieter than the JSON.
            let hint = match cause {
                IoCause::Transient => {
                    " (transient; the same command may succeed once the condition clears)"
                }
                IoCause::Permanent => " (permanent; the environment must change first)",
            };
            match path {
                Some(p) => format!("failed to {} {p}{hint}", op.en()),
                None => format!("failed to {}{hint}", op.en()),
            }
        }
        ErrorDetail::Internal { op } => op.en().into(),

        ErrorDetail::WithSuggestions { base, suggestions } => {
            let list = crate::error::suggestion::join_suggestions(suggestions);
            format!("{}; suggestions: {list}", render_en(base))
        }
    }
}
