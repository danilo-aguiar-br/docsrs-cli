//! XDG disk cache for successful HTTP GET bodies between process invocations.
//!
//! Cache key = SHA-256(url || NUL || parser_version || NUL || accept).
//! Each entry stores body bytes plus metadata with body checksum; corrupt or
//! expired entries are discarded and re-fetched.
//!
//! Budget: optional `max_bytes` evicts oldest entries after each successful put.
//! `ttl == 0` means entries never hit (always re-fetch).
//!
//! Poisoned-entry guards: body reads are capped by [`crate::config::HARD_MAX_BODY_BYTES`] (and the
//! cache soft budget); meta JSON reads are capped by [`MAX_CACHE_META_BYTES`]. Both
//! paths use fallible `try_reserve_exact` before filling buffers (never unbounded
//! `fs::read` / `fs::read_to_string` on attacker-controlled sizes).
//!
//! Layout (SRP): `types` · `hex` · `meta` · `paths` · `disk` (all crate-internal).
//! `disk` splits further by operation: `disk::read` (serve or refuse a hit),
//! `disk::store` (durable write), `disk::maintain` (inventory and eviction).

mod disk;
mod hex;
mod meta;
mod paths;
mod types;

pub use disk::DiskCache;
pub use paths::{resolve_cache_dir, resolve_cache_dir_with_source};
pub use types::{
    CACHE_PARSER_VERSION, CacheClearResult, CacheStats, DEFAULT_CACHE_TTL_SECS,
    DEFAULT_MAX_CACHE_BYTES, MAX_CACHE_META_BYTES,
};

#[cfg(test)]
mod tests {
    use super::disk::DiskCache;
    use super::hex::{input_checksum, is_cache_key_hex, key_hex};
    use super::types::{CACHE_PARSER_VERSION, CacheMeta, MAX_CACHE_META_BYTES};
    use crate::http::HttpResponse;
    use bytes::Bytes;
    use reqwest::{StatusCode, Url};
    use std::fs;
    use std::time::Duration;

    fn sample_resp(url: &Url, body: &'static [u8]) -> HttpResponse {
        HttpResponse {
            status: StatusCode::OK,
            final_url: url.clone(),
            body: Bytes::from_static(body),
            content_type: Some("text/html".into()),
            cache_hit: false,
        }
    }

    #[test]
    fn ttl_zero_never_hits() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(0), 0, false);
        let url = Url::parse("https://crates.io/api/v1/crates?q=x").unwrap();
        let resp = sample_resp(&url, b"{}");
        cache.put(&url, "application/json", &resp).unwrap();
        assert!(cache.get(&url, "application/json").is_none());
    }

    #[test]
    fn expired_entry_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(10), 0, false);
        let url = Url::parse("https://crates.io/api/v1/crates?q=x").unwrap();
        let resp = sample_resp(&url, b"{}");
        cache.put(&url, "application/json", &resp).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "application/json");
        let (meta_path, _) = cache.paths_for_key(&key).expect("valid key");
        let mut meta: CacheMeta =
            serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        meta.stored_at_unix = 1;
        fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();
        let short = DiskCache::new(dir.path().to_path_buf(), Duration::from_secs(1), 0, false);
        assert!(short.get(&url, "application/json").is_none());
    }

    #[test]
    fn body_tamper_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/x/1/x/all.html").unwrap();
        let resp = sample_resp(&url, b"original");
        cache.put(&url, "text/html", &resp).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (_, body_path) = cache.paths_for_key(&key).expect("valid key");
        fs::write(&body_path, b"tampered").unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn get_refuses_body_over_max_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let writer = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/big/1/big/index.html").unwrap();
        let body = vec![b'z'; 200];
        writer
            .put(
                &url,
                "text/html",
                &HttpResponse {
                    status: StatusCode::OK,
                    final_url: url.clone(),
                    body: Bytes::from(body),
                    content_type: Some("text/html".into()),
                    cache_hit: false,
                },
            )
            .unwrap();
        let reader = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            50,
            false,
        );
        assert!(reader.get(&url, "text/html").is_none());
    }

    #[test]
    fn get_refuses_body_over_hard_max_even_when_budget_unlimited() {
        use crate::config::HARD_MAX_BODY_BYTES;
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/poison/1/p/index.html").unwrap();
        let tiny = sample_resp(&url, b"ok");
        cache.put(&url, "text/html", &tiny).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (meta_path, body_path) = cache.paths_for_key(&key).expect("valid key");
        let over = (HARD_MAX_BODY_BYTES as usize).saturating_add(1);
        fs::write(&body_path, vec![b'P'; over]).unwrap();
        assert!(meta_path.is_file());
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn get_refuses_meta_over_cap() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/poison-meta/1/p/index.html").unwrap();
        let tiny = sample_resp(&url, b"ok");
        cache.put(&url, "text/html", &tiny).unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (meta_path, _body_path) = cache.paths_for_key(&key).expect("valid key");
        let over = (MAX_CACHE_META_BYTES as usize).saturating_add(1);
        fs::write(&meta_path, vec![b'{'; over]).unwrap();
        assert!(meta_path.is_file());
        assert!(cache.get(&url, "text/html").is_none());
        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
    }

    #[test]
    fn input_checksum_stable() {
        let a = input_checksum("https://docs.rs/a", "1", "text/html");
        let b = input_checksum("https://docs.rs/a", "1", "text/html");
        let c = input_checksum("https://docs.rs/a", "2", "text/html");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(a.len(), 64);
        assert!(is_cache_key_hex(&a));
        assert!(!is_cache_key_hex(".."));
        assert!(!is_cache_key_hex(""));
        assert!(!is_cache_key_hex(&"g".repeat(64)));
    }

    #[test]
    fn non_success_not_stored() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/missing").unwrap();
        let resp = HttpResponse {
            status: StatusCode::NOT_FOUND,
            final_url: url.clone(),
            body: Bytes::from_static(b"nope"),
            content_type: None,
            cache_hit: false,
        };
        cache.put(&url, "text/html", &resp).unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
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
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            700,
            false,
        );
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
                    cache_hit: false,
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
                    cache_hit: false,
                },
            )
            .unwrap();
        let stats = cache.stats();
        assert!(stats.entries <= 1, "stats={stats:?}");
        assert!(stats.total_bytes <= 700, "stats={stats:?}");
        assert!(cache.get(&u2, "text/html").is_some());
        assert!(cache.get(&u1, "text/html").is_none());
    }

    #[test]
    fn max_bytes_skips_body_larger_than_budget() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            100,
            false,
        );
        let url = Url::parse("https://docs.rs/big/1/big/index.html").unwrap();
        let body = vec![b'z'; 500];
        cache
            .put(
                &url,
                "text/html",
                &HttpResponse {
                    status: StatusCode::OK,
                    final_url: url.clone(),
                    body: Bytes::from(body),
                    content_type: Some("text/html".into()),
                    cache_hit: false,
                },
            )
            .unwrap();
        let stats = cache.stats();
        assert_eq!(stats.entries, 0, "stats={stats:?}");
        assert_eq!(stats.total_bytes, 0, "stats={stats:?}");
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn meta_unknown_field_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/extra/1/e/index.html").unwrap();
        cache
            .put(&url, "text/html", &sample_resp(&url, b"ok"))
            .unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (meta_path, _) = cache.paths_for_key(&key).expect("valid key");
        let mut v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        v.as_object_mut()
            .unwrap()
            .insert("evil_extra".into(), serde_json::json!(true));
        fs::write(&meta_path, serde_json::to_string(&v).unwrap()).unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }

    #[test]
    fn meta_bad_digest_shape_is_miss() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(
            dir.path().to_path_buf(),
            Duration::from_secs(3600),
            0,
            false,
        );
        let url = Url::parse("https://docs.rs/baddigest/1/b/index.html").unwrap();
        cache
            .put(&url, "text/html", &sample_resp(&url, b"ok"))
            .unwrap();
        let key = key_hex(url.as_str(), CACHE_PARSER_VERSION, "text/html");
        let (meta_path, _) = cache.paths_for_key(&key).expect("valid key");
        let mut meta: CacheMeta =
            serde_json::from_str(&fs::read_to_string(&meta_path).unwrap()).unwrap();
        meta.body_sha256 = "not-hex".into();
        fs::write(&meta_path, serde_json::to_string(&meta).unwrap()).unwrap();
        assert!(cache.get(&url, "text/html").is_none());
    }
}
