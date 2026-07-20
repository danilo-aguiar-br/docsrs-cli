//! Configuration: defaults, XDG TOML, CLI override (no product env knobs).
//!
//! Split for SRP: [`constants`], [`validate`], [`load`] (paths + TOML + `Config`).

pub(crate) mod allowlist;
mod constants;
mod load;
mod path_source;
mod validate;

pub use allowlist::{is_allowlisted_host, is_allowed_origin_scheme_host, normalize_origin};
pub use constants::*;
pub use load::*;
pub use path_source::{PathSource, resolve_config_dir, resolve_config_dir_with_source};
pub use validate::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use std::path::{Path, PathBuf};
    use std::time::Duration;
    use std::fs;
    use std::io::Write;

    #[test]
    fn stdlib_crate_reexport() {
        assert!(is_stdlib_crate("std"));
        assert!(is_stdlib_crate("core"));
        assert!(is_stdlib_crate("alloc"));
        assert!(!is_stdlib_crate("serde"));
    }

    #[test]
    fn default_user_agent_variants() {
        let ua = default_user_agent(None);
        assert!(ua.contains(APP_NAME));
        assert!(ua.contains(DEFAULT_CONTACT_URL));
        let ua2 = default_user_agent(Some("dev@example.com"));
        assert!(ua2.contains("dev@example.com"));
        let ua3 = default_user_agent(Some("https://example.com"));
        assert!(ua3.contains("+https://example.com"));
    }

    #[test]
    fn validate_origin_accepts_https_and_rejects_garbage() {
        assert_eq!(
            validate_origin("https://docs.rs/").unwrap().as_str(),
            format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}")
        );
        // Loopback requires explicit allow_loopback (no env / no cfg(test) shortcut).
        assert!(validate_origin("http://127.0.0.1:9/").is_err());
        assert_eq!(
            validate_origin_with("http://127.0.0.1:9/", true)
                .unwrap()
                .as_str(),
            "http://127.0.0.1:9"
        );
        assert!(validate_origin("").is_err());
        assert!(validate_origin("not a url").is_err());
        assert!(validate_origin("ftp://docs.rs").is_err());
        // SSRF: arbitrary hosts rejected at config boundary (not only request time).
        assert!(validate_origin("https://evil.example/").is_err());
        assert!(validate_origin("http://docs.rs/").is_err()); // production host requires https
        assert!(validate_user_agent("docsrs-cli/1.0 (+https://example.com)").is_ok());
        assert!(validate_user_agent("bad\nua").is_err());
        assert!(validate_user_agent("").is_err());
        assert!(validate_user_agent(&"x".repeat(MAX_USER_AGENT_CHARS + 1)).is_err());
    }

    #[test]
    fn clamp_caps_redirects_timeouts_and_rate_limit() {
        let mut cfg = Config {
            max_redirects: 999,
            timeout_secs: HARD_MAX_TIMEOUT_SECS.saturating_mul(4),
            connect_timeout_secs: HARD_MAX_CONNECT_TIMEOUT_SECS.saturating_mul(4),
            rate_limit_delay_ms: HARD_MAX_RATE_LIMIT_DELAY_MS.saturating_mul(2),
            ..Config::default()
        };
        cfg.clamp_resource_limits();
        assert_eq!(cfg.max_redirects, HARD_MAX_REDIRECTS);
        assert_eq!(cfg.timeout_secs, HARD_MAX_TIMEOUT_SECS);
        assert_eq!(cfg.connect_timeout_secs, HARD_MAX_CONNECT_TIMEOUT_SECS);
        assert_eq!(cfg.rate_limit_delay_ms, HARD_MAX_RATE_LIMIT_DELAY_MS);
    }

    #[test]
    fn config_load_rejects_oversized_toml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let huge = format!("# {}\ntimeout_secs = 9\n", "x".repeat(MAX_CONFIG_TOML_BYTES as usize));
        fs::write(&path, huge).unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
        assert!(err.to_string().contains("max size"));
    }

    #[test]
    fn config_load_rejects_non_allowlisted_origin() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "docs_rs_origin = \"https://evil.example\"\n",
        )
        .unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
        assert!(err.to_string().contains("allowlisted"));
    }

    #[test]
    fn default_config_toml_embeds_cargo_metadata() {
        let text = default_config_toml();
        assert!(text.contains("docsrs-cli XDG configuration"));
        assert!(text.contains(APP_VERSION));
        assert!(text.contains(DEFAULT_CONTACT_URL));
        assert!(text.contains(HOST_CRATES_IO));
        assert!(text.contains(HOST_DOCS_RS));
        assert!(text.contains(&format!("{APP_NAME}/{APP_VERSION}")));
    }

    #[test]
    fn config_load_rejects_invalid_origin() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "docs_rs_origin = \"not-a-url\"\n",
        )
        .unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
    }

    #[test]
    fn config_load_toml_and_durations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut f = fs::File::create(&path).unwrap();
        writeln!(
            f,
            "timeout_secs = 12\nrate_limit_delay_ms = 50\ncontact = \"a@b.c\"\nallow_loopback = true\ncrates_io_origin = \"http://127.0.0.1:9/\"\ndocs_rs_origin = \"http://127.0.0.1:9/\"\nlang = \"pt-BR\"\nmax_retries = 2\nmax_body_bytes = 1000\nmax_output_bytes = 2000\ncache_ttl_secs = 3600\nno_cache = false\n"
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
        assert_eq!(cfg.crates_io_origin.as_str(), "http://127.0.0.1:9");
        assert_eq!(cfg.docs_rs_origin.as_str(), "http://127.0.0.1:9");
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
    fn config_load_rejects_unknown_toml_keys() {
        // Fail closed: typos must not be silently ignored (deny_unknown_fields).
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("config.toml"),
            "timeout_secs = 9\nmax_body_bytez = 1\n",
        )
        .unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Config);
    }

    #[test]
    fn validate_contact_rejects_control_and_non_ascii() {
        assert!(validate_contact("https://example.com").is_ok());
        assert!(validate_contact("a@b.c").is_ok());
        assert!(validate_contact("").is_err());
        assert!(validate_contact("bad\nline").is_err());
        assert!(validate_contact("café").is_err());
        assert!(validate_contact(&"x".repeat(MAX_USER_AGENT_CHARS + 1)).is_err());
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
        // GAP-X-005: over-hard-max TOML is fail-closed (Config error), not silent clamp.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let huge_body = HARD_MAX_BODY_BYTES.saturating_mul(10);
        let huge_out = HARD_MAX_OUTPUT_BYTES.saturating_mul(10);
        fs::write(
            &path,
            format!("max_body_bytes = {huge_body}\nmax_output_bytes = {huge_out}\n"),
        )
        .unwrap();
        let err = Config::load(Some(dir.path().to_path_buf())).expect_err("over hard max must fail");
        assert_eq!(err.kind(), crate::error::ErrorKind::Config);
        assert!(
            err.message().contains("hard maximum"),
            "message={}",
            err.message()
        );
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
        assert_eq!(PathSource::CliFlag.as_str(), "cli");
        assert_eq!(PathSource::Xdg.as_str(), "xdg");
        assert_eq!(PathSource::Unresolved.as_str(), "unresolved");
    }

    #[test]
    fn resolve_config_dir_with_source_reports_cli() {
        let explicit = PathBuf::from("/tmp/docsrs-cli-src-explicit");
        let (got, source) = resolve_config_dir_with_source(Some(explicit.clone()));
        assert_eq!(got.as_deref(), Some(explicit.as_path()));
        assert_eq!(source, PathSource::CliFlag);
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
        assert_eq!(cfg.config_path_source, PathSource::CliFlag);
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

    /// Policy: product path resolution ignores DOCSRS_CLI_* env vars.
    #[test]
    fn product_paths_ignore_docsrs_cli_env() {
        // Safety: these vars may be set in the developer shell; resolution must
        // still prefer only explicit CLI override or XDG ProjectDirs.
        let explicit = PathBuf::from("/tmp/docsrs-cli-policy-explicit-config");
        let (got, source) = resolve_config_dir_with_source(Some(explicit.clone()));
        assert_eq!(got.as_deref(), Some(explicit.as_path()));
        assert_eq!(source, PathSource::CliFlag);

        let (cache, csource) =
            crate::cache::resolve_cache_dir_with_source(Some(PathBuf::from("/tmp/docsrs-cli-policy-cache")));
        assert_eq!(
            cache.as_deref(),
            Some(Path::new("/tmp/docsrs-cli-policy-cache"))
        );
        assert_eq!(csource, PathSource::CliFlag);
    }
}
