[Português (pt-BR)](0008-domain-types-posture.pt-BR.md)

# ADR 0008 — Domain-type crates posture (chrono · uuid · rust_decimal · url)

## Status
- Accepted (2026-07-19)

## Context
- Generic Rules Rust recommend a coordinated stack of **chrono**, **uuid**, **rust_decimal**, and **url** with serde features and e-commerce-style newtypes (`UserId`, `Money`, …).
- `docsrs-cli` is a **one-shot** agent CLI: argv → domain newtypes → allowlisted **HTTPS GET** → emit → DIE. There is no application database, payment domain, session store, or multi-entity identity model.
- Camada L (ADR 0006) already established product newtypes and parse-don't-validate on the core path. Camada K (ADR 0005) fixed serde/validation posture without `validator`.
- ADR 0001 already rejected **chrono** for HTTP-date `Retry-After` in favor of lightweight **httpdate**.

## Decision

### 1. Only `url` of the four crates is product-required

| Crate | Product role | Decision |
|-------|--------------|----------|
| **url 2** | Request construction, `final_url`, href join, cache provenance re-parse | **Required** (direct dependency) |
| **chrono 0.4** | Wall-clock `DateTime<Utc>` for APIs/DB | **N/A — do not add** |
| **uuid 1** | Entity IDs / B-tree PKs | **N/A — do not add** |
| **rust_decimal 1** | Monetary arithmetic | **N/A — do not add** |

### 2. Time model (no chrono)

| Need | Mechanism |
|------|-----------|
| Retry/backoff budgets, host politeness, cancel-aware sleeps | Monotonic **`std::time::Instant`** / **`tokio::time`** |
| Disk cache TTL / `stored_at_unix` | **`SystemTime`** + UNIX epoch seconds |
| HTTP `Retry-After` IMF-fixdate | **`httpdate`** (not chrono) |
| Operator-facing “now” in JSON | `duration_ms` from Instant only — no wall-clock timezone fields |

- Never use `Local::now()` or serialize `DateTime<Local>` for product APIs.
- Do not introduce `chrono` solely to satisfy a four-crate checklist.

### 3. Identity model (no uuid)

- Disk cache keys are **SHA-256 hex** of `(url, parser version, accept)` — content-addressed, not random UUIDs.
- One-shot processes have no session/token identity that would require UUID v4/v7.
- If a future feature needs IDs, open a new ADR; do not default to `Uuid::nil()`.

### 4. Money / decimals (no rust_decimal)

- Product has **no** prices, balances, or tax math.
- **Forbidden:** `f64`/`f32` for invented “money-like” fields. Today there are none.
- Do not add `rust_decimal` as dead weight.

### 5. URL posture (applies)

1. **Parse at boundaries:** config origins via `AllowedOrigin::parse` → internal `Url::parse` / builders return `Url`.
2. **Core path proof:** URL builders and fetch helpers that take an origin take **`&AllowedOrigin`**, not bare `impl AsRef<str>`.
3. **HTTP client:** `HttpClient::get_json` / `get_html` take **`&Url`**; `HttpResponse.final_url` is **`Url`**.
4. **Relative href:** resolve with **`Url::join`** against a known base (search hits).
5. **Cache:** meta stores `final_url` as string; load path **re-parses** and re-checks allowlist (poison resistance).
6. **Wire / stdout:** `source_url` and echo URL fields remain **plain `String`** (stable agent schemas — ADR 0006). Do **not** enable the `url` crate **`serde`** feature until an internal type actually serializes `Url` (wire stays strings).
7. **Caps:** hostile/optional API URL fields capped with `MAX_URL_FIELD_CHARS`.

### 6. Product newtypes (not e-commerce)

Correct domain types for this CLI:

- `CrateName`, `VersionArg`, `SearchQuery`, `ItemPath`, `CrateRef`, `MatchMode`, `AllowedOrigin`

Explicit non-goals: `UserId`, `OrderId`, `Money`, `Email` as product types.

### 7. Cargo / MSRV

- Single package (not a Cargo workspace): pin via **`Cargo.lock`** for the binary; `url = "2"` with posture comments in `Cargo.toml`.
- MSRV is the package `rust-version` (aligned with toolchain); no artificial MSRV raise for unused chrono/uuid/decimal.

### 8. One-shot · memory · parallelism

- **One-shot:** no long-lived UUID sessions or chrono clocks as process identity.
- **Memory:** `Url` clones only where needed; body and URL field caps remain.
- **Parallelism:** `Url` / `AllowedOrigin` are `Send + Sync`; no shared mutable URL config.

## Consequences

- Contributors **must not** add `chrono` / `uuid` / `rust_decimal` “for completeness.”
- `cargo tree -i chrono` / `uuid` / `rust_decimal` should remain empty unless a future ADR revises this.
- Doctor may report domain-type posture (`url` present; the three others absent).
- Changing wire `source_url` from `String` to serialized `Url` is a **breaking** schema change and needs its own decision.

## Related
- ADR 0001 HTTP retry (httpdate) · ADR 0005 serde pipeline · ADR 0006 type system · ADR 0003 web-fetch scope
- `src/domain/*` · `src/docs_rs/urls.rs` · `src/http/client.rs` · `src/cache/disk.rs`
- Gaps inventory: Camada N (domain types)
