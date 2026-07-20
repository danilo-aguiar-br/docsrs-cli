[Português (pt-BR)](0005-serde-validation-pipeline.pt-BR.md)

# ADR 0005 — Serde / validation pipeline for docsrs-cli

## Status
- Accepted (2026-07-19)

## Context
- Generic Rules Rust describe a **4-crate** pipeline: `serde` + `serde_json` + `validator` 0.20 + `serde_with` 3
- `docsrs-cli` is a **one-shot** agent CLI (not a multi-tenant HTTP API / form server)
- Input frontiers today:
  1. **CLI argv** → clap → domain newtypes (`CrateName`, `ItemPath`, `SearchQuery`, `VersionArg`, …)
  2. **XDG `config.toml`** → size-capped read → `toml::from_str::<TomlConfig>` with `deny_unknown_fields` → apply → **clamp** resource ceilings → `validate_security` (origins allowlist, UA ASCII)
  3. **HTTP JSON** (crates.io) → Content-Type gate → `serde_json::from_str` → map + **field caps** → agent DTO
  4. **Disk cache meta** → size-capped `from_slice` → integrity (checksums, allowlist `final_url`)
  5. **stdout** → `Serialize`-only envelopes (write-only wire)

## Decision
1. **Declare only the crates we use:** `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"`.
2. **Do not** add `validator` or `serde_with` without a concrete DTO that benefits (no dead dependencies in a one-shot binary).
3. **Domain validation** stays **parse-don't-validate** via newtypes + `TryFrom`/`FromStr`/`parse` in `domain.rs` and `config/validate.rs` — not `#[derive(Validate)]`.
4. **Write-only** types (`SuccessEnvelope`, `ErrorEnvelope`, dry-run / doctor / meta DTOs) keep **`Serialize` only** — requiring `Deserialize` on output-only wire is incorrect.
5. **Config resource ceilings** use **clamp** (fail-closed upper bound) after TOML/env/CLI; security-sensitive fields (origins, UA, contact) **reject** via `validate_security`.
6. **Critical inbound containers** use `#[serde(deny_unknown_fields)]` (`TomlConfig`, `CacheMeta`).
7. **External API DTOs** (`ApiCrate`, …) stay permissive on unknown keys (crates.io evolves) and apply **length caps** on map to product types.
8. Revisit this ADR if the product gains a first-party JSON **input** API (stdin protocol, plugin config forms, email/phone fields, `Duration` as JSON number/string dual form).

## Consequences
- Checklist “must declare four crates” is **intentionally not met** for unused crates; the **intent** of layered validation is met by the pipeline above.
- Binary stays free of unused proc-macro / validation surface.
- Future contributors must not cargo-cult `validator` onto domain newtypes (would duplicate charset/length rules and blur error kinds).
- `docs/schemas/*.json` remain the agent contract documentation (no runtime `schemars` required for this CLI).

## Pipeline (canonical order)

```text
parse (bytes/argv) → serde typed → domain validate / map → use
```

| Frontier | Parse | Serde | Domain |
|----------|-------|-------|--------|
| argv | clap | — | newtypes |
| config.toml | UTF-8 + size cap | `TomlConfig` + deny_unknown | clamp + `validate_security` |
| crates.io JSON | HTTP body + CT | `ApiResponse` | map + caps |
| cache meta | size cap | `CacheMeta` + deny_unknown | hex digests + checksums |
| stdout | — | Serialize only | — |

## Related
- ADR 0002 error model (`ErrorKind::Parse` / `Config` / `Budget`)
- ADR 0004 threat model (untrusted HTTP + cache files)
