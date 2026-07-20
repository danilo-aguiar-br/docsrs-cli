//! Cache root resolution (CLI override / XDG).

use std::path::PathBuf;

/// Resolve cache root: explicit CLI override, then XDG cache home.
///
/// Precedence:
/// 1. Explicit override (`--cache-dir` / caller)
/// 2. `directories::ProjectDirs` cache dir
///
/// Product never reads path knobs from environment variables.
pub fn resolve_cache_dir(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    resolve_cache_dir_with_source(override_dir).0
}

/// Resolve cache root and report which layer won.
pub fn resolve_cache_dir_with_source(
    override_dir: Option<PathBuf>,
) -> (Option<PathBuf>, crate::config::PathSource) {
    use crate::config::PathSource;
    if let Some(p) = override_dir {
        return (Some(p), PathSource::CliFlag);
    }
    if let Some(p) = directories::ProjectDirs::from("", "", crate::config::APP_NAME)
        .map(|d| d.cache_dir().to_path_buf())
    {
        return (Some(p), PathSource::Xdg);
    }
    (None, PathSource::Unresolved)
}
