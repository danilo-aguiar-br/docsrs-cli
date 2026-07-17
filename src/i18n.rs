//! Human-facing messages (en / pt-BR). Technical JSON error messages stay English.

/// Resolved UI locale for stderr prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    PtBr,
}

impl Locale {
    /// Detect locale from explicit flag, env, or OS.
    pub fn detect(explicit: Option<&str>) -> Self {
        if let Some(l) = explicit {
            return Self::from_tag(l);
        }
        if let Ok(l) = std::env::var("DOCSRS_CLI_LANG") {
            return Self::from_tag(&l);
        }
        let sys = sys_locale::get_locale().unwrap_or_else(|| "en".into());
        Self::from_tag(&sys)
    }

    fn from_tag(tag: &str) -> Self {
        let t = tag.to_ascii_lowercase();
        if t.starts_with("pt") {
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
            Self::PtBr => format!("{}: {technical_en}", self.error_prefix()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_pt() {
        assert_eq!(Locale::from_tag("pt-BR"), Locale::PtBr);
        assert_eq!(Locale::from_tag("pt"), Locale::PtBr);
        assert_eq!(Locale::from_tag("en-US"), Locale::En);
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
        assert_eq!(Locale::En.doctor_ok(true), "doctor: ok");
        assert_eq!(Locale::PtBr.doctor_ok(false), "doctor: falhou");
        assert_eq!(Locale::detect(Some("pt-BR")), Locale::PtBr);
        assert_eq!(Locale::detect(Some("en")), Locale::En);
    }
}
