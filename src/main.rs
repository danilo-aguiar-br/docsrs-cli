//! docsrs-cli binary entrypoint — BORN / EXECUTE / DIE.

use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    docsrs_cli::run(std::env::args_os()).await
}
