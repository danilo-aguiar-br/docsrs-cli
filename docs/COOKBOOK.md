[Português (pt-BR)](COOKBOOK.pt-BR.md)

# Cookbook

> Copy a recipe, run the command, read the JSON.


## Latency Note
- Cold network fetches depend on crates.io and docs.rs latency
- Warm cache hits stay local inside TTL
- Use `--dry-run` when you only need planned URLs


## Default Values Reference
- Timeout wall-clock defaults to a product-safe second budget via config
- Cache TTL defaults to 86400 seconds
- Cache soft budget defaults to 256 MiB
- Max body defaults to 10 MiB hard ceiling
- Max output defaults to 2 MiB hard ceiling
- JSON auto-selects on non-TTY stdout


## How To Search for a Crate
- Problem: find crates matching a keyword
```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
```


## How To Fetch a Crate Overview
- Problem: read the docs.rs overview for a crate
```bash
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json
```


## How To Fetch a Typed Item
- Problem: pull documentation for one struct, trait, or function
```bash
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio struct runtime::Runtime --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item async-trait attribute async_trait --json
```


## How To Search Symbols Inside One Crate
- Problem: locate symbols without browsing HTML
```bash
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```


## How To Discover Agent Surface
- Problem: learn commands and payload shapes programmatically
```bash
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json
docsrs-cli version --json
docsrs-cli doctor --json
```


## How To Generate Shell Completions
- Problem: install completion scripts for your shell
```bash
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json
```


## How To Work Offline or Without Network Side Effects
- Problem: plan URLs without opening sockets
```bash
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
```


## How To Manage Cache and Config
- Problem: inspect storage health and reset local state
```bash
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json
```


## How To Audit Readiness Before a Batch
- Problem: fail closed before many agent turns
```bash
docsrs-cli doctor --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --json
```


## How To Cover Every Top-Level Command Once
- Problem: smoke the full command surface in one checklist
```bash
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli search-in-crate serde Serialize --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli completions bash >/dev/null
docsrs-cli cache stats --json
docsrs-cli config path --json
```
