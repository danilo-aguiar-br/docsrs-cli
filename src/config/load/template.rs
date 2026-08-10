//! Commented `config.toml` template written by `config init`.

use crate::config::constants::*;

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
# paths, lang, log_directive) are read ONLY from CLI flags and this TOML — never
# from environment variables. Isolate storage with --config-dir / --cache-dir.
#
# RUST_LOG is NOT read: stderr verbosity comes from -q / -v or log_directive.
#
# Terminal capability only (describes the device, like isatty — never product
# configuration): NO_COLOR, TERM, CLICOLOR_FORCE. --no-color outranks all three.
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
# log_directive = "{DEFAULT_LOG_DIRECTIVE}"   # e.g. "docsrs_cli=debug,docsrs_cli::http=trace"
# crates_io_origin = "{SCHEME_HTTPS}://{HOST_CRATES_IO}"
# docs_rs_origin = "{SCHEME_HTTPS}://{HOST_DOCS_RS}"
# allow_loopback = false   # true only for offline wiremock (CLI --allow-loopback also works)
# cache_ttl_secs = {DEFAULT_CACHE_TTL_SECS}
# max_cache_bytes = {DEFAULT_MAX_CACHE_BYTES}
# no_cache = false
"#
    )
}
