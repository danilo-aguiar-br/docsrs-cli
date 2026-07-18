[English](CHANGELOG.md)

# Changelog
Todas as mudanças notáveis deste projeto ficam documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]
## [1.1.0] - 2026-07-18
### Changed
- Match padrão de `search-in-crate` é `prefix` (não substring); use `--match substring` para o legado
- Config de produto não lê mais `DOCSRS_CLI_*` (flags + TOML XDG)
- Dry-run `planned_params` usam `crate_name` (não `crate`)
- `completions` emite script cru por padrão mesmo em non-TTY; JSON só com `--json` explícito

### Fixed
- GAP-020: payloads de rede expõem `cache_hit` (cache local em disco; sem telemetria remota)
- GAP-001/019: match modes ranqueados evitam ruído `Serialize` → `Deserializer*`
- GAP-002/021: métodos associados resolvem para `#method.name` na página do tipo pai
- GAP-003: `resolved_version` da stdlib é canal (`stable`), nunca o nome do crate
- GAP-004: scrape SemVer no HTML quando a URL fica em `/latest/` (somente do crate alvo)
- GAP-005: `get-item` emite `item_name`
- GAP-006: `search-in-crate` ecoa `item_type` e `match_mode`
- GAP-007/018: `crate_name` unificado no wire
- GAP-008: dry-run faz clamp de `--limit` em 1000
- GAP-009/023: `--page-token` consome query strings de `meta.next_page` sem query posicional
- GAP-010: completions raw por padrão
- GAP-011/022: UA de contato real + `doctor --online` com probes DNS + check de contato
- GAP-012: remoção da superfície de env de produto em clap/config
- GAP-013: classificação de `index.html` / `#method.` para módulos e métodos
- GAP-014: sanitize + caminho de extração main-content
- GAP-015: `--suggest` em get-item 404 lista símbolos próximos
- GAP-016: cobertura de suite timeout/offline
- GAP-017: payloads JSON permanecem em inglês; stderr humano é i18n
- GAP-024: `join_href` produz URLs absolutas para paths de método

### Added
- `--match exact|prefix|substring`, `score` de hits, `--page-token`, `--suggest`, `doctor --online`
- Campo opcional de score; alias `method` para item type


## [0.1.0] - 2026-07-18
### Added
- CLI one-shot agent-first para busca no crates.io e fetch de item/README no docs.rs
- Envelopes JSON com auto-JSON em stdout non-TTY e caminho Markdown humano
- Cache HTTP em disco XDG com TTL, orçamento, clear/stats
- `commands`, `schema`, `doctor`, `version`, `completions`, `cache`, `config`
- Fetch de stdlib para `std` / `core` / `alloc` via `doc.rust-lang.org`
- Defaults de paralelismo: Tokio multi-thread, `ConcurrencyBudget`, `--max-concurrency`
- `RetryConfig` HTTP explícito com full-jitter e `Retry-After`
- Kill switch `--disable-retry` (e TOML `disable_retry` / `max_retries=0`)
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


[Unreleased]: https://github.com/danilo-aguiar-br/docsrs-cli/compare/v1.1.0...HEAD
[1.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.0
[0.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.0
