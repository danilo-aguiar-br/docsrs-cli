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
- Max body defaults to 10 MiB hard ceiling (values above hard max fail closed with exit 65)
- Max output defaults to 2 MiB hard ceiling (values above hard max fail closed with exit 65)
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
# data.extraction is method on success; missing #method.X is not_found (exit 66), never item_page success
```

## How To Open a Trait Associated Item
- Problem: `Iterator::Item` and `Duration::MAX` have no page of their own
```bash
docsrs-cli get-item std type iter::Iterator::Item --json
docsrs-cli get-item std const time::Duration::MAX --json
docsrs-cli get-item std method iter::Iterator::next --json
# rustdoc emits one anchor prefix per member category, all on the parent page:
#   method.NAME · tymethod.NAME · associatedtype.NAME · associatedconstant.NAME
# source_url echoes the anchor that exists, not the one planned
docsrs-cli get-item std const u32::MAX --json
# lowercase parent (a primitive or module) stays a free item on its own page
docsrs-cli get-item std type iter::Iterator::item --suggest --json
# only the PARENT's case picks the route; a leaf typo still reaches the parent
# page and fails not_found (exit 66) with the real member names, never a 404 on
# an invented path
```

## How To Open an Enum Variant or a Struct Field
- Problem: `Option::Some` and `Range::start` exist only as anchors on the parent
```bash
docsrs-cli get-item std variant option::Option::Some --json
docsrs-cli get-item std variant result::Result::Ok --json
docsrs-cli get-item std structfield ops::Range::start --json
# field is an alias of structfield; the wire item_type echo is always structfield
docsrs-cli get-item std field ops::Range::end --json
# both kinds REQUIRE a Parent::member path — rustdoc serves no variant.X.html
docsrs-cli get-item std variant Some --json
# exit 65, invalid_input: names the parent kinds that can host the member
docsrs-cli get-item std variant option::Option::some --suggest --json
# exit 66 with the real variant names, labelled (variant) so they are copy-paste ready
```

## How To Steer Diagnostics Without an Environment Variable
- Problem: the product reads no product env var, so `RUST_LOG` has no effect
```bash
docsrs-cli -v version              # per-invocation verbosity
docsrs-cli config init             # then set log_directive in config.toml
# log_directive = "docsrs_cli=debug,docsrs_cli::http=trace"
docsrs-cli config show --json | jaq -r '.data.log_directive // "unset"'
# the key is echoed only once it is set: config show omits it while it is unset,
# so a bare read of the field on a fresh config answers null, not the default
docsrs-cli -q version              # an explicit flag always outranks the file
# NO_COLOR / TERM / CLICOLOR_FORCE still apply: they describe the terminal
# device, never product configuration, and --no-color outranks all three
```

## How To Recover From a Method Typo
- Problem: agent typed `Runtime::neww` and must not accept a parent-page false success
```bash
docsrs-cli get-item tokio method Runtime::neww --suggest --json
# exit 66, ok=false, error.kind=not_found; error.suggestions holds the ranked leaves as {path, kind}
# error envelope top-level has command, duration_ms, and nested error
# never treat extraction=item_page as method success (removed in 1.2.0)
# anchor_family carries the real family: variant, structfield, associatedtype, associatedconstant, tymethod, method

# turn the top suggestion straight back into the next command — no text parsing
docsrs-cli get-item tokio method Runtime::neww --suggest --json \
  | jaq -r '.error.suggestions[0] | "docsrs-cli get-item tokio \(.kind) \(.path) --json"'
# the field is absent when the ranking found nothing, so guard with // empty when scripting
```

## How To Plan a Method URL Offline
- Problem: inspect planned parent kind and probes without network
```bash
docsrs-cli --dry-run get-item tokio method runtime::Runtime::neww --json
# planned_params.validation=url_shape_only; planned_parent_kind + parent_kind_probe present
# dry-run does not prove the remote anchor exists
```

## How To Fail Closed on Budget Hard Max Overshoot
- Problem: agent must not accept silent clamp when flags exceed hard max
```bash
docsrs-cli --max-body-bytes 999999999 version --json
docsrs-cli --max-output-bytes 999999999 version --json
# both exit 65 invalid_input (no silent clamp to 10 MiB / 2 MiB)
```

## How To Reduce Payload Without a JSON Post-Processor
- Problem: agent context is expensive and the full envelope is larger than the answer
```bash
docsrs-cli --dry-run --select planned_url readme serde --json
# data carries planned_url only; the cut happens before serialization (no jq/jaq stage needed)
docsrs-cli --dry-run --fields planned_url readme serde --json
# --fields is an alias of --select; CSV and repeated flags both work
docsrs-cli --dry-run --count-only readme serde --json
# data is {"count":1}; count runs after --filter and --dedupe-by
docsrs-cli --dry-run --truncate-content 5 readme serde --json
# strings above 5 characters are shortened; agent_surface.content_truncated is true
docsrs-cli --dry-run --select nope readme serde --json
# data is {}; a missing key is skipped, never emitted as null
docsrs-cli --select name search-in-crate serde Serialize --limit 3 --json
# with a results array, --select projects the ELEMENTS: hits:[{"name":…}]
docsrs-cli --dedupe-by name search-in-crate serde Serialize --limit 5 --json
# agent_surface shows input_count 5 and output_count 4 when one name repeats
```

## How To Take the Top N of a Filtered Set
- Problem: `--filter` narrows the list but the answer is still the first few rows
- Wrong instinct: pipe the envelope through `jaq` — that puts the work back on the agent
```bash
docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name \
  search-in-crate serde "" --limit 200 --json
# --limit 200 bounds the QUERY (how much all.html gets classified)
# --max-items 5 bounds the EMISSION (how much reaches stdout)
# order is fixed: filter, sort-by, dedupe-by, max-items, select
docsrs-cli --sort-by downloads search-crates serde --per-page 20 --json
# numbers compare numerically: 9 sorts before 10, never after it
docsrs-cli --sort-by no_such_key search-crates serde --json
# a key nobody carries is a no-op, not an error; upstream order survives
docsrs-cli --max-items 5 --count-only search-in-crate serde "" --limit 200 --json
# {"count":5}: the count describes the slice, not the filtered set
docsrs-cli --max-items 999 search-in-crate serde "" --limit 20 --json
# agent_surface.limited is false: a small set never looks like a cut one
```

## How To Trust the Counters After a Reduction
- Problem: `emitted` is documented as hits actually emitted, and reduction shrinks the array
```bash
docsrs-cli --filter kind=struct search-in-crate serde "" --limit 200 --json
# data.emitted matches hits length: a field naming the array follows the array
# data.total keeps the upstream count, because it describes docs.rs and not this envelope
docsrs-cli schema --cmd agent-surface --json
# the full agent_surface contract, including limited and the two truncation flags
```

## How To Fail Closed on a Malformed Filter
- Problem: a typo in a filter must never look like an honest empty result
```bash
docsrs-cli --dry-run --filter 'key without operator' readme serde --json
# exit 65, ok=false, error.kind=invalid_input; the message names the expected grammar
docsrs-cli --dry-run --filter '=novalue' readme serde --json
# exit 65 as well; an empty key side is rejected instead of silently matching nothing
docsrs-cli --dry-run --filter 'command=readme' --filter 'ok!=false' readme serde --json
# valid forms are key=value, key!=value, key~substring; repeat the flag for AND
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
docsrs-cli search-in-crate clap Parser --item-type trait --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```
- Filter by the kind the symbol actually is: `Parser` in clap is a trait and a derive macro, never a function
- A filter that matches nothing is still a success: exit `0` with `total` and `emitted` at `0`, not an error
- The echoed `item_type` is the canonical spelling, so `--item-type function` comes back as `fn`

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
docsrs-cli cache path --json
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
docsrs-cli schema --cmd cache-path --json
docsrs-cli schema --cmd cache-clear --json
docsrs-cli schema --cmd cache-stats --json
docsrs-cli schema --cmd config --json
docsrs-cli schema --cmd config-path --json
docsrs-cli schema --cmd config-show --json
docsrs-cli schema --cmd config-init --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd agent-surface --json
docsrs-cli schema --cmd all --json      # all twenty in one call
docsrs-cli version --json
docsrs-cli doctor --json
```
- Ask for the specific name, not the umbrella: `--cmd config-show` answers with the shape `config show` really emits
- The umbrella names `cache` and `config` describe the whole family, so they carry every variant at once

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
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --yes --json
```

## How To Audit Readiness Before a Batch
- Problem: fail closed before many agent turns
```bash
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache config init --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache doctor --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache doctor --online --json
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
docsrs-cli cache path --json
docsrs-cli cache stats --json
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
# fifteen invocable paths: nine top-level plus the six nested cache/config ones
# the two destructive verbs need a designated target, hence --yes above
# optional human live smoke (not CI):
# ./scripts/smoke-live.sh
```
