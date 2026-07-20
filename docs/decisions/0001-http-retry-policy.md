[Português (pt-BR)](0001-http-retry-policy.pt-BR.md)

# ADR 0001 — HTTP retry policy for docsrs-cli
## Status
- Accepted (2026-07-18)

## Context
- The CLI issues one-shot HTTPS GETs against crates.io, docs.rs, static.docs.rs, and doc.rust-lang.org
- Public CDNs and APIs return transient `429` / `5xx` and transport errors
- Agents need automatic recovery without retry storms or retries on permanent client errors
- Rules Rust (retry/backoff) require: explicit named policy, transient-only classification, exponential backoff with jitter, `Retry-After` respect, kill switch, single stack layer, and documentation

## Decision
- Policy type: `docsrs_cli::retry::RetryConfig` built from `Config`
- Default on: retries enabled for product GETs (idempotent)
- Defaults: `max_retries=3`, `retry_base_ms=200` (floor 50ms), `retry_max_delay_ms=30000`
- Dual budget: `max_attempts` **and** `max_elapsed_ms` (default derives from `timeout_secs`; hard cap 300s)
- A planned sleep that would exceed remaining elapsed budget aborts retry (no sleep past budget)
- Kill switch: `--disable-retry` / TOML `disable_retry` / `max_retries=0` only
- Product settings are not read from `DOCSRS_CLI_*` environment variables
- Retry set: `408`, `429`, `500`, `502`, `503`, `504`, and reqwest timeout / connect / request transport errors
- Never retry permanent `4xx` (incl. 401/403/422), parse errors, body cap (`ErrorKind::Budget`, exit 74, `retryable=false`), or cancel
- Exit `74` is shared with retryable `network`; agents must branch on `error.kind` / `error.retryable`, never exit code alone
- Backoff: full jitter `uniform(0..=min(base*2^n, max_delay))` with monotonic `tokio::time::sleep` and cancel checks between slices
- `Retry-After`: delta-seconds **or** HTTP-date via `httpdate` (past dates within 1s skew → zero wait; older past → formula)
- Honor `Retry-After` for `429` and `503` when present (no extra jitter on absolute server hint); otherwise use the formula
- Classification: `RetryClass` (status), `RetryKind` (error API), `ErrorLayer` (transport layer labels)
- Layering: single loop inside `HttpClient::request` only
- No reqwest-middleware and no agent-level blind re-exec of permanent kinds
- Observability: `tracing` span `retry_attempt` (target `docsrs_cli::retry`); JSON `retryable` + `retry_after_secs`; doctor `retry_policy`

## Consequences
- Agents get resilient fetches without custom retry loops for transient errors
- Operators can disable retries during incidents without rebuilding
- HTTP-date `Retry-After` is honored with a tiny `httpdate` dependency
- Circuit breaker / token-bucket retry budget / hedged requests remain out of scope for a one-shot single-GET CLI
- Kill switch documentation stays aligned with flags + TOML config

## Alternatives considered
- reqwest-retry middleware: extra deps; harder cancel integration; rejected
- Default off (opt-in only): worse agent UX for public CDN blips; rejected given GET-only surface and kill switch
- chrono HTTP-date parse: heavier dep; replaced by lightweight `httpdate`
- Env kill switch `DOCSRS_CLI_DISABLE_RETRY`: removed in 1.1.x along with other product env knobs; flags + TOML only
