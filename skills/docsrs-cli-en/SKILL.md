---
name: docsrs-cli
description: This skill MUST activate when the agent needs docsrs-cli, crates.io search, docs.rs fetch, rustdoc item lookup, get-item, readme, search-crates, search-in-crate, version, doctor online, commands, schema, completions, cache, config, page-token pagination, match mode exact prefix substring, get-item suggest, cache_hit interpretation, agent JSON envelopes, dry-run URL planning, XDG cache or config, timeout offline handling, stderr locale via --lang, or host allowlist work. It MUST teach the full command catalog, exact argv, global flags, stdout JSON contracts, exit codes, retry policy, multi-step agent workflows, and one-shot BORN-EXECUTE-FINALIZE-DIE lifecycle so agents fetch Rust crate docs without scraping HTML.
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
- MUST keep JSON envelopes in English always; MUST force human stderr locale only with `--lang en` or `--lang pt-BR`
### FORBIDDEN
- NEVER assume a daemon, sticky session, or product telemetry channel
- NEVER parse stderr as success JSON
- NEVER reuse process state across invocations
- NEVER invent a subcommand outside the catalog below
- NEVER use product env vars such as `DOCSRS_CLI_USER_AGENT`, `DOCSRS_CLI_CONTACT`, `DOCSRS_CLI_DISABLE_RETRY`, or `DOCSRS_CLI_TIMEOUT`
- NEVER put version-history narrative inside this skill
### Correct Pattern
```bash
docsrs-cli --version
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli --lang pt-BR doctor --json
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
# find crate name
docsrs-cli search-crates serde --sort downloads --json
# overview
docsrs-cli readme serde --json
# list or filter symbols
docsrs-cli search-in-crate serde Serialize --match prefix --limit 20 --json
# fetch one typed item
docsrs-cli get-item serde trait Serialize --json
# fetch associated method
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
# PATH: uses :: or / ; leading crate prefix is allowed

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

# 8) schema  (payload / envelope schemas for all inventory names)
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
- MUST place global flags before the subcommand when required by the invocation style you copy from formulas below
- MUST use `--json` or `--format json` for agent pipelines
- MUST use `--timeout <SECS>` and `--connect-timeout <SECS>` for wall-clock and connect limits
- MUST use `--no-cache` only when freshness is mandatory
- MUST use `--cache-ttl-secs`, `--max-cache-bytes`, `--cache-dir`, `--config-dir` to control disk cache and config roots
- MUST use `--max-body-bytes` and `--max-output-bytes` to cap download and emit size (hard product ceilings apply)
- MUST use `--rate-limit-delay-ms` and `--max-concurrency` for politeness and parse workers
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
- MUST isolate storage with path env only `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
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
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme serde --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config show --json
```

## Search and Fetch Execution
### REQUIRED
- MUST use `search-crates` for crates.io search
- MUST use `readme` for docs.rs crate overview (not git README)
- MUST use `get-item` for typed rustdoc items
- MUST use `search-in-crate` for symbols in `all.html`
- MUST accept `item_path` with `::` or `/`
- MUST treat `std`, `core`, and `alloc` via doc.rust-lang.org
- MUST pass `--crate-version` when a concrete version is required
- MUST use `--match prefix` (default) to reduce Serialize-style noise; escalate to `exact` or `substring` deliberately
- MUST use `get-item --suggest` on 404 to obtain nearby symbols (extra request)
- MUST treat `method` as alias of `fn`; associated methods resolve to parent type page plus `#method.name`
- MUST expect `resolved_version` for stdlib channels as `stable` when the product returns that channel label
- MUST expect SemVer resolution for `latest` to come from the target crate only (not dependency versions)
### FORBIDDEN
- NEVER scrape docs.rs HTML with regex when the CLI resolves the item
- NEVER invent kinds outside the supported set
- NEVER omit `--json` in agent pipelines
- NEVER combine `--page` and `--page-token` on the same `search-crates` invocation
- NEVER use substring match as the default on noisy crates
### Correct Pattern
```bash
docsrs-cli search-crates serde --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates --page-token '<TOKEN>' --json
docsrs-cli readme tokio --crate-version 1.40.0 --json
docsrs-cli readme std --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
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
- MUST validate `ok == true` before reading `data`
### FORBIDDEN
- NEVER invent flags outside `commands` and `--help`
- NEVER ignore `schema_version`
- NEVER skip live `schema --cmd` for `error`, `dry-run`, `schema`, or `completions` when parsing those surfaces
- NEVER skip `doctor --online` before large batch fetches when network health is unknown
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
docsrs-cli --dry-run search-in-crate serde Serialize --limit 5000 --json
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
- MUST expect failure with `ok:false` and `error.kind` on the JSON path
- MUST read `source_url` as provenance when present
- MUST treat `truncated:true` in `search-in-crate` as cut by `--limit`
- MUST read `cache_hit` (bool) on network command data when present
- MUST interpret `cache_hit:true` as disk cache serve (no network body fetch)
- MUST interpret `cache_hit:false` as miss or bypass
- MUST read canonical `crate_name` on readme / get-item / search-in-crate data
- MUST read `item_name` on get-item data (leaf name; for methods the method leaf such as `new`)
- MUST read `resolved_version` when present (stdlib channel MUST be treated as `stable` when returned)
- MUST read echoed `match_mode` and `item_type` when present on search-in-crate data
- MUST read `hits[].score` on search-in-crate (lower is better; product sorts by score)
- MUST paginate search-crates via `data.meta.next_page` / `data.meta.prev_page` as `--page-token`
- MUST parse error envelopes via `schema --cmd error` when the shape is unknown
### FORBIDDEN
- NEVER trust `data` when `ok` is false
- NEVER mix NDJSON and envelope JSON in the same parse
- NEVER invent null for fields that the product omits instead
- NEVER rename wire fields (`crate` is forbidden; use `crate_name`)
### Correct Pattern
```bash
out=$(docsrs-cli -q get-item serde trait Serialize --json) || code=$?
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.source_url'
echo "$out" | jaq -r '.data.crate_name'
echo "$out" | jaq -r '.data.item_name'
echo "$out" | jaq -r '.data.cache_hit'
echo "$out" | jaq -r '.data.resolved_version // empty'

out=$(docsrs-cli -q search-in-crate serde Serialize --match prefix --json) || code=$?
echo "$out" | jaq -r '.data.match_mode'
echo "$out" | jaq -c '.data.hits[0] | {name, kind, score}'

out=$(docsrs-cli -q search-crates serde --per-page 5 --json) || code=$?
token=$(echo "$out" | jaq -r '.data.meta.next_page // empty')
[ -n "$token" ] && docsrs-cli -q search-crates --page-token "$token" --json
```

## Agent Workflows — Execution Prompts
### REQUIRED
- MUST execute workflow A on unknown crate name: `search-crates` → pick name → `readme` → `search-in-crate` → `get-item`
- MUST execute workflow B on get-item 404: re-run with `--suggest`, parse suggestions, pick a symbol, call `get-item` again
- MUST execute workflow C for pagination: read `meta.next_page`, re-invoke `search-crates --page-token`, stop when token is empty
- MUST execute workflow D for noisy symbol names: `search-in-crate --match prefix`, then escalate to `exact` when the leaf is known
- MUST execute workflow E before batch network jobs: `doctor --online --json`, abort or retry when probes fail
- MUST execute workflow F for cache diagnosis: read `cache_hit`; use `--no-cache` only when freshness is mandatory
- MUST execute workflow G for offline planning: `--dry-run` first, then live fetch after plan review
- MUST keep one-shot invocations; chain workflows in the agent, not inside a long-lived process
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

# F) cache_hit interpretation
docsrs-cli -q readme serde --json | jaq -r '.data.cache_hit'
docsrs-cli --no-cache -q readme serde --json | jaq -r '.data.cache_hit'

# G) dry-run then live
docsrs-cli --dry-run -q get-item tokio method runtime::Runtime::new --json
docsrs-cli -q get-item tokio method runtime::Runtime::new --json
```

## Exit Codes and Retry
### REQUIRED
- MUST branch on exit code before stdout
- MUST treat `0` as success
- MUST treat `2` as invalid clap argv
- MUST treat `64` usage, `65` invalid input/parse, `66` not found
- MUST treat `69` rate limit/unavailable, `74` network, `124` timeout as retryable
- MUST treat `78` config, `70` internal, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- MUST retry only `69`, `74`, and `124` with backoff
- MUST honor kill switch `--disable-retry` or TOML `disable_retry` / `max_retries=0` only for incidents or debug
- MUST treat first Ctrl-C / SIGINT as cooperative cancel exit `130`; second Ctrl-C within 5s force-exits `130`
- MUST treat SIGTERM / SIGHUP as terminate exit `143`
- MUST treat Windows Ctrl+Break and console close as terminate exit `143`
### FORBIDDEN
- NEVER retry `64`, `65`, `66`, or `78` without changing inputs
- NEVER mask exit codes with `|| true` in agent pipelines
### Correct Pattern
```bash
set +e
docsrs-cli -q --timeout 15 get-item missing-crate-xyz struct Foo --json
code=$?
set -e
case "$code" in
  0) echo ok ;;
  66) echo not_found ;;
  69|74|124) echo retryable ;;
  *) echo fail_$code ;;
esac
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
- MUST cover page-token, match, suggest, doctor --online, limit clamp, global flags, and all 13 schema names
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
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type <KIND> --match prefix --limit <N> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match substring --json
docsrs-cli search-in-crate <CRATE> "" --limit <N> --json
docsrs-cli --dry-run search-crates <QUERY> --json
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
docsrs-cli --cache-ttl-secs 86400 --max-cache-bytes 268435456 cache stats --json
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme <CRATE> --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config path --json
```
### FORBIDDEN
- NEVER invent subcommands outside this surface
- NEVER document historical changelog narrative inside this skill
