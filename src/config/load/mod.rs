//! Runtime [`Config`]: defaults, clamps, validation, and derived durations.
//!
//! Layout (SRP): `template` (commented `config.toml` body) · `toml_file`
//! (capped read, schema, and the load pipeline) · `init` (`config init`) ·
//! `paths` (`config path` inventory).

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{AllowedOrigin, CrateName};
use crate::error::AppResult;

use super::constants::*;
use super::path_source::PathSource;
use super::validate::{default_user_agent, validate_contact, validate_user_agent};

mod init;
mod paths;
mod template;
mod toml_file;

pub use init::{ConfigInitResult, init_config_toml};
pub use paths::{ConfigPathData, config_path_data, config_toml_under};
pub use template::default_config_toml;

/// Runtime configuration resolved from defaults, XDG TOML, and CLI flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Wall-clock timeout in seconds for one operation.
    pub timeout_secs: u64,
    /// TCP connect timeout in seconds.
    pub connect_timeout_secs: u64,
    /// Maximum downloaded HTTP body size in bytes.
    pub max_body_bytes: u64,
    /// Maximum emitted stdout payload size in bytes.
    pub max_output_bytes: u64,
    /// Maximum HTTP redirects followed.
    pub max_redirects: u32,
    /// Maximum retries for transient HTTP errors.
    pub max_retries: u32,
    /// Base backoff delay in milliseconds between retries.
    pub retry_base_ms: u64,
    /// Ceiling for a single retry sleep in milliseconds.
    pub retry_max_delay_ms: u64,
    /// Total retry loop wall budget in milliseconds (`0` = derive from `timeout_secs`).
    pub retry_max_elapsed_ms: u64,
    /// Kill switch: when true, HTTP never retries (incident / debug).
    pub disable_retry: bool,
    /// Minimum delay between requests to the same host in milliseconds.
    pub rate_limit_delay_ms: u64,
    /// Max concurrent CPU (`spawn_blocking`) tasks; `0` = auto from CPUs/RAM.
    ///
    /// See [`crate::concurrency::resolve_max_concurrency`].
    pub max_concurrency: u32,
    /// HTTP User-Agent header value.
    pub user_agent: String,
    /// Optional contact URL or email embedded in the default User-Agent.
    ///
    /// Omitted from `config show` when unset — a `null` key is a dead field that
    /// costs an agent tokens without carrying information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>,
    /// Preferred human-message locale (`en` or `pt-BR`).
    ///
    /// Omitted from `config show` when unset (see [`Config::contact`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// Default `tracing` filter directive when `-q` / `-v` are absent.
    ///
    /// This is the **only** way to steer stderr diagnostics beyond the CLI
    /// flags: the process reads no product environment variable, so `RUST_LOG`
    /// has no effect (ADR 0009). Omitted from `config show` when unset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_directive: Option<String>,
    /// Resolved XDG (or override) config directory.
    pub config_dir: Option<PathBuf>,
    /// XDG (or override) root for HTTP response cache between process runs.
    pub cache_dir: Option<PathBuf>,
    /// Disk cache TTL in seconds (default 24h).
    pub cache_ttl_secs: u64,
    /// Soft cap on disk cache size in bytes (`0` = unlimited).
    pub max_cache_bytes: u64,
    /// When true, skip disk cache read/write (`--no-cache`).
    pub no_cache: bool,
    /// Origin for crates.io API (default `https://crates.io`). Overridable for offline mocks.
    pub crates_io_origin: AllowedOrigin,
    /// Origin for docs.rs pages (default `https://docs.rs`). Overridable for offline mocks.
    pub docs_rs_origin: AllowedOrigin,
    /// When true, SSRF allowlist accepts loopback (`127.0.0.1` / `localhost`) for offline mocks.
    ///
    /// Set only via XDG `config.toml` (`allow_loopback = true`) or CLI `--allow-loopback`.
    /// Never read from environment variables (ADR 0009).
    pub allow_loopback: bool,
    /// Layer that resolved [`Self::config_dir`].
    pub config_path_source: PathSource,
    /// Layer that resolved [`Self::cache_dir`].
    pub cache_path_source: PathSource,
    /// True when `config.toml` existed and was applied during load.
    pub config_toml_loaded: bool,
}

impl Default for Config {
    fn default() -> Self {
        // Contact comes from TOML `contact` or CLI `--user-agent`, not process env.
        let contact: Option<String> = None;
        let ua = default_user_agent(contact.as_deref());
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            connect_timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_base_ms: DEFAULT_RETRY_BASE_MS,
            retry_max_delay_ms: DEFAULT_RETRY_MAX_DELAY_MS,
            retry_max_elapsed_ms: DEFAULT_RETRY_MAX_ELAPSED_MS,
            disable_retry: false,
            rate_limit_delay_ms: DEFAULT_RATE_LIMIT_DELAY_MS,
            max_concurrency: 0,
            user_agent: ua,
            contact,
            lang: None,
            log_directive: None,
            config_dir: None,
            cache_dir: None,
            cache_ttl_secs: DEFAULT_CACHE_TTL_SECS,
            max_cache_bytes: DEFAULT_MAX_CACHE_BYTES,
            no_cache: false,
            crates_io_origin: AllowedOrigin::crates_io_default(),
            docs_rs_origin: AllowedOrigin::docs_rs_default(),
            allow_loopback: false,
            config_path_source: PathSource::Unresolved,
            cache_path_source: PathSource::Unresolved,
            config_toml_loaded: false,
        }
    }
}

impl Config {
    /// Re-validate origin fields after TOML/CLI mutation (fail-fast config).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] when either origin is not a valid
    /// allowlisted HTTP(S) URL (SSRF gate at config boundary).
    pub fn validate_origins(&mut self) -> AppResult<()> {
        // Re-parse to re-assert allowlist proof after any TOML/CLI mutation path.
        self.crates_io_origin =
            AllowedOrigin::parse_with(self.crates_io_origin.as_str(), self.allow_loopback)?;
        self.docs_rs_origin =
            AllowedOrigin::parse_with(self.docs_rs_origin.as_str(), self.allow_loopback)?;
        Ok(())
    }

    /// Validate security-sensitive config fields (origins + User-Agent + contact).
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Config`] on allowlist, contact, or
    /// User-Agent policy failures.
    pub fn validate_security(&mut self) -> AppResult<()> {
        self.validate_origins()?;
        if let Some(c) = &self.contact {
            validate_contact(c)?;
        }
        // Rebuild UA from contact when UA was not explicitly set from a prior
        // explicit user_agent key path — contact may have been applied already
        // in `apply_toml`; always re-check the effective header string.
        validate_user_agent(&self.user_agent)?;
        Ok(())
    }

    /// Absolute path to `config.toml` under the resolved config directory, if any.
    pub fn config_file_path(&self) -> Option<PathBuf> {
        self.config_dir.as_ref().map(|d| d.join(CONFIG_FILE_NAME))
    }

    /// Clamp resource budgets to hard safety ceilings (defense in depth).
    ///
    /// Call after every mutation path (TOML, CLI). Operator-facing surfaces
    /// (**CLI flags** and **TOML** body/output keys) reject values above
    /// [`HARD_MAX_BODY_BYTES`] / [`HARD_MAX_OUTPUT_BYTES`] before this runs
    /// (GAP-X-005). This method remains a last-line cap for other knobs and
    /// accidental internal elevations. Zero body/output is left as-is
    /// (fail-closed for body reads).
    pub fn clamp_resource_limits(&mut self) {
        self.max_body_bytes = self.max_body_bytes.min(HARD_MAX_BODY_BYTES);
        self.max_output_bytes = self.max_output_bytes.min(HARD_MAX_OUTPUT_BYTES);
        self.max_retries = self.max_retries.min(crate::retry::HARD_MAX_RETRIES);
        self.max_redirects = self.max_redirects.min(HARD_MAX_REDIRECTS);
        // Zero timeouts are rejected at CLI boundary; clamp absurd ceilings here.
        if self.timeout_secs > HARD_MAX_TIMEOUT_SECS {
            self.timeout_secs = HARD_MAX_TIMEOUT_SECS;
        }
        if self.connect_timeout_secs > HARD_MAX_CONNECT_TIMEOUT_SECS {
            self.connect_timeout_secs = HARD_MAX_CONNECT_TIMEOUT_SECS;
        }
        if self.connect_timeout_secs > self.timeout_secs && self.timeout_secs > 0 {
            self.connect_timeout_secs = self.timeout_secs;
        }
        if self.rate_limit_delay_ms > HARD_MAX_RATE_LIMIT_DELAY_MS {
            self.rate_limit_delay_ms = HARD_MAX_RATE_LIMIT_DELAY_MS;
        }
        if self.retry_base_ms == 0 {
            self.retry_base_ms = DEFAULT_RETRY_BASE_MS;
        } else if self.retry_base_ms < crate::retry::MIN_RETRY_BASE_MS {
            // Rules: never configure initial backoff below 50ms.
            self.retry_base_ms = crate::retry::MIN_RETRY_BASE_MS;
        }
        if self.retry_max_delay_ms == 0 {
            self.retry_max_delay_ms = DEFAULT_RETRY_MAX_DELAY_MS;
        }
        self.retry_max_delay_ms = self
            .retry_max_delay_ms
            .min(crate::retry::HARD_MAX_DELAY_MS)
            .max(self.retry_base_ms);
        // `0` keeps "derive from timeout" semantics in RetryConfig::from_config.
        if self.retry_max_elapsed_ms > 0 {
            self.retry_max_elapsed_ms = self
                .retry_max_elapsed_ms
                .min(crate::retry::HARD_MAX_ELAPSED_MS)
                .max(self.retry_max_delay_ms);
        }
        // Soft-cap oversized UA strings (full charset validation is `validate_user_agent`).
        if self.user_agent.chars().count() > MAX_USER_AGENT_CHARS {
            self.user_agent = self.user_agent.chars().take(MAX_USER_AGENT_CHARS).collect();
        }
    }

    /// Wall-clock timeout duration (at least one second).
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs.max(1))
    }

    /// TCP connect timeout duration (at least one second).
    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs.max(1))
    }

    /// Per-host rate-limit delay duration.
    pub fn rate_limit_delay(&self) -> Duration {
        Duration::from_millis(self.rate_limit_delay_ms)
    }

    /// TTL for disk cache entries.
    pub fn cache_ttl(&self) -> Duration {
        Duration::from_secs(self.cache_ttl_secs)
    }

    /// Whether disk cache should be used for HTTP GET bodies.
    pub fn cache_enabled(&self) -> bool {
        !self.no_cache && self.cache_dir.is_some()
    }
}

/// Resolve docs origin for a crate: stdlib → doc.rust-lang.org, else configured docs.rs origin.
pub fn docs_origin_for_crate(cfg: &Config, crate_name: &CrateName) -> AllowedOrigin {
    if crate_name.is_stdlib() {
        AllowedOrigin::stdlib_docs_default()
    } else {
        cfg.docs_rs_origin.clone()
    }
}
