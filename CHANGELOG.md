[Português (pt-BR)](CHANGELOG.pt-BR.md)

# Changelog
All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

## [1.2.0] — 2026-07-20

### Fixed
- **Method fail-closed (GAP-W-001 / X-001 / X-006):** missing rustdoc `#method.X` anchors return `not_found` (exit 66) instead of parent-page success with `extraction=item_page`.
- **`--suggest` on method typos (X-002):** ranks nearby method leaves from the parent type page (e.g. `neww` → `runtime::Runtime::new (method)`).
- **Error envelope parity (X-004 / W-003):** failures include top-level `command` + `duration_ms` like success envelopes.
- **Budget honesty (X-005):** `--max-body-bytes` / `--max-output-bytes` (and TOML) above hard max fail with exit 65 instead of silent clamp.
- **404 method parent URL (X-010):** reports the first parent-kind probe (`struct`) not the last (`union`).
- **Dry-run honesty (X-003 / X-007):** method dry-run params include `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe`.
- **page-token messaging (W-007):** clearer invalid/remote rejection wording.
- **English-only code comment (W-004)** in `retry.rs`.
- **Schema disk parity (W-002):** 19 offline schema files (cache/config wire aliases).
- **Workspace junk (X-008):** removed accidental empty `cli,` / `unresolved,` / `xdg,` files.

### Changed
- Skills: MUST reject method + `item_page` as success (W-009).
- Release version **1.2.0** (tree was 1.1.4; method behavior break is intentional minor).

### Documentation
- Root and `docs/` surfaces synced to **1.2.0** Camada Y contracts (method fail-closed, error envelope `command`+`duration_ms`, hard-max budget fail-closed, `schema --cmd all` 19 wire names)
- llms.txt / llms-full.txt / INTEGRATIONS / HOW_TO_USE / AGENTS / MIGRATION / COOKBOOK / TESTING / CROSS_PLATFORM / schemas README bilingual pairs updated
- Skills reaffirm hard-max overshoot → exit 65 and MUST reject method + `item_page`
- Camada AA re-audit of `docs/`: EN/pt-BR parity (AGENTS, HOW_TO_USE), MIGRATION header + 1.1→1.2 path, dry-run schema method keys, COOKBOOK fence fix + hard-max/dry-run recipes
- Camada AB consolidated rewrite of `skills/docsrs-cli-en` and `skills/docsrs-cli-pt`: full 11-command catalog, 19 schema names + `schema --cmd all`, `cache path`, dry-run method keys, hard-max workflows, `--retry-max-elapsed-ms` / `--allow-loopback`, no version-history narrative, description auto-activation ≤1024 chars
- Camada AC rewrite of `CLAUDE.md` `# docsrs-cli` block for product line **1.2.0**: full 11-command catalog, 19 schemas + `schema --cmd all`, `cache path`, method fail-closed, hard-max exits, dry-run method keys, `--retry-max-elapsed-ms` / `--allow-loopback`, no product `DOCSRS_CLI_*` envs

### Policy
- Still no product env knobs, no telemetry, no GHA.
- Published to crates.io as **docsrs-cli 1.2.0** and tagged on GitHub.
- Dogfood `./target/release/docsrs-cli` (PATH install may lag). Prefer `cargo audit --no-fetch` when the advisory index hangs.

## [1.1.4] — 2026-07-19

### Fixed
- **Docs / agent memory:** removed remaining path-sandbox env teaching (`DOCSRS_CLI_HOME` / `CONFIG_DIR` / `CACHE_DIR` / `LANG`) from docs, ADRs, skills, and `CLAUDE.md`; isolation is `--config-dir` / `--cache-dir` only.
- **Cooperative cancel on scrape:** `search-in-crate` rayon/sequential hit filters honor `CancelFlag` (SIGINT/deadline during CPU fan-out).
- **Memory:** sequential filter pre-sizes `Vec` capacity from candidate count.

### Changed
- **SRP:** extracted `src/config/path_source.rs` (`PathSource` + config path resolve).
- **Discovery:** `commands` tree notes `schema --cmd all` bundle.

### Policy
- Product knobs/paths: CLI + XDG TOML only. Host env (`NO_COLOR`, `RUST_LOG`, proxy) unchanged. No telemetry. No GHA.

## [1.1.3] - 2026-07-19

### Added
- `schema --cmd all` returns a deterministic bundle of all embedded JSON Schemas
- `cache path` meta-command (symmetric with `config path`) reporting `root` + `source` + `no_cache`
- Policy regression tests: product paths resolve only via CLI flags / XDG (no product env)

### Changed
- **XDG-only product config:** removed runtime reads of `DOCSRS_CLI_LANG`, `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- `PathSource` tokens: `cli` | `xdg` | `unresolved` (removed `cli-or-env` / `home-sandbox`)
- Locale resolution: `--lang` / TOML `lang` → OS locale → `en` (no env)
- Live network tests marked `#[ignore]` (honest default suite; harness env still gates when ignored run)
- Dropped unused clap feature `env`
- Bumped **reqwest 0.12 → 0.13** with `rustls-no-provider` + process-wide **ring** CryptoProvider (ADR 0007; no aws-lc)
- Cargo.toml hygiene: diagnostics comment (not telemetry), canonical overflow-checks note
- `deny.toml`: allow OS root-cert crates pulled by `rustls-platform-verifier`; still ban native-tls/OpenSSL/aws-lc

### Fixed
- Doctor `dotenv_runtime` detail no longer claims an env path allowlist
- Embedded `cache.schema.json` includes `cache-path` variant

### Security
- Product binary ignores product-prefixed env knobs entirely (fail-closed policy for XDG+CLI)

### Added
- Doctor `web_fetch_posture` check (ADR 0003: one-shot docs client; **robots=PROIBIDO**; scraper+source_url)
- Regression tests: hit URL join follows `source_url` host (stdlib + mock loopback)
- Live stdlib search test asserts **every** hit URL stays on `doc.rust-lang.org` (SCRAPE-R-003)
- Process-static `LazyLock` CSS selectors for rustdoc HTML paths (SCRAPE-R-004)
- Same-origin hit URL resolution (`resolve_hit_url`) with soft-skip of off-origin absolute hrefs (SCRAPE-S-001)
- Single-DOM extract/version APIs (`*_from_document`) shared by fetch CPU workers (SCRAPE-S-003)
- `reqwest` **brotli** feature + explicit `Accept-Encoding: gzip, br` (SCRAPE-S-004)
- `RAYON_HIT_THRESHOLD` in `config::constants` (SCRAPE-S-005)
- Doctor `unsafe_posture` check (ADR 0009: `forbid(unsafe_code)`, no product FFI, loopback via CLI/XDG only)
- CLI `--allow-loopback` and XDG TOML `allow_loopback` for offline wiremock origins
- `AllowedOrigin::parse_with` / `Config::load_with_options` for explicit loopback policy
- ADR 0009 unsafe/FFI posture (EN + pt-BR)
- Doctor `error_model` check (ADR 0002 posture: thiserror `AppError`/`ErrorKind`, no public anyhow)
- Unit test: `with_source` Display does not embed the cause text
- ADR 0008 domain-type posture: only `url` of the chrono/uuid/rust_decimal/url set; doctor `domain_types` check
- `AllowedOrigin::to_url()` for fail-closed WHATWG re-parse in builders
- ADR 0007 rustls posture: binary `CryptoProvider::install_default` (ring), direct `rustls` pin ≥0.23.18, doctor posture with `provider=ring`
- Local `deny.toml` bans alternate TLS crates (`native-tls`, OpenSSL, dual aws-lc)
- Retry dual budget: `max_elapsed_ms` (CLI `--retry-max-elapsed-ms`, TOML `retry_max_elapsed_ms`; `0` derives from wall timeout)
- `Retry-After` HTTP-date parsing via `httpdate` (delta-seconds still preferred when pure digits)
- `RetryKind` / `ErrorLayer` classification APIs; HTTP `408` treated as transient timeout
- Tracing span `retry_attempt` on each in-process retry sleep
- Config/security: origin host allowlist at load time; capped `config.toml` reads; `validate_user_agent`
- Hard ceilings for redirects, wall/connect timeouts, and rate-limit delay

### Changed
- Retry base delay floor raised to 50ms (Rules Rust); doctor `retry_policy` reports `max_elapsed_ms` and HTTP-date support
- Product crate forbids `unsafe` (`#![forbid(unsafe_code)]`); release builds enable `overflow-checks`
- Search queries reject control characters; cache hits re-check allowlisted `final_url`
- Drop `aquamarine` (Mermaid lifecycle stays as plain rustdoc fence on `run`; no proc-macro render)
- Bump `scraper` **0.22 → 0.27** (`selectors` 0.38 + `rustc-hash`; removes unmaintained `fxhash`)
- SRP split: `docs_rs/*`, `config/*`, `ops`, `meta_cmds` (lib dispatch thinned)
- SRP split: `src/cache/{disk,meta,hex,paths,types}` and `src/http/{client,body,content_type,allowlist,rate_limit,constants}`
- crates.io search map caps `name` / `description` / URL / page-token fields (memory before emit budget)
- ADR 0005: serde/validation pipeline (domain newtypes; no unused `validator`/`serde_with`)
- **Type system (Camada L / ADR 0006):** domain module split (`src/domain/*`); core path takes `&CrateName` / `&VersionArg` / `&SearchQuery` / `SortKind`; `AllowedOrigin` on `Config`; `OpCtx` for handlers; `#[repr(transparent)]` newtypes; removed validate-only wrappers
- **Domain types (Camada N / ADR 0008):** URL builders / fetch / crates.io planners take `&AllowedOrigin` (not bare `AsRef<str>`); wire `source_url` stays `String`; chrono/uuid/rust_decimal intentionally absent
- **Error handling (Camada O / ADR 0002):** `html_to_markdown` preserves `Error::source` (no `{e}` in Display); lowercase transport/CPU messages; serde pretty-print failures are `Internal` (no silent `unwrap_or_default`/`{}`); semaphore acquire keeps source; origin allowlist message no longer promotes test-harness env; `# Errors` on ops/meta/doctor entrypoints
- **Unsafe/FFI (Camada P / ADR 0009):** loopback allowlist is CLI/XDG only (removed `DOCSRS_CLI_ALLOW_LOCALHOST`); integration tests no longer use `unsafe set_var`; only residual harness `unsafe` is Unix `libc::kill` for signal e2e
- **Web fetch / extraction (Camada Q / ADR 0003):** `search-in-crate` hit URLs join against final `source_url` (fixes stdlib links rewriting to `docs.rs`); version scrape uses scraper path segments (no regex field extract); single UTF-8 decode per HTML body; robots.txt remains **PROIBIDO**
- **Residual scrape re-audit (Camada R):** host-agnostic HTTP error labels (`rustdoc …`); skills EN/PT clarify CLI structured extraction vs agent-side regex scraping; rebuild/dogfood re-validation after Q
- **Residual scrape re-audit (Camada S):** same-origin hit URLs; method anchors via static `[id]` selector; one `Html::parse_document` per body; gzip+br; sanitize early-empty; doctor posture updated

### Fixed
- **SCRAPE-Q-001:** `search-in-crate std …` hit URLs now stay on `doc.rust-lang.org` (were rewritten to `https://docs.rs/std/latest/…`)
- **SCRAPE-R-001:** release binary re-validated so live stdlib hits match the Q join fix
- **SCRAPE-S-001:** absolute off-origin all.html hrefs no longer appear as hits (and no longer risk failing the whole search)

### Security
- SSRF defense in depth: config origins, redirect policy, request gate, and cache `final_url` share one allowlist (loopback gated by `allow_loopback`, never env)

- Fail-closed User-Agent and oversized config.toml (poisoned-config guard)
- Threat model STRIDE documented (ADR 0004); `SECURITY.md` supports `1.1.x`
- Explicit rustls-only stack (ADR 0007): ring provider bootstrap, webpki-roots, TLS ≥1.2, no cert bypass; `ErrorLayer::Tls` when rustls is in the error source chain
- Regex compile via `RegexBuilder` with explicit `size_limit` / `dfa_size_limit`
- Search queries reject invisible/bidi format characters (not only C0/C1 controls)
- `config.toml` `deny_unknown_fields`; cache keys must be 64 lowercase hex digits
- Present-but-wrong `Content-Type` fails closed for JSON/HTML product GETs
- Validate optional `contact` (visible ASCII) before embedding into User-Agent
- Componentize `lib`: extract `doctor`, `suggest`, and `output` modules (SRP)
- Cache meta: `deny_unknown_fields` + SHA-256 hex digest shape check before body I/O
- Field caps on crates.io JSON map (`MAX_CRATE_DESCRIPTION_CHARS`, `MAX_URL_FIELD_CHARS`)

## [1.1.2] - 2026-07-18

### Fixed
- **R1** Short associated-method paths (`get-item tokio method Runtime::new`) resolve parent type via all.html (`runtime::Runtime::new`) and set `resolved_item_path`.
- **R2** Crate sugar `name@version` accepted on `readme`, `get-item`, and `search-in-crate` (equivalent to `--crate-version`; conflicts fail closed).

### Changed
- Product version **1.1.2**

## [1.1.1] - 2026-07-18
### Fixed
- BUG-001: cache hits re-check `max_body_bytes` (exit 74 `budget`, `retryable=false`)
- BUG-002: `--max-output-bytes` reduces JSON hits for `search-crates` / `search-in-crate` and sets `truncated`
- BUG-003: `get-item` auto-resolves reexports via all.html (`resolved_item_path`)
- GAP-004: clap parse failures emit JSON `usage` envelope exit **64** (not bare exit 2)
- GAP-006: `--page < 1` and `--per-page` outside 1..=100 fail with `invalid_input` 65
- GAP-007: `search-in-crate --item-type module` rejected with actionable message
- GAP-008: requested `method` is echoed on wire (`item_type` / title)
- GAP-011: search markdown defaults documentation URL to `https://docs.rs/{name}`
- WARN-013: rename module `telemetry` → `diagnostics` (stderr tracing only; no product telemetry)
- WARN-014: document and assert budget is never retryable (exit 74 shared with network)

### Changed
- Product version **1.1.1**
- Config template documents path/locale env allowlist vs product knobs
- Schemas: `truncated` on search-crates; `resolved_item_path` on get-item

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


[Unreleased]: https://github.com/danilo-aguiar-br/docsrs-cli/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.2.0
[1.1.4]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.4
[1.1.3]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.3
[1.1.2]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.2
[1.1.1]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.1
[0.1.2]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.2
[1.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.0
[0.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.0
