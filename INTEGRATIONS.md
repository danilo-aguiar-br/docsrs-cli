[Português (pt-BR)](INTEGRATIONS.pt-BR.md)

# Integrations
> One binary covers agents, shells, and CI without a sticky server.


## Coverage Snapshot
- One-shot subprocess integration for any agent that can exec a binary
- JSON auto on non-TTY stdout for pipes and orchestrators
- Shell completions for bash, zsh, fish, elvish, and PowerShell
- Offline dry-run for URL planning without sockets
- Online doctor probes for crates.io and docs.rs when opted in


## Flags Added in 1.1.0 (still current on 0.1.x)
- `--match exact|prefix|substring` on `search-in-crate` (default `prefix`)
- `--page-token` on `search-crates` for opaque pagination from `meta.next_page`
- `--suggest` on `get-item` to list nearby symbols after a 404
- `doctor --online` for opt-in DNS probes
- Network payloads expose `data.cache_hit`, canonical `crate_name`, and ranked `score`
- Associated methods resolve to parent pages with `#method.name` and `item_name`
- Product knobs use CLI flags and XDG `config.toml` only, not product `DOCSRS_CLI_*` env vars
- Optional `resolved_version` on readme and get-item (stdlib channel is `stable`)

## Contract Hardening in 0.1.2
- `--page-token` echoes effective `query` / `page` / `per_page` / `sort` from the planned URL
- Associated-method `get-item` scopes markdown to the method (`data.extraction` = `method`|`item_page`)
- Body over `--max-body-bytes` is `error.kind=budget` (exit 74, `retryable=false`)
- `doctor` top-level `ok` mirrors `data.ok` (exit 78 when unhealthy)
- `--suggest` ranks exact → prefix → substring → edit-distance
- Explicit `--timeout 0` / `--connect-timeout 0` fail-closed (exit 65)
- Human smoke script: `scripts/smoke-live.sh`


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
- Parse `ok:false`, `error.kind`, and `error.retryable` on failure
- Never retry `kind=budget` (exit 74); raise `--max-body-bytes` instead
- Read `data.cache_hit`, `data.crate_name`, `data.item_name`, `data.match_mode`, optional `data.extraction` when present
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
- Set `DOCSRS_CLI_HOME` for isolated config and cache paths only
- Product knobs come from flags or XDG `config.toml`, not product env vars
- Do not enable live network tests unless intentional


## Skill Packages
- English skill: `skills/docsrs-cli-en/SKILL.md`
- Portuguese skill: `skills/docsrs-cli-pt/SKILL.md`
- Skills teach agents exact argv, envelopes, match modes, page tokens, and retry policy
