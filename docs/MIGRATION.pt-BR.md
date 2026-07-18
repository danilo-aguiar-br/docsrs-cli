[English](MIGRATION.md)

# Migração
## O Que Muda
- A linha pública de produto é `0.1.x` (este release é `0.1.2`)
- Dual license permanece MIT OR Apache-2.0
- O framework de documentação segue bilíngue com skills em `skills/`
- A superfície de comandos para agentes permanece JSON one-shot no stdout
- Knobs de produto não vêm mais de variáveis de ambiente `DOCSRS_CLI_*`
- Variáveis de path sandbox ainda funcionam para home, config e cache

## Migrando de 1.1.x → 0.1.2
- Instale ou atualize com `cargo install docsrs-cli --locked --force`
- Rode `docsrs-cli version --json` e confirme que `data.version` é `0.1.2` (ou qualquer `0.1.x`)
- Releia `docsrs-cli schema --cmd get-item|error --json` para `extraction` opcional e `error.kind=budget`
- Loops de paginação de agentes devem confiar em `data.query` / `data.page` / `data.per_page` após `--page-token` (eco da URL efetiva)
- Trate estouro de body cap como permanente na mesma config (`kind=budget`, `retryable=false`, exit 74)
- `doctor` top-level `ok` agora espelha `data.ok` (exit 78 quando os checks falham — não trate sucesso de envelope como saudável)
- Prefira `data.source_url` quando presente; top-level `source_url` permanece espelho
- `--timeout 0` / `--connect-timeout 0` explícitos falham fechado com exit 65 (`invalid_input`), sem hang silencioso
- Segmentos de `item_path` com hífen normalizam para underscore (`async-trait` → `async_trait`)
- `--suggest` ranqueia exact → prefix → substring → edit-distance em um fetch de `all.html`
- Chrome do rustdoc (`§`, "Copy item path") é removido do markdown
- Smoke humano opcional: `scripts/smoke-live.sh` (não é CI)

## Migrando de 0.1.x → 0.1.2 (via contratos 1.1)
- Instale ou atualize com `cargo install docsrs-cli --locked --force`
- Rode `docsrs-cli version --json` e confirme que `data.version` é `0.1.2` (ou qualquer `0.1.x`)
- Releia `docsrs-cli commands --json` se seu agente cacheou uma árvore de argv antiga
- Releia `docsrs-cli schema --cmd <name> --json` antes de parsear campos required novos
- Aponte skills e links de docs para o layout e os contratos 0.1.2
- Aplique todas as quebras 1.1.x abaixo e depois as notas 0.1.2 acima

### Quebra: match default de search-in-crate
- O default de `--match` agora é `prefix` (folha exata ou prefixo da folha)
- Em 0.1.x o comportamento era contains de substring no path do símbolo
- Para o comportamento legado de contains use `--match substring`
- Modos opcionais: `exact`, `prefix`, `substring`
- Hits ranqueados podem incluir `hits[].score` (menor é melhor quando presente)
- Antes (substring implícita 0.1.x, shape parcial de data):
```json
{
  "crate_name": "serde",
  "query": "Serialize",
  "hits": [{ "name": "ser::Serialize", "kind": "trait", "url": "https://docs.rs/..." }]
}
```
- Depois (1.1.x+ / 0.1.x default `prefix` + campos required):
```json
{
  "crate_name": "serde",
  "query": "Serialize",
  "match_mode": "prefix",
  "cache_hit": false,
  "hits": [{ "name": "ser::Serialize", "kind": "trait", "url": "https://docs.rs/...", "score": 0 }]
}
```
- Para restaurar contains 0.1: `docsrs-cli search-in-crate serde Serialize --match substring --json`

### Quebra: knobs de produto não vêm de env
- Timeouts, retries, TTL de cache, caps de body/output, rate limit, concorrência, User-Agent, contact e origins não são lidos de `DOCSRS_CLI_*`
- Defina knobs de produto só com flags CLI e/ou `config.toml` XDG
- Precedência de settings de produto: flags CLI > `config.toml` XDG > defaults embutidos
- Env de path ainda permitida:
  - `DOCSRS_CLI_HOME` (raiz sandbox de config + cache)
  - `DOCSRS_CLI_CONFIG_DIR`
  - `DOCSRS_CLI_CACHE_DIR`
- Override de locale `DOCSRS_CLI_LANG` permanece só para stderr humano

### Quebra: planned_params do dry-run
- Dry-run `planned_params` usam `crate_name`, não `crate`
- Exemplo de chaves no dry-run de get-item: `crate_name`, `item_type`, `item_path`, `version`
- Antes (0.1.x / rascunhos usavam `crate`):
```json
{
  "planned_url": "https://docs.rs/tokio/latest/tokio/struct.Runtime.html",
  "planned_params": {
    "crate": "tokio",
    "item_type": "struct",
    "item_path": "runtime::Runtime",
    "version": "latest"
  }
}
```
- Depois (campo canônico de wire 1.1.x+ / 0.1.x):
```json
{
  "planned_url": "https://docs.rs/tokio/latest/tokio/struct.Runtime.html",
  "planned_params": {
    "crate_name": "tokio",
    "item_type": "struct",
    "item_path": "runtime::Runtime",
    "version": "latest"
  }
}
```

### Quebra: saída default de completions
- `completions` emitem scripts de shell brutos por default, mesmo em stdout non-TTY
- Envelope JSON só sai quando você passa `--json` ou `--format json` explicitamente
- Pipelines que assumiam auto-JSON em completions precisam adicionar `--json`

## Novas Flags e Comportamentos
- `search-in-crate --match exact|prefix|substring` (default `prefix`)
- `search-crates --page-token <token>` consome tokens opacos de `meta.next_page` / `meta.prev_page`
- `get-item --suggest` em 404 lista símbolos próximos de `all.html` (request extra)
- `doctor --online` sonda crates.io / docs.rs na rede (checks opt-in de DNS/conectividade)
- `get-item` aceita o alias de item type `method` (igual a `fn` / `function`)
- Métodos associados como `Runtime::new` resolvem para a página do tipo pai mais `#method.name`

## Novos Campos JSON em data
- Payloads de rede expõem `cache_hit` (cache local em disco apenas; sem telemetria remota)
- `get-item` exige `item_name` e pode incluir `resolved_version` opcional
- `search-in-crate` exige `match_mode` e pode ecoar `item_type` opcional
- Hits de `search-in-crate` podem incluir `score` opcional
- `readme` e `search-crates` também exigem `cache_hit`
- `readme` / `get-item` podem incluir `resolved_version` opcional (omitido quando desconhecido; nunca JSON null)
- `resolved_version` de crate é o SemVer só do crate alvo (não versões de dependências na página)
- `resolved_version` da stdlib é o nome do canal como `stable` (exemplo: `docsrs-cli readme std --json`)
- User-Agent default é `docsrs-cli/0.1.2 (+https://github.com/danilo-aguiar-br/docsrs-cli)`
- Envelopes JSON permanecem em inglês; stderr humano pode usar `--lang` / `DOCSRS_CLI_LANG`
- Envelopes de erro expõem `error.code`, `error.kind`, `error.message`, `error.retryable`, `retry_after_secs` opcional
- A lista viva de schemas agora inclui `schema`, `completions`, `error` e `dry-run` via `schema --cmd`
- 0.1.2 adiciona o campo opcional `extraction` em get-item (`method`|`item_page`) e o kind `budget` (exit 74, não retryable)

## Migração Passo a Passo
- Atualize o binário e confirme a versão `0.1.2`
- Mova quaisquer settings antigos de env `DOCSRS_CLI_*` de produto para flags ou `config.toml`
- Mantenha isolamento de path com `DOCSRS_CLI_HOME` / `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` conforme necessário
- Substitua suposições de substring em `search-in-crate` por `--match substring` quando for o caso
- Atualize parsers de dry-run para ler `planned_params.crate_name`
- Pare de esperar auto-JSON de `completions` sem `--json`
- Ensine agentes sobre eco de `--page-token`, `--suggest`, semântica de `ok` do doctor e `kind=budget`
- Rode `docsrs-cli doctor --json` (e opcionalmente `doctor --online --json`) após mudanças de path/config
- Opcional: rode `scripts/smoke-live.sh` contra hosts live antes do rollout de agentes

## Mudanças de JSON Schema
- Envelopes de sucesso mantêm `schema_version: 1`
- Schemas de payload de comando ficam em `docs/schemas/*.schema.json`
- Schemas de objeto usam `additionalProperties: false`, exceto bags livres (`planned_params`, documento `schema` embutido)
- Campos data novos ou agora required que agentes devem aceitar:
  - `cache_hit` (boolean) em search-crates, readme, get-item, search-in-crate
  - `item_name` (string) em get-item
  - `match_mode` (string) em search-in-crate
  - `resolved_version` opcional em readme e get-item
  - `hits[].score` opcional em search-in-crate
  - eco opcional de `item_type` em search-in-crate quando filtrado
  - `extraction` opcional em get-item (`method`|`item_page`) desde 0.1.2
  - `budget` como `error.kind` não retryable (exit 74) desde 0.1.2
- `meta.next_page` / `meta.prev_page` permanecem a fonte de `--page-token`
- `source_url` permanece o campo de proveniência em payloads de documento
- Antes: agentes costumavam raspar HTML sem índice de schema
- Depois: agentes carregam `docs/schemas/README.md` e schemas por comando
- Before/after do match default e do dry-run `crate` → `crate_name` estão nas seções de Quebra acima

## Notas de Compatibilidade
- Nenhuma migração de daemon é necessária porque não existe daemon
- Chaves de path sandbox mantêm o prefixo `DOCSRS_CLI_`; knobs de produto não usam env
- Hard ceilings de body e output permanecem política de produto
- Allowlist de hosts permanece crates.io, docs.rs, static.docs.rs e doc.rust-lang.org
- Kill switch de retry é `--disable-retry`, TOML `disable_retry` ou `max_retries=0` (sem kill switch por env)

## Rollback
- Instale um binário anterior só se você ainda tiver esse artefato
- Limpe experimentos locais incompatíveis com `docsrs-cli cache clear --json`
- Mantenha sandboxes `DOCSRS_CLI_HOME` para o rollback não tocar dados XDG de produção
- Restaure scripts que dependiam do match substring 0.1 ou de knobs de produto por env

## Veja Também
- [CHANGELOG.pt-BR.md](../CHANGELOG.pt-BR.md)
- [AGENTS.pt-BR.md](AGENTS.pt-BR.md)
- [HOW_TO_USE.pt-BR.md](HOW_TO_USE.pt-BR.md)
- [schemas/README.md](schemas/README.md)
