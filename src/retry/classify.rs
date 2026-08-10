//! Failure classification: HTTP status, transport layer, and `Retry-After`.

use std::time::{Duration, SystemTime};

use reqwest::StatusCode;
use reqwest::header::HeaderMap;

use super::{HARD_MAX_DELAY_MS, MILLIS_PER_SECOND};

/// Clock-skew tolerance for past HTTP-date `Retry-After` values.
const RETRY_AFTER_SKEW_SECS: u64 = 1;

/// Classification of an HTTP status for the retry gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryClass {
    /// Do not retry (4xx client errors, success, unknown permanent).
    Permanent,
    /// Retry with exponential full-jitter backoff (5xx / 408).
    Transient,
    /// Retry respecting optional `Retry-After` (429, and 503 with header).
    RateLimited {
        /// Parsed `Retry-After` delta-seconds or HTTP-date delta when present.
        retry_after: Option<Duration>,
    },
}

/// Detailed retry category exposed on errors (Rules checklist `retry_kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryKind {
    /// Never auto-retry (validation, 4xx, parse, budget, cancel).
    Permanent,
    /// Transport / connect failure (may retry).
    TransientNetwork,
    /// Remote 5xx / temporary unavailability (may retry).
    TransientServer,
    /// HTTP 429 with optional `Retry-After` (may retry).
    RateLimited,
    /// Wall or request timeout (may retry).
    Timeout,
}

/// Transport / protocol layer of a failure (Rules checklist `ErrorLayer`).
///
/// Reqwest/hyper does not always separate DNS from TCP or TLS handshake from
/// connect; `TcpConnect` covers DNS+TCP+TLS establishment when only
/// `is_connect()` is available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorLayer {
    /// Name resolution (when distinguishable; else folded into [`Self::TcpConnect`]).
    Dns,
    /// TCP connect / peer unreachable (includes DNS when not separated).
    TcpConnect,
    /// TLS handshake failure (when distinguishable).
    Tls,
    /// HTTP/1.1 or HTTP/2 protocol error after connect.
    HttpProtocol,
    /// HTTP status from a completed response.
    HttpStatus,
    /// Application / parse / budget after a successful body read.
    Application,
    /// Unclassified.
    Unknown,
}

impl ErrorLayer {
    /// Stable snake_case label for logs and doctor detail.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dns => "dns",
            Self::TcpConnect => "tcp_connect",
            Self::Tls => "tls",
            Self::HttpProtocol => "http_protocol",
            Self::HttpStatus => "http_status",
            Self::Application => "application",
            Self::Unknown => "unknown",
        }
    }
}

impl RetryKind {
    /// Stable snake_case label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Permanent => "permanent",
            Self::TransientNetwork => "transient_network",
            Self::TransientServer => "transient_server",
            Self::RateLimited => "rate_limited",
            Self::Timeout => "timeout",
        }
    }

    /// Whether this kind is eligible for in-process or agent retry.
    pub fn is_retryable(self) -> bool {
        !matches!(self, Self::Permanent)
    }
}

/// Classify HTTP status for in-process retry (GET-only product).
pub fn classify_http_status(status: StatusCode, retry_after: Option<Duration>) -> RetryClass {
    match status.as_u16() {
        429 => RetryClass::RateLimited { retry_after },
        // 408 Request Timeout — transient by design (client may retry).
        408 | 500 | 502 | 504 => RetryClass::Transient,
        // 503: prefer Retry-After when present (same as many CDNs).
        503 => {
            if retry_after.is_some() {
                RetryClass::RateLimited { retry_after }
            } else {
                RetryClass::Transient
            }
        }
        // Success and permanent client / other statuses: never retry.
        _ => RetryClass::Permanent,
    }
}

/// True when any error in the source chain is a [`rustls::Error`].
///
/// Does not string-match messages (Rules Rust — classification). When hyper
/// folds TLS into a plain connect I/O error without a rustls source, the
/// caller still reports [`ErrorLayer::TcpConnect`] (documented fold).
pub(crate) fn source_chain_has_rustls(err: &(dyn std::error::Error + 'static)) -> bool {
    let mut cur: Option<&(dyn std::error::Error + 'static)> = Some(err);
    while let Some(e) = cur {
        if e.downcast_ref::<rustls::Error>().is_some() {
            return true;
        }
        cur = e.source();
    }
    false
}

/// Classify a reqwest transport error without string-matching messages.
///
/// Returns `(app ErrorKind, layer, in_process_retryable)`.
///
/// When a [`rustls::Error`] appears in the source chain, the layer is
/// [`ErrorLayer::Tls`] even if reqwest also flags timeout/connect/request
/// (prefer precise TLS labeling for agent diagnostics).
pub fn classify_reqwest_error(err: &reqwest::Error) -> (crate::error::ErrorKind, ErrorLayer, bool) {
    use crate::error::ErrorKind;

    let tls = source_chain_has_rustls(err as &(dyn std::error::Error + 'static));

    if err.is_timeout() {
        // Connect or total timeout — both are transient for GET.
        let layer = if tls {
            ErrorLayer::Tls
        } else {
            ErrorLayer::TcpConnect
        };
        return (ErrorKind::Timeout, layer, true);
    }
    if tls {
        // Handshake / cert validation failures when discriminable via rustls.
        return (ErrorKind::Network, ErrorLayer::Tls, true);
    }
    if err.is_connect() {
        // Hyper folds DNS + TCP (+ often TLS without rustls source) into connect.
        return (ErrorKind::Network, ErrorLayer::TcpConnect, true);
    }
    if err.is_request() {
        // Protocol / request construction / HTTP/2 stream issues after dial.
        return (ErrorKind::Network, ErrorLayer::HttpProtocol, true);
    }
    if err.is_body() || err.is_decode() {
        // Incomplete body after headers — rare; treat as network, non-retry in
        // send path (body path is separate). Keep retryable=false here.
        return (ErrorKind::Network, ErrorLayer::HttpProtocol, false);
    }
    (ErrorKind::Network, ErrorLayer::Unknown, false)
}

/// Parse `Retry-After` as **delta-seconds** or **HTTP-date** (RFC 7231).
///
/// - Pure non-negative integer → seconds (capped at [`HARD_MAX_DELAY_MS`]).
/// - IMF-fixdate via `httpdate` → duration until that instant.
/// - Past date within `RETRY_AFTER_SKEW_SECS` → `Duration::ZERO`.
/// - Older past date or unparseable → `None` (caller falls back to formula).
pub fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let raw = headers
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())?
        .trim();
    if raw.is_empty() {
        return None;
    }
    // Delta-seconds (preferred when the value is purely numeric).
    if raw.bytes().all(|b| b.is_ascii_digit()) {
        let secs: u64 = raw.parse().ok()?;
        return Some(Duration::from_secs(
            secs.min(HARD_MAX_DELAY_MS / MILLIS_PER_SECOND),
        ));
    }
    // HTTP-date form.
    let when = httpdate::parse_http_date(raw).ok()?;
    let now = SystemTime::now();
    match when.duration_since(now) {
        Ok(d) => Some(d.min(Duration::from_millis(HARD_MAX_DELAY_MS))),
        Err(e) => {
            // Clock skew: slightly past → treat as immediate; deep past → formula.
            if e.duration() <= Duration::from_secs(RETRY_AFTER_SKEW_SECS) {
                Some(Duration::ZERO)
            } else {
                None
            }
        }
    }
}
