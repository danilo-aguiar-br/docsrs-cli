//! HTTPS client: rustls, retry, rate limit, body cap, host allowlist, cancel-aware.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bytes::Bytes;
use fs4::FileExt as Fs4FileExt;
use fs4::TryLockError as Fs4TryLockError;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, Method, StatusCode, Url};
use tracing::{debug, info_span, warn, Instrument};

use crate::cache::DiskCache;
use crate::concurrency::ConcurrencyBudget;
use crate::config::{CANCEL_POLL_INTERVAL_MS, Config, HARD_MAX_BODY_BYTES};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::retry::{
    ErrorLayer, RetryClass, RetryConfig, classify_http_status, classify_reqwest_error,
    parse_retry_after, politeness_delay, wait_for_retry,
};
use crate::shutdown::CancelFlag;

use super::allowlist::is_allowed_host;
use super::body::read_body_capped;
use super::constants::{
    POOL_IDLE_TIMEOUT_SECS, POOL_MAX_IDLE_PER_HOST, TCP_KEEPALIVE_IDLE_SECS,
    TCP_KEEPALIVE_INTERVAL_SECS, TCP_KEEPALIVE_RETRIES,
};
use super::rate_limit::{
    rate_limit_lock_path, rate_limit_stamp_path, stamp_remaining, write_stamp,
};

/// Install ring CryptoProvider once when none is configured (ADR 0007).
///
/// Safe to call repeatedly. Does not overwrite an existing process default
/// (binary `main` already installed ring, or a prior call succeeded).
fn ensure_ring_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if rustls::crypto::CryptoProvider::get_default().is_none() {
            let _ = rustls::crypto::ring::default_provider().install_default();
        }
    });
}

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
    budget: ConcurrencyBudget,
    retry: RetryConfig,
}

/// Successful HTTP response body and metadata.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    /// Final HTTP status after redirects.
    pub status: StatusCode,
    /// Final URL after redirects.
    pub final_url: Url,
    /// Response body bytes (capped).
    pub body: Bytes,
    /// Content-Type header when present.
    pub content_type: Option<String>,
    /// True when served from the local disk cache.
    pub cache_hit: bool,
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
        // reqwest 0.13 `rustls-no-provider`: a process CryptoProvider is required.
        // Binary installs ring first in `main` (fail-closed dual-init). Library
        // callers and tests reach this path without `main` — install once if missing.
        ensure_ring_crypto_provider();

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
        // Explicit encodings: feature `brotli` alone does not add `br` when this
        // header is set manually (SCRAPE-S-004).
        headers.insert(ACCEPT_ENCODING, HeaderValue::from_static("gzip, br"));

        let client = Client::builder()
            .default_headers(headers)
            .timeout(cfg.timeout())
            .connect_timeout(cfg.connect_timeout())
            .redirect(reqwest::redirect::Policy::custom({
                let max = cfg.max_redirects;
                let allow_loopback = cfg.allow_loopback;
                move |attempt| {
                    if attempt.previous().len() as u32 >= max {
                        return attempt
                            .error(AppError::new(ErrorKind::Network, "redirect limit exceeded"));
                    }
                    // Borrow host_str — no intermediate String on the redirect hot path.
                    // Same allowlist as request gate + config origins (SSRF defense in depth).
                    let host = attempt.url().host_str().unwrap_or("");
                    let scheme = attempt.url().scheme();
                    if !crate::config::is_allowed_origin_scheme_host(scheme, host, allow_loopback)
                    {
                        // Materialize the message before moving `attempt` (host is borrowed).
                        let msg = format!("redirect host not allowlisted: {host}");
                        return attempt.error(AppError::new(ErrorKind::Network, msg));
                    }
                    attempt.follow()
                }
            }))
            .use_rustls_tls()
            // TLS 1.2 floor (Rules Rust — TLS). Origins negotiate 1.3 when available.
            // Not `https_only(true)`: offline wiremock uses `http://127.0.0.1` when
            // `allow_loopback` is set (CLI/XDG); production hosts still require HTTPS.
            .min_tls_version(reqwest::tls::Version::TLS_1_2)
            // Low-latency request/response docs GETs (disable Nagle).
            .tcp_nodelay(true)
            .tcp_keepalive(Duration::from_secs(TCP_KEEPALIVE_IDLE_SECS))
            .tcp_keepalive_interval(Duration::from_secs(TCP_KEEPALIVE_INTERVAL_SECS))
            .tcp_keepalive_retries(TCP_KEEPALIVE_RETRIES)
            // One-shot pool: tiny idle set, short idle TTL (process dies soon).
            .pool_max_idle_per_host(POOL_MAX_IDLE_PER_HOST)
            .pool_idle_timeout(Duration::from_secs(POOL_IDLE_TIMEOUT_SECS))
            // HTTP/2 adaptive flow control when ALPN selects h2.
            .http2_adaptive_window(true)
            // system-proxy feature: honor HTTP(S)_PROXY / ALL_PROXY / NO_PROXY.
            .build()
            .map_err(|e| {
                AppError::with_source(ErrorKind::Network, "failed to build HTTP client", e)
            })?;

        let cache = if cfg.cache_enabled() {
            cfg.cache_dir.as_ref().map(|dir| {
                DiskCache::new(
                    dir.clone(),
                    cfg.cache_ttl(),
                    cfg.max_cache_bytes,
                    cfg.allow_loopback,
                )
            })
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

    /// Shared cancel flag for cooperative shutdown (HTTP + CPU scrape).
    pub fn cancel_flag(&self) -> &crate::shutdown::CancelFlag {
        &self.cancel
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
        if !is_allowed_host(url, self.cfg.allow_loopback) {
            return Err(AppError::new(
                ErrorKind::Network,
                format!("host not allowlisted: {}", url.host_str().unwrap_or("")),
            ));
        }

        // Disk cache only for GET (product surface is GET-only).
        // Invariant: every HttpResponse respects max_body_bytes (network *and* cache).
        if method == Method::GET
            && let Some(cache) = &self.cache
            && let Some(hit) = cache.get(url, accept)
        {
            let cap = self.cfg.max_body_bytes.min(HARD_MAX_BODY_BYTES);
            if (hit.body.len() as u64) > cap {
                return Err(AppError::new(
                    ErrorKind::Budget,
                    format!("cached response body exceeds max_body_bytes ({cap})"),
                ));
            }
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
        // Monotonic start for max_elapsed_ms budget (Rules: Instant, not SystemTime).
        let started = Instant::now();
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
                        RetryClass::RateLimited { retry_after } => {
                            let wait = wait_for_retry(policy, attempt, retry_after);
                            if policy.may_retry_within_budget(attempt, started.elapsed(), wait) {
                                let span = info_span!(
                                    target: "docsrs_cli::retry",
                                    "retry_attempt",
                                    attempt,
                                    max_attempts = policy.max_attempts(),
                                    status = %status,
                                    layer = ErrorLayer::HttpStatus.as_str(),
                                    wait_ms = wait.as_millis() as u64,
                                    reason = "rate_limited",
                                );
                                async {
                                    debug!(
                                        target: "docsrs_cli::retry",
                                        ?wait,
                                        attempt,
                                        max_attempts = policy.max_attempts(),
                                        status = %status,
                                        "retrying after rate-limit / Retry-After"
                                    );
                                    self.sleep_cancelable(wait).await
                                }
                                .instrument(span)
                                .await?;
                                continue;
                            }
                            let secs = retry_after.map(|d| d.as_secs()).unwrap_or(1);
                            return Err(AppError::from_http_status(
                                status.as_u16(),
                                "retries exhausted",
                            )
                            .with_retry_after(secs.max(1)));
                        }
                        RetryClass::Transient => {
                            let wait = wait_for_retry(policy, attempt, None);
                            if policy.may_retry_within_budget(attempt, started.elapsed(), wait) {
                                let span = info_span!(
                                    target: "docsrs_cli::retry",
                                    "retry_attempt",
                                    attempt,
                                    max_attempts = policy.max_attempts(),
                                    status = %status,
                                    layer = ErrorLayer::HttpStatus.as_str(),
                                    wait_ms = wait.as_millis() as u64,
                                    reason = "transient_status",
                                );
                                async {
                                    debug!(
                                        target: "docsrs_cli::retry",
                                        ?wait,
                                        attempt,
                                        max_attempts = policy.max_attempts(),
                                        status = %status,
                                        "retrying after transient status"
                                    );
                                    self.sleep_cancelable(wait).await
                                }
                                .instrument(span)
                                .await?;
                                continue;
                            }
                            // Exhausted attempts or elapsed budget on 5xx/408.
                            return Err(AppError::from_http_status(
                                status.as_u16(),
                                "retries exhausted",
                            ));
                        }
                        RetryClass::Permanent => {
                            // 2xx and permanent 4xx: read body and return (caller maps status).
                        }
                    }

                    // Defense in depth: redirect policy already gates hops; re-check
                    // final URL so a policy regression cannot leak off-allowlist bodies.
                    if !is_allowed_host(&final_url, self.cfg.allow_loopback) {
                        return Err(AppError::new(
                            ErrorKind::Network,
                            format!(
                                "final URL host not allowlisted: {}",
                                final_url.host_str().unwrap_or("")
                            ),
                        ));
                    }
                    let body = read_body_capped(resp, self.cfg.max_body_bytes).await?;
                    let out = HttpResponse {
                        status,
                        final_url,
                        body,
                        content_type,
                        cache_hit: false,
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
                    let (kind, layer, transport_retryable) = classify_reqwest_error(&e);
                    if transport_retryable {
                        let wait = wait_for_retry(policy, attempt, None);
                        if policy.may_retry_within_budget(attempt, started.elapsed(), wait) {
                            let span = info_span!(
                                target: "docsrs_cli::retry",
                                "retry_attempt",
                                attempt,
                                max_attempts = policy.max_attempts(),
                                layer = layer.as_str(),
                                wait_ms = wait.as_millis() as u64,
                                reason = "transport",
                            );
                            async {
                                warn!(
                                    target: "docsrs_cli::retry",
                                    ?wait,
                                    attempt,
                                    max_attempts = policy.max_attempts(),
                                    error = %e,
                                    layer = layer.as_str(),
                                    "retrying after transport error"
                                );
                                self.sleep_cancelable(wait).await
                            }
                            .instrument(span)
                            .await?;
                            continue;
                        }
                    }
                    return Err(AppError::with_source(
                        kind,
                        format!("http request failed for {url}"),
                        e,
                    ));
                }
            }
        }
    }

    async fn sleep_cancelable(&self, wait: Duration) -> AppResult<()> {
        let step = Duration::from_millis(CANCEL_POLL_INTERVAL_MS);
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
            let map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
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
                    self.sleep_cancelable(Duration::from_millis(CANCEL_POLL_INTERVAL_MS))
                        .await?;
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

