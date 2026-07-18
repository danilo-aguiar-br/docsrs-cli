---
name: docsrs-cli
description: This skill MUST activate when the agent needs docsrs-cli, crates.io search, docs.rs fetch, rustdoc item lookup, get-item, readme, search-crates, search-in-crate, version, doctor online, doctor data.ok, commands, schema, completions, cache, config, page-token echo, match exact prefix substring, get-item suggest cascade, method extraction, hyphen item_path normalize, rustdoc scrub of § and Copy item path, dual source_url preferring data.source_url, cache_hit, JSON envelopes, dry-run URL planning, XDG cache config, timeout zero exit 65, exit 74 budget vs network, body budget non-retryable, locale via --lang or DOCSRS_CLI_LANG, or host allowlist. It MUST teach the full 11-command catalog, exact argv, global flags, stdout JSON contracts, exit codes, retry gated by error.retryable, multi-step workflows, ready formulas, and one-shot BORN-EXECUTE-FINALIZE-DIE lifecycle so agents fetch Rust crate docs without scraping HTML.
---

# docsrs-cli
Product `docsrs-cli`. Canonical repository `https://github.com/danilo-aguiar-br/docsrs-cli`. One-shot BORN-EXECUTE-FINALIZE-DIE CLI for crates.io and docs.rs. Stdout is the data contract. Stderr is diagnostics only.

## Identity and Lifecycle
### REQUIRED
- MUST treat the binary name as always `docsrs-cli`
- MUST treat every process as BORN, EXECUTE, FINALIZE, DIE with no sticky session
- MUST use `--json` for every programmatic consumer
- MUST treat stdout as the data contract and stderr as diagnostics only
- MUST expect automatic JSON when stdout is not a TTY
- MUST force human output with `--format markdown` or `--format text`
- MUST discover the live command tree with `commands --json` when unsure
- MUST use the built-in default User-Agent unless a concrete override is required
- MUST keep JSON envelopes in English always
- MUST force human stderr locale only with `--lang en`, `--lang pt-BR`, or env `DOCSRS_CLI_LANG`
- MUST apply knob precedence flags then XDG `config.toml` then built-in defaults
### FORBIDDEN
- NEVER assume a daemon, sticky session, or product telemetry channel
- NEVER parse stderr as success JSON
- NEVER reuse process state across invocations
- NEVER invent a subcommand outside the catalog below
- NEVER use product env knobs `DOCSRS_CLI_USER_AGENT`, `DOCSRS_CLI_CONTACT`, `DOCSRS_CLI_DISABLE_RETRY`, or `DOCSRS_CLI_TIMEOUT`
- NEVER put version-history narrative inside this skill
### Correct Pattern
```bash
docsrs-cli --version
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli --lang pt-BR doctor --json
DOCSRS_CLI_LANG=pt-BR docsrs-cli doctor --json
```

## Decision Matrix — Pick the Command
### REQUIRED
- MUST run `search-crates` when the goal is to find a crate on crates.io
- MUST run `readme` when the goal is the crate overview docblock from docs.rs (not a git README)
- MUST run `search-in-crate` when the goal is to list or filter symbols inside one crate `all.html`
- MUST run `get-item` when the goal is the full documentation body of one typed rustdoc item
- MUST run `get-item` with kind `method` (alias of `fn`) for associated methods; the product resolves the parent type page plus `#method.name`
- MUST run `doctor --online --json` before batch network work when network health is unknown
- MUST run `schema --cmd <name> --json` before parsing an unfamiliar payload
- MUST run `version --json` when binary identity is required
- MUST run `cache stats|clear` and `config path|show|init` for storage and knobs
### FORBIDDEN
- NEVER scrape docs.rs or crates.io HTML with regex when this CLI resolves the need
- NEVER call `get-item` without a concrete kind and path
- NEVER use `search-in-crate` as a crates.io search (that is `search-crates`)
- NEVER treat `readme` as source-control README content
### Correct Pattern
```bash
docsrs-cli search-crates serde --sort downloads --json
docsrs-cli readme serde --json
docsrs-cli search-in-crate serde Serialize --match prefix --limit 20 --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
```

## Full Command Catalog
### REQUIRED
- MUST know all 11 top-level commands
- MUST know subcommands `cache clear`, `cache stats`
- MUST know subcommands `config path`, `config show`, `config init`
- MUST know shells for `completions` (raw script by default)
- MUST use `--match` with values `exact|prefix|substring` (default `prefix`); JSON field is always `match_mode`
- MUST treat `--limit` as clamped to 1000 (including dry-run planned limit)
- MUST know schema inventory names `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `cache`, `config`
### FORBIDDEN
- NEVER omit a top-level command from the operational catalog
- NEVER invent `--match-mode` (the flag is `--match`)
- NEVER invent schema command names outside the inventory above
- NEVER assume completions emit a JSON envelope without explicit `--json`
### Correct Pattern
```bash
# 1) search-crates  (QUERY MUST be omitted when --page-token carries the full query string)
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates <QUERY> --sort alphabetical --json
docsrs-cli search-crates --page-token '<TOKEN_FROM_meta.next_page>' --json
# --sort values: relevance|downloads|recent-downloads|recent-updates|new|alphabetical
# --page conflicts with --page-token

# 2) readme
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json
docsrs-cli readme std --json

# 3) get-item
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --suggest --json
# KIND: module|struct|trait|enum|union|fn|function|method|type|const|constant|static|macro|attr|attribute|derive
# method is an alias of fn; methods resolve to parent type page + #method.name
# PATH: uses :: or / ; leading crate prefix is allowed; hyphen segments normalize to underscore

# 4) search-in-crate
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> "" --limit 50 --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type function --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match prefix --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match substring --json
docsrs-cli search-in-crate <CRATE> <QUERY> --crate-version <VERSION> --limit 100 --json
# --limit clamped to 1000; default match is prefix; hits sorted by score (lower is better)

# 5) version
docsrs-cli version --json
docsrs-cli --format markdown version

# 6) doctor
docsrs-cli doctor --json
docsrs-cli doctor --online --json
# --online adds DNS/network probes online_crates_io and online_docs_rs

# 7) commands
docsrs-cli commands --json

# 8) schema
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# 9) completions  (raw shell script by default; JSON only with explicit --json)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# 10) cache
docsrs-cli cache stats --json
docsrs-cli cache clear --json

# 11) config
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json
```

## Global Flags — Execution Knobs
### REQUIRED
- MUST use `--json` or `--format json` for agent pipelines
- MUST accept global flags before or after the subcommand (clap global flags)
- MUST use `--timeout <SECS>` and `--connect-timeout <SECS>` for wall-clock and connect limits
- MUST treat explicit `--timeout 0` and `--connect-timeout 0` as fail-closed invalid input (exit 65)
- MUST raise `--max-body-bytes` when `error.kind=budget` instead of retrying
- MUST treat body over `--max-body-bytes` as hard error `kind=budget` with `retryable=false`
- MUST treat emit over `--max-output-bytes` as success with `truncated:true` (not budget)
- MUST use `--no-cache` only when freshness is mandatory
- MUST use `--cache-ttl-secs`, `--max-cache-bytes`, `--cache-dir`, `--config-dir` to control disk cache and config roots
- MUST use `--max-body-bytes` and `--max-output-bytes` to cap download and emit size (hard product ceilings apply)
- MUST use `--rate-limit-delay-ms` and `--max-concurrency` for politeness and parse workers
- MUST treat `--max-concurrency 0` as auto (CPU and free RAM)
- MUST use `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms`, and `--disable-retry` to control HTTP retries
- MUST use `-q` / `--quiet` to suppress non-essential stderr noise in agent pipelines
- MUST use `-v` / `--verbose` only when deeper diagnostics are mandatory (countable flag)
- MUST use `--no-color` when ANSI color must be disabled
- MUST NEVER combine `--json` with `--format markdown` or `--format text` on the same invocation
- MUST treat `search-crates --per-page` default as 10 and max as 100
- MUST treat `search-in-crate --limit` default as 100 and hard clamp as 1000
- MUST treat `--cache-ttl-secs` default as 86400 and `--max-cache-bytes` default as 268435456 (0 = unlimited)
- MUST treat hard ceilings as `--max-body-bytes` 10485760 (10 MiB) and `--max-output-bytes` 2097152 (2 MiB)
- MUST use `--dry-run` to plan URLs without opening network sockets
- MUST use `--user-agent` only when a concrete override is required
- MUST use `--lang en` or `--lang pt-BR` only for human stderr locale (JSON stays English)
- MUST accept path isolation envs `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- MUST accept locale env `DOCSRS_CLI_LANG` for human stderr only (not timeout, UA, or retry)
### FORBIDDEN
- NEVER invent product env knobs for timeout, UA, contact, or retry
- NEVER treat path env vars as carriers for timeout, UA, or retry policy
- NEVER expect a runtime `.env` for product settings
- NEVER store API keys in the product
### Correct Pattern
```bash
docsrs-cli --timeout 30 --connect-timeout 5 -q get-item serde trait Serialize --json
docsrs-cli --no-cache readme tokio --json
docsrs-cli --dry-run --max-retries 0 --retry-base-ms 100 --retry-max-delay-ms 2000 search-crates serde --json
docsrs-cli --disable-retry doctor --online --json
docsrs-cli --lang pt-BR --format markdown doctor
docsrs-cli --no-color --format text version
docsrs-cli --cache-dir /tmp/docsrs-cache --config-dir /tmp/docsrs-cfg config path --json
docsrs-cli --rate-limit-delay-ms 200 --max-concurrency 2 search-in-crate serde "" --limit 50 --json
docsrs-cli --max-concurrency 0 search-in-crate serde Serialize --json
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme serde --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config show --json
DOCSRS_CLI_LANG=en docsrs-cli --format markdown doctor
```

## Search and Fetch Execution
### REQUIRED
- MUST use `search-crates` for crates.io search
- MUST use `readme` for docs.rs crate overview (not git README)
- MUST use `get-item` for typed rustdoc items
- MUST use `search-in-crate` for symbols in `all.html`
- MUST accept `item_path` with `::` or `/`
- MUST accept hyphen segments in `item_path` and expect underscore normalization for rustc paths
- MUST treat `std`, `core`, and `alloc` via doc.rust-lang.org
- MUST pass `--crate-version` when a concrete version is required
- MUST use `--match prefix` (default) to reduce Serialize-style noise; escalate to `exact` or `substring` deliberately
- MUST use `get-item --suggest` on 404 to obtain nearby symbols (one all.html request; cascade exact→prefix→substring→edit-distance)
- MUST treat `method` as alias of `fn`; associated methods resolve to parent type page plus `#method.name`
- MUST expect optional `data.extraction` of `method` or `item_page` on get-item method fetches
- MUST expect product markdown for readme/get-item to already scrub rustdoc chrome (`§` section marks and `Copy item path` UI strings)
- MUST expect `search-crates` echo fields after `--page-token` to match the effective URL
- MUST expect `resolved_version` for stdlib channels as `stable` when the product returns that channel label
- MUST expect SemVer resolution for `latest` to come from the target crate only (not dependency versions)
### FORBIDDEN
- NEVER scrape docs.rs HTML with regex when the CLI resolves the item
- NEVER re-scrape or re-regex markdown only to remove `§` or `Copy item path` (the product already scrubs)
- NEVER invent kinds outside the supported set
- NEVER omit `--json` in agent pipelines
- NEVER combine `--page` and `--page-token` on the same `search-crates` invocation
- NEVER use substring match as the default on noisy crates
### Correct Pattern
```bash
docsrs-cli search-crates serde --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates --page-token '<TOKEN>' --json
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
docsrs-cli readme tokio --crate-version 1.40.0 --json
docsrs-cli readme std --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
docsrs-cli get-item async-trait attribute async-trait --json
docsrs-cli get-item serde trait Serde --suggest --json
docsrs-cli search-in-crate reqwest Client --item-type struct --match prefix --limit 20 --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate std Option --json
```

## Discovery Doctor Schema Completions
### REQUIRED
- MUST run `commands --json` before inventing subcommands
- MUST run `schema --cmd <name> --json` for every payload surface you parse
- MUST run `doctor --json` when paths, TLS, or retry look wrong
- MUST run `doctor --online --json` before batch network work to probe `online_crates_io` and `online_docs_rs`
- MUST run `version --json` for binary identity
- MUST generate completions only for supported shells
- MUST expect raw shell script from `completions` unless `--json` is explicit
- MUST validate top-level `ok` and process exit code before trusting success
- MUST treat doctor as healthy only when top-level `ok` and `data.ok` are both true
- MUST still read doctor `data` when `ok` is false to inspect failed checks (exit 78)
### FORBIDDEN
- NEVER invent flags outside `commands` and `--help`
- NEVER ignore `schema_version`
- NEVER skip live `schema --cmd` for `error`, `dry-run`, `schema`, or `completions` when parsing those surfaces
- NEVER skip `doctor --online` before large batch fetches when network health is unknown
- NEVER treat doctor exit 0 with `data.ok=false` as healthy (envelope `ok` mirrors `data.ok`)
- NEVER discard doctor `data` solely because `ok` is false
### Correct Pattern
```bash
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli completions zsh
docsrs-cli completions power-shell
```

## Dry-Run Cache Config
### REQUIRED
- MUST use `--dry-run` to plan URLs without network
- MUST expect dry-run success envelopes with `dry_run=true` and `data.planned_url` plus `data.planned_params`
- MUST expect planned params to use canonical `crate_name` (never `crate`)
- MUST expect planned `limit` for search-in-crate to be clamped to 1000 even when `--limit` is higher
- MUST expect dry-run search-crates page-token echo to match the effective planned URL
- MUST use `cache stats --json` and `cache clear --json`
- MUST use `config path|show|init --json`
- MUST use `config init --force` only to overwrite an existing `config.toml`
- MUST set product knobs via CLI flags and XDG `config.toml` only
- MUST kill retries with `--disable-retry` or TOML `disable_retry = true` / `max_retries = 0`
### FORBIDDEN
- NEVER expect a runtime `.env` for product settings
- NEVER store API keys in the product
- NEVER use `config init --force` without intent to overwrite
- NEVER invent product env knobs such as `DOCSRS_CLI_USER_AGENT`, `DOCSRS_CLI_CONTACT`, `DOCSRS_CLI_DISABLE_RETRY`, `DOCSRS_CLI_TIMEOUT`
### Correct Pattern
```bash
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
docsrs-cli --dry-run search-in-crate serde Serialize --limit 5000 --json
docsrs-cli --dry-run get-item async-trait attribute async-trait --json
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config init --force --json
docsrs-cli --disable-retry doctor --json
```

## JSON Contract
### REQUIRED
- MUST expect success with `schema_version`, `ok`, `command`, `data`, `duration_ms`
- MUST expect failure with `ok:false` and `error` object on the JSON path
- MUST read `error.code`, `error.kind`, `error.message`, `error.retryable` on every failure
- MUST read optional `error.retry_after_secs` when present and honor it before retry
- MUST know wire kinds include `usage|invalid_input|not_found|rate_limited|unavailable|timeout|network|budget|parse|config|internal|broken_pipe|canceled`
- MUST prefer `data.source_url` when present; MUST treat top-level envelope `source_url` as a mirror on fetch ops only
- MUST treat missing optional fields as omitted, never invent JSON null
- MUST treat `truncated:true` in `search-in-crate` as cut by `--limit`
- MUST treat `truncated:true` in `readme` / `get-item` as emit cut by `--max-output-bytes` (success path)
- MUST read `cache_hit` (bool) on network command data when present
- MUST interpret `cache_hit:true` as disk cache serve (no network body fetch)
- MUST interpret `cache_hit:false` as miss or bypass
- MUST read canonical `crate_name` on readme / get-item / search-in-crate data (NEVER wire field `crate`)
- MUST read `search-crates` data fields `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`
- MUST read `readme` data fields `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; optional `resolved_version`
- MUST read `get-item` data fields `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; optional `resolved_version`, optional `extraction` (`method`|`item_page`)
- MUST read `search-in-crate` data fields `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; optional `item_type`
- MUST read `hits[].name`, `hits[].kind`, `hits[].url` and optional `hits[].score` on search-in-crate (lower score is better)
- MUST expect readme/get-item `markdown` without rustdoc chrome `§` and without `Copy item path`
- MUST paginate search-crates via `data.meta.next_page` / `data.meta.prev_page` as `--page-token`
- MUST trust echoed `query`/`page`/`per_page`/`sort` after `--page-token` as the effective request
- MUST parse error envelopes via `schema --cmd error` when the shape is unknown
- MUST gate retries on `error.retryable` and `error.kind`, never on exit code alone
- MUST expect human-path failures to leave stdout empty and write one stderr line
### FORBIDDEN
- NEVER trust success `data` when `ok` is false except doctor checks inspection
- NEVER mix NDJSON and envelope JSON in the same parse
- NEVER invent null for fields that the product omits instead
- NEVER rename wire fields (`crate` is forbidden; use `crate_name`)
- NEVER retry `kind=budget`
- NEVER prefer top-level `source_url` over `data.source_url` when both exist
### Correct Pattern
```bash
out=$(docsrs-cli -q get-item serde trait Serialize --json) || code=$?
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.source_url // .source_url // empty'
echo "$out" | jaq -r '.data.crate_name'
echo "$out" | jaq -r '.data.item_name'
echo "$out" | jaq -r '.data.item_type'
echo "$out" | jaq -r '.data.item_path'
echo "$out" | jaq -r '.data.title // empty'
echo "$out" | jaq -r '.data.extraction // empty'
echo "$out" | jaq -r '.data.cache_hit'
echo "$out" | jaq -r '.data.truncated'
echo "$out" | jaq -r '.data.resolved_version // empty'
echo "$out" | jaq -e '(.data.markdown // "") | (contains("§") | not)'
echo "$out" | jaq -e '(.data.markdown // "") | (contains("Copy item path") | not)'

out=$(docsrs-cli -q get-item tokio method runtime::Runtime::new --json) || code=$?
echo "$out" | jaq -r '.data.extraction // empty'
echo "$out" | jaq -r '.data.item_name'

out=$(docsrs-cli -q search-in-crate serde Serialize --match prefix --json) || code=$?
echo "$out" | jaq -r '.data.match_mode'
echo "$out" | jaq -c '.data.hits[0] | {name, kind, url, score}'

out=$(docsrs-cli -q search-crates serde --per-page 5 --json) || code=$?
token=$(echo "$out" | jaq -r '.data.meta.next_page // empty')
[ -n "$token" ] && docsrs-cli -q search-crates --page-token "$token" --json
```

## Agent Workflows — Execution Prompts
### REQUIRED
- MUST execute workflow A on unknown crate name: `search-crates` → pick name → `readme` → `search-in-crate` → `get-item`
- MUST execute workflow B on get-item 404: re-run with `--suggest`, parse suggestions from `error.message`, pick a symbol, call `get-item` again
- MUST execute workflow C for pagination: read `meta.next_page`, re-invoke `search-crates --page-token`, stop when token is empty
- MUST execute workflow D for noisy symbol names: `search-in-crate --match prefix`, then escalate to `exact` when the leaf is known
- MUST execute workflow E before batch network jobs: `doctor --online --json`, abort or retry when probes fail
- MUST execute workflow F for cache diagnosis: read `cache_hit`; use `cache stats|clear`; use `--no-cache` only when freshness is mandatory
- MUST execute workflow G for offline planning: `--dry-run` first, then live fetch after plan review
- MUST execute workflow H for meta discovery: `commands` → `schema --cmd` → `version` → optional `completions`
- MUST execute workflow I for storage knobs: `config path|show|init` and `cache stats|clear` under isolated `DOCSRS_CLI_HOME` when sandboxing
- MUST keep one-shot invocations; chain workflows in the agent, not inside a long-lived process
- MUST treat Ready Formulas as the complete 11-command argv surface; workflows are multi-step chains on top
### FORBIDDEN
- NEVER ignore suggest text after a 404 when the user still needs the symbol
- NEVER pass both `--page` and `--page-token`
- NEVER assume network is healthy without doctor when previous probes failed
- NEVER mask exit codes with `|| true` in agent pipelines
### Correct Pattern
```bash
# A) discover crate → overview → symbols → item
docsrs-cli -q search-crates async --sort downloads --json
docsrs-cli -q readme tokio --json
docsrs-cli -q search-in-crate tokio Runtime --match prefix --limit 20 --json
docsrs-cli -q get-item tokio struct runtime::Runtime --json

# B) 404 + suggest
set +e
out=$(docsrs-cli -q get-item serde trait Serde --suggest --json)
code=$?
set -e
# if code==66, parse error.message for suggestions, then:
docsrs-cli -q get-item serde trait Serialize --json

# C) page-token pagination
page=$(docsrs-cli -q search-crates async --per-page 10 --json)
token=$(echo "$page" | jaq -r '.data.meta.next_page // empty')
while [ -n "$token" ]; do
  page=$(docsrs-cli -q search-crates --page-token "$token" --json)
  token=$(echo "$page" | jaq -r '.data.meta.next_page // empty')
done

# D) match prefix then exact
docsrs-cli -q search-in-crate serde Serialize --match prefix --limit 20 --json
docsrs-cli -q search-in-crate serde Serialize --match exact --json

# E) doctor online before batch
docsrs-cli doctor --online --json
docsrs-cli -q readme tokio --json
docsrs-cli -q get-item tokio struct runtime::Runtime --json

# F) cache_hit + cache surface
docsrs-cli -q readme serde --json | jaq -r '.data.cache_hit'
docsrs-cli --no-cache -q readme serde --json | jaq -r '.data.cache_hit'
docsrs-cli cache stats --json
docsrs-cli cache clear --json

# G) dry-run then live
docsrs-cli --dry-run -q get-item tokio method runtime::Runtime::new --json
docsrs-cli -q get-item tokio method runtime::Runtime::new --json

# H) meta discovery
docsrs-cli commands --json
docsrs-cli schema --cmd error --json
docsrs-cli version --json
docsrs-cli completions bash --json

# I) storage sandbox
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config path --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config show --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli cache stats --json
```

## Exit Codes and Retry
### REQUIRED
- MUST branch on exit code before stdout
- MUST treat `0` as success
- MUST treat `2` as invalid clap argv
- MUST treat `64` usage, `65` invalid input/parse (includes timeout 0), `66` not found
- MUST treat `69` rate limit/unavailable and `124` timeout as retryable when `error.retryable` is true
- MUST treat exit `74` as ambiguous until `error.kind` is read
- MUST treat `kind=network` at exit `74` as retryable
- MUST treat `kind=budget` at exit `74` as permanent for the same config (`retryable=false`)
- MUST treat `78` config (including unhealthy doctor), `70` internal, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- MUST retry only when `error.retryable` is true (`69`, retryable `74`/`network`, `124`)
- MUST honor `error.retry_after_secs` and upstream `Retry-After` when present
- MUST raise `--max-body-bytes` on budget instead of retrying
- MUST honor kill switch `--disable-retry` or TOML `disable_retry` / `max_retries=0` only for incidents or debug
- MUST treat first Ctrl-C / SIGINT as cooperative cancel exit `130`; second Ctrl-C within 5s force-exits `130`
- MUST treat SIGTERM / SIGHUP as terminate exit `143`
- MUST treat Windows Ctrl+Break and console close as terminate exit `143`
### FORBIDDEN
- NEVER retry `64`, `65`, `66`, `78`, or `kind=budget` without changing inputs/config
- NEVER treat every exit `74` as retryable
- NEVER mask exit codes with `|| true` in agent pipelines
### Correct Pattern
```bash
set +e
out=$(docsrs-cli -q --timeout 15 get-item missing-crate-xyz struct Foo --json)
code=$?
set -e
kind=$(echo "$out" | jaq -r '.error.kind // empty')
retryable=$(echo "$out" | jaq -r '.error.retryable // false')
retry_after=$(echo "$out" | jaq -r '.error.retry_after_secs // empty')
case "$code" in
  0) echo ok ;;
  66) echo not_found ;;
  69|124) echo retryable after=${retry_after:-0} ;;
  74)
    if [ "$kind" = budget ] || [ "$retryable" = false ]; then
      echo permanent_budget_or_non_retryable
    else
      echo retryable_network after=${retry_after:-0}
    fi
    ;;
  65) echo invalid_input_or_timeout_zero ;;
  78) echo config_or_doctor_unhealthy ;;
  *) echo fail_$code ;;
esac

# budget path — raise cap, NEVER blind-retry
set +e
out=$(docsrs-cli -q --max-body-bytes 50 readme serde --json)
code=$?
set -e
echo "$out" | jaq -r '.error.kind // empty'   # budget
echo "$out" | jaq -r '.error.retryable // false'  # false
docsrs-cli -q --max-body-bytes 10485760 readme serde --json
```

## Host Allowlist and Safety
### REQUIRED
- MUST accept only product hosts crates.io, docs.rs, static.docs.rs, and doc.rust-lang.org
- MUST keep an identifiable User-Agent; MUST use the built-in default unless override is required
- MUST respect rate-limit delay and product politeness
- MUST use rustls without certificate bypass
### FORBIDDEN
- NEVER request login scraping or CAPTCHA bypass
- NEVER disable TLS validation
- NEVER treat the CLI as a generic multi-host crawler
### Correct Pattern
```bash
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli --user-agent 'my-agent (+https://example.com/contact)' version --json
```

## Ready Formulas
### REQUIRED
- MUST copy formulas below and only substitute placeholders
- MUST cover all 11 top-level commands in this list
- MUST cover page-token, match, suggest, doctor --online, limit clamp, scrub expectations, dual source_url, global flags, and all 13 schema names
```bash
# fetch surface
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates --page-token <TOKEN> --json
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --suggest --json
docsrs-cli get-item <CRATE> method <TYPE::method> --json
docsrs-cli get-item <CRATE> attribute <hyphen-or-underscore-name> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type <KIND> --match prefix --limit <N> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match substring --json
docsrs-cli search-in-crate <CRATE> "" --limit <N> --json
docsrs-cli --dry-run search-crates <QUERY> --json
docsrs-cli --dry-run search-crates --page-token <TOKEN> --json
docsrs-cli --dry-run readme <CRATE> --json
docsrs-cli --dry-run get-item <CRATE> <KIND> <PATH> --json
docsrs-cli --dry-run search-in-crate <CRATE> <QUERY> --limit 5000 --json

# meta surface
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# completions surface (raw by default)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# storage surface
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json

# global knobs / sandbox
docsrs-cli --timeout 30 --connect-timeout 5 -q doctor --json
docsrs-cli --no-cache readme <CRATE> --json
docsrs-cli --disable-retry doctor --json
docsrs-cli --max-retries 3 --retry-base-ms 100 --retry-max-delay-ms 2000 doctor --json
docsrs-cli --lang pt-BR --format markdown doctor
docsrs-cli --no-color --format text version
docsrs-cli -v doctor --json
docsrs-cli --rate-limit-delay-ms 200 search-crates <QUERY> --json
docsrs-cli --max-concurrency 0 search-in-crate <CRATE> <QUERY> --json
docsrs-cli --cache-ttl-secs 86400 --max-cache-bytes 268435456 cache stats --json
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme <CRATE> --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config path --json
DOCSRS_CLI_LANG=pt-BR docsrs-cli --format markdown doctor
```
### FORBIDDEN
- NEVER invent subcommands outside this surface
- NEVER document historical changelog narrative inside this skill
