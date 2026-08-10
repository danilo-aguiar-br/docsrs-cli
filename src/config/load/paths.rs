//! `config path`: agent-readable inventory of resolved storage locations.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::constants::CONFIG_FILE_NAME;
use crate::config::path_source::PathSource;

use super::Config;

/// Path inventory for `config path` (agent-readable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigPathData {
    /// Resolved config directory, if any.
    pub config_dir: Option<String>,
    /// Layer that resolved the config directory.
    pub config_source: PathSource,
    /// Absolute `config.toml` path (even if the file does not exist yet).
    pub config_file: Option<String>,
    /// Whether `config.toml` is present on disk.
    pub config_file_exists: bool,
    /// Whether load applied TOML keys (same as exists for normal runs).
    pub config_toml_loaded: bool,
    /// Resolved cache directory, if any.
    pub cache_dir: Option<String>,
    /// Layer that resolved the cache directory.
    pub cache_source: PathSource,
    /// Product does not use `.env` at runtime after install.
    pub dotenv_runtime: bool,
    /// Product stores no API keys; keyring/cloud secret layers are unused.
    pub secrets_layers: &'static str,
}

/// Build path inventory from a loaded [`Config`].
pub fn config_path_data(cfg: &Config) -> ConfigPathData {
    let config_file = cfg.config_file_path();
    let exists = config_file.as_ref().is_some_and(|p| p.is_file());
    ConfigPathData {
        config_dir: cfg.config_dir.as_ref().map(|p| p.display().to_string()),
        config_source: cfg.config_path_source,
        config_file: config_file.as_ref().map(|p| p.display().to_string()),
        config_file_exists: exists,
        config_toml_loaded: cfg.config_toml_loaded,
        cache_dir: cfg.cache_dir.as_ref().map(|p| p.display().to_string()),
        cache_source: cfg.cache_path_source,
        dotenv_runtime: false,
        secrets_layers: "none (public HTTP only; no API keys)",
    }
}

/// Ensure `path` is under a directory (used by tests / path display).
pub fn config_toml_under(dir: &Path) -> PathBuf {
    dir.join(CONFIG_FILE_NAME)
}
