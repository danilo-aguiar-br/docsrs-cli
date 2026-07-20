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
- Current product line is 1.2.x (release 1.2.0)


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
- Method fetch sets `data.extraction` to `method` on success; missing `#method.X` is `not_found` (exit 66), never a false parent-page success
- `--suggest` on method 404 ranks nearby method leaves from the parent type page (and all.html for other kinds)
- Budgets above hard max fail closed with exit 65 (no silent clamp)
- Error envelopes include `command` and `duration_ms` like success envelopes
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


## What's New in 1.2.0
- Method fail-closed: missing `#method.X` returns `not_found` (exit 66), not a parent-page false success
- `--suggest` ranks method leaves from the parent type HTML on method typos
- Error envelopes carry `command` + `duration_ms` (parity with success)
- Values above hard max for body/output budgets fail with exit 65 (no silent clamp)
- Method 404 `source_url` keeps the first probe kind (`struct`), not the last
- Dry-run documents `validation=url_shape_only` and parent kind probes
- Offline schema files match `schema --cmd all` (19 wire names including aliases)
- Full notes under [CHANGELOG.md](CHANGELOG.md) section `[1.2.0]`

## Earlier highlights (1.1.x)
- Short method paths (`Runtime::new`) resolve via all.html parent type lookup
- `crate@version` sugar on readme / get-item / search-in-crate
- Effective-URL echo for `--page-token`, cascade `--suggest`, hyphen normalize
- Fail-closed `--timeout 0` / `--connect-timeout 0` (exit 65)
- Non-retryable local body budget errors (`kind=budget`, exit 74)
- Product knobs: CLI flags + XDG only (no product `DOCSRS_CLI_*` env)


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


## TLS
- **rustls only** (no `native-tls` / OpenSSL runtime); crypto provider **`ring`**
- Minimum protocol **TLS 1.2**; peers may negotiate TLS 1.3
- Trust store: **webpki-roots** (Mozilla); certificate validation always on
- Production hosts require **HTTPS** (allowlist); offline tests may use loopback HTTP
- No `danger_accept_invalid_*`, no product KeyLog, no mTLS (public docs origins)
- Runtime posture: `docsrs-cli doctor --json` → check `http_client_posture`
- Decision record: [`docs/decisions/0007-rustls-posture.md`](docs/decisions/0007-rustls-posture.md)


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
- Successful method fetches set `data.extraction` to `method`; missing anchors are `not_found` (exit 66)
- `item_path` accepts `::` or `/` separators and optional crate prefix
- `--suggest` on get-item 404 lists nearby symbols (method leaves from parent page; other kinds from `all.html`)
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N] [--match MODE]`
- `--match` accepts `exact`, `prefix` (default), `substring`
- Empty `query` lists classified items up to `--limit` (clamped to 1000)
- Hits may include `score` when a query is present
- `version` — binary version
- `doctor` — local TLS, paths, concurrency, and retry readiness
- `doctor --online` — also probes crates.io and docs.rs DNS
- `commands` — full command tree for agents
- `schema --cmd <name>` — JSON Schema for a command payload
- Schema targets: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `cache`, `config` plus aliases (`cache-path`, `cache-clear`, `cache-stats`, `config-path`, `config-show`, `config-init`); use `schema --cmd all --json` for the full bundle
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
- `get-item` exposes `item_name`, optional `resolved_version`; method success includes `extraction=method`
- Agents MUST reject method success when `extraction` is missing or is the legacy `item_page` value
- `readme` exposes optional `resolved_version` (stdlib channel is `stable`)
- `search-in-crate` echoes `match_mode` and optional `item_type`
- `search-crates` pagination tokens live under `data.meta.next_page` / `prev_page`
- After `--page-token`, echoed `query`/`page`/`per_page`/`sort` match the effective URL
- Failure envelopes expose `schema_version`, `ok:false`, `command`, `duration_ms`, and nested `error` (`kind`, `retryable`, …)
- Never retry `kind=budget` (exit 74); raise `--max-body-bytes` only within the hard max (above hard max is exit 65)


## Environment Variables
- Paths: use `--config-dir` / `--cache-dir` (or platform XDG via `directories`)
- Locale: use `--lang` or TOML `lang` (never product env)
- Product knobs (timeout, UA, cache TTL, retries, concurrency, lang, paths) are **never** read from `DOCSRS_CLI_*` env at runtime
- Host diagnostics only: `RUST_LOG`, `NO_COLOR`, `HTTP(S)_PROXY` / `NO_PROXY`
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
- Raise caps only with explicit CLI flags or XDG `config.toml`; values above hard ceilings fail closed (exit 65), never silent clamp


## Troubleshooting FAQ
- Exit `65` means invalid input (including explicit `--timeout 0` / `--connect-timeout 0`, or budget flags above hard max)
- Exit `66` means the crate or item was not found (including missing method anchors)
- Use `get-item ... --suggest` to list nearby symbols after a 404 (method typos include parent method leaves)
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
- Current release notes for 1.2.0 live under `[1.2.0]`


## License
- Dual-licensed under MIT or Apache-2.0
- See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE)
