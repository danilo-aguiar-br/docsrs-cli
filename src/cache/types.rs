//! Public cache inventory types and layout constants.

use serde::{Deserialize, Serialize};

/// Bump when HTML/JSON parse semantics change so stale entries are ignored.
pub const CACHE_PARSER_VERSION: &str = "1";
/// Default entry TTL (24 hours).
pub const DEFAULT_CACHE_TTL_SECS: u64 = 86_400;
/// Default on-disk budget (256 MiB). `0` means unlimited.
pub const DEFAULT_MAX_CACHE_BYTES: u64 = 256 * 1024 * 1024;
/// Hard ceiling for on-disk cache meta JSON (poisoned-entry guard).
///
/// Legitimate `CacheMeta` is a few hundred bytes (URLs + digests). A multi-KiB
/// ceiling leaves headroom while blocking multi-GiB `read_to_string` aborts.
pub const MAX_CACHE_META_BYTES: u64 = 64 * 1024;
/// On-disk layout version under the cache root.
pub(super) const CACHE_LAYOUT: &str = "http/v1";

/// Aggregate cache inventory for `cache stats` / doctor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheStats {
    /// Absolute cache root path.
    pub root: String,
    /// On-disk layout subdirectory (for example `http/v1`).
    pub layout: String,
    /// Number of complete cache entries (meta+body pairs).
    pub entries: u64,
    /// Total bytes used by meta and body files.
    pub total_bytes: u64,
    /// Configured soft budget (`0` = unlimited).
    pub max_bytes: u64,
    /// Configured TTL in seconds.
    pub ttl_secs: u64,
    /// Parser version used for cache keys.
    pub parser_version: String,
}

/// Result of `cache clear`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheClearResult {
    /// Absolute cache root path that was cleared.
    pub root: String,
    /// Number of entries removed.
    pub removed_entries: u64,
    /// Bytes reclaimed from deleted files.
    pub freed_bytes: u64,
}

/// On-disk JSON metadata for one cache entry.
///
/// `deny_unknown_fields`: local meta is product-owned; extra keys mean tamper or
/// schema drift → treat as miss (fail closed at the serde boundary).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct CacheMeta {
    pub(super) url: String,
    pub(super) parser_version: String,
    pub(super) accept: String,
    pub(super) status: u16,
    pub(super) content_type: Option<String>,
    pub(super) final_url: String,
    pub(super) stored_at_unix: u64,
    pub(super) body_sha256: String,
    /// SHA-256 of the cache key material (url + parser + accept).
    pub(super) input_sha256: String,
}

impl CacheMeta {
    /// True when digest fields look like SHA-256 hex (shape check before body I/O).
    pub(super) fn digests_look_valid(&self) -> bool {
        super::hex::is_sha256_hex(&self.body_sha256)
            && super::hex::is_sha256_hex(&self.input_sha256)
    }
}

#[derive(Debug)]
pub(super) struct EntryOnDisk {
    pub(super) key: String,
    pub(super) stored_at_unix: u64,
    pub(super) bytes: u64,
}
