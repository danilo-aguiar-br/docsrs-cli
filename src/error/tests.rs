//! Unit tests for error kinds, exit codes, and the typed detail catalogue.

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
    assert_eq!(
        AppError::broken_pipe().exit_code(),
        ExitCode::from(EXIT_BROKEN_PIPE)
    );
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
    let e = AppError::of_with_source(ErrorDetail::HtmlToMarkdown, std::io::Error::other("inner"));
    assert_eq!(e.kind(), ErrorKind::Parse);
    assert_eq!(e.message(), "html to markdown conversion failed");
}

#[test]
fn source_chain_via_std_error() {
    use std::error::Error as _;
    let e = AppError::of_with_source(ErrorDetail::HtmlToMarkdown, std::io::Error::other("inner"));
    let src = e.source().expect("source present");
    assert!(src.to_string().contains("inner"));
    // Display is message only — cause is not duplicated.
    assert_eq!(e.to_string(), "html to markdown conversion failed");
    assert!(!e.to_string().contains("inner"));
}

#[test]
fn app_error_clone_shares_source() {
    use std::error::Error as _;
    let e = AppError::of_with_source(ErrorDetail::BodyRead, std::io::Error::other("eof"));
    let c = e.clone();
    assert_eq!(c.kind(), ErrorKind::Network);
    assert_eq!(c.message(), "failed reading response body");
    assert!(c.source().is_some());
    assert!(c.is_retryable());
    assert!(!c.is_permanent());
    assert!(AppError::of(ErrorDetail::AssocParentPageNotFound).is_permanent());
    assert_eq!(c.retry_kind(), crate::retry::RetryKind::TransientNetwork);
    assert_eq!(
        AppError::of(ErrorDetail::OutputOverBudget).retry_kind(),
        crate::retry::RetryKind::Permanent
    );
    assert_eq!(
        ErrorKind::Timeout.retry_kind(),
        crate::retry::RetryKind::Timeout
    );
}

#[test]
fn display_message_style() {
    let e = AppError::of(ErrorDetail::ConfigTomlInvalid);
    let s = e.to_string();
    assert_eq!(s, "invalid config.toml");
    assert!(!s.ends_with('.'));
    assert_eq!(s, s.to_ascii_lowercase());
}

#[test]
fn with_source_display_does_not_embed_cause() {
    use std::error::Error as _;
    let e = AppError::of_with_source(
        ErrorDetail::HtmlToMarkdown,
        std::io::Error::other("inner cause text"),
    );
    assert_eq!(e.to_string(), "html to markdown conversion failed");
    assert!(!e.to_string().contains("inner cause text"));
    assert!(e.source().is_some());
    assert!(e.source().unwrap().to_string().contains("inner cause text"));
    assert_eq!(e.to_string(), e.to_string().to_ascii_lowercase());
}

// ── Catalogue contract ──────────────────────────────────────────────────────

/// Representative detail from each family, for the round-trip checks below.
///
/// Not exhaustive by construction — the compiler already guarantees that, since
/// the pt-BR renderer has no `_` arm. These cover the *shapes* whose rendering
/// carries data, where a copy-paste slip between the two languages is possible.
fn sample_details() -> Vec<ErrorDetail> {
    vec![
        ErrorDetail::Empty {
            subject: Subject::CrateName,
        },
        ErrorDetail::TooLong {
            subject: Subject::SearchQuery,
            limit: 64,
        },
        ErrorDetail::Invalid {
            subject: Subject::Version,
            value: "v1".into(),
        },
        ErrorDetail::ControlCharacters {
            subject: Subject::PageToken,
        },
        ErrorDetail::NotVisibleAscii {
            subject: Subject::UserAgent,
        },
        ErrorDetail::ContainsWhitespace {
            subject: Subject::ItemPathSegment,
            value: "a b".into(),
        },
        ErrorDetail::MustBeAtLeastOneSecond {
            subject: Subject::Timeout,
            source: ValueSource::CommandLine,
        },
        ErrorDetail::AboveHardMaximum {
            subject: Subject::MaxBodyBytes,
            hard_max: 10,
            source: ValueSource::ConfigFile,
        },
        ErrorDetail::ItemPathNoSegments,
        ErrorDetail::CrateRefMultipleAt,
        ErrorDetail::UnknownItemType {
            value: "widget".into(),
        },
        ErrorDetail::UnsupportedLang { tag: "ja".into() },
        ErrorDetail::PerPageOutOfRange { max: 100, got: 500 },
        ErrorDetail::OriginBadScheme {
            scheme: "ftp".into(),
        },
        ErrorDetail::ConfigTomlTooLarge { max_bytes: 1024 },
        ErrorDetail::HttpStatus {
            status: 429,
            hint: "search".into(),
        },
        ErrorDetail::HostNotAllowlisted {
            host: "evil.test".into(),
            stage: AllowlistStage::Redirect,
        },
        ErrorDetail::BodyOverBudget { max_bytes: 99 },
        ErrorDetail::UnexpectedContentType {
            expected: ContentKind::Json,
            got: "text/csv".into(),
        },
        ErrorDetail::AssocAnchorMissing {
            anchors: "method.x, tymethod.x".into(),
        },
        ErrorDetail::MemberKindNeedsParent {
            kind: "variant",
            member: "Some".into(),
            parent_kinds: "enum".into(),
        },
        ErrorDetail::WallClockTimeout { secs: 30 },
        ErrorDetail::Io {
            op: IoOp::Rename,
            path: Some("/tmp/x".into()),
            cause: IoCause::Transient,
        },
        ErrorDetail::Internal {
            op: InternalOp::JsonSerialize,
        },
        ErrorDetail::WithSuggestions {
            base: Box::new(ErrorDetail::AssocAnchorMissing {
                anchors: "variant.some".into(),
            }),
            suggestions: vec![Suggestion::new("Option::Some", "variant")],
        },
    ]
}

#[test]
fn every_sampled_detail_renders_differently_per_locale() {
    // The old translator returned English unchanged when it had no arm for a
    // message, so an untranslated error was indistinguishable from a translated
    // one. Identical renderings are now the signal that a translation is
    // missing — except where the text is deliberately language-neutral.
    for d in sample_details() {
        let en = AppError::of(d.clone()).localized(crate::i18n::Locale::En);
        let pt = AppError::of(d.clone()).localized(crate::i18n::Locale::PtBr);
        assert!(!en.is_empty(), "{d:?} rendered empty in en");
        assert!(!pt.is_empty(), "{d:?} rendered empty in pt-BR");
        assert_ne!(en, pt, "{d:?} was not translated");
    }
}

#[test]
fn wire_message_is_always_the_english_rendering() {
    // `message` is the JSON contract; `--lang pt-BR` must never reach it.
    for d in sample_details() {
        let e = AppError::of(d.clone());
        assert_eq!(
            e.message(),
            e.localized(crate::i18n::Locale::En),
            "{d:?} wire message drifted from the English rendering"
        );
    }
}

#[test]
fn detail_decides_kind_so_two_call_sites_cannot_disagree() {
    // Kind is derived, never passed in: the same detail always exits the same way.
    for d in sample_details() {
        assert_eq!(AppError::of(d.clone()).kind(), d.kind(), "{d:?}");
    }
    // Layer, not shape, splits an over-limit value between exit 65 and exit 78.
    assert_eq!(
        ErrorDetail::AboveHardMaximum {
            subject: Subject::MaxBodyBytes,
            hard_max: 1,
            source: ValueSource::CommandLine,
        }
        .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        ErrorDetail::AboveHardMaximum {
            subject: Subject::MaxBodyBytes,
            hard_max: 1,
            source: ValueSource::ConfigFile,
        }
        .kind(),
        ErrorKind::Config
    );
    // Same shape, different subject: command input vs operator configuration.
    assert_eq!(
        ErrorDetail::Empty {
            subject: Subject::CrateName
        }
        .kind(),
        ErrorKind::InvalidInput
    );
    assert_eq!(
        ErrorDetail::Empty {
            subject: Subject::Origin
        }
        .kind(),
        ErrorKind::Config
    );
}

#[test]
fn suggestions_keep_the_base_failure_kind_and_both_languages() {
    let wrapped = ErrorDetail::WithSuggestions {
        base: Box::new(ErrorDetail::AssocAnchorMissing {
            anchors: "variant.some".into(),
        }),
        suggestions: vec![Suggestion::new("Option::Some", "variant")],
    };
    let e = AppError::of(wrapped);
    assert_eq!(
        e.kind(),
        ErrorKind::NotFound,
        "wrapping must not change kind"
    );
    assert!(e.message().contains("suggestions:"), "{}", e.message());
    let pt = e.localized(crate::i18n::Locale::PtBr);
    assert!(pt.contains("sugestões:"), "{pt}");
    // The sentinel prefix is a wire contract the fetch path matches on.
    assert!(
        e.message().starts_with("associated item anchor "),
        "{}",
        e.message()
    );
}

#[test]
fn upstream_text_is_carried_not_invented() {
    // clap owns this string; translating it would drift from what --help prints.
    let d = ErrorDetail::ClapUsage {
        message: "error: unexpected argument".into(),
    };
    let e = AppError::of(d);
    assert_eq!(e.message(), "error: unexpected argument");
    assert_eq!(
        e.localized(crate::i18n::Locale::PtBr),
        "error: unexpected argument"
    );
    assert_eq!(e.kind(), ErrorKind::Usage);
}

#[test]
fn portuguese_adjectives_agree_with_their_noun() {
    // Found live: "consulta de busca está vazio". English has no agreement, so a
    // translator keyed on English prose could never surface this class of defect.
    let cases = [
        (Subject::SearchQuery, "consulta de busca está vazia"),
        (Subject::CrateName, "nome da crate está vazio"),
        (Subject::Version, "versão está vazia"),
        (Subject::ItemPath, "caminho do item está vazio"),
        (Subject::Origin, "URL de origem está vazia"),
    ];
    for (subject, want) in cases {
        let pt = AppError::of(ErrorDetail::Empty { subject }).localized(crate::i18n::Locale::PtBr);
        assert_eq!(pt, want, "{subject:?}");
    }

    let feminine = AppError::of(ErrorDetail::Invalid {
        subject: Subject::Version,
        value: "v1".into(),
    })
    .localized(crate::i18n::Locale::PtBr);
    assert_eq!(feminine, "versão inválida: 'v1'");

    let masculine = AppError::of(ErrorDetail::Invalid {
        subject: Subject::CrateName,
        value: "!!".into(),
    })
    .localized(crate::i18n::Locale::PtBr);
    assert_eq!(masculine, "nome da crate inválido: '!!'");
}

/// Filesystem failure is classified by the OS cause, not by the call site.
///
/// The measured defect: a full disk arrived as `kind=internal`, `retryable=false`
/// and exit 70 — three claims, all wrong. `internal` tells an agent to report a
/// bug in a binary that behaved correctly, `false` denies a retry that would
/// succeed the moment an operator frees a block, and 70 is the code reserved for
/// defects in this crate.
///
/// The permission branch is verified live in `tests/etd_target_designation.rs`
/// territory; the full-disk branch cannot be provoked from a test, so the
/// classification itself is asserted here.
#[test]
fn io_failures_carry_the_os_cause_rather_than_a_fixed_answer() {
    use std::io::{Error, ErrorKind as OsKind};

    let full = ErrorDetail::io(
        IoOp::Write,
        Some("/x".into()),
        &Error::from(OsKind::StorageFull),
    );
    assert_eq!(
        full.kind(),
        ErrorKind::Io,
        "a full disk is not a product bug"
    );
    assert_eq!(
        full.kind().exit_code(),
        74,
        "EX_IOERR, shared and disambiguated by kind"
    );
    assert!(
        full.retryable(),
        "freeing space makes the same command work; denying the retry is a lie"
    );

    let denied = ErrorDetail::io(
        IoOp::CreateDir,
        Some("/x".into()),
        &Error::from(OsKind::PermissionDenied),
    );
    assert_eq!(denied.kind(), ErrorKind::Io);
    assert!(
        !denied.retryable(),
        "retrying a permission denial spins until the budget is gone"
    );

    // Unknown kinds must stop rather than spin: an agent that halts on a
    // transient fault loses one run, one that retries a permanent fault loses
    // every run it has left.
    let unknown = ErrorDetail::io(IoOp::Sync, None, &Error::from(OsKind::Other));
    assert!(!unknown.retryable(), "the default must be to stop");
}
