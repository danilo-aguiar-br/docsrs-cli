[English](0005-serde-validation-pipeline.md)

# ADR 0005 — Pipeline serde / validação do docsrs-cli

## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust genéricas descrevem pipeline de **4 crates**: `serde` + `serde_json` + `validator` 0.20 + `serde_with` 3
- `docsrs-cli` é CLI **one-shot** para agentes (não API HTTP multi-tenant / forms)
- Fronteiras de entrada atuais:
  1. **CLI argv** → clap → newtypes de domínio
  2. **XDG `config.toml`** → leitura com teto de tamanho → `toml::from_str::<TomlConfig>` com `deny_unknown_fields` → apply → **clamp** de tetos → `validate_security`
  3. **JSON HTTP** (crates.io) → Content-Type → `serde_json::from_str` → map + **caps** de campo
  4. **Meta de cache em disco** → `from_slice` com teto → integridade (checksums, allowlist)
  5. **stdout** → envelopes só `Serialize` (wire de escrita)

## Decisão
1. Declarar **apenas** as crates usadas: `serde` 1 + derive e `serde_json` 1.
2. **Não** adicionar `validator` nem `serde_with` sem DTO que se beneficie (sem deps mortas em binário one-shot).
3. Validação de domínio permanece **parse-don't-validate** (newtypes / `TryFrom` / `FromStr`).
4. Tipos **write-only** mantêm **só `Serialize`**.
5. Tetos de recurso de config usam **clamp**; campos de segurança **rejeitam**.
6. Containers inbound críticos usam `deny_unknown_fields` (`TomlConfig`, `CacheMeta`).
7. DTOs de API externa ficam permissivos a chaves novas e aplicam **caps** no map.
8. Reavaliar se surgir API JSON de **input** de primeira parte.

## Consequências
- Checklist “quatro crates” **não** é cumprido para crates sem uso; a **intenção** de camadas é cumprida pelo pipeline acima.
- Binário sem superfície proc-macro/validação ociosa.
- Não cargo-cultar `validator` em newtypes de domínio.
- `docs/schemas/*.json` documentam o contrato do agente.

## Pipeline

```text
parse (bytes/argv) → serde tipado → validação de domínio / map → uso
```

## Relacionados
- ADR 0002 (modelo de erro)
- ADR 0004 (modelo de ameaças)
