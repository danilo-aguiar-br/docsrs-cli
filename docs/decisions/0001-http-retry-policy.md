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
- Defaults: `max_retries=3`, `retry_base_ms=200`, `retry_max_delay_ms=30000`
- Kill switch: `--disable-retry` / TOML `disable_retry` / `max_retries=0` only
- Product settings are not read from `DOCSRS_CLI_*` environment variables
- Retry set: `429`, `500`, `502`, `503`, `504`, and reqwest timeout / connect / request transport errors
- Never retry permanent `4xx`, parse errors, body cap, or cancel
- Backoff: full jitter `uniform(0..=min(base*2^n, max_delay))` with monotonic `tokio::time::sleep` and cancel checks between slices
- `Retry-After`: delta-seconds only (no HTTP-date parser dependency)
- Honor `Retry-After` for `429` and `503` when present; otherwise use the formula
- Layering: single loop inside `HttpClient::request` only
- No reqwest-middleware and no agent-level blind re-exec of permanent kinds
- Observability: `tracing` target `docsrs_cli::retry`; JSON envelope fields `retryable` + `retry_after_secs`; doctor check `retry_policy`

## Consequences
- Agents get resilient fetches without custom retry loops for transient errors
- Operators can disable retries during incidents without rebuilding
- HTTP-date `Retry-After` falls back to the formula (documented limitation)
- Circuit breaker / retry budget / hedged requests remain out of scope for a one-shot single-GET CLI
- Kill switch documentation stays aligned with 1.1.x flags + TOML config

## Alternatives considered
- reqwest-retry middleware: extra deps; harder cancel integration; rejected
- Default off (opt-in only): worse agent UX for public CDN blips; rejected given GET-only surface and kill switch
- chrono HTTP-date parse: heavier dep for rare header form; deferred
- Env kill switch `DOCSRS_CLI_DISABLE_RETRY`: removed in 1.1.x along with other product env knobs; flags + TOML only
