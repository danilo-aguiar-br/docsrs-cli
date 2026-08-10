//! Persisting a response: temp file, fsync, rename.
//!
//! The entry is two files that must agree, so neither may become visible before
//! both are durable. Body and meta are written to temporaries, synced, then
//! renamed into place; a RAII guard removes the temporaries on any early return
//! or panic, so an interrupted put leaves no half-entry for `read` to reject.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use reqwest::Url;
use tracing::debug;

use crate::cache::hex::{input_checksum, key_hex, sha256_hex};
use crate::cache::types::{CACHE_PARSER_VERSION, CacheMeta};
use crate::error::{AppError, AppResult, ErrorDetail, InternalOp, IoOp, io_at};
use crate::http::HttpResponse;

use super::{DiskCache, unix_now};

/// Removes leftover temporaries unless disarmed after both renames succeed.
struct TempCleanup {
    paths: [PathBuf; 2],
    armed: bool,
}

impl Drop for TempCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        for p in &self.paths {
            if p.as_os_str().is_empty() {
                continue;
            }
            let _ = fs::remove_file(p);
        }
    }
}

impl DiskCache {
    /// Persist a successful response. Best-effort; failures are non-fatal at call sites.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Internal`] on filesystem create/write/rename
    /// failures or when cache metadata cannot be serialized.
    pub fn put(&self, url: &Url, accept: &str, resp: &HttpResponse) -> AppResult<()> {
        if !resp.status.is_success() {
            return Ok(());
        }
        // Soft budget: never store a single body that already exceeds the cap.
        if self.max_bytes > 0 && (resp.body.len() as u64) > self.max_bytes {
            debug!(
                body = resp.body.len(),
                max = self.max_bytes,
                "cache skip: body exceeds max_bytes"
            );
            return Ok(());
        }
        let dir = self.entry_dir();
        fs::create_dir_all(&dir).map_err(io_at(IoOp::CreateDir, &dir))?;
        crate::platform::restrict_private_dir(&dir);
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, accept);
        let input_sha = input_checksum(url.as_str(), CACHE_PARSER_VERSION, accept);
        let body_sha = sha256_hex(resp.body.as_ref());
        let meta = CacheMeta {
            url: url.as_str().to_string(),
            parser_version: CACHE_PARSER_VERSION.to_string(),
            accept: accept.to_string(),
            status: resp.status.as_u16(),
            content_type: resp.content_type.clone(),
            final_url: resp.final_url.as_str().to_string(),
            stored_at_unix: unix_now(),
            body_sha256: body_sha,
            input_sha256: input_sha,
        };
        let (meta_path, body_path) = self.paths_for_key(&key).ok_or_else(|| {
            AppError::of(ErrorDetail::Internal {
                op: InternalOp::CacheKeyMalformed,
            })
        })?;
        let meta_tmp = meta_path.with_extension("meta.json.tmp");
        let body_tmp = body_path.with_extension("bin.tmp");

        // RAII: remove temps on any early return / panic; disarmed after successful renames.
        let mut temps = TempCleanup {
            paths: [body_tmp, meta_tmp],
            armed: true,
        };

        {
            let tmp = temps.paths[0].clone();
            let mut f = fs::File::create(&tmp).map_err(io_at(IoOp::CreateTemp, &tmp))?;
            f.write_all(resp.body.as_ref())
                .map_err(io_at(IoOp::Write, &tmp))?;
            f.sync_all().map_err(io_at(IoOp::Sync, &tmp))?;
        }
        {
            let text = serde_json::to_string(&meta).map_err(|e| {
                AppError::of_with_source(
                    ErrorDetail::Internal {
                        op: InternalOp::JsonSerialize,
                    },
                    e,
                )
            })?;
            let tmp = temps.paths[1].clone();
            let mut f = fs::File::create(&tmp).map_err(io_at(IoOp::CreateTemp, &tmp))?;
            f.write_all(text.as_bytes())
                .map_err(io_at(IoOp::Write, &tmp))?;
            f.sync_all().map_err(io_at(IoOp::Sync, &tmp))?;
        }

        fs::rename(&temps.paths[0], &body_path).map_err(io_at(IoOp::Rename, &body_path))?;
        crate::platform::restrict_private_file(&body_path);
        temps.paths[0] = PathBuf::new();
        fs::rename(&temps.paths[1], &meta_path).map_err(io_at(IoOp::Rename, &meta_path))?;
        crate::platform::restrict_private_file(&meta_path);
        temps.armed = false;
        debug!(%key, "cache store");
        self.enforce_max_bytes(Some(&key));
        Ok(())
    }
}
