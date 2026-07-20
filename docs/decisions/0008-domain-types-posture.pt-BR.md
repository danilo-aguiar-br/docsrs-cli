[English](0008-domain-types-posture.md)

# ADR 0008 — Postura de crates de tipos de domínio (chrono · uuid · rust_decimal · url)

## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust genéricas recomendam a pilha coordenada **chrono**, **uuid**, **rust_decimal** e **url** com features serde e newtypes de e-commerce (`UserId`, `Money`, …).
- `docsrs-cli` é CLI **one-shot** para agentes: argv → newtypes de domínio → **GET HTTPS** allowlisted → emitir → DIE. Não há banco de aplicação, domínio de pagamento, sessão nem modelo multi-entidade.
- Camada L (ADR 0006) já estabeleceu newtypes de produto e parse-don't-validate no core path. Camada K (ADR 0005) fixou serde/validação sem `validator`.
- ADR 0001 já rejeitou **chrono** para HTTP-date em `Retry-After` em favor de **httpdate**.

## Decisão

### 1. Só `url` das quatro crates é obrigatória no produto

| Crate | Papel no produto | Decisão |
|-------|------------------|----------|
| **url 2** | Construção de request, `final_url`, join de href, re-parse de cache | **Obrigatória** (dep direta) |
| **chrono 0.4** | `DateTime<Utc>` de API/DB | **N/A — não adicionar** |
| **uuid 1** | IDs de entidade / PK B-tree | **N/A — não adicionar** |
| **rust_decimal 1** | Aritmética monetária | **N/A — não adicionar** |

### 2. Modelo de tempo (sem chrono)

| Necessidade | Mecanismo |
|-------------|-----------|
| Orçamentos de retry/backoff, politeness, sleeps canceláveis | **`Instant`** monotônico / **`tokio::time`** |
| TTL de cache / `stored_at_unix` | **`SystemTime`** + epoch UNIX |
| `Retry-After` IMF-fixdate | **`httpdate`** (não chrono) |
| “Agora” no JSON | só `duration_ms` a partir de Instant — sem campos de fuso |

- Nunca `Local::now()` nem serializar `DateTime<Local>` em API de produto.
- Não introduzir `chrono` só para checklist de quatro crates.

### 3. Modelo de identidade (sem uuid)

- Chaves de cache = **SHA-256 hex** de `(url, parser, accept)`.
- Processo one-shot não tem sessão/token que exija UUID v4/v7.
- Feature futura de IDs exige novo ADR; não usar `Uuid::nil()` por padrão.

### 4. Dinheiro / decimais (sem rust_decimal)

- Produto **não** tem preços, saldos ou impostos.
- **Proibido:** `f64`/`f32` para campos “quase monetários”. Hoje não existem.
- Não adicionar `rust_decimal` como peso morto.

### 5. Postura de URL (aplicável)

1. **Parse na boundary:** origins de config via `AllowedOrigin::parse`; builders retornam `Url`.
2. **Prova no core path:** helpers com origin recebem **`&AllowedOrigin`**, não `impl AsRef<str>` nu.
3. **Cliente HTTP:** `get_json` / `get_html` com **`&Url`**; `final_url: Url`.
4. **Href relativo:** **`Url::join`** contra base conhecida.
5. **Cache:** re-parse + allowlist no load (anti-poison).
6. **Wire / stdout:** `source_url` permanece **`String`** (ADR 0006). **Não** habilitar feature **`serde`** de `url` até existir serialização interna de `Url`.
7. **Caps:** `MAX_URL_FIELD_CHARS` em campos de URL opcionais da API.

### 6. Newtypes de produto (não e-commerce)

Tipos corretos: `CrateName`, `VersionArg`, `SearchQuery`, `ItemPath`, `CrateRef`, `MatchMode`, `AllowedOrigin`.

Fora de escopo: `UserId`, `OrderId`, `Money`, `Email` como tipos de produto.

### 7. Cargo / MSRV

- Pacote único: pin via **`Cargo.lock`**; `url = "2"` com comentários de postura.
- MSRV = `rust-version` do pacote; sem subir MSRV por deps não usadas.

### 8. One-shot · memória · paralelismo

- **One-shot:** sem sessão UUID ou relógio chrono como identidade de processo.
- **Memória:** clones de `Url` só onde necessário; caps de body e URL.
- **Paralelismo:** `Url` / `AllowedOrigin` são `Send + Sync`.

## Consequências

- Contribuidores **não** devem adicionar `chrono` / `uuid` / `rust_decimal` “por completude”.
- `cargo tree -i chrono|uuid|rust_decimal` deve permanecer vazio até novo ADR.
- Doctor pode reportar postura de tipos de domínio.
- Mudar `source_url` de `String` para `Url` serializado é **breaking** de schema.

## Relacionados
- ADR 0001 · 0005 · 0006 · 0003
- `src/domain/*` · `src/docs_rs/urls.rs` · `src/http/client.rs` · `src/cache/disk.rs`
- Inventário de gaps: Camada N
