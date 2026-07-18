//! docsrs-cli binary entrypoint — BORN / EXECUTE / DIE.
//!
//! # Workload classification (Rules Rust — parallelism)
//!
//! - **Class:** mixed I/O + CPU. HTTPS GETs run on Tokio multi-thread workers;
//!   HTML/JSON parse runs on the `spawn_blocking` pool under
//!   [`docsrs_cli::concurrency::ConcurrencyBudget`].
//! - **Runtime:** multi-thread is the product default. Worker count follows
//!   `std::thread::available_parallelism` (Tokio default) so I/O and blocking
//!   pools can progress in parallel. `current_thread` would serialize signal
//!   handling behind long scraper work.
//! - **Bound:** `--max-concurrency` / `DOCSRS_CLI_MAX_CONCURRENCY` / config
//!   `max_concurrency` (0 = auto: `min(cpus, free_ram/2 / 48MiB)`). See
//!   `src/concurrency.rs` for formula, RSS notes, and Semaphore gate.
//! - **Not:** unbounded `spawn` loops, rayon without a size threshold, or a
//!   long-lived server. One command still dies after one (or few) GETs.

use std::process::ExitCode;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    docsrs_cli::run(std::env::args_os()).await
}
