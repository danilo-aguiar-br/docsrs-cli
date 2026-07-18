[Português (pt-BR)](CHANGELOG.pt-BR.md)

# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]


## [0.1.0] - 2026-07-18

### Added
- One-shot agent-first CLI for crates.io search and docs.rs item/README fetch
- JSON envelopes with auto-JSON on non-TTY stdout and Markdown human path
- XDG HTTP disk cache with TTL, size budget, clear/stats
- `commands`, `schema`, `doctor`, `version`, `completions`, `cache`, `config`
- stdlib fetch for `std` / `core` / `alloc` via `doc.rust-lang.org`
- Parallelism defaults: multi-thread Tokio, `ConcurrencyBudget`, `--max-concurrency`
- Explicit HTTP `RetryConfig` with full-jitter backoff and `Retry-After`
- Kill switch `--disable-retry` / `DOCSRS_CLI_DISABLE_RETRY`
- `politeness_delay` with per-host floor plus additive jitter
- ADRs for HTTP retry, error model, and web-fetch scope
- Bilingual public documentation framework and agent skills
- Dual license MIT OR Apache-2.0

### Security
- GET-only allowlist hosts, rustls TLS, no cookie jar, no invalid-cert bypass
- No runtime `.env`, no API keys, public HTTP cache only
- Unix owner-only modes for config and cache writes

### Policy
- No product telemetry
- No GitHub Actions / CI workflows in-tree
- MSRV 1.88


[Unreleased]: https://github.com/docsrs-cli/docsrs-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/docsrs-cli/docsrs-cli/releases/tag/v0.1.0
