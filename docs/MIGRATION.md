[Português (pt-BR)](MIGRATION.pt-BR.md)

# Migration
## What Changes
- Public product line is `1.1.x` (this release is `1.1.0`)
- Dual license remains MIT OR Apache-2.0
- Documentation framework stays bilingual with skills under `skills/`
- Command surface for agents remains one-shot JSON on stdout
- Product knobs no longer come from `DOCSRS_CLI_*` environment variables
- Path sandbox env vars still work for home, config, and cache directories

## Migrating from 0.1.x → 1.1.0
- Install or upgrade with `cargo install docsrs-cli --locked --force`
- Run `docsrs-cli version --json` and confirm `data.version` is `1.1.0` (or any `1.1.x`)
- Re-read `docsrs-cli commands --json` if your agent cached an older argv tree
- Re-read `docsrs-cli schema --cmd <name> --json` before parsing new required fields
- Point skills and docs links at the 1.1 layout and contracts

### Breaking: search-in-crate default match
- Default `--match` is now `prefix` (exact leaf or leaf prefix)
- 0.1.x behaved like substring contains on the symbol path
- For legacy contains behavior use `--match substring`
- Optional modes: `exact`, `prefix`, `substring`
- Ranked hits may include `hits[].score` (lower is better when present)
- Before (0.1.x implicit substring, partial data shape):
```json
{
  "crate_name": "serde",
  "query": "Serialize",
  "hits": [{ "name": "ser::Serialize", "kind": "trait", "url": "https://docs.rs/..." }]
}
```
- After (1.1.x default `prefix` + required fields):
```json
{
  "crate_name": "serde",
  "query": "Serialize",
  "match_mode": "prefix",
  "cache_hit": false,
  "hits": [{ "name": "ser::Serialize", "kind": "trait", "url": "https://docs.rs/...", "score": 0 }]
}
```
- To restore 0.1 contains behavior: `docsrs-cli search-in-crate serde Serialize --match substring --json`

### Breaking: product knobs no longer from env
- Timeouts, retries, cache TTL, body/output caps, rate limit, concurrency, User-Agent, contact, and origins are not read from `DOCSRS_CLI_*`
- Set product knobs with CLI flags and/or XDG `config.toml` only
- Precedence for product settings: CLI flags > XDG `config.toml` > built-in defaults
- Path env still allowed:
  - `DOCSRS_CLI_HOME` (sandbox root for config + cache)
  - `DOCSRS_CLI_CONFIG_DIR`
  - `DOCSRS_CLI_CACHE_DIR`
- Locale override `DOCSRS_CLI_LANG` remains for human stderr only

### Breaking: dry-run planned_params
- Dry-run `planned_params` use `crate_name`, not `crate`
- Example keys for get-item dry-run: `crate_name`, `item_type`, `item_path`, `version`
- Before (0.1.x / early drafts used `crate`):
```json
{
  "planned_url": "https://docs.rs/tokio/latest/tokio/struct.Runtime.html",
  "planned_params": {
    "crate": "tokio",
    "item_type": "struct",
    "item_path": "runtime::Runtime",
    "version": "latest"
  }
}
```
- After (1.1.x canonical wire field):
```json
{
  "planned_url": "https://docs.rs/tokio/latest/tokio/struct.Runtime.html",
  "planned_params": {
    "crate_name": "tokio",
    "item_type": "struct",
    "item_path": "runtime::Runtime",
    "version": "latest"
  }
}
```

### Breaking: completions default output
- `completions` emit raw shell scripts by default, even on non-TTY stdout
- JSON envelope is emitted only when you pass `--json` or `--format json` explicitly
- Pipelines that assumed auto-JSON for completions must add `--json`

## New Flags and Commands Behavior
- `search-in-crate --match exact|prefix|substring` (default `prefix`)
- `search-crates --page-token <token>` consumes opaque tokens from `meta.next_page` / `meta.prev_page`
- `get-item --suggest` on 404 lists nearby symbols from `all.html` (extra request)
- `doctor --online` probes crates.io / docs.rs over the network (opt-in DNS/connectivity checks)
- `get-item` accepts item type alias `method` (same as `fn` / `function`)
- Associated methods such as `Runtime::new` resolve to the parent type page plus `#method.name`

## New JSON Data Fields
- Network payloads expose `cache_hit` (local disk cache only; no remote telemetry)
- `get-item` requires `item_name` and may include optional `resolved_version`
- `search-in-crate` requires `match_mode` and may echo optional `item_type`
- `search-in-crate` hits may include optional `score`
- `readme` and `search-crates` also require `cache_hit`
- `readme` / `get-item` may include optional `resolved_version` (omitted when unknown; never JSON null)
- Crate `resolved_version` is the SemVer of the target crate only (not dependency versions on the page)
- Stdlib `resolved_version` is a channel name such as `stable` (example: `docsrs-cli readme std --json`)
- Default User-Agent is `docsrs-cli/1.1.0 (+https://github.com/danilo-aguiar-br/docsrs-cli)`
- JSON envelopes stay English; human stderr may use `--lang` / `DOCSRS_CLI_LANG`
- Error envelopes expose `error.code`, `error.kind`, `error.message`, `error.retryable`, optional `retry_after_secs`
- Live schema list now includes `schema`, `completions`, `error`, and `dry-run` via `schema --cmd`

## Step-by-Step Migration
- Upgrade the binary and confirm version `1.1.0`
- Move any former `DOCSRS_CLI_*` product env settings into flags or `config.toml`
- Keep path isolation with `DOCSRS_CLI_HOME` / `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` as needed
- Replace bare `search-in-crate` substring assumptions with `--match substring` when required
- Update dry-run parsers to read `planned_params.crate_name`
- Stop expecting auto-JSON from `completions` without `--json`
- Teach agents about `--page-token`, `--suggest`, and `doctor --online`
- Run `docsrs-cli doctor --json` (and optionally `doctor --online --json`) after path/config moves

## JSON Schema Changes
- Success envelopes keep `schema_version: 1`
- Command payload schemas live under `docs/schemas/*.schema.json`
- Object schemas set `additionalProperties: false` except free-form bags (`planned_params`, embedded `schema` document)
- New or newly required data fields agents must accept:
  - `cache_hit` (boolean) on search-crates, readme, get-item, search-in-crate
  - `item_name` (string) on get-item
  - `match_mode` (string) on search-in-crate
  - optional `resolved_version` on readme and get-item
  - optional `hits[].score` on search-in-crate
  - optional `item_type` echo on search-in-crate when filtered
- `meta.next_page` / `meta.prev_page` remain the source for `--page-token`
- `source_url` remains the provenance field on fetched document payloads
- Before: agents often scraped HTML without a schema index
- After: agents load `docs/schemas/README.md` and per-command schema files
- Before/after for match default and dry-run `crate` → `crate_name` are under the Breaking sections above

## Compatibility Notes
- No daemon migration is required because no daemon exists
- Path sandbox keys keep the `DOCSRS_CLI_` prefix; product knobs do not use env
- Hard ceilings for body and output bytes remain product policy
- Host allowlist remains crates.io, docs.rs, static.docs.rs, and doc.rust-lang.org
- Retry kill switch is `--disable-retry`, TOML `disable_retry`, or `max_retries=0` (no env kill switch)

## Rollback
- Install a previous binary only if you still have that artifact
- Clear incompatible local experiments with `docsrs-cli cache clear --json`
- Keep `DOCSRS_CLI_HOME` sandboxes so rollback does not touch production XDG data
- Restore any scripts that depended on 0.1 substring match or product env knobs

## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [AGENTS.md](AGENTS.md)
- [HOW_TO_USE.md](HOW_TO_USE.md)
- [schemas/README.md](schemas/README.md)
