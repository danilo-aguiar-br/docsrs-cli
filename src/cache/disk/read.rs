//! Serving a cached response: every guard that can turn a hit into a miss.
//!
//! A miss is always safe; a wrong hit is not. Every check here answers one
//! question — "could this entry have been produced by the request we are
//! serving?" — and a corrupt or stale entry is evicted on the spot rather than
//! left to fail the same way on the next invocation.

use std::fs;
use std::io::Read;

use bytes::Bytes;
use reqwest::{StatusCode, Url};
use tracing::debug;

use crate::cache::hex::{input_checksum, key_hex, sha256_hex};
use crate::cache::meta::read_meta_file;
use crate::cache::types::{CACHE_PARSER_VERSION, DEFAULT_MAX_CACHE_BYTES};
use crate::config::HARD_MAX_BODY_BYTES;
use crate::http::HttpResponse;

use super::{DiskCache, unix_now};

impl DiskCache {
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
}
