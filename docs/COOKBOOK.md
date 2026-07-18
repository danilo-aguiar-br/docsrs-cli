[Português (pt-BR)](COOKBOOK.pt-BR.md)

# Cookbook
> Copy a recipe, run the command, read the JSON.

## Latency Note
- Cold network fetches depend on crates.io and docs.rs latency
- Warm cache hits stay local inside TTL and report `cache_hit: true`
- Use `--dry-run` when you only need planned URLs

## Default Values Reference
- Timeout wall-clock defaults to a product-safe second budget via config
- Cache TTL defaults to 86400 seconds
- Cache soft budget defaults to 256 MiB
- Max body defaults to 10 MiB hard ceiling
- Max output defaults to 2 MiB hard ceiling
- `search-in-crate --match` defaults to `prefix`
- `search-in-crate --limit` defaults to 100 and clamps at 1000
- JSON auto-selects on non-TTY stdout (except raw `completions`)

## How To Search for a Crate
- Problem: find crates matching a keyword
```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
```

## How To Paginate With page-token
- Problem: walk crates.io results without hand-building query strings
```bash
docsrs-cli search-crates async --page 1 --per-page 20 --json
# read data.meta.next_page into NEXT, then:
docsrs-cli search-crates --page-token "$NEXT" --json
# echoed query/page/per_page/sort match the effective URL (not stale argv)
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
```

## How To Fetch a Crate Overview
- Problem: read the docs.rs overview for a crate
```bash
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json
docsrs-cli readme tokio --crate-version latest --json
# resolved_version is the SemVer of the target crate only, never a dependency
```

## How To Fetch stdlib Overview
- Problem: get std/core/alloc channel docs without guessing HTML
```bash
docsrs-cli readme std --json
docsrs-cli readme core --json
docsrs-cli readme alloc --json
# resolved_version is the channel name such as stable when known
```

## How To Observe Limit Clamp Offline
- Problem: prove search-in-crate --limit hard clamp without network
```bash
docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json
# planned_params.limit is 1000 (hard clamp, including dry-run)
```

## How To Fetch a Typed Item
- Problem: pull documentation for one struct, trait, or function
```bash
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio struct runtime::Runtime --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item async-trait attribute async_trait --json
```

## How To Resolve Associated Methods
- Problem: open method docs such as `Runtime::new` without guessing HTML
```bash
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio fn runtime::Runtime::new --json
# parent type page + #method.new; payload includes item_name, optional resolved_version
# data.extraction is method when markdown is scoped to the method body
```

## How To Confirm Rustdoc Chrome Scrub
- Problem: markdown must not carry rustdoc UI chrome into agent context
```bash
docsrs-cli readme serde --json
# data.markdown has no § section markers and no "Copy item path" UI strings
```

## How To Pass Hyphenated item_path Segments
- Problem: crates.io style names use hyphens but rustc paths use underscores
```bash
docsrs-cli --dry-run get-item async-trait attribute async-trait --json
# planned URL uses async_trait; live get-item accepts hyphen input the same way
docsrs-cli get-item async-trait attribute async-trait --json
```

## How To Suggest Nearby Symbols on 404
- Problem: recover from a wrong item path without scraping all.html yourself
```bash
docsrs-cli get-item serde struct Serde --suggest --json
docsrs-cli get-item tokio struct RuntimeX --suggest --json
# suggestions rank exact → prefix → substring → edit-distance (one all.html fetch)
# typos like Parserx can surface Parser (trait) in the error message
```

## How To Handle Body Budget Without Retry Storms
- Problem: a response exceeds `--max-body-bytes` and agents must not spin
```bash
docsrs-cli --max-body-bytes 50 readme serde --json
# exit 74, error.kind=budget, error.retryable=false
# raise --max-body-bytes (within hard ceiling) instead of retrying
```

## How To Fail Closed on Explicit Zero Timeouts
- Problem: prove timeout 0 is rejected instead of hanging forever
```bash
docsrs-cli --timeout 0 version --json
docsrs-cli --connect-timeout 0 doctor --json
# both exit 65 invalid_input
```

## How To Search Symbols Inside One Crate
- Problem: locate symbols without browsing HTML
```bash
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```

## How To Choose Match Modes
- Problem: tighten or relax symbol search ranking
```bash
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate serde Ser --match prefix --json
docsrs-cli search-in-crate serde de --match substring --limit 20 --json
# default is prefix; use substring for legacy contains behavior
```

## How To Detect cache_hit
- Problem: know whether a payload came from local disk cache
```bash
docsrs-cli readme serde --json
docsrs-cli readme serde --json
# second call should often show data.cache_hit true inside TTL
docsrs-cli --no-cache readme serde --json
# forced network path reports cache_hit false
docsrs-cli cache stats --json
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
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
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
# raw shell by default even on non-TTY; JSON only when explicit:
docsrs-cli completions bash --json
```

## How To Work Offline or Without Network Side Effects
- Problem: plan URLs without opening sockets
```bash
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
# planned_params use crate_name (not crate)
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
docsrs-cli doctor --online --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --online --json
# treat healthy only when process exit is 0 and both top-level ok and data.ok are true
```

## How To Cover Every Top-Level Command Once
- Problem: smoke the full command surface in one checklist
```bash
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli completions bash >/dev/null
docsrs-cli cache stats --json
docsrs-cli config path --json
# optional human live smoke (not CI):
# ./scripts/smoke-live.sh
```
