[Português (pt-BR)](0002-error-model.pt-BR.md)

# ADR 0002 — Structured error model for docsrs-cli
## Status
- Accepted (2026-07-18)

## Context
- docsrs-cli is both a library (`docsrs_cli`) and a one-shot agent CLI
- Agents parse JSON error envelopes and exit codes
- Operators need stable SemVer for `ErrorKind` and technical English messages without secrets
- Rules Rust (error handling) require: typed `Result`, `thiserror` library errors, `source` chains, no production `.unwrap()`, justified `.expect`, lowercase Display without trailing period, and structured exit codes

## Decision
- Public `E`: `AppError` via `thiserror`, never `anyhow` / `String` / bare `Box<dyn Error>` as the library result error type
- Kind + payload: `ErrorKind` (stable snake_case JSON discriminant + UNIX-style exit code) paired with a technical `message` string and optional `retry_after_secs` / `source`
- SemVer: `ErrorKind` and `AppError` are `#[non_exhaustive]` so new kinds or variants do not force a major bump for external matchers
- Cause chain: `with_source` stores `Arc<dyn Error + Send + Sync>`; Display shows only `message`; callers walk `Error::source` for the root cause
- **Never** embed the cause text in `message` via `format!("…{e}")` + `AppError::new` — that duplicates Display and leaves `source()` empty (Camada O / ERR-O-001)
- Domain Display messages: technical English, **lowercase start**, no trailing period (acronyms inside the sentence are fine, e.g. `http`, `html`)
- **Usage exception (ERR-O-008):** `ErrorKind::Usage` JSON envelopes **may** embed multi-line clap help as the agent-facing `message` payload. That text is not subject to the short-domain Display style; domain failures still use short lowercase messages
- Clone: `AppError: Clone` shares the source via `Arc` so retries and logging can retain the original error cheaply
- Classification: `ErrorKind::is_retryable` / `is_permanent` and matching methods on `AppError` for agent contracts (aligned with ADR 0001 retry)
- The two do **not** always agree, and `ErrorKind::Io` is why: the kind alone cannot separate a full disk from a permission denial
- `ErrorKind::Io.retryable()` answers `false` conservatively, while `AppError::retryable` reads the cause and may answer `true`
- The wire field always comes from `AppError`, so the conservative kind-level answer never reaches an envelope
- Exit 74 carries three kinds, not two: `Network` (retryable), `Budget` (permanent for the same config), and `Io` (retryable only when the cause is transient)
- `Io` is therefore the one kind whose retryability is not a function of the kind, so branching on `kind` alone is wrong for it — read `error.retryable`
- Local body/output caps use `ErrorKind::Budget` (exit 74, permanent for the same config); transport failures keep `ErrorKind::Network` (exit 74, retryable)
- Emit path: every domain failure in the CLI goes through `emit_error` (JSON envelope or localized stderr)
- Wire JSON error envelope (1.2.0+): top-level `schema_version`, `ok:false`, **`command`**, **`duration_ms`**, and nested `error` (`code`, `kind`, `message`, `retryable`, optional `retry_after_secs`, optional `suggestions`) — parity with success envelopes for agent correlation
- `suggestions` (1.3.0) publishes the `--suggest` ranking as structured data, so an agent recovering from a 404 never parses `message` prose
- Config load failures must not use bare `?` into a hardcoded exit-70 path
- Panic policy: only static invariants (hardcoded regex / CSS selectors) use `.expect("… valid by construction")`
- External I/O, parse, and config always return `AppResult`
- Security: messages and JSON envelopes never include credentials, raw response bodies, or cache paths with secrets
- Operator-facing messages must **not** promote test-harness environment variables as product knobs (path product env is forbidden: CLI flags + XDG only since 1.1.3)
- `from_http_status` accepts a short non-sensitive context label only

## Consequences
- Agents get stable top-level `command`/`duration_ms` plus `error.kind`, `error.code`, `retryable`, and optional `retry_after_secs` without scraping free-form text
- Adding a new `ErrorKind` is a minor SemVer change for external consumers
- Developers must route early CLI failures through `emit_error`, not `?` to `main`'s last-resort handler
- OOS for this product: Sentry/OTLP, HTTP server status mapping, FFI `catch_unwind`, database rollback, circuit breaker (see ADR 0001 for retry OOS list)
- Camada O (2026-07-19): fixed html→markdown source chain, Display style on transport/CPU messages, silent serde pretty defaults, doctor `error_model` check
