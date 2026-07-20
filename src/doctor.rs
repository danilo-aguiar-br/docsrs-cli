//! `doctor` command: local health checks (optional online DNS probes).
//!
//! Split from `lib` dispatch so diagnostics stay SRP-isolated (Rules: componentization).
//! One-shot: no background probes; online mode does blocking DNS only for two hosts.

use std::io::Write;
use std::net::ToSocketAddrs;
use std::path::Path;
use std::process::ExitCode;

use serde::Serialize;
use tokio::time::Instant;

use crate::cache::DiskCache;
use crate::config::{APP_NAME, Config};
use crate::error::{AppResult, EXIT_CONFIG};
use crate::i18n::Locale;
use crate::output::{map_stdout_err, write_json};
use crate::render;
use crate::shutdown::duration_ms;

#[derive(Debug, Serialize)]
struct DoctorCheck {
    name: &'static str,
    ok: bool,
    detail: String,
}

/// Typed `doctor` success data (matches `docs/schemas/doctor.schema.json`).
#[derive(Debug, Serialize)]
struct DoctorData {
    ok: bool,
    checks: Vec<DoctorCheck>,
}

/// Run local (and optional online) health checks; emit JSON or human lines.
///
/// # Errors
///
/// Returns stdout write failures ([`crate::error::ErrorKind::Internal`] or broken pipe).
/// Individual check failures set `data.ok = false` and exit 78 without bubbling as `Err`.
pub(crate) fn doctor<Out: Write>(
    cfg: &Config,
    online: bool,
    wants_json: bool,
    start: Instant,
    locale: Locale,
    stdout: &mut Out,
) -> AppResult<ExitCode> {
    // Seed static checks; remaining checks are push-appended after path resolution.
    let mut checks = vec![
        DoctorCheck {
            name: "platform",
            ok: true,
            detail: crate::platform::platform_detail(),
        },
        DoctorCheck {
            name: "http_client_posture",
            ok: true,
            detail: crate::http::client_posture_detail(),
        },
        // Domain-type posture (ADR 0008): url only of the four generic domain crates.
        DoctorCheck {
            name: "domain_types",
            ok: true,
            detail: "url=2; chrono=absent; uuid=absent; rust_decimal=absent; newtypes=CrateName|VersionArg|SearchQuery|ItemPath|CrateRef|MatchMode|AllowedOrigin; wire_urls=String; time=Instant|SystemTime|httpdate".into(),
        },
        // Error-model posture (ADR 0002 / Camada O): typed AppError, no anyhow as public E.
        DoctorCheck {
            name: "error_model",
            ok: true,
            detail: "thiserror AppError+ErrorKind non_exhaustive; source=Arc; no anyhow/eyre public E; Display=message only; exit via ErrorKind; emit_error once; usage may embed clap help".into(),
        },
        // Unsafe/FFI posture (ADR 0009 / Camada P): forbid product unsafe; loopback via CLI/XDG only.
        DoctorCheck {
            name: "unsafe_posture",
            ok: true,
            detail: format!(
                "forbid(unsafe_code) lib+bin; no product FFI/extern C; libc=dev-only signal harness; loopback=cli|toml allow_loopback={} (no env); panic=abort; ADR 0009",
                cfg.allow_loopback
            ),
        },
        // Web fetch / extraction posture (ADR 0003 / Camada Q): docs client, not crawler.
        DoctorCheck {
            name: "web_fetch_posture",
            ok: true,
            detail: "one-shot docs client; GET host-allowlist; scraper+htmd; UA+politeness; source_url; robots=PROIBIDO (ADR 0003); no cookies; UTF-8 only; gzip+br; hit join=source_url same-origin".into(),
        },
        DoctorCheck {
            name: "user_agent",
            ok: !cfg.user_agent.is_empty() && cfg.user_agent.contains(APP_NAME),
            detail: cfg.user_agent.clone(),
        },
    ];

    let (config_ok, config_detail) = match &cfg.config_dir {
        Some(p) => dir_ready_check(p),
        None => (false, "default-not-resolved".into()),
    };
    checks.push(DoctorCheck {
        name: "config_dir",
        ok: config_ok,
        detail: config_detail,
    });

    // XDG storage layer diagnostics (no .env runtime; no secret layers).
    checks.push(DoctorCheck {
        name: "config_source",
        ok: cfg.config_path_source != crate::config::PathSource::Unresolved,
        detail: cfg.config_path_source.as_str().into(),
    });
    let (cfg_file_ok, cfg_file_detail) = match cfg.config_file_path() {
        Some(p) => {
            if p.is_file() {
                (
                    true,
                    format!(
                        "present loaded={} path={}",
                        cfg.config_toml_loaded,
                        p.display()
                    ),
                )
            } else {
                (
                    true,
                    format!("absent (optional; run config init) path={}", p.display()),
                )
            }
        }
        None => (false, "config file path unresolved".into()),
    };
    checks.push(DoctorCheck {
        name: "config_file",
        ok: cfg_file_ok,
        detail: cfg_file_detail,
    });
    checks.push(DoctorCheck {
        name: "cache_source",
        ok: cfg.no_cache || cfg.cache_path_source != crate::config::PathSource::Unresolved,
        detail: if cfg.no_cache {
            "n/a (--no-cache)".into()
        } else {
            cfg.cache_path_source.as_str().into()
        },
    });
    checks.push(DoctorCheck {
        name: "dotenv_runtime",
        ok: true,
        detail: "disabled (XDG + CLI flags only; no product env; no .env required after install)".into(),
    });
    checks.push(DoctorCheck {
        name: "secrets_layers",
        ok: true,
        detail: "none (public HTTP only; keyring/cloud secret managers out of scope)".into(),
    });

    // Runtime clamps both to min 1s (`Config::timeout` / `connect_timeout`); report effective values.
    let eff_timeout = cfg.timeout_secs.max(1);
    let eff_connect = cfg.connect_timeout_secs.max(1);
    checks.push(DoctorCheck {
        name: "timeouts",
        ok: true,
        detail: format!("timeout={eff_timeout}s connect={eff_connect}s"),
    });

    let resolved_conc = crate::concurrency::resolve_max_concurrency(cfg.max_concurrency);
    let workers = crate::concurrency::runtime_worker_threads();
    let max_blocking = crate::concurrency::max_blocking_threads();
    checks.push(DoctorCheck {
        name: "concurrency",
        ok: true,
        detail: format!(
            "max_concurrency={} (configured={}; 0=auto) runtime_workers={} max_blocking={} formula=min(cpus,free_ram/2/{}MiB) cap={}",
            resolved_conc,
            cfg.max_concurrency,
            workers,
            max_blocking,
            crate::concurrency::RAM_PER_TASK_MIB,
            crate::concurrency::MAX_AUTO_CONCURRENCY
        ),
    });

    let retry = crate::retry::RetryConfig::from_config(cfg);
    checks.push(DoctorCheck {
        name: "retry_policy",
        ok: true,
        detail: format!(
            "enabled={} max_retries={} base_ms={} max_delay_ms={} max_elapsed_ms={} max_attempts={} formula=full_jitter(min(base*2^n,max_delay)) retry_after=delta|http-date kill_switch=--disable-retry",
            retry.enabled,
            retry.max_retries,
            retry.base_ms,
            retry.max_delay_ms,
            retry.max_elapsed_ms,
            retry.max_attempts()
        ),
    });

    let (cache_ok, cache_detail) = if cfg.no_cache {
        (true, "disabled (--no-cache)".to_string())
    } else {
        match &cfg.cache_dir {
            Some(p) => {
                let (ready, ready_detail) = dir_ready_check(p);
                if !ready {
                    (false, ready_detail)
                } else {
                    let stats =
                        DiskCache::new(p.clone(), cfg.cache_ttl(), cfg.max_cache_bytes, cfg.allow_loopback).stats();
                    let budget = if cfg.max_cache_bytes == 0 {
                        "unlimited".to_string()
                    } else {
                        format!("{}B", cfg.max_cache_bytes)
                    };
                    (
                        true,
                        format!(
                            "enabled dir={} ttl={}s max={} entries={} used={}B parser={}",
                            p.display(),
                            cfg.cache_ttl_secs,
                            budget,
                            stats.entries,
                            stats.total_bytes,
                            crate::cache::CACHE_PARSER_VERSION
                        ),
                    )
                }
            }
            None => (false, "no cache dir resolved".to_string()),
        }
    };
    checks.push(DoctorCheck {
        name: "disk_cache",
        ok: cache_ok,
        detail: cache_detail,
    });

    // Cross-process rate-limit lock+stamp directory (same root as disk cache).
    let (rl_ok, rl_detail) = if cfg.no_cache {
        (
            true,
            "n/a (--no-cache; in-process rate limit only)".to_string(),
        )
    } else {
        match &cfg.cache_dir {
            Some(p) => {
                let rl = p.join("rate-limit");
                dir_ready_check(&rl)
            }
            None => (
                false,
                "no cache dir resolved (cross-process rate limit unavailable)".to_string(),
            ),
        }
    };
    checks.push(DoctorCheck {
        name: "rate_limit_dir",
        ok: rl_ok,
        detail: rl_detail,
    });

    // Resolved human locale for stderr prose (JSON messages stay English).
    checks.push(DoctorCheck {
        name: "lang",
        ok: true,
        detail: locale.as_bcp47().into(),
    });

    // Contact URL should not be the historical placeholder host.
    let ua_contact_ok = !cfg.user_agent.contains("github.com/docsrs-cli/docsrs-cli");
    checks.push(DoctorCheck {
        name: "user_agent_contact",
        ok: ua_contact_ok,
        detail: if ua_contact_ok {
            "contact is not the placeholder docsrs-cli org".into()
        } else {
            "placeholder contact host github.com/docsrs-cli/docsrs-cli is invalid".into()
        },
    });

    if online {
        // Sync DNS/TCP probe (no async runtime needed inside doctor).
        for (name, host) in [
            ("online_crates_io", "crates.io"),
            ("online_docs_rs", "docs.rs"),
        ] {
            let ok_probe = ToSocketAddrs::to_socket_addrs(&format!("{host}:443"))
                .map(|mut it| it.next().is_some())
                .unwrap_or(false);
            checks.push(DoctorCheck {
                name,
                ok: ok_probe,
                detail: if ok_probe {
                    format!("{host}:443 resolves")
                } else {
                    format!("{host}:443 DNS/resolve failed")
                },
            });
        }
    }

    let ok = checks.iter().all(|c| c.ok);
    let data = DoctorData { ok, checks };
    if wants_json {
        // Agent-first: top-level `ok` mirrors `data.ok` (GAP-004). Exit 78 when unhealthy.
        write_json(
            stdout,
            &render::success_envelope_with_ok("doctor", &data, duration_ms(start), None, data.ok),
        )?;
    } else {
        writeln!(stdout, "{}", locale.doctor_ok(ok)).map_err(map_stdout_err)?;
        for c in &data.checks {
            writeln!(
                stdout,
                "- {} [{}] {}",
                c.name,
                if c.ok { "ok" } else { "fail" },
                c.detail
            )
            .map_err(map_stdout_err)?;
        }
    }
    Ok(if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(EXIT_CONFIG)
    })
}

/// Check that `path` is an existing writable directory, or can be created under a
/// writable ancestor. Does not leave directories behind when only probing parents.
fn dir_ready_check(path: &Path) -> (bool, String) {
    if path.is_dir() {
        let probe = path.join(".docsrs-cli-doctor-write-probe");
        return match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&probe)
        {
            Ok(_) => {
                let _ = std::fs::remove_file(&probe);
                (true, path.display().to_string())
            }
            Err(e) => (false, format!("{} (not writable: {e})", path.display())),
        };
    }
    if path.exists() {
        return (
            false,
            format!("{} (exists but is not a directory)", path.display()),
        );
    }
    let mut ancestor = path.parent();
    while let Some(a) = ancestor {
        if a.as_os_str().is_empty() {
            break;
        }
        if a.is_dir() {
            let probe = a.join(".docsrs-cli-doctor-write-probe");
            return match std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&probe)
            {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                    (
                        true,
                        format!("{} (creatable under {})", path.display(), a.display()),
                    )
                }
                Err(e) => (
                    false,
                    format!("{} (ancestor not writable: {e})", path.display()),
                ),
            };
        }
        if a.exists() {
            return (
                false,
                format!("{} (ancestor is not a directory)", path.display()),
            );
        }
        ancestor = a.parent();
    }
    (
        false,
        format!("{} (missing; no writable ancestor)", path.display()),
    )
}
