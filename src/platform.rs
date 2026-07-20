//! Cross-platform filesystem and console helpers.
//!
//! Keeps `#[cfg]` conditionals out of business modules. Product code uses
//! [`PathBuf`] / [`Path`] only — no hardcoded `/` or `\\` separators.
//!
//! # Out of scope (agent-first network CLI)
//!
//! - **External process spawn:** product `src/` never uses `std::process::Command`
//!   (or `tokio::process`). Docs/crates.io are fetched via `reqwest` only.
//!   Integration tests may spawn the under-test binary (see `tests/common/`).
//! - Browser/Chrome path discovery (no external browser binary)
//! - Windows console UTF-8 code-page bootstrap via raw Win32 (would require
//!   `unsafe` or a windows-only crate; product has zero `unsafe` in `src/`)
//! - WASM / WASI targets (HTTP client + tokio signal surface is native OS)
//! - Seccomp / landlock / Job Objects (one-shot short-lived process)

use std::path::Path;

/// Restrict a regular file to owner read/write on Unix (`0o600`).
///
/// No-op on non-Unix (ACLs inherit from the parent directory).
/// Best-effort: I/O errors are ignored so a permission failure never
/// aborts a successful cache write path.
pub(crate) fn restrict_private_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Restrict a directory to owner rwx on Unix (`0o700`).
///
/// No-op on non-Unix. Best-effort (see [`restrict_private_file`]).
pub(crate) fn restrict_private_dir(path: &Path) {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o700);
            let _ = fs::set_permissions(path, perms);
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

/// Whether ANSI colors should be used for stderr diagnostics.
///
/// Precedence (first match wins disable):
/// 1. `--no-color` CLI flag
/// 2. `NO_COLOR` present in the environment (any value, including empty)
/// 3. `TERM=dumb`
///
/// Then `CLICOLOR_FORCE` with a non-empty value other than `0` forces colors on.
/// Otherwise colors default to enabled for the tracing fmt layer (historical
/// agent CLI behavior: stderr may be captured without a TTY).
pub(crate) fn ansi_colors_enabled(no_color: bool) -> bool {
    if no_color {
        return false;
    }
    if std::env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if std::env::var_os("TERM").is_some_and(|t| t == "dumb") {
        return false;
    }
    if let Some(v) = std::env::var_os("CLICOLOR_FORCE") {
        if !v.is_empty() && v != "0" {
            return true;
        }
    }
    true
}

/// Compact platform identity for doctor / version diagnostics.
pub(crate) fn platform_detail() -> String {
    format!(
        "{}-{} ({})",
        std::env::consts::ARCH,
        std::env::consts::OS,
        std::env::consts::FAMILY
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn platform_detail_is_non_empty() {
        let d = platform_detail();
        assert!(d.contains(std::env::consts::OS));
        assert!(d.contains(std::env::consts::ARCH));
    }

    #[test]
    fn restrict_private_file_is_noop_or_tightens_unix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("secret.bin");
        fs::write(&path, b"x").unwrap();
        restrict_private_file(&path);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
    }

    #[test]
    fn restrict_private_dir_is_noop_or_tightens_unix() {
        let dir = tempfile::tempdir().unwrap();
        let nested: PathBuf = dir.path().join("nested");
        fs::create_dir_all(&nested).unwrap();
        restrict_private_dir(&nested);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&nested).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o700);
        }
    }

    #[test]
    fn ansi_disabled_when_no_color_flag() {
        assert!(!ansi_colors_enabled(true));
    }
}
