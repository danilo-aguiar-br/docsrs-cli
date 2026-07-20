//! Disk cache get/put/clear/stats/evict.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use reqwest::{StatusCode, Url};
use tracing::debug;

use crate::config::HARD_MAX_BODY_BYTES;
use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::HttpResponse;

use super::hex::{input_checksum, is_cache_key_hex, key_hex, sha256_hex};
use super::meta::read_meta_file;
use super::types::{
    CACHE_LAYOUT, CACHE_PARSER_VERSION, CacheClearResult, CacheMeta, CacheStats,
    DEFAULT_MAX_CACHE_BYTES, EntryOnDisk,
};

/// Disk cache rooted at an XDG (or override) directory.
#[derive(Debug, Clone)]
pub struct DiskCache {
    root: PathBuf,
    ttl: Duration,
    /// Soft cap on total body+meta bytes under `http/v1`. `0` = unlimited.
    max_bytes: u64,
    /// Same SSRF loopback policy as the owning [`crate::config::Config`].
    allow_loopback: bool,
}

impl DiskCache {
    /// Create a cache at `root` with the given TTL, byte budget, and loopback policy.
    pub fn new(root: PathBuf, ttl: Duration, max_bytes: u64, allow_loopback: bool) -> Self {
        Self {
            root,
            ttl,
            max_bytes,
            allow_loopback,
        }
    }

    /// Absolute cache root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Configured entry time-to-live.
    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    /// Soft size budget in bytes (`0` = unlimited).
    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    fn entry_dir(&self) -> PathBuf {
        self.root.join(CACHE_LAYOUT)
    }

    /// Build meta/body paths only for SHA-256 hex keys (64 lowercase hex digits).
    ///
    /// Refuses path traversal / odd filenames that could appear if on-disk names
    /// are tampered (scan_entries reads directory entries as keys).
    pub(super) fn paths_for_key(&self, key_hex_str: &str) -> Option<(PathBuf, PathBuf)> {
        if !is_cache_key_hex(key_hex_str) {
            debug!(%key_hex_str, "refuse cache path: key is not sha256 hex");
            return None;
        }
        let dir = self.entry_dir();
        Some((
            dir.join(format!("{key_hex_str}.meta.json")),
            dir.join(format!("{key_hex_str}.bin")),
        ))
    }

    /// Look up a successful GET response. Returns `None` on miss, expire, or corrupt entry.
    pub fn get(&self, url: &Url, accept: &str) -> Option<HttpResponse> {
        // TTL 0 = never serve from disk (always re-fetch).
        if self.ttl.as_secs() == 0 {
            return None;
        }
        let input_sha = input_checksum(url.as_str(), CACHE_PARSER_VERSION, accept);
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, accept);
        let (meta_path, body_path) = self.paths_for_key(&key)?;
        if !meta_path.is_file() || !body_path.is_file() {
            return None;
        }
        let Some(meta) = read_meta_file(&meta_path) else {
            debug!(%key, "cache miss: meta unreadable or over cap");
            return None;
        };
        // Best-effort eviction on miss paths: cleanup IO must not turn a cache miss
        // into a hard error (get returns Option; put/clear use AppResult).
        if meta.parser_version != CACHE_PARSER_VERSION {
            debug!(%key, "cache miss: parser version mismatch");
            let _ = self.remove_entry(&key);
            return None;
        }
        if meta.url != url.as_str() || meta.accept != accept {
            debug!(%key, "cache miss: url/accept mismatch");
            let _ = self.remove_entry(&key);
            return None;
        }
        if meta.input_sha256 != input_sha {
            debug!(%key, "cache miss: input checksum mismatch");
            let _ = self.remove_entry(&key);
            return None;
        }
        let now = unix_now();
        if now.saturating_sub(meta.stored_at_unix) >= self.ttl.as_secs() {
            debug!(%key, "cache miss: expired");
            let _ = self.remove_entry(&key);
            return None;
        }
        // Refuse oversized on-disk bodies before allocating (OOM / corrupt entry guard).
        let body_len = fs::metadata(&body_path).ok()?.len();
        let budget = if self.max_bytes > 0 {
            self.max_bytes
        } else {
            DEFAULT_MAX_CACHE_BYTES
        };
        let read_cap = budget.min(HARD_MAX_BODY_BYTES);
        if body_len > read_cap {
            debug!(%key, body_len, read_cap, "cache miss: body exceeds read budget");
            let _ = self.remove_entry(&key);
            return None;
        }
        let n = usize::try_from(body_len).ok()?;
        let mut buf = Vec::new();
        if let Err(e) = buf.try_reserve_exact(n) {
            debug!(%key, body_len, error = %e, "cache miss: failed to reserve body buffer");
            return None;
        }
        buf.resize(n, 0);
        {
            let mut file = fs::File::open(&body_path).ok()?;
            if file.read_exact(&mut buf).is_err() {
                debug!(%key, "cache miss: body read failed");
                return None;
            }
        }
        let body_sha = sha256_hex(&buf);
        if body_sha != meta.body_sha256 {
            debug!(%key, "cache miss: body checksum mismatch");
            let _ = self.remove_entry(&key);
            return None;
        }
        let final_url = Url::parse(&meta.final_url).ok()?;
        // Poisoned meta could claim an off-allowlist final_url; refuse the hit.
        if !crate::http::is_allowed_host(&final_url, self.allow_loopback) {
            debug!(%key, final = %final_url, "cache miss: final_url not allowlisted");
            let _ = self.remove_entry(&key);
            return None;
        }
        let status = StatusCode::from_u16(meta.status).ok()?;
        debug!(%key, "cache hit");
        Some(HttpResponse {
            status,
            final_url,
            body: Bytes::from(buf),
            content_type: meta.content_type,
            cache_hit: true,
        })
    }

    /// Persist a successful response. Best-effort; failures are non-fatal at call sites.
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Internal`] on filesystem create/write/rename failures or
    /// when cache metadata cannot be serialized.
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
        fs::create_dir_all(&dir).map_err(|e| {
            AppError::with_source(
                ErrorKind::Internal,
                format!("failed to create cache dir {}", dir.display()),
                e,
            )
        })?;
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
            AppError::new(
                ErrorKind::Internal,
                "cache key is not valid sha256 hex (internal invariant broken)",
            )
        })?;
        let meta_tmp = meta_path.with_extension("meta.json.tmp");
        let body_tmp = body_path.with_extension("bin.tmp");

        // RAII: remove temps on any early return / panic; disarmed after successful renames.
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
        let mut temps = TempCleanup {
            paths: [body_tmp, meta_tmp],
            armed: true,
        };

        {
            let mut f = fs::File::create(&temps.paths[0]).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache body temp create", e)
            })?;
            f.write_all(resp.body.as_ref())
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body write", e))?;
            f.sync_all()
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body sync", e))?;
        }
        {
            let text = serde_json::to_string(&meta).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache meta serialize", e)
            })?;
            let mut f = fs::File::create(&temps.paths[1]).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache meta temp create", e)
            })?;
            f.write_all(text.as_bytes())
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache meta write", e))?;
            f.sync_all()
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache meta sync", e))?;
        }

        fs::rename(&temps.paths[0], &body_path)
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body rename", e))?;
        crate::platform::restrict_private_file(&body_path);
        temps.paths[0] = PathBuf::new();
        fs::rename(&temps.paths[1], &meta_path).map_err(|e| {
            AppError::with_source(ErrorKind::Internal, "cache meta rename", e)
        })?;
        crate::platform::restrict_private_file(&meta_path);
        temps.armed = false;
        debug!(%key, "cache store");
        self.enforce_max_bytes(Some(&key));
        Ok(())
    }

    /// Remove every entry under `http/v1` (temps included).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Internal`] when the cache directory cannot be read.
    pub fn clear(&self) -> AppResult<CacheClearResult> {
        let dir = self.entry_dir();
        let mut removed_entries = 0u64;
        let mut freed_bytes = 0u64;
        if dir.is_dir() {
            for entry in fs::read_dir(&dir).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache clear read_dir", e)
            })? {
                let entry = entry.map_err(|e| {
                    AppError::with_source(ErrorKind::Internal, "cache clear entry", e)
                })?;
                let path = entry.path();
                let len = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if path.is_file() {
                    if path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(".meta.json"))
                    {
                        removed_entries = removed_entries.saturating_add(1);
                    }
                    freed_bytes = freed_bytes.saturating_add(len);
                    let _ = fs::remove_file(&path);
                }
            }
        }
        Ok(CacheClearResult {
            root: self.root.display().to_string(),
            removed_entries,
            freed_bytes,
        })
    }

    /// Inventory of on-disk entries and budget.
    pub fn stats(&self) -> CacheStats {
        let entries = self.scan_entries();
        let total_bytes = entries.iter().map(|e| e.bytes).sum();
        CacheStats {
            root: self.root.display().to_string(),
            layout: CACHE_LAYOUT.to_string(),
            entries: entries.len() as u64,
            total_bytes,
            max_bytes: self.max_bytes,
            ttl_secs: self.ttl.as_secs(),
            parser_version: CACHE_PARSER_VERSION.to_string(),
        }
    }

    fn scan_entries(&self) -> Vec<EntryOnDisk> {
        let dir = self.entry_dir();
        let mut out = Vec::new();
        let Ok(rd) = fs::read_dir(&dir) else {
            return out;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(key) = name.strip_suffix(".meta.json") else {
                continue;
            };
            let Some((meta_path, body_path)) = self.paths_for_key(key) else {
                continue;
            };
            if !meta_path.is_file() || !body_path.is_file() {
                continue;
            }
            let meta_len = fs::metadata(&meta_path).map(|m| m.len()).unwrap_or(0);
            let body_len = fs::metadata(&body_path).map(|m| m.len()).unwrap_or(0);
            let stored_at = read_meta_file(&meta_path)
                .map(|m| m.stored_at_unix)
                .unwrap_or(0);
            out.push(EntryOnDisk {
                key: key.to_string(),
                stored_at_unix: stored_at,
                bytes: meta_len.saturating_add(body_len),
            });
        }
        out
    }

    /// Evict oldest entries until total size is within `max_bytes`.
    /// Never deletes `keep_key` (the entry just written).
    fn enforce_max_bytes(&self, keep_key: Option<&str>) {
        if self.max_bytes == 0 {
            return;
        }
        loop {
            let mut entries = self.scan_entries();
            let total: u64 = entries.iter().map(|e| e.bytes).sum();
            if total <= self.max_bytes {
                break;
            }
            entries.sort_by_key(|e| e.stored_at_unix);
            let victim = entries
                .iter()
                .find(|e| keep_key.is_none_or(|k| k != e.key.as_str()));
            let Some(v) = victim else {
                break;
            };
            debug!(key = %v.key, "cache evict for budget");
            let _ = self.remove_entry(&v.key);
        }
    }

    fn remove_entry(&self, key_hex_str: &str) -> AppResult<()> {
        let Some((meta_path, body_path)) = self.paths_for_key(key_hex_str) else {
            return Ok(());
        };
        let _ = fs::remove_file(meta_path);
        let _ = fs::remove_file(body_path);
        Ok(())
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
