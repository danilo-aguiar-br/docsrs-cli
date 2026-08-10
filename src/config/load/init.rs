//! `config init`: atomic creation of the default `config.toml`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config::constants::{APP_NAME, CONFIG_FILE_NAME};
use crate::config::path_source::{PathSource, resolve_config_dir_with_source};
use crate::error::{AppError, AppResult, ErrorDetail, IoOp, io_at};
use crate::platform::{restrict_private_dir, restrict_private_file};

use super::template::default_config_toml;

/// Result of `config init`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigInitResult {
    /// Absolute path written (or already present when not forced).
    pub path: String,
    /// Directory that holds the file.
    pub config_dir: String,
    /// Layer that resolved the directory this call wrote into.
    ///
    /// Named `target_source` to match `cache-clear`, the other verb that
    /// destroys. It carried the same value under the name `source` while its
    /// sibling said `target_source`, and two names for one concept across two
    /// destructive verbs is the drift this repository keeps finding.
    pub target_source: PathSource,
    /// True when the file was created or overwritten in this call.
    pub created: bool,
    /// True when an existing file was replaced (`--force`).
    pub overwritten: bool,
}

/// Create default `config.toml` under the resolved config directory.
///
/// # Errors
///
/// - [`crate::error::ErrorKind::Config`] when the directory cannot be resolved or created,
///   when the file already exists without `force`, or on I/O failure.
pub fn init_config_toml(
    config_dir_override: Option<PathBuf>,
    force: bool,
) -> AppResult<ConfigInitResult> {
    let (dir, source) = resolve_config_dir_with_source(config_dir_override);
    let dir = dir.ok_or_else(|| AppError::of(ErrorDetail::ConfigDirUnresolved))?;
    fs::create_dir_all(&dir).map_err(io_at(IoOp::CreateDir, &dir))?;
    restrict_private_dir(&dir);

    let path = dir.join(CONFIG_FILE_NAME);
    let existed = path.is_file();
    if existed && !force {
        return Err(AppError::of(ErrorDetail::ConfigAlreadyExists {
            path: path.display().to_string(),
        }));
    }

    // Atomic-ish write: tempfile in same dir then rename.
    let tmp = dir.join(format!("{CONFIG_FILE_NAME}.{APP_NAME}.tmp"));
    let template = default_config_toml();
    {
        let mut f = fs::File::create(&tmp).map_err(io_at(IoOp::CreateTemp, &tmp))?;
        f.write_all(template.as_bytes())
            .map_err(io_at(IoOp::Write, &tmp))?;
        f.sync_all().map_err(io_at(IoOp::Sync, &tmp))?;
    }
    restrict_private_file(&tmp);
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        io_at(IoOp::Install, &path)(e)
    })?;
    restrict_private_file(&path);

    Ok(ConfigInitResult {
        path: path.display().to_string(),
        config_dir: dir.display().to_string(),
        target_source: source,
        created: true,
        overwritten: existed,
    })
}
