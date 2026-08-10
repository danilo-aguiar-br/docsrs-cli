//! Disk cache handle: identity, key derivation, and the clock the entries carry.
//!
//! The three operations a cache performs answer different questions and fail in
//! different ways, so each owns a child module: [`read`] decides whether an entry
//! may be served, [`store`] makes a new entry durable, [`maintain`] keeps the
//! directory inside its budget. What stays here is what all three share — the
//! handle, the key-to-path rule, and `unix_now`.
//!
//! Child modules see the private fields of [`DiskCache`] because privacy in Rust
//! extends downward, so the split costs no widened visibility.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::debug;

use super::hex::is_cache_key_hex;
use super::types::CACHE_LAYOUT;

mod maintain;
mod read;
mod store;

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
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
