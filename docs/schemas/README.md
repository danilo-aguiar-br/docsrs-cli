[Português (pt-BR)](#português-brasileiro)

# JSON Schemas
Machine-readable payload contracts for docsrs-cli command success `data` objects.

## English
- Schemas describe the object under envelope field `data`
- Outer envelope always carries `schema_version`, `ok`, `command`, and `duration_ms` on **success and error** (1.2.0+; error also has nested `error`)
- File names are kebab-case mirrors of command names
- Load a live schema from the binary with `docsrs-cli schema --cmd <name> --json`
- Network schemas require `cache_hit` and use canonical `crate_name`; method success includes `extraction=method` only (missing anchors are errors, not `item_page` success)
- Use `schema --cmd all --json` for the full 20-name offline/runtime bundle (includes cache/config aliases)
- `error.kind=budget` means local body/output cap exceeded (exit 74, `retryable=false`); do not confuse with retryable `network` on the same exit code
- Outer envelope `ok` usually tracks success; for `doctor`, envelope `ok` mirrors `data.ok` and may be `false` with process exit 78 (still a success envelope, not `error`)
- `search-crates` echo fields `query` / `page` / `per_page` / `sort` always match the effective request URL after `--page-token`
- Object schemas set `additionalProperties: false` except free-form bags (`planned_params`, embedded `schema` document)

### Inventory
- [search-crates.schema.json](search-crates.schema.json) — crates.io search hit list with `cache_hit`, effective-URL echo, and `meta.next_page` / `prev_page`
- [readme.schema.json](readme.schema.json) — docs.rs crate overview with `crate_name`, optional `resolved_version`, `cache_hit`
- [get-item.schema.json](get-item.schema.json) — typed rustdoc item with `item_name`, optional `resolved_version`, `cache_hit`, optional `extraction`, and `anchor_family` reporting the real rustdoc family
- [search-in-crate.schema.json](search-in-crate.schema.json) — in-crate symbol search with `match_mode`, optional `item_type`, hit `score`, `cache_hit`
- [commands.schema.json](commands.schema.json) — command tree discovery payload
- [doctor.schema.json](doctor.schema.json) — readiness checks payload including optional `online_*` checks; envelope `ok` mirrors `data.ok`
- [version.schema.json](version.schema.json) — binary identity payload
- [cache.schema.json](cache.schema.json) — cache stats or clear payload (wire aliases also: cache-path, cache-clear, cache-stats)
- [cache-path.schema.json](cache-path.schema.json) / [cache-clear.schema.json](cache-clear.schema.json) / [cache-stats.schema.json](cache-stats.schema.json) — same body as cache (offline discovery parity with `schema --cmd all`)
- [config.schema.json](config.schema.json) — config path, show, or init payload (wire aliases: config-path, config-show, config-init)
- [config-path.schema.json](config-path.schema.json) / [config-show.schema.json](config-show.schema.json) / [config-init.schema.json](config-init.schema.json) — same body as config
- [schema.schema.json](schema.schema.json) — meta schema payload, in two shapes: one document for `schema --cmd <name>`, and the `mode`/`commands`/`items` bundle for `schema --cmd all`
- [completions.schema.json](completions.schema.json) — completions JSON mode payload (`shell`, `script`)
- [error.schema.json](error.schema.json) — top-level JSON error envelope (not under `data`); includes `budget`, the `io` kind at exit 74, and `suggestions`, which carries the `--suggest` ranking as data instead of prose
- [dry-run.schema.json](dry-run.schema.json) — dry-run envelope fragment with `planned_url` / `planned_params`; for get-item associated-item dry-run, `planned_params` may also include `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe` and `planned_method_anchors`; `search-crates` plans a query, so its `planned_params` carry no `crate_name`
- [agent-surface.schema.json](agent-surface.schema.json) — reduction report attached beside `data`, never inside it, when any reduction knob is active; `--max-output-bytes` alone does not attach it, and an `ok=false` envelope never carries it

### Architecture decisions
- [0001-http-retry-policy.md](../decisions/0001-http-retry-policy.md) / [pt-BR](../decisions/0001-http-retry-policy.pt-BR.md) — HTTP retry policy
- [0002-error-model.md](../decisions/0002-error-model.md) / [pt-BR](../decisions/0002-error-model.pt-BR.md) — error model
- [0003-web-fetch-scope.md](../decisions/0003-web-fetch-scope.md) / [pt-BR](../decisions/0003-web-fetch-scope.pt-BR.md) — web fetch scope
- [0004-threat-model.md](../decisions/0004-threat-model.md) / [pt-BR](../decisions/0004-threat-model.pt-BR.md) — STRIDE threat model
- [0005-serde-validation-pipeline.md](../decisions/0005-serde-validation-pipeline.md) / [pt-BR](../decisions/0005-serde-validation-pipeline.pt-BR.md) — serde and validation pipeline
- [0006-type-system-posture.md](../decisions/0006-type-system-posture.md) / [pt-BR](../decisions/0006-type-system-posture.pt-BR.md) — type-system posture
- [0007-rustls-posture.md](../decisions/0007-rustls-posture.md) / [pt-BR](../decisions/0007-rustls-posture.pt-BR.md) — rustls TLS posture
- [0008-domain-types-posture.md](../decisions/0008-domain-types-posture.md) / [pt-BR](../decisions/0008-domain-types-posture.pt-BR.md) — domain-type crates posture
- [0009-unsafe-ffi-posture.md](../decisions/0009-unsafe-ffi-posture.md) / [pt-BR](../decisions/0009-unsafe-ffi-posture.pt-BR.md) — unsafe code and FFI posture
- [0010-explicit-target-designation.md](../decisions/0010-explicit-target-designation.md) / [pt-BR](../decisions/0010-explicit-target-designation.pt-BR.md) — a destructive verb needs its target named in argv or waived, or it refuses and destroys nothing

## Português Brasileiro
- Schemas descrevem o objeto sob o campo `data` do envelope
- O envelope externo sempre carrega `schema_version`, `ok`, `command` e `duration_ms`
- Nomes de arquivo são kebab-case espelhando nomes de comando
- Carregue um schema vivo do binário com `docsrs-cli schema --cmd <name> --json`
- Schemas de rede exigem `cache_hit` e usam `crate_name` canônico; sucesso de method inclui apenas `extraction=method` (âncoras ausentes são erros, não sucesso `item_page`)
- Use `schema --cmd all --json` para o bundle completo de 20 nomes offline/runtime (inclui aliases cache/config)
- `error.kind=budget` significa teto local de body/output (exit 74, `retryable=false`); não confunda com `network` retryable no mesmo exit code
- O `ok` do envelope externo em geral acompanha sucesso; em `doctor`, o `ok` do envelope espelha `data.ok` e pode ser `false` com exit 78 (ainda é envelope de sucesso, não `error`)
- Campos de eco de `search-crates` (`query` / `page` / `per_page` / `sort`) sempre batem com a URL efetiva após `--page-token`
- Schemas de objeto usam `additionalProperties: false`, exceto bags livres (`planned_params`, documento `schema` embutido)

### Inventário
- [search-crates.schema.json](search-crates.schema.json) — lista de hits da busca no crates.io com `cache_hit`, eco da URL efetiva e `meta.next_page` / `prev_page`
- [readme.schema.json](readme.schema.json) — overview do crate no docs.rs com `crate_name`, `resolved_version` opcional, `cache_hit`
- [get-item.schema.json](get-item.schema.json) — item rustdoc tipado com `item_name`, `resolved_version` opcional, `cache_hit`, `extraction` opcional, e `anchor_family` reportando a família rustdoc real
- [search-in-crate.schema.json](search-in-crate.schema.json) — busca de símbolos com `match_mode`, `item_type` opcional, `score`, `cache_hit`
- [commands.schema.json](commands.schema.json) — payload de descoberta da árvore de comandos
- [doctor.schema.json](doctor.schema.json) — payload de checks de prontidão incluindo checks `online_*` opcionais; `ok` do envelope espelha `data.ok`
- [version.schema.json](version.schema.json) — payload de identidade do binário
- [cache.schema.json](cache.schema.json) — payload de stats ou clear do cache (aliases no wire também: cache-path, cache-clear, cache-stats)
- [cache-path.schema.json](cache-path.schema.json) / [cache-clear.schema.json](cache-clear.schema.json) / [cache-stats.schema.json](cache-stats.schema.json) — mesmo corpo que cache (paridade offline de descoberta com `schema --cmd all`)
- [config.schema.json](config.schema.json) — payload de path, show ou init da config (aliases no wire: config-path, config-show, config-init)
- [config-path.schema.json](config-path.schema.json) / [config-show.schema.json](config-show.schema.json) / [config-init.schema.json](config-init.schema.json) — mesmo corpo que config
- [schema.schema.json](schema.schema.json) — payload meta em duas formas: um documento para `schema --cmd <nome>`, e o bundle `mode`/`commands`/`items` para `schema --cmd all`
- [completions.schema.json](completions.schema.json) — payload do modo JSON de completions (`shell`, `script`)
- [error.schema.json](error.schema.json) — envelope JSON de erro no topo (não sob `data`); inclui `budget`, o kind `io` no exit 74, e `suggestions`, que publica o ranking do `--suggest` como dado em vez de prosa
- [dry-run.schema.json](dry-run.schema.json) — fragmento de envelope dry-run com `planned_url` / `planned_params`; no dry-run de item associado de get-item, `planned_params` pode incluir `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe` e `planned_method_anchors`; `search-crates` planeja uma consulta, então seu `planned_params` não traz `crate_name`
- [agent-surface.schema.json](agent-surface.schema.json) — relatório de redução anexado ao lado de `data`, nunca dentro dele, quando qualquer knob de redução está ativo; `--max-output-bytes` sozinho não o anexa, e envelope com `ok=false` nunca o carrega

### Decisões de arquitetura
- [0001-http-retry-policy.md](../decisions/0001-http-retry-policy.md) / [pt-BR](../decisions/0001-http-retry-policy.pt-BR.md) — política de retry HTTP
- [0002-error-model.md](../decisions/0002-error-model.md) / [pt-BR](../decisions/0002-error-model.pt-BR.md) — modelo de erro
- [0003-web-fetch-scope.md](../decisions/0003-web-fetch-scope.md) / [pt-BR](../decisions/0003-web-fetch-scope.pt-BR.md) — escopo de busca na web
- [0004-threat-model.md](../decisions/0004-threat-model.md) / [pt-BR](../decisions/0004-threat-model.pt-BR.md) — modelo de ameaças STRIDE
- [0005-serde-validation-pipeline.md](../decisions/0005-serde-validation-pipeline.md) / [pt-BR](../decisions/0005-serde-validation-pipeline.pt-BR.md) — pipeline de serde e validação
- [0006-type-system-posture.md](../decisions/0006-type-system-posture.md) / [pt-BR](../decisions/0006-type-system-posture.pt-BR.md) — postura do sistema de tipos
- [0007-rustls-posture.md](../decisions/0007-rustls-posture.md) / [pt-BR](../decisions/0007-rustls-posture.pt-BR.md) — postura TLS com rustls
- [0008-domain-types-posture.md](../decisions/0008-domain-types-posture.md) / [pt-BR](../decisions/0008-domain-types-posture.pt-BR.md) — postura dos crates de tipos de domínio
- [0009-unsafe-ffi-posture.md](../decisions/0009-unsafe-ffi-posture.md) / [pt-BR](../decisions/0009-unsafe-ffi-posture.pt-BR.md) — postura de código unsafe e FFI
- [0010-explicit-target-designation.md](../decisions/0010-explicit-target-designation.md) / [pt-BR](../decisions/0010-explicit-target-designation.pt-BR.md) — verbo destrutivo precisa do alvo nomeado no argv ou de waiver, senão recusa e não destrói nada
