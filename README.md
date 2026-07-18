[Português (pt-BR)](README.pt-BR.md)

# docsrs-cli

> Fetch crates.io and docs.rs docs in one shot for agents.

[![docs.rs](https://img.shields.io/docsrs/docsrs-cli)](https://docs.rs/docsrs-cli)
[![crates.io](https://img.shields.io/crates/v/docsrs-cli)](https://crates.io/crates/docsrs-cli)
[![License](https://img.shields.io/crates/l/docsrs-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/docsrs-cli)](https://crates.io/crates/docsrs-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org/)


## What is it
- One-shot stdin/stdout CLI for crates.io search and docs.rs item pages
- Lifecycle is always BORN, EXECUTE, FINALIZE, DIE
- No daemon, no sticky session, no product telemetry
- JSON auto-selects when stdout is not a TTY
- Public HTTP only against an allowlist of docs hosts


## The Pain
- Agents scrape HTML by hand and burn tokens on noise
- Browser tabs and curl pipelines do not form a stable contract
- Sticky MCP servers keep sockets open between turns


## Why
- Stable JSON envelopes on stdout for every command
- Markdown human path when you force `--format markdown`
- XDG disk cache with TTL and soft budget
- Polite rate limits, full-jitter HTTP retry, cancel-safe shutdown
- Exit codes agents can branch on without parsing prose


## Superpowers
- `search-crates` against crates.io with pagination and sort
- `readme` for the docs.rs crate overview docblock
- `get-item` for typed rustdoc pages by kind and path
- `search-in-crate` over the crate `all.html` index
- stdlib fetch for `std`, `core`, and `alloc` via doc.rust-lang.org
- `commands`, `schema`, `doctor`, `version` for agent discovery
- `cache` and `config` for XDG maintenance without secrets


## Quick Start
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme tokio --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli doctor --json
```


## Installation
- From crates.io: `cargo install docsrs-cli --locked`
- From a local checkout: `cargo install --path . --locked`
- MSRV is Rust 1.88
- No Cargo feature flags exist in this package


## Usage
- Pass `--json` or pipe stdout for the agent envelope
- Force human output with `--format markdown` or `--format text`
- Plan URLs without network using `--dry-run`
- Raise wall-clock budget with `--timeout <seconds>`
- Disable disk cache with `--no-cache`


## Commands
- Full surface has 11 top-level commands
- `search-crates <query> [--page N] [--per-page N] [--sort KIND]` — crates.io search
- `--sort` accepts `relevance`, `downloads`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- `readme <crate> [--crate-version V]` — docs.rs crate overview docblock
- `get-item <crate> <item_type> <item_path> [--crate-version V]` — typed rustdoc item
- `item_type` accepts `module`, `struct`, `trait`, `enum`, `union`, `fn`/`function`, `type`, `const`/`constant`, `static`, `macro`, `attr`/`attribute`, `derive`
- `item_path` accepts `::` or `/` separators and optional crate prefix
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N]` — `all.html` symbol search
- Empty `query` lists classified items up to `--limit`
- `version` — binary version
- `doctor` — local TLS, paths, concurrency, and retry readiness
- `commands` — full command tree for agents
- `schema --cmd <name>` — JSON Schema for a command payload
- Schema targets: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `cache`, `config`
- `completions <shell>` — `bash`, `zsh`, `fish`, `elvish`, `power-shell` (`powershell` alias)
- Examples: `docsrs-cli completions bash`, `completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`
- `cache stats` — report entry count, bytes, and budget
- `cache clear` — delete cached HTTP bodies
- `config path` — print resolved config/cache dirs and winning layer
- `config show` — print effective runtime configuration
- `config init [--force]` — create default `config.toml`
- Example overwrite: `docsrs-cli config init --force --json`


## Environment Variables
- `DOCSRS_CLI_HOME` — sandbox root for config and cache
- `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` — path overrides
- `DOCSRS_CLI_TIMEOUT_SECS` — wall-clock timeout
- `DOCSRS_CLI_USER_AGENT` / `DOCSRS_CLI_CONTACT` — identity headers
- `DOCSRS_CLI_CACHE_TTL_SECS` / `DOCSRS_CLI_MAX_CACHE_BYTES` / `DOCSRS_CLI_NO_CACHE`
- `DOCSRS_CLI_MAX_BODY_BYTES` / `DOCSRS_CLI_MAX_OUTPUT_BYTES` — hard caps
- `DOCSRS_CLI_MAX_CONCURRENCY` — CPU parse worker budget (`0` = auto)
- `DOCSRS_CLI_MAX_RETRIES` / `DOCSRS_CLI_RETRY_BASE_MS` / `DOCSRS_CLI_RETRY_MAX_DELAY_MS`
- `DOCSRS_CLI_DISABLE_RETRY` — kill switch for HTTP retries
- `DOCSRS_CLI_LANG` — human stderr locale (`en` or `pt-BR`)
- `RUST_LOG` / `NO_COLOR` / `CLICOLOR_FORCE` — diagnostics only


## Integration Patterns
- Agent subprocess: `docsrs-cli get-item serde trait Serialize --json`
- Discovery first: `docsrs-cli commands --json` then `schema --cmd get-item --json`
- Offline plan: `docsrs-cli --dry-run readme tokio --json`
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)


## Performance
- One primary GET per command in the happy path
- Multi-thread Tokio runtime with Semaphore concurrency budget
- `spawn_blocking` for large HTML parse work
- Disk cache avoids repeat downloads within TTL


## Memory Requirements
- Default body cap is 10 MiB per response
- Default output cap is 2 MiB per emission
- Default disk cache budget is 256 MiB soft
- Raise caps only with explicit flags or env, never above hard ceilings


## Troubleshooting FAQ
- Exit `66` means the crate or item was not found
- Exit `69` means rate limit or temporary upstream outage
- Exit `74` means transport failure; retry with backoff
- Exit `78` means local config or path readiness failed
- Exit `124` means wall-clock timeout
- Run `docsrs-cli doctor --json` before blaming the network


## Documentation Map
- [How to use](docs/HOW_TO_USE.md)
- [Agents](docs/AGENTS.md)
- [Cookbook](docs/COOKBOOK.md)
- [Cross platform](docs/CROSS_PLATFORM.md)
- [Migration](docs/MIGRATION.md)
- [Testing](docs/TESTING.md)
- [JSON schemas](docs/schemas/README.md)
- [Integrations](INTEGRATIONS.md)
- [llms.txt](llms.txt)


## Contributing
- Read [CONTRIBUTING.md](CONTRIBUTING.md)
- Follow the [Code of Conduct](CODE_OF_CONDUCT.md)


## Security
- Read [SECURITY.md](SECURITY.md)
- Report issues privately to daniloaguiarbr@proton.me


## Changelog
- See [CHANGELOG.md](CHANGELOG.md) for version history


## License
- Dual-licensed under MIT or Apache-2.0
- See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE)
