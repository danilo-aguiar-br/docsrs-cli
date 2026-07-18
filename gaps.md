# gaps.md — Fechamento total v1.1.0 (GAP-001…024 CLOSED)
**Data:** 2026-07-18  
**Binário:** `target/release/docsrs-cli` **1.1.0**  
**Escopo:** todos os gaps da auditoria 2026-07-18 **fechados** (código + testes + e2e live)  
**Proibições:** sem push, sem publish crates.io, sem CI/CD, sem telemetria remota  

---

## Sumário
| Dimensão | Resultado |
|----------|-----------|
| Versão | **1.1.0** |
| `cargo test --locked` | **OK** |
| `cargo clippy -D warnings` | **OK** |
| `cargo build --release` | **OK** |
| Matriz e2e live GAP-001…024 | **24/24 CLOSED** (`OPEN_COUNT=0`) |
| Residual GAP-020 | **CLOSED** — `data.cache_hit` no wire |

---

## Status por GAP
| ID | Status | Evidência |
|----|--------|-----------|
| GAP-001 | **CLOSED** | `MatchMode` prefix/exact/substring; Serialize first; sem Deserialize* no default |
| GAP-002 | **CLOSED** | URL `…/struct.Runtime.html#method.new`; `item_name=new` |
| GAP-003 | **CLOSED** | `readme std` → `resolved_version=stable` |
| GAP-004 | **CLOSED** | scrape SemVer **só do crate alvo**; `tokio latest` → `1.53.0` (não deps) |
| GAP-005 | **CLOSED** | `item_name` em GetItemData + schema |
| GAP-006 | **CLOSED** | `item_type` + `match_mode` ecoados |
| GAP-007 | **CLOSED** | dry-run/sucesso usam `crate_name` |
| GAP-008 | **CLOSED** | dry-run `--limit 5000` → planned 1000 |
| GAP-009 | **CLOSED** | `--page-token` aceita query completa **sem** positional query |
| GAP-010 | **CLOSED** | completions pipe = script bash cru |
| GAP-011 | **CLOSED** | UA default `github.com/danilo-aguiar-br/docsrs-cli` |
| GAP-012 | **CLOSED** | knobs de produto: flags + XDG TOML only; README alinhado |
| GAP-013 | **CLOSED** | `from_href` classifica `index` e `#method.` |
| GAP-014 | **CLOSED** | sanitize + main-content path |
| GAP-015 | **CLOSED** | `--suggest` em get-item 404 |
| GAP-016 | **CLOSED** | suite timeout/offline |
| GAP-017 | **CLOSED** | JSON EN; i18n stderr |
| GAP-018 | **CLOSED** | canônico `crate_name` |
| GAP-019 | **CLOSED** | sort por score |
| GAP-020 | **CLOSED** | `data.cache_hit: bool` em search-crates/readme/get-item/search-in-crate; miss→false, hit→true |
| GAP-021 | **CLOSED** | methods via get-item + classificador |
| GAP-022 | **CLOSED** | `doctor --online` DNS crates.io/docs.rs |
| GAP-023 | **CLOSED** | mesmo que GAP-009 |
| GAP-024 | **CLOSED** | `join_href` absolutos; path method |

---

## Provas live (pós-fix final)
```text
docsrs-cli version → 1.1.0
search-in-crate serde Serialize --item-type trait → Serialize first, match_mode=prefix
get-item tokio fn runtime::Runtime::new → #method.new, resolved_version=1.53.0
readme std → resolved_version=stable
readme tokio --crate-version latest → resolved_version=1.53.0
search-crates --page-token '?q=tokio&…&page=2' → ok (sem query posicional)
--cache-dir $TMP readme serde (1ª) → cache_hit=false
--cache-dir $TMP readme serde (2ª) → cache_hit=true, duration_ms≈2
doctor --online → online_crates_io/docs_rs ok
OPEN_COUNT=0
```

---

## Declaração
- Nenhum GAP da auditoria 2026-07-18 permanece aberto no código
- Documentação pública bilíngue, schemas e skills sincronizados com a superfície 1.1.0
- Pasta `docs/` reauditada e fechada contra o framework GraphRAG e GAP-001…024 (guias, schemas, ADRs)
- Schemas vivos incluem `schema`, `completions`, `error` e `dry-run` além dos contratos de rede/ops
- Gaps residuais de documentação também fechados:
  - zero linhas em branco pós-heading em raiz, `docs/`, skills e `llms*`
  - ADRs em bullets com pares `.pt-BR` no tarball
  - `additionalProperties: false` em objetos fechados (bags livres `planned_params` / `schema` permanecem abertos)
  - MIGRATION com before/after JSON (`substring→prefix`, `crate→crate_name`)
  - HOW_TO_USE com tabela Feature 1.1 → guia (EN+PT)
  - inventário completo de 13 schemas em README, skills e `llms*`
- Produto: one-shot BORN→DIE, sem telemetria remota, sem CI/CD/push/publish

Fechamento total v1.1.0 — docsrs-cli
