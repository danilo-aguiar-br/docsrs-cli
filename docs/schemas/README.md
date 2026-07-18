[Português (pt-BR)](#português-brasileiro)

# JSON Schemas
Machine-readable payload contracts for docsrs-cli command success `data` objects.

## English
- Schemas describe the object under envelope field `data`
- Outer envelope always carries `schema_version`, `ok`, `command`, and `duration_ms`
- File names are kebab-case mirrors of command names
- Load a live schema from the binary with `docsrs-cli schema --cmd <name> --json`
- Release 1.1.0 network schemas require `cache_hit` and use canonical `crate_name`
- Object schemas set `additionalProperties: false` except free-form bags (`planned_params`, embedded `schema` document)

### Inventory
- [search-crates.schema.json](search-crates.schema.json) — crates.io search hit list with `cache_hit` and `meta.next_page` / `prev_page`
- [readme.schema.json](readme.schema.json) — docs.rs crate overview with `crate_name`, optional `resolved_version`, `cache_hit`
- [get-item.schema.json](get-item.schema.json) — typed rustdoc item with `item_name`, optional `resolved_version`, `cache_hit`
- [search-in-crate.schema.json](search-in-crate.schema.json) — in-crate symbol search with `match_mode`, optional `item_type`, hit `score`, `cache_hit`
- [commands.schema.json](commands.schema.json) — command tree discovery payload
- [doctor.schema.json](doctor.schema.json) — readiness checks payload including optional `online_*` checks
- [version.schema.json](version.schema.json) — binary identity payload
- [cache.schema.json](cache.schema.json) — cache stats or clear payload
- [config.schema.json](config.schema.json) — config path, show, or init payload
- [schema.schema.json](schema.schema.json) — meta schema payload for `schema --cmd`
- [completions.schema.json](completions.schema.json) — completions JSON mode payload (`shell`, `script`)
- [error.schema.json](error.schema.json) — top-level JSON error envelope (not under `data`)
- [dry-run.schema.json](dry-run.schema.json) — dry-run envelope fragment with `planned_url` / `planned_params.crate_name`

### Architecture decisions
- [0001-http-retry-policy.md](../decisions/0001-http-retry-policy.md) / [pt-BR](../decisions/0001-http-retry-policy.pt-BR.md)
- [0002-error-model.md](../decisions/0002-error-model.md) / [pt-BR](../decisions/0002-error-model.pt-BR.md)
- [0003-web-fetch-scope.md](../decisions/0003-web-fetch-scope.md) / [pt-BR](../decisions/0003-web-fetch-scope.pt-BR.md)

## Português Brasileiro
- Schemas descrevem o objeto sob o campo `data` do envelope
- O envelope externo sempre carrega `schema_version`, `ok`, `command` e `duration_ms`
- Nomes de arquivo são kebab-case espelhando nomes de comando
- Carregue um schema vivo do binário com `docsrs-cli schema --cmd <name> --json`
- Schemas de rede da release 1.1.0 exigem `cache_hit` e usam `crate_name` canônico
- Schemas de objeto usam `additionalProperties: false`, exceto bags livres (`planned_params`, documento `schema` embutido)

### Inventário
- [search-crates.schema.json](search-crates.schema.json) — lista de hits da busca no crates.io com `cache_hit` e `meta.next_page` / `prev_page`
- [readme.schema.json](readme.schema.json) — overview do crate no docs.rs com `crate_name`, `resolved_version` opcional, `cache_hit`
- [get-item.schema.json](get-item.schema.json) — item rustdoc tipado com `item_name`, `resolved_version` opcional, `cache_hit`
- [search-in-crate.schema.json](search-in-crate.schema.json) — busca de símbolos com `match_mode`, `item_type` opcional, `score`, `cache_hit`
- [commands.schema.json](commands.schema.json) — payload de descoberta da árvore de comandos
- [doctor.schema.json](doctor.schema.json) — payload de checks de prontidão incluindo checks `online_*` opcionais
- [version.schema.json](version.schema.json) — payload de identidade do binário
- [cache.schema.json](cache.schema.json) — payload de stats ou clear do cache
- [config.schema.json](config.schema.json) — payload de path, show ou init da config
- [schema.schema.json](schema.schema.json) — payload meta do `schema --cmd`
- [completions.schema.json](completions.schema.json) — payload do modo JSON de completions (`shell`, `script`)
- [error.schema.json](error.schema.json) — envelope JSON de erro no topo (não sob `data`)
- [dry-run.schema.json](dry-run.schema.json) — fragmento de envelope dry-run com `planned_url` / `planned_params.crate_name`

### Decisões de arquitetura
- [0001-http-retry-policy.md](../decisions/0001-http-retry-policy.md) / [pt-BR](../decisions/0001-http-retry-policy.pt-BR.md)
- [0002-error-model.md](../decisions/0002-error-model.md) / [pt-BR](../decisions/0002-error-model.pt-BR.md)
- [0003-web-fetch-scope.md](../decisions/0003-web-fetch-scope.md) / [pt-BR](../decisions/0003-web-fetch-scope.pt-BR.md)
