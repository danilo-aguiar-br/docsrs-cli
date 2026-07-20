[Português (pt-BR)](MIGRATION.pt-BR.md)

# Migration

## 1.2.0 breaking (from 1.1.x) — Camada Y

- Method missing `#method.X` is **`not_found` (exit 66)** — no longer a success with parent-page markdown and `extraction=item_page`
- Successful method fetch always sets `data.extraction` to **`method` only**
- Agents MUST reject method success when `extraction` is missing or is legacy `item_page`
- `--suggest` on method 404 ranks **method leaves from the parent type page** (not only all.html symbols)
- Error envelopes include top-level **`command`** and **`duration_ms`** (parity with success; see `error.schema.json`)
- Budget flags **above hard max** fail closed with **exit 65** (no silent clamp to 10 MiB body / 2 MiB output)
- Method 404 `source_url` keeps the **first** probe kind (`struct`), not the last (`union`)
- Dry-run documents `validation=url_shape_only` and parent kind probes for methods
- Offline schemas: **19** wire names matching `schema --cmd all` (includes cache/config aliases)
- Confirm `docsrs-cli version --json` reports `1.2.0`
- Dogfood with `./target/release/docsrs-cli` after rebuild; PATH install may lag until `cargo install --path . --force`

## 1.1.2 residual fixes (from 1.1.1)

- `get-item` short associated methods (`Type::method`) resolve parent type via all.html when the root-level page 404s
- `crate@version` sugar is accepted on `readme`, `get-item`, and `search-in-crate` (same validation as `--crate-version`; conflicting values → exit 65)

## 1.1.1 breaking (from 0.1.x)

- Clap argv failures: exit **64** + JSON `error.kind=usage` when `--json` or non-TTY (was bare exit 2).
- `--page 0` / `--per-page` outside 1..=100: exit **65** (was silent clamp).
- Cache bodies honor `--max-body-bytes` (was bypass on hit).
- JSON `search-crates` / `search-in-crate` honor `--max-output-bytes` via reduced hits + `truncated`.
- `get-item` may set `resolved_item_path` when resolving reexports.
- Module rename: `diagnostics` (was `telemetry`); still no product telemetry.

## What Changes
- Public product line is `1.2.x` (this release is `1.2.0`)
- Dual license remains MIT OR Apache-2.0
- Documentation framework stays bilingual with skills under `skills/`
- Command surface for agents remains one-shot JSON on stdout
- Product knobs no longer come from `DOCSRS_CLI_*` environment variables
- Path sandbox env vars were **removed** in 1.1.3; use `--config-dir` / `--cache-dir` only

## Migrating from 1.1.x → 1.2.0
- Install or upgrade with `cargo install docsrs-cli --locked --force`
- Run `docsrs-cli version --json` and confirm `data.version` is `1.2.0`
- Re-read the **1.2.0 breaking** section above (Camada Y) before rewiring agents
- Agent checklist for 1.2.0:
  - Reject method success when `extraction` is missing or is legacy `item_page`
  - Missing method anchors are `not_found` (exit 66), never parent-page markdown success
  - Budget above hard max fails closed with exit **65** (not a silent clamp)
  - Failure envelopes expose top-level `command` and `duration_ms` (parity with success)
  - `--suggest` on method 404 ranks method leaves from the parent type page
- Re-read `docsrs-cli schema --cmd get-item|error|dry-run --json` for `extraction`, error envelope, and dry-run validation fields
- Optional: `schema --cmd all --json` for the full 19-name wire bundle

## Historical notes (1.1.x contracts still relevant on 1.1.x/1.2.x)
- These contracts landed on the 1.1 line and remain true on current `1.2.0` trees (they are not a path *to* 0.1.2)
- Re-read `docsrs-cli schema --cmd get-item|error --json` for optional `extraction` history and `error.kind=budget`
- Agent pagination loops must trust `data.query` / `data.page` / `data.per_page` after `--page-token` (echo matches the effective URL)
- Treat body-cap failures as permanent for the same config (`kind=budget`, `retryable=false`, exit 74)
- `doctor` top-level `ok` now mirrors `data.ok` (exit 78 when checks fail — do not treat envelope success as healthy)
- Prefer `data.source_url` when present; top-level `source_url` remains a mirror
- Explicit `--timeout 0` / `--connect-timeout 0` fail closed with exit 65 (`invalid_input`), not a silent hang
- Hyphenated `item_path` segments normalize to underscores (`async-trait` → `async_trait`)
- `--suggest` ranks exact → prefix → substring → edit-distance in one `all.html` fetch
- Rustdoc chrome (`§`, "Copy item path") is scrubbed from markdown
- Optional human smoke: `scripts/smoke-live.sh` (not CI)

## Migrating from 0.1.x → 1.1.x (historical)
- Historical path through the 1.1 contracts; current public product is `1.2.0` — after a full upgrade confirm `data.version` is `1.2.0` (not 0.1.2)
- Install or upgrade with `cargo install docsrs-cli --locked --force`
- Re-read `docsrs-cli commands --json` if your agent cached an older argv tree
- Re-read `docsrs-cli schema --cmd <name> --json` before parsing new required fields
- Point skills and docs links at the current layout and contracts
- Apply all 1.1.x breaking changes below, then the **1.2.0 breaking** section and agent checklist above

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
- After (1.1.x+ default `prefix` + required fields):
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
  - Paths: `--config-dir` / `--cache-dir` (or XDG)
  - Locale: `--lang` / TOML `lang` only (no product env)

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
- After (1.1.x+ canonical wire field):
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
- Default User-Agent is `docsrs-cli/<version> (+https://github.com/danilo-aguiar-br/docsrs-cli)` matching the binary
- JSON envelopes stay English; human stderr may use `--lang` / `--lang`
- Error envelopes expose `error.code`, `error.kind`, `error.message`, `error.retryable`, optional `retry_after_secs`
- Live schema list now includes `schema`, `completions`, `error`, and `dry-run` via `schema --cmd`
- 1.2.0: method success requires `extraction=method`; missing anchors are `not_found`. Error kind `budget` (exit 74, not retryable) remains; hard-max overshoot is exit 65

## Step-by-Step Migration
- Upgrade the binary and confirm version `1.2.0`
- Move any former `DOCSRS_CLI_*` product env settings into flags or `config.toml`
- Keep path isolation with `--config-dir` / `--cache-dir` as needed
- Replace bare `search-in-crate` substring assumptions with `--match substring` when required
- Update dry-run parsers to read `planned_params.crate_name`
- Stop expecting auto-JSON from `completions` without `--json`
- Teach agents about `--page-token` echo fields, `--suggest`, `doctor` ok semantics, and `kind=budget`
- Apply the 1.2.0 agent checklist (reject `item_page`, hard max exit 65, error `command`/`duration_ms`)
- Run `docsrs-cli doctor --json` (and optionally `doctor --online --json`) after path/config moves
- Optional: run `scripts/smoke-live.sh` against live hosts before agent rollout

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
  - method success `extraction=method` only since 1.2.0 (legacy `item_page` success removed)
  - `budget` as a non-retryable `error.kind` (exit 74) since the 1.1 line (historically noted on 0.1.2-era docs)
- `meta.next_page` / `meta.prev_page` remain the source for `--page-token`
- `source_url` remains the provenance field on fetched document payloads
- Before: agents often scraped HTML without a schema index
- After: agents load `docs/schemas/README.md` and per-command schema files
- Before/after for match default and dry-run `crate` → `crate_name` are under the Breaking sections above

## Compatibility Notes
- No daemon migration is required because no daemon exists
- Product never reads `DOCSRS_CLI_*` env (paths, lang, or knobs)
- Hard ceilings for body and output bytes remain product policy
- Host allowlist remains crates.io, docs.rs, static.docs.rs, and doc.rust-lang.org
- Retry kill switch is `--disable-retry`, TOML `disable_retry`, or `max_retries=0` (no env kill switch)

## Rollback
- Install a previous binary only if you still have that artifact
- Clear incompatible local experiments with `docsrs-cli cache clear --json`
- Keep dedicated `--config-dir`/`--cache-dir` sandboxes so rollback does not touch production XDG data
- Restore any scripts that depended on 0.1 substring match or product env knobs

## See Also
- [CHANGELOG.md](../CHANGELOG.md)
- [AGENTS.md](AGENTS.md)
- [HOW_TO_USE.md](HOW_TO_USE.md)
- [schemas/README.md](schemas/README.md)
