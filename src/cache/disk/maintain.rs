//! Space accounting: inventory, eviction, and wholesale removal.
//!
//! Everything here derives from one scan of the entry directory, so the number
//! `stats` reports and the number eviction acts on can never disagree. The
//! directory itself is the source of truth: a key is only trusted after
//! [`DiskCache::paths_for_key`] accepts it, so a tampered filename is skipped
//! instead of being turned back into a path.

use std::fs;

use tracing::debug;

use crate::cache::meta::read_meta_file;
use crate::cache::types::EntryOnDisk;
use crate::cache::types::{CACHE_LAYOUT, CACHE_PARSER_VERSION, CacheClearResult, CacheStats};
use crate::error::{AppResult, IoOp, io_at};

use super::DiskCache;

impl DiskCache {
    /// Remove every entry under `http/v1` (temps included).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Internal`] when the cache directory cannot
    /// be read.
    pub fn clear(&self) -> AppResult<CacheClearResult> {
        let dir = self.entry_dir();
        let mut removed_entries = 0u64;
        let mut freed_bytes = 0u64;
        if dir.is_dir() {
            for entry in fs::read_dir(&dir).map_err(io_at(IoOp::ReadDir, &dir))? {
                let entry = entry.map_err(io_at(IoOp::Remove, &dir))?;
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
            root: self.root().display().to_string(),
            removed_entries,
            freed_bytes,
        })
    }

    /// Inventory of on-disk entries and budget.
    pub fn stats(&self) -> CacheStats {
        let entries = self.scan_entries();
        let total_bytes = entries.iter().map(|e| e.bytes).sum();
        CacheStats {
            root: self.root().display().to_string(),
            layout: CACHE_LAYOUT.to_string(),
            entries: entries.len() as u64,
            total_bytes,
            max_bytes: self.max_bytes(),
            ttl_secs: self.ttl().as_secs(),
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
    pub(super) fn enforce_max_bytes(&self, keep_key: Option<&str>) {
        if self.max_bytes() == 0 {
            return;
        }
        loop {
            let mut entries = self.scan_entries();
            let total: u64 = entries.iter().map(|e| e.bytes).sum();
            if total <= self.max_bytes() {
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

    pub(super) fn remove_entry(&self, key_hex_str: &str) -> AppResult<()> {
        let Some((meta_path, body_path)) = self.paths_for_key(key_hex_str) else {
            return Ok(());
        };
        let _ = fs::remove_file(meta_path);
        let _ = fs::remove_file(body_path);
        Ok(())
    }
}
