---
name: docsrs-cli-en
description: This skill MUST activate when the agent needs docsrs-cli, crates.io crate search, docs.rs documentation fetch, rustdoc item lookup, readme overview, search-in-crate symbol listing, get-item typed extraction with method fail-closed, variant and structfield anchors, page-token pagination, match modes, doctor health probes, commands discovery, schema inventory, completions, cache path stats clear, config path show init, explicit target designation for destructive verbs, dry-run URL planning, XDG config.toml keys, JSON envelope parsing, agent-native payload reduction, error kinds, retryable gating, exit code branching, or hard-max caps. It MUST teach the eleven-command catalog, exact argv, every global flag, stdout data contracts with per-command fields, retry policy, workflows, and ready formulas so agents extract Rust crate documentation through structured CLI calls and NEVER through agent-side HTML regex scraping.
---


# docsrs-cli


## Identity and Execution Contract
### REQUIRED
- MUST invoke the binary as `docsrs-cli` and NEVER an alias
- MUST treat every process as BORN, EXECUTE, FINALIZE, DIE
- MUST pass `--json` on every programmatic invocation
- MUST parse stdout as the contract and stderr as diagnostics
- MUST expect automatic JSON when stdout is not a TTY
- MUST force human output with `--format markdown` or `--format text`
- MUST resolve unknown surface with `commands --json` before inventing argv
- MUST load `schema --cmd all --json` when a payload is unfamiliar
- MUST apply precedence CLI flags, then XDG `config.toml`, then defaults
- MUST accept only crates.io, docs.rs, static.docs.rs, doc.rust-lang.org
- MUST keep the built-in User-Agent unless an override is concretely required
- MUST rely on rustls with no certificate bypass
### FORBIDDEN
- NEVER assume a daemon, sticky session, telemetry, or reused process state
- NEVER parse stderr as success JSON
- NEVER set knobs through environment variables, a runtime `.env`, or API keys
- NEVER scrape docs.rs or crates.io HTML when a command resolves the need
- NEVER request login scraping, CAPTCHA bypass, multi-host crawling, or TLS bypass
- NEVER write release history or migration narrative into agent output


## Command Selection and Catalog
### REQUIRED
- MUST run `search-crates` to discover a crate on crates.io
- MUST run `readme` for the docs.rs overview docblock
- MUST run `search-in-crate` to list symbols inside one crate index
- MUST run `get-item` for the body of one typed rustdoc item
- MUST run `doctor --online --json` before batch network work
- MUST run `schema --cmd <name> --json` before parsing an unfamiliar payload
- MUST run `version --json` when binary identity is required
- MUST run `cache path|stats|clear` and `config path|show|init` for storage and knobs
- MUST know the eleven commands `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `cache`, `config`
- MUST pass `--sort` values `relevance|downloads|recent-downloads|recent-updates|new|alphabetical`
- MUST treat the `search-crates` query as optional only when `--page-token` carries it
- MUST pass `get-item` kinds `module|mod|struct|trait|enum|union|fn|function|method|type|const|constant|static|macro|attr|attribute|derive|variant|structfield|field`
- MUST treat `method` as alias of `fn` echoing `method`, `mod` of `module`, `const` of `constant`, `field` of `structfield`
- MUST know `variant` and `structfield` own NO page and REQUIRE a `Parent::member` path
- MUST expect them at `enum.Option.html#variant.Some` and `struct.Range.html#structfield.start`
- MUST expect exit 65 with a rewrite hint when either arrives without a parent
- MUST NOT expect `variant.Name.html`, which rustdoc never served
- MUST steer stderr with `-q` or `-v` or the XDG key `log_directive`, since `RUST_LOG` is NOT read
- MUST expect an unparsable `log_directive` to fail closed at load with exit 78
- MUST expect `timeout_secs = 0` in TOML to exit 78 while the same zero on the flag exits 65
- MUST know host locale steers stderr prose only, never a knob, and only absent `--lang` and TOML `lang`
- MUST know the product reads NO environment variable, and `--no-color` outranks terminal signals
- MUST expect JSON `error.message` to stay English under every `--lang`
- MUST write item paths with `::` or `/`, allow a crate prefix, and expect hyphens to become underscore
- MUST reach `std`, `core`, `alloc` through doc.rust-lang.org automatically
- MUST pin a release with `--crate-version` or `name@version` sugar
- MUST pass `--match` values `exact|prefix|substring`, read the echo `match_mode`, and know aliases `contains` and `substr`
- MUST expect an unknown `--match` token to fail closed
- MUST start lookups at `--match prefix` and escalate deliberately
- MUST pass `--item-type` to filter `search-in-crate` hits by kind
- MUST pass `--suggest` on a miss and read `error.suggestions` as `{path, kind}`, never parsing the message
- MUST know the twenty schema names `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `agent-surface`, `cache`, `cache-path`, `cache-clear`, `cache-stats`, `config`, `config-path`, `config-show`, `config-init`
- MUST pass `completions` shells `bash|zsh|fish|elvish|power-shell` with `powershell` as alias
### FORBIDDEN
- NEVER call `get-item` without a concrete kind and path
- NEVER use `search-in-crate` as a crates.io search
- NEVER treat `readme` as source-control README content
- NEVER combine `name@version` with a conflicting `--crate-version`
- NEVER combine `--page` with `--page-token`
- NEVER invent `--match-mode`, because the flag is `--match`
- NEVER invent kinds, sort values, shells, or schema names outside these lists
- NEVER expect `completions` to emit JSON without `--json`
- NEVER default to `--match substring` on noisy crates


## Explicit Target Designation
### REQUIRED
- MUST know two verbs destroy and refuse an ambient target they were never given
- MUST designate `cache clear` with `--cache-dir <DIR>` or waive with `--yes`
- MUST designate `config init --force` with `--config-dir <DIR>` or waive with `--yes`
- MUST expect exit 64 kind `usage` when neither flag nor waiver is present
- MUST read the refusal, which names the victim path and both ways out
- MUST read `data.target_source` as `cli` when argv named it and `xdg` when waived
- MUST know `cache clear` and `config init` carry `target_source` while `cache path` carries `source`
- MUST prefer naming the directory whenever the caller can compute the path
### FORBIDDEN
- NEVER waive to reach a directory the caller never resolved
- NEVER read the refusal as a bug, because argv named the verb and the environment named the victim
- NEVER expect `--yes` on any verb outside these two


## Global Flags
### REQUIRED
- MUST place global flags before or after the subcommand freely
- MUST pass `--json` or `--format json` for machine consumption
- MUST pass `--format` values `json|markdown|text` and no other token
- MUST bound time with `--timeout <SECS>` and dial-up with `--connect-timeout <SECS>`
- MUST treat either at `0` as fail-closed invalid input
- MUST cap download with `--max-body-bytes`, hard ceiling 10485760
- MUST cap emission with `--max-output-bytes`, hard ceiling 2097152
- MUST treat any value above a ceiling as fail-closed, never a silent clamp
- MUST reduce payload in the binary and NEVER pipe through `jq` or `jaq`
- MUST project with `--select <KEYS>` as CSV or repeated flag, alias `--fields`
- MUST treat a key absent from `data` as skipped, never an emitted null
- MUST know `--select` projects array ELEMENTS when the payload holds results
- MUST pass `--select name` and NEVER `--select hits`, which yields empty objects
- MUST filter with `--filter` as `key=value`, `key!=value`, `key~substring`, with `==` a synonym
- MUST repeat `--filter` to conjoin with AND
- MUST treat a malformed `--filter` as exit `65`, never an empty result set
- MUST sort with `--sort-by <KEY>`, taking the same dotted paths as `--select`
- MUST expect a STABLE sort, numeric comparison for numbers, and missing keys landing LAST
- MUST treat `--sort-by` on an absent key as a no-op, never an error
- MUST cap emitted elements with `--max-items <N>`
- MUST distinguish `--max-items`, bounding EMISSION, from `--limit`, bounding the QUERY
- MUST NEVER expect a global `--limit`, since `search-in-crate` owns that name
- MUST treat `--max-items 0` as an empty array with `ok: true`
- MUST pair `--max-items` with `--sort-by`, since an unordered slice is arbitrary
- MUST drop repeats with `--dedupe-by <KEY>`, keeping elements lacking it
- MUST replace payload with `{"count": N}` using `--count-only`, counted after filter and limit
- MUST shorten strings with `--truncate-content <N>`, never splitting UTF-8
- MUST apply the order filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- MUST read top-level `agent_surface` keys `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- MUST read `limited` to tell a small result from a `--max-items` cut
- MUST expect `emitted` rewritten to the reduced array while `total` survives untouched
- MUST control cache with `--no-cache`, `--cache-ttl-secs`, `--max-cache-bytes`
- MUST treat defaults 86400 and 268435456, and `--max-cache-bytes 0` as unlimited
- MUST isolate storage with `--config-dir` and `--cache-dir`
- MUST throttle with `--rate-limit-delay-ms` and size workers with `--max-concurrency`, where `0` autosizes
- MUST tune retries with `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms`, `--retry-max-elapsed-ms`
- MUST treat `--retry-max-elapsed-ms 0` as derived from the timeout
- MUST kill retries with `--disable-retry` only during incidents
- MUST plan without sockets using `--dry-run`
- MUST override identity with `--user-agent` only when concretely required
- MUST set stderr locale with `--lang en` or `--lang pt-BR` while JSON stays English
- MUST silence stderr with `-q`, deepen tracing with countable `-v`, disable ANSI with `--no-color`
- MUST enable `--allow-loopback` only for local offline test origins
- MUST treat `--per-page` default as 10 with legal range 1 through 100
- MUST treat explicit `--per-page` or `--page` outside range as fail-closed
- MUST expect pagination carried by `--page-token` to clamp instead of reject
- MUST treat `--limit` default as 100 with a silent clamp at 1000
### FORBIDDEN
- NEVER expect `--per-page 500` to clamp, since explicit argv fails closed
- NEVER pass `--page 0`, since the minimum page is one
- NEVER combine `--json` with `--format markdown` or `--format text`
- NEVER invent environment knobs for timeout, identity, paths, or retry
- NEVER use `--allow-loopback` against production hosts
- NEVER raise a budget cap above the hard ceiling


## JSON Contract and Data Fields
### REQUIRED
- MUST expect success envelopes with `schema_version`, `ok`, `command`, `data`, `duration_ms`
- MUST expect failure envelopes with the same keys plus nested `error`
- MUST read `error.code`, `error.kind`, `error.message`, `error.retryable` on every failure
- MUST read `error.retry_after_secs` only when present and honor it
- MUST know kinds `usage`, `invalid_input`, `not_found`, `rate_limited`, `unavailable`, `timeout`, `network`, `budget`, `parse`, `config`, `io`, `internal`, `broken_pipe`, `canceled`
- MUST treat omitted optional fields as absent and NEVER invent null
- MUST read canonical `crate_name` and NEVER the wire name `crate`
- MUST prefer `data.source_url` over any top-level mirror
- MUST read `cache_hit` true for a disk serve, false for a miss or bypass
- MUST read `search-crates` data `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`, `truncated`
- MUST read required hit fields `name`, `description`, `downloads`, `version`
- MUST read optional hit fields `exact_match`, `yanked`, `recent_downloads`, `max_version`, `max_stable_version`, `default_version`, `homepage`, `documentation`, `repository`
- MUST use `exact_match` to find the literal crate name inside a ranked list
- MUST use `yanked` to reject a withdrawn release before fetching it
- MUST use `max_stable_version` to pin instead of guessing from `version`
- MUST read `data.meta.total` as the upstream result count
- MUST paginate with `meta.next_page` and `meta.prev_page` fed back as `--page-token`
- MUST expect `prev_page` omitted first and `next_page` omitted last
- MUST read `readme` data `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`
- MUST read `get-item` data `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`
- MUST read optional `resolved_version` and `resolved_item_path`
- MUST require `data.extraction` equal to `method` on a successful method fetch
- MUST read `data.anchor_family` for the real family, since `extraction` says `method` for all
- MUST treat a missing anchor as `not_found`, never a parent-page success
- MUST read `search-in-crate` data `crate_name`, `query`, `version`, `match_mode`, `item_type`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`
- MUST compare `emitted` against `total` to see how much the index was cut
- MUST read `hits[].name`, `hits[].kind`, `hits[].url`, optional `hits[].score` where lower ranks better
- MUST read `version` data `name`, `version`, `msrv`, `os`, `arch`
- MUST read `commands` data `name`, `version`, `msrv`, `schema_version`, `commands`, `agent_notes`
- MUST read `agent_notes` keys `stdout`, `stderr`, `json_auto`, `lifecycle`
- MUST read `completions` data `shell` and `script`
- MUST read `doctor` data `ok` and `checks` with `name`, `ok`, `detail`
- MUST expect twenty offline checks plus exactly `online_crates_io` and `online_docs_rs` under `--online`
- MUST read `cache path` data `root`, `source` as `cli|xdg|unresolved`, and `no_cache`
- MUST read `cache stats` data `root`, `layout`, `entries`, `total_bytes`, `max_bytes`, `ttl_secs`, `parser_version`
- MUST read `cache clear` data `root`, `removed_entries`, `freed_bytes`, `target_source`
- MUST read `config path` data `config_dir`, `config_file`, `config_source`, `config_file_exists`, `config_toml_loaded`, `cache_dir`, `cache_source`, `dotenv_runtime`, `secrets_layers`
- MUST read `config init` data `path`, `config_dir`, `target_source`, `created`, `overwritten`
- MUST NEVER read `config init` data as `source`, because the field is `target_source`
- MUST read `config show` as effective knobs after defaults, TOML, then flags
- MUST read `config show` numbers `timeout_secs`, `connect_timeout_secs`, `max_body_bytes`, `max_output_bytes`, `max_redirects`, `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `rate_limit_delay_ms`, `max_concurrency`, `cache_ttl_secs`, `max_cache_bytes`
- MUST read `config show` rest `user_agent`, `config_dir`, `cache_dir`, `crates_io_origin`, `docs_rs_origin`, `disable_retry`, `no_cache`, `allow_loopback`, `config_path_source`, `cache_path_source`, `config_toml_loaded`
- MUST expect `lang`, `log_directive`, `contact` OMITTED from `config show` until set
- MUST expect readme and get-item markdown already scrubbed of rustdoc chrome
- MUST gate retries on `error.retryable` and `error.kind`, never on exit code alone
### FORBIDDEN
- NEVER trust `data` when `ok` is false, except doctor checks
- NEVER accept `extraction` absent or `item_page` as method success
- NEVER re-scrub markdown for marks already removed
- NEVER rename wire fields or mix envelope JSON with NDJSON in one parse
- NEVER apply a fallback when reading `retryable`, `ok`, `cache_hit`, `truncated`, `empty`
- NEVER confuse a legitimate false with an absent field


## Exit Codes and Retry
### REQUIRED
- MUST branch on the exit code before trusting stdout
- MUST treat `0` as success
- MUST treat `64` as usage, covering bad argv, unknown subcommand, flag conflict, refused ambient target
- MUST treat `65` as invalid input or parse failure, including timeout zero and CLI hard-max overshoot
- MUST treat `66` as not found, including a missing method anchor
- MUST treat `69` as rate limited or unavailable, therefore retryable
- MUST treat `74` as ambiguous until `error.kind` is read
- MUST treat `network` at `74` as retryable and `budget` at `74` as permanent for that configuration
- MUST treat `io` at `74` as a local filesystem failure from the environment
- MUST read `error.retryable` for `io`, since a full disk clears and a denial does not
- NEVER expect `internal` for a filesystem failure, since `internal` means a defect here
- MUST treat `78` as configuration failure, including unhealthy doctor and TOML hard-max overshoot
- MUST treat `70` internal, `124` timeout, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- MUST retry only when `error.retryable` is true
- MUST honor `error.retry_after_secs` before the next attempt
- MUST resolve a budget failure by raising the cap toward the hard maximum
- MUST read doctor `data` even when `ok` is false, to inspect failed checks
- MUST treat a first interrupt as cooperative cancel and a repeat as forced exit
### FORBIDDEN
- NEVER retry `64`, `65`, `66`, `78`, or `budget` without changing inputs
- NEVER treat every `74` as retryable
- NEVER mask exit codes with a success fallback
- NEVER capture the exit status after a pipe, since it reports the last stage


## Dry-Run Cache and XDG Config
### REQUIRED
- MUST plan offline with `--dry-run`, reading top-level `dry_run` and `data.planned_url`
- MUST read `data.planned_params`, whose keys differ by command
- MUST expect `crate_name`, `item_type`, `item_path`, `version` for `get-item` and `search-in-crate`
- MUST expect `q`, `per_page`, `sort`, `page` for `search-crates`, which carries NO `crate_name`
- MUST expect a planned `search-in-crate` limit already clamped to 1000
- MUST read `validation`, `planned_parent_kind`, `parent_kind_probe`, `planned_method_anchors` INSIDE `planned_params`
- MUST read `planned_method_anchors` as the ordered anchors a live fetch would probe
- MUST expect `validation` equal to `url_shape_only`, which is why dry-run is never anchor proof
- MUST expect nested command values `cache-path`, `cache-stats`, `cache-clear`, `config-path`, `config-show`, `config-init`
- MUST expect `schema --cmd <name>` data `command`, `schema`, `schema_version`
- MUST expect `schema --cmd all` data `mode`, `commands`, `items`, `schema_version`
- MUST create the optional file with `config init` and overwrite only with `--force`
- MUST inspect effective knobs with `config show --json`
- MUST set persistent knobs only through CLI flags and XDG `config.toml`
- MUST know TOML keys `timeout_secs`, `connect_timeout_secs`, `max_body_bytes`, `max_output_bytes`, `max_redirects`, `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `disable_retry`, `rate_limit_delay_ms`, `max_concurrency`, `user_agent`, `contact`, `lang`, `log_directive`, `crates_io_origin`, `docs_rs_origin`, `allow_loopback`, `cache_ttl_secs`, `max_cache_bytes`, `no_cache`
- MUST use `max_redirects`, `contact`, `crates_io_origin`, `docs_rs_origin` through TOML, since no flag exposes them
- MUST kill retries persistently with `disable_retry = true` or `max_retries = 0`
- MUST enable loopback persistently with `allow_loopback = true`
### FORBIDDEN
- NEVER treat a planned URL as proof a live anchor exists
- NEVER run `config init --force` without intent to overwrite
- NEVER declare a knob outside the key list above


## Execution Workflows
### REQUIRED
- MUST resolve an unknown crate by chaining `search-crates`, `readme`, `search-in-crate`, `get-item`
- MUST recover a miss by re-running with `--suggest`, reading suggestions, then refetching
- MUST paginate by reading `meta.next_page`, feeding `--page-token`, stopping on an empty token
- MUST narrow noisy symbols starting at `--match prefix` and escalating to `--match exact`
- MUST probe with `doctor --online --json` and abort batch work when probes fail
- MUST diagnose caching by reading `cache_hit`, inspecting `cache stats`, clearing only when stale
- MUST plan risky fetches with `--dry-run` before the live call
- MUST discover surface by chaining `commands`, `schema --cmd all`, then the targeted schema
- MUST sandbox experiments by pairing `--config-dir` with `--cache-dir` on every call
- MUST fix a budget failure by raising `--max-body-bytes`, and an overshoot by lowering it
- MUST fix a refused target by naming the directory rather than reaching for the waiver
- MUST capture stdout and the exit status separately before parsing
- MUST chain workflows inside the agent, since the process never persists state
### FORBIDDEN
- NEVER ignore suggestions when the caller still needs the symbol
- NEVER assume healthy network without doctor after a probe failure
- NEVER treat dry-run output as evidence a live method exists


## Ready Formulas
### REQUIRED
- MUST copy these verbatim and substitute only the placeholders
- MUST run `docsrs-cli search-crates <QUERY> --json`
- MUST run `docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --json`
- MUST run `docsrs-cli search-crates <QUERY> --sort downloads --json`, swapping for `relevance`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- MUST run `docsrs-cli search-crates --page-token <TOKEN> --json`
- MUST run `docsrs-cli readme <CRATE> --json`
- MUST run `docsrs-cli readme <CRATE>@<VERSION> --json`, equal to `--crate-version <VERSION>`
- MUST run `docsrs-cli readme std --json`
- MUST run `docsrs-cli get-item <CRATE> <KIND> <PATH> --json`
- MUST run `docsrs-cli get-item <CRATE>@<VERSION> <KIND> <PATH> --suggest --json`
- MUST run `docsrs-cli get-item serde trait Serialize --json`
- MUST run `docsrs-cli get-item tokio struct runtime::Runtime --json`
- MUST run `docsrs-cli get-item tokio struct runtime/Runtime --json`
- MUST run `docsrs-cli get-item tokio method runtime::Runtime::new --json`
- MUST run `docsrs-cli get-item std method iter::Iterator::next --json`
- MUST run `docsrs-cli get-item std type iter::Iterator::Item --json`
- MUST run `docsrs-cli get-item std const time::Duration::MAX --json`
- MUST run `docsrs-cli get-item std const u32::MAX --json`
- MUST run `docsrs-cli get-item std variant option::Option::Some --json`
- MUST run `docsrs-cli get-item std structfield ops::Range::start --json`
- MUST run `docsrs-cli get-item async-trait attribute async-trait --json`
- MUST run `docsrs-cli get-item tokio fn task::spawn --json`
- MUST run `docsrs-cli get-item tokio mod runtime --json`
- MUST run `docsrs-cli get-item serde derive Serialize --json`
- MUST know the PARENT segment case alone routes `method`, `type` and `const`
- MUST read an uppercase parent as a type reaching a parent anchor, since an uppercase module fires `non_snake_case`
- MUST expect a lowercase parent to keep its own page, and the LEAF case to change nothing
- MUST read a case typo as a not-found carrying suggestions, never an unsupported shape
- MUST run `docsrs-cli search-in-crate <CRATE> <QUERY> --json`
- MUST run `docsrs-cli search-in-crate <CRATE> "" --limit 50 --json`
- MUST run `docsrs-cli search-in-crate <CRATE> <QUERY> --item-type struct --json`
- MUST run `docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json`, swapping for `prefix`, `substring`, `contains`, `substr`
- MUST run `docsrs-cli search-in-crate <CRATE> <QUERY> --crate-version <VERSION> --limit 25 --json`
- MUST run `docsrs-cli version --json`
- MUST run `docsrs-cli doctor --json`
- MUST run `docsrs-cli doctor --online --json`
- MUST run `docsrs-cli commands --json`
- MUST run `docsrs-cli schema --cmd all --json`
- MUST run `docsrs-cli schema --cmd get-item --json`, swapping for any of the twenty names
- MUST run `docsrs-cli schema --cmd agent-surface --json` for the reduction contract
- MUST run `docsrs-cli completions bash`, swapping for `zsh`, `fish`, `elvish`, `power-shell`
- MUST run `docsrs-cli completions bash --json` to wrap the script in an envelope
- MUST run `docsrs-cli cache path --json`
- MUST run `docsrs-cli cache stats --json`
- MUST run `docsrs-cli cache clear --cache-dir <DIR> --json` to purge a named root
- MUST run `docsrs-cli cache clear --yes --json` to purge the ambient root on purpose
- MUST run `docsrs-cli config path --json`
- MUST run `docsrs-cli config show --json`
- MUST run `docsrs-cli config init --json`
- MUST run `docsrs-cli config init --force --config-dir <DIR> --json` to overwrite a named directory
- MUST run `docsrs-cli config init --force --yes --json` to overwrite the ambient one on purpose
- MUST run `docsrs-cli --dry-run readme <CRATE> --json`
- MUST run `docsrs-cli --dry-run get-item <CRATE> method <TYPE::method> --json`
- MUST run `docsrs-cli --dry-run search-crates <QUERY> --json`
- MUST run `docsrs-cli --dry-run search-in-crate <CRATE> <QUERY> --limit 5000 --json` to prove the clamp
- MUST run `docsrs-cli --timeout 30 --connect-timeout 5 -q readme <CRATE> --json`
- MUST run `docsrs-cli --no-cache readme <CRATE> --json`
- MUST run `docsrs-cli --cache-ttl-secs 86400 --max-cache-bytes 268435456 cache stats --json`
- MUST run `docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme <CRATE> --json`
- MUST run `docsrs-cli --select planned_url --dry-run readme <CRATE> --json`, or `--fields` as alias
- MUST run `docsrs-cli --count-only --dry-run readme <CRATE> --json`
- MUST run `docsrs-cli --truncate-content 200 readme <CRATE> --json`
- MUST run `docsrs-cli --dedupe-by name search-in-crate <CRATE> <QUERY> --json`
- MUST run `docsrs-cli --sort-by downloads search-crates <QUERY> --json`
- MUST run `docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name search-in-crate <CRATE> "" --limit 200 --json`
- MUST run `docsrs-cli --max-items 5 --count-only search-in-crate <CRATE> "" --limit 200 --json`
- MUST run `docsrs-cli --filter 'command=readme' --filter 'ok!=false' --dry-run readme <CRATE> --json`
- MUST run `docsrs-cli --filter 'key without operator' --dry-run readme <CRATE> --json` to prove fail-closed filtering
- MUST run `docsrs-cli --rate-limit-delay-ms 200 search-crates <QUERY> --json`
- MUST run `docsrs-cli --max-concurrency 0 search-in-crate <CRATE> <QUERY> --json`
- MUST run `docsrs-cli --max-retries 3 --retry-base-ms 100 --retry-max-delay-ms 2000 --retry-max-elapsed-ms 10000 doctor --json`
- MUST run `docsrs-cli --disable-retry doctor --json`
- MUST run `docsrs-cli --user-agent 'my-agent (+https://example.com/contact)' version --json`
- MUST run `docsrs-cli --lang pt-BR --format markdown doctor`
- MUST run `docsrs-cli --no-color --format text version`
- MUST run `docsrs-cli -v doctor --json`
- MUST run `docsrs-cli --config-dir <CFG> --cache-dir <CACHE> config path --json`
- MUST run `docsrs-cli --allow-loopback doctor --json`
### FORBIDDEN
- NEVER invent argv outside this surface
- NEVER document release history inside this skill
