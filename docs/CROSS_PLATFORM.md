[Português (pt-BR)](CROSS_PLATFORM.pt-BR.md)

# Cross Platform
> One Rust binary, six documented targets, zero platform hacks in argv.

## The Pain You Already Know
- Hardcoded path separators break on Windows
- Native TLS stacks differ by host and surprise agents
- Signal semantics differ between Unix and Windows consoles

## Support Matrix
| Target | Platform | docs.rs build | cross-checked locally |
|--------|----------|---------------|-----------------------|
| `x86_64-unknown-linux-gnu` | Linux glibc | default-target | yes |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | no (ring builds C) | yes (via cargo-zigbuild) |
| `x86_64-pc-windows-gnu` | Windows GNU | no (ring builds C) | yes (via cargo-zigbuild) |
| `x86_64-apple-darwin` | macOS Intel | no (ring builds C) | no (Apple SDK frameworks) |
| `aarch64-apple-darwin` | macOS Apple Silicon | no (ring builds C) | no (Apple SDK frameworks) |
| `x86_64-pc-windows-msvc` | Windows MSVC | no (ring builds C) | yes (via cargo-xwin) |

- The `docs.rs build` column read `yes` for four targets while `Cargo.toml` shipped `targets = []`
- Those four produced permanent 404s, and the manifest was right while the matrix was wrong
- The `docs.rs build` column derives from one cause: `ring` compiles C, and the docs.rs image ships no cross C compiler
- The `local cross-check` column no longer derives from that cause, and saying it did outlived the measurement by one day: `zig` supplies the missing C cross-compiler
- Of the five non-host rows, three now build **with** `ring`: `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-gnu` and `x86_64-pc-windows-msvc`
- The two Apple rows are the remainder, and they stop at the link step rather than at the C compile
- Measured in 2026-08 with a pure-Rust provider swapped in, all four cross-checked clean with no compiler installed
- That measurement is the cost of GAP-TOOLCHAIN-001, and it is why the column says `no` rather than `unsupported`
- `cross-checked locally` comes from `scripts/check-targets.sh`, which fails closed at zero coverage
- The msvc row read `no (no MSVC SDK here)` until 2026-08-10, when the gate stopped probing for `lib.exe` — an archiver that cannot exist on Linux — and started probing for `cargo-xwin`, `clang-cl` and `llvm-lib`, which were already installed. That row was wrong, not the host
- So `#[cfg(windows)]` is type-checked here after all, and on 2026-08-10 `zig` plus `cargo-zigbuild` — both installable under `$HOME` with no root — took `x86_64-pc-windows-gnu` and `aarch64-unknown-linux-gnu` from skipped to built, ring included
- **The Apple rows are not blocked by `ring`, and this document said they were.** zig compiles ring's C for both Apple targets and the build reaches the LINK step, where it fails with `unable to find framework 'CoreFoundation' / 'Security' / 'SystemConfiguration'`
- Those frameworks are pulled by `rustls-platform-verifier` (through `security-framework` and `core-foundation`) and by reqwest's `system-configuration` — and the first is the crate reqwest forces into the graph unconditionally via `rustls-no-provider`, which no code path in this product consults
- The Apple blocker is therefore Apple SDK frameworks at link time, which zig does not redistribute; removing `ring` would not move that row

## Host Tools That Decide Coverage
- Target coverage is a property of the HOST, never of this crate; these tools decide it
- `zig` and `cargo-zigbuild` drive `x86_64-pc-windows-gnu` and `aarch64-unknown-linux-gnu`
- `cargo-xwin`, `clang-cl` and `llvm-lib` drive `x86_64-pc-windows-msvc`
- `oa64-clang` is the probe for both Apple targets, and needs a real Apple SDK behind it
- All of these install under `$HOME` with no root, except the Apple SDK itself
- `cross_checked` counts only the four non-Linux targets, so `aarch64-unknown-linux-gnu` never raises it
- Remove `zig` and `cross_checked` drops from 2 to 1 with no change to this repository, because `zig` covers exactly one counted target
- A gate policy gate derives this list from `cross_tools_for()` so the two cannot drift

## TLS Posture
- The crypto provider is `ring`, which compiles C and therefore contradicts the Rust-native product rule
- Both pure-Rust replacements were re-measured on 2026-08-10 and both stay rejected
- `graviola` requires x86_64 `adx`, `bmi2` and `avx2`, which its own README dates at ~2014 hardware
- `rustls-graviola` 0.4.0 comes from the rustls author, so the barrier is CPU reach, never trust
- `rustls-rustcrypto` has never left `0.0.2-alpha`, and an alpha is not a TLS dependency
- `rustls-webpki` carries RUSTSEC-2026-0098 and RUSTSEC-2026-0104 in the same window
- Trading a build-time dependency for weakened certificate validation is not a trade a TLS client may make
- `docsrs-cli doctor --json` prints the provider and why the pure-Rust path is blocked

## Linux Notes
- Config defaults to `$XDG_CONFIG_HOME/docsrs-cli` or `~/.config/docsrs-cli`
- Cache defaults to `$XDG_CACHE_HOME/docsrs-cli` or `~/.cache/docsrs-cli`
- Private modes prefer `0o700` dirs and `0o600` files
- SIGINT is 130; SIGTERM and SIGHUP are 143

## macOS Notes
- Paths come from the `directories` crate platform layout
- TLS is rustls only (`provider=ring`); no OpenSSL, but `ring` does build C (ADR 0007)
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
- CPU parse offload uses `spawn_blocking` under the concurrency budget
- Hit scans are sequential on every target and pull in no data-parallel crate
- Disk cache behavior is identical across supported platforms
- `cache_hit` semantics are the same on every host class

## Agents Validated per Platform
- Any agent that can exec a binary and parse JSON works on all targets
- Use the packaged skills as the operational source of truth
- Validate offline with `docsrs-cli doctor --json` on each host class you ship
- Validate live connectivity with `docsrs-cli doctor --online --json` on each host class
- Confirm `docsrs-cli version --json` reports `1.3.0` (or newer 1.3.x) after deploy
