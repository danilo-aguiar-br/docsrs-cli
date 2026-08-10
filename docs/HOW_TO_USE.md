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
- Confirm `data.version` from `docsrs-cli version --json` is `1.3.0` on the 1.3 line

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
| Schemas `schema` / `completions` / `error` / `dry-run` / `agent-surface` | Full Command Surface |
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
| `schema --cmd all` (19 wire names at that release) | Full Command Surface |
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
- Fetch a required trait method: `docsrs-cli get-item std method iter::Iterator::next --json`
- Fetch an associated type: `docsrs-cli get-item std type iter::Iterator::Item --json`
- Fetch an associated constant: `docsrs-cli get-item std const time::Duration::MAX --json`
- `method`, `type` and `const` accept `Parent::member` when rustdoc renders the member as an anchor on the parent page
- Rustdoc uses one anchor prefix per member category, all coexisting on the parent page
- `source_url` echoes the anchor that exists (`#tymethod.next`, `#associatedtype.Item`), not the one planned
- A lowercase parent stays a free item: `docsrs-cli get-item std const u32::MAX --json` keeps its module page
- Successful member payloads set `data.extraction` to `method` only
- That value means "came from the member anchor", never "the member is a function"
- `data.anchor_family` reports which family actually matched (`variant`, `structfield`, `tymethod`, `associatedtype`, `associatedconstant`, `method`)
- Read the family from `anchor_family` and keep asserting `extraction`: one names the shape, the other rejects a parent-page false success
- Example: `docsrs-cli get-item std variant option::Option::Some --json` returns `extraction=method` with `anchor_family=variant`
- Missing member anchors return `not_found` (exit 66); never treat parent-page markdown as member success
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
- `schema --cmd` for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions, error, dry-run, agent-surface (plus aliases; use `schema --cmd all --json` for the 20-name bundle)
- `completions` for bash, zsh, fish, elvish, power-shell, powershell (raw shell by default; `--json` only when explicit)
- `cache path`, `cache stats` and `cache clear`
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
- A not-found answered by `--suggest` adds `error.suggestions`, an array of `{path, kind}` ordered best first
- Read that array instead of parsing `error.message`: each entry is a ready command line, as in `docsrs-cli get-item tokio <kind> <path>`
- The field is absent when the ranking found nothing, never JSON `null` and never an empty array
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
- Clear cache: `docsrs-cli cache clear --yes --json` (or `--cache-dir <DIR>` to name the root)
- Create default config: `docsrs-cli config init --json`
- Overwrite config: `docsrs-cli config init --force --yes --json` (or `--config-dir <DIR>`)
- Both destructive verbs exit 64 and act on nothing when given neither flag
- Generate completions (raw script): `docsrs-cli completions bash`
- Completions as JSON only when asked: `docsrs-cli completions bash --json`
- Other shells: `docsrs-cli completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`

## Payload Reduction
- Cut the payload with the CLI itself; a `jq` / `jaq` stage is no longer needed
- Project keys: `docsrs-cli --select planned_url --dry-run readme serde --json`
- Alias form: `docsrs-cli --fields planned_url --dry-run readme serde --json`
- A key absent from `data` is skipped, never emitted as null
- Filter elements: `key=value`, `key!=value`, `key~substring` (repeat the flag for AND)
- Malformed filter fails closed: `docsrs-cli --filter 'no operator' --dry-run readme serde --json` → exit 65
- Drop repeats: `docsrs-cli --dedupe-by name search-in-crate serde Serialize --json`
- Count instead of payload: `docsrs-cli --count-only --dry-run readme serde --json` → `data` is `{"count":1}`
- Sort elements: `docsrs-cli --sort-by name search-in-crate serde Serialize --json`
- The sort is stable and ascending; elements without the key sort last, never first
- Numbers compare numerically: `--sort-by downloads` puts `9` before `10`
- Cap the emission: `docsrs-cli --max-items 5 search-in-crate serde "" --limit 200 --json`
- `--max-items` bounds what is written; `search-in-crate --limit` bounds what is classified
- Top-N after a filter, with no `jaq` stage: `docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name search-in-crate serde "" --limit 200 --json`
- Shorten strings: `docsrs-cli --truncate-content 200 readme serde --json`
- Cap the envelope: `docsrs-cli --max-output-bytes 2000 search-in-crate serde "" --limit 200 --json`
- That budget drops whole hits and re-serialises after each one, so the JSON is never cut mid-string
- Measured: 1973 bytes and 12 of 62 hits survive; at `--max-output-bytes 500` only 1 hit does
- `--max-output-bytes` alone does not activate the pipeline, so `agent_surface` is absent and `data.truncated` is the signal
- Order is filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- `--count-only` counts the slice: with `--max-items 5` it reports at most 5
- Read `agent_surface` for `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- `limited` is true only when `--max-items` actually discarded something
- `emitted` is rewritten to match the reduced `hits`; `total` keeps describing the upstream index
- Full contract: `docsrs-cli schema --cmd agent-surface --json`

## Configuration
- Precedence for product settings: CLI flags > XDG `config.toml` > built-in defaults
- Product knobs are not read from `DOCSRS_CLI_*` environment variables
- No product knob is read from ANY environment variable, including `RUST_LOG`
- Steer stderr verbosity with `-q` / `-v`, or TOML `log_directive` (e.g. `docsrs_cli=debug,docsrs_cli::http=trace`)
- An explicit `-q` / `-v` outranks `log_directive`; an unparseable directive fails closed at load (exit 78), like any other bad TOML value
- The host locale (`LC_ALL` / `LC_MESSAGES` / `LANG`) picks stderr prose when `--lang` and TOML `lang` are absent; it never changes a setting and never changes stdout
- `NO_COLOR`, `TERM` and `CLICOLOR_FORCE` are honoured, and only those three: they describe the terminal device the way `isatty` does, never product configuration, and `--no-color` outranks all three
- Isolate storage with `--config-dir` / `--cache-dir` only (product ignores `DOCSRS_CLI_*` path env)
- Show effective config: `docsrs-cli config show --json`
- Print resolved paths: `docsrs-cli config path --json`
- Default User-Agent is `docsrs-cli/<version> (+https://github.com/danilo-aguiar-br/docsrs-cli)` (version matches the binary)
- Override User-Agent with `--user-agent` or TOML `user_agent`
- Contact for the default UA comes from TOML `contact` (no `DOCSRS_CLI_USER_AGENT` / `DOCSRS_CLI_CONTACT`)
- Origins for mocks/tests: TOML `crates_io_origin` / `docs_rs_origin` under a sandbox home

## Other Subcommands
- `version` prints binary identity (`1.3.0` on this line)
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
- Branch retries on `error.retryable`, not exit code alone (exit 74 may be `budget` or `io`)
- `kind=io` is a filesystem failure from the environment (full disk, read-only mount), not a product bug
- Its `retryable` follows the OS cause, so read the field rather than the kind
- JSON stdout stays English; human stderr may follow the `--lang` flag or the `lang` key in `config.toml`
- Read [AGENTS.md](AGENTS.md) and packaged skills [docsrs-cli-en](../skills/docsrs-cli-en/SKILL.md) / [docsrs-cli-pt](../skills/docsrs-cli-pt/SKILL.md)
- Read machine schemas under [schemas/README.md](schemas/README.md)
- Read [MIGRATION.md](MIGRATION.md) when upgrading from 0.1.x, 1.1.x, 1.2.x, or to 1.3.0
