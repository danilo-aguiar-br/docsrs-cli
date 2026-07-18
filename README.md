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
- Current product line is 0.1.x


## The Pain
- Agents scrape HTML by hand and burn tokens on noise
- Browser tabs and curl pipelines do not form a stable contract
- Sticky MCP servers keep sockets open between turns


## Why
- Stable JSON envelopes on stdout for every command
- Markdown human path when you force `--format markdown`
- XDG disk cache with TTL and soft budget plus `data.cache_hit`
- Polite rate limits, full-jitter HTTP retry, cancel-safe shutdown
- Exit codes agents can branch on without parsing prose
- Ranked in-crate match modes stop noisy false positives


## Superpowers
- `search-crates` against crates.io with pagination, sort, and `--page-token`
- `--page-token` echoes `query`/`page`/`per_page`/`sort` from the effective URL
- `readme` for the docs.rs crate overview docblock with `resolved_version`
- `get-item` for typed rustdoc pages including associated methods
- Method fetch scopes markdown and may set `data.extraction` to `method` or `item_page`
- `search-in-crate` over `all.html` with `--match exact|prefix|substring`
- `--suggest` ranks exact → prefix → substring → edit-distance on get-item 404
- `item_path` accepts hyphens and normalizes to underscores for rustc paths
- Markdown scrub removes rustdoc chrome (`§`, “Copy item path”)
- Body over cap is `error.kind=budget` (exit 74, `retryable=false`)
- stdlib fetch for `std`, `core`, and `alloc` via doc.rust-lang.org
- `commands`, `schema`, `doctor`, `version` for agent discovery
- `doctor --online` for opt-in DNS probes to crates.io and docs.rs
- `doctor` top-level `ok` mirrors `data.ok`
- `cache` and `config` for XDG maintenance without secrets


## What's New in 0.1.2
- Effective-URL echo for `--page-token` (and matching dry-run `planned_params`)
- Method-scoped extraction with optional `data.extraction`
- Non-retryable local body budget errors (`kind=budget`)
- Cascade `--suggest`, hyphen path normalize, rustdoc chrome scrub
- Fail-closed `--timeout 0` / `--connect-timeout 0` (exit 65)
- Human smoke ritual: `scripts/smoke-live.sh` (not CI)
- Full notes under [CHANGELOG.md](CHANGELOG.md) section `[0.1.2]`


## Quick Start
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme tokio --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio fn runtime::Runtime::new --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli doctor --online --json
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
- Product knobs use CLI flags or XDG `config.toml`, never product env vars


## Commands
- Full surface has 11 top-level commands
- `search-crates [query] [--page N] [--per-page N] [--sort KIND] [--page-token TOKEN]`
- Query may be omitted when `--page-token` carries a full query string
- `--page` conflicts with `--page-token`
- `--sort` accepts `relevance`, `downloads`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- `readme <crate> [--crate-version V]` — docs.rs crate overview docblock
- `get-item <crate> <item_type> <item_path> [--crate-version V] [--suggest]`
- `item_type` accepts `module`, `struct`, `trait`, `enum`, `union`, `fn`/`function`/`method`, `type`, `const`/`constant`, `static`, `macro`, `attr`/`attribute`, `derive`
- `method` is an alias of `fn` for associated methods
- Associated methods resolve to parent type pages with `#method.name` and `item_name`
- `item_path` accepts `::` or `/` separators and optional crate prefix
- `--suggest` on get-item 404 lists nearby symbols from `all.html`
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N] [--match MODE]`
- `--match` accepts `exact`, `prefix` (default), `substring`
- Empty `query` lists classified items up to `--limit` (clamped to 1000)
- Hits may include `score` when a query is present
- `version` — binary version
- `doctor` — local TLS, paths, concurrency, and retry readiness
- `doctor --online` — also probes crates.io and docs.rs DNS
- `commands` — full command tree for agents
- `schema --cmd <name>` — JSON Schema for a command payload
- Schema targets: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `cache`, `config`
- `completions <shell>` — raw shell script by default; JSON only with explicit `--json`
- Shells: `bash`, `zsh`, `fish`, `elvish`, `power-shell` (`powershell` alias)
- `cache stats` — report entry count, bytes, and budget
- `cache clear` — delete cached HTTP bodies
- `config path` — print resolved config/cache dirs and winning layer
- `config show` — print effective runtime configuration
- `config init [--force]` — create default `config.toml`


## JSON contract highlights
- Success envelope: `schema_version`, `ok`, `command`, `data`, `duration_ms`
- Network payloads use canonical `crate_name` (never `crate`)
- Network payloads expose `cache_hit` for local disk cache only
- `get-item` exposes `item_name`, optional `resolved_version`, optional `extraction`
- `readme` exposes optional `resolved_version` (stdlib channel is `stable`)
- `search-in-crate` echoes `match_mode` and optional `item_type`
- `search-crates` pagination tokens live under `data.meta.next_page` / `prev_page`
- After `--page-token`, echoed `query`/`page`/`per_page`/`sort` match the effective URL
- Failure envelopes expose `error.kind` and `error.retryable` (never retry `kind=budget`)


## Environment Variables
- `DOCSRS_CLI_HOME` — sandbox root for config and cache (tests / isolation)
- `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` — path overrides only
- Product knobs (timeout, UA, cache TTL, retries, concurrency, lang) are not read from `DOCSRS_CLI_*` env at runtime
- Use CLI flags and XDG `config.toml` for product settings
- `RUST_LOG` — tracing filter (stderr diagnostics only; not product telemetry)
- `NO_COLOR` / `CLICOLOR_FORCE` — diagnostics only


## Integration Patterns
- Agent subprocess: `docsrs-cli get-item serde trait Serialize --json`
- Method fetch: `docsrs-cli get-item tokio fn runtime::Runtime::new --json`
- Ranked symbols: `docsrs-cli search-in-crate serde Serialize --match prefix --json`
- Paginate: read `meta.next_page` then `docsrs-cli search-crates --page-token '...' --json`
- Discovery first: `docsrs-cli commands --json` then `schema --cmd get-item --json`
- Offline plan: `docsrs-cli --dry-run readme tokio --json`
- See [INTEGRATIONS.md](INTEGRATIONS.md) and [docs/AGENTS.md](docs/AGENTS.md)


## Performance
- One primary GET per command in the happy path
- Multi-thread Tokio runtime with Semaphore concurrency budget
- `spawn_blocking` for large HTML parse work
- Disk cache avoids repeat downloads within TTL
- Warm cache responses set `data.cache_hit` to true


## Memory Requirements
- Default body cap is 10 MiB per response
- Default output cap is 2 MiB per emission
- Default disk cache budget is 256 MiB soft
- Raise caps only with explicit CLI flags or XDG `config.toml`, never above hard ceilings


## Troubleshooting FAQ
- Exit `65` means invalid input (including explicit `--timeout 0` / `--connect-timeout 0`)
- Exit `66` means the crate or item was not found
- Use `get-item ... --suggest` to list nearby symbols after a 404
- Exit `69` means rate limit or temporary upstream outage (retryable)
- Exit `74` with `error.kind=network` means transport failure; retry with backoff
- Exit `74` with `error.kind=budget` means local body cap; do not retry — raise `--max-body-bytes`
- Always read `error.retryable` before retrying any non-zero exit
- Exit `78` means local config or path readiness failed
- Exit `124` means wall-clock timeout
- Run `docsrs-cli doctor --json` before blaming the network
- Treat doctor healthy only when top-level `ok` and `data.ok` are both true
- Run `docsrs-cli doctor --online --json` to probe crates.io and docs.rs DNS
- Noisy search hits: switch from `--match substring` to `prefix` or `exact`
- Hyphenated paths: pass `async-trait` segments; the CLI normalizes to `async_trait`


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
- Current release notes for 0.1.2 live under `[0.1.2]`


## License
- Dual-licensed under MIT or Apache-2.0
- See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE)
