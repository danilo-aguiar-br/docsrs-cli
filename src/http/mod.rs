//! HTTP client: rustls, retry, rate limit, body cap, host allowlist, cancel-aware.
//!
//! # Product fetch posture (Rules Rust — rede + web scraping)
//!
//! This is a **one-shot docs client**, not a general crawler or HTTP server:
//! - **GET-only** product surface against a fixed HTTPS host allowlist (SSRF gate).
//! - **One primary request** per command (no multi-URL frontier, no sitemap/RSS).
//! - **User-Agent** identifies `docsrs-cli/{version}` plus optional contact.
//! - **Politeness:** per-host delay floor + additive jitter ([`crate::retry::politeness_delay`]),
//!   in-process clock and cross-process lock+stamp.
//! - **Body budget:** stream with `Content-Length` early reject + `try_reserve*`
//!   (no unbounded `bytes().await`).
//! - **TLS:** rustls only; provider, trust anchors and version floor are the
//!   constants `content_type::TLS_CRYPTO_PROVIDER`,
//!   `content_type::TLS_TRUST_ANCHORS` and
//!   `content_type::RUSTLS_VERSION_FLOOR`, never repeated as literals here;
//!   min TLS 1.2; never `danger_accept_invalid_certs` (ADR 0007).
//! - **HTTP/2** enabled (feature `http2`); ALPN negotiates with the origin.
//! - **Proxy:** system env `HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`
//!   via reqwest `system-proxy` (allowlist still applies to the **target** URL).
//! - **Sockets:** `TCP_NODELAY` on; keepalive idle/interval/retries named constants.
//! - **Pool:** tiny idle pool sized for one-shot multi-host GETs (not a daemon).
//! - **No cookie jar** (`cookie_store` feature not enabled); no shared cookies.
//! - **Provenance:** HTML payloads expose `source_url` on success envelopes.
//! - **robots.txt:** **PROIBIDO** — never fetch/parse/enforce REP (ADR 0003).
//! - **OOS:** ETag conditional GETs, anti-bot fingerprinting, headless Chrome,
//!   proxy rotation, DoH/DoT custom resolver, server bind/accept, circuit
//!   breaker, mTLS. See `docs/decisions/0003-web-fetch-scope.md`.
//!
//! # Workload classification (Rules Rust — parallelism + memória)
//!
//! - **Class:** mixed — async I/O stage on multi-thread Tokio + CPU parse stage
//!   via [`crate::concurrency::ConcurrencyBudget::run_cpu_bound`].
//! - **Shareable:** methods take `&self`. Per-host in-process clock is a
//!   `Mutex<HashMap>` with sleep **outside** the lock (no `Mutex` across `.await`
//!   for the map itself; cross-process flock still owns multi-process throttle).
//! - **Budget:** each client owns a [`ConcurrencyBudget`](crate::concurrency::ConcurrencyBudget) for `spawn_blocking`
//!   parse work (and future multi-GET fan-out). Bound is explicit (auto or
//!   `--max-concurrency`).
//! - **Network:** product commands still issue one primary GET at a time; rate
//!   limit + body stream cap remain the I/O backpressure.
//!
//! # Retry (Rules Rust — retry/backoff)
//!
//! Policy lives in [`crate::retry::RetryConfig`] (not inline magic). GET-only
//! product surface ⇒ idempotent retries for 408/429/5xx/transport only. Dual
//! budget: `max_attempts` + `max_elapsed_ms`. `Retry-After` accepts delta-seconds
//! and HTTP-date. Kill switch `--disable-retry` / TOML `disable_retry` /
//! `max_retries=0`. See module docs on `retry`.
//!
//! Layout (SRP): `client` · `body` · `content_type` · `allowlist` ·
//! `rate_limit` · `constants` (all crate-internal).

mod allowlist;
mod body;
mod client;
mod constants;
mod content_type;
mod rate_limit;
mod tls;

pub use rate_limit::RATE_LIMIT_DIR_NAME;

pub(crate) use allowlist::is_allowed_host;
pub use body::decode_utf8;
pub use client::{HttpClient, HttpResponse};
pub use constants::{
    POOL_IDLE_TIMEOUT_SECS, POOL_MAX_IDLE_PER_HOST, TCP_KEEPALIVE_IDLE_SECS,
    TCP_KEEPALIVE_INTERVAL_SECS, TCP_KEEPALIVE_RETRIES,
};
pub use content_type::{
    client_posture_detail, content_type_looks_html, content_type_looks_json,
    require_content_type_html, require_content_type_json,
};

#[cfg(test)]
mod tests {
    use super::content_type::ascii_contains_ignore_case;
    use super::*;
    use crate::config::Config;
    use crate::error::ErrorKind;
    use crate::shutdown::CancelFlag;
    use bytes::Bytes;
    use reqwest::Url;
    use std::sync::Once;

    /// reqwest 0.13 `rustls-no-provider` needs a process CryptoProvider (ring; ADR 0007).
    /// Binary installs in `main`; lib unit tests install once here.
    fn ensure_test_provider() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| {
            let _ = rustls::crypto::ring::default_provider().install_default();
        });
    }

    #[test]
    fn allowlist_https_only() {
        let ok = Url::parse("https://docs.rs/serde").unwrap();
        assert!(is_allowed_host(&ok, false));
        let stdlib = Url::parse("https://doc.rust-lang.org/stable/std/").unwrap();
        assert!(is_allowed_host(&stdlib, false));
        let bad = Url::parse("http://docs.rs/serde").unwrap();
        assert!(!is_allowed_host(&bad, false));
        let evil = Url::parse("https://evil.example/").unwrap();
        assert!(!is_allowed_host(&evil, false));
    }

    #[test]
    fn utf8_bom_strip() {
        let mut v = vec![0xEF, 0xBB, 0xBF];
        v.extend_from_slice(b"hello");
        let s = decode_utf8(&Bytes::from(v)).unwrap();
        assert_eq!(s, "hello");
    }

    #[test]
    fn content_types() {
        assert!(content_type_looks_json(Some(
            "application/json; charset=utf-8"
        )));
        assert!(content_type_looks_json(Some("Application/JSON")));
        assert!(content_type_looks_html(Some("text/html")));
        assert!(content_type_looks_html(Some("Text/HTML; charset=utf-8")));
        assert!(content_type_looks_html(Some("application/xhtml+xml")));
        assert!(!content_type_looks_json(Some("text/html")));
        assert!(!content_type_looks_json(None));
        assert!(!content_type_looks_html(None));
        assert!(require_content_type_json(None).is_ok());
        assert!(require_content_type_json(Some("application/json")).is_ok());
        assert!(require_content_type_json(Some("text/html")).is_err());
        assert!(require_content_type_html(None).is_ok());
        assert!(require_content_type_html(Some("text/html")).is_ok());
        assert!(require_content_type_html(Some("application/json")).is_err());
    }

    #[test]
    fn ascii_contains_ignore_case_edges() {
        assert!(ascii_contains_ignore_case("abJSON", "json"));
        assert!(ascii_contains_ignore_case("json", "json"));
        assert!(!ascii_contains_ignore_case("js", "json"));
        assert!(ascii_contains_ignore_case("x", ""));
    }

    #[test]
    fn retry_policy_kill_switch() {
        ensure_test_provider();
        let cfg = Config {
            disable_retry: true,
            max_retries: 5,
            ..Config::default()
        };
        let http = HttpClient::new(cfg, CancelFlag::new()).unwrap();
        assert!(!http.retry_policy().enabled);
        assert_eq!(http.retry_policy().max_attempts(), 1);
    }

    #[test]
    fn invalid_user_agent_rejected() {
        let cfg = Config {
            user_agent: "bad\nua".into(),
            ..Config::default()
        };
        match HttpClient::new(cfg, CancelFlag::new()) {
            Ok(_) => panic!("expected invalid user-agent to fail"),
            Err(err) => assert_eq!(err.kind(), ErrorKind::Config),
        }
    }

    #[test]
    fn localhost_requires_allow_loopback_flag() {
        let ok = Url::parse("http://127.0.0.1:9/x").unwrap();
        assert!(!is_allowed_host(&ok, false));
        assert!(is_allowed_host(&ok, true));
        let local = Url::parse("http://localhost:9/x").unwrap();
        assert!(!is_allowed_host(&local, false));
        assert!(is_allowed_host(&local, true));
    }

    #[test]
    fn decode_utf8_rejects_invalid() {
        let err = decode_utf8(&Bytes::from(vec![0xff, 0xfe])).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Parse);
    }

    #[test]
    fn named_socket_and_pool_constants_are_sane() {
        // Exact product posture (one-shot CLI) — change only with ADR.
        assert_eq!(POOL_MAX_IDLE_PER_HOST, 2);
        assert_eq!(POOL_IDLE_TIMEOUT_SECS, 30);
        assert_eq!(TCP_KEEPALIVE_IDLE_SECS, 60);
        assert_eq!(TCP_KEEPALIVE_INTERVAL_SECS, 10);
        assert_eq!(TCP_KEEPALIVE_RETRIES, 3);
    }

    #[test]
    fn client_posture_mentions_tls_http2_and_proxy() {
        let d = client_posture_detail();
        assert!(d.contains("rustls"));
        assert!(d.contains("provider=ring"));
        // `doctor` must disclose that this binary carries a C build dependency,
        // and why the pure-Rust replacements were rejected. A Rust-native claim
        // that is only true in the source is not auditable from the binary.
        assert!(d.contains("c-build-dep(ring)"));
        assert!(d.contains("pure-rust-blocked"));
        // Read from the constant, not repeated as a literal: asserting the
        // literal here verified the string against a copy of itself and looked
        // like coverage while proving nothing about the manifest pin.
        assert!(d.contains(content_type::RUSTLS_VERSION_FLOOR));
        assert!(d.contains("http2"));
        assert!(d.contains("system-proxy"));
        assert!(d.contains("tcp_nodelay"));
        assert!(d.contains("host-allowlist"));
    }

    #[test]
    fn client_builds_with_network_posture() {
        ensure_test_provider();
        let cfg = Config {
            rate_limit_delay_ms: 0,
            ..Config::default()
        };
        let http = HttpClient::new(cfg, CancelFlag::new()).expect("client builds");
        assert!(http.retry_policy().enabled);
        assert!(http.budget().max() >= 1);
    }
}
