//! Configuration: defaults, XDG TOML, env allowlist, CLI override.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorKind};
use crate::platform::{restrict_private_dir, restrict_private_file};

/// Default wall-clock timeout (seconds) for one operation.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default TCP connect timeout (seconds).
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Default maximum downloaded response body size (bytes).
pub const DEFAULT_MAX_BODY_BYTES: u64 = 10_485_760;
/// Hard ceiling for HTTP body size (CLI / TOML / env cannot raise above this).
///
/// Equal to [`DEFAULT_MAX_BODY_BYTES`] so operators may only lower the cap.
/// Prevents multi-GiB allocations from untrusted Content-Length or config.
pub const HARD_MAX_BODY_BYTES: u64 = DEFAULT_MAX_BODY_BYTES;
/// Default maximum emitted stdout payload size (bytes).
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 2_097_152;
/// Hard ceiling for emitted stdout payload size (CLI / TOML / env cannot raise above).
pub const HARD_MAX_OUTPUT_BYTES: u64 = DEFAULT_MAX_OUTPUT_BYTES;
/// Default maximum HTTP redirects followed.
pub const DEFAULT_MAX_REDIRECTS: u32 = 5;
/// Default maximum retries for transient HTTP errors.
pub const DEFAULT_MAX_RETRIES: u32 = crate::retry::DEFAULT_MAX_RETRIES;
/// Base backoff delay (milliseconds) for retries.
pub const DEFAULT_RETRY_BASE_MS: u64 = crate::retry::DEFAULT_RETRY_BASE_MS;
/// Default ceiling for a single retry sleep (milliseconds).
pub const DEFAULT_RETRY_MAX_DELAY_MS: u64 = crate::retry::DEFAULT_RETRY_MAX_DELAY_MS;
/// Default minimum delay between requests to the same host (milliseconds).
pub const DEFAULT_RATE_LIMIT_DELAY_MS: u64 = 1000;
/// Default `per_page` for crates.io search.
pub const DEFAULT_PER_PAGE: u32 = 10;
/// Default hit limit for search-in-crate.
pub const DEFAULT_SEARCH_LIMIT: u32 = 100;
/// Maximum `per_page` accepted for crates.io search.
pub const MAX_PER_PAGE: u32 = 100;
/// Maximum hit limit for search-in-crate.
pub const MAX_SEARCH_IN_CRATE_LIMIT: u32 = 1000;
/// Maximum query string length.
pub const MAX_QUERY_CHARS: usize = 256;
/// Maximum crate name length.
pub const MAX_CRATE_NAME_CHARS: usize = 64;
/// Maximum rustdoc item path length.
pub const MAX_ITEM_PATH_CHARS: usize = 512;
/// Maximum version string length.
pub const MAX_VERSION_CHARS: usize = 64;
/// Maximum User-Agent length.
pub const MAX_USER_AGENT_CHARS: usize = 256;
/// Default HTTP disk cache TTL (seconds).
pub const DEFAULT_CACHE_TTL_SECS: u64 = crate::cache::DEFAULT_CACHE_TTL_SECS;
/// Default HTTP disk cache budget in bytes (256 MiB). `0` = unlimited.
pub const DEFAULT_MAX_CACHE_BYTES: u64 = crate::cache::DEFAULT_MAX_CACHE_BYTES;
/// Product binary / crate name.
pub const APP_NAME: &str = "docsrs-cli";
/// Package version from Cargo.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Minimum supported Rust version.
pub const MSRV: &str = "1.88.0";
/// JSON envelope `schema_version` constant.
pub const SCHEMA_VERSION: u32 = 1;

/// crates.io host allowlist entry.
pub const HOST_CRATES_IO: &str = "crates.io";
/// docs.rs host allowlist entry.
pub const HOST_DOCS_RS: &str = "docs.rs";
/// static.docs.rs host allowlist entry (redirects).
pub const HOST_STATIC_DOCS_RS: &str = "static.docs.rs";
/// Official stdlib rustdoc host (`std` / `core` / `alloc`).
pub const HOST_DOC_RUST_LANG_ORG: &str = "doc.rust-lang.org";

/// Which layer won when resolving a storage path (XDG hierarchy diagnostics).
///
/// Product has no API keys; layers for OS keyring / cloud secret managers are
/// out of scope. Precedence for paths: CLI/env dir override → `DOCSRS_CLI_HOME`
/// sandbox → `ProjectDirs` XDG/AppData/Library.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PathSource {
    /// `--config-dir` / `--cache-dir` or clap env aliases for those flags.
    CliOrEnv,
    /// `{DOCSRS_CLI_HOME}/config` or `{DOCSRS_CLI_HOME}/cache`.
    HomeSandbox,
    /// `directories::ProjectDirs` platform defaults.
    Xdg,
    /// No path could be resolved.
    #[default]
    Unresolved,
}

impl PathSource {
    /// Stable machine token for doctor / config path JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CliOrEnv => "cli-or-env",
            Self::HomeSandbox => "home-sandbox",
            Self::Xdg => "xdg",
            Self::Unresolved => "unresolved",
        }
    }
}

/// Default `config.toml` body written by `config init` (commented template).
///
/// Values are documentation only until the user uncomments. Runtime still
/// applies defaults when keys are absent. No secrets — product is public HTTP.
pub const DEFAULT_CONFIG_TOML: &str = r#"# docsrs-cli XDG configuration
# Precedence: CLI flags > this file (XDG) > built-in defaults.
# Path: /docsrs-cli/config.toml (or platform equivalent).
# No .env file is required after cargo install. Product stores no API keys.
# Product settings are NOT read from DOCSRS_CLI_* environment variables.
#
# timeout_secs = 30
# connect_timeout_secs = 10
# max_body_bytes = 10485760
# max_output_bytes = 2097152
# max_redirects = 5
# max_retries = 3
# retry_base_ms = 200
# retry_max_delay_ms = 30000
# disable_retry = false
# rate_limit_delay_ms = 1000
# max_concurrency = 0
# user_agent = "docsrs-cli/0.1.2 (+https://github.com/danilo-aguiar-br/docsrs-cli)"
# contact = "https://github.com/danilo-aguiar-br/docsrs-cli"
# lang = "en"
# crates_io_origin = "https://crates.io"
# docs_rs_origin = "https://docs.rs"
# cache_ttl_secs = 86400
# max_cache_bytes = 268435456
# no_cache = false
"#;

/// Runtime configuration resolved from defaults, XDG TOML, env, and CLI.
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
    pub crates_io_origin: String,
    /// Origin for docs.rs pages (default `https://docs.rs`). Overridable for offline mocks.
    pub docs_rs_origin: String,
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
            crates_io_origin: format!("https://{HOST_CRATES_IO}"),
            docs_rs_origin: format!("https://{HOST_DOCS_RS}"),
            config_path_source: PathSource::Unresolved,
            cache_path_source: PathSource::Unresolved,
            config_toml_loaded: false,
        }
    }
}

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
        _ => format!("{APP_NAME}/{APP_VERSION} (+https://github.com/danilo-aguiar-br/docsrs-cli)"),
    }
}

/// Strip trailing slash from an origin base URL.
pub fn normalize_origin(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

#[derive(Debug, Default, Deserialize)]
struct TomlConfig {
    timeout_secs: Option<u64>,
    connect_timeout_secs: Option<u64>,
    max_body_bytes: Option<u64>,
    max_output_bytes: Option<u64>,
    max_redirects: Option<u32>,
    max_retries: Option<u32>,
    retry_base_ms: Option<u64>,
    retry_max_delay_ms: Option<u64>,
    disable_retry: Option<bool>,
    rate_limit_delay_ms: Option<u64>,
    max_concurrency: Option<u32>,
    user_agent: Option<String>,
    contact: Option<String>,
    lang: Option<String>,
    crates_io_origin: Option<String>,
    docs_rs_origin: Option<String>,
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
        let mut cfg = Self::default();

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
                let text = fs::read_to_string(&path).map_err(|e| {
                    AppError::with_source(
                        ErrorKind::Config,
                        format!("failed to read {}", path.display()),
                        e,
                    )
                })?;
                let parsed: TomlConfig = toml::from_str(&text).map_err(|e| {
                    AppError::with_source(ErrorKind::Config, "invalid config.toml", e)
                })?;
                cfg.apply_toml(parsed);
                cfg.config_toml_loaded = true;
            }
        }
        cfg.config_dir = config_dir;

        cfg.apply_env()?;
        cfg.clamp_resource_limits();
        Ok(cfg)
    }

    /// Absolute path to `config.toml` under the resolved config directory, if any.
    pub fn config_file_path(&self) -> Option<PathBuf> {
        self.config_dir.as_ref().map(|d| d.join("config.toml"))
    }

    /// Clamp body/output budgets so config, env, or CLI cannot force multi-GiB allocs.
    ///
    /// Call after every mutation path (TOML, env, CLI). Values above the hard
    /// ceilings are silently reduced to [`HARD_MAX_BODY_BYTES`] /
    /// [`HARD_MAX_OUTPUT_BYTES`]. Zero is left as-is (fail-closed for body reads).
    pub fn clamp_resource_limits(&mut self) {
        self.max_body_bytes = self.max_body_bytes.min(HARD_MAX_BODY_BYTES);
        self.max_output_bytes = self.max_output_bytes.min(HARD_MAX_OUTPUT_BYTES);
        self.max_retries = self.max_retries.min(crate::retry::HARD_MAX_RETRIES);
        if self.retry_base_ms == 0 {
            self.retry_base_ms = DEFAULT_RETRY_BASE_MS;
        }
        if self.retry_max_delay_ms == 0 {
            self.retry_max_delay_ms = DEFAULT_RETRY_MAX_DELAY_MS;
        }
        self.retry_max_delay_ms = self
            .retry_max_delay_ms
            .min(crate::retry::HARD_MAX_DELAY_MS)
            .max(self.retry_base_ms);
    }

    fn apply_toml(&mut self, t: TomlConfig) {
        if let Some(v) = t.timeout_secs {
            self.timeout_secs = v;
        }
        if let Some(v) = t.connect_timeout_secs {
            self.connect_timeout_secs = v;
        }
        if let Some(v) = t.max_body_bytes {
            self.max_body_bytes = v;
        }
        if let Some(v) = t.max_output_bytes {
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
        if let Some(v) = t.crates_io_origin {
            self.crates_io_origin = normalize_origin(&v);
        }
        if let Some(v) = t.docs_rs_origin {
            self.docs_rs_origin = normalize_origin(&v);
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
    }

    /// Resolve cache path when still unset (XDG / test sandbox).
    ///
    /// Product knobs are **not** read from `DOCSRS_CLI_*` environment variables
    /// (flags + XDG TOML only). Path sandbox `DOCSRS_CLI_HOME` remains available
    /// for tests via [`crate::cache::resolve_cache_dir_with_source`].
    fn apply_env(&mut self) -> AppResult<()> {
        if self.cache_dir.is_none() {
            let (dir, source) = crate::cache::resolve_cache_dir_with_source(None);
            self.cache_dir = dir;
            self.cache_path_source = source;
        }
        Ok(())
    }
}

/// Resolve config directory: CLI/env override, then `DOCSRS_CLI_HOME/config`, then XDG.
///
/// Precedence:
/// 1. Explicit override (`--config-dir` / `DOCSRS_CLI_CONFIG_DIR`)
/// 2. `{DOCSRS_CLI_HOME}/config` when `DOCSRS_CLI_HOME` is set (sandbox / tests)
/// 3. `directories::ProjectDirs` config dir (platform XDG / AppData / Library)
pub fn resolve_config_dir(override_dir: Option<PathBuf>) -> Option<PathBuf> {
    resolve_config_dir_with_source(override_dir).0
}

/// Resolve config directory and report which layer won.
pub fn resolve_config_dir_with_source(
    override_dir: Option<PathBuf>,
) -> (Option<PathBuf>, PathSource) {
    if let Some(p) = override_dir {
        return (Some(p), PathSource::CliOrEnv);
    }
    if let Some(p) = std::env::var_os("DOCSRS_CLI_CONFIG_DIR") {
        return (Some(PathBuf::from(p)), PathSource::CliOrEnv);
    }
    if let Some(p) = home_config_dir() {
        return (Some(p), PathSource::HomeSandbox);
    }
    if let Some(p) = ProjectDirs::from("", "", APP_NAME).map(|d| d.config_dir().to_path_buf()) {
        return (Some(p), PathSource::Xdg);
    }
    (None, PathSource::Unresolved)
}

/// `{DOCSRS_CLI_HOME}/config` when the home override is set.
fn home_config_dir() -> Option<PathBuf> {
    std::env::var_os("DOCSRS_CLI_HOME").map(|h| PathBuf::from(h).join("config"))
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
            "config directory could not be resolved (set --config-dir or DOCSRS_CLI_CONFIG_DIR)",
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
    {
        let mut f = fs::File::create(&tmp).map_err(|e| {
            AppError::with_source(
                ErrorKind::Config,
                format!("failed to create {}", tmp.display()),
                e,
            )
        })?;
        f.write_all(DEFAULT_CONFIG_TOML.as_bytes()).map_err(|e| {
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

/// Validate crate name (compat wrapper over [`crate::domain::CrateName::parse`]).
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] from [`crate::domain::CrateName::parse`].
pub fn validate_crate_name(name: &str) -> AppResult<()> {
    crate::domain::CrateName::parse(name).map(|_| ())
}

/// True for Rust standard library crates documented on `doc.rust-lang.org`.
pub fn is_stdlib_crate(name: &str) -> bool {
    matches!(name, "std" | "core" | "alloc")
}

/// Resolve docs origin for a crate: stdlib → doc.rust-lang.org, else configured docs.rs origin.
pub fn docs_origin_for_crate(cfg: &Config, crate_name: &str) -> String {
    if is_stdlib_crate(crate_name) {
        format!("https://{HOST_DOC_RUST_LANG_ORG}")
    } else {
        cfg.docs_rs_origin.clone()
    }
}

/// Map CLI version token to a rustdoc channel for stdlib paths.
///
/// `latest` → `stable`. Accepts `stable` / `beta` / `nightly` and other path-safe tokens.
pub fn stdlib_channel(version: &str) -> &str {
    match version {
        "latest" => "stable",
        other => other,
    }
}

/// Validate search query (compat wrapper over [`crate::domain::SearchQuery::parse`]).
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] from [`crate::domain::SearchQuery::parse`].
pub fn validate_query(query: &str, allow_empty: bool) -> AppResult<()> {
    crate::domain::SearchQuery::parse(query, allow_empty).map(|_| ())
}

/// Validate item path (compat wrapper over [`crate::domain::ItemPath::parse`]).
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] from [`crate::domain::ItemPath::parse`].
pub fn validate_item_path(path: &str) -> AppResult<Vec<String>> {
    crate::domain::ItemPath::parse(path).map(|p| p.into_segments())
}

/// Resolve version argument: `latest` or SemVer without `v` prefix / build metadata.
///
/// # Errors
///
/// Propagates [`ErrorKind::InvalidInput`] from [`crate::domain::VersionArg::parse_opt`].
pub fn resolve_version_arg(raw: Option<&str>) -> AppResult<String> {
    crate::domain::VersionArg::parse_opt(raw).map(|v| v.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn crate_name_rules() {
        assert!(validate_crate_name("serde").is_ok());
        assert!(validate_crate_name("async-trait").is_ok());
        // std/core/alloc are valid; docs are fetched from doc.rust-lang.org.
        assert!(validate_crate_name("std").is_ok());
        assert!(validate_crate_name("core").is_ok());
        assert!(validate_crate_name("alloc").is_ok());
        assert!(is_stdlib_crate("std"));
        assert!(!is_stdlib_crate("serde"));
        assert!(validate_crate_name("").is_err());
        assert!(validate_crate_name("1bad").is_err());
        assert!(validate_crate_name(&"a".repeat(MAX_CRATE_NAME_CHARS + 1)).is_err());
    }

    #[test]
    fn version_policy() {
        assert_eq!(resolve_version_arg(None).unwrap(), "latest");
        assert_eq!(resolve_version_arg(Some("")).unwrap(), "latest");
        assert_eq!(resolve_version_arg(Some("1.0.0")).unwrap(), "1.0.0");
        assert_eq!(
            resolve_version_arg(Some("1.0.0-rc.1")).unwrap(),
            "1.0.0-rc.1"
        );
        assert!(resolve_version_arg(Some("v1.0.0")).is_err());
        assert!(resolve_version_arg(Some("1.0.0+build")).is_err());
        assert!(resolve_version_arg(Some("not-a-version")).is_err());
    }

    #[test]
    fn query_and_path_rules() {
        assert!(validate_query("serde", false).is_ok());
        assert!(validate_query("", false).is_err());
        assert!(validate_query("", true).is_ok());
        assert!(validate_query(&"q".repeat(MAX_QUERY_CHARS + 1), true).is_err());
        let segs = validate_item_path("clap::Parser").unwrap();
        assert_eq!(segs, vec!["clap", "Parser"]);
        assert!(validate_item_path("::").is_err());
        assert!(validate_item_path("").is_err());
        assert!(validate_item_path(&format!("a::{}", "b".repeat(MAX_ITEM_PATH_CHARS))).is_err());
    }

    #[test]
    fn item_path_accepts_slash_and_rejects_bad_segments() {
        let segs = validate_item_path("runtime/Runtime").unwrap();
        assert_eq!(segs, vec!["runtime", "Runtime"]);
        let mixed = validate_item_path("tokio::runtime/Runtime").unwrap();
        assert_eq!(mixed, vec!["tokio", "runtime", "Runtime"]);
        assert_eq!(
            validate_item_path("async_trait").unwrap(),
            vec!["async_trait"]
        );
        assert!(validate_item_path("has space").is_err());
        assert!(validate_item_path("foo.bar").is_err());
        assert!(validate_item_path("..").is_err());
        assert!(validate_item_path("a/../b").is_err());
        assert!(validate_item_path("/").is_err());
    }

    #[test]
    fn default_user_agent_variants() {
        let ua = default_user_agent(None);
        assert!(ua.contains(APP_NAME));
        let ua2 = default_user_agent(Some("dev@example.com"));
        assert!(ua2.contains("dev@example.com"));
        let ua3 = default_user_agent(Some("https://example.com"));
        assert!(ua3.contains("+https://example.com"));
    }

    #[test]
    fn config_load_toml_and_durations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            "timeout_secs = 12\nrate_limit_delay_ms = 50\ncontact = \"a@b.c\"\ncrates_io_origin = \"http://127.0.0.1:9/\"\ndocs_rs_origin = \"http://127.0.0.1:9/\"\nlang = \"pt-BR\"\nmax_retries = 2\nmax_body_bytes = 1000\nmax_output_bytes = 2000\ncache_ttl_secs = 3600\nno_cache = false\n"
        )
        .unwrap();
        let cache_dir = dir.path().join("cache");
        let cfg =
            Config::load_with_cache_dir(Some(dir.path().to_path_buf()), Some(cache_dir.clone()))
                .unwrap();
        assert_eq!(cfg.timeout_secs, 12);
        assert_eq!(cfg.rate_limit_delay_ms, 50);
        assert!(cfg.user_agent.contains("a@b.c"));
        assert_eq!(cfg.timeout(), Duration::from_secs(12));
        assert_eq!(cfg.rate_limit_delay(), Duration::from_millis(50));
        assert!(cfg.connect_timeout() >= Duration::from_secs(1));
        assert_eq!(cfg.crates_io_origin, "http://127.0.0.1:9");
        assert_eq!(cfg.docs_rs_origin, "http://127.0.0.1:9");
        assert_eq!(cfg.lang.as_deref(), Some("pt-BR"));
        assert_eq!(cfg.max_retries, 2);
        assert_eq!(cfg.max_body_bytes, 1000);
        assert_eq!(cfg.max_output_bytes, 2000);
        assert_eq!(cfg.cache_ttl_secs, 3600);
        assert_eq!(cfg.cache_dir.as_deref(), Some(cache_dir.as_path()));
        assert!(cfg.cache_enabled());
    }

    #[test]
    fn config_invalid_toml_errors() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "timeout_secs = [\n").unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
    }

    #[test]
    fn normalize_origin_trims_slash() {
        assert_eq!(normalize_origin(" https://docs.rs/ "), "https://docs.rs");
    }

    #[test]
    fn resolve_config_dir_prefers_explicit_override() {
        let explicit = PathBuf::from("/tmp/docsrs-cli-explicit-config");
        let got = resolve_config_dir(Some(explicit.clone()));
        assert_eq!(got.as_deref(), Some(explicit.as_path()));
    }

    #[test]
    fn resolve_cache_dir_home_layout() {
        // Pure path join contract (no env mutation): HOME root + "cache".
        let home = PathBuf::from("/tmp/docsrs-cli-home-sandbox");
        let cache = home.join("cache");
        assert_eq!(cache.file_name().and_then(|s| s.to_str()), Some("cache"));
        // resolve_cache_dir with explicit override must win over env/XDG.
        let got = crate::cache::resolve_cache_dir(Some(cache.clone()));
        assert_eq!(got.as_deref(), Some(cache.as_path()));
    }

    #[test]
    fn config_defaults_positive() {
        let cfg = Config::default();
        assert!(cfg.timeout_secs >= 1);
        assert!(cfg.max_body_bytes > 0);
        assert!(cfg.user_agent.contains(APP_NAME));
    }

    #[test]
    fn clamp_resource_limits_caps_body_and_output() {
        let mut cfg = Config {
            max_body_bytes: HARD_MAX_BODY_BYTES.saturating_mul(8),
            max_output_bytes: HARD_MAX_OUTPUT_BYTES.saturating_mul(8),
            ..Config::default()
        };
        cfg.clamp_resource_limits();
        assert_eq!(cfg.max_body_bytes, HARD_MAX_BODY_BYTES);
        assert_eq!(cfg.max_output_bytes, HARD_MAX_OUTPUT_BYTES);
        // Lowering remains allowed (tests / tight budgets).
        cfg.max_body_bytes = 64;
        cfg.max_output_bytes = 32;
        cfg.clamp_resource_limits();
        assert_eq!(cfg.max_body_bytes, 64);
        assert_eq!(cfg.max_output_bytes, 32);
    }

    #[test]
    fn toml_cannot_raise_max_body_above_hard_ceiling() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let huge_body = HARD_MAX_BODY_BYTES.saturating_mul(10);
        let huge_out = HARD_MAX_OUTPUT_BYTES.saturating_mul(10);
        fs::write(
            &path,
            format!("max_body_bytes = {huge_body}\nmax_output_bytes = {huge_out}\n"),
        )
        .unwrap();
        let cfg = Config::load(Some(dir.path().to_path_buf())).unwrap();
        assert_eq!(cfg.max_body_bytes, HARD_MAX_BODY_BYTES);
        assert_eq!(cfg.max_output_bytes, HARD_MAX_OUTPUT_BYTES);
    }

    #[test]
    fn hard_ceilings_equal_defaults() {
        // Product policy: operators may only lower body/output caps, never raise.
        assert_eq!(HARD_MAX_BODY_BYTES, DEFAULT_MAX_BODY_BYTES);
        assert_eq!(HARD_MAX_OUTPUT_BYTES, DEFAULT_MAX_OUTPUT_BYTES);
        assert_eq!(Config::default().max_body_bytes, HARD_MAX_BODY_BYTES);
        assert_eq!(Config::default().max_output_bytes, HARD_MAX_OUTPUT_BYTES);
    }

    #[test]
    fn path_source_tokens_are_stable() {
        assert_eq!(PathSource::CliOrEnv.as_str(), "cli-or-env");
        assert_eq!(PathSource::HomeSandbox.as_str(), "home-sandbox");
        assert_eq!(PathSource::Xdg.as_str(), "xdg");
        assert_eq!(PathSource::Unresolved.as_str(), "unresolved");
    }

    #[test]
    fn resolve_config_dir_with_source_reports_cli() {
        let explicit = PathBuf::from("/tmp/docsrs-cli-src-explicit");
        let (got, source) = resolve_config_dir_with_source(Some(explicit.clone()));
        assert_eq!(got.as_deref(), Some(explicit.as_path()));
        assert_eq!(source, PathSource::CliOrEnv);
    }

    #[test]
    fn init_config_toml_creates_and_refuses_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let r1 = init_config_toml(Some(dir.path().to_path_buf()), false).unwrap();
        assert!(r1.created);
        assert!(!r1.overwritten);
        assert!(Path::new(&r1.path).is_file());
        let text = fs::read_to_string(&r1.path).unwrap();
        assert!(text.contains("docsrs-cli XDG configuration"));
        assert!(text.contains("No .env file is required"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&r1.path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
        let err = init_config_toml(Some(dir.path().to_path_buf()), false).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
        let r2 = init_config_toml(Some(dir.path().to_path_buf()), true).unwrap();
        assert!(r2.created);
        assert!(r2.overwritten);
    }

    #[test]
    fn config_load_marks_toml_loaded() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("config.toml"), "timeout_secs = 9\n").unwrap();
        let cfg = Config::load(Some(dir.path().to_path_buf())).unwrap();
        assert!(cfg.config_toml_loaded);
        assert_eq!(cfg.timeout_secs, 9);
        assert_eq!(cfg.config_path_source, PathSource::CliOrEnv);
        let inv = config_path_data(&cfg);
        assert!(inv.config_file_exists);
        assert!(!inv.dotenv_runtime);
    }

    #[test]
    fn config_load_without_toml_is_not_loaded() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = Config::load(Some(dir.path().to_path_buf())).unwrap();
        assert!(!cfg.config_toml_loaded);
        assert!(!config_path_data(&cfg).config_file_exists);
    }
}
