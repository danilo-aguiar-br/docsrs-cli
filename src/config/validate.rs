//! UA/contact validation and re-exports for origin allowlist / domain helpers.
//!
//! Fail-closed SSRF host policy lives in [`super::allowlist`]. Domain input
//! (crate name, query, path, version) lives in [`crate::domain`] (ADR 0006).

use crate::domain::AllowedOrigin;
use crate::error::{AppError, AppResult, ErrorKind};

use super::constants::{APP_NAME, APP_VERSION, DEFAULT_CONTACT_URL, MAX_USER_AGENT_CHARS};

// Re-export stdlib name check from the single domain source (DRY-L-001).
pub use crate::domain::is_stdlib_name as is_stdlib_crate;

/// Builds the default User-Agent, optionally including a contact token.
pub fn default_user_agent(contact: Option<&str>) -> String {
    match contact {
        Some(c) if !c.is_empty() => {
            if c.starts_with("http://") || c.starts_with("https://") {
                format!("{APP_NAME}/{APP_VERSION} (+{c})")
            } else {
                format!("{APP_NAME}/{APP_VERSION} ({c})")
            }
        }
        _ => format!("{APP_NAME}/{APP_VERSION} (+{DEFAULT_CONTACT_URL})"),
    }
}

/// Parse an allowlisted origin (compat wrapper over [`AllowedOrigin::parse`]).
///
/// Prefer [`AllowedOrigin`] at call sites so the type carries the allowlist proof.
///
/// # Errors
///
/// Propagates [`ErrorKind::Config`] from [`AllowedOrigin::parse`].
pub fn validate_origin(raw: &str) -> AppResult<AllowedOrigin> {
    AllowedOrigin::parse(raw)
}

/// Parse an origin with an explicit loopback policy (CLI/XDG `allow_loopback`).
///
/// # Errors
///
/// Propagates [`ErrorKind::Config`] from [`AllowedOrigin::parse_with`].
pub fn validate_origin_with(raw: &str, allow_loopback: bool) -> AppResult<AllowedOrigin> {
    AllowedOrigin::parse_with(raw, allow_loopback)
}

/// Validate User-Agent: non-empty, length-capped, visible ASCII only (HTTP header).
///
/// # Errors
///
/// Returns [`ErrorKind::Config`] when empty, over [`MAX_USER_AGENT_CHARS`], or
/// containing control / non-ASCII bytes (would fail `HeaderValue::from_str`).
pub fn validate_user_agent(ua: &str) -> AppResult<()> {
    if ua.is_empty() {
        return Err(AppError::new(
            ErrorKind::Config,
            "user-agent must not be empty",
        ));
    }
    if ua.chars().count() > MAX_USER_AGENT_CHARS {
        return Err(AppError::new(
            ErrorKind::Config,
            format!("user-agent exceeds {MAX_USER_AGENT_CHARS} characters"),
        ));
    }
    // RFC 9110 field values: reject CTL and non-ASCII so HeaderValue cannot fail later.
    if !ua.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(AppError::new(
            ErrorKind::Config,
            "user-agent must be visible ASCII (no control characters or non-ASCII)",
        ));
    }
    Ok(())
}

/// Validate optional `contact` token embedded into the default User-Agent.
///
/// Same visible-ASCII policy as [`validate_user_agent`] (contact is interpolated
/// into the UA string). Empty contact is rejected when present as `Some("")`.
///
/// # Errors
///
/// Returns [`ErrorKind::Config`] when empty, over [`MAX_USER_AGENT_CHARS`], or
/// not visible ASCII.
pub fn validate_contact(contact: &str) -> AppResult<()> {
    if contact.is_empty() {
        return Err(AppError::new(
            ErrorKind::Config,
            "contact must not be empty when set",
        ));
    }
    if contact.chars().count() > MAX_USER_AGENT_CHARS {
        return Err(AppError::new(
            ErrorKind::Config,
            format!("contact exceeds {MAX_USER_AGENT_CHARS} characters"),
        ));
    }
    if !contact.bytes().all(|b| (0x20..=0x7e).contains(&b)) {
        return Err(AppError::new(
            ErrorKind::Config,
            "contact must be visible ASCII (no control characters or non-ASCII)",
        ));
    }
    Ok(())
}
