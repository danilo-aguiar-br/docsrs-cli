[English](MIGRATION.md)

# Migração


## O Que Muda
- A linha pública de lançamento é `0.1.x`
- Dual license é MIT OR Apache-2.0
- O framework de documentação é bilíngue com skills em `skills/`
- A superfície de comandos para agentes permanece JSON one-shot no stdout


## Migração Passo a Passo
- Instale ou atualize com `cargo install docsrs-cli --locked --force`
- Rode `docsrs-cli version --json` e confirme `0.1.0` ou mais novo na linha 0.1
- Rode `docsrs-cli doctor --json` contra seus dirs de config e cache
- Releia `commands --json` se seu agente cacheou uma árvore antiga
- Aponte skills e links de docs para o novo layout do repositório


## Mudanças de JSON Schema
- Envelopes de sucesso mantêm `schema_version: 1`
- Schemas de payload de comando ficam em `docs/schemas/*.schema.json`
- Antes: agentes costumavam raspar HTML sem índice de schema
- Depois: agentes carregam `docs/schemas/README.md` e schemas por comando
- `source_url` permanece o campo de proveniência em payloads de documento


## Notas de Compatibilidade
- Nenhuma migração de daemon é necessária porque não existe daemon
- Chaves da allowlist de ambiente mantêm o prefixo `DOCSRS_CLI_`
- Hard ceilings de body e output permanecem política de produto
- Allowlist de hosts permanece crates.io, docs.rs, static.docs.rs e doc.rust-lang.org


## Rollback
- Instale um binário anterior só se você ainda tiver esse artefato
- Limpe experimentos locais incompatíveis com `docsrs-cli cache clear --json`
- Mantenha sandboxes `DOCSRS_CLI_HOME` para o rollback não tocar dados XDG de produção


## Veja Também
- [CHANGELOG.pt-BR.md](../CHANGELOG.pt-BR.md)
- [AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- [schemas/README.md](schemas/README.md)
