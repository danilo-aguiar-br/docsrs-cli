[Português (pt-BR)](HOW_TO_USE.pt-BR.md)

# How to Use docsrs-cli
> Go from install to a real docs fetch in under 60 seconds.

## Prerequisites
- Install Rust 1.88 or newer with rustup
- Ensure outbound HTTPS works to crates.io and docs.rs
- Prefer a terminal with a working PATH after cargo install

## First Command in 60 Seconds
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
```
- Confirm exit code 0 after each command
- Confirm stdout is a JSON object with `"ok":true`
- Confirm `data.version` from `docsrs-cli version --json` is `1.2.0` on the 1.2 line

## Feature 1.1 → Guide
| Feature 1.1 | Guide section |
|-------------|---------------|
| `--match exact\|prefix\|substring` (default `prefix`) | Match Modes |
| `search-crates --page-token` | Pagination With page-token |
| `data.cache_hit` | cache_hit Concept |
| `item_name` / `resolved_version` / `match_mode` | JSON Fields Agents Should Read |
| `get-item` method alias + `#method.name` | Core Commands |
| `get-item --suggest` | Core Commands |
| `doctor --online` | Other Subcommands |
| Dry-run `planned_params.crate_name` | Advanced Patterns |
| Completions raw shell (JSON only with `--json`) | Advanced Patterns |
| Product knobs: flags + TOML only (no `DOCSRS_CLI_*`) | Configuration |
| Schemas `schema` / `completions` / `error` / `dry-run` | Full Command Surface |
| Upgrade from 0.1.x | [MIGRATION.md](MIGRATION.md) |

## Feature 1.2.0 → Guide
| Feature 1.2.0 | Guide section |
|-------------|---------------|
| Method missing `#method.X` → `not_found` (exit 66) | Core Commands / JSON Fields |
| Method success requires `data.extraction=method` | Core Commands / JSON Fields |
| Error envelope `command` + `duration_ms` | JSON Fields Agents Should Read |
| Budget above hard max → exit 65 | Advanced Patterns / JSON Fields |
| `--suggest` method leaves from parent page | Core Commands |
| Dry-run `validation=url_shape_only` | Advanced Patterns |
| `schema --cmd all` (19 wire names) | Full Command Surface |
| Upgrade from 1.1.x | [MIGRATION.md](MIGRATION.md) |

## Feature 1.1.x → Guide (retained)
| Feature | Guide section |
|---------|---------------|
| Effective-URL echo after `--page-token` | Pagination With page-token |
| `error.kind=budget` non-retryable (exit 74) | JSON Fields / Troubleshooting in README |
| Cascade `--suggest` (exact→prefix→substring→edit-distance) | Core Commands |
| Hyphen `item_path` → underscore normalize | Core Commands |
| Rustdoc chrome scrub (`§`) | Core Commands |
| `doctor` top-level `ok` mirrors `data.ok` | Other Subcommands |
| `--timeout 0` / `--connect-timeout 0` fail-closed (exit 65) | Advanced Patterns |
| `scripts/smoke-live.sh` human smoke | [TESTING.md](TESTING.md) |

## Core Commands
- Search the registry: `docsrs-cli search-crates tokio --json`
- Paginate and sort: `docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json`
- Continue with a page token: `docsrs-cli search-crates --page-token "$NEXT" --json`
- Fetch crate overview: `docsrs-cli readme tokio --json`
- Pin overview version: `docsrs-cli readme clap --crate-version 4.5.0 --json`
- Resolve latest SemVer for the target crate only: `docsrs-cli readme tokio --crate-version latest --json`
- Fetch stdlib overview (channel in `resolved_version`): `docsrs-cli readme std --json`
- Fetch a typed item: `docsrs-cli get-item clap trait clap::Parser --json`
- Fetch an associated method: `docsrs-cli get-item tokio method runtime::Runtime::new --json`
- Successful method payloads set `data.extraction` to `method` only
- Missing method anchors return `not_found` (exit 66); never treat parent-page markdown as method success
- Typo example (expect exit 66 + suggestions): `docsrs-cli get-item tokio method Runtime::neww --suggest --json`
- Hyphenated paths normalize: `docsrs-cli --dry-run get-item async-trait attribute async-trait --json`
- Rustdoc chrome such as `§` and "Copy item path" is scrubbed from markdown
- Suggest nearby symbols on miss: `docsrs-cli get-item serde struct Serde --suggest --json`
- Method typos: `--suggest` ranks method leaves from the parent type page
- Suggest cascade ranks exact → prefix → substring → edit-distance
- Search symbols in one crate: `docsrs-cli search-in-crate reqwest Client --json`
- Choose match mode: `docsrs-cli search-in-crate serde Serialize --match exact --json`
- List symbols with empty query: `docsrs-cli search-in-crate tokio "" --limit 50 --json`
- Discover the tree: `docsrs-cli commands --json`
- Print a payload schema: `docsrs-cli schema --cmd get-item --json`

## Full Command Surface
- `search-crates` with `--page`, `--per-page`, `--sort`, `--page-token`
- `readme` with optional `--crate-version`
- `get-item` with optional `--crate-version` and `--suggest`
- `search-in-crate` with optional `--crate-version`, `--item-type`, `--limit`, `--match`
- `version`, `doctor`, `doctor --online`, `commands`
- `schema --cmd` for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions, error, dry-run (plus aliases; use `schema --cmd all --json` for the 19-name bundle)
- `completions` for bash, zsh, fish, elvish, power-shell, powershell (raw shell by default; `--json` only when explicit)
- `cache stats` and `cache clear`
- `config path`, `config show`, `config init`, `config init --force`

## Daemon
- docsrs-cli has no daemon
- Every invocation is BORN, EXECUTE, FINALIZE, DIE
- Do not expect sticky sessions or background workers

## Match Modes
- Default for `search-in-crate` is `--match prefix`
- `exact` keeps only exact leaf (or exact full path) matches
- `prefix` ranks exact leaf first, then leaf prefixes
- `substring` restores legacy contains behavior across the path
- Hits may include `score`; lower scores rank better when present
- Example exact: `docsrs-cli search-in-crate serde Serialize --match exact --json`
- Example substring: `docsrs-cli search-in-crate serde de --match substring --limit 20 --json`

## Pagination With page-token
- First page: `docsrs-cli search-crates async --page 1 --per-page 20 --json`
- Read `data.meta.next_page` when present
- Next page: `docsrs-cli search-crates --page-token "$NEXT" --json`
- After `--page-token`, echoed `query` / `page` / `per_page` / `sort` match the effective URL
- Dry-run with the same token shows matching `planned_params`
- `--page` and `--page-token` conflict; pick one per invocation
- Tokens are opaque query strings from the previous response; do not invent them by hand

## cache_hit Concept
- Network command payloads include `data.cache_hit`
- `true` means the HTTP body was served from the local disk cache inside TTL
- `false` means a network fetch populated (or bypassed) the cache
- Use `--no-cache` to force network; use `cache stats` / `cache clear` to inspect or reset

## JSON Fields Agents Should Read
- `crate_name` is the canonical crate field on the wire
- `get-item` always emits `item_name`
- Successful method `get-item` emits `extraction=method`; missing anchors are errors, not `item_page` success
- Agents MUST reject method success when `extraction` is missing or is legacy `item_page`
- `readme` and `get-item` may emit optional `resolved_version`
- Stdlib `resolved_version` is the channel name such as `stable` (example: `docsrs-cli readme std --json`)
- For crates, `resolved_version` is the SemVer of the target crate only (never a dependency version scraped from the page)
- `search-in-crate` always echoes `match_mode`
- Ranked hits may include `score` when a query is present
- Failure envelopes expose `command`, `duration_ms`, `error.kind`, and `error.retryable`
- Never retry `kind=budget` (exit 74); raise `--max-body-bytes` only within hard max (above hard max is exit 65)
- JSON field names and technical messages stay English even when stderr is localized

## Advanced Patterns
- Plan without network: `docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json`
- Dry-run method plans document `validation=url_shape_only` and parent kind probes
- Dry-run search: `docsrs-cli --dry-run search-crates serde --json`
- Dry-run planned params use `crate_name` (not `crate`)
- Dry-run clamps `search-in-crate --limit` to 1000: `docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json`
- Explicit zero timeouts fail closed: `docsrs-cli --timeout 0 version --json` → exit 65
- Budget above hard max fails closed: `docsrs-cli --max-body-bytes 999999999 version --json` → exit 65
- Force human markdown on a pipe: `docsrs-cli --format markdown version`
- Isolate storage: `docsrs-cli --config-dir /tmp/docsrs-sandbox/config --cache-dir /tmp/docsrs-sandbox/cache doctor --json`
- Online readiness: `docsrs-cli doctor --online --json`
- Treat doctor healthy only when top-level `ok` and `data.ok` are both true
- Inspect cache: `docsrs-cli cache stats --json`
- Clear cache: `docsrs-cli cache clear --json`
- Create default config: `docsrs-cli config init --json`
- Overwrite config: `docsrs-cli config init --force --json`
- Generate completions (raw script): `docsrs-cli completions bash`
- Completions as JSON only when asked: `docsrs-cli completions bash --json`
- Other shells: `docsrs-cli completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`

## Configuration
- Precedence for product settings: CLI flags > XDG `config.toml` > built-in defaults
- Product knobs are not read from `DOCSRS_CLI_*` environment variables
- Isolate storage with `--config-dir` / `--cache-dir` only (product ignores `DOCSRS_CLI_*` path env)
- Show effective config: `docsrs-cli config show --json`
- Print resolved paths: `docsrs-cli config path --json`
- Default User-Agent is `docsrs-cli/<version> (+https://github.com/danilo-aguiar-br/docsrs-cli)` (version matches the binary)
- Override User-Agent with `--user-agent` or TOML `user_agent`
- Contact for the default UA comes from TOML `contact` (no `DOCSRS_CLI_USER_AGENT` / `DOCSRS_CLI_CONTACT`)
- Origins for mocks/tests: TOML `crates_io_origin` / `docs_rs_origin` under a sandbox home

## Other Subcommands
- `version` prints binary identity (`1.2.0` on this line)
- `doctor` validates TLS, paths, concurrency, contact, and retry policy
- `doctor` top-level `ok` mirrors `data.ok` (exit 78 when checks fail)
- `doctor --online` adds opt-in network probes to crates.io and docs.rs
- `completions <shell>` emits raw shell completion scripts by default
- `config path|show|init` manages XDG config without secrets
- `cache stats|clear` manages the HTTP disk cache
- Optional human live smoke: `scripts/smoke-live.sh` (not CI)

## Integration With AI Agents
- Always prefer `--json` for machine consumers
- Parse exit code before reading stdout
- Branch retries on `error.retryable`, not exit code alone (exit 74 may be `budget`)
- JSON stdout stays English; human stderr may follow `--lang` or `--lang`
- Read [AGENTS.md](AGENTS.md) and packaged skills [docsrs-cli-en](../skills/docsrs-cli-en/SKILL.md) / [docsrs-cli-pt](../skills/docsrs-cli-pt/SKILL.md)
- Read machine schemas under [schemas/README.md](schemas/README.md)
- Read [MIGRATION.md](MIGRATION.md) when upgrading from 0.1.x, 1.1.x, or to 1.2.0
