[English](CHANGELOG.md)

# Changelog

Todas as mudanças notáveis deste projeto ficam documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]


## [0.1.0] - 2026-07-18

### Added
- CLI one-shot agent-first para busca no crates.io e fetch de item/README no docs.rs
- Envelopes JSON com auto-JSON em stdout non-TTY e caminho Markdown humano
- Cache HTTP em disco XDG com TTL, orçamento, clear/stats
- `commands`, `schema`, `doctor`, `version`, `completions`, `cache`, `config`
- Fetch de stdlib para `std` / `core` / `alloc` via `doc.rust-lang.org`
- Defaults de paralelismo: Tokio multi-thread, `ConcurrencyBudget`, `--max-concurrency`
- `RetryConfig` HTTP explícito com full-jitter e `Retry-After`
- Kill switch `--disable-retry` / `DOCSRS_CLI_DISABLE_RETRY`
- `politeness_delay` com piso por host e jitter aditivo
- ADRs de retry HTTP, modelo de erro e escopo de web-fetch
- Framework bilíngue de documentação pública e skills de agente
- Dual license MIT OR Apache-2.0

### Security
- Hosts allowlist GET-only, TLS rustls, sem cookie jar, sem bypass de cert inválido
- Sem `.env` em runtime, sem API keys, cache HTTP público apenas
- Modos owner-only em Unix para escritas de config e cache

### Policy
- Sem telemetria de produto
- Sem workflows GitHub Actions / CI no repositório
- MSRV 1.88


[Unreleased]: https://github.com/docsrs-cli/docsrs-cli/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/docsrs-cli/docsrs-cli/releases/tag/v0.1.0
