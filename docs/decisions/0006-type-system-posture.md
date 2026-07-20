[Português (pt-BR)](0006-type-system-posture.pt-BR.md)

# ADR 0006 — Type-system posture for docsrs-cli

## Status
- Accepted (2026-07-19)

## Context
- Generic Rules Rust emphasize parse-don't-validate, newtypes, unit types, typestate, and strong naming.
- `docsrs-cli` is a **one-shot** agent CLI: argv → bounded HTTPS GET → emit → DIE (no long-lived session state machine).
- Domain newtypes already exist (`CrateName`, `ItemPath`, `SearchQuery`, `VersionArg`, `CrateRef`, `MatchMode`) but, historically, the **core path** after `ops` re-lowered values to `&str`, so the compiler stopped proving validity in `docs_rs` / `crates_io`.
- Camada K (ADR 0005) fixed the serde/validation *crate* posture; this ADR fixes the *type* posture.

## Decision

### 1. Layers of types
| Layer | Role | Examples |
|-------|------|----------|
| **Domain** | Invariants in the type; fallible `parse` / `TryFrom` only | `CrateName`, `VersionArg`, `AllowedOrigin`, `SearchQuery` |
| **Config DTO** | Runtime policy after load/clamp; resource ceilings as named `u64` + `Duration` helpers | `Config.timeout_secs`, `Config::timeout()` |
| **Wire DTO** | stdout/HTTP JSON; plain `String`/`u64`/`bool` | `ReadmeData`, `SearchCratesData`, envelopes |

### 2. Parse-don't-validate (core path)
1. Parse argv at the handler frontier (`ops` / clap `String` → domain).
2. Propagate **`&CrateName`**, **`&VersionArg`**, **`&SearchQuery`**, **`&AllowedOrigin`** through URL builders, fetch, search, and crates.io planners.
3. Re-lower with `.as_str()` only for formatting, progress text, dry-run echo DTOs, and hash inputs.
4. Do **not** keep public “validate then discard” wrappers (`validate_crate_name() -> ()`).

### 3. AllowedOrigin
- Origins in `Config` are `AllowedOrigin`, constructed only via the allowlist parse (former `validate_origin`).
- Prevents a future bug from assigning an arbitrary `String` origin without re-validation.

### 4. Explicit non-goals (one-shot)
- **No typestate** on `HttpClient` / lifecycle — cancel is `CancelFlag`, not compile-time phases.
- **No `Deref`** on domain newtypes (would erase type safety).
- **No infallible `From<String>`** into validated newtypes.
- **No unit newtype per Config field** (`TimeoutSecs`) — field names encode units; methods return `Duration`.
- **No domain newtypes inside stdout wire DTOs** — agent JSON stays plain strings (stable schemas).
- HTML scrape helpers may keep `&str` (hostile external text is not pre-validated crate names).

### 5. Naming
- `as_str` = free borrow; `to_*` only when allocating; no `get_` field getters.
- `get_item` / `get_json` / `get_html` are **command / HTTP verbs**, not field getters.

### 6. Zero-cost
- String newtypes use `#[repr(transparent)]` where they wrap a single `String`.
- Prefer `&Newtype` over cloning through layers.

## Consequences
- Internal APIs that *assume* a valid crate/version/query **must** take the domain type.
- Tests construct values via `CrateName::parse("serde").unwrap()` (or similar) instead of raw strings on core-path functions.
- Complements ADR 0005 (serde crates) and ADR 0004 (threat model / allowlist).

## Related
- ADR 0002 error model · ADR 0005 serde pipeline · `src/domain/*`
