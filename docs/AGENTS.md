[Português (pt-BR)](AGENTS.pt-BR.md)

# Agents Guide for docsrs-cli
> Spend tokens on answers, not on scraping HTML by hand.

## Why Agents Use docsrs-cli
- Stable JSON beats fragile HTML scraping
- One process per question keeps state honest
- Exit codes make retry policy mechanical
- Product line is `1.1.x` (`version` reports `1.1.0`)

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
- Human stderr may localize via `--lang` or `DOCSRS_CLI_LANG` (pt-BR / en)
- JSON is automatic when stdout is not a TTY for most commands
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
- Confirm `docsrs-cli version --json` reports `1.1.0` (or newer 1.1.x)

## Contract: Success Envelope
- Success JSON includes `schema_version`, `ok:true`, `command`, `data`, `duration_ms`
- Read `data` only after `ok` is true
- Provenance fields such as `source_url` identify the fetched page
- Dry-run success may include `dry_run:true` and planned URL fields

## Contract: JSON data Fields
- `search-crates` data: `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`
- `search-crates` meta may include `next_page` / `prev_page` for `--page-token`
- `readme` data: `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; optional `resolved_version`
- `get-item` data: `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; optional `resolved_version`
- `search-in-crate` data: `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; optional `item_type`
- `search-in-crate` hits: `name`, `kind`, `url`; optional `score`
- `cache_hit` is local disk cache only; never remote telemetry
- Optional fields are omitted when absent (never JSON null)

## Contract: Error Envelope
- Failure JSON is a top-level envelope: `schema_version`, `ok:false`, `error`
- `error` always has `code`, `kind`, `message`, and `retryable`
- Optional `error.retry_after_secs` is omitted when absent (never JSON null)
- Message text is technical English; never secrets or raw response bodies
- Human path failures leave stdout empty and write one stderr line
- Branch on process exit code before trusting any field
- Retry only when exit is `69`, `74`, or `124` and/or `error.retryable` is true
- `get-item --suggest` may enrich not-found paths with nearby symbols (error message only)
- Machine schema: [error.schema.json](schemas/error.schema.json)

## Contract: Exit Codes
- `0` success
- `2` clap parse failure
- `64` usage
- `65` invalid input or parse
- `66` not found
- `69` rate limited or unavailable
- `70` internal
- `74` network
- `78` config
- `124` timeout
- `130` SIGINT
- `141` broken pipe on stdout
- `143` SIGTERM or SIGHUP

## Contract: Retry
- Retry only `69`, `74`, and `124` with backoff
- Honor `Retry-After` when the upstream sends it
- Do not retry `64`, `65`, `66`, or `78` without changing inputs
- Disable retries with `--disable-retry`, TOML `disable_retry`, or `max_retries=0`
- There is no product env kill switch for retries

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
docsrs-cli search-in-crate clap Parser --item-type function --limit 20 --json
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

# completions (raw shell by default)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# cache / config
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json

# dry-run
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme tokio --json
docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
```

## Contract: get-item Rules
- `item_path` accepts `::` or `/` separators
- Optional leading crate prefix is allowed
- Accepted kinds include module, struct, trait, enum, union, fn, function, method, type, const, constant, static, macro, attr, attribute, derive
- Alias `method` maps like `fn` / `function`
- Associated methods such as `Runtime::new` resolve to the parent type page plus `#method.name`
- Payload always includes `item_name`
- Optional `resolved_version` is the concrete SemVer of the target crate only, or the stdlib channel (`stable`) when known
- Never treat dependency versions on a docs.rs page as the crate version
- `--suggest` on 404 issues an extra `all.html` request and lists nearby symbols
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
- Path sandbox still allows `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- Default User-Agent is `docsrs-cli/1.1.0 (+https://github.com/danilo-aguiar-br/docsrs-cli)`
- User-Agent: `--user-agent` or TOML `user_agent`; contact: TOML `contact`
- Dry-run `planned_params` use `crate_name` (not `crate`)
- Dry-run envelope shape is documented in [dry-run.schema.json](schemas/dry-run.schema.json)

## Contract: schema Rules
- Payload schemas exist for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions
- Shared contracts also cover `error` and `dry-run` via `schema --cmd error|dry-run`
- Index of all files: [schemas/README.md](schemas/README.md)
- Prefer live schemas from `docsrs-cli schema --cmd <name> --json` before hardcoding field lists
