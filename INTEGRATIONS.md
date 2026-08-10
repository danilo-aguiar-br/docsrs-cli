[Português (pt-BR)](INTEGRATIONS.pt-BR.md)

# Integrations
> One binary covers agents, shells, and CI without a sticky server.


## Coverage Snapshot
- One-shot subprocess integration for any agent that can exec a binary
- JSON auto on non-TTY stdout for pipes and orchestrators
- Shell completions for bash, zsh, fish, elvish, and PowerShell
- Offline dry-run for URL planning without sockets
- Online doctor probes for crates.io and docs.rs when opted in


## Command Surface
- Data commands: `search-crates`, `readme`, `get-item`, `search-in-crate`
- Discovery commands: `version`, `doctor`, `commands`, `schema`, `completions`
- Storage commands: `cache path`, `cache stats`, `cache clear`
- Configuration commands: `config path`, `config show`, `config init`
- Eleven top-level commands, seventeen invocable paths counting subcommands
- Every flag and every `config.toml` key: [Configuration](docs/CONFIGURATION.md)


## Contract Hardening in 1.3.0
- `--sort-by` and `--max-items` complete the reduction pipeline; ordering runs before limiting
- `agent_surface` exposes `limited`, so a caller can tell a small result from a capped one
- `schema --cmd agent-surface` publishes the reduction report contract
- `error.suggestions` carries the `--suggest` ranking as data; no agent parses `error.message`
- `anchor_family` names the real rustdoc family behind `extraction=method`
- `get-item` reaches `variant` and `structfield`, plus trait associated items and required trait methods
- `config.toml` key `log_directive`; an unparseable value fails at load with exit 78
- `RUST_LOG` is not read; it used to outrank the CLI, which is a product knob in env
- TLS trust anchors are bundled `webpki-roots` again after a `reqwest` upgrade had moved them to the OS store


## Flags Added in 1.1.x (retained)
- `--match exact|prefix|substring` on `search-in-crate` (default `prefix`)
- `--page-token` on `search-crates` for opaque pagination from `meta.next_page`
- `--suggest` on `get-item` to list nearby symbols after a 404
- `doctor --online` for opt-in DNS probes
- Network payloads expose `data.cache_hit`, canonical `crate_name`, and ranked `score`
- Associated methods resolve to parent pages with `#method.name` and `item_name`
- Product knobs use CLI flags and XDG `config.toml` only, not product `DOCSRS_CLI_*` env vars
- Optional `resolved_version` on readme and get-item (stdlib channel is `stable`)

## Contract Hardening in 1.2.0 (Camada Y)
- Method missing `#method.X` is `not_found` (exit 66), never parent-page false success
- Successful method fetch sets `data.extraction` to `method` only
- Agents MUST reject method success if `extraction` is missing or is legacy `item_page`
- `--suggest` on method 404 ranks method leaves from the parent type page
- Error envelopes include top-level `command` and `duration_ms` (parity with success)
- Budget values above hard max fail closed with exit 65 (no silent clamp to 10 MiB / 2 MiB)
- Body over configured `--max-body-bytes` (within hard max) is `error.kind=budget` (exit 74, `retryable=false`)
- Method 404 `source_url` keeps the first probe kind (`struct`), not the last
- Dry-run reports `validation=url_shape_only` and parent kind probes for methods
- Offline `docs/schemas` matches `schema --cmd all` (19 wire names including aliases at that release; `agent-surface` arrived in 1.3.0)

## Contract Hardening retained from 1.1.x
- `--page-token` echoes effective `query` / `page` / `per_page` / `sort` from the planned URL
- `doctor` top-level `ok` mirrors `data.ok` (exit 78 when unhealthy)
- `--suggest` ranks exact → prefix → substring → edit-distance
- Explicit `--timeout 0` / `--connect-timeout 0` fail-closed (exit 65)
- Human smoke script: `scripts/smoke-live.sh`


## Payload Reduction Flags
- Eight global flags cut the JSON envelope before it is written, so no jq stage is needed
- `--select KEYS` projects dotted keys (alias `--fields`); missing keys are skipped, never null
- `--filter EXPR` keeps elements matching `key=value`, `key!=value`, `key~substring`; repeat for AND
- A malformed `--filter` fails closed with exit 65, never an empty set
- `--sort-by KEY` sorts ascending and stable; elements without the key go last
- `--dedupe-by KEY` drops later elements repeating the value
- `--max-items N` bounds the EMISSION; `search-in-crate --limit` bounds the query
- `--count-only` replaces the payload with `{"count": N}`
- `--truncate-content N` shortens strings above N characters, never splitting UTF-8
- `--max-output-bytes N` caps the emitted payload; hard max 2097152 (2 MiB)
- Fixed order: filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes

## Flag Aliases
- `--json` forces the JSON envelope
- `--format json` is an alias of `--json`
- `--format markdown` and `--format text` force the human path
- Completions accept `powershell` and `power-shell`
- Item kind `method` is an alias of `fn` for associated methods


## Completions Contract
- `completions <shell>` always emits a raw shell script by default
- JSON for completions requires an explicit `--json`
- This is an intentional exception to non-TTY auto-JSON


## Summary Table
| Surface | Integration style | Primary contract |
|---------|-------------------|------------------|
| Claude Code / generic LLM agents | subprocess + `--json` | stdout JSON envelope |
| Codex / Cursor / OpenCode | subprocess + pipe | auto-JSON non-TTY |
| Shell humans | TTY default markdown | stderr diagnostics |
| CI / scripts | non-TTY pipe | exit codes + JSON |
| Completions | `completions <shell>` | raw shell scripts |


## Claude Code and Generic Agents
- Invoke as a one-shot subprocess per operation
- Pass `--json` or rely on non-TTY auto-JSON
- Parse `ok`, `command`, `data`, and `duration_ms` on success
- Parse `ok:false`, `command`, `duration_ms`, `error.kind`, and `error.retryable` on failure
- Never retry `kind=budget` (exit 74); raise `--max-body-bytes` only within hard max (above hard max is exit 65)
- Read `data.cache_hit`, `data.crate_name`, `data.item_name`, `data.match_mode`; method success requires `data.extraction=method`
- Treat doctor healthy only when top-level `ok` and `data.ok` are both true
- Start with `commands --json` and `schema --cmd <name> --json`
- Prefer `--match prefix` or `exact` for precise symbol lookup
- Paginate with `data.meta.next_page` into `--page-token` and trust effective-URL echo fields
- Recover 404s with `get-item ... --suggest` (cascade match in the error message)


## Codex Cursor and OpenCode
- Keep the binary on PATH after `cargo install docsrs-cli --locked`
- Prefer quiet mode with `-q` when stderr must stay clean
- Branch on exit codes before trusting stdout
- Use `--dry-run` to validate planned URLs in sandboxes
- Use `doctor --online --json` before large online batches


## Shell Humans
- Default TTY output is Markdown
- Use `--format markdown` to force human output on pipes
- Generate completions with `docsrs-cli completions bash`
- Run `doctor --json` after changing XDG paths
- Run `doctor --online --json` when network readiness matters


## CI and Scripts
- Always pass `--json` for stable parsing
- Treat exit `0` as success and non-zero as failure classes
- Use `--config-dir` / `--cache-dir` for isolated config and cache paths
- Product knobs come from flags or XDG `config.toml`, not product env vars
- Do not enable live network tests unless intentional


## Skill Packages
- English skill: `skills/docsrs-cli-en/SKILL.md`
- Portuguese skill: `skills/docsrs-cli-pt/SKILL.md`
- Skills teach agents exact argv, envelopes, match modes, page tokens, and retry policy
