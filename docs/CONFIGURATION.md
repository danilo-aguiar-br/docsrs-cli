[Português (pt-BR)](CONFIGURATION.pt-BR.md)

# Configuration

## Why This Document Exists
- Every knob below is settable, and eleven of them were documented nowhere a user reads
- The reduction flags had a gate keeping them in every contract document; the transport knobs had none
- A knob that exists and is never taught is a knob nobody can use

## Precedence
- CLI flags win over `config.toml`
- `config.toml` wins over the compiled defaults
- Product knobs are **never** read from `DOCSRS_CLI_*` environment variables
- `RUST_LOG` is not read: use `-q` / `-v` or the `log_directive` key
- `NO_COLOR`, `TERM`, `CLICOLOR_FORCE` describe the terminal, not the product
- `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` are honored by `reqwest` transport, never as a product knob

## Where Configuration Lives
- `docsrs-cli config path --json` prints the resolved directories and the winning layer
- `docsrs-cli config show --json` prints the effective values after all layers
- `docsrs-cli config init` writes a default `config.toml`; `--force` overwrites
- `config init --force` designates its target in argv: pass `--config-dir <DIR>`, or `--yes` to accept the XDG root
- Without one of the two it exits 64 and writes nothing, naming the file it refused to replace
- The envelope carries `target_source` (`cli` or `xdg`), the same field `cache clear` reports
- The waiver is required even where no file exists yet: you cannot know that about a directory you never named
- `--config-dir <DIR>` overrides the config directory
- `--cache-dir <DIR>` overrides the HTTP cache directory
- Unknown keys in `config.toml` fail closed with exit 78; a typo is never silently ignored

## Output Format
- `--json` emits the JSON envelope on stdout
- `--format json|markdown|text` selects the rendering (`json` is the alias of `--json`)
- JSON is automatic when stdout is not a TTY; force human output with `--format markdown`
- `--lang en|pt-BR` forces the human stderr locale; JSON stays English
- `--no-color` disables ANSI on stderr
- `-v` / `--verbose` increases stderr verbosity, repeatable
- `-q` / `--quiet` suppresses non-error prose

## Payload Reduction
- `--select KEYS` projects dotted keys (alias `--fields`); missing keys are skipped, never null
- `--filter EXPR` keeps matching elements with `key=value`, `key!=value`, `key~substring`; repeat for AND
- A malformed `--filter` fails with exit 65 instead of returning an empty set
- `--sort-by KEY` sorts ascending and stable; elements without the key go last
- `--dedupe-by KEY` drops later elements repeating the value; elements without the key are kept
- `--max-items N` emits at most N elements, counted after `--filter` and `--dedupe-by`
- `--count-only` replaces the payload with `{"count": N}`
- `--truncate-content N` shortens strings above N characters, never splitting UTF-8
- `--max-output-bytes N` caps the emitted payload; hard max `2097152` (2 MiB)
- Fixed order: filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- `--max-items` bounds the emission; `search-in-crate --limit` bounds the query

## Timeouts
- `--timeout <SECS>` is the wall-clock budget; TOML `timeout_secs`
- `--connect-timeout <SECS>` is the connect budget; TOML `connect_timeout_secs`
- Both must be at least 1 when set; `0` fails closed, and the exit code names the layer that carried it
- On the flag it is exit 65 `kind=invalid_input`; in `config.toml` it is exit 78 `kind=config`
- The file failure lands at load, so an inherited `timeout_secs = 0` breaks every command, not only the one you ran
- Defaults are `timeout_secs` 30 and `connect_timeout_secs` 10

## Retry
- `--max-retries N` bounds retries after the first attempt; TOML `max_retries`
- `--retry-base-ms N` is the base backoff delay; TOML `retry_base_ms`
- `--retry-max-delay-ms N` caps a single retry sleep; TOML `retry_max_delay_ms`
- `--retry-max-elapsed-ms N` is the total retry wall budget; `0` derives it from `--timeout`
- `--disable-retry` is the incident kill switch; TOML `disable_retry = true`
- Setting `max_retries = 0` disables retries from the file layer
- Only idempotent GET failures retry: 408, 429, 5xx and transport errors
- Never retry `kind=budget` (exit 74); that failure is local and permanent for the same settings

## Network Politeness
- `--rate-limit-delay-ms N` is the minimum delay between requests to the same host
- TOML `rate_limit_delay_ms`
- The delay carries an additive jitter and holds across processes through a lock and stamp

## Concurrency
- `--max-concurrency N` bounds concurrent CPU parse workers; TOML `max_concurrency`
- `0` means auto, derived from CPU count and free RAM
- This bounds parsing, not sockets: product commands issue one primary GET at a time

## Body and Output Budgets
- `--max-body-bytes N` caps the downloaded body; hard max `10485760` (10 MiB)
- `--max-output-bytes N` caps the emitted payload; hard max `2097152` (2 MiB)
- A value above a hard max fails closed: exit 65 on the flag, exit 78 `kind=config` in `config.toml`
- Those two ceilings are the only ones that refuse; seven other knobs are clamped in silence
- Clamped down without a word: `max_redirects` to 20, `timeout_secs` to 600, `connect_timeout_secs` to 120
- Also clamped: `rate_limit_delay_ms` to 60000, `max_retries` to 10, and `retry_base_ms` raised to a floor of 50
- Finally, `connect_timeout_secs` is lowered to `timeout_secs` whenever it would exceed it
- Read any of them back with `config show --json`, which reports the value that took effect, not the one you wrote
- Other defaults: `max_redirects` 5, `rate_limit_delay_ms` 1000, `max_retries` 3, `retry_base_ms` 200, `retry_max_delay_ms` 30000
- A body budget hit is `kind=budget`, exit 74, and is not retryable

## Disk Cache
- `--no-cache` disables the disk cache and always hits the network; TOML `no_cache`
- `--cache-ttl-secs N` sets the entry TTL; default `86400` (24 h); TOML `cache_ttl_secs`
- `--max-cache-bytes N` is a soft cap; default `268435456` (256 MiB); `0` means unlimited
- `docsrs-cli cache path --json` reports `root`, `source` and `no_cache`
- `docsrs-cli cache stats --json` reports entries, bytes and the budget
- `docsrs-cli cache clear --yes --json` deletes cached bodies and reports what was freed
- `cache clear` designates its target in argv: pass `--cache-dir <DIR>`, or `--yes` to accept the XDG root
- Without one of the two it exits 64 and deletes nothing, naming the directory it refused to empty
- The envelope carries `target_source` (`cli` or `xdg`), so an audit sees which layer chose the path
- Network payloads expose `cache_hit`, which describes the local disk only

## Identity
- `--user-agent <STRING>` overrides the User-Agent; TOML `user_agent`
- TOML `contact` appends a contact string to the default identity
- `contact` must be non-empty printable ASCII with no control characters and bounded length
- An invalid header value fails with `kind=config`

## Origins and Loopback
- TOML `crates_io_origin` and `docs_rs_origin` override the allowed origins
- Neither has a CLI flag; both exist for offline test rigs
- `--allow-loopback` permits `127.0.0.1` and `localhost`; TOML `allow_loopback = true`
- The host allowlist still applies to every target URL

## Diagnostics
- TOML `log_directive` sets the `tracing` filter, for example `docsrs_cli=debug,docsrs_cli::http=trace`
- `-q` and `-v` outrank the key; the compiled floor is `error`
- An unparseable directive is rejected at load with exit 78
- `config show --json` echoes `log_directive` when it is set, and omits it when it is not

## Planning Without Sockets
- `--dry-run` plans URLs and opens no network socket
- The payload carries `planned_url` and `planned_params`; `validation` lives inside `planned_params`, never beside it
- A planned URL is URL shape only; it is never evidence that a live anchor exists

## Complete `config.toml` Keys
- `timeout_secs`, `connect_timeout_secs`
- `max_body_bytes`, `max_output_bytes`, `max_redirects`
- `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `disable_retry`
- `rate_limit_delay_ms`, `max_concurrency`
- `user_agent`, `contact`, `lang`, `log_directive`
- `crates_io_origin`, `docs_rs_origin`, `allow_loopback`
- `cache_ttl_secs`, `max_cache_bytes`, `no_cache`
- Five keys have no CLI flag and are settable from the file layer only
- They are `max_redirects`, `contact`, `log_directive`, `crates_io_origin` and `docs_rs_origin`

## Verify Your Settings
```bash
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli doctor --json
docsrs-cli --config-dir /tmp/rig config show --json
```

## See Also
- [How to use](HOW_TO_USE.md)
- [Agents](AGENTS.md)
- [Cookbook](COOKBOOK.md)
- [JSON schemas](schemas/README.md)
