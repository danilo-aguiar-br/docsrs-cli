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

use crate::error::{AppError, AppResult, ErrorKind};

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
    /// Product never reads locale from environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] when an explicit tag is unsupported.
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
    /// Returns [`ErrorKind::InvalidInput`] when `tag` is empty or unsupported.
    pub fn parse_supported(tag: &str) -> AppResult<Self> {
        let t = normalize_lang_tag(tag);
        if t.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "unsupported lang ''; expected en or pt-BR",
            ));
        }
        if t == "en" || t.starts_with("en-") {
            return Ok(Self::En);
        }
        // Explicit path requires region BR — bare `pt` / `pt-PT` are not MVP locales.
        if t == "pt-br" {
            return Ok(Self::PtBr);
        }
        Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("unsupported lang '{tag}'; expected en or pt-BR"),
        ))
    }

    /// Policy test helper: env must never win over system when no explicit tag.
    #[cfg(test)]
    pub fn resolve_ignores_product_env_for_tests() -> AppResult<Self> {
        // Call the public resolver with no explicit tag — same path as product.
        Self::resolve(None)
    }

    /// Soft-detect from OS locale (unknown tags → English).
    ///
    /// Called after tracing init in the binary path. Logs when the OS returns
    /// no locale so detection failure is visible under `-v` / `RUST_LOG`.
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

    /// Human-readable stderr line for a technical English message.
    pub fn format_error(self, technical_en: &str) -> String {
        match self {
            Self::En => format!("{}: {technical_en}", self.error_prefix()),
            Self::PtBr => {
                let human = translate_error_pt_br(technical_en);
                format!("{}: {human}", self.error_prefix())
            }
        }
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

/// Best-effort human translation for common English error messages (stderr only).
///
/// Technical JSON envelopes always keep the English `message` field.
fn translate_error_pt_br(technical_en: &str) -> String {
    // Prefer exact / prefix matches for product messages.
    if technical_en.starts_with("unsupported lang") {
        return technical_en
            .replace("unsupported lang", "idioma não suportado")
            .replace("expected en or pt-BR", "esperado en ou pt-BR");
    }
    if technical_en.starts_with("crate name is empty") {
        return "nome da crate está vazio".into();
    }
    if technical_en.starts_with("crate name exceeds") {
        return technical_en.replace("crate name exceeds", "nome da crate excede");
    }
    if technical_en.starts_with("invalid crate name") {
        return technical_en.replace("invalid crate name", "nome de crate inválido");
    }
    if technical_en.starts_with("search query is empty") {
        return "consulta de busca está vazia".into();
    }
    if technical_en.starts_with("query exceeds") {
        return technical_en.replace("query exceeds", "consulta excede");
    }
    if technical_en.starts_with("item path is empty") {
        return "caminho do item está vazio".into();
    }
    if technical_en.starts_with("item path exceeds") {
        return technical_en.replace("item path exceeds", "caminho do item excede");
    }
    if technical_en.starts_with("item path missing item name") {
        return "caminho do item sem nome do item".into();
    }
    if technical_en.starts_with("invalid item path segment") {
        return technical_en.replace(
            "invalid item path segment",
            "segmento de caminho de item inválido",
        );
    }
    if technical_en.starts_with("item path segment contains whitespace") {
        return technical_en.replace(
            "item path segment contains whitespace",
            "segmento de caminho de item contém espaço em branco",
        );
    }
    if technical_en.starts_with("version exceeds") {
        return technical_en.replace("version exceeds", "versão excede");
    }
    if technical_en.starts_with("invalid version") {
        return technical_en.replace("invalid version", "versão inválida");
    }
    if technical_en.starts_with("unknown item type") {
        return technical_en.replace("unknown item type", "tipo de item desconhecido");
    }
    if technical_en.contains("not found") || technical_en.contains("Not Found") {
        return technical_en
            .replace("not found", "não encontrado")
            .replace("Not Found", "Não encontrado")
            .replace("resource not found", "recurso não encontrado");
    }
    if technical_en.starts_with("rate limited") {
        return technical_en.replace("rate limited by remote", "limitado por taxa no remoto");
    }
    if technical_en.starts_with("wall-clock timeout") {
        return technical_en.replace("wall-clock timeout after", "tempo esgotado após");
    }
    if technical_en.starts_with("response body exceeds max_body_bytes") {
        return technical_en.replace(
            "response body exceeds max_body_bytes",
            "corpo da resposta excede max_body_bytes",
        );
    }
    if technical_en.starts_with("timeout must be >=") {
        return technical_en
            .replace(
                "timeout must be >= 1 second (got 0)",
                "timeout deve ser >= 1 segundo (recebido 0)",
            )
            .replace("timeout must be >=", "timeout deve ser >=");
    }
    if technical_en.starts_with("connect_timeout must be >=") {
        return technical_en.replace(
            "connect_timeout must be >= 1 second (got 0)",
            "connect_timeout deve ser >= 1 segundo (recebido 0)",
        );
    }
    if technical_en.starts_with("interrupted by SIGINT") {
        return "interrompido por SIGINT".into();
    }
    if technical_en.starts_with("terminated by SIGTERM") {
        return "encerrado por SIGTERM".into();
    }
    if technical_en.starts_with("broken pipe") {
        return "pipe quebrado ao escrever em stdout".into();
    }
    if technical_en.starts_with("host not allowlisted") {
        return technical_en.replace("host not allowlisted", "host fora da allowlist");
    }
    if technical_en.starts_with("redirect host not allowlisted") {
        return technical_en.replace(
            "redirect host not allowlisted",
            "host de redirecionamento fora da allowlist",
        );
    }
    if technical_en.starts_with("cannot combine --json") {
        return "não é possível combinar --json com --format text ou --format markdown".into();
    }
    if technical_en.starts_with("unknown schema command") {
        return technical_en.replace("unknown schema command", "comando de schema desconhecido");
    }
    if technical_en.starts_with("bad request from remote") {
        return technical_en.replace("bad request from remote", "requisição inválida do remoto");
    }
    if technical_en.starts_with("remote unavailable") {
        return technical_en.replace("remote unavailable", "remoto indisponível");
    }
    if technical_en.starts_with("unexpected HTTP status") {
        return technical_en.replace("unexpected HTTP status", "status HTTP inesperado");
    }
    if technical_en.starts_with("response body exceeds") {
        return technical_en.replace("response body exceeds", "corpo da resposta excede");
    }
    if technical_en.starts_with("response body is not valid UTF-8") {
        return "corpo da resposta não é UTF-8 válido".into();
    }
    // Fallback: keep English body with localized prefix only.
    technical_en.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(Locale::En.format_error("boom").starts_with("error:"));
        assert!(Locale::PtBr.format_error("boom").starts_with("erro:"));
        let pt = Locale::PtBr.format_error("crate name is empty");
        assert!(pt.contains("nome da crate está vazio"), "{pt}");
        let pt2 = Locale::PtBr.format_error("interrupted by SIGINT");
        assert!(pt2.contains("interrompido por SIGINT"), "{pt2}");
        let pt3 = Locale::PtBr.format_error("invalid version 'v1'");
        assert!(pt3.contains("versão inválida"), "{pt3}");
        assert_eq!(Locale::En.doctor_ok(true), "doctor: ok");
        assert_eq!(Locale::PtBr.doctor_ok(false), "doctor: falhou");
        assert_eq!(Locale::resolve(Some("pt-BR")).unwrap(), Locale::PtBr);
        assert_eq!(Locale::resolve(Some("en")).unwrap(), Locale::En);
        assert!(Locale::resolve(Some("ja")).is_err());
        assert!(Locale::resolve(Some("pt")).is_err());
    }
}
