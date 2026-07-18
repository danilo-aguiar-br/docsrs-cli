[Português (pt-BR)](INTEGRATIONS.pt-BR.md)

# Integrations
> One binary covers agents, shells, and CI without a sticky server.


## Coverage Snapshot
- One-shot subprocess integration for any agent that can exec a binary
- JSON auto on non-TTY stdout for pipes and orchestrators
- Shell completions for bash, zsh, fish, elvish, and PowerShell
- Offline dry-run for URL planning without sockets
- Online doctor probes for crates.io and docs.rs when opted in


## Flags Added in 1.1.0
- `--match exact|prefix|substring` on `search-in-crate` (default `prefix`)
- `--page-token` on `search-crates` for opaque pagination from `meta.next_page`
- `--suggest` on `get-item` to list nearby symbols after a 404
- `doctor --online` for opt-in DNS probes
- Network payloads expose `data.cache_hit`, canonical `crate_name`, and ranked `score`
- Associated methods resolve to parent pages with `#method.name` and `item_name`
- Product knobs use CLI flags and XDG `config.toml` only, not product `DOCSRS_CLI_*` env vars
- Optional `resolved_version` on readme and get-item (stdlib channel is `stable`)


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
- Parse `ok:false` and `error.kind` on failure
- Read `data.cache_hit`, `data.crate_name`, `data.item_name`, `data.match_mode` when present
- Start with `commands --json` and `schema --cmd <name> --json`
- Prefer `--match prefix` or `exact` for precise symbol lookup
- Paginate with `data.meta.next_page` into `--page-token`
- Recover 404s with `get-item ... --suggest`


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
