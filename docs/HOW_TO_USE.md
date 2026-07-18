[Português (pt-BR)](HOW_TO_USE.pt-BR.md)

# How to Use docsrs-cli

> Go from install to a real docs fetch in under 60 seconds.


## Prerequisites
- Install Rust 1.88 or newer with rustup
- Ensure outbound HTTPS works to crates.io and docs.rs
- Prefer a terminal with a working PATH after cargo install


## First Command in 60 Seconds
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
```
- Confirm exit code 0 after each command
- Confirm stdout is a JSON object with `"ok":true`


## Core Commands
- Search the registry: `docsrs-cli search-crates tokio --json`
- Paginate and sort: `docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json`
- Fetch crate overview: `docsrs-cli readme tokio --json`
- Pin overview version: `docsrs-cli readme clap --crate-version 4.5.0 --json`
- Fetch a typed item: `docsrs-cli get-item clap trait clap::Parser --json`
- Search symbols in one crate: `docsrs-cli search-in-crate reqwest Client --json`
- List symbols with empty query: `docsrs-cli search-in-crate tokio "" --limit 50 --json`
- Discover the tree: `docsrs-cli commands --json`
- Print a payload schema: `docsrs-cli schema --cmd get-item --json`


## Full Command Surface
- `search-crates` with `--page`, `--per-page`, `--sort`
- `readme` with optional `--crate-version`
- `get-item` with optional `--crate-version`
- `search-in-crate` with optional `--crate-version`, `--item-type`, `--limit`
- `version`, `doctor`, `commands`
- `schema --cmd` for search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config
- `completions` for bash, zsh, fish, elvish, power-shell, powershell
- `cache stats` and `cache clear`
- `config path`, `config show`, `config init`, `config init --force`


## Daemon
- docsrs-cli has no daemon
- Every invocation is BORN, EXECUTE, FINALIZE, DIE
- Do not expect sticky sessions or background workers


## Advanced Patterns
- Plan without network: `docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json`
- Dry-run search: `docsrs-cli --dry-run search-crates serde --json`
- Force human markdown on a pipe: `docsrs-cli --format markdown version`
- Isolate storage: `DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli doctor --json`
- Inspect cache: `docsrs-cli cache stats --json`
- Clear cache: `docsrs-cli cache clear --json`
- Create default config: `docsrs-cli config init --json`
- Overwrite config: `docsrs-cli config init --force --json`
- Generate completions: `docsrs-cli completions bash`
- Other shells: `docsrs-cli completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`


## Configuration
- Prefer flags, then env allowlist, then XDG `config.toml`, then defaults
- Show effective config: `docsrs-cli config show --json`
- Print resolved paths: `docsrs-cli config path --json`
- Contact and User-Agent come from `--user-agent`, `DOCSRS_CLI_USER_AGENT`, or `DOCSRS_CLI_CONTACT`


## Other Subcommands
- `version` prints binary identity
- `doctor` validates TLS, paths, concurrency, and retry policy
- `completions <shell>` emits shell completion scripts
- `config path|show|init` manages XDG config without secrets
- `cache stats|clear` manages the HTTP disk cache


## Integration With AI Agents
- Always prefer `--json` for machine consumers
- Parse exit code before reading stdout
- Read [AGENTS.md](AGENTS.md) and the packaged skills under `skills/`
