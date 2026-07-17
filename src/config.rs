//! Configuration: defaults, XDG TOML, env allowlist, CLI override.

use std::path::PathBuf;
use std::time::Duration;

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult, ErrorKind};

/// Default wall-clock timeout (seconds) for one operation.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default TCP connect timeout (seconds).
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Default maximum downloaded response body size (bytes).
pub const DEFAULT_MAX_BODY_BYTES: u64 = 10_485_760;
/// Default maximum emitted stdout payload size (bytes).
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 2_097_152;
/// Default maximum HTTP redirects followed.
pub const DEFAULT_MAX_REDIRECTS: u32 = 5;
/// Default maximum retries for transient HTTP errors.
pub const DEFAULT_MAX_RETRIES: u32 = 3;
/// Base backoff delay (milliseconds) for retries.
pub const DEFAULT_RETRY_BASE_MS: u64 = 200;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub timeout_secs: u64,
    pub connect_timeout_secs: u64,
    pub max_body_bytes: u64,
    pub max_output_bytes: u64,
    pub max_redirects: u32,
    pub max_retries: u32,
    pub retry_base_ms: u64,
    pub rate_limit_delay_ms: u64,
    pub user_agent: String,
    pub contact: Option<String>,
    pub lang: Option<String>,
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
}

impl Default for Config {
    fn default() -> Self {
        let contact = std::env::var("DOCSRS_CLI_CONTACT").ok();
        let ua = default_user_agent(contact.as_deref());
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            connect_timeout_secs: DEFAULT_CONNECT_TIMEOUT_SECS,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_redirects: DEFAULT_MAX_REDIRECTS,
            max_retries: DEFAULT_MAX_RETRIES,
            retry_base_ms: DEFAULT_RETRY_BASE_MS,
            rate_limit_delay_ms: DEFAULT_RATE_LIMIT_DELAY_MS,
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
        }
    }
}

pub fn default_user_agent(contact: Option<&str>) -> String {
    match contact {
        Some(c) if !c.is_empty() => {
            if c.starts_with("http://") || c.starts_with("https://") {
                format!("{APP_NAME}/{APP_VERSION} (+{c})")
            } else {
                format!("{APP_NAME}/{APP_VERSION} ({c})")
            }
        }
        _ => format!("{APP_NAME}/{APP_VERSION} (+https://github.com/docsrs-cli/docsrs-cli)"),
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
    rate_limit_delay_ms: Option<u64>,
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
    pub fn load(config_dir_override: Option<PathBuf>) -> AppResult<Self> {
        Self::load_with_cache_dir(config_dir_override, None)
    }

    /// Load config with optional config-dir and cache-dir overrides (CLI / tests).
    pub fn load_with_cache_dir(
        config_dir_override: Option<PathBuf>,
        cache_dir_override: Option<PathBuf>,
    ) -> AppResult<Self> {
        let mut cfg = Self::default();

        let config_dir = config_dir_override
            .or_else(|| std::env::var_os("DOCSRS_CLI_CONFIG_DIR").map(PathBuf::from))
            .or_else(|| ProjectDirs::from("", "", APP_NAME).map(|p| p.config_dir().to_path_buf()));
        cfg.config_dir = config_dir.clone();
        cfg.cache_dir = crate::cache::resolve_cache_dir(cache_dir_override);

        if let Some(dir) = &config_dir {
            let path = dir.join("config.toml");
            if path.is_file() {
                let text = std::fs::read_to_string(&path).map_err(|e| {
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
            }
        }

        cfg.apply_env()?;
        Ok(cfg)
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
        if let Some(v) = t.rate_limit_delay_ms {
            self.rate_limit_delay_ms = v;
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

    fn apply_env(&mut self) -> AppResult<()> {
        if let Ok(v) = std::env::var("DOCSRS_CLI_TIMEOUT_SECS") {
            self.timeout_secs = v.parse().map_err(|_| {
                AppError::new(
                    ErrorKind::Config,
                    "DOCSRS_CLI_TIMEOUT_SECS must be a positive integer",
                )
            })?;
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_MAX_OUTPUT_BYTES") {
            self.max_output_bytes = v.parse().map_err(|_| {
                AppError::new(
                    ErrorKind::Config,
                    "DOCSRS_CLI_MAX_OUTPUT_BYTES must be a positive integer",
                )
            })?;
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_USER_AGENT") {
            self.user_agent = v;
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_CONTACT") {
            self.contact = Some(v.clone());
            if std::env::var("DOCSRS_CLI_USER_AGENT").is_err() {
                self.user_agent = default_user_agent(Some(&v));
            }
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_LANG") {
            self.lang = Some(v);
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_CRATES_IO_ORIGIN") {
            self.crates_io_origin = normalize_origin(&v);
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_DOCS_RS_ORIGIN") {
            self.docs_rs_origin = normalize_origin(&v);
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_CACHE_TTL_SECS") {
            self.cache_ttl_secs = v.parse().map_err(|_| {
                AppError::new(
                    ErrorKind::Config,
                    "DOCSRS_CLI_CACHE_TTL_SECS must be a non-negative integer",
                )
            })?;
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_MAX_CACHE_BYTES") {
            self.max_cache_bytes = v.parse().map_err(|_| {
                AppError::new(
                    ErrorKind::Config,
                    "DOCSRS_CLI_MAX_CACHE_BYTES must be a non-negative integer",
                )
            })?;
        }
        if let Ok(v) = std::env::var("DOCSRS_CLI_NO_CACHE") {
            let low = v.to_ascii_lowercase();
            self.no_cache = matches!(low.as_str(), "1" | "true" | "yes" | "on");
        }
        // Env cache dir wins over XDG default when set after load_with_cache_dir
        // only if caller did not already pin cache_dir via CLI override path.
        if self.cache_dir.is_none()
            && let Some(p) = std::env::var_os("DOCSRS_CLI_CACHE_DIR")
        {
            self.cache_dir = Some(PathBuf::from(p));
        }
        Ok(())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.timeout_secs.max(1))
    }

    pub fn connect_timeout(&self) -> Duration {
        Duration::from_secs(self.connect_timeout_secs.max(1))
    }

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

/// Validate crate name against PRD regex and length.
pub fn validate_crate_name(name: &str) -> AppResult<()> {
    let name = name.trim();
    if name.is_empty() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "crate name is empty",
        ));
    }
    if name.chars().count() > MAX_CRATE_NAME_CHARS {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("crate name exceeds {MAX_CRATE_NAME_CHARS} characters"),
        ));
    }
    let re = regex::Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$").expect("valid regex");
    if !re.is_match(name) {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("invalid crate name '{name}'"),
        ));
    }
    Ok(())
}

pub fn validate_query(query: &str, allow_empty: bool) -> AppResult<()> {
    let q = query.trim();
    if !allow_empty && q.is_empty() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "search query is empty",
        ));
    }
    if q.chars().count() > MAX_QUERY_CHARS {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("query exceeds {MAX_QUERY_CHARS} characters"),
        ));
    }
    Ok(())
}

pub fn validate_item_path(path: &str) -> AppResult<Vec<String>> {
    let path = path.trim();
    if path.is_empty() || path == "::" {
        return Err(AppError::new(ErrorKind::InvalidInput, "item path is empty"));
    }
    if path.chars().count() > MAX_ITEM_PATH_CHARS {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("item path exceeds {MAX_ITEM_PATH_CHARS} characters"),
        ));
    }
    let parts: Vec<String> = path
        .split("::")
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "item path has no segments",
        ));
    }
    Ok(parts)
}

/// Resolve version argument: `latest` or SemVer without `v` prefix / build metadata.
pub fn resolve_version_arg(raw: Option<&str>) -> AppResult<String> {
    let v = raw
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("latest");
    if v.chars().count() > MAX_VERSION_CHARS {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("version exceeds {MAX_VERSION_CHARS} characters"),
        ));
    }
    if v == "latest" {
        return Ok(v.to_string());
    }
    if v.starts_with('v') || v.starts_with('V') {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "version must not start with 'v' prefix",
        ));
    }
    if v.contains('+') {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            "version build metadata is not accepted",
        ));
    }
    // loose SemVer check: major.minor.patch with optional pre-release
    let re = regex::Regex::new(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$").expect("valid regex");
    if !re.is_match(v) {
        return Err(AppError::new(
            ErrorKind::InvalidInput,
            format!("invalid version '{v}'"),
        ));
    }
    Ok(v.to_string())
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
        assert!(validate_crate_name("std").is_ok());
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
    fn config_defaults_positive() {
        let cfg = Config::default();
        assert!(cfg.timeout_secs >= 1);
        assert!(cfg.max_body_bytes > 0);
        assert!(cfg.user_agent.contains(APP_NAME));
    }
}
