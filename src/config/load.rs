//! Config load/init, XDG paths, and TOML schema (`deny_unknown_fields`).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{AllowedOrigin, CrateName};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::platform::{restrict_private_dir, restrict_private_file};

use super::constants::*;
use super::path_source::{PathSource, resolve_config_dir_with_source};
use super::validate::{default_user_agent, validate_contact, validate_user_agent};

/// Default `config.toml` body written by `config init` (commented template).
///
/// Values are documentation only until the user uncomments. Runtime still
/// applies defaults when keys are absent. No secrets — product is public HTTP.
///
/// Version and contact URL come from [`APP_VERSION`] / [`DEFAULT_CONTACT_URL`]
/// (`Cargo.toml`) so the template never pins a stale release string.
pub fn default_config_toml() -> String {
    format!(
        r#"# docsrs-cli XDG configuration
# Precedence: CLI flags > this file (XDG) > built-in defaults.
# Path: …/docsrs-cli/config.toml (platform XDG / AppData / Library).
# No .env file is required after cargo install. Product stores no API keys.
#
# Product knobs (timeout, UA, retries, budgets, origins, concurrency, cache TTL,
# paths, lang) are read ONLY from CLI flags and this TOML — never from product
# environment variables. Isolate storage with --config-dir / --cache-dir.
#
# Host diagnostics / proxy (not product knobs — OS env only):
#   RUST_LOG, NO_COLOR, HTTP_PROXY, HTTPS_PROXY, ALL_PROXY, NO_PROXY
# HTTP client honors system proxy via reqwest feature `system-proxy`.
#
# timeout_secs = {DEFAULT_TIMEOUT_SECS}
# connect_timeout_secs = {DEFAULT_CONNECT_TIMEOUT_SECS}
# max_body_bytes = {DEFAULT_MAX_BODY_BYTES}
# max_output_bytes = {DEFAULT_MAX_OUTPUT_BYTES}
# max_redirects = {DEFAULT_MAX_REDIRECTS}
# max_retries = {DEFAULT_MAX_RETRIES}
# retry_base_ms = {DEFAULT_RETRY_BASE_MS}
# retry_max_delay_ms = {DEFAULT_RETRY_MAX_DELAY_MS}
# retry_max_elapsed_ms = 0   # 0 = derive from timeout_secs (ms); hard cap 300000
# disable_retry = false
# rate_limit_delay_ms = {DEFAULT_RATE_LIMIT_DELAY_MS}
# max_concurrency = 0
# user_agent = "{APP_NAME}/{APP_VERSION} (+{DEFAULT_CONTACT_URL})"
# contact = "{DEFAULT_CONTACT_URL}"
# lang = "en"
# crates_io_origin = "{SCHEME_HTTPS}://{HOST_CRATES_IO}"
# docs_rs_origin = "{SCHEME_HTTPS}://{HOST_DOCS_RS}"
# allow_loopback = false   # true only for offline wiremock (CLI --allow-loopback also works)
# cache_ttl_secs = {DEFAULT_CACHE_TTL_SECS}
# max_cache_bytes = {DEFAULT_MAX_CACHE_BYTES}
# no_cache = false
"#
    )
}

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
    pub contact: Option<String>,
    /// Preferred human-message locale (`en` or `pt-BR`).
    pub lang: Option<String>,
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

/// Read `config.toml` with a hard size cap (poisoned-config / TOML bomb guard).
///
/// Uses `fs::read` + length check (not unbounded `read_to_string`). Metadata
/// size is an early reject; the post-read length is authoritative (TOCTOU-safe).
///
/// # Errors
///
/// Returns [`ErrorKind::Config`] when the file is unreadable, over
/// [`MAX_CONFIG_TOML_BYTES`], or not valid UTF-8.
fn read_config_toml_capped(path: &Path) -> AppResult<String> {
    if let Ok(meta) = fs::metadata(path)
        && meta.len() > MAX_CONFIG_TOML_BYTES
    {
        return Err(AppError::new(
            ErrorKind::Config,
            format!(
                "config.toml exceeds max size ({MAX_CONFIG_TOML_BYTES} bytes)"
            ),
        ));
    }
    let bytes = fs::read(path).map_err(|e| {
        AppError::with_source(
            ErrorKind::Config,
            format!("failed to read {}", path.display()),
            e,
        )
    })?;
    if (bytes.len() as u64) > MAX_CONFIG_TOML_BYTES {
        return Err(AppError::new(
            ErrorKind::Config,
            format!(
                "config.toml exceeds max size ({MAX_CONFIG_TOML_BYTES} bytes)"
            ),
        ));
    }
    String::from_utf8(bytes).map_err(|e| {
        AppError::with_source(
            ErrorKind::Config,
            "config.toml is not valid UTF-8",
            e,
        )
    })
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
    /// Returns [`ErrorKind::Config`] when the TOML file is unreadable or invalid.
    pub fn load(config_dir_override: Option<PathBuf>) -> AppResult<Self> {
        Self::load_with_cache_dir(config_dir_override, None)
    }

    /// Load config with optional config-dir and cache-dir overrides (CLI / tests).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Config`] when `config.toml` is unreadable or invalid TOML.
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
    /// Returns [`ErrorKind::Config`] when `config.toml` is unreadable or invalid TOML.
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
            let path = dir.join("config.toml");
            if path.is_file() {
                let text = read_config_toml_capped(&path)?;
                let parsed: TomlConfig = toml::from_str(&text).map_err(|e| {
                    AppError::with_source(ErrorKind::Config, "invalid config.toml", e)
                })?;
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

    /// Re-validate origin fields after TOML/CLI mutation (fail-fast config).
    ///
    /// # Errors
    ///
    /// Returns [`ErrorKind::Config`] when either origin is not a valid allowlisted
    /// HTTP(S) URL (SSRF gate at config boundary).
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
    /// Returns [`ErrorKind::Config`] on allowlist, contact, or User-Agent policy failures.
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
        self.config_dir.as_ref().map(|d| d.join("config.toml"))
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
            self.user_agent = self
                .user_agent
                .chars()
                .take(MAX_USER_AGENT_CHARS)
                .collect();
        }
    }

    fn apply_toml(&mut self, t: TomlConfig) -> AppResult<()> {
        if let Some(v) = t.timeout_secs {
            self.timeout_secs = v;
        }
        if let Some(v) = t.connect_timeout_secs {
            self.connect_timeout_secs = v;
        }
        if let Some(v) = t.max_body_bytes {
            // Fail-closed for operator honesty (GAP-X-005): TOML cannot raise above HARD_MAX.
            if v > HARD_MAX_BODY_BYTES {
                return Err(AppError::new(
                    ErrorKind::Config,
                    format!("max_body_bytes exceeds hard maximum ({HARD_MAX_BODY_BYTES})"),
                ));
            }
            self.max_body_bytes = v;
        }
        if let Some(v) = t.max_output_bytes {
            if v > HARD_MAX_OUTPUT_BYTES {
                return Err(AppError::new(
                    ErrorKind::Config,
                    format!("max_output_bytes exceeds hard maximum ({HARD_MAX_OUTPUT_BYTES})"),
                ));
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

/// Result of `config init`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigInitResult {
    /// Absolute path written (or already present when not forced).
    pub path: String,
    /// Directory that holds the file.
    pub config_dir: String,
    /// Layer that resolved the directory.
    pub source: PathSource,
    /// True when the file was created or overwritten in this call.
    pub created: bool,
    /// True when an existing file was replaced (`--force`).
    pub overwritten: bool,
}

/// Create default `config.toml` under the resolved config directory.
///
/// # Errors
///
/// - [`ErrorKind::Config`] when the directory cannot be resolved or created,
///   when the file already exists without `force`, or on I/O failure.
pub fn init_config_toml(
    config_dir_override: Option<PathBuf>,
    force: bool,
) -> AppResult<ConfigInitResult> {
    let (dir, source) = resolve_config_dir_with_source(config_dir_override);
    let dir = dir.ok_or_else(|| {
        AppError::new(
            ErrorKind::Config,
            "config directory could not be resolved (set --config-dir or ensure XDG config home)",
        )
    })?;
    fs::create_dir_all(&dir).map_err(|e| {
        AppError::with_source(
            ErrorKind::Config,
            format!("failed to create config dir {}", dir.display()),
            e,
        )
    })?;
    restrict_private_dir(&dir);

    let path = dir.join("config.toml");
    let existed = path.is_file();
    if existed && !force {
        return Err(AppError::new(
            ErrorKind::Config,
            format!(
                "config already exists: {} (pass --force to overwrite)",
                path.display()
            ),
        ));
    }

    // Atomic-ish write: tempfile in same dir then rename.
    let tmp = dir.join("config.toml.docsrs-cli.tmp");
    let template = default_config_toml();
    {
        let mut f = fs::File::create(&tmp).map_err(|e| {
            AppError::with_source(
                ErrorKind::Config,
                format!("failed to create {}", tmp.display()),
                e,
            )
        })?;
        f.write_all(template.as_bytes()).map_err(|e| {
            AppError::with_source(
                ErrorKind::Config,
                format!("failed to write {}", tmp.display()),
                e,
            )
        })?;
        f.sync_all().map_err(|e| {
            AppError::with_source(
                ErrorKind::Config,
                format!("failed to sync {}", tmp.display()),
                e,
            )
        })?;
    }
    restrict_private_file(&tmp);
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::with_source(
            ErrorKind::Config,
            format!("failed to install {}", path.display()),
            e,
        )
    })?;
    restrict_private_file(&path);

    Ok(ConfigInitResult {
        path: path.display().to_string(),
        config_dir: dir.display().to_string(),
        source,
        created: true,
        overwritten: existed,
    })
}

/// Path inventory for `config path` (agent-readable).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigPathData {
    /// Resolved config directory, if any.
    pub config_dir: Option<String>,
    /// Layer that resolved the config directory.
    pub config_source: PathSource,
    /// Absolute `config.toml` path (even if the file does not exist yet).
    pub config_file: Option<String>,
    /// Whether `config.toml` is present on disk.
    pub config_file_exists: bool,
    /// Whether load applied TOML keys (same as exists for normal runs).
    pub config_toml_loaded: bool,
    /// Resolved cache directory, if any.
    pub cache_dir: Option<String>,
    /// Layer that resolved the cache directory.
    pub cache_source: PathSource,
    /// Product does not use `.env` at runtime after install.
    pub dotenv_runtime: bool,
    /// Product stores no API keys; keyring/cloud secret layers are unused.
    pub secrets_layers: &'static str,
}

/// Build path inventory from a loaded [`Config`].
pub fn config_path_data(cfg: &Config) -> ConfigPathData {
    let config_file = cfg.config_file_path();
    let exists = config_file.as_ref().is_some_and(|p| p.is_file());
    ConfigPathData {
        config_dir: cfg.config_dir.as_ref().map(|p| p.display().to_string()),
        config_source: cfg.config_path_source,
        config_file: config_file.as_ref().map(|p| p.display().to_string()),
        config_file_exists: exists,
        config_toml_loaded: cfg.config_toml_loaded,
        cache_dir: cfg.cache_dir.as_ref().map(|p| p.display().to_string()),
        cache_source: cfg.cache_path_source,
        dotenv_runtime: false,
        secrets_layers: "none (public HTTP only; no API keys)",
    }
}

/// Ensure `path` is under a directory (used by tests / path display).
pub fn config_toml_under(dir: &Path) -> PathBuf {
    dir.join("config.toml")
}

impl Config {
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

