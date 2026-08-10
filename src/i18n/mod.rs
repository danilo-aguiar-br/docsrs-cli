//! Human-facing messages for stderr prose (`en` / `pt-BR`).
//!
//! # Product contract (agent-first)
//!
//! - **Stdout** (JSON envelopes, markdown data): always English technical content.
//!   Agents parse `error.message` as stable English; never localize JSON fields.
//! - **Stderr** human path (`--format text|markdown`, progress, doctor prose):
//!   localized via [`Locale`] after resolution at process start.
//! - **Technical logs** (`tracing`): English only.
//!
//! # Resolution precedence
//!
//! 1. Explicit CLI `--lang` / config `lang` (fail-closed)
//! 2. OS locale via [`sys_locale::get_locale`] (soft-fallback)
//! 3. Default `en`
//!
//! Explicit tags accept only `en` / `en-*` and exact `pt-BR` (after BCP47-ish
//! normalization). Bare `pt` and regional Portuguese other than `pt-BR` fail
//! closed (exit 65). Soft system detection maps any `pt` primary language to
//! Brazilian Portuguese messages (MVP has no `pt-PT` bundle).
//!
//! # What step 2 actually reads
//!
//! [`sys_locale::get_locale`] consults the host's locale, which on Unix means
//! `LC_ALL`, `LC_MESSAGES` and `LANG`. So `LANG=pt_BR.UTF-8` does change stderr
//! prose when no explicit tag is given — state it plainly rather than claim the
//! process never touches the environment, because the read is real and merely
//! delegated to a dependency where the policy gate cannot see it.
//!
//! This belongs to the same category as `NO_COLOR` and `TERM` in
//! [`crate::platform`]: it describes the *user's environment*, the way `isatty`
//! does, and carries no product configuration. The distinction that matters is
//! the one this module already enforces — **no product knob** comes from the
//! environment, and stdout stays English byte for byte whatever `LANG` says.
//! An explicit `--lang` or the TOML `lang` key always outranks the host.
//!
//! # Out of scope (OOS) for this one-shot CLI
//!
//! Full Rules Rust i18n stacks are **not** product requirements here:
//!
//! - `fluent` / `fluent-langneg` / `i18n-embed` resource bundles
//! - `unic-langid` (two fixed locales; lightweight normalize is enough)
//! - Top-20 languages, Cargo `i18n-*` features, RTL/CJK/ICU calendars
//! - Global `OnceLock` (locale is threaded as a value for testability)
//! - Admin reload of language, Weblate/Crowdin pipelines
//! - Windows console UTF-8 code-page bootstrap (no Windows-only deps)
//! - Translating clap `--help` / about strings
//!
//! Keep this module the single place for human stderr copy.

#[cfg(test)]
mod tests;

use crate::error::{AppError, AppResult, ErrorDetail};

/// Resolved UI locale for stderr prose.
///
/// Not `#[non_exhaustive]`: only `en` and `pt-BR` ship in the MVP binary and
/// exhaustive matches in this crate are intentional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// English human messages.
    En,
    /// Brazilian Portuguese human messages.
    PtBr,
}

impl Locale {
    /// Stable BCP 47 tag for diagnostics (`en` or `pt-BR`).
    pub fn as_bcp47(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::PtBr => "pt-BR",
        }
    }

    /// Resolve locale with fail-closed explicit tags and soft system fallback.
    ///
    /// Precedence: `explicit` (CLI/config) → system locale → `en`.
    /// Explicit tags that are not `en` / `pt-BR` (and `en-*` variants)
    /// return `InvalidInput` (exit 65). Unknown system locales soft-fallback to English.
    ///
    /// No product **knob** comes from the environment. The host locale does
    /// reach step 2 through `sys_locale` (`LC_ALL` / `LC_MESSAGES` / `LANG` on
    /// Unix); it steers stderr prose only, never a setting, and never stdout.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] when an explicit tag is unsupported.
    pub fn resolve(explicit: Option<&str>) -> AppResult<Self> {
        if let Some(tag) = explicit {
            return Self::parse_supported(tag);
        }
        Ok(Self::from_system())
    }

    /// Parse an explicit lang tag. Fail-closed.
    ///
    /// Accepted after normalization:
    /// - `en`, `en-US`, `en_GB`, `en.UTF-8`, …
    /// - `pt-BR`, `pt_BR`, `pt-br`, `pt_BR.UTF-8` (region **must** be BR)
    ///
    /// Rejected: bare `pt`, `pt-PT`, `fr`, empty, unknown tags.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] when `tag` is empty or unsupported.
    pub fn parse_supported(tag: &str) -> AppResult<Self> {
        let t = normalize_lang_tag(tag);
        if t.is_empty() {
            return Err(AppError::of(ErrorDetail::UnsupportedLang {
                tag: String::new(),
            }));
        }
        if t == "en" || t.starts_with("en-") {
            return Ok(Self::En);
        }
        // Explicit path requires region BR — bare `pt` / `pt-PT` are not MVP locales.
        if t == "pt-br" {
            return Ok(Self::PtBr);
        }
        Err(AppError::of(ErrorDetail::UnsupportedLang {
            tag: tag.to_string(),
        }))
    }

    /// Policy test helper: env must never win over system when no explicit tag.
    ///
    /// # Errors
    ///
    /// Propagates [`crate::error::ErrorKind::InvalidInput`] from [`Locale::resolve`]. With no
    /// explicit tag the resolver takes the soft system path, so this helper
    /// returns `Ok` in practice; the signature mirrors the product resolver.
    #[cfg(test)]
    pub fn resolve_ignores_product_env_for_tests() -> AppResult<Self> {
        // Call the public resolver with no explicit tag — same path as product.
        Self::resolve(None)
    }

    /// Soft-detect from OS locale (unknown tags → English).
    ///
    /// Called after tracing init in the binary path. Logs when the OS returns
    /// no locale so detection failure is visible under `-v`.
    pub fn from_system() -> Self {
        match sys_locale::get_locale() {
            Some(sys) => {
                let loc = Self::soft_from_tag(&sys);
                tracing::debug!(
                    target: "docsrs_cli::i18n",
                    system_locale = %sys,
                    resolved = %loc.as_bcp47(),
                    "locale resolved from system"
                );
                loc
            }
            None => {
                tracing::debug!(
                    target: "docsrs_cli::i18n",
                    "sys-locale returned None; defaulting to en"
                );
                Self::En
            }
        }
    }

    /// Soft map a system/BCP47-ish tag (never errors).
    ///
    /// Any primary language `pt` (including `pt`, `pt-PT`, `pt_BR.UTF-8`) maps
    /// to [`Locale::PtBr`] for the MVP bilingual bundle.
    pub fn soft_from_tag(tag: &str) -> Self {
        let t = normalize_lang_tag(tag);
        if t == "pt" || t.starts_with("pt-") {
            Self::PtBr
        } else {
            Self::En
        }
    }

    /// Progress line after the 2s threshold.
    pub fn progress_fetching(self, target: &str) -> String {
        match self {
            Self::En => format!("fetching {target}..."),
            Self::PtBr => format!("buscando {target}..."),
        }
    }

    /// Prefix for human stderr errors.
    pub fn error_prefix(self) -> &'static str {
        match self {
            Self::En => "error",
            Self::PtBr => "erro",
        }
    }

    /// Prefix and render an [`AppError`] for human stderr.
    ///
    /// The message comes from the typed catalogue via [`AppError::localized`],
    /// so pt-BR coverage is enforced by the compiler. This replaced a
    /// `starts_with` matcher over English prose whose fallback returned English
    /// unchanged: a new message was untranslatable *and* indistinguishable from
    /// a translated one.
    pub fn format_error(self, err: &AppError) -> String {
        format!("{}: {}", self.error_prefix(), err.localized(self))
    }

    /// Doctor summary line.
    pub fn doctor_ok(self, ok: bool) -> String {
        match (self, ok) {
            (Self::En, true) => "doctor: ok".into(),
            (Self::En, false) => "doctor: failed".into(),
            (Self::PtBr, true) => "doctor: ok".into(),
            (Self::PtBr, false) => "doctor: falhou".into(),
        }
    }

    /// Locale for messages emitted **before** argv is parsed.
    ///
    /// The binary entrypoint reports two failures that happen during bootstrap:
    /// a duplicate rustls `CryptoProvider` and a runtime that will not build.
    /// Both precede `clap`, so neither [`Locale::resolve`] nor the XDG config is
    /// available — reading `config.toml` there would put disk I/O ahead of TLS
    /// setup and invert the bootstrap order fixed by ADR 0007.
    ///
    /// So this scans argv directly for `--lang <tag>` / `--lang=<tag>`: no disk,
    /// no environment variable, one linear pass. It uses
    /// [`Locale::soft_from_tag`] rather than [`Locale::parse_supported`] on
    /// purpose — a malformed tag must not turn a bootstrap failure into a second,
    /// unrelated error. Absent or unknown tags fall back to English, matching
    /// what the normal path would print before the system locale is consulted.
    ///
    /// This is the **only** sanctioned pre-argv message path in the product.
    pub fn from_argv_for_bootstrap<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: AsRef<std::ffi::OsStr>,
    {
        const FLAG: &str = "--lang";
        let mut want_value = false;
        for arg in args {
            // Non-UTF-8 argv cannot hold a locale tag; skip without allocating.
            let Some(a) = arg.as_ref().to_str() else {
                want_value = false;
                continue;
            };
            if want_value {
                return Self::soft_from_tag(a);
            }
            if let Some(tag) = a.strip_prefix("--lang=") {
                return Self::soft_from_tag(tag);
            }
            if a == FLAG {
                want_value = true;
            }
        }
        Self::En
    }

    /// Bootstrap failure: a rustls `CryptoProvider` was already installed.
    pub fn bootstrap_provider_conflict(self) -> &'static str {
        match self {
            Self::En => {
                "docsrs-cli: rustls CryptoProvider was already installed; refusing dual init"
            }
            Self::PtBr => {
                "docsrs-cli: o CryptoProvider do rustls já estava instalado; recusando init duplo"
            }
        }
    }

    /// Bootstrap failure: the async runtime could not be built.
    pub fn bootstrap_runtime_failure(self, detail: &str) -> String {
        match self {
            Self::En => format!("docsrs-cli: failed to build async runtime: {detail}"),
            Self::PtBr => format!("docsrs-cli: falha ao construir o runtime assíncrono: {detail}"),
        }
    }
}

/// Normalize a locale tag for comparison (BCP47-ish, no full unic-langid).
///
/// - trim whitespace
/// - drop encoding / modifier suffixes (`.UTF-8`, `@euro`)
/// - `_` → `-`
/// - lowercase
fn normalize_lang_tag(tag: &str) -> String {
    let trimmed = tag.trim();
    let base = trimmed.split(['.', '@']).next().unwrap_or(trimmed).trim();
    base.replace('_', "-").to_ascii_lowercase()
}
