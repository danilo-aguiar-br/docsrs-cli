//! HTTP client: rustls, retry, rate limit, body cap, host allowlist, cancel-aware.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use reqwest::header::{ACCEPT, ACCEPT_ENCODING, HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Client, Method, StatusCode, Url};
use tracing::{debug, warn};

use crate::cache::DiskCache;
use crate::config::{Config, HOST_CRATES_IO, HOST_DOCS_RS, HOST_STATIC_DOCS_RS};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::shutdown::CancelFlag;

/// HTTPS client bound to product allowlist and config.
pub struct HttpClient {
    client: Client,
    cfg: Config,
    last_host_hit: Mutex<HashMap<String, Instant>>,
    cancel: CancelFlag,
    cache: Option<DiskCache>,
}

/// Successful HTTP response body and metadata.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub final_url: Url,
    pub body: Bytes,
    pub content_type: Option<String>,
}

impl HttpClient {
    /// Build a client with rustls, default headers, and redirect allowlist.
    pub fn new(cfg: Config, cancel: CancelFlag) -> AppResult<Self> {
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
                    let host = attempt.url().host_str().unwrap_or("").to_string();
                    let scheme_ok = attempt.url().scheme() == "https";
                    let host_ok = matches!(
                        host.as_str(),
                        HOST_CRATES_IO | HOST_DOCS_RS | HOST_STATIC_DOCS_RS
                    ) || ((host == "127.0.0.1" || host == "localhost")
                        && (cfg!(test) || std::env::var("DOCSRS_CLI_ALLOW_LOCALHOST").is_ok()));
                    if !(scheme_ok && host_ok) {
                        return attempt.error(AppError::new(
                            ErrorKind::Network,
                            format!("redirect host not allowlisted: {host}"),
                        ));
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
        })
    }

    /// GET expecting JSON (`Accept: application/json`).
    pub async fn get_json(&self, url: &Url) -> AppResult<HttpResponse> {
        self.request(Method::GET, url, "application/json", true)
            .await
    }

    /// GET expecting HTML (`Accept: text/html`).
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

        let mut attempt = 0u32;
        let max_attempts = if retryable {
            self.cfg.max_retries.saturating_add(1)
        } else {
            1
        };

        loop {
            self.cancel.check()?;
            attempt += 1;
            self.rate_limit(url).await;
            self.cancel.check()?;

            let mut req = self.client.request(method.clone(), url.clone());
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

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = parse_retry_after(resp.headers());
                        if attempt < max_attempts {
                            let wait = retry_after
                                .map(Duration::from_secs)
                                .unwrap_or_else(|| backoff(self.cfg.retry_base_ms, attempt));
                            debug!(?wait, attempt, "retrying after 429");
                            self.sleep_cancelable(wait).await?;
                            continue;
                        }
                        return Err(AppError::from_http_status(429, "retries exhausted")
                            .with_retry_after(retry_after.unwrap_or(1)));
                    }

                    if matches!(status.as_u16(), 500 | 502 | 503 | 504) && attempt < max_attempts {
                        let wait = backoff(self.cfg.retry_base_ms, attempt);
                        debug!(?wait, attempt, status = %status, "retrying after 5xx");
                        self.sleep_cancelable(wait).await?;
                        continue;
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
                    let is_timeout = e.is_timeout() || e.is_connect();
                    let kind = if is_timeout {
                        ErrorKind::Timeout
                    } else {
                        ErrorKind::Network
                    };
                    if retryable && attempt < max_attempts && (is_timeout || e.is_request()) {
                        let wait = backoff(self.cfg.retry_base_ms, attempt);
                        warn!(?wait, attempt, error = %e, "retrying after transport error");
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

    async fn rate_limit(&self, url: &Url) {
        let host = url.host_str().unwrap_or("").to_string();
        let delay = self.cfg.rate_limit_delay();
        if delay.is_zero() {
            return;
        }
        let sleep_for = {
            let map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(prev) = map.get(&host) {
                let elapsed = prev.elapsed();
                if elapsed < delay {
                    Some(delay - elapsed)
                } else {
                    None
                }
            } else {
                None
            }
        };
        if let Some(d) = sleep_for {
            let _ = self.sleep_cancelable(d).await;
        }
        let mut map = self.last_host_hit.lock().unwrap_or_else(|e| e.into_inner());
        map.insert(host, Instant::now());
    }
}

fn is_allowed_host(url: &Url) -> bool {
    match url.host_str() {
        Some(HOST_CRATES_IO) | Some(HOST_DOCS_RS) | Some(HOST_STATIC_DOCS_RS) => {
            url.scheme() == "https"
        }
        Some("127.0.0.1") | Some("localhost") => {
            cfg!(test) || std::env::var("DOCSRS_CLI_ALLOW_LOCALHOST").is_ok()
        }
        _ => false,
    }
}

fn backoff(base_ms: u64, attempt: u32) -> Duration {
    let exp = base_ms.saturating_mul(1u64 << attempt.min(5));
    let jitter = exp / 2 + (exp / 2).saturating_mul((attempt as u64 % 3) + 1) / 3;
    Duration::from_millis(jitter.max(base_ms))
}

fn parse_retry_after(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}

async fn read_body_capped(resp: reqwest::Response, max_bytes: u64) -> AppResult<Bytes> {
    let mut stream = resp.bytes_stream();
    let mut buf: Vec<u8> = Vec::new();
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
        buf.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buf))
}

/// Decode body as UTF-8, stripping a leading UTF-8 BOM.
pub fn decode_utf8(body: &Bytes) -> AppResult<String> {
    let mut bytes = body.as_ref();
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = &bytes[3..];
    }
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|e| AppError::with_source(ErrorKind::Parse, "response body is not valid UTF-8", e))
}

/// True when Content-Type looks like JSON.
pub fn content_type_looks_json(ct: Option<&str>) -> bool {
    ct.map(|c| c.to_ascii_lowercase().contains("json"))
        .unwrap_or(false)
}

/// True when Content-Type looks like HTML/XHTML.
pub fn content_type_looks_html(ct: Option<&str>) -> bool {
    ct.map(|c| {
        let c = c.to_ascii_lowercase();
        c.contains("html") || c.contains("xhtml")
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_https_only() {
        let ok = Url::parse("https://docs.rs/serde").unwrap();
        assert!(is_allowed_host(&ok));
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
        assert!(content_type_looks_html(Some("text/html")));
        assert!(!content_type_looks_json(Some("text/html")));
    }

    #[test]
    fn backoff_grows() {
        let a = backoff(100, 1);
        let b = backoff(100, 3);
        assert!(b >= a);
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
