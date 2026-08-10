//! Apply parsed CLI flags onto the loaded [`Config`].
//!
//! Precedence is defaults → XDG `config.toml` → CLI flags, so this runs last and
//! wins. Explicit budgets are **fail-closed**: a value above a hard ceiling is an
//! error (exit 65), never a silent clamp, because silently shrinking a cap the
//! caller asked for makes the resulting truncation look like upstream behaviour.

use super::Cli;
use crate::config::{Config, HARD_MAX_BODY_BYTES, HARD_MAX_OUTPUT_BYTES, validate_user_agent};
use crate::error::{AppError, AppResult, ErrorDetail, Subject, ValueSource};

/// Overlay `cli` flags onto `cfg`, then re-apply the hard resource ceilings.
///
/// # Errors
///
/// Returns [`crate::error::ErrorKind::InvalidInput`] for a zero timeout or a budget above its
/// hard maximum, and [`crate::error::ErrorKind::Config`] when `--user-agent` fails validation.
pub fn apply_cli_overrides(cli: &Cli, cfg: &mut Config) -> AppResult<()> {
    if let Some(t) = cli.timeout {
        if t == 0 {
            return Err(AppError::of(ErrorDetail::MustBeAtLeastOneSecond {
                subject: Subject::Timeout,
                source: ValueSource::CommandLine,
            }));
        }
        cfg.timeout_secs = t;
    }
    if let Some(t) = cli.connect_timeout {
        if t == 0 {
            return Err(AppError::of(ErrorDetail::MustBeAtLeastOneSecond {
                subject: Subject::ConnectTimeout,
                source: ValueSource::CommandLine,
            }));
        }
        cfg.connect_timeout_secs = t;
    }
    if let Some(ref ua) = cli.user_agent {
        validate_user_agent(ua)?;
        cfg.user_agent.clone_from(ua);
    }
    if let Some(b) = cli.max_body_bytes {
        // Fail-closed: never silently clamp explicit CLI budgets (GAP-X-005).
        if b > HARD_MAX_BODY_BYTES {
            return Err(AppError::of(ErrorDetail::AboveHardMaximum {
                subject: Subject::MaxBodyBytes,
                hard_max: HARD_MAX_BODY_BYTES,
                source: ValueSource::CommandLine,
            }));
        }
        cfg.max_body_bytes = b;
    }
    if let Some(b) = cli.agent.max_output_bytes {
        if b > HARD_MAX_OUTPUT_BYTES {
            return Err(AppError::of(ErrorDetail::AboveHardMaximum {
                subject: Subject::MaxOutputBytes,
                hard_max: HARD_MAX_OUTPUT_BYTES,
                source: ValueSource::CommandLine,
            }));
        }
        cfg.max_output_bytes = b;
    }
    if let Some(d) = cli.rate_limit_delay_ms {
        cfg.rate_limit_delay_ms = d;
    }
    if let Some(n) = cli.max_concurrency {
        cfg.max_concurrency = n;
    }
    if let Some(r) = cli.max_retries {
        cfg.max_retries = r;
    }
    if let Some(ms) = cli.retry_base_ms {
        cfg.retry_base_ms = ms;
    }
    if let Some(ms) = cli.retry_max_delay_ms {
        cfg.retry_max_delay_ms = ms;
    }
    if let Some(ms) = cli.retry_max_elapsed_ms {
        cfg.retry_max_elapsed_ms = ms;
    }
    if cli.disable_retry {
        cfg.disable_retry = true;
    }
    if let Some(ref l) = cli.lang {
        match &mut cfg.lang {
            Some(dst) => dst.clone_from(l),
            None => cfg.lang = Some(l.clone()),
        }
    }
    if cli.no_cache {
        cfg.no_cache = true;
    }
    if cli.allow_loopback {
        cfg.allow_loopback = true;
    }
    if let Some(ttl) = cli.cache_ttl_secs {
        cfg.cache_ttl_secs = ttl;
    }
    if let Some(max) = cli.max_cache_bytes {
        cfg.max_cache_bytes = max;
    }
    if let Some(ref dir) = cli.cache_dir {
        cfg.cache_dir = Some(dir.clone());
    }
    // After all CLI mutations: hard ceiling (cannot elevate above HARD_MAX_*).
    cfg.clamp_resource_limits();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::ErrorKind;
    use clap::Parser;

    fn cfg_from(args: &[&str]) -> AppResult<Config> {
        let cli = Cli::try_parse_from(args).expect("parse argv");
        let mut cfg = Config::default();
        apply_cli_overrides(&cli, &mut cfg)?;
        Ok(cfg)
    }

    #[test]
    fn zero_timeouts_are_rejected() {
        for args in [
            ["docsrs-cli", "version", "--timeout", "0"],
            ["docsrs-cli", "version", "--connect-timeout", "0"],
        ] {
            let e = cfg_from(&args).expect_err("must reject zero");
            assert_eq!(e.kind(), ErrorKind::InvalidInput);
            assert_eq!(e.kind().exit_code(), 65);
        }
    }

    #[test]
    fn budgets_above_hard_max_fail_closed() {
        let over_body = (HARD_MAX_BODY_BYTES + 1).to_string();
        let e = cfg_from(&["docsrs-cli", "version", "--max-body-bytes", &over_body])
            .expect_err("must reject");
        assert_eq!(e.kind(), ErrorKind::InvalidInput);
        let over_out = (HARD_MAX_OUTPUT_BYTES + 1).to_string();
        let e = cfg_from(&["docsrs-cli", "version", "--max-output-bytes", &over_out])
            .expect_err("must reject");
        assert_eq!(e.kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn scalar_flags_land_on_config() {
        let cfg = cfg_from(&[
            "docsrs-cli",
            "version",
            "--timeout",
            "42",
            "--max-retries",
            "7",
            "--rate-limit-delay-ms",
            "250",
            "--no-cache",
            "--disable-retry",
        ])
        .expect("valid overrides");
        assert_eq!(cfg.timeout_secs, 42);
        assert_eq!(cfg.max_retries, 7);
        assert_eq!(cfg.rate_limit_delay_ms, 250);
        assert!(cfg.no_cache);
        assert!(cfg.disable_retry);
    }

    #[test]
    fn lang_overwrites_existing_config_value() {
        let cli = Cli::try_parse_from(["docsrs-cli", "version", "--lang", "pt-BR"]).expect("parse");
        let mut cfg = Config {
            lang: Some("en".into()),
            ..Config::default()
        };
        apply_cli_overrides(&cli, &mut cfg).expect("valid");
        assert_eq!(cfg.lang.as_deref(), Some("pt-BR"));
    }

    #[test]
    fn absent_flags_leave_defaults_untouched() {
        let base = Config::default();
        let cfg = cfg_from(&["docsrs-cli", "version"]).expect("valid");
        assert_eq!(cfg.timeout_secs, base.timeout_secs);
        assert_eq!(cfg.max_retries, base.max_retries);
        assert_eq!(cfg.user_agent, base.user_agent);
    }
}
