[Português (pt-BR)](CHANGELOG.pt-BR.md)

# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

## [0.1.2] - 2026-07-18
### Fixed
- GAP-001: `search-crates --page-token` echoes `query`/`page`/`per_page`/`sort` from the **effective URL** (and dry-run `planned_params` match)
- GAP-002: associated-method `get-item` extracts the method `details`/`section` by rustdoc anchor (not the full parent page); optional `extraction` field (`method`|`item_page`)
- GAP-003: body over `max_body_bytes` is `error.kind=budget`, exit 74, **`retryable=false`** (no agent retry storms)
- GAP-004: `doctor` top-level `ok` mirrors `data.ok` (exit 78 when unhealthy)
- GAP-005: `--suggest` ranks exact → prefix → substring → edit-distance (one all.html fetch)
- GAP-006: scrub rustdoc chrome (`§`, “Copy item path”) from markdown
- GAP-007: `item_path` accepts hyphens and normalizes to underscores (rustc paths)
- GAP-012: tree formatted with `cargo fmt`
- GAP-013: deduplicated rustdoc comments on method/search helpers
- GAP-015: explicit `--timeout 0` / `--connect-timeout 0` fail-closed with exit 65

### Changed
- Product version **0.1.2**
- PRD / agent docs aligned: default `--match prefix`, wire field `crate_name`, dual license MIT OR Apache-2.0, env allowlist (paths/lang only for product sandbox)
- Secondary bilingual docs synced to the 0.1.2 line (README, HOW_TO_USE, COOKBOOK, MIGRATION, SECURITY, INTEGRATIONS, CROSS_PLATFORM, TESTING, schemas README, AGENTS, skills, `llms*.txt`)

### Added
- `scripts/smoke-live.sh` human pre-release live smoke (not CI)
- Offline fixture `tests/fixtures/docs_rs/method_runtime_new.html` for method extract golden

## [1.1.0] - 2026-07-18
### Changed
- `search-in-crate` default match is now `prefix` (exact leaf / prefix leaf), not substring; use `--match substring` for legacy contains behavior
- Product settings are no longer read from `DOCSRS_CLI_*` environment variables (use CLI flags + XDG `config.toml`)
- Dry-run `planned_params` use `crate_name` (not `crate`)
- `completions` emit raw shell scripts by default even on non-TTY; JSON only with explicit `--json`

### Fixed
- GAP-020: network command payloads expose `cache_hit` (local disk cache only; no remote telemetry)
- GAP-001/019: ranked match modes stop `Serialize` from returning `Deserializer*` noise
- GAP-002/021: associated methods resolve to `#method.name` on the parent type page
- GAP-003: stdlib `resolved_version` is channel (`stable`), never the crate name
- GAP-004: attempt HTML scrape for SemVer when URL stays on `/latest/` (crate-scoped only)
- GAP-005: `get-item` emits `item_name`
- GAP-006: `search-in-crate` echoes `item_type` and `match_mode`
- GAP-007/018: unified `crate_name` on wire
- GAP-008: dry-run clamps `--limit` to 1000
- GAP-009/023: `--page-token` consumes `meta.next_page` query strings without positional query
- GAP-010: completions raw-by-default
- GAP-011/022: real default contact URL + `doctor --online` DNS probes + contact check
- GAP-012: removed product env surface from clap/config
- GAP-013: `index.html` / `#method.` classification for modules and methods
- GAP-014: sanitize + main-content extraction path
- GAP-015: `--suggest` on get-item 404 lists nearby symbols
- GAP-016: suite timeout/offline coverage
- GAP-017: JSON payloads stay English; human stderr is i18n-aware
- GAP-024: `join_href` produces absolute URLs for method paths

### Added
- `--match exact|prefix|substring`, hit `score`, `--page-token`, `--suggest`, `doctor --online`
- Optional hit score field; method alias `method` for item type


## [0.1.0] - 2026-07-18
### Added
- One-shot agent-first CLI for crates.io search and docs.rs item/README fetch
- JSON envelopes with auto-JSON on non-TTY stdout and Markdown human path
- XDG HTTP disk cache with TTL, size budget, clear/stats
- `commands`, `schema`, `doctor`, `version`, `completions`, `cache`, `config`
- stdlib fetch for `std` / `core` / `alloc` via `doc.rust-lang.org`
- Parallelism defaults: multi-thread Tokio, `ConcurrencyBudget`, `--max-concurrency`
- Explicit HTTP `RetryConfig` with full-jitter backoff and `Retry-After`
- Kill switch `--disable-retry` (and TOML `disable_retry` / `max_retries=0`)
- `politeness_delay` with per-host floor plus additive jitter
- ADRs for HTTP retry, error model, and web-fetch scope
- Bilingual public documentation framework and agent skills
- Dual license MIT OR Apache-2.0

### Security
- GET-only allowlist hosts, rustls TLS, no cookie jar, no invalid-cert bypass
- No runtime `.env`, no API keys, public HTTP cache only
- Unix owner-only modes for config and cache writes

### Policy
- No product telemetry
- No GitHub Actions / CI workflows in-tree
- MSRV 1.88


[Unreleased]: https://github.com/danilo-aguiar-br/docsrs-cli/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.2
[1.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.0
[0.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.0
