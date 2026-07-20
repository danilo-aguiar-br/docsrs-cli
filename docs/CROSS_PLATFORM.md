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
- TLS is rustls only (`provider=ring`); no OpenSSL runtime dependency (ADR 0007)
- Completions work for bash, zsh, and fish commonly used on macOS

## Windows Notes
- Use PowerShell completions via `docsrs-cli completions powershell`
- Ctrl+C maps to exit 130
- Ctrl+Break and console close map to exit 143
- Cache and config inherit parent ACLs instead of Unix modes

## Containers
- Install with `cargo install docsrs-cli --locked` in the image build
- Pass `--config-dir` / `--cache-dir` to writable volumes (or rely on XDG/AppData under a writable home)
- Provide CA roots so rustls can validate public HTTPS
- Prefer non-root users with a writable home or explicit config/cache dirs
- Product knobs come from flags and `config.toml` only (never product `DOCSRS_CLI_*` env)

## Shell Support
- Completions: bash, zsh, fish, elvish, powershell, power-shell
- Completions emit raw shell scripts by default even on non-TTY
- Pass `--json` only when you want a JSON envelope for completions
- Human Markdown is the TTY default for other commands
- JSON auto-selects for non-TTY pipes on network and meta commands

## File Paths and XDG
- Never hardcode separators; the CLI uses `PathBuf`
- Path precedence:
  1. CLI flags `--config-dir` / `--cache-dir`
  2. ProjectDirs / platform XDG (Linux), AppData (Windows), Library (macOS)
- Product knobs (timeouts, retries, UA, origins, cache TTL, …) use only flags + TOML
- Product knobs and paths are not read from `DOCSRS_CLI_*` environment variables
- `config path --json` prints the winning layer for audits

## Performance by Target
- I/O bound network work dominates wall time on all targets
- CPU parse offload uses `spawn_blocking` and optional rayon scans
- Disk cache behavior is identical across supported platforms
- `cache_hit` semantics are the same on every host class

## Agents Validated per Platform
- Any agent that can exec a binary and parse JSON works on all targets
- Use the packaged skills as the operational source of truth
- Validate offline with `docsrs-cli doctor --json` on each host class you ship
- Validate live connectivity with `docsrs-cli doctor --online --json` on each host class
- Confirm `docsrs-cli version --json` reports `1.2.0` (or newer 1.2.x) after deploy
