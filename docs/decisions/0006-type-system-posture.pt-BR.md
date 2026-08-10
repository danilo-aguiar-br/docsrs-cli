[English](0006-type-system-posture.md)

# ADR 0006 — Postura do sistema de tipos no docsrs-cli

## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust genéricas enfatizam parse-don't-validate, newtypes, unidades, typestate e nomenclatura forte.
- `docsrs-cli` é CLI **one-shot**: argv → GET HTTPS limitado → emit → DIE (sem máquina de estados de sessão).
- Newtypes de domínio já existiam, mas o **core path** após `ops` rebaixava para `&str`, e o compilador deixava de provar validade em `docs_rs` / `crates_io`.
- Camada K (ADR 0005) fechou a postura de *crates* serde; este ADR fecha a postura de *tipos*.

## Decisão

### 1. Camadas de tipos
| Camada | Papel | Exemplos |
|--------|-------|----------|
| **Domínio** | Invariantes no tipo; só `parse` / `TryFrom` falível | `CrateName`, `VersionArg`, `AllowedOrigin`, `SearchQuery` |
| **DTO Config** | Política runtime pós-load/clamp; tetos como `u64` nomeado + helpers `Duration` | `Config.timeout_secs`, `Config::timeout()` |
| **DTO wire** | JSON stdout/HTTP; `String`/`u64`/`bool` planos | `ReadmeData`, envelopes |

### 2. Parse-don't-validate (core path)
1. Parsear argv na fronteira do handler.
2. Propagar **`&CrateName`**, **`&VersionArg`**, **`&SearchQuery`**, **`&AllowedOrigin`** por URLs, fetch, search e planejadores crates.io.
3. Rebaixar com `.as_str()` só para format, progresso, dry-run e hashes.
4. **Não** manter wrappers públicos “valida e descarta”.

### 3. AllowedOrigin
- Origins em `Config` são `AllowedOrigin` (parse com allowlist).
- Impede atribuição futura de origin arbitrário sem revalidação.

### 4. Não-objetivos (one-shot)
- Sem typestate em `HttpClient`.
- Sem `Deref` em newtypes de domínio.
- Sem `From<String>` infalível para newtypes validados.
- Sem newtype de unidade por campo de Config.
- Sem newtypes de domínio dentro de DTOs stdout.
- Scrape HTML pode permanecer `&str`.

### 5. Nomenclatura
- `as_str` = empréstimo barato; `to_*` somente quando aloca; sem getters de campo com `get_`.
- `get_item` / `get_json` / `get_html` são **verbos de comando / HTTP**, não getters de campo.

### 6. Zero-cost
- Newtypes de string usam `#[repr(transparent)]` onde embrulham uma única `String`.
- Preferir `&Newtype` a clones entre camadas.

## Consequências
- APIs internas que *assumem* crate/versão/query válidos **devem** receber o tipo de domínio.
- Testes usam `CrateName::parse(...)` no core path.
- Complementa ADR 0005 e ADR 0004.

## Relacionados
- ADR 0002 · ADR 0005 · `src/domain/*`
