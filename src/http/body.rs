//! Response body budget streaming and UTF-8 decode.

use bytes::Bytes;
use futures_util::StreamExt;

use crate::config::HARD_MAX_BODY_BYTES;
use crate::error::{AppError, AppResult, ErrorDetail};

/// Read a response body with a hard byte budget and fallible allocation.
///
/// Primary defense is [`HARD_MAX_BODY_BYTES`] (operators may only lower the cap).
/// `try_reserve` / `try_reserve_exact` map allocation failure to
/// [`crate::error::ErrorKind::Network`] instead of aborting via `with_capacity` on hostile sizes.
/// On Linux overcommit, the allocator may still report success and the OOM killer
/// can fire later — the hard ceiling remains the main bound.
pub(super) async fn read_body_capped(resp: reqwest::Response, max_bytes: u64) -> AppResult<Bytes> {
    // Never honor a budget above the product hard ceiling (defense in depth).
    let max_bytes = max_bytes.min(HARD_MAX_BODY_BYTES);
    // When Content-Length is known and already over budget, fail without buffering.
    // (With gzip, length may be compressed size or absent; still a useful early guard.)
    let content_length = resp.content_length();
    if let Some(n) = content_length
        && n > max_bytes
    {
        // Permanent local budget — not a transport failure; do not auto-retry.
        return Err(AppError::of(ErrorDetail::BodyOverBudget { max_bytes }));
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
            AppError::of_with_source(ErrorDetail::BodyReserveFailed { bytes: capacity }, e)
        })?;
    }
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| AppError::of_with_source(ErrorDetail::BodyRead, e))?;
        if (buf.len() as u64).saturating_add(chunk.len() as u64) > max_bytes {
            // Permanent local budget — not a transport failure; do not auto-retry.
            return Err(AppError::of(ErrorDetail::BodyOverBudget { max_bytes }));
        }
        // Grow with try_reserve when stream chunks exceed Content-Length estimate.
        let need = chunk.len();
        if buf.capacity().saturating_sub(buf.len()) < need {
            buf.try_reserve(need).map_err(|e| {
                AppError::of_with_source(ErrorDetail::BodyReserveFailed { bytes: need }, e)
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
/// Returns [`crate::error::ErrorKind::Parse`] when the body is not valid UTF-8.
pub fn decode_utf8(body: &Bytes) -> AppResult<String> {
    let mut bytes = body.as_ref();
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes = &bytes[3..];
    }
    std::str::from_utf8(bytes)
        .map(|s| s.to_string())
        .map_err(|e| AppError::of_with_source(ErrorDetail::BodyNotUtf8, e))
}
