# docsrs-cli

One-shot stdin/stdout CLI that fetches Rust crate documentation from crates.io and docs.rs for LLM agents.

The process **BORN → EXECUTE → FINALIZE → DIE**. No daemon, no sticky session, no embedded server, no product telemetry.

## Install (local)

```bash
cargo install --path . --locked
```

## Commands

```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates serde --page 2 --per-page 20 --sort downloads --json
docsrs-cli search-crates serde --sort alphabetical --json
docsrs-cli readme tokio --json
docsrs-cli readme async-trait --dry-run --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item clap trait Parser --json
docsrs-cli get-item tokio struct tokio::runtime::Runtime --json
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
docsrs-cli search-in-crate tokio "" --limit 1 --json   # truncated:true when total > emitted
docsrs-cli doctor --json
docsrs-cli schema --cmd get-item --json
docsrs-cli version --json
docsrs-cli completions bash
docsrs-cli completions powershell
docsrs-cli completions power-shell
docsrs-cli cache stats --json
docsrs-cli cache clear --json
```

## Agent contract

- stdout: data only (Markdown default or JSON with `--json`)
- stderr: tracing, progress after 2s, and human errors (without `--json`)
- process exits after one operation (no daemon)
- exit codes: `0` success, `2` clap, `64` usage, `65` invalid input, `66` not found, `69` rate limit/unavailable, `70` internal, `74` network, `78` config, `124` timeout, `130` SIGINT, `143` SIGTERM
- JSON errors use `error.kind` (SIGINT/SIGTERM surface as `canceled` with codes 130/143)
- without `--json`, failures leave stdout empty and write a human line to stderr
- `search-in-crate` sets `data.truncated` to `true` when `total > emitted` (limit cut the hit list)
- dry-run clamps `page` and `per_page` in both `planned_url` and `planned_params` (`page` min 1, `per_page` 1..=100)
- `--limit 0` on `search-in-crate` emits zero hits with `truncated` true when any matches exist

## Configuration

XDG config file via `directories::ProjectDirs::from("", "", "docsrs-cli")`:

- Linux config: `$XDG_CONFIG_HOME/docsrs-cli/config.toml` (or `~/.config/docsrs-cli/config.toml`)
- Linux cache: `$XDG_CACHE_HOME/docsrs-cli/` (or `~/.cache/docsrs-cli/`)
- macOS/Windows: platform equivalent from the `directories` crate

Precedence: CLI flags > environment allowlist > TOML > defaults.

Environment allowlist:

- `DOCSRS_CLI_CONFIG_DIR`
- `DOCSRS_CLI_CACHE_DIR`
- `DOCSRS_CLI_CACHE_TTL_SECS` (default 86400; `0` never serves hits from disk)
- `DOCSRS_CLI_MAX_CACHE_BYTES` (default 268435456 = 256 MiB; `0` = unlimited)
- `DOCSRS_CLI_NO_CACHE` (`1`/`true`/`yes`/`on` disables disk cache)
- `DOCSRS_CLI_LANG`
- `DOCSRS_CLI_TIMEOUT_SECS`
- `DOCSRS_CLI_USER_AGENT`
- `DOCSRS_CLI_CONTACT`
- `DOCSRS_CLI_MAX_OUTPUT_BYTES`
- `DOCSRS_CLI_NETWORK_TESTS` (live integration tests only)
- `DOCSRS_CLI_ALLOW_LOCALHOST` (wiremock / local mocks only)
- `DOCSRS_CLI_CRATES_IO_ORIGIN` (override crates.io base URL; offline mocks)
- `DOCSRS_CLI_DOCS_RS_ORIGIN` (override docs.rs base URL; offline mocks)
- `RUST_LOG`, `NO_COLOR`, proxy vars

HTTP disk cache stores successful GET bodies under the cache dir (`http/v1/`). Key is SHA-256 of URL + parser version + Accept. Each entry stores a body checksum. Use `--no-cache` or `DOCSRS_CLI_NO_CACHE=1` for a fresh fetch. Default TTL is 24 hours. Default size budget is 256 MiB; oldest entries are evicted after each store when over budget. Manage with:

```bash
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli --max-cache-bytes 1048576 doctor --json
```

Default rate limit delay is **1000 ms per host per process**. Parallel processes do not share a delay clock. Agents must serialize many invocations against the same host to avoid HTTP 429.

## JSON schemas

Machine-readable schemas live in `docs/schemas/`:

- `search-crates.schema.json`
- `readme.schema.json`
- `get-item.schema.json`
- `search-in-crate.schema.json`
- `version.schema.json`
- `doctor.schema.json`
- `cache.schema.json` (`schema --cmd cache` / `cache-clear` / `cache-stats`)

Emit a schema at runtime:

```bash
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd cache --json
```

## Testing

Offline (default, no external network):

```bash
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo fmt --check
cargo llvm-cov --locked --summary-only
```

Live network opt-in (respects 1s delay; crawler-friendly UA):

```bash
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live -- --nocapture
```

Coverage gate for 0.1.0 is lines at least 80 percent via `cargo llvm-cov`.

Do not publish to crates.io or push GitHub releases without explicit maintainer authorization.

## Policy

- TLS: rustls only (no OpenSSL / native-tls)
- HTTP methods: GET only for product operations
- Host allowlist: `crates.io`, `docs.rs`, `static.docs.rs`
- User-Agent identifies `docsrs-cli/{version}` plus contact
- No product telemetry
- No CI/CD workflows in this repository
- XDG HTTP disk cache with checksum, TTL, size budget, clear/stats (disable via `--no-cache`)
- Code and public APIs in English; human stderr messages support `en` and `pt-BR`

## License

MIT
