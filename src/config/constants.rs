//! Compile-time defaults, hard caps, hosts, and app identity constants.
//!
//! No I/O. Safe for one-shot process boot before config.toml is read.

/// Default wall-clock timeout (seconds) for one operation.
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
/// Default TCP connect timeout (seconds).
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 10;
/// Default maximum downloaded response body size (bytes).
pub const DEFAULT_MAX_BODY_BYTES: u64 = 10_485_760;
/// Hard ceiling for HTTP body size (CLI / TOML cannot raise above this).
///
/// Equal to [`DEFAULT_MAX_BODY_BYTES`] so operators may only lower the cap.
/// Prevents multi-GiB allocations from untrusted Content-Length or config.
pub const HARD_MAX_BODY_BYTES: u64 = DEFAULT_MAX_BODY_BYTES;
/// Default maximum emitted stdout payload size (bytes).
pub const DEFAULT_MAX_OUTPUT_BYTES: u64 = 2_097_152;
/// Hard ceiling for emitted stdout payload size (CLI / TOML cannot raise above).
pub const HARD_MAX_OUTPUT_BYTES: u64 = DEFAULT_MAX_OUTPUT_BYTES;
/// Default maximum HTTP redirects followed.
pub const DEFAULT_MAX_REDIRECTS: u32 = 5;
/// Hard ceiling for HTTP redirects (CLI / TOML cannot raise above this).
pub const HARD_MAX_REDIRECTS: u32 = 20;
/// Hard ceiling for wall-clock timeout (seconds).
pub const HARD_MAX_TIMEOUT_SECS: u64 = 600;
/// Hard ceiling for TCP connect timeout (seconds).
pub const HARD_MAX_CONNECT_TIMEOUT_SECS: u64 = 120;
/// Hard ceiling for per-host politeness delay (milliseconds).
pub const HARD_MAX_RATE_LIMIT_DELAY_MS: u64 = 60_000;
/// Hard ceiling for on-disk `config.toml` size (poisoned-config guard).
pub const MAX_CONFIG_TOML_BYTES: u64 = 64 * 1024;
/// Default maximum retries for transient HTTP errors.
pub const DEFAULT_MAX_RETRIES: u32 = crate::retry::DEFAULT_MAX_RETRIES;
/// Base backoff delay (milliseconds) for retries.
pub const DEFAULT_RETRY_BASE_MS: u64 = crate::retry::DEFAULT_RETRY_BASE_MS;
/// Default ceiling for a single retry sleep (milliseconds).
pub const DEFAULT_RETRY_MAX_DELAY_MS: u64 = crate::retry::DEFAULT_RETRY_MAX_DELAY_MS;
/// Default total retry wall budget (`0` = derive from `timeout_secs`).
pub const DEFAULT_RETRY_MAX_ELAPSED_MS: u64 = 0;
/// Default minimum delay between requests to the same host (milliseconds).
pub const DEFAULT_RATE_LIMIT_DELAY_MS: u64 = 1000;
/// Default `tracing` filter when neither `-q` / `-v` nor `log_directive` apply.
///
/// Agent-first: stderr stays silent below `error` so a caller parsing stdout is
/// never asked to skip banner noise it did not request.
pub const DEFAULT_LOG_DIRECTIVE: &str = "error";
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
/// Maximum crate description length when mapping crates.io JSON → agent payload.
///
/// Caps hostile/oversized API strings before `max_output_bytes` emit budgeting
/// (memory: avoid multi-MB descriptions in every hit).
pub const MAX_CRATE_DESCRIPTION_CHARS: usize = 4_096;
/// Maximum length for optional URL / page-token fields from crates.io JSON
/// (`documentation`, `repository`, `homepage`, `next_page`, `prev_page`).
pub const MAX_URL_FIELD_CHARS: usize = 2_048;
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
/// Loopback IPv4 host (offline mocks when `allow_loopback` is true).
pub const HOST_LOOPBACK_IPV4: &str = "127.0.0.1";
/// Loopback hostname (offline mocks when `allow_loopback` is true).
pub const HOST_LOCALHOST: &str = "localhost";
/// HTTPS scheme token (product endpoints are TLS-only in production).
pub const SCHEME_HTTPS: &str = "https";
/// TLS port used by the `doctor --online` DNS/TCP reachability probe.
pub const HTTPS_PORT: u16 = 443;
/// File name of the optional XDG configuration file.
///
/// Four modules join this onto the config directory — loader, path reporter,
/// `config init`, and `config path`. Renaming it in three of them would leave
/// the fourth silently reading or reporting a file nobody writes.
pub const CONFIG_FILE_NAME: &str = "config.toml";
/// Historical placeholder contact host that `doctor` must reject.
///
/// Named here so the check and the message it prints read the same string:
/// two literals would let one be corrected while the other kept accusing.
pub const PLACEHOLDER_CONTACT_HOST: &str = "github.com/docsrs-cli/docsrs-cli";
/// Default contact / repository URL (from `Cargo.toml` `[package].repository`).
///
/// Single source of truth — never duplicate the GitHub URL as a string literal.
pub const DEFAULT_CONTACT_URL: &str = env!("CARGO_PKG_REPOSITORY");
/// Delay before printing a progress hint on slow network ops (seconds).
pub const PROGRESS_HINT_DELAY_SECS: u64 = 2;
/// Cancel-aware sleep / flock poll step (milliseconds).
pub const CANCEL_POLL_INTERVAL_MS: u64 = 50;
