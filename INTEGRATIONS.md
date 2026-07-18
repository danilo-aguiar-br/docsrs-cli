[Português (pt-BR)](INTEGRATIONS.pt-BR.md)

# Integrations

> One binary covers agents, shells, and CI without a sticky server.


## Coverage Snapshot
- One-shot subprocess integration for any agent that can exec a binary
- JSON auto on non-TTY stdout for pipes and orchestrators
- Shell completions for bash, zsh, fish, elvish, and PowerShell
- Offline dry-run for URL planning without sockets


## Flag Aliases
- `--json` forces the JSON envelope
- `--format json` is an alias of `--json`
- `--format markdown` and `--format text` force the human path
- Completions accept `powershell` and `power-shell`


## Summary Table

| Surface | Integration style | Primary contract |
|---------|-------------------|------------------|
| Claude Code / generic LLM agents | subprocess + `--json` | stdout JSON envelope |
| Codex / Cursor / OpenCode | subprocess + pipe | auto-JSON non-TTY |
| Shell humans | TTY default markdown | stderr diagnostics |
| CI / scripts | non-TTY pipe | exit codes + JSON |
| Completions | `completions <shell>` | shell scripts |


## Claude Code and Generic Agents
- Invoke as a one-shot subprocess per operation
- Pass `--json` or rely on non-TTY auto-JSON
- Parse `ok`, `command`, `data`, and `duration_ms` on success
- Parse `ok:false` and `error.kind` on failure
- Start with `commands --json` and `schema --cmd <name> --json`


## Codex Cursor and OpenCode
- Keep the binary on PATH after `cargo install docsrs-cli --locked`
- Prefer quiet mode with `-q` when stderr must stay clean
- Branch on exit codes before trusting stdout
- Use `--dry-run` to validate planned URLs in sandboxes


## Shell Humans
- Default TTY output is Markdown
- Use `--format markdown` to force human output on pipes
- Generate completions with `docsrs-cli completions bash`
- Run `doctor --json` after changing XDG paths


## CI and Scripts
- Always pass `--json` for stable parsing
- Treat exit `0` as success and non-zero as failure classes
- Set `DOCSRS_CLI_HOME` for isolated config and cache
- Do not enable live network tests unless intentional


## Skill Packages
- English skill: `skills/docsrs-cli-en/SKILL.md`
- Portuguese skill: `skills/docsrs-cli-pt/SKILL.md`
- Skills teach agents exact argv, envelopes, and retry policy
