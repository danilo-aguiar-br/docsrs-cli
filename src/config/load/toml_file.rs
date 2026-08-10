//! Capped `config.toml` read, strict schema, and the load pipeline.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::config::constants::*;
use crate::config::path_source::resolve_config_dir_with_source;
use crate::config::validate::default_user_agent;
use crate::domain::AllowedOrigin;
use crate::error::{AppError, AppResult, ErrorDetail, IoOp, Subject, ValueSource, io_at};

use super::Config;

/// Read `config.toml` with a hard size cap (poisoned-config / TOML bomb guard).
///
/// Uses `fs::read` + length check (not unbounded `read_to_string`). Metadata
/// size is an early reject; the post-read length is authoritative (TOCTOU-safe).
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::Config`] when the file is unreadable, over
/// [`MAX_CONFIG_TOML_BYTES`], or not valid UTF-8.
fn read_config_toml_capped(path: &Path) -> AppResult<String> {
    if let Ok(meta) = fs::metadata(path)
        && meta.len() > MAX_CONFIG_TOML_BYTES
    {
        return Err(AppError::of(ErrorDetail::ConfigTomlTooLarge {
            max_bytes: MAX_CONFIG_TOML_BYTES,
        }));
    }
    let bytes = fs::read(path).map_err(io_at(IoOp::Read, path))?;
    if (bytes.len() as u64) > MAX_CONFIG_TOML_BYTES {
        return Err(AppError::of(ErrorDetail::ConfigTomlTooLarge {
            max_bytes: MAX_CONFIG_TOML_BYTES,
        }));
    }
    String::from_utf8(bytes)
        .map_err(|e| AppError::of_with_source(ErrorDetail::ConfigTomlNotUtf8, e))
}

/// XDG `config.toml` keys only — unknown keys fail closed (typo ≠ silent ignore).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct TomlConfig {
    timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
    max_body_bytes: Option<u64>,
    max_output_bytes: Option<u64>,
    max_redirects: Option<u32>,
    max_retries: Option<u32>,
    retry_base_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    retry_max_elapsed_ms: Option<u64>,
    disable_retry: Option<bool>,
    rate_limit_delay_ms: Option<u64>,
    max_concurrency: Option<u32>,
    user_agent: Option<String>,
    contact: Option<String>,
    lang: Option<String>,
    log_directive: Option<String>,
    crates_io_origin: Option<String>,
    docs_rs_origin: Option<String>,
    allow_loopback: Option<bool>,
    cache_ttl_secs: Option<u64>,
    max_cache_bytes: Option<u64>,
    no_cache: Option<bool>,
}

impl Config {
    /// Loads configuration with an optional config-directory override.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when the TOML file is unreadable or invalid.
    pub fn load(config_dir_override: Option<PathBuf>) -> AppResult<Self> {
        Self::load_with_cache_dir(config_dir_override, None)
    }

    /// Load config with optional config-dir and cache-dir overrides (CLI / tests).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when `config.toml` is unreadable or invalid TOML.
    pub fn load_with_cache_dir(
        config_dir_override: Option<PathBuf>,
        cache_dir_override: Option<PathBuf>,
    ) -> AppResult<Self> {
        Self::load_with_options(config_dir_override, cache_dir_override, false)
    }

    /// Load config with optional path overrides and a CLI loopback seed.
    ///
    /// `allow_loopback` is applied **before** TOML origin parsing so
    /// `--allow-loopback` works with mock origins in the same invocation
    /// (ADR 0009 — never via env).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when `config.toml` is unreadable or invalid TOML.
    pub fn load_with_options(
        config_dir_override: Option<PathBuf>,
        cache_dir_override: Option<PathBuf>,
        allow_loopback: bool,
    ) -> AppResult<Self> {
        let mut cfg = Self::default();
        if allow_loopback {
            cfg.allow_loopback = true;
        }

        let (config_dir, config_source) = resolve_config_dir_with_source(config_dir_override);
        cfg.config_path_source = config_source;

        let (cache_dir, cache_source) =
            crate::cache::resolve_cache_dir_with_source(cache_dir_override);
        cfg.cache_dir = cache_dir;
        cfg.cache_path_source = cache_source;

        // Borrow for TOML load, then move into `cfg` (no PathBuf clone).
        if let Some(dir) = &config_dir {
            let path = dir.join(CONFIG_FILE_NAME);
            if path.is_file() {
                let text = read_config_toml_capped(&path)?;
                let parsed: TomlConfig = toml::from_str(&text)
                    .map_err(|e| AppError::of_with_source(ErrorDetail::ConfigTomlInvalid, e))?;
                cfg.apply_toml(parsed)?;
                cfg.config_toml_loaded = true;
            }
        }
        cfg.config_dir = config_dir;

        cfg.apply_cache_path_defaults()?;
        cfg.clamp_resource_limits();
        cfg.validate_security()?;
        Ok(cfg)
    }

    fn apply_toml(&mut self, t: TomlConfig) -> AppResult<()> {
        // Same invariant as the flags, same failure, different exit code. A zero
        // timeout is rejected on the command line (exit 65) but used to be
        // accepted here, so `--timeout 0` and `timeout_secs = 0` disagreed about
        // the very same knob. `ValueSource::ConfigFile` is what makes the two
        // paths report the one shared rule with the remediation each caller can
        // actually act on: fix the argument, or fix the file.
        if let Some(v) = t.timeout_secs {
            if v == 0 {
                return Err(AppError::of(ErrorDetail::MustBeAtLeastOneSecond {
                    subject: Subject::Timeout,
                    source: ValueSource::ConfigFile,
                }));
            }
            self.timeout_secs = v;
        }
        if let Some(v) = t.connect_timeout_secs {
            if v == 0 {
                return Err(AppError::of(ErrorDetail::MustBeAtLeastOneSecond {
                    subject: Subject::ConnectTimeout,
                    source: ValueSource::ConfigFile,
                }));
            }
            self.connect_timeout_secs = v;
        }
        if let Some(v) = t.max_body_bytes {
            // Fail-closed for operator honesty (GAP-X-005): TOML cannot raise above HARD_MAX.
            if v > HARD_MAX_BODY_BYTES {
                return Err(AppError::of(ErrorDetail::AboveHardMaximum {
                    subject: Subject::MaxBodyBytes,
                    hard_max: HARD_MAX_BODY_BYTES,
                    source: ValueSource::ConfigFile,
                }));
            }
            self.max_body_bytes = v;
        }
        if let Some(v) = t.max_output_bytes {
            if v > HARD_MAX_OUTPUT_BYTES {
                return Err(AppError::of(ErrorDetail::AboveHardMaximum {
                    subject: Subject::MaxOutputBytes,
                    hard_max: HARD_MAX_OUTPUT_BYTES,
                    source: ValueSource::ConfigFile,
                }));
            }
            self.max_output_bytes = v;
        }
        if let Some(v) = t.max_redirects {
            self.max_redirects = v;
        }
        if let Some(v) = t.max_retries {
            self.max_retries = v;
        }
        if let Some(v) = t.retry_base_ms {
            self.retry_base_ms = v;
        }
        if let Some(v) = t.retry_max_delay_ms {
            self.retry_max_delay_ms = v;
        }
        if let Some(v) = t.retry_max_elapsed_ms {
            self.retry_max_elapsed_ms = v;
        }
        if let Some(v) = t.disable_retry {
            self.disable_retry = v;
        }
        if let Some(v) = t.rate_limit_delay_ms {
            self.rate_limit_delay_ms = v;
        }
        if let Some(v) = t.max_concurrency {
            self.max_concurrency = v;
        }
        if let Some(v) = t.contact {
            self.contact = Some(v);
        }
        if let Some(v) = t.user_agent {
            self.user_agent = v;
        } else if let Some(c) = &self.contact {
            self.user_agent = default_user_agent(Some(c));
        }
        if let Some(v) = t.lang {
            self.lang = Some(v);
        }
        if let Some(v) = t.log_directive {
            // Validate here, not at subscriber install time. `init_tracing` runs
            // before any structured error path exists, so its only option is to
            // swallow a bad value and fall back — which leaves the operator with
            // a knob that is silently ignored and no way to find out why. That is
            // the same shape as the ambient override this key replaced. Parsing
            // is pure and cheap, so the failure becomes representable exactly
            // where `config.toml` is read, and joins every other bad TOML value
            // at exit 78. The fallback in `init_tracing` stays as defence in
            // depth for callers that bypass this path.
            if tracing_subscriber::EnvFilter::try_new(&v).is_err() {
                return Err(AppError::of(ErrorDetail::Invalid {
                    subject: Subject::LogDirective,
                    value: v,
                }));
            }
            self.log_directive = Some(v);
        }
        // Apply loopback gate before parsing origins so mock hosts validate.
        if let Some(v) = t.allow_loopback {
            self.allow_loopback = v;
        }
        if let Some(v) = t.crates_io_origin {
            self.crates_io_origin = AllowedOrigin::parse_with(&v, self.allow_loopback)?;
        }
        if let Some(v) = t.docs_rs_origin {
            self.docs_rs_origin = AllowedOrigin::parse_with(&v, self.allow_loopback)?;
        }
        if let Some(v) = t.cache_ttl_secs {
            self.cache_ttl_secs = v;
        }
        if let Some(v) = t.max_cache_bytes {
            self.max_cache_bytes = v;
        }
        if let Some(v) = t.no_cache {
            self.no_cache = v;
        }
        Ok(())
    }

    /// Resolve cache path when still unset (XDG / test sandbox).
    ///
    /// Resolve cache path defaults from CLI override / XDG only (no product env).
    fn apply_cache_path_defaults(&mut self) -> AppResult<()> {
        if self.cache_dir.is_none() {
            let (dir, source) = crate::cache::resolve_cache_dir_with_source(None);
            self.cache_dir = dir;
            self.cache_path_source = source;
        }
        Ok(())
    }
}
