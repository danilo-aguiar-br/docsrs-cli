//! Unit tests for locale resolution and the message catalogue.

use super::*;
use crate::error::ErrorKind;

#[test]
fn bootstrap_locale_reads_lang_from_argv_only() {
    // Space form, `=` form, and the `pt` family all reach pt-BR.
    assert_eq!(
        Locale::from_argv_for_bootstrap(["docsrs-cli", "--lang", "pt-BR", "version"]),
        Locale::PtBr
    );
    assert_eq!(
        Locale::from_argv_for_bootstrap(["docsrs-cli", "--lang=pt_BR.UTF-8"]),
        Locale::PtBr
    );
    // No flag, unknown tag, and a dangling flag all fall back to English
    // instead of raising a second, unrelated error during bootstrap.
    assert_eq!(
        Locale::from_argv_for_bootstrap(["docsrs-cli", "version"]),
        Locale::En
    );
    assert_eq!(
        Locale::from_argv_for_bootstrap(["docsrs-cli", "--lang", "xx-YY"]),
        Locale::En
    );
    assert_eq!(
        Locale::from_argv_for_bootstrap(["docsrs-cli", "--lang"]),
        Locale::En
    );
}

#[test]
fn bootstrap_messages_differ_per_locale() {
    // The whole point of the pre-argv resolver: these two lines are the only
    // product messages emitted before clap runs, and they must localize.
    assert_ne!(
        Locale::En.bootstrap_provider_conflict(),
        Locale::PtBr.bootstrap_provider_conflict()
    );
    assert_ne!(
        Locale::En.bootstrap_runtime_failure("boom"),
        Locale::PtBr.bootstrap_runtime_failure("boom")
    );
    // The technical detail survives translation in both locales.
    assert!(
        Locale::En
            .bootstrap_runtime_failure("boom")
            .contains("boom")
    );
    assert!(
        Locale::PtBr
            .bootstrap_runtime_failure("boom")
            .contains("boom")
    );
}

#[test]
fn normalize_strips_encoding_and_underscore() {
    assert_eq!(normalize_lang_tag("pt_BR.UTF-8"), "pt-br");
    assert_eq!(normalize_lang_tag("en-US"), "en-us");
    assert_eq!(normalize_lang_tag("  PT-br@euro  "), "pt-br");
}

#[test]
fn parse_supported_en_pt_br() {
    assert_eq!(Locale::parse_supported("pt-BR").unwrap(), Locale::PtBr);
    assert_eq!(Locale::parse_supported("pt_BR").unwrap(), Locale::PtBr);
    assert_eq!(
        Locale::parse_supported("pt_BR.UTF-8").unwrap(),
        Locale::PtBr
    );
    assert_eq!(Locale::parse_supported("en-US").unwrap(), Locale::En);
    assert_eq!(Locale::parse_supported("en").unwrap(), Locale::En);
    assert_eq!(Locale::parse_supported("en.UTF-8").unwrap(), Locale::En);
}

#[test]
fn parse_supported_fail_closed() {
    // Bare `pt` and other regions are not explicit MVP tags.
    assert!(Locale::parse_supported("pt").is_err());
    assert!(Locale::parse_supported("pt-PT").is_err());
    assert!(Locale::parse_supported("fr").is_err());
    assert!(Locale::parse_supported("xx").is_err());
    assert!(Locale::parse_supported("").is_err());
    assert!(Locale::parse_supported("  ").is_err());
    let err = Locale::parse_supported("de").unwrap_err();
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.message().contains("unsupported lang"));
}

#[test]
fn soft_system_maps_any_portuguese() {
    assert_eq!(Locale::soft_from_tag("de-DE"), Locale::En);
    assert_eq!(Locale::soft_from_tag("pt-BR"), Locale::PtBr);
    assert_eq!(Locale::soft_from_tag("pt"), Locale::PtBr);
    assert_eq!(Locale::soft_from_tag("pt-PT"), Locale::PtBr);
    assert_eq!(Locale::soft_from_tag("pt_BR.UTF-8"), Locale::PtBr);
    assert_eq!(Locale::soft_from_tag("C"), Locale::En);
    assert_eq!(Locale::soft_from_tag("C.UTF-8"), Locale::En);
}

#[test]
fn as_bcp47_stable() {
    assert_eq!(Locale::En.as_bcp47(), "en");
    assert_eq!(Locale::PtBr.as_bcp47(), "pt-BR");
}

#[test]
fn progress_localized() {
    assert!(Locale::En.progress_fetching("x").contains("fetching"));
    assert!(Locale::PtBr.progress_fetching("x").contains("buscando"));
}

#[test]
fn error_and_doctor_lines() {
    use crate::error::{AppError, ErrorDetail, Subject};

    let empty_name = AppError::of(ErrorDetail::Empty {
        subject: Subject::CrateName,
    });
    assert!(Locale::En.format_error(&empty_name).starts_with("error:"));
    assert!(Locale::PtBr.format_error(&empty_name).starts_with("erro:"));

    let pt = Locale::PtBr.format_error(&empty_name);
    assert!(pt.contains("nome da crate está vazio"), "{pt}");

    let pt2 = Locale::PtBr.format_error(&AppError::interrupted());
    assert!(pt2.contains("interrompido por SIGINT"), "{pt2}");

    let bad_version = AppError::of(ErrorDetail::Invalid {
        subject: Subject::Version,
        value: "v1".into(),
    });
    let pt3 = Locale::PtBr.format_error(&bad_version);
    assert!(pt3.contains("versão inválid"), "{pt3}");
    // The wire message stays English whatever the stderr locale is.
    assert_eq!(bad_version.message(), "invalid version 'v1'");

    assert_eq!(Locale::En.doctor_ok(true), "doctor: ok");
    assert_eq!(Locale::PtBr.doctor_ok(false), "doctor: falhou");
    assert_eq!(Locale::resolve(Some("pt-BR")).unwrap(), Locale::PtBr);
    assert_eq!(Locale::resolve(Some("en")).unwrap(), Locale::En);
    assert!(Locale::resolve(Some("ja")).is_err());
    assert!(Locale::resolve(Some("pt")).is_err());
}
