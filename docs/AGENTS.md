[Português (pt-BR)](AGENTS.pt-BR.md)

# Agents Guide for docsrs-cli
> Spend tokens on answers, not on scraping HTML by hand.

## Why Agents Use docsrs-cli
- Stable JSON beats fragile HTML scraping
- One process per question keeps state honest
- Exit codes make retry policy mechanical
- Product line is `1.3.x` (`version` reports `1.3.0`)

## Economy
- Disk cache removes repeat downloads inside TTL
- `cache_hit` tells you when the body stayed local
- Dry-run validates planned URLs without burning quota
- Truncation flags tell you when to raise limits
- Ranked `--match` modes cut noisy substring hits

## Sovereignty
- No sticky MCP daemon required
- No product telemetry leaves the host
- Public docs hosts only; no login scraping
- Product knobs come from flags and XDG `config.toml` only

## Compatible Agents and Orchestrators
- Claude Code, Codex, Cursor, OpenCode, and any exec-capable agent
- Shell pipelines and CI jobs that can parse JSON
- Skill packages under `skills/docsrs-cli-en` and `skills/docsrs-cli-pt`

## Agent Integration Details
- Lifecycle is always one-shot: BORN, EXECUTE, FINALIZE, DIE
- Stdout is the data contract; stderr is diagnostics only
- JSON field names and technical error messages are always English
- Human stderr may localize via the `--lang` flag or the `lang` TOML key (pt-BR / en); the flag outranks the key
- JSON is automatic when stdout is not a TTY for most commands, unless `--format markdown|text` overrides it
- `commands --json` states that rule back to you in `data.agent_notes.json_auto`, so a caller can read the policy instead of assuming it
- The same block carries `stdout`, `stderr` and `lifecycle`, which is the one-shot model: BORN, EXECUTE, FINALIZE, DIE
- Force JSON with `--json` or `--format json`
- Force human with `--format markdown` or `--format text`
- Prefer `-q` when stderr must not pollute transcripts
- `completions` are the exception: raw shell by default; JSON only with explicit `--json`

## Crate Integrations
- crates.io powers `search-crates`
- docs.rs powers `readme`, `get-item`, and `search-in-crate`
- doc.rust-lang.org powers `std`, `core`, and `alloc`
- Host allowlist is fixed in the product HTTP layer

## Contract: Discovery
- Run `docsrs-cli commands --json` before inventing argv
- Run `docsrs-cli schema --cmd <name> --json` before parsing new fields
- Run `docsrs-cli doctor --json` when paths or TLS look wrong
- Run `docsrs-cli doctor --online --json` when you need live host probes
- Confirm `docsrs-cli version --json` reports `1.3.0` (or newer 1.3.x)

## Contract: Success Envelope
- Success JSON includes `schema_version`, `ok`, `command`, `data`, `duration_ms`
- For most commands, success means `ok:true`
- Exception (`doctor`): top-level `ok` mirrors `data.ok` (may be `false` with exit 78 when checks fail)
- Read `data` after inspecting `ok` and the process exit code
- Prefer `data.source_url` when present; envelope top-level `source_url` is a mirror for fetch ops
- Dry-run success may include `dry_run:true` and planned URL fields

## Contract: JSON data Fields
- `search-crates` data: `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit` — echo fields always match the effective request URL (including `--page-token`)
- `search-crates` meta may include `next_page` / `prev_page` for `--page-token`
- `readme` data: `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; optional `resolved_version`
- `get-item` data: `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; optional `resolved_version`; method success includes `extraction=method` only
- MUST reject method success when `extraction` is missing or is legacy `item_page` (fail-closed since 1.2.0)
- `extraction=method` means "came from the member anchor", never "the member is a function"
- `anchor_family` names the real family: `method`, `tymethod`, `associatedtype`, `associatedconstant`, `variant`, `structfield`
- Read `anchor_family` to tell an enum variant from a function; `iter::Iterator::next` returns `extraction=method` with `anchor_family=tymethod`
- Both fields are omitted for items with their own page (never JSON null)
- Associated types and constants report the same value, so the fail-closed check is unchanged
- `method`, `type` and `const` accept `Parent::member`; rustdoc anchors them on the parent page
- One anchor prefix per member category: `method.` · `tymethod.` · `associatedtype.` · `associatedconstant.`
- `source_url` echoes the anchor that exists on the page, not the one the URL builder planned
- A lowercase parent (`u32::MAX`) stays a free item on its own page, never an anchor
- Missing member anchors are `not_found` (exit 66), never a parent-page false success
- `search-in-crate` data: `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; optional `item_type`
- `search-in-crate` default `--match` is `prefix` (use `substring` for legacy contains)
- `search-in-crate` hits: `name`, `kind`, `url`; optional `score`
- `cache_hit` is local disk cache only; never remote telemetry
- readme/get-item markdown scrubs rustdoc chrome (`§`, "Copy item path")
- Optional fields are omitted when absent (never JSON null)
- Wire field is always `crate_name` (never `crate`)

## Contract: Payload Reduction
- Eight global flags cut the payload before serialization; no `jq` / `jaq` stage is required
- `--select <KEYS>` projects only these dotted keys (CSV or repeated); alias is `--fields`
- A key absent from `data` is skipped, never emitted as JSON null
- When the payload holds a results array, `--select` projects the array ELEMENTS, not `data`
- Measured: `--select name` on `search-in-crate` yields `hits:[{"name":…}]` and keeps other `data` keys
- Measured: `--select hits` yields `hits:[{},{},{}]` because no element owns a key named `hits`
- `--filter <EXPR>` keeps matching elements: `key=value`, `key!=value`, `key~substring`
- `==` is a synonym of `=`; repeat `--filter` to conjoin with AND
- A malformed `--filter` fails closed with exit `65` (`kind=invalid_input`), never an empty set
- `--sort-by <KEY>` sorts elements ascending by that dotted key; elements without it go last
- `--dedupe-by <KEY>` drops later elements repeating that key; elements without the key are kept
- `--max-items <N>` emits at most N elements; it bounds the EMISSION, never the query
- `search-in-crate --limit` is the query bound; the two are different budgets
- `--count-only` replaces the payload with `{"count": N}`, counted after filter, dedupe-by and max-items
- Measured on `search-in-crate tokio "" --limit 200`: `--filter kind=struct --count-only` returns 164,
  and adding `--max-items 5` returns 5 — the limit is inside the count, not after it
- `--truncate-content <N>` shortens every string above N characters (never bytes; UTF-8 is never split)
- `--max-output-bytes <N>` still caps the emitted payload (hard max 2097152)
- `--max-output-bytes` alone does not activate the pipeline; per-command budgeting enforces it upstream
- Application order is filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- Sorting before dedupe decides WHICH duplicate survives; limiting after it never spends a slot on one
- Read `agent_surface` for `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- `limited` separates "the set was small" from "the set was cut": only then does raising `--max-items` return more
- Truncation is never silent: `content_truncated` / `output_truncated` say when a cap bit

## Contract: Error Envelope
- Failure JSON is a top-level envelope: `schema_version`, `ok:false`, `command`, `duration_ms`, `error`
- `error` always has `code`, `kind`, `message`, and `retryable`
- Optional `error.retry_after_secs` is omitted when absent (never JSON null)
- Message text is technical English; never secrets or raw response bodies
- Human path failures leave stdout empty and write one stderr line
- Branch on process exit code before trusting any field
- Retry only when `error.retryable` is true (typically rate_limited/unavailable/timeout/network)
- Do not retry `kind=budget` (body over `--max-body-bytes`; raise the cap only within hard max)
- Budget flags above hard max fail closed with exit `65` (no silent clamp)
- Exit `74` is shared by `network` (retryable), `budget` (never) and `io` (depends) — always read `error.kind` / `error.retryable`
- `kind=io` is a local filesystem failure caused by the environment, never a defect in this binary
- `io` retryability comes from the OS cause: a full disk is retryable, a permission denial is not
- Never read `retryable` off the kind for `io`; read the field
- Explicit `--timeout 0` / `--connect-timeout 0` fail-closed with exit `65`
- `max_output_bytes` truncates success payloads (`truncated:true`); body over cap is a hard error (`budget`)
- `get-item --suggest` may enrich not-found paths with nearby symbols (structured in `error.suggestions`, also spelled into the message; cascade exact→prefix→substring→edit-distance)
- Machine schema: [error.schema.json](schemas/error.schema.json)

## Contract: Exit Codes
- `0` success
- `64` usage, which is also every clap parse failure; the binary never exits `2`
- `65` invalid input or parse (includes explicit timeout 0)
- `66` not found
- `69` rate limited or unavailable
- `70` internal
- `74` network, budget or io (disambiguate with `error.kind`)
- `78` config (doctor with failing checks also exits 78; top-level `ok` mirrors `data.ok`)
- `124` timeout
- `130` SIGINT
- `141` broken pipe on stdout
- `143` SIGTERM or SIGHUP

## Contract: Retry
- Retry only when `error.retryable` is true — typically exit `69`, retryable `74` (`kind=network`), and `124`
- Honor `Retry-After` when the upstream sends it
- Do not retry `64`, `65`, `66`, `78`, or `kind=budget` without changing inputs/config
- Never treat every exit `74` as retryable
- Disable retries with `--disable-retry`, TOML `disable_retry`, or `max_retries=0`
- There is no product env kill switch for retries
- Shape the backoff with `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms`, `--retry-max-elapsed-ms`
- `--retry-max-elapsed-ms 0` derives the ceiling from `--timeout` instead of standing on its own

## Contract: Tuning Flags
- Every knob below is a CLI flag with a matching `config.toml` key; the flag outranks the file
- Pace requests with `--rate-limit-delay-ms`, and size the worker pool with `--max-concurrency`
- `--max-concurrency 0` means automatic: the pool is derived from CPUs and free memory
- Steer the disk cache with `--cache-ttl-secs` and `--max-cache-bytes`, or bypass it with `--no-cache`
- `--max-cache-bytes 0` means unlimited, so it removes the ceiling rather than closing the cache
- `--allow-loopback` permits a local test origin; it is not a TLS bypass and never relaxes verification
- Read the effective value of any of these back with `config show --json`, which echoes the resolved knob

## Contract: Full Command Catalog
- Surface has 11 top-level commands and nested `cache` / `config` actions
```bash
# search-crates
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates tokio --sort alphabetical --json
docsrs-cli search-crates --page-token "$NEXT" --json

# readme
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json

# get-item
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item serde struct Serde --suggest --json

# search-in-crate
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate serde de --match substring --limit 20 --json
docsrs-cli search-in-crate clap Parser --item-type trait --limit 20 --json
docsrs-cli search-in-crate tokio "" --limit 50 --json

# meta
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd agent-surface --json
docsrs-cli schema --cmd cache-path --json
docsrs-cli schema --cmd cache-stats --json
docsrs-cli schema --cmd cache-clear --json
docsrs-cli schema --cmd config-path --json
docsrs-cli schema --cmd config-show --json
docsrs-cli schema --cmd config-init --json
docsrs-cli schema --cmd all --json   # the whole bundle: 20 schemas in one call

# completions (raw shell by default)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# cache / config
docsrs-cli cache path --json
docsrs-cli cache stats --json
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --yes --json

# dry-run
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme tokio --json
docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
```

## Contract: get-item Rules
- `item_path` accepts `::` or `/` separators
- `item_path` accepts `-` and normalizes segments to rustc style (`async-trait` → `async_trait` in the URL)
- Optional leading crate prefix is allowed
- Accepted kinds are module, mod, struct, trait, enum, union, fn, function, method, type, const, constant, static, macro, attr, attribute, derive, variant, structfield, field
- `variant`, `structfield` and `field` need a qualified path, because they name a member of a parent: `Option::Some`, `Range::end`
- `field` is an alias the echo normalizes to `structfield`, which is the spelling rustdoc uses for the anchor
- Alias `method` maps like `fn` / `function`
- Associated methods such as `Runtime::new` resolve to the parent type page plus `#method.name`
- Method markdown success sets `extraction` to `method` only; missing anchors are `not_found` (exit 66), never parent-page fallback success
- Payload always includes `item_name`
- Optional `resolved_version` is the concrete SemVer of the target crate only, or the stdlib channel (`stable`) when known
- Never treat dependency versions on a docs.rs page as the crate version
- `--suggest` on 404 issues an extra `all.html` request and lists nearby symbols (cascade exact→prefix→substring→edit-distance)
- `std`, `core`, and `alloc` resolve through doc.rust-lang.org
- Example stdlib channel: `docsrs-cli readme std --json` → `resolved_version` is `stable` when known

## Contract: search-in-crate Rules
- Default `--match` is `prefix` (exact leaf or leaf prefix)
- Use `--match substring` for legacy contains behavior
- Modes: `exact`, `prefix`, `substring`
- Hits may include `score` (lower is better when present)
- Default `--limit` is 100; hard clamp is 1000 (including dry-run)
- Prove the clamp offline: `docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json` plans limit 1000
- Optional `--item-type` filters kinds (`struct`, `fn`, `method`, …) and is echoed when set
- Payload always includes `match_mode` and `cache_hit`
- Empty query lists classified items up to `--limit`

## Contract: search-crates Rules
- `--page` is 1-based and conflicts with `--page-token`
- `--per-page` max is 100
- `--sort` values are relevance, downloads, recent-downloads, recent-updates, new, alphabetical
- Pagination tokens come from `meta.next_page` / `meta.prev_page`
- Pass tokens back with `--page-token` without inventing query strings by hand
- Payload always includes `cache_hit`

## Contract: doctor Rules
- Default `doctor` stays offline and checks TLS, paths, concurrency, contact, retry policy
- `doctor --online` adds opt-in network probes for crates.io and docs.rs
- Use online mode before large agent batches when connectivity matters

## Contract: config and path Rules
- Product settings: CLI flags > XDG `config.toml` > defaults
- Product knobs are not read from `DOCSRS_CLI_*` env vars
- Isolate storage with CLI flags `--config-dir` / `--cache-dir` (never product `DOCSRS_CLI_*` env)
- Default User-Agent is `docsrs-cli/<version> (+https://github.com/danilo-aguiar-br/docsrs-cli)` (version matches the binary)
- User-Agent: `--user-agent` or TOML `user_agent`; contact: TOML `contact`
- Where dry-run `planned_params` names a crate it uses `crate_name` (never `crate`): `readme`, `get-item`, `search-in-crate`
- `search-crates` plans a query, so its `planned_params` are `q`, `per_page`, `sort`, `page`, `page_token`, with no `crate_name` at all
- Dry-run `planned_params` may include `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe` and `planned_method_anchors` for methods
- Dry-run envelope shape is documented in [dry-run.schema.json](schemas/dry-run.schema.json)

## Contract: schema Rules
- Payload schemas exist for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions
- Shared contracts also cover `error` and `dry-run` via `schema --cmd error|dry-run`
- Index of all files: [schemas/README.md](schemas/README.md)
- Prefer live schemas from `docsrs-cli schema --cmd <name> --json` before hardcoding field lists
