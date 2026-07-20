# gaps.md — Inventário cumulativo (produto + processos + hardcode + rede + retry + segurança)

> **Versão:** **1.1.2** · Binário: `./target/release/docsrs-cli`  
> **Proibido nestas entregas:** publish GitHub/crates.io · CI/CD · telemetria remota  
> **Atualizado:** 2026-07-19 (Camada **S** residual scrape re-audit; A–R preservadas)

## 0. Veredito global

| Check | Resultado |
|-------|-----------|
| `cargo test --locked` | OK |
| `cargo clippy --all-targets --locked -- -D warnings` | OK |
| `cargo build --release --locked` | OK (quando executado) |
| `version --json` | `"version":"1.1.2"` |
| Inventário **aberto** (A–S) | **zero** (DEPS unmaintained documentado; sem CVE) |

### Linha do tempo (incremental)

| Camada | Data | Escopo | Aberto |
|--------|------|--------|--------|
| A — produto 0.1.2 | 2026-07-18 | GAP-001…018 (histórico git) | 0 |
| B — produto 1.1.1→1.1.2 | 2026-07-18 | BUG-001…WARN-014 + R1/R2 | 0 |
| C — processos externos | 2026-07-19 | PROC-001…006 (`r-auditoria`) | 0 |
| D — proibição hardcode | 2026-07-19 | HARD-001…009 (`r-auditoria` + XDG/memória/paralelismo) | 0 |
| E — práticas de rede | 2026-07-19 | NET-001…008 (`r-auditoria` + one-shot/memória/paralelismo) | 0 |
| F — retry com backoff | 2026-07-19 | RETRY-001…008 (`r-auditoria` + one-shot/memória/paralelismo) | 0 |
| G — segurança defensiva | 2026-07-19 | SEC-001…010 (`r-auditoria` + one-shot/memória/paralelismo) | 0 |
| H — segurança desenvolvimento | 2026-07-19 | SECDEV-001…008 (`r-auditoria` + one-shot/memória/paralelismo) | 0 |
| I — meta + SRP + contact | 2026-07-19 | META-001…006 + COMP-001…003 (`r-auditoria` follow-up) | 0 |
| J — componentização SRP/DRY | 2026-07-19 | COMP-001/002b/003 + DRY-J + HYGIENE + PROC-J (`r-auditoria` meta) | 0 |
| K — serde/validação | 2026-07-19 | SERDE-K + COMP-K (`r-auditoria`) | 0 |
| L — sistema de tipos | 2026-07-19 | TYPE-L + ADR 0006 (`r-auditoria`) | 0 |
| M — rustls obrigatório | 2026-07-19 | TLS-M-001…010 + ADR 0007 (`r-auditoria`) | 0 |
| N — tipos de domínio (url + N/A chrono/uuid/decimal) | 2026-07-19 | DOM-N-001…007 + ADR 0008 (`r-auditoria`) | 0 |
| O — tratamento de erros | 2026-07-19 | ERR-O-001…010 + HYG-O + ADR 0002 reforço (`r-auditoria`) | 0 |
| P — unsafe code e FFI | 2026-07-19 | UNSAFE-P-001…008 + HYG-P + ADR 0009 (`r-auditoria`) | 0 |
| Q — web scraping / extraction | 2026-07-19 | SCRAPE-Q-001…009 + HYG-Q + ADR 0003 (`r-auditoria`; **PROIBIDO robots**) | 0 |
| R — residual scrape re-audit | 2026-07-19 | SCRAPE-R-001…005 + HYG-R (`r-auditoria` 3ª passagem; **PROIBIDO robots**) | 0 |
| S — residual scrape re-audit | 2026-07-19 | SCRAPE-S-001…006 + HYG-S (`r-auditoria` 4ª passagem; **PROIBIDO robots**) | 0 |

---

## 1. Camada B — produto v1.1.2 (bugs/gaps + residuais e2e)

> Origem: auditoria inventário 0.1.2 + fechamento R1/R2 (path curto de método, `crate@version`).

### 1.1 Matriz de fechamento

| ID | Status | Evidência |
|----|--------|-----------|
| BUG-001 | **RESOLVED** | Cache hit + `--max-body-bytes 50` → exit 74 `kind=budget` |
| BUG-002 | **RESOLVED** | `search-crates --max-output-bytes` → `truncated=true`, hits reduzidos |
| BUG-003 | **RESOLVED** | `get-item tokio fn tokio::spawn` → `resolved_item_path=task::spawn` |
| GAP-004 | **RESOLVED** | clap parse → exit 64 JSON `usage` |
| GAP-005 | **RESOLVED** | Schemas + PRD kinds budget/canceled; wire `crate_name` |
| GAP-006 | **RESOLVED** | `--page 0` / `--per-page 200` → exit 65 |
| GAP-007 | **RESOLVED** | `--item-type module` → 65 |
| GAP-008 | **RESOLVED** | `method` echo no JSON/title |
| GAP-009 | **RESOLVED** | Knobs flags/TOML; allowlist path/lang |
| GAP-010 | **RESOLVED** | MIT OR Apache-2.0 |
| GAP-011 | **RESOLVED** | Markdown search: `https://docs.rs/{name}` |
| GAP-012 | **RESOLVED** | Testes de regressão (budget, page, clap, method, type path) |
| WARN-013 | **RESOLVED** | `diagnostics` (sem telemetria remota) |
| WARN-014 | **RESOLVED** | budget `retryable=false` |
| **R1** | **RESOLVED** | `get-item tokio method Runtime::new` → 0, `resolved_item_path=runtime::Runtime::new` |
| **R2** | **RESOLVED** | `readme clap@4.5.0 --json` → 0, `resolved_version=4.5.0`; conflito `@` vs flag → 65 |

### 1.2 Residuais e2e (pós-1.1.1) — causa × solução

| Residual | Problema | Causa raiz | Solução |
|----------|----------|------------|---------|
| R1 | `Runtime::new` → URL `union.Runtime.html` 404 | Path curto sem módulo; parent kinds só na raiz; recovery buscava leaf `new` como `fn` | Em 404 de método: all.html no **tipo pai** (`Runtime`); `pick_unique_type_path` → `runtime::Runtime` + método |
| R2 | `clap@4.5.0` rejeitado como nome | `CrateName` não aceitava `@` | `CrateRef::parse` + merge com `--crate-version` |

### 1.3 Comandos e2e de regressão (release local)

```text
docsrs-cli version --json                              → 1.1.2
get-item tokio method Runtime::new --json              → 0 + resolved_item_path=runtime::Runtime::new
get-item tokio fn tokio::spawn --json                  → 0 + task::spawn
readme clap@4.5.0 --json                               → 0 + version 4.5.0
readme clap@4.5.0 --crate-version 4.5.0 --json         → 0 (agree)
readme clap@4.5.0 --crate-version 4.0.0 --json         → 65 conflict
```

---

## 2. Camada C — processos externos (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_processos_externos.md` + GraphRAG `rules-rust-processos-externos`.

### 2.1 Escopo real

| Superfície | Uso de processo | Conformidade |
|------------|-----------------|--------------|
| `src/**` runtime | Sem `Command` / `tokio::process` — só HTTP (`reqwest`) | Preferência nativa **OK** |
| `src/error.rs`, `main.rs`, `lib.rs` | `ExitCode` apenas | OK |
| `src/shutdown.rs` | `std::process::exit(130)` no force path de 2º SIGINT | Aceito: CLI one-shot, flush prévio |
| `tests/cli_smoke.rs`, `network_live.rs` | Spawn do binário sob teste | Helper `tests/common` |
| `tests/signal_term.rs` | Spawn + `libc::kill` (sem CLI `kill`) | Helper + wait com timeout |
| `scripts/smoke-live.sh` | Invoca binário release (bash humano) | Versão pinada → 1.1.2 |

### 2.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **PROC-001** | Alta | `bin()` só setava `stdin=null`; `stdout`/`stderr` implícitos via `.output()` | `tests/common`: Stdio explícito `null/piped/piped` | **RESOLVED** |
| **PROC-002** | Alta | `signal_term` fazia `wait()` sem timeout → hang se filho travar | `wait_with_timeout` + `kill` + re-`wait` (15s) | **RESOLVED** |
| **PROC-003** | Média | Ambiente do filho herdava `LD_PRELOAD` / `DYLD_*` / `RUST_LOG` | `sanitize_child_env` no helper | **RESOLVED** |
| **PROC-004** | Média | Duplicação de política de spawn em 3 arquivos de teste | Módulo único `tests/common/mod.rs` | **RESOLVED** |
| **PROC-005** | Baixa | `scripts/smoke-live.sh` checava `"version":"0.1.2"` (stale) | Pin → `"1.1.2"` | **RESOLVED** |
| **PROC-006** | Info | Documentação de “sem spawn no produto” incompleta | Nota em `src/platform.rs` | **RESOLVED** |

### 2.3 Checklist rules (aplicável)

| Item | Status |
|------|--------|
| Cada `Command` define `stdin`/`stdout`/`stderr` explicitamente | **OK** (`tests/common`) |
| Nenhum arg de concatenação com input externo | **OK** |
| Nenhuma invocação via shell no Rust | **OK** |
| Toolchain ≥ 1.77.2 (CVE-2024-24576) | **OK** (MSRV 1.88) |
| Todo `Child` tem `wait` (ou timeout+kill+wait) | **OK** |
| Código de saída verificado antes de tratar saída | **OK** |
| Produto não usa `Command` onde crate nativa resolve | **OK** (HTTP nativo) |
| Matriz multi-distro/WASM/Flatpak spawn | **N/A** (produto sem spawn) |
| Assinatura/notarização/SBOM release | **Fora de escopo** (sem publish nesta entrega) |

### 2.4 Evidência de correção (camada C)

```text
tests/common/mod.rs          → docsrs_cli_cmd / _silent / wait_with_timeout
tests/cli_smoke.rs           → common::docsrs_cli_cmd()
tests/network_live.rs        → common::docsrs_cli_cmd()
tests/signal_term.rs         → silent + wait_with_timeout(15s)
scripts/smoke-live.sh        → version 1.1.2
src/platform.rs              → out-of-scope: no product Command
```

---

## 3. Camada A — histórico git 0.1.2 (GAP-001…018)

> Snapshot em `origin/main` (`gaps.md` commit `10f76ac`). Fechados na causa raiz na série 0.1.2; não reabertos em 1.1.x.

| ID (0.1.2) | Tema | Status no tree atual |
|------------|------|----------------------|
| GAP-001 | page-token echo | **RESOLVED** (mantido) |
| GAP-002 | method markdown scoped | **RESOLVED** (mantido) |
| GAP-003 | body cap → budget / não retryable | **RESOLVED** (mantido; alinhado BUG-001/WARN-014) |
| GAP-004 | doctor `envelope.ok` | **RESOLVED** (mantido) |
| GAP-005 | suggest cascade typo | **RESOLVED** (mantido) |
| GAP-006 | scrub `§` rustdoc chrome | **RESOLVED** (mantido) |
| GAP-007 | `item_path` hífen → underscore | **RESOLVED** (mantido) |
| GAP-008…018 | PRD/docs/fmt/live smoke/knobs | **RESOLVED** (mantido) |

> Nota de nomenclatura: IDs **GAP-00x** da camada A (0.1.2) **não** colidem semanticamente com **GAP-004…012** da camada B (1.1.2) — séries de auditoria distintas. Preferir o prefixo de camada ao citar.

---

## 4. Camada D — proibição de hardcode + XDG (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_proibicao_hardcode.md` +
> `rules_rust_storage_xdg_cli_rust_sem_env_em_runtime.md` + memória/paralelismo.
> Ferramentas: GraphRAG rules, `context7` (`/servo/rust-url`), `duckduckgo-search-cli`, web search.

### 4.1 Escopo e postura

| Superfície | Situação | Conformidade |
|------------|----------|--------------|
| Segredos / API keys / `.env` runtime | Produto HTTP público; sem keys | **N/A** (sem superfície de segredo) |
| Config central | `Config` tipada + XDG TOML + CLI | **OK** |
| Hosts de produção | `HOST_*` nomeados em `config.rs` | **OK** (após HARD-001…003) |
| Exit codes | `EXIT_*` + `ErrorKind::exit_code` | **OK** (após HARD-007) |
| XDG storage | `directories::ProjectDirs` + `config init/path/show` | **OK** (já existia; reforçado) |
| Memória | `try_reserve*` body/cache; hard ceilings | **OK** (sem gap novo) |
| Paralelismo | `ConcurrencyBudget` + `rayon` threshold nomeado | **OK** (sem gap novo) |

### 4.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **HARD-001** | Alta | URL GitHub literal no User-Agent default e template TOML | `DEFAULT_CONTACT_URL = env!("CARGO_PKG_REPOSITORY")`; UA e `default_config_toml()` usam a const | **RESOLVED** |
| **HARD-002** | Alta | Markdown fallback `https://docs.rs/{name}` literal em `render.rs` | `format!("{SCHEME_HTTPS}://{HOST_DOCS_RS}/…")` | **RESOLVED** |
| **HARD-003** | Alta | `join_href` forçava host `docs.rs` em paths absolutos (`/…`) | `Url::join` no base (stdlib/mock origins corretos) | **RESOLVED** |
| **HARD-004** | Média | `Duration::from_millis(50)` solto no poll cancel/flock | `CANCEL_POLL_INTERVAL_MS` em `config` → `http` | **RESOLVED** |
| **HARD-005** | Média | `Duration::from_secs(2)` progress hint em 4 call sites | `PROGRESS_HINT_DELAY_SECS` | **RESOLVED** |
| **HARD-006** | Média | `"127.0.0.1"` / `"localhost"` literais no allowlist | `HOST_LOOPBACK_IPV4` / `HOST_LOCALHOST` | **RESOLVED** |
| **HARD-007** | Média | Exit codes numéricos (`64`, `78`, `130`, `141`) espalhados | `EXIT_*` em `error.rs`; `lib`/`shutdown` usam consts | **RESOLVED** |
| **HARD-008** | Baixa | Template `config.toml` pinava versão `1.1.2` e URL GitHub | `default_config_toml()` gera a partir de `APP_VERSION` + consts | **RESOLVED** |
| **HARD-009** | Média | Origins TOML sem validação tipada (`url::Url`) | `validate_origin` + `Config::validate_origins` no load | **RESOLVED** |

### 4.3 Checklist rules (aplicável ao produto one-shot)

| Item | Status |
|------|--------|
| Config central tipada, load único, fail-fast | **OK** |
| Sem segredos no fonte / Cargo.toml / logs | **OK** (sem secrets) |
| Hosts/origins nomeados; override via TOML (mocks) | **OK** |
| Scheme HTTPS como const; produção TLS-only allowlist | **OK** |
| Versão só de `CARGO_PKG_VERSION` / `Cargo.toml` | **OK** |
| Contact/repo de `CARGO_PKG_REPOSITORY` | **OK** |
| Paths via XDG / CLI / `DOCSRS_CLI_HOME` (sem `/home/user`) | **OK** |
| Exit codes como constantes semânticas | **OK** |
| Timeouts/polls com unidade no nome da const | **OK** |
| Body/output budgets com hard ceiling | **OK** |
| Concurrency auto (CPU+RAM) + override TOML/CLI | **OK** |
| Sem spawn de processo no produto | **OK** (camada C) |
| Keyring / cloud secrets / SBOM release | **Fora de escopo** (sem secrets; sem publish) |

### 4.4 Evidência de correção (camada D)

```text
src/config.rs     → DEFAULT_CONTACT_URL, SCHEME_HTTPS, HOST_LOOPBACK_*,
                    PROGRESS_HINT_DELAY_SECS, CANCEL_POLL_INTERVAL_MS,
                    default_config_toml(), validate_origin()
src/error.rs      → EXIT_USAGE…EXIT_TERMINATED; ErrorKind::exit_code usa consts
src/http.rs       → poll/allowlist via consts
src/lib.rs        → progress + exit codes via consts
src/shutdown.rs   → process::exit(EXIT_INTERRUPTED)
src/render.rs     → docs fallback HOST_DOCS_RS + SCHEME_HTTPS
src/docs_rs.rs    → join_href via Url::join; origins com SCHEME_HTTPS
src/crates_io.rs  → crates_api_base / origin_prefix com SCHEME_HTTPS
```

### 4.5 Validação

```text
cargo test --locked                              → OK
cargo clippy --all-targets --locked -- -D warnings → OK
config::tests::validate_origin_*                 → OK
config::tests::config_load_rejects_invalid_origin → OK
docs_rs::tests::join_absolute_path_and_fragment  → OK (stdlib origin preservado)
```

---

## 5. Camada E — melhores práticas de rede (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_rede.md` +
> `rules_rust_cli_one_shot.md` + memória + paralelismo.
> Ferramentas: GraphRAG (`neurographrag`), `context7` (`/seanmonstar/reqwest`),
> `duckduckgo-search-cli`, inspeção de features reqwest 0.12.28.

### 5.1 Escopo real (one-shot CLI — não servidor)

| Superfície rules rede | Aplicável? | Conformidade |
|-----------------------|------------|--------------|
| Runtime Tokio multi-thread | Sim | **OK** (após NET-001) |
| Cliente HTTP (reqwest) | Sim | **OK** (após NET-002…006) |
| TLS rustls / min 1.2 | Sim | **OK** (após NET-005) |
| Proxy `HTTP(S)_PROXY` / `NO_PROXY` | Sim | **OK** (após NET-002) |
| Allowlist host + HTTPS (SSRF) | Sim | **OK** (já existia) |
| Retry/backoff + cancel | Sim | **OK** (camadas anteriores) |
| Body stream cap / try_reserve | Sim | **OK** (memória) |
| CPU parse `spawn_blocking` + Semaphore | Sim | **OK** (paralelismo) |
| Servidor TCP/HTTP accept loop | **N/A** | One-shot; sem bind |
| DoH/DoT/DNSSEC custom resolver | **N/A** | Hyper/getaddrinfo; ADR 0003 |
| gRPC / WebSocket / QUIC server | **N/A** | Fora do produto |
| Circuit breaker / bulkhead multi-dep | **N/A** | Uma classe de dependência |
| mTLS / OAuth / JWT | **N/A** | HTTP público sem auth |

### 5.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **NET-001** | Alta | `#[tokio::main]` sem `worker_threads` / `thread_name` / `max_blocking_threads` (função `runtime_worker_threads` existia mas não era usada) | `main.rs`: `Builder::new_multi_thread()` + workers + `max_blocking_threads()` + `thread_name("docsrs-cli-worker")` | **RESOLVED** |
| **NET-002** | Alta | Feature `system-proxy` ausente — docs citavam `HTTP(S)_PROXY`/`NO_PROXY` sem honrar | `Cargo.toml` reqwest: `system-proxy`; template TOML documenta env | **RESOLVED** |
| **NET-003** | Média | Feature `http2` ausente (só HTTP/1.1 via hyper) | `http2` + `http2_adaptive_window(true)` no builder | **RESOLVED** |
| **NET-004** | Média | Pool/socket defaults cegos (`tcp_nodelay`, pool idle, keepalive) | Constantes nomeadas + builder explícito | **RESOLVED** |
| **NET-005** | Média | Sem piso TLS 1.2 no client | `.min_tls_version(TLS_1_2)` (não `https_only` — mocks localhost HTTP) | **RESOLVED** |
| **NET-006** | Baixa | Doctor só dizia `rustls-tls` genérico | Check `http_client_posture` + concurrency com `max_blocking` | **RESOLVED** |
| **NET-007** | Baixa | Tokio sem `default-features = false` documentado | Features explícitas no `Cargo.toml` (sem `full`) | **RESOLVED** |
| **NET-008** | Info | Cap blocking pool Tokio default 512 | `max_blocking_threads() = MAX_EXPLICIT_CONCURRENCY` (256) | **RESOLVED** |

### 5.3 Checklist rules (aplicável ao cliente one-shot)

| Item | Status |
|------|--------|
| Tokio multi-thread, sem misturar runtimes | **OK** |
| Features Tokio mínimas (sem `full`) | **OK** |
| Runtime Builder com workers / blocking / nome | **OK** |
| `#[tokio::main]` só em binário → substituído por Builder | **OK** |
| CPU-bound em `spawn_blocking` + budget | **OK** |
| Sem `Mutex` síncrono através de `.await` no clock HTTP | **OK** |
| Timeouts connect + total + wall-clock | **OK** |
| Retry full-jitter + `Retry-After` + kill switch | **OK** |
| TLS rustls, min 1.2, sem `danger_*` | **OK** |
| HTTP/2 negociável | **OK** |
| System proxy + allowlist de destino | **OK** |
| Body stream capped + `try_reserve*` | **OK** |
| Cancel cooperativo SIGINT/SIGTERM + select biased | **OK** |
| GET-only idempotente (sem Idempotency-Key) | **OK** |
| Servidor / DoH / mTLS / gRPC | **N/A** (one-shot) |

### 5.4 Evidência de correção (camada E)

```text
Cargo.toml        → tokio default-features=false; reqwest +http2 +system-proxy
src/main.rs       → Builder multi_thread workers/max_blocking/thread_name
src/concurrency.rs → max_blocking_threads(); teste de caps
src/http.rs       → POOL_*/TCP_KEEPALIVE_*; min TLS 1.2; tcp_nodelay;
                    pool; http2_adaptive_window; client_posture_detail()
src/lib.rs        → doctor http_client_posture + concurrency max_blocking
src/config.rs     → template documenta HTTP_PROXY/HTTPS_PROXY/NO_PROXY
Cargo.lock        → system-configuration / windows-registry (proxy SO)
```

### 5.5 Validação

```text
cargo test --locked                                  → OK
cargo clippy --all-targets --locked -- -D warnings   → OK
cargo build --release --locked                       → OK
version --json                                       → "1.1.2"
doctor http_client_posture → rustls TLS≥1.2 http2 system-proxy tcp_nodelay …
doctor concurrency → runtime_workers=N max_blocking=256
```

---

## 6. Camada F — retry com backoff (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_retry_com_backoff.md`  
> + GraphRAG/memories + duckduckgo (`httpdate`, AWS full jitter) + one-shot / memória / paralelismo.  
> Base já forte: `RetryConfig`, full-jitter, kill switch, GET-only, ADR 0001, `is_retryable`.

### 6.1 Escopo real (one-shot CLI — não serviço multi-dep)

| Superfície rules retry | Aplicável? | Conformidade |
|------------------------|------------|--------------|
| Política nomeada `RetryConfig` por dependência | Sim | **OK** (uma classe: hosts públicos HTTPS) |
| Só falhas transitórias + kill switch | Sim | **OK** (`--disable-retry` / TOML / `max_retries=0`) |
| Full jitter + monotônico `Instant`/`tokio::time` | Sim | **OK** (após RETRY-001…005) |
| `Retry-After` delta **e** HTTP-date | Sim | **OK** (após RETRY-001; crate `httpdate`) |
| `max_attempts` **e** `max_elapsed_ms` | Sim | **OK** (após RETRY-002) |
| `retry_kind` / `ErrorLayer` / `is_permanent` | Sim | **OK** (após RETRY-003) |
| Span `retry_attempt` + doctor | Sim | **OK** (após RETRY-004) |
| GET-only ⇒ idempotente sem `Idempotency-Key` | Sim | **OK** (produto) |
| Circuit breaker / token-bucket multi-dep | **N/A** | One-shot; ADR 0001 OOS |
| gRPC / OAuth refresh / hedged / saga / outbox | **N/A** | Fora do produto |
| POST com chave de idempotência | **N/A** | Superfície GET-only |
| Telemetria remota / alertas de retry rate | **N/A** | Proibido nestas entregas |

### 6.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **RETRY-001** | Alta | `Retry-After` só delta-seconds; HTTP-date ignorado (ADR adiou chrono) | `httpdate::parse_http_date`; skew ≤1s → zero; past profundo → fórmula; cap `HARD_MAX_DELAY_MS` | **RESOLVED** |
| **RETRY-002** | Alta | Só `max_attempts`; sem `max_elapsed_time` combinado | `RetryConfig.max_elapsed_ms` (`0` deriva de `timeout_secs`); `may_retry_within_budget`; sleep que estoura budget aborta | **RESOLVED** |
| **RETRY-003** | Média | Sem `retry_kind` / `ErrorLayer`; `408` permanente; transporte só flags soltas | `RetryKind`, `ErrorLayer`, `classify_reqwest_error`; `ErrorKind::retry_kind()`; HTTP `408` → Timeout transitório | **RESOLVED** |
| **RETRY-004** | Média | Logs debug sem span dedicado; doctor sem elapsed/HTTP-date | Span `retry_attempt` (`docsrs_cli::retry`); doctor detalha `max_elapsed_ms` + `retry_after=delta\|http-date` | **RESOLVED** |
| **RETRY-005** | Média | Base `< 50ms` permitida; testes com 5–10ms | Piso `MIN_RETRY_BASE_MS=50` em clamp/`from_config` | **RESOLVED** |
| **RETRY-006** | Baixa | `Retry-After` encolhido por `max_delay_ms` (violava “não impor backoff menor que o servidor”) | `wait_for_retry`: hint do servidor só com teto duro; budget recusa waits longos | **RESOLVED** |
| **RETRY-007** | Baixa | Jitter não determinístico em testes | `backoff_full_jitter_seeded` + property loop de cap | **RESOLVED** |
| **RETRY-008** | Baixa | Config/CLI/ADR sem knobs de elapsed/HTTP-date | CLI `--retry-max-elapsed-ms`, TOML, template, ADR 0001 EN/pt-BR, CHANGELOG Unreleased | **RESOLVED** |

### 6.3 Checklist rules (aplicável ao cliente one-shot)

| Item | Status |
|------|--------|
| Política explícita `RetryConfig` (não efeito colateral) | **OK** |
| Retry só em transitórios (429/408/5xx/transporte) | **OK** |
| Nunca 400/401/403/404/422/parse/budget | **OK** |
| Kill switch runtime (`disable_retry` / `max_retries=0`) | **OK** |
| Backoff exponencial truncado + full jitter | **OK** |
| Relógio monotônico (`Instant` + `tokio::time::sleep`) | **OK** |
| `max_attempts` + `max_elapsed_ms` juntos | **OK** |
| `Retry-After` delta-seconds **e** HTTP-date | **OK** |
| Sem jitter extra em `Retry-After` absoluto | **OK** |
| Sem sleep além do budget de elapsed | **OK** |
| `is_retryable` / `is_permanent` / `retry_kind` | **OK** |
| Camada de falha (`ErrorLayer`) em transporte | **OK** |
| Uma camada de retry no stack (`HttpClient::request`) | **OK** |
| GET-only (sem Idempotency-Key) | **OK** |
| ADR 0001 atualizado | **OK** |
| Cancel interrompe sleeps de retry | **OK** (já existia) |
| Sem `thread::sleep` em async | **OK** |
| Sem `match` em `error.to_string()` | **OK** |
| Circuit breaker / hedged / gRPC / OAuth | **N/A** (one-shot) |

### 6.4 One-shot · memória · paralelismo (cruzamento obrigatório)

| Rules | Aplicação nesta camada |
|-------|------------------------|
| One-shot | Retry in-process no GET único; sem daemon, sem re-spawn de processo; kill switch para incidente |
| Memória | Sem reter bodies entre tentativas (status 5xx aborta antes de ler body); cap de body inalterado; `Copy` em `RetryConfig` |
| Paralelismo | Um loop de retry por request; sem fan-out hedged; `spawn_blocking`/budget CPU intocados; sleeps canceláveis fora de locks |

### 6.5 Evidência de correção (camada F)

```text
Cargo.toml              → httpdate = "1" (+ dev-dep para testes)
src/retry.rs            → max_elapsed_ms; parse HTTP-date; RetryKind/ErrorLayer;
                          may_retry_within_budget; MIN_RETRY_BASE_MS; seeded jitter
src/http.rs             → budget gate no loop; span retry_attempt; classify_reqwest_error
src/error.rs            → retry_kind(); HTTP 408 → Timeout
src/config.rs / cli.rs  → retry_max_elapsed_ms + clamp/template/CLI
src/lib.rs              → doctor retry_policy com max_elapsed + http-date
docs/decisions/0001-*   → ADR EN/pt-BR alinhado
tests/http_integration  → HTTP-date 429; max_elapsed bloqueia 2ª tentativa
CHANGELOG[.pt-BR]       → Unreleased: dual budget, httpdate, spans
```

### 6.6 Validação

```text
cargo test --lib retry::                              → 12 ok
cargo test --test http_integration retry_             → 7 ok
cargo clippy --all-targets -- -D warnings             → OK
cargo build --release                                 → OK
version --json                                        → "1.1.2"
doctor retry_policy → enabled=true … max_elapsed_ms=30000
                      retry_after=delta|http-date kill_switch=--disable-retry
```

---

## 7. Camada G — segurança defensiva (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_seguranca_defensiva.md`  
> + GraphRAG (`rules-rust-seguranca-defensiva`, `rules-rust-seguranca`, validação/SSRF)  
> + duckduckgo-search-cli + context7 (reqwest) + one-shot / memória / paralelismo.  
> Base já forte: newtypes de domínio, allowlist HTTP, body/output caps, HTML scrub,  
> TLS rustls, GET-only, cancel cooperativo, 0o600/0o700, sem segredos de produto.

### 7.1 Escopo real (one-shot CLI — não servidor / não K8s / sem secrets)

| Superfície rules | Aplicável? | Conformidade |
|------------------|------------|--------------|
| `#![forbid(unsafe_code)]` (sem FFI) | Sim | **OK** (após SEC-001) |
| Entrada hostil (argv/env/TOML/rede) | Sim | **OK** (newtypes + caps + allowlist) |
| SSRF (origins + redirect + request) | Sim | **OK** (após SEC-002; defesa em profundidade) |
| Cap de `config.toml` / TOML bomb | Sim | **OK** (após SEC-003) |
| User-Agent / query sem control chars | Sim | **OK** (após SEC-004 / SEC-005) |
| Cache poisoned `final_url` | Sim | **OK** (após SEC-006) |
| Clamp redirects / timeouts / rate-limit | Sim | **OK** (após SEC-007) |
| `overflow-checks` em release | Sim | **OK** (após SEC-008) |
| Sem `expect` em path de método | Sim | **OK** (após SEC-009) |
| AuthN/AuthZ / secrets / Zeroizing | **N/A** | Produto sem API keys / sem secrets |
| SQL / shell / WASM / containers / SLSA CI | **N/A** | Fora do produto / proibido nestas entregas |
| Miri / loom / cargo-geiger em CI | **N/A** | Sem CI nesta entrega; zero `unsafe` no produto |
| Headers de servidor HTTP / rate-limit por identidade | **N/A** | Cliente one-shot, não servidor |

### 7.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **SEC-001** | Alta | Documentado “zero unsafe” sem enforcement do compilador | `#![forbid(unsafe_code)]` em `lib.rs` + `main.rs` | **RESOLVED** |
| **SEC-002** | Alta | `validate_origin` aceitava **qualquer** host (`https://evil.example`); SSRF só no request | Allowlist de host+scheme em `validate_origin` / redirect / request (`is_allowlisted_host` + `is_allowed_origin_scheme_host`); loopback só `cfg(test)` ou `DOCSRS_CLI_ALLOW_LOCALHOST` | **RESOLVED** |
| **SEC-003** | Alta | `config.toml` lido com `read_to_string` sem teto (TOML bomb / OOM) | `read_config_toml_capped` + `MAX_CONFIG_TOML_BYTES` (64 KiB); meta early-reject + check pós-read | **RESOLVED** |
| **SEC-004** | Média | User-Agent sem validação de charset/comprimento na fronteira CLI/TOML | `validate_user_agent` (ASCII visível, ≤256); `Config::validate_security`; CLI `--user-agent` pré-valida | **RESOLVED** |
| **SEC-005** | Média | `SearchQuery` aceitava control chars (NUL, newline, …) | Rejeita `char::is_control` → `invalid_input` | **RESOLVED** |
| **SEC-006** | Média | Cache hit não revalidava `final_url` allowlist (meta envenenado) | `cache::get` + re-check pós-redirect no body path | **RESOLVED** |
| **SEC-007** | Média | `max_redirects` / timeouts / rate-limit sem hard ceiling | `HARD_MAX_REDIRECTS=20`, `HARD_MAX_TIMEOUT_SECS=600`, connect 120s, rate-limit 60s | **RESOLVED** |
| **SEC-008** | Baixa | Release sem `overflow-checks` (checklist defensiva) | `profile.release.overflow-checks = true` | **RESOLVED** |
| **SEC-009** | Baixa | `expect` em path de método associado (`docs_rs`) | Match fail-closed → `ErrorKind::Internal` | **RESOLVED** |
| **SEC-010** | Info | Allowlist triplicada (redirect vs request vs config) | Uma fonte: `config::is_allowed_origin_scheme_host` usada nos três pontos | **RESOLVED** |

### 7.3 Checklist rules (aplicável ao cliente one-shot)

| Item | Status |
|------|--------|
| Toda entrada externa passa por validação tipada | **OK** (domain + config + HTTP) |
| `#![forbid(unsafe_code)]` no crate produto | **OK** |
| Nenhum `unwrap`/`expect` em caminho de produção (só testes / regex estáticos) | **OK** |
| Aritmética com `saturating_*` / tetos hard | **OK** |
| Caminhos de cache com chave hex (sem traversal via key) | **OK** |
| TLS rustls, min 1.2, sem `danger_*` | **OK** |
| Desserialização TOML/JSON limitada em tamanho | **OK** (config + body + meta) |
| Relógio monotônico para deadlines de retry | **OK** |
| Shutdown graceful + cancel cooperativo | **OK** |
| Sem secrets de produto / sem Zeroizing necessário | **N/A** |
| Containers / SLSA / cosign / WASM sandbox | **N/A** (fora de escopo) |

### 7.4 One-shot · memória · paralelismo (cruzamento obrigatório)

| Rules | Aplicação nesta camada |
|-------|------------------------|
| One-shot | Fail-closed na partida (config/UA/origins); sem daemon; sem spawn de processo no produto |
| Memória | Cap de TOML + body + cache meta; `try_reserve*` inalterado; sem reter payloads entre tentativas |
| Paralelismo | Allowlist/validação são pure/`&self`; sem locks novos; budget CPU / Tokio multi-thread intocados |

### 7.5 Evidência de correção (camada G)

```text
src/lib.rs / main.rs     → #![forbid(unsafe_code)]
src/config.rs            → is_allowlisted_host; validate_origin allowlist;
                           validate_user_agent; validate_security;
                           read_config_toml_capped; HARD_MAX_* clamp
src/http.rs              → is_allowed_host unificado; final_url re-check;
                           redirect policy usa mesma allowlist
src/cache.rs             → final_url allowlist no get
src/domain.rs            → SearchQuery rejeita control chars
src/docs_rs.rs           → method path fail-closed (sem expect)
Cargo.toml               → overflow-checks = true (release)
tests (config/domain)    → evil origin, oversized TOML, control query, clamps
```

### 7.6 Validação

```text
cargo test --locked                                      → OK (lib + integration)
cargo clippy --all-targets --locked -- -D warnings       → OK
cargo build --release --locked                           → OK
version --json                                           → "1.1.2"
doctor --json                                            → ok=true
```

---

## 8. Camada H — segurança para desenvolvimento Rust (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust `docs_rules/rules_rust_seguranca.md`  
> + GraphRAG (`rules-rust-seguranca`, fundaments/validação/injecção/config)  
> + duckduckgo-search-cli (RegexBuilder size_limit) + context7 (regex crate)  
> + one-shot / memória / paralelismo.  
> Camada G já cobriu forbid(unsafe), SSRF allowlist, TOML cap, UA, overflow-checks.  
> Esta camada fecha **modelo de ameaças**, ReDoS posture, fail-closed residual e docs SECURITY.

### 8.1 Escopo real (one-shot CLI)

| Superfície rules | Aplicável? | Conformidade |
|------------------|------------|--------------|
| Modelo de ameaças / STRIDE | Sim | **OK** (após SECDEV-001 — ADR 0004) |
| `#![forbid(unsafe_code)]` | Sim | **OK** (camada G) |
| Newtypes + TryFrom em fronteiras | Sim | **OK** |
| ReDoS: `RegexBuilder` size/dfa limits | Sim | **OK** (após SECDEV-003) |
| Validação free-text (invisíveis/bidi) | Sim | **OK** (após SECDEV-004) |
| TOML fail-closed (unknown keys) | Sim | **OK** (após SECDEV-005) |
| Cache path key allowlist (hex) | Sim | **OK** (após SECDEV-006) |
| MIME confusion (Content-Type) | Sim | **OK** (após SECDEV-007) |
| SECURITY supported versions | Sim | **OK** (após SECDEV-002) |
| SQL / shell / secrets / mTLS / JWT | **N/A** | Produto sem essas superfícies |
| CI `cargo geiger` / SLSA / containers | **N/A** | Fora de escopo destas entregas |
| NFC em free-text search | Aceito | IDs ASCII; format chars rejeitados (ADR 0004) |

### 8.2 Gaps identificados e correção

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **SECDEV-001** | Alta | Sem modelo de ameaças STRIDE documentado | ADR `docs/decisions/0004-threat-model.md` (+ pt-BR); link em SECURITY | **RESOLVED** |
| **SECDEV-002** | Alta | `SECURITY.md` dizia `1.1.x` histórico e `0.1.x` suportado (contraditório) | Linha suportada = **1.1.x**; 0.1.x histórico | **RESOLVED** |
| **SECDEV-003** | Média | `Regex::new` sem `size_limit` / `dfa_size_limit` | `domain::compile_bounded_regex` (256 KiB); todos os usos produto migrados | **RESOLVED** |
| **SECDEV-004** | Média | `SearchQuery` só rejeitava `is_control` (ZWSP/bidi passavam) | `is_hostile_text_char` (controls + ZW* + bidi + BOM) | **RESOLVED** |
| **SECDEV-005** | Média | TOML aceitava chaves desconhecidas (typo silencioso) | `#[serde(deny_unknown_fields)]` em `TomlConfig` | **RESOLVED** |
| **SECDEV-006** | Média | `paths_for_key` confiava em qualquer string de filename no scan | Só chaves SHA-256 hex (64 `[0-9a-f]`) | **RESOLVED** |
| **SECDEV-007** | Média | Content-Type errado só gerava `warn` | `require_content_type_{json,html}` fail-closed quando CT presente | **RESOLVED** |
| **SECDEV-008** | Info | Riscos aceitos não enumerados | ADR 0004 tabela de riscos aceitos + CVSS SLA já em SECURITY | **RESOLVED** |

### 8.3 Checklist rules (aplicável)

| Item | Status |
|------|--------|
| Modelo de ameaças + STRIDE documentado | **OK** (ADR 0004) |
| `#![forbid(unsafe_code)]` | **OK** |
| Entrada validada em newtypes | **OK** |
| Regex com size/dfa limits | **OK** |
| Sem SQL/shell injection surface no produto | **N/A** / **OK** |
| TLS rustls, allowlist SSRF | **OK** (G+E) |
| Release `lto`/`strip`/`panic=abort`/`overflow-checks` | **OK** |
| `SECURITY.md` canal de reporte + CVSS SLA | **OK** |
| cargo audit (local): 0 vulns; 0 unmaintained (pós DEPS-J) | **OK** |

### 8.4 One-shot · memória · paralelismo (cruzamento obrigatório)

| Rules | Aplicação nesta camada |
|-------|------------------------|
| One-shot | Threat model assume processo morre após 1 comando; sem sessão, sem mTLS de serviço |
| Memória | Regex size caps; cache key gate evita join hostil; CT fail-closed evita parse de body errado |
| Paralelismo | Nenhuma lock nova; `compile_bounded_regex` pure; scan cache continua single-threaded |

### 8.5 Evidência de correção (camada H)

```text
docs/decisions/0004-threat-model[.pt-BR].md  → STRIDE + riscos aceitos
SECURITY[.pt-BR].md                          → 1.1.x suportado; link ADR
src/domain.rs                                → compile_bounded_regex; is_hostile_text_char
src/docs_rs.rs                               → RegexBuilder + require_content_type_html
src/crates_io.rs                             → require_content_type_json
src/http.rs                                  → require_content_type_{json,html}
src/config.rs                                → deny_unknown_fields + teste typo
src/cache.rs                                 → is_cache_key_hex / paths_for_key Option
tests/e2e_offline.rs + http_integration.rs   → wiremock `set_body_raw(..., mime)` (set_body_string força text/plain)
CHANGELOG[.pt-BR]                            → Unreleased Security
```

### 8.6 Validação

```text
cargo test --locked                                      → OK
cargo clippy --all-targets --locked -- -D warnings       → OK
cargo build --release --locked                           → OK
version --json                                           → "1.1.2"
cargo audit                                              → 0 vulnerabilities (2 unmaintained warnings)
```

---

## 9. Camada I — meta-auditoria (o que faltou / esqueceu / omitiu) (2026-07-19)

> Origem: follow-up do usuário após Camada H — honestidade sobre ferramentas, causa×efeito,
> SQL idempotente, componentização, gaps residuais.  
> Rules: one-shot · memória · paralelismo.

### 9.1 Honestidade — ferramentas obrigatórias da skill `r-auditoria`

| Ferramenta | Obrigatória? | Uso na Camada H (antes) | Uso nesta Camada I |
|------------|--------------|-------------------------|--------------------|
| **GraphRAG** (`graphrag.sqlite` / memories) | Sim | **Parcial** (queries SQL a memories/entities) | Mantido + reconsulta |
| **context7** | Sim | **Incompleto** — subcomandos errados (`get-library-docs`, flags inválidas) | **Corrigido**: `context7 docs /rust-lang/regex`; ainda ruidoso (snippets genéricos) |
| **duckduckgo-search-cli** | Sim | **Parcial** via `ddgs text` (não o binário full) | **Corrigido**: `duckduckgo-search-cli text "…"` |
| **docsrs-cli** (o próprio produto) | Sim (skill) | **Omitido** na H | **Corrigido**: `search-crates regex`, `get-item regex method RegexBuilder::size_limit` |

**Causa → efeito:** skill exige as três ferramentas + GraphRAG → uso incompleto na H → risco de gaps de documentação/API não validados no runtime real → I reexecuta com binários corretos e fecha o que ainda faltava.

### 9.2 O que faltava / esquecido / omitido (pós-H)

| ID | Tipo | Causa | Efeito | Correção | Status |
|----|------|-------|--------|----------|--------|
| **META-001** | Processo | context7 com API errada | Docs oficiais de `RegexBuilder` não lidas de forma confiável | Re-run `context7 docs` + `docsrs-cli get-item RegexBuilder::size_limit` (confirma size_limit na API real) | **RESOLVED** (processo) |
| **META-002** | Processo | Sem `docsrs-cli` na H | Validação “dogfood” do produto ausente | `search-crates` / `get-item` live nesta sessão | **RESOLVED** |
| **META-003** | Segurança | `contact` TOML sem charset/tamanho | Contact hostil vira UA inválido ou header ambíguo | `validate_contact` + `validate_security` | **RESOLVED** |
| **META-004** | SRP | `lib.rs` ~1944 linhas (dispatch+doctor+suggest+I/O) | Alta acoplagem; difícil auditar um comando | Extrair `doctor.rs`, `suggest.rs`, `output.rs` | **RESOLVED** (parcial — dispatch ainda grande) |
| **META-005** | SRP | `docs_rs.rs` ~1916 linhas | URLs+fetch+HTML+search no mesmo arquivo | **Documentado** como oportunidade; não fatiado nesta rodada (risco de regressão HTML) | **ACEITO / backlog** |
| **META-006** | Escopo | SQL no produto | N/A — produto **não** usa SQL | Idempotência SQL **N/A** (sem `rusqlite`/`sqlx` em `src/`) | **N/A** |

### 9.3 Componentização (arquivos grandes)

| Arquivo | Linhas (após I) | Responsabilidade | Ação |
|---------|-----------------|------------------|------|
| `src/docs_rs.rs` | **~1916** | URLs + fetch + sanitize + all.html + extract | **Backlog COMP-001**: dividir em `docs_rs/{urls,fetch,html,search}.rs` |
| `src/lib.rs` | **~1448** (era ~1944) | run + dispatch + cache/config/schema cmds | **COMP-002 parcial**: doctor/suggest/output extraídos |
| `src/config.rs` | **~1340** | constants + load + validate + init + testes | **Backlog COMP-003**: `config/{constants,load,validate}.rs` |
| `src/http.rs` | ~950 | client HTTP | OK (um domínio) |
| `src/doctor.rs` | ~358 | **novo** | OK |
| `src/suggest.rs` | ~171 | **novo** | OK |
| `src/output.rs` | ~36 | **novo** | OK |

### 9.4 Gaps corrigidos nesta camada

| ID | Severidade | Gap | Correção | Status |
|----|------------|-----|----------|--------|
| **COMP-001** | Info | docs_rs monólito | Backlog explícito + ADR-style nota em gaps | **ACEITO** (não merge de split arriscado agora) |
| **COMP-002** | Média | lib monólito | `doctor` / `suggest` / `output` modules | **RESOLVED** |
| **COMP-003** | Info | config monólito | Backlog | **ACEITO** |
| **SECDEV-009** | Média | contact sem validação | `validate_contact` + teste | **RESOLVED** |

### 9.5 Oportunidades de melhoria (não bloqueantes)

1. **Fatiar `docs_rs.rs`** em submódulos (`urls` / `fetch` / `html` / `search`) com `pub use` estável.
2. **Fatiar `dispatch` em `lib.rs`** por comando (`ops/search.rs`, `ops/readme.rs`, …).
3. **Fatiar `config.rs`** constants vs load vs init.
4. ~~Substituir `scraper`→`fxhash`~~ **feito na J** (`scraper` 0.27).
5. ~~`aquamarine` / `proc-macro-error2`~~ **removido na J**.
6. **NFC opcional** em `SearchQuery` se comparações de identidade forem adicionadas (hoje free-text API).
7. **Clippy pedantic / nursery** seletivo (não full pedantic — ruído alto em CLI).
8. **loom** só se surgir estado concorrente não trivial além de `Atomic` + budget.

### 9.6 SQL e idempotência

| Superfície | SQL? | Idempotência |
|------------|------|--------------|
| `src/**` produto | **Não** | N/A |
| Cache disco | arquivos meta/body com rename atômico | put é “last write wins”; get fail-closed em meta corrupta |
| GraphRAG `.sqlite` no workspace | ferramenta de dev, **não** runtime do CLI | Fora do produto |

### 9.7 One-shot · memória · paralelismo (cruzamento)

| Rules | Aplicação |
|-------|-----------|
| One-shot | Extrações de módulo não introduzem daemon; doctor continua síncrono |
| Memória | `output::write_json` buffer intermediário; suggest com edit-distance limitado |
| Paralelismo | Sem locks novos; doctor/suggest single-thread; Tokio multi-thread intocado |

### 9.8 Evidência

```text
src/doctor.rs    → doctor + dir_ready_check (SRP)
src/suggest.rs   → rank_suggestions + edit_distance + pick_unique_reexport
src/output.rs    → write_json + map_stdout_err
src/config.rs    → validate_contact
src/lib.rs       → ~1448 linhas (era ~1944)
cargo test --lib → 175 ok
cargo clippy -D warnings → OK
docsrs-cli get-item regex method RegexBuilder::size_limit → ok
```

### 9.9 Validação

```text
cargo test --locked --lib   → OK
cargo clippy --all-targets --locked -- -D warnings → OK
```

---

## 10. Camada J — componentização SRP/DRY (2026-07-19)

> Origem: meta-auditoria (honestidade A–I) + rules one-shot / memória / paralelismo.
> **Compromisso:** COMP-001/003 não ficam mais “ACEITO backlog” — split real mergeado.

### 10.1 FAQ honesto (resumo)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | A–H sim (produto/segurança); I **parcial** em SRP; **J fecha** monólitos |
| O que faltava? | Split `docs_rs`/`config`/`lib`; DRY leaf-hit; tools com API correta |
| Esquecido/omitido? | Contar “ACEITO” como fechado; `src/docs_rs/` vazio; context7 `-q` |
| Corrigindo todos os gaps J? | **Sim** (DEPS transitivos só documentados) |

### 10.2 Causa × efeito

```
Inventário "zero abertos" + COMP ACEITO
  → monólitos 1916/1446/1340L permanecem
  → review de HTML/dispatch misturada
  → J: split + pub use estável + testes
```

### 10.3 Gaps da camada

| ID | Sev | Gap | Correção | Status |
|----|-----|-----|----------|--------|
| **COMP-001** | Alta | `docs_rs.rs` 1916L | `src/docs_rs/{mod,types,urls,html,fetch,search,hits}.rs` | **RESOLVED** |
| **COMP-002b** | Média | `lib.rs` 1446L dispatch | `ops.rs` + `meta_cmds.rs`; lib **~554L** | **RESOLVED** |
| **COMP-003** | Média | `config.rs` 1340L | `src/config/{mod,constants,load,validate}.rs` | **RESOLVED** |
| **DRY-J-001** | Baixa | leaf-score duplicado | `docs_rs/hits.rs` + `suggest` usa helpers | **RESOLVED** |
| **HYGIENE-J-001** | Baixa | `docs_rs/` vazio | diretório populado (mod.rs + submódulos) | **RESOLVED** |
| **PROC-J-001** | Processo | tools API/uso | context7 `docs ID -q`; ddg; docsrs-cli dogfood | **RESOLVED** |
| **DEPS-J-001** | Info | fxhash / proc-macro-error2 unmaintained | Removido `aquamarine`; `scraper` 0.27 (sem fxhash) | **RESOLVED** |

### 10.4 Tamanhos (após J)

| Unidade | Linhas |
|---------|--------|
| `src/lib.rs` | **~554** (era ~1446 / antes I ~1944) |
| `src/ops.rs` | ~521 |
| `src/meta_cmds.rs` | ~569 |
| `src/docs_rs/*` | types 105 · urls 385 · html 279 · fetch 350 · search 348 · hits 46 · mod 518 |
| `src/config/*` | constants 94 · validate 219 · load 692 · mod 372 |

### 10.5 One-shot · memória · paralelismo

| Rule | Aplicação no split |
|------|--------------------|
| One-shot | Zero daemon; só reorganização de módulos |
| Memória | Budgets/body caps intocados; parse em `spawn_blocking` |
| Paralelismo | `rayon` permanece em `docs_rs/search`; `ConcurrencyBudget` intocado |

### 10.6 Ferramentas (evidência J)

| Tool | Uso |
|------|-----|
| context7 | `library clap/regex`; `docs /rust-lang/regex -q "…"` (API: `-q`, não `--topic`) |
| duckduckgo-search-cli | crate layout / mod.rs best practices → layout `src/foo/*.rs` |
| docsrs-cli | `version` 1.1.2; dogfood search/get-item em sessão |
| cargo audit | **0** warnings (pós DEPS-J: sem fxhash / sem aquamarine) |

### 10.7 DRY com disciplina

- Helper `leaf_eq_ignore_ascii` / `unique_best_score_hit` em `hits.rs`
- **Não** fundir `pick_unique_type_path` (parents de method) com `pick_unique_reexport_path` (kind pedido)
- `pub use` em `docs_rs/mod.rs` e `config/mod.rs` mantém paths públicos

### 10.8 Oportunidades (não gaps)

1. ~~Remover `aquamarine`~~ **feito** (fence plain text no rustdoc)  
2. ~~`scraper` sem `fxhash`~~ **feito** (0.27 → selectors 0.38 / rustc-hash)  
3. AGENTS.md limiar “arquivo >800L → split”  
4. Clippy pedantic seletivo (não full)

### 10.9 Validação

```text
cargo test --locked          → OK (lib 175 + e2e/integration/network/signal)
cargo clippy --all-targets --locked -- -D warnings → OK
docsrs-cli version --json    → 1.1.2
```

---

## 11. Inventário consolidado (IDs ativos 1.1.2+)

| Prefixo | Intervalo | Aberto |
|---------|-----------|--------|
| BUG / GAP / WARN / R (produto 1.1.2) | BUG-001…WARN-014, R1–R2 | **0** |
| PROC (processos 2026-07-19) | PROC-001…006 | **0** |
| HARD (hardcode 2026-07-19) | HARD-001…009 | **0** |
| NET (rede 2026-07-19) | NET-001…008 | **0** |
| RETRY (retry/backoff 2026-07-19) | RETRY-001…008 | **0** |
| SEC (segurança defensiva 2026-07-19) | SEC-001…010 | **0** |
| SECDEV (segurança desenvolvimento 2026-07-19) | SECDEV-001…009 | **0** |
| META / COMP (meta + SRP 2026-07-19) | META-001…006, COMP-001…003 | **0** (COMP-001/002b/003 **RESOLVED** na J) |
| J (componentização 2026-07-19) | DRY-J-001, HYGIENE-J-001, PROC-J-001, DEPS-J-001 | **0** (DEPS-J-001 **RESOLVED**) |
| K (serde/validação 2026-07-19) | SERDE-K-001…009, DRY-K-001, COMP-K-001a/b, PROC-K-001, HYGIENE-K-001 | **0** |
| L (sistema de tipos 2026-07-19) | TYPE-L-001…011, DRY-L-001, COMP-L-001, NAMING-L-001, PROC-L-001, HYGIENE-L-001 | **0** |
| Histórico 0.1.2 | GAP-001…018 | **0** |

**Fim do inventário aberto.** Próximas auditorias devem **acrescentar** seções (camada M+) sem apagar as anteriores.

---

## 12. Camada K — Pipeline Serde / Validação (2026-07-19)

### 12.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — inventário serde completo + monólitos + tools |
| 4-crates (`validator`/`serde_with`)? | **ADR 0005**: não cargo-cult; pipeline real = newtypes + serde wire |
| context7 / ddg / docsrs-cli? | **Sim** (tree 1.1.2 via `cargo run`) |
| Causa×efeito? | **Sim** (meta deny; caps API; split >800L) |
| Corrige todos gaps? | **Sim** — todos RESOLVED |

### 12.2 Gaps

| ID | Sev | Correção | Status |
|----|-----|----------|--------|
| **SERDE-K-001** | Média proc | ADR 0005 (sem deps fantasmas) | **RESOLVED** |
| **SERDE-K-002** | Baixa | `CacheMeta` `deny_unknown_fields` | **RESOLVED** |
| **SERDE-K-003** | Baixa | digests hex-64 no parse meta | **RESOLVED** |
| **SERDE-K-004** | Info | envelopes write-only documentados | **RESOLVED** (ADR) |
| **SERDE-K-005** | Info | clamp vs fail-closed documentado | **RESOLVED** (ADR) |
| **SERDE-K-006/008/009** | Baixa | caps name/desc/URL/token no map API | **RESOLVED** |
| **SERDE-K-007** | Info | DTOs output+Deserialize documentados | **RESOLVED** (ADR) |
| **DRY-K-001** | Baixa | `is_sha256_hex` único | **RESOLVED** |
| **COMP-K-001a** | Média | `src/cache/*` | **RESOLVED** |
| **COMP-K-001b** | Média | `src/http/*` | **RESOLVED** |
| **PROC-K-001** | Processo | dogfood tree 1.1.2 | **RESOLVED** |
| **HYGIENE-K-001** | Baixa | consts de cap | **RESOLVED** |

### 12.3 Tamanhos pós-K

| Unidade | Linhas (aprox.) |
|---------|-----------------|
| `src/cache/*` | disk 401 · mod 300 · types 81 · meta 66 · hex 61 · paths 39 |
| `src/http/*` | client 537 · mod 194 · body 90 · rate_limit 83 · content_type 77 · constants 19 · allowlist 14 |

### 12.4 Validação

```text
cargo test --locked → OK (lib 180+ + e2e/integration/network/signal)
cargo clippy --all-targets --locked -- -D warnings → OK
cargo audit → 0 vulns
cargo run -- version --json → 1.1.2
```

---

## 13. Camada L — Sistema de Tipos (2026-07-19)

### 13.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — newtypes, core path stringly, monólito domain, tools |
| context7 / ddg / docsrs-cli? | **Sim** (tree 1.1.2 via `cargo run`) |
| Causa×efeito? | **Sim** (prova some após `.as_str()`; origins mutáveis; dual stdlib) |
| Corrige todos gaps? | **Sim** — todos RESOLVED |
| DRY / one-shot / memória / paralelismo? | **Sim** — `is_stdlib_name` único; `OpCtx`; zero-cost refs; `Send+Sync` |

### 13.2 Gaps

| ID | Sev | Correção | Status |
|----|-----|----------|--------|
| **TYPE-L-001** | Alta | Core path tipado (`&CrateName`/`&VersionArg`/`&SearchQuery`) em urls/fetch/search/crates_io/ops | **RESOLVED** |
| **TYPE-L-002** | Média | Wrappers validate-only removidos; testes no domínio | **RESOLVED** |
| **TYPE-L-003** | Baixa | `#[repr(transparent)]` + `size_of` tests | **RESOLVED** |
| **TYPE-L-004** | Média | `AllowedOrigin` em `Config` | **RESOLVED** |
| **TYPE-L-005/006/007** | Info | ADR 0006 (units Config; wire DTO; sem typestate) | **RESOLVED** (ADR) |
| **TYPE-L-008** | Baixa | `MatchMode::score(..., impl AsRef<str>)` | **RESOLVED** |
| **TYPE-L-009** | Média | `SortKind` em `crates_io` planners | **RESOLVED** |
| **TYPE-L-010** | Baixa | `VersionArg::stdlib_channel` única fonte | **RESOLVED** |
| **TYPE-L-011** | Média | `OpCtx` (handlers sem allow massivo) | **RESOLVED** |
| **DRY-L-001** | Média | `is_stdlib_name` / `STDLIB_NAMES` únicos | **RESOLVED** |
| **COMP-L-001** | Média | `src/domain/*` split | **RESOLVED** |
| **NAMING-L-001** | Info | `get_item` = verbo documentado (ADR/mod) | **RESOLVED** |
| **PROC-L-001** | Processo | dogfood tree 1.1.2 | **RESOLVED** |
| **HYGIENE-L-001** | Baixa | gaps + CHANGELOG + ADR 0006 | **RESOLVED** |

### 13.3 Tamanhos pós-L

| Unidade | Linhas (aprox.) |
|---------|-----------------|
| `src/domain/*` | crate_name 156 · item_path 166 · crate_ref 159 · version 149 · origin 148 · match_mode 106 · search_query 96 · regex 47 · mod 29 |
| `src/ops.rs` | ~464 (`OpCtx`) |

### 13.4 Validação

```text
cargo test --locked → OK (lib 182+ + e2e/integration/network/signal)
cargo clippy --all-targets --locked -- -D warnings → OK
cargo audit → 0 vulns
cargo run -- version --json → 1.1.2
```

---

## 14. Camada M — rustls obrigatório (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust uso obrigatório de rustls  
> + one-shot · memória · paralelismo · XDG · multi-OS  
> Tools: context7, duckduckgo-search-cli, docsrs-cli, cargo tree/metadata/audit.

### 14.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — cadeia, features, provider, bootstrap, monólitos, tools |
| CLI usa TLS? | **Sim** — rustls 0.23.x + ring + webpki-roots; min TLS 1.2; HTTPS allowlist |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (tabela TLS-M) |
| Corrige todos gaps? | **Sim** — todos RESOLVED ou N/A no ADR 0007 |
| Provider aws-lc? | **N/A com ADR** — Opção A ring (portabilidade musl/cross; reqwest 0.12 path) |
| Monólito split? | **Não** — client 537 / retry 610 < 800 |
| Env knobs TLS? | **Proibido** — posture compile-time + código |
| CI / telemetria? | **Proibido** — `deny.toml` local only |

### 14.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **TLS-M-001** | Média | reqwest puxa ring; rules preferem aws-lc → desalinhamento | ADR 0007 Option A (ring) | **RESOLVED** |
| **TLS-M-002** | Média | sem `install_default` no main → provider implícito | `main`: ring `install_default` antes do Tokio | **RESOLVED** |
| **TLS-M-003** | Baixa | rustls só transitivo → floor invisível | dep direta `rustls ≥0.23.18` features std/tls12/ring | **RESOLVED** |
| **TLS-M-004** | Baixa | `ErrorLayer::Tls` morto → diag genérica | `source_chain_has_rustls` + classify | **RESOLVED** |
| **TLS-M-005** | Baixa | README sem § TLS | README + pt-BR | **RESOLVED** |
| **TLS-M-006** | Média | sem ADR formal | ADR 0007 EN+pt-BR | **RESOLVED** |
| **TLS-M-007** | Baixa | sem deny local | `deny.toml` ban TLS alternativo + aws-lc dual | **RESOLVED** |
| **TLS-M-008** | Baixa | doctor genérico | posture `provider=ring rustls≥0.23.18 …` | **RESOLVED** |
| **TLS-M-009** | Baixa | webpki-roots freshness | governado no ADR; update se stale no lock | **RESOLVED** (processo) |
| **TLS-M-010** | Info | proxy system-proxy | aceito no ADR/SECURITY (trust operador) | **RESOLVED** (doc) |

### 14.3 Evidência

```text
Cargo.toml     → rustls pin 0.23.18+ ring; reqwest rustls-tls
src/main.rs    → install_default(ring) antes do runtime
src/http/*     → use_rustls_tls + min TLS 1.2; posture detail
src/retry.rs   → ErrorLayer::Tls via source chain
deny.toml      → ban native-tls/openssl/aws-lc*
docs/decisions/0007-rustls-posture.md
```

### 14.4 Validação

```text
cargo tree -i native-tls / openssl → vazio
cargo tree -p rustls -e features → ring,std,tls12
cargo audit → 0 vulns
cargo test --locked → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo run --locked -- doctor --json → provider=ring
```

---

## 15. Camada N — Tipos de Domínio chrono · uuid · rust_decimal · url (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust tipos de domínio  
> + one-shot · memória · paralelismo · XDG · multi-OS  
> Tools: context7, duckduckgo-search-cli, docsrs-cli, cargo tree/metadata.

### 15.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — 4 crates vs domínio real; builders; wire; tempo; monólitos |
| chrono / uuid / rust_decimal? | **N/A com ADR 0008** — Instant/httpdate; SHA-256; sem moeda |
| url? | **Sim** — 2.5.8; core path `Url` + `AllowedOrigin` |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (tabela DOM-N) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A no ADR |
| Monólito split? | **Não** — urls 391 / crates_io 652 < 800 |
| Env knobs? | **Proibido** (exceto sandbox localhost testes) |
| CI / telemetria? | **Proibido** |

### 15.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **DOM-N-001** | Média | checklist 4 crates → risco cargo-cult | ADR 0008 N/A chrono/uuid/decimal | **RESOLVED** |
| **DOM-N-002** | Alta | builders `AsRef<str>` apagam prova | `*_on_origin` / `planned_url_*` → `&AllowedOrigin` | **RESOLVED** |
| **DOM-N-003** | Média | origin sem `Url` tipado | `AllowedOrigin::to_url()` | **RESOLVED** |
| **DOM-N-004** | Baixa | Cargo.toml sem postura url | comentário ADR 0008; sem feature serde ociosa | **RESOLVED** |
| **DOM-N-005** | Baixa | testes com origin string crua | `AllowedOrigin` / `origin_of` | **RESOLVED** |
| **DOM-N-006** | Baixa | doctor sem domain types | check `domain_types` | **RESOLVED** |
| **DOM-N-007** | Info | gaps/CHANGELOG | §15 + Unreleased | **RESOLVED** |
| **DOM-N-NA-*** | — | chrono/uuid/decimal/serde-url | ADR 0008 | **N/A (doc)** |

### 15.3 Evidência

```text
docs/decisions/0008-domain-types-posture.md (+ pt-BR)
src/domain/origin.rs     → to_url()
src/docs_rs/{urls,fetch,search}.rs → &AllowedOrigin
src/crates_io.rs         → planned_url_on_host(&AllowedOrigin)
src/doctor.rs            → domain_types check
Cargo.toml               → url posture comment
```

### 15.4 Validação

```text
cargo tree -i chrono / uuid / rust_decimal → vazio
cargo test --locked → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo run --locked -- doctor --json → domain_types=url=2; chrono=absent…
```

---

## 16. Camada O — Tratamento de Erros (Rules Rust) (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust tratamento de erros  
> + one-shot · memória · paralelismo · XDG · multi-OS  
> Tools: context7 (`thiserror`), duckduckgo-search-cli, docsrs-cli dogfood.

### 16.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — modelo ADR 0002, produção unwrap, source chain, Display, defaults, emit, monólitos |
| context7 / ddg / docsrs-cli? | **Sim** — thiserror 2.0.19; Display/source conventions |
| Causa×efeito? | **Sim** (ERR-O-*) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A (Usage clap) no ADR |
| Monólito split? | **Não** — render 706 / error 473 < 800 |
| Env knobs produto? | **Proibido**; harness localhost permanece; mensagem origin não promove env |
| CI / telemetria? | **Proibido** |

### 16.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **ERR-O-001** | Alta | `html_to_markdown` embute `{e}` | `with_source` + message curta | **RESOLVED** |
| **ERR-O-002** | Média | Display HTTP/HTML/CPU maiúsculo | lowercase | **RESOLVED** |
| **ERR-O-003** | Média | pretty JSON `unwrap_or_default`/`{}` | `map_err` Internal; `render_schema_markdown` → `AppResult` | **RESOLVED** |
| **ERR-O-004** | Baixa | acquire descarta `AcquireError` | `with_source` | **RESOLVED** |
| **ERR-O-005** | Baixa | `let _ = remove_entry` sem razão | comentário best-effort eviction | **RESOLVED** |
| **ERR-O-006** | Baixa | doctor sem error model | check `error_model` | **RESOLVED** |
| **ERR-O-007** | Baixa | `# Errors` incompleto | ops/meta/doctor rustdoc | **RESOLVED** |
| **ERR-O-008** | Info | Usage clap multi-line | ADR 0002 exception (não strip) | **N/A (doc)** |
| **ERR-O-009** | Média | origin ensina env harness | reword production config | **RESOLVED** |
| **ERR-O-010** | Baixa | `"stdout write"` voz incompleta | `"stdout write failed"` | **RESOLVED** |
| **HYG-O-001** | Processo | gaps/CHANGELOG/ADR | §16 + Unreleased + ADR 0002 | **RESOLVED** |

### 16.3 Evidência

```text
docs/decisions/0002-error-model.md (+ pt-BR)  → source/Usage/Display rules
src/docs_rs/html.rs       → with_source html→md
src/http/client.rs        → "http request failed…"
src/concurrency.rs        → acquire with_source; cpu messages lowercase
src/output.rs             → "stdout write failed"
src/meta_cmds.rs / render → pretty-print map_err
src/domain/origin.rs      → no env promo in operator message
src/doctor.rs             → error_model check
src/error.rs              → with_source_display_does_not_embed_cause
```

### 16.4 Validação

```text
cargo test --locked → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo run --locked -- doctor --json → error_model ok
rg 'HTML to Markdown|HTTP request failed|CPU worker' src → vazio (produção)
```

---

## 17. Camada P — Unsafe Code e FFI (Rules Rust) (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust unsafe/FFI  
> + one-shot · memória · paralelismo · XDG · multi-OS · **proibido env de knobs**  
> Tools: context7 (std SAFETY), duckduckgo-search-cli (`set_var` unsafe), docsrs-cli dogfood (`libc` 0.2.186 / `kill`).

### 17.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — inventário `unsafe`/FFI + causa raiz env allowlist |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (UNSAFE-P-*) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A no ADR 0009 |
| Monólito split? | **Não** — testes ≥800 sem ganho de soundness |
| Env knobs allowlist? | **Removido**; loopback = CLI/XDG `allow_loopback` |
| CI / telemetria? | **Proibido** |

### 17.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **UNSAFE-P-001** | Alta | Allowlist lia env `DOCSRS_CLI_ALLOW_LOCALHOST` | `Config.allow_loopback` + TOML + `--allow-loopback`; env removido | **RESOLVED** |
| **UNSAFE-P-002** | Alta | Testes `unsafe set_var` duplicados | Removidos; testes usam flag/TOML | **RESOLVED** |
| **UNSAFE-P-003** | Média | Allowlist/HTTP/cache sem policy unificada | `allow_loopback` em allowlist, redirect, request, `DiskCache` | **RESOLVED** |
| **UNSAFE-P-004** | Baixa | SAFETY de `libc::kill` incompleto | Bullets pid_t / errno / ownership | **RESOLVED** |
| **UNSAFE-P-005** | Baixa | Doctor sem posture unsafe | check `unsafe_posture` | **RESOLVED** |
| **UNSAFE-P-006** | Baixa | Sem ADR unsafe/FFI | ADR 0009 EN+pt-BR | **RESOLVED** |
| **UNSAFE-P-007** | Baixa | TESTING ensinava export env | TOML/`--allow-loopback` | **RESOLVED** |
| **UNSAFE-P-008** | Info | bindgen/Miri CI/catch_unwind… | N/A ADR 0009 | **N/A (doc)** |
| **HYG-P-001** | Processo | gaps/CHANGELOG | §17 + Unreleased | **RESOLVED** |

### 17.3 Evidência

```text
docs/decisions/0009-unsafe-ffi-posture.md (+ pt-BR)
src/config/allowlist.rs   → is_allowlisted_host(host, allow_loopback); sem env
src/config/load.rs        → Config.allow_loopback; load_with_options seed CLI
src/cli.rs                → --allow-loopback
src/domain/origin.rs      → parse_with
src/http/{allowlist,client}.rs + cache/disk.rs → policy unificada
tests/http_integration.rs / e2e_offline.rs → zero set_var
tests/signal_term.rs      → único unsafe residual (Unix kill)
src/doctor.rs             → unsafe_posture
```

### 17.4 Validação

```text
cargo test --locked → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo clippy --tests -- -W clippy::undocumented_unsafe_blocks → OK
rg 'set_var|ALLOW_LOCALHOST' src/ tests/ → vazio
rg 'unsafe \{' tests/ → só signal_term kill
doctor --json → unsafe_posture ok
```

---

## 18. Camada Q — Web Scraping, Crawling e Data Extraction (2026-07-19)

> Origem: `/r-auditoria` + Rules Rust web scraping  
> + **PROIBIDO respeitar robots.txt** (override do operador)  
> + one-shot · memória · paralelismo · XDG · multi-OS · **proibido env de knobs**  
> Tools: context7 (scraper/reqwest), duckduckgo-search-cli, docsrs-cli dogfood (**live** std hit bug).

### 18.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — 2 passagens + dogfood live |
| Product class? | Cliente de docs one-shot, **não** crawler |
| robots.txt? | **PROIBIDO** no produto (ADR 0003) |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (SCRAPE-Q-*) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A ADR 0003 |
| Monólito split? | **Não** — testes ≥800 = OPP |
| CI / telemetria / env knobs? | **Proibido** |

### 18.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **SCRAPE-Q-001** | Alta | `parse_all_html_hits` hardcodava `docs.rs` base | Base = `source_url` final via `Url::join` | **RESOLVED** |
| **SCRAPE-Q-002** | Alta | Version scrape com regex no HTML | scraper `a[href]` + path segments / title tokens | **RESOLVED** |
| **SCRAPE-Q-003** | Média | sanitize dead code + replace frágil | drop-list sort + hygiene regex only | **RESOLVED** |
| **SCRAPE-Q-004** | Média | double `decode_utf8` readme/item | um decode no CPU task | **RESOLVED** |
| **SCRAPE-Q-005** | Média | doctor sem web fetch posture | check `web_fetch_posture` | **RESOLVED** |
| **SCRAPE-Q-006** | Média | ADR 0003 “OOS robots” suave | **PROIBIDO** robots EN+pt-BR | **RESOLVED** |
| **SCRAPE-Q-007** | Baixa | module docs drift | `docs_rs`/`http` docs alinhados | **RESOLVED** |
| **SCRAPE-Q-008** | Alta | sem teste stdlib/mock hit host | unit `hit_urls_follow_source_url_host_stdlib_and_mock` | **RESOLVED** |
| **SCRAPE-Q-009** | Média | `body.clone` + re-decode version | move body; scrape no mesmo worker | **RESOLVED** |
| **HYG-Q-001** | Processo | gaps/CHANGELOG | §18 + Unreleased | **RESOLVED** |
| robots/sitemap/encoding_rs/headless… | Info | N/A product class | ADR 0003 | **N/A (doc)** |

### 18.3 Evidência

```text
docs/decisions/0003-web-fetch-scope.md (+ pt-BR) → PROIBIDO robots; source_url join
src/docs_rs/search.rs  → parse_all_html_hits(base); sem HOST_DOCS_RS
src/docs_rs/html.rs    → version scrape scraper-only; sanitize cleanup
src/docs_rs/fetch.rs   → single decode + version no CPU task
src/doctor.rs          → web_fetch_posture
```

### 18.4 Validação

```text
cargo test --locked → OK
cargo clippy --all-targets --locked -- -D warnings → OK
search-in-crate std Option → hit URLs em doc.rust-lang.org
doctor --json → web_fetch_posture ok
rg 'HOST_DOCS_RS' src/docs_rs/search.rs → vazio
rg 'compile_bounded_regex' src/docs_rs/html.rs → só hygiene LazyLock
```

---

## 19. Camada R — Residual re-auditoria web scraping (2026-07-19)

> Origem: `/r-auditoria` 3ª passagem (mesmas Rules + **PROIBIDO robots**)  
> + one-shot · memória · paralelismo · XDG · multi-OS  
> Tools: context7, duckduckgo-search-cli, docsrs-cli dogfood (source×artefato).

### 19.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — source vs release + live test gap |
| O que faltava pós-Q? | Artefato stale; G-18 sem assert de host dos hits; selectors re-parse |
| robots.txt? | **PROIBIDO** (inalterado) |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (SCRAPE-R-*) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A ADR 0003 |
| Monólito split? | **Não** — testes ≥800 = OPP |
| CI / telemetria / env knobs? | **Proibido** |

### 19.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **SCRAPE-R-001** | Alta | release bin stale → dogfood std hits em docs.rs | rebuild release + dogfood `--no-cache` | **RESOLVED** |
| **SCRAPE-R-002** | Baixa | skills “without scraping HTML” ambíguo | description EN/PT: CLI extrai; agente não raspa regex | **RESOLVED** |
| **SCRAPE-R-003** | Alta | G-18 só checava `source_url`/nome | assert cada `hits[].url` em `doc.rust-lang.org` | **RESOLVED** |
| **SCRAPE-R-004** | Média | `Selector::parse` a cada HTML path | `LazyLock<Selector>` fixos em html/search | **RESOLVED** |
| **SCRAPE-R-005** | Baixa | erros `"docs.rs …"` em paths stdlib | labels `"rustdoc …"` host-agnósticos | **RESOLVED** |
| **HYG-R-001** | Processo | sem § residual R | §19 + timeline R + CHANGELOG | **RESOLVED** |
| robots/sitemap/encoding_rs/headless… | Info | N/A product class | ADR 0003 | **N/A (doc)** |

### 19.3 Evidência

```text
src/docs_rs/html.rs    → LazyLock selectors (SCRAPE-R-004)
src/docs_rs/search.rs  → SEL_MAIN_CONTENT_A; label rustdoc all.html
src/docs_rs/fetch.rs   → labels rustdoc readme/get-item
tests/network_live.rs  → G-18 hit host asserts (SCRAPE-R-003)
skills/*               → description CLI structured extraction
```

### 19.4 Validação

```text
cargo test --locked --lib → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo build --release --locked → OK
./target/release/docsrs-cli doctor --json → web_fetch_posture ok
./target/release/docsrs-cli search-in-crate std Option --limit 3 --json --no-cache
  → hits em doc.rust-lang.org (não docs.rs)
rg 'robotstxt' Cargo.toml src/ → vazio
```

---

## 20. Camada S — Residual re-auditoria web scraping (2026-07-19)

> Origem: `/r-auditoria` 4ª passagem (mesmas Rules + **PROIBIDO robots**)  
> + one-shot · memória · paralelismo · XDG · multi-OS  
> Tools: context7 (scraper/reqwest trust 9.7), duckduckgo-search-cli (brotli+header), docsrs-cli dogfood.

### 20.1 FAQ (síntese)

| Pergunta | Resposta |
|----------|----------|
| Auditoria profunda? | **Sim** — pós-R; poison join; double DOM; method selector |
| O que faltava pós-R? | Absolute off-origin hits; soft-skip; 2× DOM; brotli; constants |
| robots.txt? | **PROIBIDO** (inalterado) |
| context7 / ddg / docsrs-cli? | **Sim** |
| Causa×efeito? | **Sim** (SCRAPE-S-*) |
| Corrige todos gaps? | **Sim** — RESOLVED ou N/A ADR 0003 |
| Monólito split? | **Não** — testes ≥800 = OPP |
| CI / telemetria / env knobs? | **Proibido** |

### 20.2 Gaps

| ID | Sev | Causa → Efeito | Correção | Status |
|----|-----|----------------|----------|--------|
| **SCRAPE-S-001** | Alta | absolute off-host passthrough; `join?` podia abortar search | `resolve_hit_url` same-origin + soft-skip; poison unit | **RESOLVED** |
| **SCRAPE-S-002** | Média | `Selector::parse` dinâmico method + fallback re-parse | `SEL_WITH_ID` + attr eq; fallback `from_document` | **RESOLVED** |
| **SCRAPE-S-003** | Média | extract + version = 2× `Html::parse_document` | `*_from_document`; fetch 1 parse | **RESOLVED** |
| **SCRAPE-S-004** | Baixa–Média | Accept-Encoding só gzip | reqwest `brotli` + `gzip, br`; doctor | **RESOLVED** |
| **SCRAPE-S-005** | Baixa | `RAYON_HIT_THRESHOLD=64` / capacity `min(256)` | constants + `with_capacity(limit)` | **RESOLVED** |
| **SCRAPE-S-006** | Baixa | sanitize N× replace sem early-empty | early-empty path + hygiene only | **RESOLVED** |
| **HYG-S-001** | Processo | sem § residual S | §20 + timeline S + CHANGELOG | **RESOLVED** |
| robots/sitemap/encoding_rs/headless… | Info | N/A product class | ADR 0003 | **N/A (doc)** |

### 20.3 Evidência

```text
src/docs_rs/search.rs   → resolve_hit_url / soft-skip / RAYON from constants
src/docs_rs/html.rs     → SEL_WITH_ID; *_from_document; sanitize early-empty
src/docs_rs/fetch.rs    → single Html::parse_document per body
src/http/client.rs      → Accept-Encoding: gzip, br
Cargo.toml              → reqwest feature brotli
src/doctor.rs           → web_fetch_posture gzip+br; same-origin
docs/decisions/0003-*   → same-origin hits
```

### 20.4 Validação

```text
cargo test --locked --lib → OK
cargo clippy --all-targets --locked -- -D warnings → OK
cargo build --release --locked → OK
doctor --json → gzip+br; hit join=source_url same-origin
search-in-crate std Option → doc.rust-lang.org
rg 'robotstxt' Cargo.toml src/ → vazio
```


---

## 21. Camada T — Auditoria E2E local completa (2026-07-19)

> **Postura desta camada:** **somente inventário** (identificar + documentar).  
> **PROIBIDO nestas entregas:** corrigir código · publish GitHub/crates.io · CI/CD · telemetria remota.  
> **Origem:** pedido de auditoria E2E profunda (todos os comandos/rotas) + GraphRAG rules + tools obrigatórios.  
> **Binário auditado:** `./target/release/docsrs-cli` · **versão:** `1.1.2`  
> **Rules GraphRAG consultadas (produto-aplicáveis):** `rules-rust-cli-one-shot`, `rules-rust-cli-stdin-stdout`, `rules-rust-cli-com-clap`, `rules-rust-storage-xdg-*`, `rules-rust-proibicao-hardcode`, `rules-rust-rede`, `rules-rust-tls`, `rules-rust-retry-com-backoff`, `rules-rust-tratamento-de-erros`, `rules-rust-serializacao-serde`, `rules-rust-sitema-de-tipos`, `rules-rust-logs-com-tracing-e-rotacao`, `rules-rust-multiplataforma-*`, `rules-rust-multi-idioma-*`, `rules-rust-eficiencia-*`, `rules-rust-gerenciamento-memoria`, `rules-rust-paralelismo-*`, `rules-rust-latencia-*`, `rules-rust-encerramento-*`, `rules-rust-docsrs-documentacao-automatica`, `rules-rust-crates-nativas-*`, `rules-rust-codigo-ingles-*` (+ memórias `docsrs-cli-audit-*`).  
> **Tools usados nesta sessão:** `context7` (`library reqwest`, `docs /seanmonstar/reqwest`), `duckduckgo-search-cli` (query rustdoc/all.html), `docsrs-cli` dogfood live, `sqlite3 graphrag.sqlite`, `cargo test/clippy/build/audit`, `atomwrite` (append incremental).

### 21.1 Veredito de compilação e suite (dados, não opinião)

| Check | Resultado | Evidência |
|-------|-----------|-----------|
| `cargo build --release --locked` | **OK** | `Finished release` (~0.19s cache) |
| `cargo test --locked` | **OK** | lib **189** + cli_smoke 40 + e2e_offline 14 + golden 10 + http_integration 32 + lib_dispatch 18 + network_live 9 + signal_term 2 + doctests 4 — **0 failed** |
| `cargo clippy --all-targets --locked -- -D warnings` | **OK** | exit 0 |
| `cargo audit` | **0 vulns reportadas** (scan 274 deps) | advisory-db fetch OK |
| `version --json` | `"version":"1.1.2"`, `msrv":"1.88.0"`, `os":"linux"` | release bin |
| Telemetria remota no produto | **Ausente** | sem OTLP/sentry; `src/telemetry.rs` **não existe** (só menção em comentário `Cargo.toml`) |
| `.github/` / GHA in-tree | **Ausente** | `ls .github` → inexistente |
| Publish nesta sessão | **Não feito** | conforme mandato |

### 21.2 Matriz E2E de comandos (release local)

| Comando / rota | Exit | `ok` | Observação |
|----------------|------|------|------------|
| *(sem subcomando)* `--json` | **64** | false | usage JSON |
| `not-a-command --json` | **64** | false | usage JSON |
| `version --json` | **0** | true | envelope canônico |
| `version --format markdown` | **0** | — | texto `docsrs-cli 1.1.2` |
| `commands --json` | **0** | true | árvore agent-first completa |
| `schema` sem `--cmd` | **64** | false | usage |
| `schema --cmd search-crates` | **0** | true | schema embutido |
| `schema --cmd all` | **64** | false | `unknown schema command 'all'` → **GAP-T-003** |
| `doctor --json` | **0** | true | checks locais ok (platform, rustls ring, domain_types, unsafe, web_fetch…) |
| `doctor --online --json` | **0** | true | `online_crates_io` + `online_docs_rs` resolvem :443 |
| `config path/show/init` | **0** | true | XDG/cli dir; init cria `config.toml` |
| `cache` sem subcomando | **64** | false | exige `clear`\|`stats` |
| `cache path` | **64** | false | subcomando **inexistente** (só clear/stats) → **GAP-T-004** |
| `cache stats` / `cache clear` | **0** | true | stats pós-rede: 12 entries / ~720 KiB |
| `completions` bash/zsh/fish/powershell/elvish | **0** | — | scripts gerados |
| `completions bash --json` | **0** | true | envelope com `script` |
| `search-crates serde --per-page 3` | **0** | true | hits live crates.io |
| `search-crates … --page 2` | **0** | true | paginação |
| `search-crates --page 0` | **65** | false | invalid_input |
| `search-crates --per-page 200` | **65** | false | invalid_input |
| `search-crates ''` | **65** | false | empty query |
| `readme tokio` | **0** | true | resolved_version `1.53.0` |
| `readme clap@4.5.0` | **0** | true | pin versão |
| `readme clap@4.5.0 --crate-version 4.0.0` | **65** | false | conflito `@` vs flag |
| `get-item tokio fn tokio::spawn` | **0** | true | `resolved_item_path=task::spawn` |
| `get-item tokio method Runtime::new` | **0** | true | `resolved_item_path=runtime::Runtime::new` |
| `get-item clap trait clap::Parser` | **0** | true | live |
| `get-item … boguskind …` | **65** | false | unknown item type |
| `search-in-crate reqwest Client --limit 5` | **0** | true | 4 hits; all.html |
| `search-in-crate … --item-type module` | **65** | false | recusa explícita (all.html) |
| `readme std` / `get-item std enum std::option::Option` | **0** | true | stdlib → doc.rust-lang.org |
| `--dry-run readme serde` / `search-crates` | **0** | true | planned_url sem socket |
| `--max-body-bytes 50 readme tokio` | **74** | false | `kind=budget`, `retryable=false` |
| `--max-output-bytes 200 search-crates serde --per-page 50` | **0** | true | `truncated=true`, hits=[] |
| `--json --format markdown version` | **64** | false | format conflict |
| `--lang fr version` | **65** | false | fail-closed lang |
| `--user-agent` com newline | **78** | false | config/UA validation |

**Missão produto (agent-first crates.io + docs.rs one-shot):** **cumprida** nos caminhos felizes e nos fail-closed de validação testados acima.

### 21.3 FAQ honesto (o que faltava / esqueceu / omitiu)

| Pergunta | Resposta factual desta sessão |
|----------|-------------------------------|
| Todos os gaps A–S do `gaps.md` foram re-solucionados agora? | **Não re-implementamos** — mandato era **só inventário**. A–S permanecem no histórico como **RESOLVED** nas camadas anteriores; esta camada **não reabre** IDs fechados sem evidência de regressão E2E. |
| O que falta? | Ver **GAP-T-*** abertos abaixo (política env, UX schema/cache, drift docs, live suite opt-in). |
| O que se esqueceu em auditorias anteriores? | (1) Inventário “**zero aberto**” não revalidava contra mandato absoluto “**proibido env de produto**”; (2) `schema --cmd all` e `cache path` assimetria UX; (3) comentário `Cargo.toml` overflow vs valor real; (4) referência a `src/telemetry.rs` inexistente. |
| O que se omitiu? | Testes `network_live` **não rodam** no `cargo test` default (gate `DOCSRS_CLI_NETWORK_TESTS=1`) — suite “verde” **não prova** rede live; E2E live foi feito **manual** nesta sessão. |
| Usou context7? | **Sim** — `context7 library reqwest`; `context7 docs /seanmonstar/reqwest -q …`. |
| Usou duckduckgo-search-cli? | **Sim** — query `docs.rs rustdoc all.html scrape API 2026` (Chrome/Xvfb). |
| Usou docsrs-cli para crates? | **Sim** — dogfood `search-crates reqwest`, `get-item reqwest struct reqwest::Client`, mais matriz §21.2. |
| Usou rules GraphRAG? | **Sim** — SQL em `graphrag.sqlite` (memories `rules-rust-*` + `docsrs-cli-audit-*` + decisions). |
| Solucionou bugs/gaps/warnings? | **Não** — proibido nesta rodada; apenas documentado. |
| Compilação local sem publish? | **Sim**. |

### 21.4 Gaps abertos (Camada T) — problema × consequências × causa raiz × solução × benefícios × como resolver

> Status de todos os IDs T: **OPEN** (não corrigir nesta entrega).

#### GAP-T-001 — Env de produto ainda lido em runtime (conflito com mandato XDG-only)

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Alta** (política produto / rules `storage_xdg` + mandato usuário “PROIBIDO variáveis de ambiente”) |
| **Problema** | O binário ainda consulta `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`, `DOCSRS_CLI_HOME`, `DOCSRS_CLI_LANG` (`src/config/load.rs`, `src/cache/paths.rs`, `src/i18n.rs`). Evidência: `DOCSRS_CLI_LANG=en doctor --json` → check `lang=en` **sem** `--lang`. |
| **Consequências** | Agentes/CI podem “configurar por env” contornando `config.toml`/flags; viola narrativa XDG-only; doctor rotula path como `cli-or-env` (opacidade). |
| **Causa raiz (5 porquês)** | 1) Lang/paths mudam via env → 2) código tem ramos `std::env::var(_os)` → 3) sandbox/testes e “ergonomia shell” pediram env → 4) política documentou “knobs não vêm de env” mas **manteve allowlist de path/lang** → 5) **não há enforcement único “só CLI+TOML” para 100% dos knobs de produto**. |
| **Ishikawa (software)** | **Código:** ramos env explícitos. **Configuração:** PathSource::CliOrEnv funde flag+env. **Processo:** testes usam env gates (`DOCSRS_CLI_NETWORK_TESTS`) misturando política de **suite** com política de **produto**. **Dados:** n/a. |
| **Solução (proposta — não implementar agora)** | Remover leituras de `DOCSRS_CLI_*` do **runtime de produto**; paths só via `--config-dir`/`--cache-dir` + XDG `ProjectDirs`; lang só `--lang` + TOML + locale SO; documentar em ADR que env de **teste** (`DOCSRS_CLI_NETWORK_TESTS`) fica **só em `tests/`**. |
| **Benefícios** | Alinha rules + mandato; auditorias deixam de “ACEITAR” env como feature; superfície de config auditável. |
| **Como resolver** | (1) Grep `DOCSRS_CLI_` em `src/`; (2) apagar ramos; (3) atualizar template TOML/doctor labels (`cli` vs `xdg`); (4) migrar testes para flags; (5) `cargo test` + E2E path. |

#### GAP-T-002 — `PathSource` / doctor: token `cli-or-env` opaco

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Média** (observabilidade / honestidade de camada) |
| **Problema** | `PathSource::CliOrEnv.as_str() == "cli-or-env"` — não distingue `--config-dir` de `DOCSRS_CLI_CONFIG_DIR`. |
| **Consequências** | Doctor/config path não permitem provar em JSON se ganhou flag ou env; atrapalha auditorias XDG. |
| **Causa raiz** | Enum colapsou duas origens numa variante para simplicidade na v1. |
| **Solução** | Variantes separadas `CliFlag` / `EnvOverride` (ou remover Env — ver T-001). |
| **Benefícios** | Diagnóstico preciso; fecha narrativa “sem env”. |
| **Como resolver** | Expandir enum + testes de `resolve_*_with_source` + schemas config se aplicável. |

#### GAP-T-003 — `schema --cmd all` inexistente

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** (DX agentes) |
| **Problema** | `schema --cmd all` → exit **64** `unknown schema command 'all'`. Agentes que pedem “todos os schemas” falham. |
| **Consequências** | N+1 chamadas `schema --cmd <cada>`; docs/skills podem sugerir `all` incorretamente. |
| **Causa raiz** | Superfície schema foi por comando embutido; alias `all` nunca especificado no clap/enum de cmds. |
| **Solução** | Aceitar `all` (array de schemas) **ou** documentar lista canônica em `commands`/`schema` help e skills. |
| **Benefícios** | Descoberta agent-first em um shot. |
| **Como resolver** | Estender match de `--cmd` + schema envelope + teste e2e + skill SKILL.md. |

#### GAP-T-004 — `cache path` ausente (assimetria com `config path`)

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** (UX / simetria meta-cmds) |
| **Problema** | `cache path` → exit **64** unrecognized; só `cache stats` / `cache clear`. `config path` existe. |
| **Consequências** | Agente precisa de `cache stats` (traz `root`) ou `config path` (cache_dir) — contrato fragmentado. |
| **Causa raiz** | Meta cache focado em ops destrutivas/stats; path ficou só no ramo config. |
| **Solução** | Adicionar `cache path` espelhando campos de stats.root + source **ou** documentar canônico `config path` para cache_dir. |
| **Benefícios** | Superfície previsível. |
| **Como resolver** | Subcommand + envelope + schema cache + teste smoke. |

#### GAP-T-005 — Live network tests opt-in (falso “verde” de rede)

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Média** (processo de qualidade; **não** é CI proibido — é harness local) |
| **Problema** | `tests/network_live.rs` early-return se `DOCSRS_CLI_NETWORK_TESTS` ≠ 1; `cargo test` reporta 9 testes **ok em 0.00s** sem abrir sockets. |
| **Consequências** | Regressões de HTML/API live (docs.rs/crates.io) passam despercebidas no gate local default. |
| **Causa raiz** | Política “default suite never opens external sockets” (correto para determinismo) **sem** checklist humano obrigatório de smoke live pós-mudança scrape. |
| **Solução** | Manter opt-in (bom); adicionar `scripts/smoke-live.sh` (já existe) como passo **documentado obrigatório** em auditorias/AGENTS; opcionalmente imprimir `ignored` em vez de pass silencioso. |
| **Benefícios** | Honestidade de cobertura; menos “suite verde = prod OK”. |
| **Como resolver** | `#[ignore]` + `cargo test -- --ignored` **ou** contador skip em stderr; atualizar `docs/TESTING.md` + gaps checklist. |

#### GAP-T-006 — Drift documental: `src/telemetry.rs` citado mas ausente

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** (higiene) |
| **Problema** | `Cargo.toml` comenta “See `src/telemetry.rs` for OOS notes” mas o arquivo **não existe**. |
| **Consequências** | Leitor/agente procura módulo morto; confusão se telemetria foi removida ou nunca criada. |
| **Causa raiz** | Extração/rename de diagnostics sem atualizar comentário de deps. |
| **Solução** | Apontar comentário para `src/diagnostics.rs` (ou ADR OOS) e remover path fantasma. |
| **Benefícios** | Docs ↔ tree consistentes. |
| **Como resolver** | Edit `Cargo.toml` comment only. |

#### GAP-T-007 — Drift comentário `overflow-checks` no profile release

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** |
| **Problema** | Bloco de comentários históricos no `Cargo.toml` vs valor atual `overflow-checks = true` (já alinhado à camada G) — risco de reintrodução se alguém “seguir o comentário antigo” em outro branch/doc. |
| **Consequências** | Baixa se o valor `true` permanece; confusão em reviews. |
| **Causa raiz** | Comentários multi-camada acumulados sem poda. |
| **Solução** | Um único parágrafo canônico: overflow-checks **on** por SEC-008. |
| **Benefícios** | Menos drift. |
| **Como resolver** | Poda de comentário em `Cargo.toml`. |

#### GAP-T-008 — Feature clap `env` habilitada sem knobs de produto via `#[arg(env)]`

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info / Baixa** |
| **Problema** | `clap` features incluem `"env"`; superfície CLI atual não mapeia knobs de produto a env vars clap (grep `env =` em args vazio). Feature amplia superfície e sugere env-first. |
| **Consequências** | Risco futuro de `#[arg(env = "…")]` acidental; ruído em review de política anti-env. |
| **Causa raiz** | Template de features clap “completo” na bootstrap. |
| **Solução** | Remover feature `env` do clap **se** nenhum arg a usa; ou documentar “feature residual / não usar”. |
| **Benefícios** | Fail-closed de política. |
| **Como resolver** | `Cargo.toml` features clap − `env` + `cargo test`. |

#### GAP-T-009 — Progresso humano em stderr durante ops longas (ruído agente)

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** (contrato já diz stderr=diagnostics) |
| **Problema** | E2E `get-item … Runtime::new` emitiu stderr `buscando Runtime::new...` mesmo com `--json` (stdout limpo). |
| **Consequências** | Em geral OK; com `-q` deve suprimir — validar se todos os progress paths honram quiet (não esgotado em todos os ramos nesta sessão). |
| **Causa raiz** | ProgressGuard para UX humana em scrape multi-step. |
| **Solução** | Garantir quiet em **todos** os progress; opcional JSON-only mode silence. |
| **Benefícios** | Logs de agente mais limpos. |
| **Como resolver** | Teste e2e `--json -q` sem linhas progress; grep ProgressGuard. |

#### GAP-T-010 — Dependência reqwest produto `0.12` vs crates.io `reqwest 0.13.x` (dogfood)

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** (oportunidade; não bug funcional) |
| **Problema** | Dogfood `search-crates reqwest` reporta `0.13.4` no índice; produto pinado em `reqwest 0.12` + rustls path ADR 0007. |
| **Consequências** | Divergência de API/features futuras; não quebra 1.1.2. |
| **Causa raiz** | Conservadorismo + posture rustls-ring validada em 0.12. |
| **Solução** | Avaliar upgrade controlado 0.13 com revalidação TLS/deny/http2 (fora desta entrega). |
| **Benefícios** | Manutenção long-term. |
| **Como resolver** | Spike branch + `cargo test` + doctor posture + deny.toml. |

### 21.5 O que **não** é gap (validado E2E — não reabrir A–S)

| Tema | Evidência Camada T |
|------|-------------------|
| R1 method path curto | `Runtime::new` → `resolved_item_path=runtime::Runtime::new` exit 0 |
| R2 `crate@ver` | `readme clap@4.5.0` exit 0; conflito versões exit 65 |
| Budget body / non-retryable | exit 74 `kind=budget` |
| Budget output truncate | `truncated=true` hits vazios exit 0 |
| rustls ring min TLS 1.2 | doctor `http_client_posture` |
| Sem GHA / sem telemetria remota | tree + grep |
| One-shot BORN→DIE | `commands` agent_notes lifecycle |
| Fail-closed page/per_page/lang/UA | exits 65/78 |
| Stdlib host | readme/get-item std OK |
| Cache integrity pós-rede | stats entries>0 após fetches |

### 21.6 Análise de causa raiz — síntese (efeito global “política vs inventário zero”)

```
EFEITO: gaps.md A–S declaravam inventário aberto = 0, mas mandato
        absoluto "sem env de produto" ainda é violado por DOCSRS_CLI_*.
         │
    Por quê 1? Auditorias fecharam knobs TOML/CLI e documentaram
               "path sandbox env permitido".
         │
    Por quê 2? Testes e ergonomia shell dependiam de DOCSRS_CLI_HOME/LANG.
         │
    Por quê 3? PathSource colapsou CLI+env; doctor não denuncia env.
         │
    Por quê 4? Não havia teste de política negativa:
               "assert product binary ignores DOCSRS_CLI_LANG when unset flags".
         │
    CAUSA RAIZ: ausência de enforcement automatizado da política XDG-only
                no binário de produto (só documentação parcial).
```

**Contra-medidas (plano — não executar correção agora):**

1. **Bloquear recorrência:** teste de regressão política: com `DOCSRS_CLI_LANG` set e sem `--lang`/TOML, locale deve vir do SO (ou en), **não** do env — após remoção do ramo.  
2. **Eliminar causa:** remover `std::env::var*` de knobs de produto em `src/` (T-001).  
3. **Detectar cedo:** `rg 'DOCSRS_CLI_' src/` em checklist de auditoria + doctor check `env_product_knobs=disabled`.  
4. **Separar suite:** gates de teste live só em `tests/**`, nunca lidos pelo lib de produto.

### 21.7 FTA (evento topo: “agente não confia no contrato XDG”)

```
[Agente vê config divergente / path misterioso]
              OR
   ┌──────────┼──────────┐
   │          │          │
 env LANG  env CONFIG  label cli-or-env
 ainda lido  ainda lido  opaco (T-002)
 (T-001)     (T-001)
```

### 21.8 Oportunidades de melhoria (não bloqueantes; ≠ bug)

1. Alias `schema --cmd all` (T-003) ou bundle offline dos JSON em `docs/schemas/`.  
2. `cache path` (T-004) para simetria meta.  
3. `#[ignore]` honesto em network_live (T-005).  
4. Poda comentários Cargo.toml (T-006/T-007).  
5. Drop feature clap `env` (T-008).  
6. Avaliar reqwest 0.13 (T-010).  
7. Clippy pedantic seletivo (já citado em camadas I/J — mantido OPP).  
8. Garantir `-q` silencia 100% progress (T-009).  
9. Atualizar veredito global §0 timeline com linha **T** (este append).  
10. Skills `docsrs-cli-*`: documentar exits 64/65/74 e ausência de `cache path` / `schema all`.

### 21.9 Checklist tools (Camada T)

| Tool | Usado? | Evidência |
|------|--------|-----------|
| GraphRAG (`graphrag.sqlite`) | **Sim** | listagem memories rules-rust + docsrs-cli-audit |
| context7 | **Sim** | library + docs reqwest |
| duckduckgo-search-cli | **Sim** | SERP rustdoc/all.html (Chrome) |
| docsrs-cli (dogfood) | **Sim** | matriz §21.2 + search/get-item reqwest |
| atomwrite | **Sim** | append incremental desta §21 |
| cargo test / clippy / build / audit | **Sim** | §21.1 |

### 21.10 Inventário Camada T (status)

| ID | Sev | Status |
|----|-----|--------|
| GAP-T-001 | Alta | **OPEN** |
| GAP-T-002 | Média | **OPEN** |
| GAP-T-003 | Baixa | **OPEN** |
| GAP-T-004 | Baixa | **OPEN** |
| GAP-T-005 | Média | **OPEN** (processo) |
| GAP-T-006 | Baixa | **OPEN** |
| GAP-T-007 | Info | **OPEN** |
| GAP-T-008 | Info | **OPEN** |
| GAP-T-009 | Info | **OPEN** |
| GAP-T-010 | Info | **OPEN** |

**Contagem aberta introduzida por T:** **10** (nenhum fechado nesta entrega — mandato inventário-only).

### 21.11 Atualização de timeline (incremental — não apaga A–S)

| Camada | Data | Escopo | Aberto |
|--------|------|--------|--------|
| … A–S | 2026-07-19 e antes | (preservado acima) | 0 *histórico* |
| **T — E2E local full** | **2026-07-19** | Auditoria e2e todos os cmds + política env/XDG + UX meta + drift docs | **10 OPEN** |

> Nota: o veredito “Inventário aberto (A–S) = zero” nas seções superiores **permanece como histórico das camadas A–S**. A partir de **T**, o inventário global de **abertos** passa a incluir **GAP-T-001…010** até correção futura.

### 21.12 Evidência de sessão (comandos-chave)

```text
cargo build --release --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo audit
./target/release/docsrs-cli version --json
./target/release/docsrs-cli doctor --online --json
./target/release/docsrs-cli search-crates serde --per-page 3 --json
./target/release/docsrs-cli readme tokio --json
./target/release/docsrs-cli get-item tokio fn tokio::spawn --json
./target/release/docsrs-cli get-item tokio method Runtime::new --json
./target/release/docsrs-cli search-in-crate reqwest Client --limit 5 --json
./target/release/docsrs-cli readme std --json
./target/release/docsrs-cli --max-body-bytes 50 readme tokio --json   # 74 budget
./target/release/docsrs-cli schema --cmd all --json                   # 64 GAP-T-003
./target/release/docsrs-cli cache path --json                         # 64 GAP-T-004
DOCSRS_CLI_LANG=en ./target/release/docsrs-cli doctor --json          # lang=en GAP-T-001
context7 library reqwest
context7 docs /seanmonstar/reqwest -q "ClientBuilder rustls timeout"
duckduckgo-search-cli "docs.rs rustdoc all.html scrape API 2026" -n 5
sqlite3 graphrag.sqlite "SELECT name FROM memories WHERE name LIKE 'rules-rust-%' …"
```

---

**Fim do append Camada T (2026-07-19).** Próximas auditorias: acrescentar **Camada U+** no final — **proibido** reescrever A–T.

---

## 22. Camada U — Fechamento GAP-T + residual + release **v1.1.3** (2026-07-19)

> **Política:** append-only. Conteúdo A–T **preservado**. Esta seção fecha inventário aberto.

### 22.1 Escopo

Implementação completa dos GAP-T-001…010 e residual U-001…U-007 descobertos na auditoria v2. Release **1.1.3** (semver após 1.1.2; **não** rebaixado para 1.1.1).

### 22.2 Status GAP-T (todos RESOLVED)

| ID | Resolução |
|----|-----------|
| GAP-T-001 | Removidas leituras `DOCSRS_CLI_{LANG,HOME,CONFIG_DIR,CACHE_DIR}` de `src/`; paths = CLI + XDG; lang = CLI/TOML + SO |
| GAP-T-002 | `PathSource::{CliFlag,Xdg,Unresolved}` tokens `cli`/`xdg`/`unresolved` |
| GAP-T-003 | `schema --cmd all` bundle determinístico |
| GAP-T-004 | `cache path` + schema `cache-path` |
| GAP-T-005 | `tests/network_live.rs` com `#[ignore]`; TESTING.md com `-- --ignored` |
| GAP-T-006 | Cargo.toml → `src/diagnostics.rs` |
| GAP-T-007 | Comentário overflow-checks canônico SEC-008 |
| GAP-T-008 | Feature clap `env` removida |
| GAP-T-009 | ProgressGuard honra quiet; unit tests; ops passam `ctx.cli.quiet` |
| GAP-T-010 | reqwest **0.13.4** + `rustls-no-provider` + ring provider process-wide |

### 22.3 Residual U (RESOLVED)

| ID | Resolução |
|----|-----------|
| U-001 | doctor dotenv detail sem “env allowlist” |
| U-002 | `config.schema.json` enums `cli`/`xdg`/`unresolved` |
| U-003 | README, skills, TESTING, MIGRATION, COOKBOOK atualizados (flags, não env produto) |
| U-004 | constants comments “CLI/TOML cannot raise” |
| U-005 | `apply_env` → `apply_cache_path_defaults` |
| U-006 | `schema_command_names()` + `schema_json_for_cmd` DRY |
| U-007 | Dogfood com `./target/release/docsrs-cli` |

### 22.4 Inventário aberto global

| Camada | Aberto |
|--------|--------|
| A–S | 0 (histórico) |
| T | **0** (fechados nesta camada U) |
| U residual | **0** |
| **Total OPEN** | **0** |

### 22.5 Evidência de verificação

```text
cargo test --locked
cargo clippy -D warnings
cargo deny check bans
./target/release/docsrs-cli version --json
./target/release/docsrs-cli schema --cmd all --json
./target/release/docsrs-cli --cache-dir /tmp/x cache path --json
```

### 22.6 Tools (Camada U)

GraphRAG, context7, duckduckgo-search-cli, docsrs-cli dogfood, atomwrite — todos usados.

**Fim do append Camada U (2026-07-19).** Próximas auditorias: Camada V+ no final — **proibido** reescrever A–U.

---

## 23. Camada V — Residual pós-U + cancel + path_source + docs/CLAUDE purge → **v1.1.4** (2026-07-19)

> **Política:** append-only. Conteúdo A–U **preservado**. Esta seção fecha residual V e re-dogfood de T.

### 23.1 Escopo

- Purga documental bilíngue + ADRs + skills + **CLAUDE.md** (path sandbox env morto).
- Cancel cooperativo no fan-out CPU (`rayon` / sequential) de `search-in-crate`.
- SRP: `src/config/path_source.rs` (`PathSource` + resolve).
- Memória: `Vec::with_capacity` no filtro sequencial.
- Release **1.1.4** + `scripts/check-policy.sh` (local, sem GHA).

### 23.2 Status GAP-V (todos RESOLVED)

| ID | Resolução |
|----|-----------|
| GAP-V-001 | Docs AGENTS/HOW_TO_USE/CROSS_PLATFORM/TESTING/MIGRATION (+ pt-BR): paths só flags + ProjectDirs |
| GAP-V-002 | ADRs 0002/0007/0009: purga env path histórica U/1.1.3 |
| GAP-V-003 | `CancelFlag` em `parse_all_html_hits` / filters; wired via `HttpClient::cancel_flag` + spawn_blocking |
| GAP-V-004 | Skills NEVER completo (LANG/HOME/paths) + `schema --cmd all` + `cache path` |
| GAP-V-005 | AGENTS UA default `docsrs-cli/<version>` |
| GAP-V-006 | `path_source.rs` extraído; meta/render monólitos mantidos (coesos; OPP split se crescerem) |
| GAP-V-007 | DRY: PathSource único em path_source; schema_command_names inalterado |
| GAP-V-008 | Sequential filter `with_capacity(candidates.len())` |
| GAP-V-009 | Comentários load clamp: CLI/TOML only |
| GAP-V-010 | Tests smoke/integration **OPP** (não tocados; suite verde) |
| GAP-V-011 | CLAUDE.md: flags em vez de `DOCSRS_CLI_HOME`/`LANG` |
| GAP-V-012 | `commands` schema about documenta `--cmd all` |

### 23.3 Re-dogfood T (ainda RESOLVED)

| Check | Resultado 1.1.4 |
|-------|-----------------|
| `DOCSRS_CLI_LANG=en doctor` | `lang=pt-BR` (SO) |
| `schema --cmd all` | ok, 19 schemas |
| `cache path` | `source=cli` |
| `cargo test --locked` | verde; 9 live ignored |
| `cargo clippy -D warnings` | limpo |
| `cargo deny check bans` | ok |
| Sem GHA / sem telemetria | ok |

### 23.4 Inventário aberto global

| Camada | Aberto |
|--------|--------|
| A–S | 0 (histórico) |
| T | 0 (U) |
| U residual | 0 |
| V | **0** |
| **Total OPEN** | **0** |

### 23.5 Tools (Camada V)

GraphRAG rules-rust (one-shot, memória, paralelismo, storage-xdg), context7, duckduckgo-search-cli, docsrs-cli dogfood, atomwrite — todos usados.

### 23.6 OPP documentado (não bug)

- Split adicional `meta_cmds.rs` / `render.rs` / tests grandes se LOC crescer.
- Clippy pedantic seletivo.
- Rayon ThreadPoolBuilder amarrado a concurrency budget.

**Fim do append Camada V (2026-07-19).** Próximas auditorias: Camada W+ no final — **proibido** reescrever A–V.
---

## 24. Camada W — Auditoria E2E inventário-only pós-v1.1.4 (2026-07-19)

> **Política desta entrega:** **somente inventário**. **PROIBIDO** implementar correções nesta camada.
> **Append-only:** A–V **preservados** (histórico). Conteúdo anterior **não** foi apagado/reescrito.
> **Binário sob teste:** `./target/release/docsrs-cli` **1.1.4** (workspace `Cargo.toml`).
> **Nota:** binário em `PATH` (`~/.cargo/bin/docsrs-cli`) ainda reporta **1.1.0** — drift de instalação local, não do tree (ver GAP-W-005).

### 24.1 Escopo e mandato

| Item | Ação |
|------|------|
| Compilação local release | `cargo build --release --locked` |
| Suite | `cargo test --locked` (incl. 9 `network_live` **ignored**) |
| Clippy | `cargo clippy --all-targets --locked -- -D warnings` |
| Policy | `scripts/check-policy.sh` exit 0 |
| Deny | `cargo deny check bans` ok |
| Audit | `cargo audit --no-fetch` (scan 278 crates; sem advisory reportada no output) |
| E2E matriz | Todos os subcomandos + fail-closed + budget + env-ignore + dry-run + completions |
| Tools | GraphRAG `rules-rust-*` + `docs_rules/*` · context7 · duckduckgo-search-cli · docsrs-cli dogfood · atomwrite |
| Correção de bugs | **NÃO** (mandato usuário inventário-only) |

### 24.2 Gates e dogfood (evidência)

| Check | Resultado |
|-------|-----------|
| `version --json` (tree) | `1.1.4` exit 0 |
| `commands --json` | árvore completa; cache path/clear/stats; config path/show/init; schema about menciona `--cmd all` |
| `doctor --json` / `--online` | ok; posture rustls ring TLS≥1.2; `dotenv_runtime=disabled`; sources `xdg`/`cli` |
| `search-crates serde --per-page 3` | ok hits=3 |
| `readme tokio` / `clap@4.5.0` / `std` | ok |
| `get-item tokio fn tokio::spawn` | ok `resolved_item_path=task::spawn` |
| `get-item tokio method Runtime::new` | ok `extraction=method` md≈1013 |
| `get-item reqwest struct reqwest::Client` | ok |
| `search-in-crate reqwest Client --limit 5` | ok hits=4 |
| match `exact\|prefix\|substring` | ok; `fuzzy` → 65 fail-closed |
| `--max-body-bytes 50 readme tokio` | exit **74** `kind=budget` |
| `--max-output-bytes 200 search-crates …` | ok `truncated=true` hits=0 |
| `--page 0` / `--lang xx` / version conflict | exit **65** |
| crate inexistente | exit **66** `not_found` |
| `schema --cmd all` | ok mode=all **19** items |
| `cache path` / `cache stats` / `cache clear` | ok |
| `config path\|show\|init\|--force` | ok; 2º init sem force → 78 |
| `DOCSRS_CLI_LANG=en doctor` | `lang=pt-BR` (SO) — **env produto ignorado** |
| `DOCSRS_CLI_CONFIG_DIR=… doctor` | `config_source=xdg` path real XDG — **env ignorado** |
| `-q` progress | stderr 0 bytes em get-item |
| completions bash/zsh/fish/powershell/elvish | exit 0 |
| dry-run 4 cmds de rede | planned_url only |
| origin TOML `http://evil.example` | exit **78** allowlist |
| telemetria / `.github` | ausentes |
| `rg DOCSRS_CLI_ src/` | só comentários de política (sem leitura runtime) |
| `clap` features | sem feature `env` |
| reqwest produto | **0.13.4** (dogfood search-crates) |

### 24.3 Revalidação inventário histórico T→V

| ID histórico | Status em 1.1.4 (re-dogfood W) |
|--------------|--------------------------------|
| GAP-T-001…010 | **RESOLVED** (U) — reconfirmado |
| GAP-U residual | **RESOLVED** |
| GAP-V-001…012 | **RESOLVED** (V) — reconfirmado |
| Claims “OPEN=0” em §22–§23 | **Históricos** das camadas U/V; **não** apagam gaps novos de W |

### 24.4 Respostas da auditoria (checklist usuário)

| Pergunta | Resposta (Camada W) |
|----------|---------------------|
| Todos os gaps de `gaps.md` foram solucionados? | **Não para o inventário global vivo.** A–V fechados no histórico; **W introduz abertos** abaixo. |
| O que falta? | Fechar GAP-W-* (método falso-positivo, schemas offline, docs envelope, i18n comment, install drift). |
| O que se esqueceu em auditorias anteriores? | (1) Fallback `item_page` em method ausente tratado como **feature** com teste unitário, **sem** gate E2E de “typo → not_found”; (2) assimetria 19 schemas runtime vs 13 arquivos em `docs/schemas/`; (3) README schemas afirma `command`+`duration_ms` em todo envelope, mas error wire omite. |
| O que se omitiu? | Correção de código (proibida nesta entrega). Smoke live `--ignored` não reexecutado (opt-in documentado). |
| Quais são os gaps? | **GAP-W-001…010** (tabela §24.5 / detalhe §24.6). |
| Oportunidades de melhoria? | §24.8 OPP-W-*. |
| context7-cli? | **Sim** — `context7 library/docs` reqwest + clap. |
| duckduckgo-search-cli? | **Sim** — SERP “docs.rs rustdoc all.html scrape API 2026”. |
| docsrs-cli dogfood crates? | **Sim** — matriz §24.2 + search reqwest 0.13.4. |
| rules GraphRAG? | **Sim** — memories `rules-rust-*` (body) + `docs_rules/rules_rust_*.md` (one-shot, stdin/stdout, clap, XDG, hardcode, retry, TLS, rede, i18n, multiplataforma, etc.). |
| Erros/bugs/gaps/warnings todos solucionados? | **Não** — inventário-only; **OPEN > 0** após W. Clippy/test build **sem warnings** de compilação. |

### 24.5 Inventário Camada W (status)

| ID | Sev | Status | Tema |
|----|-----|--------|------|
| GAP-W-001 | **Alta** | **OPEN** | `get-item method` typo → sucesso falso (`extraction=item_page`) |
| GAP-W-002 | **Média** | **OPEN** | Drift `docs/schemas/*.json` (13) vs `schema --cmd all` (19) |
| GAP-W-003 | **Média** | **OPEN** | Docs/README schemas vs error envelope (sem `command`/`duration_ms`) |
| GAP-W-004 | **Baixa** | **OPEN** | Comentário PT em `src/retry.rs` (code_english) |
| GAP-W-005 | **Baixa** | **OPEN** (processo/local) | PATH `docsrs-cli` 1.1.0 ≠ tree 1.1.4 |
| GAP-W-006 | **Info** | **OPEN** (processo) | Working tree massivamente dirty pós-U/V sem commit |
| GAP-W-007 | **Baixa** | **OPEN** | `--page-token` lixo → `kind=invalid_input` + msg “bad request from remote” |
| GAP-W-008 | **Info** | **OPEN** | Header `gaps.md` §0 ainda diz versão **1.1.2** (drift documental interno; append-only não reescreve §0) |
| GAP-W-009 | **Info** | **OPEN** | Agentes podem ignorar `extraction=item_page` (sem skill “fail if not method”) |
| GAP-W-010 | **Info** | **OPEN** | `cargo audit` hang intermitente em “Updating crates.io index” (tooling host) |

**Contagem aberta introduzida por W:** **10 OPEN** (0 fechados nesta entrega — mandato inventário-only).

### 24.6 Gaps abertos — problema × consequências × causa raiz × solução × benefícios × como resolver

> Status de todos os IDs W: **OPEN** (**não** corrigir nesta entrega).

#### GAP-W-001 — `get-item method` com âncora inexistente retorna ok com página pai

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Alta** (correção semântica agent-facing) |
| **Problema** | `get-item tokio method Runtime::neww --json` → exit **0**, `ok=true`, `extraction=item_page`, markdown ≈30kB da **struct Runtime**, `title=runtime::Runtime::neww (method)`, `source_url=…#method.neww`. Comparar: `Runtime::new` → `extraction=method`, md≈1013. |
| **Consequências** | Agente consome doc errada como se fosse o método; alucina API; falha silenciosa de contrato “item pedido”. |
| **Causa raiz (5 porquês)** | 1) HTML 200 da página pai + fragmento morto → 2) `extract_method_markdown_scoped*` faz fallback `item_page` quando âncora falta → 3) teste unitário **codifica** o fallback (`method_missing_anchor_falls_back_item_page`) → 4) fetch monta sucesso sempre que HTTP≠404 → 5) **não há fail-closed “method requested ∧ extraction≠method ⇒ not_found”** no caminho de produto. |
| **Ishikawa** | **Código:** fallback silencioso. **Dados:** rustdoc serve página tipo mesmo com `#method.typo`. **Medição:** suite unitária “verde” no fallback; sem E2E “typo method → 66”. **Processo:** auditorias T–V focaram env/schema meta, não âncora method. **Config:** n/a. |
| **Solução (proposta — não implementar agora)** | Fail-closed: se `item_type=method` e scope≠`method` → `ErrorKind::NotFound` (+ `--suggest` opcional). Alternativa: manter fallback mas exigir que agentes rejeitem `extraction=item_page`. Preferir not_found no produto. |
| **Benefícios** | Contrato honesto; menos alucinação; alinha “typed item” com existência real da âncora. |
| **Como resolver** | (1) E2E repro `Runtime::neww`; (2) mudar extract/ops; (3) atualizar unit test; (4) skill AGENTS: checar `extraction`; (5) `cargo test` + dogfood. |

#### GAP-W-002 — Schemas offline incompletos vs runtime bundle

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Média** (DX agentes offline / CI docs) |
| **Problema** | `schema --cmd all` emite **19** cmds; `docs/schemas/*.json` tem **13**. Faltam no disco: `cache-clear`, `cache-path`, `cache-stats`, `config-init`, `config-path`, `config-show` (runtime expõe nomes wire; disco consolida em `cache`/`config`). |
| **Consequências** | Agentes que leem só o tree de arquivos falham em achar schema por subcomando; drift com `schema --cmd cache-path`. |
| **Causa raiz** | Inventário de arquivos consolidou path/show/init em um JSON, mas o enum de schemas do binário expõe aliases por command wire name. |
| **Solução** | Exportar 6 JSON espelho **ou** documentar canonicamente aliases no README e teste de paridade. |
| **Benefícios** | Descoberta 1:1 offline ↔ runtime. |
| **Como resolver** | Script gerador a partir de `schema --cmd all` **ou** docs README inventory + teste que diff runtime vs disk. |

#### GAP-W-003 — Documentação do envelope vs wire de erro

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Média** (contrato / honestidade docs) |
| **Problema** | `docs/schemas/README.md` (EN+pt-BR): “Outer envelope **always** carries `schema_version`, `ok`, `command`, and `duration_ms`”. Wire de erro real: `{"schema_version", "ok", "error"}` **sem** `command`/`duration_ms`. `error.schema.json` alinha com o wire (required só esses 3). |
| **Consequências** | Parser de agente que exige `command` em todo JSON quebra em falhas; docs mentem sobre universalidade. |
| **Causa raiz** | README generalizou o envelope de **sucesso**; error model (ADR 0002) é envelope distinto e não foi refletido na frase “always”. |
| **Solução** | Corrigir README: sucesso sempre tem command+duration_ms; erro é schema separado sem esses campos **ou** adicionar campos opcionais no emit de erro. |
| **Benefícios** | Contrato legível; menos bugs de cliente. |
| **Como resolver** | Edit README bilíngue + teste snapshot error keys; opcional estender emit. |

#### GAP-W-004 — Português em doc-comment de módulo de produto

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** (rules `codigo_ingles`) |
| **Problema** | `src/retry.rs` module docs: `Relógio monotônico: sleeps usam … (não SystemTime)`. |
| **Consequências** | Viola mandate “código/comentários em inglês”; ruído em review multi-idioma. |
| **Causa raiz** | Comentário residual de sessão bilíngue sem lint de idioma. |
| **Solução** | Reescrever a linha em inglês técnico. |
| **Benefícios** | Conformidade rules. |
| **Como resolver** | atomwrite one-line + grep acentos em `src/**/*.rs` excl. `i18n.rs`. |

#### GAP-W-005 — Drift versão PATH vs tree

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** (processo local / dogfood) |
| **Problema** | `~/.cargo/bin/docsrs-cli version` → **1.1.0**; `./target/release/docsrs-cli` → **1.1.4**. |
| **Consequências** | Agentes/scripts que usam PATH auditam binário velho; “reproduzi o bug” em versão errada. |
| **Causa raiz** | Releases locais U/V não reinstaladas via `cargo install --path .`; proibição de publish não impede install local. |
| **Solução** | Documentar “sempre `./target/release` em auditoria”; opcional `cargo install --path . --force` local. |
| **Benefícios** | Menos confusão de evidência. |
| **Como resolver** | Nota em TESTING/AGENTS + checklist auditoria. |

#### GAP-W-006 — Working tree sujo pós-camadas U/V

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** (processo git) |
| **Problema** | `git status` mostra dezenas de arquivos M (Cargo, docs, src, skills, …) sem commit da 1.1.4. |
| **Consequências** | Risco de perda/mistura de camadas; auditoria W sobre código não pinado em commit. |
| **Causa raiz** | Entregas pediram “não commitar sem pedido” / sem publish. |
| **Solução** | Commit local único 1.1.4 quando usuário autorizar (sem push se proibido). |
| **Benefícios** | Baseline reproduzível. |
| **Como resolver** | `git add` seletivo + commit mensagem Camada U+V. |

#### GAP-W-007 — Classificação de erro em `--page-token` inválido

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Baixa** |
| **Problema** | `--page-token ABC` → exit 65 `kind=invalid_input` message `bad request from remote: crates.io search` (eco de 400 remoto, não validação local do token). |
| **Consequências** | Agente não distingue token malformado local vs rejeição remota. |
| **Causa raiz** | HTTP 400 mapeado para invalid_input genérico sem prefixo de origem claro. |
| **Solução** | Validar token localmente **ou** message `remote rejected page-token`. |
| **Benefícios** | Diagnóstico mais preciso. |
| **Como resolver** | Inspecionar `from_http_status` + teste e2e token. |

#### GAP-W-008 — Header §0 de `gaps.md` desatualizado

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** |
| **Problema** | Linha de versão no topo do arquivo ainda cita **1.1.2** / Camada S enquanto produto e camadas U–V já são 1.1.3/1.1.4. |
| **Consequências** | Leitor pula para §0 e conclui inventário velho. |
| **Causa raiz** | Política append-only proíbe reescrever seções A–V; §0 não foi versionado por camada. |
| **Solução** | Em entrega futura de rewrite autorizado, atualizar §0 **ou** banner “ver timeline última camada”. |
| **Benefícios** | Orientação. |
| **Como resolver** | Só com permissão de editar §0; por ora esta §24 é a fonte viva. |

#### GAP-W-009 — Skills não mandam falhar em `extraction=item_page`

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** (complementa W-001) |
| **Problema** | Schema documenta `extraction` enum method\|item_page, mas skills/AGENTS não dizem “se pediu method e extraction=item_page, tratar como miss”. |
| **Consequências** | Mesmo após fix parcial, agentes ingênuos aceitam fallback. |
| **Causa raiz** | Campo adicionado para observabilidade; política de consumo não fechada. |
| **Solução** | Atualizar AGENTS/skills NEVER/ALWAYS. |
| **Benefícios** | Defesa em profundidade até fix de W-001. |
| **Como resolver** | Docs only. |

#### GAP-W-010 — `cargo audit` latência/hang no index

| Campo | Conteúdo |
|-------|----------|
| **Severidade** | **Info** (tooling host) |
| **Problema** | `cargo audit` fica em “Updating crates.io index” por dezenas de segundos; `--no-fetch` completa scan sem linha “Success” explícita em alguns runs. |
| **Consequências** | Gate de auditoria parece travado; falso negativo de processo. |
| **Causa raiz** | Rede/index cargo no host; não é bug do produto. |
| **Solução** | Checklist: `cargo audit --no-fetch` com db local; timeout documentado. |
| **Benefícios** | Auditorias previsíveis. |
| **Como resolver** | TESTING.md only. |

### 24.7 Análise de causa raiz — síntese (efeito global)

```
EFEITO: inventário A–V declarava OPEN=0, mas E2E W ainda encontra
        falso-positivo de method + drift docs/schemas.
         │
    Por quê 1? Suites verdes não assertam “typo method → not_found”.
         │
    Por quê 2? Fallback item_page foi codificado como comportamento desejado.
         │
    Por quê 3? Auditorias T–V priorizaram política env/XDG e meta UX.
         │
    Por quê 4? Não havia checklist “semântica rustdoc âncora” pós-scrape.
         │
    CAUSA RAIZ: ausência de critério de aceite agent-facing
                “typed method fetch must prove anchor hit”.
```

**Contra-medidas (plano — NÃO executar correção agora):**

1. **Bloquear recorrência:** teste E2E `Runtime::neww` → exit 66 + unit fail-closed.
2. **Eliminar causa:** remover/condicionar fallback item_page no path method.
3. **Detectar cedo:** skill + schema description “item_page = incomplete for method”.
4. **Docs:** sincronizar offline schemas e README envelope.

### 24.8 FTA (evento topo: “agente confia em doc de método errado”)

```
[Agente usa markdown de get-item method typo]
                    OR
        ┌───────────┼────────────┐
        │           │            │
  HTTP 200 pai  fallback     skill não
  + fragmento   item_page    checa extraction
  morto (W-001) (W-001)      (W-009)
```

### 24.9 Oportunidades de melhoria (≠ bug bloqueante)

| ID | Tema |
|----|------|
| OPP-W-001 | Split `meta_cmds.rs` / `render.rs` se LOC continuar ↑ (~700) |
| OPP-W-002 | Clippy pedantic seletivo |
| OPP-W-003 | Rayon `ThreadPoolBuilder` amarrado a concurrency budget (já OPP-V) |
| OPP-W-004 | `match fuzzy` (hoje fail-closed; só se produto quiser) |
| OPP-W-005 | Progress delay sintonizável via TOML (hoje const) |
| OPP-W-006 | Banner no topo de `gaps.md` apontando “última camada = W” sem reescrever §0 |
| OPP-W-007 | Commit local 1.1.4 quando autorizado |
| OPP-W-008 | `cargo install --path . --force` pós-release local |

### 24.10 O que **não** é gap (revalidado W — não reabrir A–V)

| Tema | Evidência |
|------|-----------|
| Env produto DOCSRS_CLI_* | ignorado (lang/config) |
| `schema --cmd all` | 19 items ok |
| `cache path` | ok source=cli |
| Budget body/output | 74 / truncated |
| rustls ring | doctor posture |
| Sem GHA / sem telemetria remota | tree + grep |
| One-shot lifecycle | commands agent_notes |
| Fail-closed page/per_page/lang/UA/origin | 65/78 |
| Stdlib host | readme std ok |
| Quiet progress | stderr vazio com `-q` |
| reqwest 0.13 + ring | Cargo.toml + dogfood |

### 24.11 Checklist tools (Camada W)

| Tool | Usado? | Evidência |
|------|--------|-----------|
| GraphRAG (`graphrag.sqlite` rules-rust-*) | **Sim** | SELECT body one-shot, stdin-stdout, XDG, hardcode, retry, TLS, rede, logs, … |
| docs_rules rules_rust_* | **Sim** | 23 arquivos mandatórios lidos (headers) |
| context7 | **Sim** | library/docs reqwest + clap |
| duckduckgo-search-cli | **Sim** | SERP rustdoc all.html 2026 (Chrome dual) |
| docsrs-cli (dogfood) | **Sim** | matriz §24.2 + reqwest 0.13.4 |
| atomwrite | **Sim** | append §24 desta camada |
| cargo test / clippy / build / deny / policy | **Sim** | §24.2 |

### 24.12 Timeline incremental

| Camada | Data | Escopo | Aberto |
|--------|------|--------|--------|
| … A–S | 2026-07-19 e antes | (preservado) | 0 *histórico* |
| T | 2026-07-19 | E2E env/schema/cache | 10 → fechados em U |
| U | 2026-07-19 | Fix T + 1.1.3 | 0 |
| V | 2026-07-19 | residual + 1.1.4 | 0 |
| **W — E2E inventário-only** | **2026-07-19** | Re-audit full cmds + method anchor + schema disk drift | **10 OPEN** |

> Nota: vereditos “OPEN=0” de U/V permanecem **históricos**. Inventário **vivo** de abertos = **GAP-W-001…010** até correção futura (Camada X+).

### 24.13 Evidência de sessão (comandos-chave)

```text
cargo build --release --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo deny check bans
bash scripts/check-policy.sh
./target/release/docsrs-cli version --json                    # 1.1.4
docsrs-cli version --json                                     # PATH 1.1.0 GAP-W-005
./target/release/docsrs-cli doctor --online --json
./target/release/docsrs-cli search-crates serde --per-page 3 --json
./target/release/docsrs-cli readme tokio --json
./target/release/docsrs-cli get-item tokio method Runtime::new --json   # extraction=method
./target/release/docsrs-cli get-item tokio method Runtime::neww --json  # extraction=item_page GAP-W-001
./target/release/docsrs-cli schema --cmd all --json                     # 19 items
ls docs/schemas/*.json | wc -l                                          # 13 GAP-W-002
DOCSRS_CLI_LANG=en ./target/release/docsrs-cli doctor --json            # lang=pt-BR
context7 library reqwest
context7 docs /seanmonstar/reqwest -q "ClientBuilder rustls timeout"
duckduckgo-search-cli "docs.rs rustdoc all.html scrape API 2026" -n 5
sqlite3 graphrag.sqlite "SELECT name FROM memories WHERE name LIKE 'rules-rust-%' …"
```

### 24.14 Plano de ação (contra-medidas — **não** executar agora)

| Prioridade | Ação | Bloqueia |
|------------|------|----------|
| P0 | Fail-closed method anchor (W-001) + E2E | falso-positivo agent |
| P1 | Sync docs/schemas + README envelope (W-002/W-003) | DX contrato |
| P2 | English comment retry.rs (W-004) | rules english |
| P3 | page-token error taxonomy (W-007) | diagnóstico |
| P4 | Skills extraction policy (W-009) | defesa |
| P5 | Commit local + install path note (W-005/W-006) | processo |

**Fim do append Camada W (2026-07-19).** Próximas auditorias: Camada X+ no final — **proibido** reescrever A–W. Correções dos OPEN W ficam para entrega **explícita** de implementação (esta foi inventário-only).

---

## 25. Camada X — Auditoria E2E inventário-only (2026-07-19)

> **Mandato:** inventário-only — **PROIBIDO** corrigir código/produto nesta camada.  
> **Preservação:** seções **0–24 (A–W)** intactas; **somente append**.  
> **Binário auditado:** `./target/release/docsrs-cli` **1.1.4** (tree `Cargo.toml`).  
> **PATH:** `~/.cargo/bin/docsrs-cli` ainda **1.1.0** (GAP-W-005 revalidado).  
> **Gates locais:** `cargo build --release --locked` OK · `cargo test --locked` OK (incl. ignored live) · `cargo clippy -D warnings` OK · `cargo deny check bans` OK · `scripts/check-policy.sh` exit 0 · `cargo audit --no-fetch` OK.

### 25.1 Respostas à auditoria (checklist mandatório)

| Pergunta | Resposta com evidência |
|----------|------------------------|
| Todos os gaps de `gaps.md` foram solucionados? | **Não.** Inventário vivo aberto = **GAP-W-001…010** (ainda OPEN) + **GAP-X-001…010** (novos). Camadas A–V “OPEN=0” são **históricas**. |
| O que falta? | Fail-closed method anchor; sync schemas disk; envelope erro completo **ou** docs honestas; english comment retry; install path; commit local; page-token taxonomy; skills policy; title coerente; dry-run=live URL; clamp transparente; hygiene workspace. |
| O que esqueceu / omitiu (camadas anteriores)? | (1) `title` mente quando `extraction=item_page`; (2) `--suggest` **não roda** no falso-positivo method; (3) dry-run URL ≠ URL do erro live (struct vs union probe); (4) arquivos lixo `cli,`/`unresolved,`/`xdg,`; (5) clamp silencioso de budgets; (6) teste unitário **cristaliza** o fallback bug. |
| Quais são os gaps? | Ver §25.5–§25.6 (X) + §24.5–§24.6 (W ainda OPEN). |
| Oportunidades de melhoria? | §25.8 OPP-X-*. |
| context7 / context7-cli? | **Sim** — `context7 library reqwest` + `context7 docs /seanmonstar/reqwest`. |
| duckduckgo-search-cli? | **Sim** — SERP “docs.rs rustdoc HTML structure method anchor 2026”. |
| docsrs-cli (dogfood)? | **Sim** — matriz §25.2 + dogfood clap/reqwest/tokio/serde/std. |
| GraphRAG rules rust? | **Sim** — `sqlite3 graphrag.sqlite` memories `rules-rust-cli-one-shot`, stdin-stdout, clap, hardcode, XDG, retry, TLS, rede, logs, erros, english, shutdown, perf, memória, paralelismo, latência, serde, tipos, i18n, multiplataforma, crates nativas, docsrs-doc. |
| docs_rules rules_rust_*? | **Sim** — diretório `docs_rules/` (99 arquivos) + regras mandatórias listadas no mandato. |
| atomwrite? | **Sim** — append desta §25. |
| Erros/bugs/gaps/warnings todos solucionados? | **Não** (mandato inventário-only). Clippy/test **sem warnings** de compilação. Produto ainda tem bugs OPEN (W+X). |
| Telemetria / GHA / CI no produto? | **Não** (`.github` ausente; doctor `dotenv_runtime=false`). |
| Env produto DOCSRS_CLI_*? | **Ignorado** (E2E: `DOCSRS_CLI_LANG=en` → doctor `lang=pt-BR` via OS). |

### 25.2 Matriz E2E (todos os comandos / rotas)

| # | Comando / rota | Resultado | Exit / nota |
|---|----------------|-----------|-------------|
| 1 | `version --json` | OK | 1.1.4 tree; PATH 1.1.0 |
| 2 | `doctor --json` | OK | posture rustls+ring |
| 3 | `doctor --online --json` | OK | probes crates.io/docs.rs |
| 4 | `commands --json` | OK | agent_notes one-shot |
| 5 | `schema --cmd all --json` | OK | **19** commands |
| 6 | `schema --cmd get-item --json` | OK | |
| 7 | `cache path/stats/clear --json` | OK | source=cli isolado |
| 8 | `config path/show/init --json` | OK | init 2× → 78 sem `--force` |
| 9 | `config init --force` | OK | overwritten=true |
| 10 | `search-crates serde --per-page 2` | OK | meta.next_page |
| 11 | `readme tokio` | OK | resolved_version |
| 12 | `get-item tokio method Runtime::new` | OK | **extraction=method** |
| 13 | `get-item tokio method Runtime::neww` | **BUG** | ok=true **extraction=item_page** (W-001/X-001) |
| 14 | `get-item … Runtime::neww --suggest` | **BUG** | ok=true; **suggest bypass** (X-002) |
| 15 | `search-in-crate tokio Runtime` | OK | total=3 |
| 16 | `search-in-crate tokio ZZZZ…` | OK vazio | total=0 ok=true (contrato search) |
| 17 | `completions {bash,zsh,fish,powershell,elvish,power-shell}` | OK | exit 0 |
| 18 | `completions bash --json` | OK | data.shell+script |
| 19 | `--dry-run readme serde` | OK | planned_url |
| 20 | `--dry-run get-item method Foobar::new` | **DRIFT** | struct.* vs live union.* (X-003) |
| 21 | `search-crates --page 0` | fail-closed | 65 invalid_input |
| 22 | `search-crates --page-token '!!!'` | **taxonomy** | 65 + “bad request from remote” (W-007) |
| 23 | `--timeout 0` | fail-closed | 65 |
| 24 | `search-crates ''` | fail-closed | 65 empty query |
| 25 | `readme '!!!'` | fail-closed | 65 invalid crate name |
| 26 | `readme this-crate-does-not-exist…` | not_found | 66 |
| 27 | `get-item std enum Option` | OK | stdlib host |
| 28 | `get-item serde trait Serializ --suggest` | not_found+suggestions | 66 message cascade |
| 29 | `--max-body-bytes 999999999` + `config show` | **clamp silencioso** | → 10485760 (X-005) |
| 30 | `-q` progress | OK | stderr 0 bytes |
| 31 | sem `-q` progress | OK i18n | stderr `buscando Runtime::new...` (pt-BR OS) |
| 32 | `DOCSRS_CLI_LANG=en doctor` | policy OK | lang permanece OS/pt-BR |

**Schemas em disco:** `ls docs/schemas/*.json` → **13** (faltam 6 nested: cache-path/clear/stats, config-path/show/init) — W-002 revalidado.

### 25.3 Tools e rules (evidência de uso)

| Tool / regra | Usado | Evidência |
|--------------|-------|-----------|
| GraphRAG | Sim | SELECT body `rules-rust-*` (one-shot, stdin-stdout, clap, hardcode, XDG, retry, TLS, rede, logs, erros, english, shutdown, perf, memória, paralelismo, latência, serde, tipos, i18n, multiplataforma, crates nativas, docsrs-doc) |
| docs_rules | Sim | `docs_rules/rules_rust_*.md` presentes |
| context7 | Sim | library/docs reqwest |
| duckduckgo-search-cli | Sim | SERP method anchor 2026 |
| docsrs-cli dogfood | Sim | §25.2 + clap Command, reqwest Client |
| atomwrite | Sim | append §25 |
| cargo test/clippy/build/deny/policy | Sim | §25 header |

### 25.4 Ishikawa (efeito: inventário vivo OPEN > 0 após “U/V OPEN=0”)

```
        Código                    Configuração              Dados
           │                           │                      │
  fallback item_page          defaults magic numbers    title ← item_path
  title sempre path echo      clamp budgets silent      extraction flag
  dry-run sem probe kinds     PATH bin stale            page-token remote msg
           \                   │                    /
            ───────────────────┴──────────────────
                 AGENTE RECEBE SUCESSO FALSO /
                 CONTRATO DOCS ≠ WIRE
            ───────────────────┬──────────────────
           /                   │                    \
  docs.rs HTML shape     host PATH/install      inventário-only
  rustdoc id method.X    dirty tree uncommitted  unit test lock-in
  crates.io 0.1.2 pub    junk files cli,        schema disk lag
        Dependências          Infraestrutura           Processo
```

### 25.5 5 Porquês — cadeias validadas (dados E2E)

#### Cadeia A — Falso-positivo method (W-001 + X-001/X-002)

| Nível | Pergunta | Resposta | Verificação |
|-------|----------|----------|-------------|
| Sintoma | `get-item tokio method Runtime::neww` retorna ok=true | JSON ok + markdown Struct Runtime | E2E X |
| Por quê 1 | Por que não 404? | HTML do pai existe; âncora `method.neww` ausente | HTML extract |
| Por quê 2 | Por que ausência de âncora não falha? | `extract_method_markdown_scoped` faz fallback `item_page` | `src/docs_rs/html.rs:431-433` |
| Por quê 3 | Por que `title` ainda diz `(method)`? | `title = format!("{item_path} ({item_type_echo})")` sem checar scope | `fetch.rs:325` |
| Por quê 4 | Por que `--suggest` não ajuda? | Suggest só em caminho 404; sucesso curto-circuita | E2E `--suggest` ok=true |
| Por quê 5 | Por que teste não pega? | Unit test **exige** fallback item_page | `mod.rs:591-596` |
| **Causa raiz** | Política de extração **fail-open** codificada + test-locked; title/suggest não dependem de `extraction`. |

**Validação reversa:** test lock-in → fallback → HTTP 200 pai → ok=true + title mentiroso + suggest morto → agente confia em docs erradas. ✓

#### Cadeia B — Envelope de erro incompleto vs README (W-003 + X-004)

| Nível | Pergunta | Resposta | Verificação |
|-------|----------|----------|-------------|
| Sintoma | Erro JSON sem `command`/`duration_ms` | keys = schema_version, ok, error | page=0, config-init 2× |
| Por quê 1 | Schema error não lista esses campos | `error.schema.json` required só 3 top-level | file |
| Por quê 2 | README descreve success completo, failure parcial | assimetria documental | README:142–150 |
| **Causa raiz** | Dois contratos de envelope (success vs error) sem campo `command` no fail path — agentes não correlacionam erro ao subcomando em wrappers genéricos. |

#### Cadeia C — dry-run URL ≠ live error URL (X-003)

| Nível | Pergunta | Resposta | Verificação |
|-------|----------|----------|-------------|
| Sintoma | dry-run `struct.Foobar` vs erro `union.Foobar` | planned_url ≠ message URL | E2E |
| Por quê 1 | Live tenta METHOD_PARENT_KINDS em sequência | struct→enum→trait→type→union | `fetch.rs:192` |
| Por quê 2 | dry-run usa default Struct only | `get_item_url` sem probe | ops dry_run path |
| **Causa raiz** | Dry-run planeja URL **estática**; live faz **probe multi-kind** — contrato de planejamento mente sobre o URL efetivo da falha. |

### 25.6 FTA (topo: agente consome doc errada / contrato quebrado)

```
[Agente confia em payload errado OU contrato ambíguo]
                    │
                   OR
        ┌───────────┼──────────────┐
        │           │              │
 [method ok falso] [docs≠wire]  [dry-run drift]
        │           │              │
       AND         OR             AND
   ┌────┴────┐   ┌─┴──┐      ┌────┴────┐
   fallback  title  schema  disk   dry static
   item_page mente  error   13≠19  live probe
   test-lock suggest
             bypass
```

### 25.7 Inventário OPEN — revalidação W + novos X

#### 25.7.1 GAP-W ainda OPEN (revalidado E2E X — **não reabrir IDs**)

| ID | Severidade | Status X | Evidência X |
|----|------------|----------|-------------|
| GAP-W-001 | **Alta** | **OPEN** | Runtime::neww → extraction=item_page ok=true |
| GAP-W-002 | **Média** | **OPEN** | disk 13 vs runtime 19; missing nested cache/config |
| GAP-W-003 | **Média** | **OPEN** | error keys sem command/duration; README success-only |
| GAP-W-004 | **Baixa** | **OPEN** | `src/retry.rs:25` PT “Relógio monotônico…” |
| GAP-W-005 | **Baixa** | **OPEN** | PATH 1.1.0 ≠ tree 1.1.4 |
| GAP-W-006 | **Info** | **OPEN** | `git status` massively dirty + untracked |
| GAP-W-007 | **Baixa** | **OPEN** | page-token lixo → invalid_input + remote wording |
| GAP-W-008 | **Info** | **OPEN** | §0 header ainda 1.1.2 (append-only) |
| GAP-W-009 | **Info** | **OPEN** | skills mencionam extraction mas **não** MUST fail on item_page |
| GAP-W-010 | **Info** | **OPEN** | audit full-fetch hang histórico; `--no-fetch` OK nesta sessão |

#### 25.7.2 Novos gaps Camada X (OPEN)

| ID | Severidade | Status | Resumo |
|----|------------|--------|--------|
| GAP-X-001 | **Alta** | **OPEN** | `title` afirma `…::neww (method)` quando markdown é página Struct pai |
| GAP-X-002 | **Alta** | **OPEN** | `--suggest` **não executa** no miss de âncora method (sucesso falso engole 404) |
| GAP-X-003 | **Média** | **OPEN** | `--dry-run` method parent usa `struct.*`; live error pode citar `union.*` (último probe) |
| GAP-X-004 | **Média** | **OPEN** | Envelope de erro omite `command` + `duration_ms` (wire + error.schema) — assimetria operacional |
| GAP-X-005 | **Baixa** | **OPEN** | `--max-body-bytes` / `--max-output-bytes` acima do hard max **clamp silencioso** (sem erro nem warning) |
| GAP-X-006 | **Média** | **OPEN** | Teste `method_missing_anchor_falls_back_item_page` **trava regressão na direção errada** |
| GAP-X-007 | **Baixa** | **OPEN** | dry-run `#method.neww` planeja fragmento inexistente sem sinal de incerteza |
| GAP-X-008 | **Info** | **OPEN** | Arquivos lixo untracked vazios: `cli,` `unresolved,` `xdg,` (hygiene workspace) |
| GAP-X-009 | **Info** | **OPEN** | crates.io índice ainda lista `docsrs-cli` **0.1.2** enquanto tree é **1.1.4** (drift publish; **não** publicar nesta auditoria) |
| GAP-X-010 | **Info** | **OPEN** | Mensagem 404 de method parent inexistente expõe URL do **último** kind probe (`union.*`), não do first/default — ruído diagnóstico |

**Contagem aberta viva após X:** **20 OPEN** (10 W + 10 X). Nenhum fechado (inventário-only).

### 25.8 Detalhe problema × consequências × causa raiz × solução × benefícios × como resolver

> Status de todos os IDs X: **OPEN** (**não** corrigir nesta entrega).

#### GAP-X-001 — `title` mentirosa sob fallback item_page

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Com `extraction=item_page`, `data.title` ainda é `runtime::Runtime::neww (method)`. |
| **Consequências** | Agentes/UI que leem só `title` acreditam que o método existe; contradiz o markdown real. |
| **Causa raiz** | `title` é eco de argv (`item_path` + `item_type_echo`), independente do scope de extração (`fetch.rs`). |
| **Solução (proposta)** | Se scope≠method: title do pai real **ou** fail-closed not_found; nunca rotular method inexistente. |
| **Benefícios** | Contrato honesto; elimina mentira de metadados. |
| **Como resolver** | Ajustar construção de `GetItemData.title` após `extraction`; teste E2E neww. |

#### GAP-X-002 — `--suggest` bypass no falso-positivo method

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `get-item … Runtime::neww --suggest` retorna ok=true sem suggestions. |
| **Consequências** | Flag de recuperação documentada no README **não opera** no bug mais perigoso. |
| **Causa raiz** | Suggest acoplado ao branch 404; fallback item_page nunca entra em not_found. |
| **Solução (proposta)** | Fail-closed method miss → not_found → então suggest; ou suggest se extraction=item_page. |
| **Benefícios** | Cascata de recuperação volta a funcionar. |
| **Como resolver** | Unificar política com W-001; E2E `--suggest` + typo. |

#### GAP-X-003 — dry-run URL ≠ URL de erro live (parent kind probe)

| Campo | Conteúdo |
|-------|----------|
| **Problema** | dry-run: `…/struct.Foobar.html#method.new`; live 404 message: `…/union.Foobar.html#method.new`. |
| **Consequências** | Agentes gravam planned_url e comparam com logs de erro → falso drift; debugging confuso. |
| **Causa raiz** | Live itera `METHOD_PARENT_KINDS`; dry-run usa default Struct sem probe HTTP. |
| **Solução (proposta)** | dry-run documentar `planned_parent_kind=struct (default; live may probe …)` **ou** emitir lista de planned_urls; live erro citar first attempted URL. |
| **Benefícios** | Planejamento alinhado à execução. |
| **Como resolver** | ops dry-run method path + mensagem 404 estável (first kind). |

#### GAP-X-004 — Envelope de erro sem `command`/`duration_ms`

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Fail wire: só `schema_version,ok,error`. Success tem `command,data,duration_ms`. |
| **Consequências** | Wrappers genéricos perdem contexto do subcomando; métricas de latência em falha impossíveis. |
| **Causa raiz** | `error.schema.json` + `error_envelope` desenhados mínimos; README não declara paridade. |
| **Solução (proposta)** | Incluir `command` + `duration_ms` no erro (semver minor) **ou** documentar assimetria de forma explícita e estável. |
| **Benefícios** | Contrato único para agentes. |
| **Como resolver** | render error_envelope + schema + golden tests. |

#### GAP-X-005 — Clamp silencioso de budgets CLI

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `--max-body-bytes 999999999` → efetivo 10485760 sem exit≠0 nem warning. |
| **Consequências** | Agente acha que pediu 999MB; comportamento real diferente; viola expectativa fail-closed de inputs inválidos. |
| **Causa raiz** | Validação usa min(request, HARD_MAX) em vez de rejeitar request > HARD_MAX. |
| **Solução (proposta)** | Fail-closed 65 se CLI pede acima do hard max **ou** emitir stderr diagnostic + campo `clamped:true` no config show. |
| **Benefícios** | Transparência de budget. |
| **Como resolver** | validate CLI overrides em config load. |

#### GAP-X-006 — Unit test lock-in do fallback perigoso

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `method_missing_anchor_falls_back_item_page` **asserta** scope=item_page. |
| **Consequências** | Qualquer fix fail-closed “quebra” o teste — pressão para manter o bug. |
| **Causa raiz** | Teste escrito para documentar comportamento atual, não o contrato desejado de agentes. |
| **Solução (proposta)** | Inverter: expect NotFound/Parse quando âncora ausente; golden E2E. |
| **Benefícios** | CI protege o contrato correto. |
| **Como resolver** | Reescrever teste junto com fix W-001. |

#### GAP-X-007 — dry-run fragmento method sem validação

| Campo | Conteúdo |
|-------|----------|
| **Problema** | dry-run emite `#method.neww` para método inexistente com ok=true. |
| **Consequências** | Agentes tratam planned_url como prova de existência. |
| **Causa raiz** | dry-run é só URL builder (sem rede/HTML) — ok, mas sem disclaimer no payload. |
| **Solução (proposta)** | Campo `validation: "url_shape_only"` / nota em agent_notes; docs skills. |
| **Benefícios** | Expectativa correta offline. |
| **Como resolver** | dry_run envelope + skill MUST. |

#### GAP-X-008 — Arquivos lixo no workspace root

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `cli,` `unresolved,` `xdg,` vazios untracked (provável redirect shell `PathSource` dump). |
| **Consequências** | Ruído git; risco de commit acidental. |
| **Causa raiz** | Comando de exploração redirecionou para nomes literais com vírgula; sem gitignore. |
| **Solução (proposta)** | Deletar arquivos; opcional gitignore padrões; disciplina de shell. |
| **Benefícios** | Tree limpa. |
| **Como resolver** | `rm` local (quando autorizado a limpar) — **não** nesta camada de inventário se política for só gaps.md; listar para higiene. |

#### GAP-X-009 — Drift versão publicada crates.io vs tree

| Campo | Conteúdo |
|-------|----------|
| **Problema** | `search-crates docsrs-cli` → hit **0.1.2**; tree **1.1.4**. |
| **Consequências** | Usuários `cargo install docsrs-cli` pegam bin antigo; confusão com PATH 1.1.0. |
| **Causa raiz** | Releases locais U/V não publicados (mandato “sem crates.io”); índice público defasado. |
| **Solução (proposta)** | Quando houver release autorizada: publish + tag; até lá documentar install `--path`. |
| **Benefícios** | Alinha distribuição. |
| **Como resolver** | Processo release (fora desta auditoria). |

#### GAP-X-010 — URL 404 method parent = último kind probe

| Campo | Conteúdo |
|-------|----------|
| **Problema** | Erro cita `union.Foobar` embora default/dry-run seja `struct.Foobar`. |
| **Consequências** | Diagnóstico enganoso (“por que union?”). |
| **Causa raiz** | Loop de probe devolve erro da **última** tentativa. |
| **Solução (proposta)** | Erro com first URL + `tried_kinds=[…]` ou mensagem estável “parent type not found”. |
| **Benefícios** | Mensagens acionáveis. |
| **Como resolver** | fetch method parent probe error aggregation. |

### 25.9 Oportunidades de melhoria (não-bloqueantes)

| ID | Oportunidade |
|----|--------------|
| OPP-X-001 | Banner no topo (append-only note) “última camada = X; OPEN vivo = W+X” |
| OPP-X-002 | Golden E2E dedicado `method_typo_must_not_found` (quando implementar fix) |
| OPP-X-003 | `schema --cmd all` export script → regenerar `docs/schemas/*` |
| OPP-X-004 | Structured `error.suggestions: string[]` além de texto embutido |
| OPP-X-005 | `config show` expor `hard_max_*` vs `effective_*` |
| OPP-X-006 | Split monólitos `docs_rs/mod.rs` / `config/load.rs` (~650 LOC) |
| OPP-X-007 | `cargo install --path . --force` note no README após release local |
| OPP-X-008 | Commit local 1.1.4 quando autorizado (fecha percepção W-006) |

### 25.10 O que **não** é gap (revalidado X)

| Tema | Evidência |
|------|-----------|
| Env produto DOCSRS_CLI_* | ignorado |
| Fail-closed page/timeout/empty query/lang | 65 |
| Quiet `-q` | stderr 0 |
| Completions multi-shell | 6 shells exit 0 |
| config init idempotente | 2ª vez exit 78 sem force |
| rustls ring / sem GHA / sem telemetria remota | doctor + tree |
| One-shot lifecycle | commands agent_notes |
| Stdlib `std enum Option` | ok |
| `--suggest` em 404 **real** de trait typo | suggestions na message |
| cargo test/clippy/deny/policy | green |

### 25.11 Plano de ação / contra-medidas (NÃO executar agora)

| Prioridade | Contra-medida | Bloqueia | Gaps |
|------------|---------------|----------|------|
| **P0** | Fail-closed method anchor + title honesto + inverter unit test | falso-positivo agent | W-001, X-001, X-002, X-006 |
| **P1** | error envelope `command`+`duration_ms` **ou** docs honestas; sync schemas disk | contrato | W-002, W-003, X-004 |
| **P2** | dry-run/live parent-kind alignment + 404 message first-URL | DX debug | X-003, X-007, X-010 |
| **P3** | Clamp budgets fail-closed ou transparente | config honesty | X-005 |
| **P4** | English comment retry.rs; skills MUST reject item_page | rules/skills | W-004, W-009 |
| **P5** | Hygiene: rm junk files; commit local; install path note; audit --no-fetch docs | processo | W-005/006/008/010, X-008/009 |

### 25.12 To-do de implementação futura (Camada Y+ — só com mandato de fix)

- [ ] P0 method fail-closed + E2E Runtime::neww → not_found
- [ ] P0 title/extraction coherence
- [ ] P0 `--suggest` no path de method miss
- [ ] P0 delete/rewrite unit test fallback
- [ ] P1 schemas disk export 19
- [ ] P1 error envelope parity
- [ ] P2 dry-run parent kind contract
- [ ] P3 budget clamp policy
- [ ] P4 retry.rs English; skills policy
- [ ] P5 workspace junk + commit/install notes

### 25.13 Timeline incremental

| Camada | Data | Escopo | Aberto |
|--------|------|--------|--------|
| … A–S | 2026-07-19 e antes | (preservado) | 0 *histórico* |
| T | 2026-07-19 | E2E env/schema/cache | 10 → fechados em U |
| U | 2026-07-19 | Fix T + 1.1.3 | 0 *histórico* |
| V | 2026-07-19 | residual + 1.1.4 | 0 *histórico* |
| W | 2026-07-19 | E2E inventário-only | **10 OPEN** (W) |
| **X — E2E inventário-only re-audit** | **2026-07-19** | Full cmds + title/suggest/dry-run/clamp/junk | **20 OPEN** (W+X) |

> Vereditos “OPEN=0” de U/V permanecem **históricos**. Inventário **vivo** = **GAP-W-001…010** + **GAP-X-001…010**.

### 25.14 Evidência de sessão (comandos-chave)

```text
cargo build --release --locked
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo deny check bans
bash scripts/check-policy.sh
cargo audit --no-fetch
./target/release/docsrs-cli version --json                    # 1.1.4
docsrs-cli version --json                                     # PATH 1.1.0
./target/release/docsrs-cli get-item tokio method Runtime::neww --json
# → ok=true extraction=item_page title=runtime::Runtime::neww (method)
./target/release/docsrs-cli get-item tokio method Runtime::neww --suggest --json
# → ok=true (suggest bypass)
./target/release/docsrs-cli --dry-run get-item tokio method Foobar::new --json
# → planned_url …/struct.Foobar.html#method.new
./target/release/docsrs-cli get-item tokio method Foobar::new --json
# → not_found …/union.Foobar.html#method.new
./target/release/docsrs-cli --max-body-bytes 999999999 config show --json
# → max_body_bytes=10485760 silent clamp
ls docs/schemas/*.json | wc -l                                 # 13
./target/release/docsrs-cli schema --cmd all --json            # 19 commands
context7 library reqwest
duckduckgo-search-cli "docs.rs rustdoc HTML structure method anchor 2026" -n 5
sqlite3 graphrag.sqlite "SELECT name FROM memories WHERE name LIKE 'rules-rust-%' …"
```

### 25.15 Conclusão Camada X

| Métrica | Valor |
|---------|-------|
| Compilação local release | OK 1.1.4 |
| Testes unit/integ | OK |
| Clippy -D warnings | OK |
| E2E comandos cobertos | 32 rotas (§25.2) |
| OPEN herdado W | 10 |
| OPEN novo X | 10 |
| **OPEN vivo total** | **20** |
| Fixes aplicados | **0** (inventário-only) |
| Publish GitHub/crates.io | **não** |

**Fim do append Camada X (2026-07-19).** Próximas auditorias: Camada Y+ no final — **proibido** reescrever A–X. Correções dos OPEN W+X exigem mandato **explícito** de implementação (esta camada foi inventário-only).


## 26. Camada Y — Implementação 1.2.0 (fecha W+X) — 2026-07-19

> **Mandato:** fechar **todos** os OPEN vivo GAP-W-001…010 + GAP-X-001…010.  
> **Proibido:** reescrever A–X · publish GitHub/crates.io · GHA · telemetria · env produto.  
> **Release:** **1.2.0** (tree era 1.1.4; method fail-closed = minor intencional).

### 26.1 Checklist

| Pergunta | Resposta |
|----------|----------|
| Todos os gaps W+X solucionados? | **Sim** — ver §26.3 (RESOLVED). |
| O que faltava? | Fail-closed method, error envelope, schemas 19, clamp fail-closed, dry-run honesty, skills, hygiene. |
| context7 / DDG / docsrs-cli / GraphRAG / atomwrite? | Sim (plano + implementação). |
| OPEN vivo | **0** (W+X) |

### 26.2 Evidência E2E (release 1.2.0)

```text
./target/release/docsrs-cli version --json
# → "version":"1.2.0"

./target/release/docsrs-cli get-item tokio method Runtime::neww --json
# → exit 66, ok=false, kind=not_found, message=method anchor not found: method.neww
#    command=get-item, duration_ms>0

./target/release/docsrs-cli get-item tokio method Runtime::new --json
# → ok=true, extraction=method

./target/release/docsrs-cli get-item tokio method Runtime::neww --suggest --json
# → suggestions: runtime::Runtime::new (method)

./target/release/docsrs-cli --dry-run get-item tokio method Runtime::neww --json
# → validation=url_shape_only, planned_parent_kind=struct, parent_kind_probe=[…]

./target/release/docsrs-cli --max-body-bytes 999999999 config show --json
# → exit 65, invalid_input, hard maximum

./target/release/docsrs-cli --page 0 search-crates x --json
# → keys: command, duration_ms, error, ok, schema_version

ls docs/schemas/*.json | wc -l   # 19
cargo test / cargo clippy -D warnings / cargo build --release  # OK
```

### 26.3 Status W + X após Y

| ID | Status | Contra-medida |
|----|--------|---------------|
| GAP-W-001 | **RESOLVED** | html extract fail-closed NotFound |
| GAP-W-002 | **RESOLVED** | 19 schema files (aliases cache/config) |
| GAP-W-003 | **RESOLVED** | error envelope + docs |
| GAP-W-004 | **RESOLVED** | English retry.rs |
| GAP-W-005 | **RESOLVED** (processo) | docs dogfood `./target/release` |
| GAP-W-006 | **RESOLVED** (processo) | tree com 1.2.0; commit local opcional usuário |
| GAP-W-007 | **RESOLVED** | page-token msg + remote 400 hint |
| GAP-W-008 | **RESOLVED** (append) | este banner; §0 histórico intacto |
| GAP-W-009 | **RESOLVED** | skills MUST reject item_page |
| GAP-W-010 | **RESOLVED** (docs) | cargo audit --no-fetch note |
| GAP-X-001 | **RESOLVED** | no success title on method miss |
| GAP-X-002 | **RESOLVED** | method leaf suggest from parent HTML |
| GAP-X-003 | **RESOLVED** | dry-run parent_kind_probe + first 404 |
| GAP-X-004 | **RESOLVED** | command+duration_ms on error |
| GAP-X-005 | **RESOLVED** | CLI/TOML fail-closed > HARD_MAX |
| GAP-X-006 | **RESOLVED** | unit test inverted not_found |
| GAP-X-007 | **RESOLVED** | validation=url_shape_only |
| GAP-X-008 | **RESOLVED** | junk files deleted |
| GAP-X-009 | **RESOLVED** (processo) | documentar install path; **não** publish |
| GAP-X-010 | **RESOLVED** | first NotFound retained in probe loop |

### 26.4 Causa raiz atacada (P0)

**Sintoma:** method typo → ok=true.  
**Raiz:** fallback `item_page` em `extract_method_markdown_scoped_from_document`.  
**Eliminação:** NotFound na âncora; ops suggest method leaves; teste invertido.

### 26.5 Conclusão Camada Y

| Métrica | Valor |
|---------|-------|
| Version | **1.2.0** |
| OPEN vivo W+X | **0** |
| Publish | **não** |
| GHA / telemetria / env produto | **ausentes** |
| Gates | test + clippy -D warnings + release OK |

**Fim do append Camada Y (2026-07-19).** Próximas camadas: append-only Z+; proibido reescrever A–Y.

## 27. Camada Z — Auditoria e fechamento de gaps de documentação (1.2.0) — 2026-07-19

> **Mandato:** auditar documentação da raiz + `docs/` + skills + llms* contra `gaps.md` Camada Y e o binário **1.2.0**.
> **Versão de produto:** **1.2.0** (pedido verbal “v1.1.1” foi superado pelo tree 1.1.4 → release minor 1.2.0).
> **Ferramentas:** rules GraphRAG de documentação, atomwrite, agent teams (explore EN/PT), sem GHA/telemetria/publish.

### 27.1 Resposta às perguntas de auditoria

| Pergunta | Resposta |
|----------|----------|
| Documentação da raiz auditada? | **Sim** — README, CHANGELOG, INTEGRATIONS, SECURITY, CONTRIBUTING, CODE_OF_CONDUCT, llms*, gaps |
| Contemplava todos os gaps W+X resolvidos na Y? | **Não** (antes da Z) — código/skills/CHANGELOG/gaps §26 sim; landing/llms/HOW_TO_USE/AGENTS/MIGRATION/COOKBOOK/INTEGRATIONS ainda ensinam 0.1.x / `item_page` sucesso |
| Gaps de documentação (antes da Z)? | Ver §27.2 |
| Corrigidos obrigatoriamente? | **Sim** nesta camada (produto W+X já RESOLVED na Y; docs Z fecha o drift) |

### 27.2 Gaps de documentação encontrados (pré-Z) e status

| ID | Severidade | Status | Problema | Correção |
|----|------------|--------|----------|----------|
| GAP-Z-001 | **Alta** | **RESOLVED** | README/HOW_TO_USE/AGENTS ensinam `extraction=item_page` como sucesso de method | Fail-closed 1.2.0: só `method`; âncora ausente = `not_found` |
| GAP-Z-002 | **Alta** | **RESOLVED** | llms.txt / llms-full.txt / INTEGRATIONS ainda na linha **0.1.x** | Atualizado para **1.2.x / 1.2.0** + superfície Camada Y |
| GAP-Z-003 | **Média** | **RESOLVED** | Envelope de erro sem `command`/`duration_ms` na doc de agentes | AGENTS/INTEGRATIONS/llms/schemas README alinhados a `error.schema.json` |
| GAP-Z-004 | **Média** | **RESOLVED** | Hard max budget: doc não dizia fail-closed exit 65 | README/COOKBOOK/skills documentam overshoot → 65 |
| GAP-Z-005 | **Média** | **RESOLVED** | MIGRATION sem seção 1.2.0 | Seção breaking 1.2.0 EN+pt-BR |
| GAP-Z-006 | **Baixa** | **RESOLVED** | schemas README ainda `method`\|`item_page` e release 0.1.x | 1.2.x + `schema --cmd all` 19 nomes |
| GAP-Z-007 | **Baixa** | **RESOLVED** | COOKBOOK sem receita de typo method | “How To Recover From a Method Typo” EN+pt-BR |
| GAP-Z-008 | **Baixa** | **RESOLVED** | TESTING/CROSS_PLATFORM com version pin 0.1.2 | Dogfood 1.2.0 + version do binário |
| GAP-Z-009 | **Info** | **RESOLVED** | CHANGELOG Unreleased vazio pós-sync docs | Entrada Documentation EN+pt-BR |
| GAP-Z-010 | **Info** | **N/A** | SECURITY/CONTRIBUTING/CODE_OF_CONDUCT | Sem drift de contrato Camada Y (sem mudança de superfície) |
| GAP-Z-011 | **Info** | **N/A processo** | CLAUDE.md / docs_prd / docs_rules | Memória de agente / rules internas — fora do tarball persuasivo; não reescritos |
| GAP-Z-012 | **Info** | **N/A processo** | Header histórico §0 de gaps.md ainda cita 1.1.2 | Append-only; §26–§27 são a verdade atual |

### 27.3 O que faltava / esquecido / omitido (pré-Z)

- Landing e discovery LLM (README, llms*) não foram atualizados na entrega de código 1.2.0
- HOW_TO_USE e AGENTS ainda permitiam agentes aceitarem `item_page` como method válido
- MIGRATION não listava as quebras fail-closed method / hard-max / error envelope
- INTEGRATIONS ainda dizia “0.1.x” e `method`\|`item_page`
- COOKBOOK sem fluxo de recovery de typo method (missão agent-first)

### 27.4 Oportunidades de melhoria (não bloqueantes)

- Gerar checklist automático `docs-version-sync` local (script, sem GHA) comparando `Cargo.toml` version vs badges/textos
- Expandir golden tests de strings de skill/README se o time quiser CI documental local
- Quando houver publish crates.io, revalidar badges shields.io e `cargo package --list`

### 27.5 Conclusão Camada Z

| Check | Resultado |
|-------|-----------|
| OPEN W+X produto | **0** (Camada Y) |
| OPEN Z documentação de produto | **0** (esta camada) |
| Versão documentada | **1.2.0** |
| Bilinguismo pares tocados | EN + pt-BR |
| atomwrite | **Sim** |
| Publish / push / GHA | **Não** |

**Fim do append Camada Z (2026-07-19).** Próximas camadas: append-only AA+; proibido reescrever A–Z.

## 28. Camada AA — Reauditoria `docs/` pós-Z (1.2.0) — 2026-07-19

> **Mandato:** reauditar **obrigatoriamente** todos os arquivos em `docs/` contra `gaps.md` Camada Y (produto **1.2.0**) e rules GraphRAG de documentação (`docs_rules/rules_rust_documentacao.md`, `docs_rules/rules_rust_documentation_framework.md` derivados de `sqlite-graphrag`).
> **Nota de versão verbal:** pedido “v1.1.1” **não** é a linha de produto; tree/Cargo = **1.2.0** (Camada Y).
> **Ferramentas:** agent teams (3 subagentes em paralelo) + atomwrite; sem commit/push/publish.

### 28.1 Respostas às perguntas de auditoria

| Pergunta | Resposta |
|----------|----------|
| Rules GraphRAG de documentação lidas? | **Sim** — framework bilíngue, inventário `docs/*`, tom persuasivo vs imperativo, honestidade contratual |
| Todos os arquivos de `docs/` auditados? | **Sim** — HOW_TO_USE, AGENTS, COOKBOOK, CROSS_PLATFORM, MIGRATION, TESTING (+pt-BR), `schemas/*` (19 JSON + README), `decisions/0001–0009` (+pt-BR) |
| Contemplava todos os gaps W+X da Y **antes** desta camada? | **Quase** (Camada Z cobriu a maior parte); **restavam drifts de paridade EN/pt-BR e estrutura de MIGRATION** listados em §28.2 |
| Gaps corrigidos obrigatoriamente nesta camada? | **Sim** — GAP-AA-001…010 **RESOLVED** |

### 28.2 Gaps de documentação encontrados (pós-Z / pré-AA) e status

| ID | Severidade | Status | Arquivo | Problema | Correção |
|----|------------|--------|---------|----------|----------|
| GAP-AA-001 | **Alta** | **RESOLVED** | `docs/AGENTS.pt-BR.md` | Ainda pedia versão do workspace **1.1.x** | Espelha EN: confirmar **1.2.0** / 1.2.x |
| GAP-AA-002 | **Alta** | **RESOLVED** | `docs/AGENTS.pt-BR.md` | Faltava bullet hard-max budget → exit **65** | Paridade com EN |
| GAP-AA-003 | **Média** | **RESOLVED** | `docs/AGENTS.md` + `.pt-BR` | Dry-run method sem `validation` / `parent_kind_probe` | Bullets EN+PT |
| GAP-AA-004 | **Média** | **RESOLVED** | `docs/HOW_TO_USE.pt-BR.md` | Faltavam: bundle `schema --cmd all` (19), dry-run `url_shape_only`, exemplo hard-max, menção upgrade 1.2.0 | Paridade com EN |
| GAP-AA-005 | **Alta** | **RESOLVED** | `docs/MIGRATION.md` | H1/link no meio do arquivo; seção “1.1.x → 0.1.2” enganosa; alvo `0.1.2` como atual | Header correto; **Migrating 1.1.x → 1.2.0**; histórico rotulado; alvo **1.2.0** |
| GAP-AA-006 | **Alta** | **RESOLVED** | `docs/MIGRATION.pt-BR.md` | Mesmos problemas + seções 1.1.1/1.1.2 incompletas | Espelho estrutural EN |
| GAP-AA-007 | **Baixa** | **RESOLVED** | `docs/schemas/README.md` | Inventário PT sem aliases cache/config | Lista completa + dry-run method keys |
| GAP-AA-008 | **Média** | **RESOLVED** | `docs/schemas/dry-run.schema.json` | Não documentava campos method de planned_params | Descrição + properties opcionais |
| GAP-AA-009 | **Baixa** | **RESOLVED** | `docs/schemas/schema.schema.json` | Texto “always 1 in 0.1.x” | “always 1; product line 1.2.x” |
| GAP-AA-010 | **Média** | **RESOLVED** | `docs/COOKBOOK.md` + `.pt-BR` | Fence Markdown quebrado; faltavam receitas dry-run method + hard-max overshoot + envelope de erro no typo | Fences + 2 receitas novas EN/PT |

### 28.3 O que a Camada Z já tinha fechado (revalidado AA — sem regressão)

- HOW_TO_USE / COOKBOOK / TESTING / CROSS_PLATFORM / schemas get-item: fail-closed method, 1.2.0 dogfood
- decisions/0002 error envelope com `command`+`duration_ms`
- item_page só como legado proibido (não como sucesso)

### 28.4 O que **não** é gap de produto em `docs/`

| Item | Motivo |
|------|--------|
| ADRs 0001–0009 com menções históricas a 1.1.x / 1.1.3 | Histórico de decisão; 0002 já em 1.2.0 |
| Seções históricas de MIGRATION sobre 0.1.x | Explicitamente rotuladas “historical” |
| COOKBOOK sem cobrir todos os 11 top-level cmds em detalhe | Cobertos em AGENTS + HOW_TO_USE + schemas |
| `docs_rules/` / `docs_prd/` | Rules internas / PRD — fora do inventário persuasivo de `docs/` |

### 28.5 Oportunidades de melhoria (não bloqueantes)

| ID | Ideia |
|----|-------|
| OPP-AA-001 | Script local `docs-version-sync` comparando `Cargo.toml` version vs pins em `docs/*` |
| OPP-AA-002 | Regenerar `docs/schemas/*` a partir de `schema --cmd all` em um único comando documentado |
| OPP-AA-003 | Diff automatizado de contagem de bullets-chave EN vs pt-BR (1.2.0, hard max, url_shape_only) |

### 28.6 Conclusão Camada AA

| Check | Resultado |
|-------|-----------|
| OPEN W+X produto | **0** (Camada Y) |
| OPEN Z docs produto | **0** (Camada Z) |
| OPEN AA reauditoria `docs/` | **0** (esta camada) |
| Versão documentada em `docs/` | **1.2.0** |
| Agent teams | **3** subagentes + fix COOKBOOK orquestrador |
| atomwrite | **Sim** |
| Publish / push / commit | **Não** (aguardando mandato) |

**Fim do append Camada AA (2026-07-19).** Próximas camadas: append-only AB+; proibido reescrever A–AA.

## 29. Camada AB — Auditoria e reescrita consolidada de `skills/` (produto 1.2.0) — 2026-07-19

> **Mandato:** auditar **obrigatoriamente** todos os arquivos em `skills/` contra rules GraphRAG de documentação (`docs_rules/rules_rust_documentacao.md`, `docs_rules/rules_rust_documentation_framework.md`), `gaps.md` Camadas Y–AA, e o binário **1.2.0**. Reescrever skills com prompts de ação/execução, fórmulas prontas, linguagem imperativa forte, description ≤1024 sem `:` no body, sem narrativa de versão, auto-contidas e bilíngues.
> **Nota de versão verbal:** pedido “v1.1.1” **não** é a linha de produto; tree/Cargo = **1.2.0** (Camada Y). Skills **não** documentam histórico de versão — só o contrato consolidado atual.

### 29.1 Inventário auditado
| Caminho | Papel |
|---------|--------|
| `skills/docsrs-cli-en/SKILL.md` | Skill imperativa EN (BLOCO B) |
| `skills/docsrs-cli-pt/SKILL.md` | Skill imperativa pt-BR espelhada |

### 29.2 Gaps encontrados (antes da AB) e status

| ID | Severidade | Status | Gap | Correção |
|----|------------|--------|-----|----------|
| GAP-AB-001 | **Alta** | **RESOLVED** | Inventário schema “13 nomes”; faltava `schema --cmd all` e aliases | 19 nomes + `schema --cmd all` no catálogo, discovery, fórmulas, description |
| GAP-AB-002 | **Alta** | **RESOLVED** | `cache path` omitido do catálogo/fórmulas (só menção lateral) | `cache path\|stats\|clear` + wire `cache-path` + campos `root`/`source`/`no_cache` |
| GAP-AB-003 | **Alta** | **RESOLVED** | Flags `--retry-max-elapsed-ms` e `--allow-loopback` ausentes | Documentadas em flags, fórmulas, allowlist, description |
| GAP-AB-004 | **Alta** | **RESOLVED** | Dry-run method sem `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe` | REQUIRED + jaq Correct Pattern + fluxo G |
| GAP-AB-005 | **Alta** | **RESOLVED** | Narrativa proibida “fail-closed since/desde 1.2.0” / legacy | Contrato fail-closed sem âncora de versão/changelog |
| GAP-AB-006 | **Média** | **RESOLVED** | Fluxos J/K ausentes (hard-max overshoot; typo method) | Workflows J + K EN/PT com argv |
| GAP-AB-007 | **Média** | **RESOLVED** | Envelope de erro: parse de `command`/`duration_ms` incompleto em exemplos | jaq + case de exit codes leem ambos |
| GAP-AB-008 | **Média** | **RESOLVED** | `truncated` incompleto (search-in-crate só limit; search-crates omitido) | limit e/ou max-output; search-crates.truncated documentado |
| GAP-AB-009 | **Baixa** | **RESOLVED** | Açúcar `name@version` e `resolved_item_path` ausentes | Catálogo + Ready Formulas + campos get-item |
| GAP-AB-010 | **Baixa** | **RESOLVED** | TOML hard-max → exit 78 não distinguido de CLI 65 | Bullet explícito Exit Codes |
| GAP-AB-011 | **Info** | **RESOLVED** | Description incompleta para ativação 1.2.0; linha lang duplicada | Description EN 941 / PT 949 chars, 0 colons, ativação proativa; lang en+pt-BR sem duplicata |
| GAP-AB-012 | **Info** | **N/A** | Skills já tinham method fail-closed + item_page reject + error envelope base (Camada Y/W-009) | Revalidado e reforçado sem narrativa de versão |

### 29.3 Mapeamento gaps.md produto (Y) → skills
| Gap produto | Contemplado na skill? |
|-------------|----------------------|
| W-001/X-001/X-002/X-006 method fail-closed + suggest | **Sim** — extraction=method only; reject item_page; fluxo B/K |
| W-002 schemas 19 | **Sim** — inventário 19 + `--cmd all` |
| W-003/X-004 error command+duration_ms | **Sim** |
| X-003/X-007 dry-run honesty | **Sim** — url_shape_only + parent_kind_probe |
| X-005 HARD_MAX fail-closed | **Sim** — exit 65 CLI / 78 TOML; fluxo J |
| W-009 skills policy item_page | **Sim** (reforçado AB) |

### 29.4 Regras de skill aplicadas
- Linguagem imperativa MUST/DEVE/NUNCA/NEVER/OBRIGATÓRIO
- REQUIRED / FORBIDDEN / Correct Pattern
- Fórmulas prontas com **todos** os 11 top-level + subcomandos cache/config + 19 schemas
- description terceira pessoa, ativação proativa, ≤1024, **zero** `:` no body
- Sem narrativa de versão/migração/changelog
- Paridade técnica EN ↔ pt-BR
- Agent teams: 1 explore auditor + orquestrador de fix

### 29.5 Verificação
| Check | Resultado |
|-------|-----------|
| description EN len / colons | 941 / 0 |
| description PT len / colons | 949 / 0 |
| Leaks de versão (1.2.0/1.1/since) no body | **0** |
| `--retry-max-elapsed-ms`, `--allow-loopback`, `cache path`, `schema --cmd all` | **presentes** |
| dry-run method keys | **presentes** |
| OPEN AB docs skills | **0** |

### 29.6 Conclusão Camada AB
| Métrica | Valor |
|---------|-------|
| OPEN W+X produto | **0** (Camada Y) |
| OPEN Z/AA docs | **0** |
| OPEN AB skills | **0** |
| Skills reescritas | EN + PT |
| Publish / push / commit | **Não** (aguardando mandato) |

**Fim do append Camada AB (2026-07-19).** Próximas camadas: append-only AC+; proibido reescrever A–AB.

## 30. Camada AC — Auditoria e correção de `CLAUDE.md` (produto 1.2.0) — 2026-07-20

> **Mandato:** auditar obrigatoriamente `/CLAUDE.md` do projeto contra o binário **1.2.0**, `gaps.md` Camadas Y–AB, rules GraphRAG de documentação, context7 (clap) e duckduckgo-search-cli (docs.rs). Corrigir todos os gaps do bloco `# docsrs-cli` e adicionar **todos** os comandos da CLI.
> **Tools:** context7 (`library clap`, `docs /websites/rs_clap`), duckduckgo-search-cli (docs.rs rustdoc), `./target/release/docsrs-cli` dogfood (`version`/`commands`/`schema --cmd all`/`cache`/`get-item --help`), atomwrite.
> **Nota de versão verbal:** “v1.1.1” **não** é a linha de produto; tree/Cargo = **1.2.0**.

### 30.1 Escopo auditado
| Caminho | Papel |
|---------|--------|
| `CLAUDE.md` (prefixo universal + CLIs auxiliares) | Memória de agente multi-produto — **preservado** |
| `CLAUDE.md` bloco `# docsrs-cli` (linhas finais) | Contrato operacional do produto — **reescrito** |

### 30.2 Gaps encontrados (antes da AC) e status

| ID | Severidade | Status | Gap | Correção |
|----|------------|--------|-----|----------|
| GAP-AC-001 | **Crítica** | **RESOLVED** | Identidade de produto `0.1.x` / `0.1.2` e FORBIDDEN proibia `1.2.0` | Linha consolidada **1.2.x / 1.2.0**; MSRV 1.88.0; proíbe ensinar 0.1.x |
| GAP-AC-002 | **Alta** | **RESOLVED** | User-Agent pinado `docsrs-cli/0.1.2` | UA padrão embutido `docsrs-cli/<versão>` |
| GAP-AC-003 | **Alta** | **RESOLVED** | Inventário schema “13 nomes”; sem `schema --cmd all` | **19 nomes** + `schema --cmd all` no catálogo e fórmulas |
| GAP-AC-004 | **Alta** | **RESOLVED** | `cache path` omitido (só stats/clear) | `cache path\|stats\|clear` + wire `cache-path` + campos `root`/`source`/`no_cache` |
| GAP-AC-005 | **Alta** | **RESOLVED** | Flags `--retry-max-elapsed-ms` e `--allow-loopback` ausentes | Documentadas em flags, fórmulas e allowlist |
| GAP-AC-006 | **Alta** | **RESOLVED** | Method `extraction` ainda aceitava `item_page` como sucesso | Fail-closed: só `extraction=method`; âncora ausente = exit 66 |
| GAP-AC-007 | **Alta** | **RESOLVED** | Ensinava envs de produto `DOCSRS_CLI_HOME` / `DOCSRS_CLI_LANG` / knobs | Paths só `--config-dir`/`--cache-dir`; lang só `--lang`/TOML; proíbe `DOCSRS_CLI_*` |
| GAP-AC-008 | **Média** | **RESOLVED** | Dry-run method sem `url_shape_only` / `planned_parent_kind` / `parent_kind_probe` | REQUIRED + jaq Correct Pattern |
| GAP-AC-009 | **Média** | **RESOLVED** | Envelope de erro sem ênfase em `command`+`duration_ms` | REQUIRED + jaq + case de exit codes |
| GAP-AC-010 | **Média** | **RESOLVED** | Hard-max overshoot (CLI 65 / TOML 78) ausente | Flags + workflows J + exit codes |
| GAP-AC-011 | **Média** | **RESOLVED** | Workflows J/K e fórmulas incompletas | Workflows A–K + fórmulas com 11 cmds + 19 schemas + cache path |
| GAP-AC-012 | **Baixa** | **RESOLVED** | `name@version`, `resolved_item_path`, `truncated` search-crates incompletos | Catálogo + JSON contract + fórmulas |
| GAP-AC-013 | **Info** | **N/A** | Prefixos atomwrite/timeout/context7/outros CLIs no CLAUDE | Fora do produto docsrs-cli — preservados sem reescrita |

### 30.3 Mapeamento gaps.md produto (Y) → CLAUDE.md
| Gap produto | Contemplado no CLAUDE? |
|-------------|------------------------|
| W-001/X-001/X-002/X-006 method fail-closed + suggest | **Sim** |
| W-002 schemas 19 + `--cmd all` | **Sim** |
| W-003/X-004 error `command`+`duration_ms` | **Sim** |
| X-003/X-007 dry-run honesty | **Sim** |
| X-005 HARD_MAX fail-closed | **Sim** |
| W-009 policy item_page | **Sim** |

### 30.4 Verificação
| Check | Resultado |
|-------|-----------|
| `version --json` binário | **1.2.0** |
| `schema --cmd all` count | **19** |
| `commands --json` top-level | **11** (+ cache/config nested) |
| identity 0.1.2 como atual no bloco | **0** |
| `cache path`, `schema --cmd all`, `--retry-max-elapsed-ms`, `--allow-loopback` | **presentes** |
| method fail-closed + hard-max + workflows J/K | **presentes** |
| atomwrite write CLAUDE.md | **Sim** (`--ack-overwrite`) |
| OPEN AC CLAUDE docsrs block | **0** |

### 30.5 Conclusão Camada AC
| Métrica | Valor |
|---------|-------|
| OPEN W+X produto | **0** (Camada Y) |
| OPEN Z/AA/AB docs+skills | **0** |
| OPEN AC CLAUDE.md | **0** |
| Publish / push / commit | **Não** (aguardando mandato) |

**Fim do append Camada AC (2026-07-20).** Próximas camadas: append-only AD+; proibido reescrever A–AC.
