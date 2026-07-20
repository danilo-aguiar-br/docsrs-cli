[English](CHANGELOG.md)

# Changelog
Todas as mudanças notáveis deste projeto ficam documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
e este projeto adere ao [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## [Unreleased]

## [1.2.0] — 2026-07-20

### Corrigido
- **Method fail-closed (GAP-W-001 / X-001 / X-006):** âncora `#method.X` ausente retorna `not_found` (exit 66) em vez de sucesso com página pai e `extraction=item_page`.
- **`--suggest` em typos de method (X-002):** ranqueia leaves próximos na página do tipo pai.
- **Envelope de erro (X-004 / W-003):** falhas incluem `command` + `duration_ms`.
- **Budgets (X-005):** valores acima do hard max falham com exit 65 (sem clamp silencioso).
- **URL 404 method (X-010):** primeiro probe (`struct`), não o último (`union`).
- **Dry-run (X-003 / X-007):** `validation=url_shape_only` + probes documentados.
- **page-token (W-007), English retry (W-004), schemas 19 (W-002), junk (X-008).**

### Alterado
- Skills MUST rejeitar method + `item_page` como sucesso (W-009).
- Versão **1.2.0**.

### Documentação
- Superfícies da raiz e de `docs/` sincronizadas com contratos **1.2.0** Camada Y (method fail-closed, envelope de erro com `command`+`duration_ms`, budget acima do hard max fail-closed, `schema --cmd all` com 19 nomes wire)
- Pares bilíngues llms.txt / llms-full.txt / INTEGRATIONS / HOW_TO_USE / AGENTS / MIGRATION / COOKBOOK / TESTING / CROSS_PLATFORM / schemas README atualizados
- Skills reafirmam overshoot de hard-max → exit 65 e MUST rejeitar method + `item_page`
- Camada AB reescrita consolidada de `skills/docsrs-cli-en` e `skills/docsrs-cli-pt`: catálogo completo dos 11 comandos, 19 nomes de schema + `schema --cmd all`, `cache path`, chaves dry-run de method, fluxos hard-max, `--retry-max-elapsed-ms` / `--allow-loopback`, sem narrativa de histórico de versão, description de auto-ativação ≤1024 caracteres
- Camada AC reescrita do bloco `# docsrs-cli` em `CLAUDE.md` para a linha de produto **1.2.0**: catálogo completo dos 11 comandos, 19 schemas + `schema --cmd all`, `cache path`, method fail-closed, exits hard-max, chaves dry-run de method, `--retry-max-elapsed-ms` / `--allow-loopback`, sem envs de produto `DOCSRS_CLI_*`
- Camada AA reauditoria de `docs/`: paridade EN/pt-BR (AGENTS, HOW_TO_USE), MIGRATION header + caminho 1.1→1.2, dry-run schema com keys de method, fence do COOKBOOK + receitas hard-max/dry-run

### Política
- Sem env de produto, sem telemetria, sem GHA.
- Publicado no crates.io como **docsrs-cli 1.2.0** e tagueado no GitHub.
- Dogfood `./target/release/docsrs-cli`. Preferir `cargo audit --no-fetch` se o index travar.

## [1.1.4] — 2026-07-19

### Corrigido
- **Docs / memória de agente:** removido ensino residual de env de path sandbox (`DOCSRS_CLI_HOME` / `CONFIG_DIR` / `CACHE_DIR` / `LANG`) em docs, ADRs, skills e `CLAUDE.md`; isolamento só com `--config-dir` / `--cache-dir`.
- **Cancel cooperativo no scrape:** filtros rayon/sequenciais de `search-in-crate` honram `CancelFlag` (SIGINT/deadline no fan-out CPU).
- **Memória:** filtro sequencial pré-aloca capacidade do `Vec` a partir dos candidatos.

### Alterado
- **SRP:** extraído `src/config/path_source.rs` (`PathSource` + resolve de config).
- **Discovery:** árvore `commands` documenta bundle `schema --cmd all`.

### Política
- Knobs/paths de produto: só CLI + TOML XDG. Env de host (`NO_COLOR`, `RUST_LOG`, proxy) inalterado. Sem telemetria. Sem GHA.

## [1.1.3] - 2026-07-19

### Adicionado
- `schema --cmd all` devolve bundle determinístico de todos os JSON Schemas embutidos
- `cache path` (simetria com `config path`) com `root` + `source` + `no_cache`
- Testes de política: paths de produto só via flags CLI / XDG (sem env de produto)

### Alterado
- **Config XDG-only:** removidas leituras em runtime de `DOCSRS_CLI_LANG`, `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- Tokens `PathSource`: `cli` | `xdg` | `unresolved`
- Locale: `--lang` / TOML → locale SO → `en`
- Testes live de rede com `#[ignore]` (suite default honesta)
- Removida feature clap `env`
- **reqwest 0.12 → 0.13** com `rustls-no-provider` + CryptoProvider **ring** (ADR 0007)
- Higiene Cargo.toml e `deny.toml` (raízes OS via platform-verifier; ban nativo/aws-lc)

### Corrigido
- Doctor não menciona mais allowlist de path env
- Schema `cache` inclui variante `cache-path`

### Segurança
- Binário de produto ignora completamente knobs `DOCSRS_CLI_*`

### Adicionado
- Check doctor `web_fetch_posture` (ADR 0003: cliente de docs one-shot; **robots=PROIBIDO**; scraper+source_url)
- Testes de regressão: join de hit URLs segue o host de `source_url` (stdlib + mock loopback)
- Teste live stdlib: **cada** hit URL permanece em `doc.rust-lang.org` (SCRAPE-R-003)
- Selectors CSS `LazyLock` process-static nos paths HTML rustdoc (SCRAPE-R-004)
- Resolução same-origin de hit URLs (`resolve_hit_url`) com soft-skip de hrefs absolute off-origin (SCRAPE-S-001)
- APIs `*_from_document` compartilhadas pelos workers CPU de fetch (SCRAPE-S-003)
- Feature `brotli` do reqwest + `Accept-Encoding: gzip, br` explícito (SCRAPE-S-004)
- `RAYON_HIT_THRESHOLD` em `config::constants` (SCRAPE-S-005)
- Check doctor `unsafe_posture` (ADR 0009: `forbid(unsafe_code)`, sem FFI de produto, loopback só CLI/XDG)
- CLI `--allow-loopback` e TOML XDG `allow_loopback` para origins de wiremock offline
- `AllowedOrigin::parse_with` / `Config::load_with_options` para política de loopback explícita
- ADR 0009 postura unsafe/FFI (EN + pt-BR)
- Check doctor `error_model` (postura ADR 0002: thiserror `AppError`/`ErrorKind`, sem anyhow público)
- Teste unitário: Display de `with_source` não embute o texto da causa
- ADR 0008 postura de tipos de domínio: só `url` do conjunto chrono/uuid/rust_decimal/url; check doctor `domain_types`
- `AllowedOrigin::to_url()` para re-parse WHATWG fail-closed nos builders
- ADR 0007 postura rustls: `CryptoProvider::install_default` (ring) no binário, pin direto `rustls` ≥0.23.18, doctor com `provider=ring`
- `deny.toml` local bane crates TLS alternativos (`native-tls`, OpenSSL, dual aws-lc)
- Orçamento duplo de retry: `max_elapsed_ms` (CLI `--retry-max-elapsed-ms`, TOML `retry_max_elapsed_ms`; `0` deriva do timeout de parede)
- Parse de `Retry-After` em HTTP-date via `httpdate` (delta-seconds continua preferido quando só dígitos)
- APIs de classificação `RetryKind` / `ErrorLayer`; HTTP `408` tratado como timeout transitório
- Span de tracing `retry_attempt` em cada sleep de retry in-process
- Segurança/config: allowlist de host na carga de origins; leitura limitada de `config.toml`; `validate_user_agent`
- Tetos duros para redirects, timeouts e delay de rate-limit

### Alterado
- Piso do delay base de retry elevado a 50ms (Rules Rust); doctor `retry_policy` reporta `max_elapsed_ms` e suporte a HTTP-date
- Crate de produto proíbe `unsafe` (`#![forbid(unsafe_code)]`); release com `overflow-checks`
- Queries de busca rejeitam caracteres de controle; cache revalida `final_url` na allowlist
- Remoção de `aquamarine` (ciclo de vida em fence de texto no rustdoc de `run`; sem proc-macro)
- `scraper` **0.22 → 0.27** (`selectors` 0.38 + `rustc-hash`; elimina `fxhash` unmaintained)
- Split SRP: `docs_rs/*`, `config/*`, `ops`, `meta_cmds`
- Split SRP: `src/cache/{disk,meta,hex,paths,types}` e `src/http/{client,body,content_type,allowlist,rate_limit,constants}`
- Map crates.io com caps de `name` / `description` / URL / page-token (memória antes do budget de emit)
- ADR 0005: pipeline serde/validação (newtypes de domínio; sem `validator`/`serde_with` ociosos)
- **Sistema de tipos (Camada L / ADR 0006):** split `src/domain/*`; core path com `&CrateName` / `&VersionArg` / `&SearchQuery` / `SortKind`; `AllowedOrigin` em `Config`; `OpCtx` nos handlers; newtypes `#[repr(transparent)]`; wrappers validate-only removidos
- **Tipos de domínio (Camada N / ADR 0008):** builders/fetch/planners de crates.io recebem `&AllowedOrigin` (não `AsRef<str>` nu); wire `source_url` permanece `String`; chrono/uuid/rust_decimal intencionalmente ausentes
- **Tratamento de erros (Camada O / ADR 0002):** `html_to_markdown` preserva `Error::source` (sem `{e}` no Display); mensagens de transporte/CPU em minúsculas; falhas de pretty-print serde viram `Internal` (sem `unwrap_or_default`/`{}` silencioso); acquire do semáforo mantém source; mensagem de allowlist de origin não promove env de harness; `# Errors` em ops/meta/doctor
- **Unsafe/FFI (Camada P / ADR 0009):** allowlist de loopback só CLI/XDG (removido `DOCSRS_CLI_ALLOW_LOCALHOST`); testes de integração sem `unsafe set_var`; único `unsafe` residual de harness é `libc::kill` Unix no e2e de sinal
- **Web fetch / extração (Camada Q / ADR 0003):** hit URLs de `search-in-crate` juntam-se à `source_url` final (corrige links do stdlib reescritos para `docs.rs`); version scrape via path segments do scraper (sem regex de field-extract); um único decode UTF-8 por body HTML; robots.txt permanece **PROIBIDO**
- **Re-auditoria residual scrape (Camada R):** labels de erro HTTP host-agnósticos (`rustdoc …`); skills EN/PT esclarecem extração estruturada da CLI vs regex no agente; re-validação rebuild/dogfood pós-Q
- **Re-auditoria residual scrape (Camada S):** hit URLs same-origin; âncoras de method via selector `[id]` estático; um `Html::parse_document` por body; gzip+br; sanitize early-empty; doctor atualizado

### Corrigido
- **SCRAPE-Q-001:** hit URLs de `search-in-crate std …` permanecem em `doc.rust-lang.org` (antes viravam `https://docs.rs/std/latest/…`)
- **SCRAPE-R-001:** binário release re-validado para hits stdlib batem com o fix de join da Q
- **SCRAPE-S-001:** hrefs absolute off-origin em all.html não viram hits (e não arriscam falhar o search inteiro)

### Segurança
- Defesa em profundidade anti-SSRF: origins, redirects, request e `final_url` do cache usam a mesma allowlist (loopback via `allow_loopback`, nunca env)

- Fail-closed para User-Agent e `config.toml` oversized (guarda de config envenenado)
- Modelo de ameaças STRIDE (ADR 0004); `SECURITY.md` suporta `1.1.x`
- Stack rustls-only explícita (ADR 0007): bootstrap provider ring, webpki-roots, TLS ≥1.2, sem bypass de cert; `ErrorLayer::Tls` quando rustls está na cadeia de source
- Regex via `RegexBuilder` com `size_limit` / `dfa_size_limit` explícitos
- Queries rejeitam caracteres invisíveis/bidi (não só C0/C1)
- `config.toml` com `deny_unknown_fields`; chaves de cache só hex SHA-256
- `Content-Type` presente porém errado falha fechado em GETs JSON/HTML
- Validação de `contact` opcional (ASCII visível) antes de embutir no User-Agent
- Componentização de `lib`: módulos `doctor`, `suggest` e `output` (SRP)
- Meta de cache: `deny_unknown_fields` + shape hex SHA-256 dos digests antes do I/O do body
- Caps de campos no map JSON crates.io (`MAX_CRATE_DESCRIPTION_CHARS`, `MAX_URL_FIELD_CHARS`)

## [1.1.2] - 2026-07-18

### Corrigido
- **R1** Paths curtos de método associado (`Runtime::new`) resolvem o tipo pai via all.html.
- **R2** Açúcar `nome@versão` em `readme`, `get-item` e `search-in-crate`.

### Alterado
- Versão de produto **1.1.2**

## [1.1.1] - 2026-07-18
### Corrigido
- BUG-001..003, GAP-004..012, WARN-013/014 da auditoria 0.1.2 (budget cache, max-output JSON, reexport, clap 64, validação page, module filter, method echo, docs fallback, diagnostics)
### Alterado
- Versão de produto **1.1.1**

## [0.1.2] - 2026-07-18
### Fixed
- GAP-001: eco de `search-crates --page-token` a partir da URL efetiva (e dry-run `planned_params` batem)
- GAP-002: extração de método por âncora rustdoc (não página inteira do tipo); campo opcional `extraction` (`method`|`item_page`)
- GAP-003: body acima do teto → `kind=budget`, exit 74, `retryable=false`
- GAP-004: `doctor` com `ok` no topo = `data.ok`
- GAP-005: `--suggest` multi-modo + distância de edição (um fetch de all.html)
- GAP-006: scrub de chrome rustdoc (`§`, Copy item path)
- GAP-007: hífen em `item_path` normaliza para `_`
- GAP-012: árvore formatada com `cargo fmt`
- GAP-013: comentários rustdoc deduplicados em helpers de method/search
- GAP-015: `--timeout 0` / `--connect-timeout 0` explícitos fail-closed com exit 65

### Changed
- Versão de produto **0.1.2**; PRD/docs alinhados
- Docs bilíngues sincronizados com a linha 0.1 (README, HOW_TO_USE, MIGRATION, SECURITY, INTEGRATIONS, CROSS_PLATFORM, TESTING, schemas README, AGENTS, COOKBOOK, skills, `llms*.txt`)

### Added
- `scripts/smoke-live.sh` (ritual humano pré-release, sem CI)
- Fixture offline `tests/fixtures/docs_rs/method_runtime_new.html` para golden de extração de método

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


[Unreleased]: https://github.com/danilo-aguiar-br/docsrs-cli/compare/v1.2.0...HEAD
[1.2.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.2.0
[1.1.4]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.4
[1.1.3]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.3
[1.1.2]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.2
[1.1.1]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.1
[0.1.2]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.2
[1.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v1.1.0
[0.1.0]: https://github.com/danilo-aguiar-br/docsrs-cli/releases/tag/v0.1.0
