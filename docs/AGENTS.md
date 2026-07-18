[Português (pt-BR)](AGENTS.pt-BR.md)

# Agents Guide for docsrs-cli

> Spend tokens on answers, not on scraping HTML by hand.


## Why Agents Use docsrs-cli
- Stable JSON beats fragile HTML scraping
- One process per question keeps state honest
- Exit codes make retry policy mechanical


## Economy
- Disk cache removes repeat downloads inside TTL
- Dry-run validates planned URLs without burning quota
- Truncation flags tell you when to raise limits


## Sovereignty
- No sticky MCP daemon required
- No product telemetry leaves the host
- Public docs hosts only; no login scraping


## Compatible Agents and Orchestrators
- Claude Code, Codex, Cursor, OpenCode, and any exec-capable agent
- Shell pipelines and CI jobs that can parse JSON
- Skill packages under `skills/docsrs-cli-en` and `skills/docsrs-cli-pt`


## Agent Integration Details
- Lifecycle is always one-shot: BORN, EXECUTE, FINALIZE, DIE
- Stdout is the data contract; stderr is diagnostics only
- JSON is automatic when stdout is not a TTY
- Force JSON with `--json` or `--format json`
- Force human with `--format markdown` or `--format text`
- Prefer `-q` when stderr must not pollute transcripts


## Crate Integrations
- crates.io powers `search-crates`
- docs.rs powers `readme`, `get-item`, and `search-in-crate`
- doc.rust-lang.org powers `std`, `core`, and `alloc`
- Host allowlist is fixed in the product HTTP layer


## Contract: Discovery
- Run `docsrs-cli commands --json` before inventing argv
- Run `docsrs-cli schema --cmd <name> --json` before parsing new fields
- Run `docsrs-cli doctor --json` when paths or TLS look wrong


## Contract: Success Envelope
- Success JSON includes `schema_version`, `ok:true`, `command`, `data`, `duration_ms`
- Read `data` only after `ok` is true
- Provenance fields such as `source_url` identify the fetched page


## Contract: Error Envelope
- Failure JSON includes `ok:false` and `error` with `kind` and message
- Human path failures leave stdout empty and write one stderr line
- Branch on process exit code before trusting any field


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
- Disable retries only with `--disable-retry` for incidents


## Contract: Full Command Catalog
- Surface has 11 top-level commands and nested `cache` / `config` actions
```bash
# search-crates
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates tokio --sort alphabetical --json

# readme
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json

# get-item
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json

# search-in-crate
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --limit 20 --json
docsrs-cli search-in-crate tokio "" --limit 50 --json

# meta
docsrs-cli version --json
docsrs-cli doctor --json
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

# completions
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell

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
- Accepted kinds include module, struct, trait, enum, union, fn, function, type, const, constant, static, macro, attr, attribute, derive
- `std`, `core`, and `alloc` resolve through doc.rust-lang.org


## Contract: search-crates Rules
- `--page` is 1-based
- `--per-page` max is 100
- `--sort` values are relevance, downloads, recent-downloads, recent-updates, new, alphabetical


## Contract: schema Rules
- Payload schemas exist for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config
- `schema` and `completions` do not expose payload schemas via `--cmd`
