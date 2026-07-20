//! docsrs-cli binary entrypoint — BORN / EXECUTE / DIE.
//!
//! # Workload classification (Rules Rust — parallelism + rede)
//!
//! - **Class:** mixed I/O + CPU. HTTPS GETs run on Tokio multi-thread workers;
//!   HTML/JSON parse runs on the `spawn_blocking` pool under
//!   [`docsrs_cli::concurrency::ConcurrencyBudget`].
//! - **Runtime:** explicit [`tokio::runtime::Builder::new_multi_thread`] (not
//!   `#[tokio::main]` defaults). Worker count follows
//!   [`docsrs_cli::concurrency::runtime_worker_threads`]; blocking pool is capped
//!   by [`docsrs_cli::concurrency::max_blocking_threads`]. Named threads aid
//!   profiling. `current_thread` would serialize signal handling behind long
//!   scraper work.
//! - **Bound:** `--max-concurrency` / XDG `max_concurrency` in `config.toml`
//!   (0 = auto: `min(cpus, free_ram/2 / 48MiB)`). See `src/concurrency.rs`.
//! - **Not:** unbounded `spawn` loops, rayon without a size threshold, a
//!   long-lived server, or multiple concurrent runtimes. One command dies after
//!   one (or few) GETs.

// Defensive security: binary has no FFI; match lib `forbid(unsafe_code)`.
#![forbid(unsafe_code)]

use std::process::ExitCode;

use docsrs_cli::concurrency::{max_blocking_threads, runtime_worker_threads};
use docsrs_cli::error::EXIT_INTERNAL;

fn main() -> ExitCode {
    // Bootstrap order (Rules Rust — rustls / ADR 0007):
    // 1) install process CryptoProvider once (ring; reqwest rustls-no-provider)
    // 2) build Tokio multi-thread runtime
    // 3) run one-shot command
    // Never call install_default from the library crate.
    // `install_default` returns Err when a process default already exists.
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        eprintln!(
            "docsrs-cli: rustls CryptoProvider was already installed; refusing dual init"
        );
        return ExitCode::from(EXIT_INTERNAL);
    }

    let workers = runtime_worker_threads();
    let max_blocking = max_blocking_threads();
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(workers)
        .max_blocking_threads(max_blocking)
        .thread_name("docsrs-cli-worker")
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("docsrs-cli: failed to build async runtime: {e}");
            return ExitCode::from(EXIT_INTERNAL);
        }
    };
    runtime.block_on(docsrs_cli::run(std::env::args_os()))
}
