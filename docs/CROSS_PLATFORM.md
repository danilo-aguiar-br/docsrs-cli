[Português (pt-BR)](CROSS_PLATFORM.pt-BR.md)

# Cross Platform

> One Rust binary, five documented targets, zero platform hacks in argv.


## The Pain You Already Know
- Hardcoded path separators break on Windows
- Native TLS stacks differ by host and surprise agents
- Signal semantics differ between Unix and Windows consoles


## Support Matrix

| Target | Platform | docs.rs build |
|--------|----------|---------------|
| `x86_64-unknown-linux-gnu` | Linux glibc | default-target |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | yes |
| `x86_64-apple-darwin` | macOS Intel | yes |
| `aarch64-apple-darwin` | macOS Apple Silicon | yes |
| `x86_64-pc-windows-msvc` | Windows MSVC | yes |


## Linux Notes
- Config defaults to `$XDG_CONFIG_HOME/docsrs-cli` or `~/.config/docsrs-cli`
- Cache defaults to `$XDG_CACHE_HOME/docsrs-cli` or `~/.cache/docsrs-cli`
- Private modes prefer `0o700` dirs and `0o600` files
- SIGINT is 130; SIGTERM and SIGHUP are 143


## macOS Notes
- Paths come from the `directories` crate platform layout
- TLS is rustls only; no OpenSSL runtime dependency
- Completions work for bash, zsh, and fish commonly used on macOS


## Windows Notes
- Use PowerShell completions via `docsrs-cli completions powershell`
- Ctrl+C maps to exit 130
- Ctrl+Break and console close map to exit 143
- Cache and config inherit parent ACLs instead of Unix modes


## Containers
- Install with `cargo install docsrs-cli --locked` in the image build
- Set `DOCSRS_CLI_HOME` for a writable sandbox volume
- Provide CA roots so rustls can validate public HTTPS
- Prefer non-root users with a writable home or explicit config/cache dirs


## Shell Support
- Completions: bash, zsh, fish, elvish, powershell, power-shell
- Human Markdown is the TTY default
- JSON auto-selects for non-TTY pipes


## File Paths and XDG
- Never hardcode separators; the CLI uses `PathBuf`
- Precedence: CLI flags, env allowlist, `DOCSRS_CLI_HOME`, ProjectDirs
- `config path --json` prints the winning layer for audits


## Performance by Target
- I/O bound network work dominates wall time on all targets
- CPU parse offload uses `spawn_blocking` and optional rayon scans
- Disk cache behavior is identical across supported platforms


## Agents Validated per Platform
- Any agent that can exec a binary and parse JSON works on all targets
- Use the packaged skills as the operational source of truth
- Validate with `docsrs-cli doctor --json` on each host class you ship
