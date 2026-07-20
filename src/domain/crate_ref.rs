//! Crate reference with optional `name@version` sugar.

use crate::error::{AppError, AppResult, ErrorKind};

use super::{CrateName, VersionArg};

/// Crate reference with optional `name@version` sugar (R2).
///
/// Accepts plain names (`serde`) or `@`-qualified versions (`clap@4.5.0`,
/// `std@stable`). The `@` form is agent-friendly sugar equivalent to
/// `--crate-version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateRef {
    /// Validated crate name.
    pub name: CrateName,
    /// Version from `@…` when present.
    pub version: Option<VersionArg>,
}

impl CrateRef {
    /// Parse `crate` or `crate@version`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] for empty parts, illegal names, or
    /// invalid version tokens (including a forbidden `v` SemVer prefix).
    ///
    /// # Examples
    ///
    /// ```
    /// use docsrs_cli::domain::CrateRef;
    ///
    /// let plain = CrateRef::parse("tokio").expect("ok");
    /// assert_eq!(plain.name.as_str(), "tokio");
    /// assert!(plain.version.is_none());
    ///
    /// let at = CrateRef::parse("clap@4.5.0").expect("ok");
    /// assert_eq!(at.name.as_str(), "clap");
    /// assert_eq!(at.version.as_ref().unwrap().as_str(), "4.5.0");
    /// ```
    pub fn parse(raw: &str) -> AppResult<Self> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(AppError::new(
                ErrorKind::InvalidInput,
                "crate name is empty",
            ));
        }
        if let Some((name_part, ver_part)) = raw.split_once('@') {
            if name_part.is_empty() {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "crate name is empty before '@'",
                ));
            }
            if ver_part.is_empty() {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "version is empty after '@'",
                ));
            }
            if ver_part.contains('@') {
                return Err(AppError::new(
                    ErrorKind::InvalidInput,
                    "crate reference must contain at most one '@'",
                ));
            }
            let name = CrateName::parse(name_part)?;
            let version = VersionArg::parse(ver_part)?;
            Ok(Self {
                name,
                version: Some(version),
            })
        } else {
            Ok(Self {
                name: CrateName::parse(raw)?,
                version: None,
            })
        }
    }

    /// Merge `@version` sugar with an optional `--crate-version` flag.
    ///
    /// Precedence: when both are present they must agree; otherwise the
    /// non-empty source wins. Missing both → `latest`.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::InvalidInput`] on conflicting versions or invalid flag.
    pub fn into_name_and_version(
        self,
        crate_version_flag: Option<&str>,
    ) -> AppResult<(CrateName, VersionArg)> {
        let flag = crate_version_flag
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match (self.version, flag) {
            (Some(from_at), Some(flag_raw)) => {
                let from_flag = VersionArg::parse(flag_raw)?;
                if from_at.as_str() != from_flag.as_str() {
                    return Err(AppError::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "conflicting versions: crate@{} vs --crate-version {}",
                            from_at.as_str(),
                            from_flag.as_str()
                        ),
                    ));
                }
                Ok((self.name, from_at))
            }
            (Some(from_at), None) => Ok((self.name, from_at)),
            (None, flag_raw) => Ok((self.name, VersionArg::parse_opt(flag_raw)?)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_ref_plain_and_at_version() {
        let p = CrateRef::parse("serde").unwrap();
        assert_eq!(p.name.as_str(), "serde");
        assert!(p.version.is_none());
        let (n, v) = p.into_name_and_version(None).unwrap();
        assert_eq!(n.as_str(), "serde");
        assert_eq!(v.as_str(), "latest");

        let at = CrateRef::parse("clap@4.5.0").unwrap();
        assert_eq!(at.name.as_str(), "clap");
        assert_eq!(at.version.as_ref().unwrap().as_str(), "4.5.0");
        let (n, v) = at.into_name_and_version(None).unwrap();
        assert_eq!(v.as_str(), "4.5.0");
        assert_eq!(n.as_str(), "clap");
    }

    #[test]
    fn crate_ref_merge_and_errors() {
        let at = CrateRef::parse("tokio@1.0.0").unwrap();
        let (n, v) = at.clone().into_name_and_version(Some("1.0.0")).unwrap();
        assert_eq!(n.as_str(), "tokio");
        assert_eq!(v.as_str(), "1.0.0");
        assert!(
            CrateRef::parse("tokio@1.0.0")
                .unwrap()
                .into_name_and_version(Some("2.0.0"))
                .is_err()
        );
        assert!(CrateRef::parse("clap@v4.5.0").is_err());
        assert!(CrateRef::parse("@1.0.0").is_err());
        assert!(CrateRef::parse("clap@").is_err());
        assert!(CrateRef::parse("a@b@c").is_err());
        let plain = CrateRef::parse("once_cell").unwrap();
        let (_, v) = plain.into_name_and_version(Some("1.21.0")).unwrap();
        assert_eq!(v.as_str(), "1.21.0");
    }
}
