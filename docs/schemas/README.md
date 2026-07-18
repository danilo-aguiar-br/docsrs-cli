[Português (pt-BR)](#português-brasileiro)

# JSON Schemas

Machine-readable payload contracts for docsrs-cli command success `data` objects.

## English
- Schemas describe the object under envelope field `data`
- Outer envelope always carries `schema_version`, `ok`, `command`, and `duration_ms`
- File names are kebab-case mirrors of command names
- Load a live schema from the binary with `docsrs-cli schema --cmd <name> --json`

### Inventory
- [search-crates.schema.json](search-crates.schema.json) — crates.io search hit list
- [readme.schema.json](readme.schema.json) — docs.rs crate overview payload
- [get-item.schema.json](get-item.schema.json) — typed rustdoc item payload
- [search-in-crate.schema.json](search-in-crate.schema.json) — in-crate symbol search payload
- [commands.schema.json](commands.schema.json) — command tree discovery payload
- [doctor.schema.json](doctor.schema.json) — readiness checks payload
- [version.schema.json](version.schema.json) — binary identity payload
- [cache.schema.json](cache.schema.json) — cache stats or clear payload
- [config.schema.json](config.schema.json) — config path, show, or init payload


## Português Brasileiro
- Schemas descrevem o objeto sob o campo `data` do envelope
- O envelope externo sempre carrega `schema_version`, `ok`, `command` e `duration_ms`
- Nomes de arquivo são kebab-case espelhando nomes de comando
- Carregue um schema vivo do binário com `docsrs-cli schema --cmd <name> --json`

### Inventário
- [search-crates.schema.json](search-crates.schema.json) — lista de hits da busca no crates.io
- [readme.schema.json](readme.schema.json) — payload de overview do crate no docs.rs
- [get-item.schema.json](get-item.schema.json) — payload de item rustdoc tipado
- [search-in-crate.schema.json](search-in-crate.schema.json) — payload de busca de símbolos no crate
- [commands.schema.json](commands.schema.json) — payload de descoberta da árvore de comandos
- [doctor.schema.json](doctor.schema.json) — payload de checks de prontidão
- [version.schema.json](version.schema.json) — payload de identidade do binário
- [cache.schema.json](cache.schema.json) — payload de stats ou clear do cache
- [config.schema.json](config.schema.json) — payload de path, show ou init da config
