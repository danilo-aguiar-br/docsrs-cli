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
//! - **Not:** unbounded `spawn` loops, CPU fan-out for work too small to
//!   amortize it, a long-lived server, or multiple concurrent runtimes. One
//!   command dies after one (or few) GETs.

// Defensive security: binary has no FFI; match lib `forbid(unsafe_code)`.
#![forbid(unsafe_code)]

use std::process::ExitCode;

use docsrs_cli::concurrency::{max_blocking_threads, runtime_worker_threads};
use docsrs_cli::error::EXIT_INTERNAL;
use docsrs_cli::i18n::Locale;

fn main() -> ExitCode {
    // Bootstrap order (Rules Rust — rustls / ADR 0007):
    // 0) resolve locale from argv alone (no disk, no env) for the two failures below
    // 1) install process CryptoProvider once (ring; reqwest rustls-no-provider)
    // 2) build Tokio multi-thread runtime
    // 3) run one-shot command
    // Never call install_default from the library crate.
    //
    // Step 0 exists because steps 1 and 2 can fail before clap ever runs, and a
    // product that ships en / pt-BR must not print English-only there.
    let locale = Locale::from_argv_for_bootstrap(std::env::args_os());

    // `install_default` returns Err when a process default already exists.
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        emit_bootstrap_error(locale.bootstrap_provider_conflict());
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
            emit_bootstrap_error(&locale.bootstrap_runtime_failure(&e.to_string()));
            return ExitCode::from(EXIT_INTERNAL);
        }
    };
    runtime.block_on(docsrs_cli::run(std::env::args_os()))
}

/// Write one already-localized bootstrap line to stderr.
///
/// stdout stays payload-only even when the process dies before parsing argv, so
/// this never touches it. A failed write is dropped on purpose: the process is
/// already returning [`EXIT_INTERNAL`], and panicking on a closed stderr would
/// replace a clean exit code with a stack trace.
fn emit_bootstrap_error(message: &str) {
    use std::io::Write as _;
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "{message}");
    let _ = err.flush();
}
