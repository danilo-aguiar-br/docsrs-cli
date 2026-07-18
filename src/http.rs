//! HTTP client: rustls, retry, rate limit, body cap, host allowlist, cancel-aware.
//!
//! # Product fetch posture (Rules Rust — web scraping / crawling)
//!
//! This is a **one-shot docs client**, not a general crawler:
//! - **GET-only** product surface against a fixed HTTPS host allowlist.
//! - **One primary request** per command (no multi-URL frontier, no sitemap/RSS).
//! - **User-Agent** identifies `docsrs-cli/{version}` plus optional contact.
//! - **Politeness:** per-host delay floor + additive jitter ([`crate::retry::politeness_delay`]),
//!   in-process clock and cross-process lock+stamp.
//! - **Body budget:** stream with `Content-Length` early reject + `try_reserve*`
//!   (no unbounded `bytes().await`).
//! - **TLS:** rustls only; never `danger_accept_invalid_certs`.
//! - **No cookie jar** (`cookie_store` feature not enabled); no shared cookies.
//! - **Provenance:** HTML payloads expose `source_url` on success envelopes.
//! - **OOS:** robots.txt REP parser, meta robots / X-Robots-Tag, ETag conditional
//!   GETs, anti-bot TLS fingerprinting, headless Chrome, proxy rotation, MinHash.
//!   See `docs/decisions/0003-web-fetch-scope.md`.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **Class:** mixed — async I/O stage on multi-thread Tokio + CPU parse stage
//!   via [`crate::concurrency::ConcurrencyBudget::run_cpu_bound`].
//! - **Shareable:** methods take `&self`. Per-host in-process clock is a
//!   `Mutex<HashMap>` with sleep **outside** the lock (no `Mutex` across `.await`
//!   for the map itself; cross-process flock still owns multi-process throttle).
//! - **Budget:** each client owns a [`ConcurrencyBudget`] for `spawn_blocking`
//!   parse work (and future multi-GET fan-out). Bound is explicit (auto or
//!   `--max-concurrency`).
//! - **Network:** product commands still issue one primary GET at a time; rate
//!   limit + body stream cap remain the I/O backpressure.
//!
//! # Retry (Rules Rust — retry/backoff)
//!
//! Policy lives in [`crate::retry::RetryConfig`] (not inline magic). GET-only
//! product surface ⇒ idempotent retries for 429/5xx/transport only. Kill switch
//! `--disable-retry` / `DOCSRS_CLI_DISABLE_RETRY`. See module docs on `retry`.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use fs4::FileExt as Fs4FileExt;
use fs4::TryLockError as Fs4TryLockError;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, Method, StatusCode, Url};
use tracing::{debug, warn};

use crate::cache::DiskCache;
use crate::concurrency::ConcurrencyBudget;
use crate::config::{
    Config, HARD_MAX_BODY_BYTES, HOST_CRATES_IO, HOST_DOC_RUST_LANG_ORG, HOST_DOCS_RS,
    HOST_STATIC_DOCS_RS,
};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::retry::{
    RetryClass, RetryConfig, classify_http_status, parse_retry_after, politeness_delay,
    wait_for_retry,
};
use crate::shutdown::CancelFlag;

/// HTTPS client bound to product allowlist and config.
///
/// Shareable across tasks (`&self`): rate-limit clock uses a short-lived
/// [`Mutex`] around the map only; sleeps run without holding the lock.
pub struct HttpClient {
    client: Client,
    cfg: Config,
    /// In-process last hit per host. Lock is never held across `.await`.
    last_host_hit: Mutex<HashMap<String, Instant>>,
    cancel: CancelFlag,
    cache: Option<DiskCache>,
    /// Admission gate for CPU-bound parse (and future multi-GET fan-out).
    budget: ConcurrencyBudget,
    /// Explicit retry policy for this client's dependency class.
    retry: RetryConfig,
}

/// Successful HTTP response body and metadata.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// HTTP status code of the final response.
    pub status: StatusCode,
    /// Final URL after redirects.
    pub final_url: Url,
    /// Response body bytes (already capped).
    pub body: Bytes,
    /// Content-Type header when present.
    pub content_type: Option<String>,
}

impl HttpClient {
    /// Build a client with rustls, default headers, and redirect allowlist.
    ///
    /// Owned by one one-shot command. Call sites may `Config::clone` before passing
    /// `cfg` when they still need the parent config after construction. That clone is
    /// intentional and cheap relative to TLS/RTT/body I/O. [`ConcurrencyBudget`] is
    /// built from `cfg.max_concurrency` (0 = auto).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Config`] when the User-Agent is not a valid header value.
    /// Returns [`ErrorKind::Network`] when the reqwest client cannot be built.
    pub fn new(cfg: Config, cancel: CancelFlag) -> AppResult<Self> {
        // Defense in depth: clamp even if a caller skipped Config::clamp_resource_limits.
        let mut cfg = cfg;
        cfg.clamp_resource_limits();
        let budget = ConcurrencyBudget::from_configured(cfg.max_concurrency);
        let retry = RetryConfig::from_config(&cfg);

        let mut headers = HeaderMap::new();
        headers.insert(
            USER_AGENT,
            HeaderValue::from_str(&cfg.user_agent).map_err(|e| {
                AppError::with_source(ErrorKind::Config, "invalid user-agent header", e)
            })?,
        );
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(cfg.timeout())
            .connect_timeout(cfg.connect_timeout())
            .redirect(reqwest::redirect::Policy::custom({
                let max = cfg.max_redirects;
                move |attempt| {
                    if attempt.previous().len() as u32 >= max {
                        return attempt
                            .error(AppError::new(ErrorKind::Network, "redirect limit exceeded"));
                    }
                    // Borrow host_str — no intermediate String on the redirect hot path.
                    let host = attempt.url().host_str().unwrap_or("");
                    let scheme_ok = attempt.url().scheme() == "https";
                    let host_ok = matches!(
                        host,
                        HOST_CRATES_IO
                            | HOST_DOCS_RS
                            | HOST_STATIC_DOCS_RS
                            | HOST_DOC_RUST_LANG_ORG
                    ) || ((host == "127.0.0.1" || host == "localhost")
                        && (cfg!(test) || std::env::var("DOCSRS_CLI_ALLOW_LOCALHOST").is_ok()));
                    if !(scheme_ok && host_ok) {
                        // Materialize the message before moving `attempt` (host is borrowed).
                        let msg = format!("redirect host not allowlisted: {host}");
                        return attempt.error(AppError::new(ErrorKind::Network, msg));
                    }
                    attempt.follow()
                }
            }))
            .use_rustls_tls()
            .build()
            .map_err(|e| {
                AppError::with_source(ErrorKind::Network, "failed to build HTTP client", e)
            })?;

        let cache = if cfg.cache_enabled() {
            cfg.cache_dir
                .as_ref()
                .map(|dir| DiskCache::new(dir.clone(), cfg.cache_ttl(), cfg.max_cache_bytes))
        } else {
            None
        };

        Ok(Self {
            client,
            cfg,
            last_host_hit: Mutex::new(HashMap::new()),
            cancel,
            cache,
            budget,
            retry,
        })
    }

    /// Shared concurrency budget for CPU-bound parse work.
    pub fn budget(&self) -> &ConcurrencyBudget {
        &self.budget
    }

    /// Resolved retry policy (for doctor / tests).
    pub fn retry_policy(&self) -> RetryConfig {
        self.retry
    }

    /// GET expecting JSON (`Accept: application/json`).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Network`] for allowlist, redirect, TLS, or body-cap failures.
    /// Returns [`ErrorKind::Timeout`] when the wall-clock budget expires.
    /// Returns [`ErrorKind::Interrupted`] or [`ErrorKind::Terminated`] on cancel
    /// (kind preserved from the first `CancelFlag::cancel_with`).
    /// Maps HTTP statuses via [`AppError::from_http_status`] (e.g. not-found, rate-limit).
    pub async fn get_json(&self, url: &Url) -> AppResult<HttpResponse> {
        self.request(Method::GET, url, "application/json", true)
            .await
    }

    /// GET expecting HTML (`Accept: text/html`).
    ///
    /// # Errors
    ///
    /// Same error classes as [`Self::get_json`].
    pub async fn get_html(&self, url: &Url) -> AppResult<HttpResponse> {
        self.request(Method::GET, url, "text/html", true).await
    }

    async fn request(
        &self,
        method: Method,
        url: &Url,
        accept: &str,
        retryable: bool,
    ) -> AppResult<HttpResponse> {
        if !is_allowed_host(url) {
            return Err(AppError::new(
                ErrorKind::Network,
                format!("host not allowlisted: {}", url.host_str().unwrap_or("")),
            ));
        }

        // Disk cache only for GET (product surface is GET-only).
        if method == Method::GET
            && let Some(cache) = &self.cache
            && let Some(hit) = cache.get(url, accept)
        {
            return Ok(hit);
        }

        // Policy is opt-out kill switch + max_retries; `retryable` is for future
        // non-idempotent methods (product is GET-only today).
        let policy = if retryable {
            self.retry
        } else {
            RetryConfig {
                enabled: false,
                ..self.retry
            }
        };
        // Prefer `url.as_str()` over `url.clone()`: reqwest implements IntoUrl for
        // `&str` but not for `&Url` (0.12). Method is a small enum — clone is trivial.
        let url_str = url.as_str();
        let mut attempt = 0u32;
        loop {
            self.cancel.check()?;
            attempt += 1;
            self.rate_limit(url).await?;
            self.cancel.check()?;

            let mut req = self.client.request(method.clone(), url_str);
            req = req.header(ACCEPT, accept);

            let result = req.send().await;
            match result {
                Ok(resp) => {
                    let status = resp.status();
                    let final_url = resp.url().clone();
                    let content_type = resp
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    let retry_after = parse_retry_after(resp.headers());
                    let class = classify_http_status(status, retry_after);

                    match class {
                        RetryClass::RateLimited { retry_after } if policy.may_retry(attempt) => {
                            let wait = wait_for_retry(policy, attempt, retry_after);
                            debug!(
                                target: "docsrs_cli::retry",
                                ?wait,
                                attempt,
                                max_attempts = policy.max_attempts(),
                                status = %status,
                                "retrying after rate-limit / Retry-After"
                            );
                            self.sleep_cancelable(wait).await?;
                            continue;
                        }
                        RetryClass::Transient if policy.may_retry(attempt) => {
                            let wait = wait_for_retry(policy, attempt, None);
                            debug!(
                                target: "docsrs_cli::retry",
                                ?wait,
                                attempt,
                                max_attempts = policy.max_attempts(),
                                status = %status,
                                "retrying after transient 5xx"
                            );
                            self.sleep_cancelable(wait).await?;
                            continue;
                        }
                        RetryClass::RateLimited { retry_after } => {
                            let secs = retry_after.map(|d| d.as_secs()).unwrap_or(1);
                            return Err(AppError::from_http_status(
                                status.as_u16(),
                                "retries exhausted",
                            )
                            .with_retry_after(secs.max(1)));
                        }
                        RetryClass::Transient => {
                            // Exhausted retries on 5xx: map status without body read.
                            return Err(AppError::from_http_status(
                                status.as_u16(),
                                "retries exhausted",
                            ));
                        }
                        RetryClass::Permanent => {
                            // 2xx and permanent 4xx: read body and return (caller maps status).
                        }
                    }

                    let body = read_body_capped(resp, self.cfg.max_body_bytes).await?;
                    let out = HttpResponse {
                        status,
                        final_url,
                        body,
                        content_type,
                    };
                    if method == Method::GET
                        && out.status.is_success()
                        && let Some(cache) = &self.cache
                        && let Err(e) = cache.put(url, accept, &out)
                    {
                        warn!(error = %e, "failed to write HTTP cache entry");
                    }
                    return Ok(out);
                }
                Err(e) => {
                    // Classify transport layers without string-matching messages.
                    let is_timeout = e.is_timeout();
                    let is_connect = e.is_connect();
                    let is_request = e.is_request();
                    let kind = if is_timeout {
                        ErrorKind::Timeout
                    } else {
                        ErrorKind::Network
                    };
                    let transport_retryable = is_timeout || is_connect || is_request;
                    if policy.may_retry(attempt) && transport_retryable {
                        let wait = wait_for_retry(policy, attempt, None);
                        warn!(
                            target: "docsrs_cli::retry",
                            ?wait,
                            attempt,
                            max_attempts = policy.max_attempts(),
                            error = %e,
                            is_timeout,
                            is_connect,
                            "retrying after transport error"
                        );
                        self.sleep_cancelable(wait).await?;
                        continue;
                    }
                    return Err(AppError::with_source(
                        kind,
                        format!("HTTP request failed for {url}"),
                        e,
                    ));
                }
            }
        }
    }

    async fn sleep_cancelable(&self, wait: Duration) -> AppResult<()> {
        let step = Duration::from_millis(50);
        let mut remaining = wait;
        while !remaining.is_zero() {
            self.cancel.check()?;
            let slice = remaining.min(step);
            tokio::time::sleep(slice).await;
            remaining = remaining.saturating_sub(slice);
        }
        self.cancel.check()
    }

    /// Enforce per-host delay: in-process clock + exclusive cross-process lock+stamp.
    ///
    /// Floor is `rate_limit_delay_ms`; each wait uses [`politeness_delay`] so the
    /// effective interval is never fixed (additive jitter up to +20%).
    ///
    /// In-process map uses a short [`Mutex`] hold (no sleep under the lock). Cross-process
    /// flock is released on drop before return (never held across unrelated work).
    async fn rate_limit(&self, url: &Url) -> AppResult<()> {
        // Lookup borrows host; allocate only when inserting/updating the clock map.
        let host = url.host_str().unwrap_or("");
        let delay = politeness_delay(self.cfg.rate_limit_delay());
        if delay.is_zero() {
            return Ok(());
        }

        // Prefer exclusive cross-process section when cache_dir is available.
        if let Some(dir) = self.cfg.cache_dir.as_ref() {
            match self.rate_limit_cross_process(dir, host, delay).await {
                Ok(()) => {
                    self.touch_host_clock(host);
                    return Ok(());
                }
                Err(e)
                    if e.kind() == ErrorKind::Terminated || e.kind() == ErrorKind::Interrupted =>
                {
                    return Err(e);
                }
                Err(e) => {
                    // FS without flock or I/O failure: fall back to in-process only.
                    warn!(error = %e, host = %host, "cross-process rate-limit lock failed; using in-process only");
                }
            }
        }

        let in_proc = {
            let map = self
                .last_host_hit
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            map.get(host).and_then(|prev| {
                let elapsed = prev.elapsed();
                if elapsed < delay {
                    Some(delay - elapsed)
                } else {
                    None
                }
            })
        };
        if let Some(d) = in_proc {
            self.sleep_cancelable(d).await?;
        }
        self.touch_host_clock(host);
        Ok(())
    }

    fn touch_host_clock(&self, host: &str) {
        let mut map = self
            .last_host_hit
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        map.insert(host.to_string(), Instant::now());
    }

    /// Exclusive lock on `{cache_dir}/rate-limit/{host}.lock`, then stamp throttle.
    async fn rate_limit_cross_process(
        &self,
        cache_dir: &Path,
        host: &str,
        delay: Duration,
    ) -> AppResult<()> {
        let lock_path = rate_limit_lock_path(cache_dir, host);
        let stamp_path = rate_limit_stamp_path(cache_dir, host);
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                AppError::with_source(ErrorKind::Network, "rate-limit lock dir create failed", e)
            })?;
            crate::platform::restrict_private_dir(parent);
        }

        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| {
                AppError::with_source(ErrorKind::Network, "rate-limit lock open failed", e)
            })?;
        crate::platform::restrict_private_file(&lock_path);

        // Cancel-aware exclusive acquisition (fs4 try_lock + short sleep).
        // Fully-qualified calls avoid collision with std::fs::File::try_lock (Rust 1.89+).
        loop {
            self.cancel.check()?;
            match Fs4FileExt::try_lock(&file) {
                Ok(()) => break,
                Err(Fs4TryLockError::WouldBlock) => {
                    self.sleep_cancelable(Duration::from_millis(50)).await?;
                }
                Err(Fs4TryLockError::Error(e)) => {
                    return Err(AppError::with_source(
                        ErrorKind::Network,
                        "rate-limit exclusive lock failed",
                        e,
                    ));
                }
            }
        }

        // Guard unlock on all exit paths.
        struct UnlockOnDrop<'a>(&'a File);
        impl Drop for UnlockOnDrop<'_> {
            fn drop(&mut self) {
                let _ = Fs4FileExt::unlock(self.0);
            }
        }
        let _guard = UnlockOnDrop(&file);

        let remaining = stamp_remaining(&stamp_path, delay);
        if let Some(d) = remaining {
            self.sleep_cancelable(d).await?;
        }
        write_stamp(&stamp_path)?;
        Ok(())
    }
}

fn safe_host_name(host: &str) -> String {
    let safe: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() {
        "unknown".to_string()
    } else {
        safe
    }
}

/// Path for host rate-limit stamp: `{cache_dir}/rate-limit/{safe_host}.stamp`.
fn rate_limit_stamp_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join("rate-limit")
        .join(format!("{}.stamp", safe_host_name(host)))
}

/// Path for host rate-limit exclusive lock: `{cache_dir}/rate-limit/{safe_host}.lock`.
fn rate_limit_lock_path(cache_dir: &Path, host: &str) -> PathBuf {
    cache_dir
        .join("rate-limit")
        .join(format!("{}.lock", safe_host_name(host)))
}

fn now_unix_ms() -> Option<u128> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis())
}

/// Remaining sleep from stamp file, if any.
fn stamp_remaining(path: &Path, delay: Duration) -> Option<Duration> {
    let text = fs::read_to_string(path).ok()?;
    let last_ms: u128 = text.trim().parse().ok()?;
    let now = now_unix_ms()?;
    if now < last_ms {
        return Some(delay);
    }
    let elapsed = Duration::from_millis((now - last_ms) as u64);
    if elapsed < delay {
        Some(delay - elapsed)
    } else {
        None
    }
}

/// Write stamp under exclusive lock (temp + rename).
fn write_stamp(path: &Path) -> AppResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            AppError::with_source(ErrorKind::Network, "rate-limit stamp dir create failed", e)
        })?;
    }
    let now = now_unix_ms()
        .ok_or_else(|| AppError::new(ErrorKind::Internal, "system clock before UNIX epoch"))?;
    let tmp = path.with_extension("stamp.tmp");
    fs::write(&tmp, now.to_string()).map_err(|e| {
        AppError::with_source(ErrorKind::Network, "rate-limit stamp write failed", e)
    })?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::with_source(ErrorKind::Network, "rate-limit stamp rename failed", e)
    })?;
    crate::platform::restrict_private_file(path);
    Ok(())
}

fn is_allowed_host(url: &Url) -> bool {
    match url.host_str() {
        Some(HOST_CRATES_IO)
        | Some(HOST_DOCS_RS)
        | Some(HOST_STATIC_DOCS_RS)
        | Some(HOST_DOC_RUST_LANG_ORG) => url.scheme() == "https",
        Some("127.0.0.1") | Some("localhost") => {
            cfg!(test) || std::env::var("DOCSRS_CLI_ALLOW_LOCALHOST").is_ok()
        }
        _ => false,
    }
}

/// Read a response body with a hard byte budget and fallible allocation.
///
/// Primary defense is [`HARD_MAX_BODY_BYTES`] (operators may only lower the cap).
/// `try_reserve` / `try_reserve_exact` map allocation failure to
/// [`ErrorKind::Network`] instead of aborting via `with_capacity` on hostile sizes.
/// On Linux overcommit, the allocator may still report success and the OOM killer
/// can fire later — the hard ceiling remains the main bound.
async fn read_body_capped(resp: reqwest::Response, max_bytes: u64) -> AppResult<Bytes> {
    // Never honor a budget above the product hard ceiling (defense in depth).
    let max_bytes = max_bytes.min(HARD_MAX_BODY_BYTES);
    // When Content-Length is known and already over budget, fail without buffering.
    // (With gzip, length may be compressed size or absent; still a useful early guard.)
    let content_length = resp.content_length();
    if let Some(n) = content_length
        && n > max_bytes
    {
        return Err(AppError::new(
            ErrorKind::Network,
            format!("response body exceeds max_body_bytes ({max_bytes})"),
        ));
    }
    // Pre-size when Content-Length is present to avoid realloc churn on large docs pages.
    // Prefer try_reserve_exact when length is known; never with_capacity on external size.
    let capacity = content_length
        .map(|n| (n as usize).min(max_bytes as usize))
        .unwrap_or(0);
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
    if capacity > 0 {
        buf.try_reserve_exact(capacity).map_err(|e| {
            AppError::with_source(
                ErrorKind::Network,
                format!("failed to reserve {capacity} bytes for response body"),
                e,
            )
        })?;
    }
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            AppError::with_source(ErrorKind::Network, "failed reading response body", e)
        })?;
        if (buf.len() as u64).saturating_add(chunk.len() as u64) > max_bytes {
            // Body cap is a transport/resource limit, not user input validation.
            return Err(AppError::new(
                ErrorKind::Network,
                format!("response body exceeds max_body_bytes ({max_bytes})"),
            ));
        }
        // Grow with try_reserve when stream chunks exceed Content-Length estimate.
        let need = chunk.len();
        if buf.capacity().saturating_sub(buf.len()) < need {
            buf.try_reserve(need).map_err(|e| {
                AppError::with_source(
                    ErrorKind::Network,
                    format!("failed to reserve {need} more body bytes"),
                    e,
                )
            })?;
        }
        buf.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buf))
}

/// Decode body as UTF-8, stripping a leading UTF-8 BOM.
///
/// # Errors
///
/// Returns [`ErrorKind::Parse`] when the body is not valid UTF-8.
pub fn decode_utf8(body: &Bytes) -> AppResult<String> {
    let mut bytes = body.as_ref();
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = &bytes[3..];
    }
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|e| AppError::with_source(ErrorKind::Parse, "response body is not valid UTF-8", e))
}

/// ASCII case-insensitive substring check without heap allocation.
///
/// Used on Content-Type sniffing (hot path after every successful GET) so we
/// never build a lowercased temporary `String` for a few-byte header.
fn ascii_contains_ignore_case(haystack: &str, needle: &str) -> bool {
    if needle.is_empty() {
        return true;
    }
    let n = needle.as_bytes();
    haystack
        .as_bytes()
        .windows(n.len())
        .any(|w| w.eq_ignore_ascii_case(n))
}

/// True when Content-Type looks like JSON.
pub fn content_type_looks_json(ct: Option<&str>) -> bool {
    ct.is_some_and(|c| ascii_contains_ignore_case(c, "json"))
}

/// True when Content-Type looks like HTML/XHTML.
pub fn content_type_looks_html(ct: Option<&str>) -> bool {
    ct.is_some_and(|c| {
        ascii_contains_ignore_case(c, "html") || ascii_contains_ignore_case(c, "xhtml")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_https_only() {
        let ok = Url::parse("https://docs.rs/serde").unwrap();
        assert!(is_allowed_host(&ok));
        let stdlib = Url::parse("https://doc.rust-lang.org/stable/std/").unwrap();
        assert!(is_allowed_host(&stdlib));
        let bad = Url::parse("http://docs.rs/serde").unwrap();
        assert!(!is_allowed_host(&bad));
        let evil = Url::parse("https://evil.example/").unwrap();
        assert!(!is_allowed_host(&evil));
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
    fn localhost_allowed_in_unit_tests() {
        let ok = Url::parse("http://127.0.0.1:9/x").unwrap();
        assert!(is_allowed_host(&ok));
        let local = Url::parse("http://localhost:9/x").unwrap();
        assert!(is_allowed_host(&local));
    }

    #[test]
    fn decode_utf8_rejects_invalid() {
        let err = decode_utf8(&Bytes::from(vec![0xff, 0xfe])).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Parse);
    }
}
