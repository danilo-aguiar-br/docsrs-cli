//! XDG disk cache for successful HTTP GET bodies between process invocations.
//!
//! Cache key = SHA-256(url || NUL || parser_version || NUL || accept).
//! Each entry stores body bytes plus metadata with body checksum; corrupt or
//! expired entries are discarded and re-fetched.
//!
//! Budget: optional `max_bytes` evicts oldest entries after each successful put.
//! `ttl == 0` means entries never hit (always re-fetch).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use reqwest::{StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::debug;

use crate::error::{AppError, AppResult, ErrorKind};
use crate::http::HttpResponse;

/// Bump when HTML/JSON parse semantics change so stale entries are ignored.
pub const CACHE_PARSER_VERSION: &str = "1";
/// Default entry TTL (24 hours).
pub const DEFAULT_CACHE_TTL_SECS: u64 = 86_400;
/// Default on-disk budget (256 MiB). `0` means unlimited.
pub const DEFAULT_MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;
/// On-disk layout version under the cache root.
const CACHE_LAYOUT: &str = "http/v1";

/// Disk cache rooted at an XDG (or override) directory.
#[derive(Debug, Clone)]
pub struct DiskCache {
    root: PathBuf,
    ttl: Duration,
    /// Soft cap on total body+meta bytes under `http/v1`. `0` = unlimited.
    max_bytes: u64,
}

/// Aggregate cache inventory for `cache stats` / doctor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheStats {
    pub root: String,
    pub layout: String,
    pub entries: u64,
    pub total_bytes: u64,
    pub max_bytes: u64,
    pub ttl_secs: u64,
    pub parser_version: String,
}

/// Result of `cache clear`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheClearResult {
    pub root: String,
    pub removed_entries: u64,
    pub freed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheMeta {
    url: String,
    parser_version: String,
    accept: String,
    status: u16,
    content_type: Option<String>,
    final_url: String,
    stored_at_unix: u64,
    body_sha256: String,
    /// SHA-256 of the cache key material (url + parser + accept).
    input_sha256: String,
}

#[derive(Debug)]
struct EntryOnDisk {
    key: String,
    stored_at_unix: u64,
    bytes: u64,
}

impl DiskCache {
    /// Create a cache at `root` with the given TTL and byte budget.
    pub fn new(root: PathBuf, ttl: Duration, max_bytes: u64) -> Self {
        Self {
            root,
            ttl,
            max_bytes,
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn ttl(&self) -> Duration {
        self.ttl
    }

    pub fn max_bytes(&self) -> u64 {
        self.max_bytes
    }

    fn entry_dir(&self) -> PathBuf {
        self.root.join(CACHE_LAYOUT)
    }

    fn paths_for_key(&self, key_hex: &str) -> (PathBuf, PathBuf) {
        let dir = self.entry_dir();
        (
            dir.join(format!("{key_hex}.meta.json")),
            dir.join(format!("{key_hex}.bin")),
        )
    }

    /// Look up a successful GET response. Returns `None` on miss, expire, or corrupt entry.
    pub fn get(&self, url: &Url, accept: &str) -> Option<HttpResponse> {
        // TTL 0 = never serve from disk (always re-fetch).
        if self.ttl.as_secs() == 0 {
            return None;
        }
        let input_sha = input_checksum(url.as_str(), CACHE_PARSER_VERSION, accept);
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, accept);
        let (meta_path, body_path) = self.paths_for_key(&key);
        if !meta_path.is_file() || !body_path.is_file() {
            return None;
        }
        let meta_text = fs::read_to_string(&meta_path).ok()?;
        let meta: CacheMeta = serde_json::from_str(&meta_text).ok()?;
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
        let body = fs::read(&body_path).ok()?;
        let body_sha = sha256_hex(&body);
        if body_sha != meta.body_sha256 {
            debug!(%key, "cache miss: body checksum mismatch");
            let _ = self.remove_entry(&key);
            return None;
        }
        let final_url = Url::parse(&meta.final_url).ok()?;
        let status = StatusCode::from_u16(meta.status).ok()?;
        debug!(%key, "cache hit");
        Some(HttpResponse {
            status,
            final_url,
            body: Bytes::from(body),
            content_type: meta.content_type,
        })
    }

    /// Persist a successful response. Best-effort; failures are non-fatal at call sites.
    pub fn put(&self, url: &Url, accept: &str, resp: &HttpResponse) -> AppResult<()> {
        if !resp.status.is_success() {
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
        let (meta_path, body_path) = self.paths_for_key(&key);
        let meta_tmp = meta_path.with_extension("meta.json.tmp");
        let body_tmp = body_path.with_extension("bin.tmp");

        {
            let mut f = fs::File::create(&body_tmp).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache body temp create", e)
            })?;
            f.write_all(resp.body.as_ref())
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body write", e))?;
            f.sync_all()
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body sync", e))?;
        }
        {
            let text = serde_json::to_string_pretty(&meta).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache meta serialize", e)
            })?;
            let mut f = fs::File::create(&meta_tmp).map_err(|e| {
                AppError::with_source(ErrorKind::Internal, "cache meta temp create", e)
            })?;
            f.write_all(text.as_bytes())
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache meta write", e))?;
            f.sync_all()
                .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache meta sync", e))?;
        }

        fs::rename(&body_tmp, &body_path)
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache body rename", e))?;
        fs::rename(&meta_tmp, &meta_path)
            .map_err(|e| AppError::with_source(ErrorKind::Internal, "cache meta rename", e))?;
        debug!(%key, "cache store");
        self.enforce_max_bytes(Some(&key));
        Ok(())
    }

    /// Remove every entry under `http/v1` (temps included).
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
                    // Count complete pairs by .meta.json files.
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
            let (meta_path, body_path) = self.paths_for_key(key);
            if !meta_path.is_file() || !body_path.is_file() {
                continue;
            }
            let meta_len = fs::metadata(&meta_path).map(|m| m.len()).unwrap_or(0);
            let body_len = fs::metadata(&body_path).map(|m| m.len()).unwrap_or(0);
            let stored_at = fs::read_to_string(&meta_path)
                .ok()
                .and_then(|t| serde_json::from_str::<CacheMeta>(&t).ok())
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
    /// Never deletes `keep_key` (the entry just written): a single large body
    /// may temporarily exceed the soft budget.
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
                // Only keep_key remains (or empty); stop even if still over budget.
                break;
            };
            debug!(key = %v.key, "cache evict for budget");
            let _ = self.remove_entry(&v.key);
        }
    }

    fn remove_entry(&self, key_hex: &str) -> AppResult<()> {
        let (meta_path, body_path) = self.paths_for_key(key_hex);
        let _ = fs::remove_file(meta_path);
        let _ = fs::remove_file(body_path);
        Ok(())
    }
}

/// Resolve cache root: explicit override, env, then XDG cache home.
pub fn resolve_cache_dir(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    override_dir
        .or_else(|| std::env::var_os("DOCSRS_CLI_CACHE_DIR").map(PathBuf::from))
        .or_else(|| {
            directories::ProjectDirs::from("", "", crate::config::APP_NAME)
                .map(|p| p.cache_dir().to_path_buf())
        })
}

fn key_hex(url: &str, parser_version: &str, accept: &str) -> String {
    input_checksum(url, parser_version, accept)
}

fn input_checksum(url: &str, parser_version: &str, accept: &str) -> String {
    let mut h = Sha256::new();
    h.update(url.as_bytes());
    h.update([0u8]);
    h.update(parser_version.as_bytes());
    h.update([0u8]);
    h.update(accept.as_bytes());
    hex_encode(h.finalize())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex_encode(h.finalize())
}

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_resp(url: &Url, body: &'static [u8]) -> HttpResponse {
        HttpResponse {
            status: StatusCode::OK,
            final_url: url.clone(),
            body: Bytes::from_static(body),
            content_type: Some("text/html".into()),
        }
    }

    #[test]
    fn put_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(3600), 0);
        let url = Url::parse("https://docs.rs/serde/latest/serde/index.html").unwrap();
        let resp = sample_resp(&url, b"<html>ok</html>");
        cache.put(&url, "text/html", &resp).unwrap();
        let hit = cache.get(&url, "text/html").expect("cache hit");
        assert_eq!(hit.body.as_ref(), b"<html>ok</html>");
        assert_eq!(hit.status, StatusCode::OK);
    }

    #[test]
    fn ttl_zero_never_hits() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(0), 0);
        let url = Url::parse("https://crates.io/api/v1/crates?q=x").unwrap();
        let resp = sample_resp(&url, b"{}");
        cache.put(&url, "application/json", &resp).unwrap();
        assert!(cache.get(&url, "application/json").is_none());
    }

    #[test]
    fn expired_entry_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(10), 0);
        let url = Url::parse("https://crates.io/api/v1/crates?q=x").unwrap();
        let resp = sample_resp(&url, b"{}");
        cache.put(&url, "application/json", &resp).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "application/json");
        let (meta_path, _) = cache.paths_for_key(&key);
        let mut meta: CacheMeta =
            serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        meta.stored_at_unix = 1;
        fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();
        let short = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(1), 0);
        assert!(short.get(&url, "application/json").is_none());
    }

    #[test]
    fn body_tamper_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(3600), 0);
        let url = Url::parse("https://docs.rs/x/1/x/all.html").unwrap();
        let resp = sample_resp(&url, b"original");
        cache.put(&url, "text/html", &resp).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (_, body_path) = cache.paths_for_key(&key);
        fs::write(&body_path, b"tampered").unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn input_checksum_stable() {
        let a = input_checksum("https://docs.rs/a", "1", "text/html");
        let b = input_checksum("https://docs.rs/a", "1", "text/html");
        let c = input_checksum("https://docs.rs/a", "2", "text/html");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn non_success_not_stored() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(3600), 0);
        let url = Url::parse("https://docs.rs/missing").unwrap();
        let resp = HttpResponse {
            status: StatusCode::NOT_FOUND,
            final_url: url.clone(),
            body: Bytes::from_static(b"nope"),
            content_type: None,
        };
        cache.put(&url, "text/html", &resp).unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(3600), 0);
        for (i, body) in [(1, b"a" as &[u8]), (2, b"bb"), (3, b"ccc")] {
            let url = Url::parse(&format!("https://docs.rs/c{i}/1/c{i}/index.html")).unwrap();
            cache
                .put(&url, "text/html", &sample_resp(&url, body))
                .unwrap();
        }
        let before = cache.stats();
        assert_eq!(before.entries, 3);
        assert!(before.total_bytes > 0);
        let cleared = cache.clear().unwrap();
        assert_eq!(cleared.removed_entries, 3);
        assert!(cleared.freed_bytes > 0);
        let after = cache.stats();
        assert_eq!(after.entries, 0);
        assert_eq!(after.total_bytes, 0);
    }

    #[test]
    fn max_bytes_evicts_oldest() {
        let dir = tempfile::tempdir().unwrap();
        // Budget fits one ~200B body+meta pair but not two.
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(3600), 700);
        let u1 = Url::parse("https://docs.rs/a/1/a/index.html").unwrap();
        let u2 = Url::parse("https://docs.rs/b/1/b/index.html").unwrap();
        let body1 = vec![b'x'; 200];
        let body2 = vec![b'y'; 200];
        cache
            .put(
                &u1,
                "text/html",
                &HttpResponse {
                    status: StatusCode::OK,
                    final_url: u1.clone(),
                    body: Bytes::from(body1),
                    content_type: Some("text/html".into()),
                },
            )
            .unwrap();
        std::thread::sleep(Duration::from_millis(20));
        cache
            .put(
                &u2,
                "text/html",
                &HttpResponse {
                    status: StatusCode::OK,
                    final_url: u2.clone(),
                    body: Bytes::from(body2),
                    content_type: Some("text/html".into()),
                },
            )
            .unwrap();
        let stats = cache.stats();
        assert!(stats.entries <= 1, "stats={stats:?}");
        assert!(stats.total_bytes <= 700, "stats={stats:?}");
        // Newest entry is preserved; oldest was evicted for budget.
        assert!(cache.get(&u2, "text/html").is_some());
        assert!(cache.get(&u1, "text/html").is_none());
    }
}
