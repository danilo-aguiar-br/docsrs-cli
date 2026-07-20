//! Path resolution layers for config/cache (CLI flags + ProjectDirs only).
//!
//! Product never reads `DOCSRS_CLI_*` path env vars. Precedence:
//! 1. Explicit CLI `--config-dir` / `--cache-dir`
//! 2. `directories::ProjectDirs` (XDG / AppData / Library)

use std::path::PathBuf;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use super::constants::APP_NAME;

/// Which layer won when resolving a storage path (XDG hierarchy diagnostics).
///
/// Product has no API keys; layers for OS keyring / cloud secret managers are
/// out of scope. Precedence for paths: explicit CLI dir override →
/// `ProjectDirs` XDG/AppData/Library. Product knobs are never read from env.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathSource {
    /// `--config-dir` / `--cache-dir` CLI flags.
    #[serde(rename = "cli")]
    CliFlag,
    /// `directories::ProjectDirs` platform defaults.
    #[serde(rename = "xdg")]
    Xdg,
    /// No path could be resolved.
    #[default]
    #[serde(rename = "unresolved")]
    Unresolved,
}

impl PathSource {
    /// Stable machine token for doctor / config path JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliFlag => "cli",
            Self::Xdg => "xdg",
            Self::Unresolved => "unresolved",
        }
    }
}

/// Resolve config directory: CLI override, then XDG.
///
/// Precedence:
/// 1. Explicit override (`--config-dir`)
/// 2. `directories::ProjectDirs` config dir (platform XDG / AppData / Library)
pub fn resolve_config_dir(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    resolve_config_dir_with_source(override_dir).0
}

/// Resolve config directory and report which layer won.
pub fn resolve_config_dir_with_source(
    override_dir: Option<PathBuf>,
) -> (Option<PathBuf>, PathSource) {
    if let Some(p) = override_dir {
        return (Some(p), PathSource::CliFlag);
    }
    if let Some(p) = ProjectDirs::from("", "", APP_NAME).map(|d| d.config_dir().to_path_buf()) {
        return (Some(p), PathSource::Xdg);
    }
    (None, PathSource::Unresolved)
}
