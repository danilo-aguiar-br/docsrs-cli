//! Cache meta JSON read path (size-capped, deny_unknown, digest shape).

use std::fs;
use std::io::Read;
use std::path::Path;

use tracing::debug;

use super::hex::is_sha256_hex;
use super::types::{CacheMeta, MAX_CACHE_META_BYTES};

/// Read and parse a cache meta file with a hard size ceiling and fallible reserve.
///
/// Returns `None` on missing file, oversize, OOM reserve, I/O error, JSON parse
/// failure, unknown fields, or digest shape failure (best-effort cache — miss).
pub(super) fn read_meta_file(path: &Path) -> Option<CacheMeta> {
    let len = fs::metadata(path).ok()?.len();
    if len > MAX_CACHE_META_BYTES {
        debug!(
            path = %path.display(),
            len,
            cap = MAX_CACHE_META_BYTES,
            "cache meta exceeds cap"
        );
        return None;
    }
    let n = usize::try_from(len).ok()?;
    let mut buf = Vec::new();
    if let Err(e) = buf.try_reserve_exact(n) {
        debug!(
            path = %path.display(),
            len,
            error = %e,
            "cache meta reserve failed"
        );
        return None;
    }
    buf.resize(n, 0);
    {
        let mut file = fs::File::open(path).ok()?;
        if file.read_exact(&mut buf).is_err() {
            return None;
        }
    }
    let meta: CacheMeta = serde_json::from_slice(&buf).ok()?;
    if !meta.digests_look_valid() {
        debug!(
            path = %path.display(),
            "cache meta digests are not sha256 hex"
        );
        return None;
    }
    // parser_version is a short product token; refuse absurd lengths early.
    if meta.parser_version.len() > 32 || !meta.parser_version.is_ascii() {
        debug!(path = %path.display(), "cache meta parser_version invalid");
        return None;
    }
    // final_url / url / accept must be non-empty for a meaningful entry.
    if meta.url.is_empty() || meta.final_url.is_empty() || meta.accept.is_empty() {
        debug!(path = %path.display(), "cache meta missing required string fields");
        return None;
    }
    // Defensive: key material digest field already checked via digests_look_valid.
    debug_assert!(is_sha256_hex(&meta.input_sha256));
    Some(meta)
}
