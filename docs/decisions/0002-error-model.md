# ADR 0002 — Structured error model for docsrs-cli

## Status

Accepted (2026-07-18)

## Context

docsrs-cli is both a library (`docsrs_cli`) and a one-shot agent CLI. Agents
parse JSON error envelopes and exit codes. Operators need stable SemVer for
`ErrorKind` and technical English messages without secrets. Rules Rust (error
handling) require: typed `Result`, `thiserror` library errors, `source` chains,
no production `.unwrap()`, justified `.expect`, lowercase Display without
trailing period, and structured exit codes.

## Decision

1. **Public `E`:** `AppError` via `thiserror`, never `anyhow` / `String` /
   bare `Box<dyn Error>` as the library result error type.
2. **Kind + payload:** `ErrorKind` (stable snake_case JSON discriminant +
   UNIX-style exit code) paired with a technical `message` string and optional
   `retry_after_secs` / `source`.
3. **SemVer:** `ErrorKind` and `AppError` are `#[non_exhaustive]` so new kinds
   or variants do not force a major bump for external matchers.
4. **Cause chain:** `with_source` stores `Arc<dyn Error + Send + Sync>`; Display
   shows only `message`; callers walk `Error::source` for the root cause.
5. **Clone:** `AppError: Clone` shares the source via `Arc` so retries and
   logging can retain the original error cheaply.
6. **Classification:** `ErrorKind::is_retryable` / `is_permanent` and matching
   methods on `AppError` for agent contracts (aligned with ADR 0001 retry).
7. **Emit path:** every domain failure in the CLI goes through `emit_error`
   (JSON envelope or localized stderr). Config load failures must not use bare
   `?` into a hardcoded exit-70 path.
8. **Panic policy:** only static invariants (hardcoded regex / CSS selectors)
   use `.expect("… valid by construction")`. External I/O, parse, and config
   always return `AppResult`.
9. **Security:** messages and JSON envelopes never include credentials, raw
   response bodies, or cache paths with secrets. `from_http_status` accepts a
   short non-sensitive context label only.

## Consequences

- Agents get stable `error.kind`, `error.code`, `retryable`, and optional
  `retry_after_secs` without scraping free-form text.
- Adding a new `ErrorKind` is a minor SemVer change for external consumers.
- Developers must route early CLI failures through `emit_error`, not `?` to
  `main`'s last-resort handler.
- OOS for this product: Sentry/OTLP, HTTP server status mapping, FFI
  `catch_unwind`, database rollback, circuit breaker (see ADR 0001 for retry
  OOS list).
