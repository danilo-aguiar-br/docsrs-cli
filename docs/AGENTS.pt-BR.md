[English](AGENTS.md)

# Guia de Agentes para docsrs-cli
> Gaste tokens em respostas, não em raspar HTML à mão.

## Por que Agentes Usam docsrs-cli
- JSON estável vence scraping frágil de HTML
- Um processo por pergunta mantém o estado honesto
- Exit codes tornam a política de retry mecânica
- A linha de produto é `1.1.x` (`version` reporta `1.1.0`)

## Economia
- Cache em disco remove downloads repetidos dentro do TTL
- `cache_hit` diz quando o body ficou local
- Dry-run valida URLs planejadas sem queimar cota
- Flags de truncagem dizem quando elevar limites
- Modos ranqueados de `--match` cortam hits barulhentos de substring

## Soberania
- Não exige daemon MCP sticky
- Nenhuma telemetria de produto sai do host
- Apenas hosts públicos de docs; sem scraping com login
- Knobs de produto vêm só de flags e `config.toml` XDG

## Agentes e Orquestradores Compatíveis
- Claude Code, Codex, Cursor, OpenCode e qualquer agente que execute binário
- Pipelines de shell e jobs de CI que parseiam JSON
- Pacotes de skill em `skills/docsrs-cli-en` e `skills/docsrs-cli-pt`

## Detalhes de Integração de Agente
- Lifecycle é sempre one-shot: BORN, EXECUTE, FINALIZE, DIE
- Stdout é o contrato de dados; stderr é só diagnóstico
- Nomes de campos JSON e mensagens técnicas de erro são sempre em inglês
- Stderr humano pode localizar via `--lang` ou `DOCSRS_CLI_LANG` (pt-BR / en)
- JSON é automático quando stdout não é TTY na maioria dos comandos
- Force JSON com `--json` ou `--format json`
- Force humano com `--format markdown` ou `--format text`
- Prefira `-q` quando stderr não deve poluir transcripts
- `completions` são a exceção: shell bruto por default; JSON só com `--json` explícito

## Integrações de Crates e Hosts
- crates.io alimenta `search-crates`
- docs.rs alimenta `readme`, `get-item` e `search-in-crate`
- doc.rust-lang.org alimenta `std`, `core` e `alloc`
- A allowlist de hosts é fixa na camada HTTP do produto

## Contrato: Descoberta
- Rode `docsrs-cli commands --json` antes de inventar argv
- Rode `docsrs-cli schema --cmd <name> --json` antes de parsear campos novos
- Rode `docsrs-cli doctor --json` quando paths ou TLS parecerem errados
- Rode `docsrs-cli doctor --online --json` quando precisar de sondas live de host
- Confirme que `docsrs-cli version --json` reporta `1.1.0` (ou 1.1.x mais novo)

## Contrato: Envelope de Sucesso
- JSON de sucesso inclui `schema_version`, `ok:true`, `command`, `data`, `duration_ms`
- Leia `data` só depois de `ok` ser true
- Campos de proveniência como `source_url` identificam a página buscada
- Sucesso de dry-run pode incluir `dry_run:true` e campos de URL planejada

## Contrato: Campos JSON de data
- data de `search-crates`: `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`
- meta de `search-crates` pode incluir `next_page` / `prev_page` para `--page-token`
- data de `readme`: `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; `resolved_version` opcional
- data de `get-item`: `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; `resolved_version` opcional
- data de `search-in-crate`: `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; `item_type` opcional
- hits de `search-in-crate`: `name`, `kind`, `url`; `score` opcional
- `cache_hit` é só cache local em disco; nunca telemetria remota
- Campos opcionais são omitidos quando ausentes (nunca JSON null)

## Contrato: Envelope de Erro
- JSON de falha é envelope de topo: `schema_version`, `ok:false`, `error`
- `error` sempre tem `code`, `kind`, `message` e `retryable`
- `error.retry_after_secs` opcional é omitido quando ausente (nunca JSON null)
- A mensagem é inglês técnico; nunca segredos nem bodies crus de resposta
- Falhas no caminho humano deixam stdout vazio e escrevem uma linha no stderr
- Ramifique pelo exit code do processo antes de confiar em qualquer campo
- Retry só quando o exit é `69`, `74` ou `124` e/ou `error.retryable` é true
- `get-item --suggest` pode enriquecer paths not-found com símbolos próximos (só na mensagem de erro)
- Schema máquina: [error.schema.json](schemas/error.schema.json)

## Contrato: Exit Codes
- `0` sucesso
- `2` falha de parse do clap
- `64` usage
- `65` input inválido ou parse
- `66` not found
- `69` rate limited ou unavailable
- `70` internal
- `74` network
- `78` config
- `124` timeout
- `130` SIGINT
- `141` broken pipe no stdout
- `143` SIGTERM ou SIGHUP

## Contrato: Retry
- Retente apenas `69`, `74` e `124` com backoff
- Honre `Retry-After` quando o upstream enviar
- Não retente `64`, `65`, `66` ou `78` sem mudar inputs
- Desabilite retries com `--disable-retry`, TOML `disable_retry` ou `max_retries=0`
- Não há kill switch de retry por env de produto

## Contrato: Catálogo Completo de Comandos
- Superfície tem 11 top-level commands e ações aninhadas de `cache` / `config`
```bash
# search-crates
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates tokio --sort alphabetical --json
docsrs-cli search-crates --page-token "$NEXT" --json

# readme
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json

# get-item
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item serde struct Serde --suggest --json

# search-in-crate
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate serde de --match substring --limit 20 --json
docsrs-cli search-in-crate clap Parser --item-type function --limit 20 --json
docsrs-cli search-in-crate tokio "" --limit 50 --json

# meta
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json

# completions (shell bruto por default)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# cache / config
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json

# dry-run
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme tokio --json
docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
```

## Contrato: Regras de get-item
- `item_path` aceita separadores `::` ou `/`
- Prefixo opcional do crate é permitido
- Kinds aceitos incluem module, struct, trait, enum, union, fn, function, method, type, const, constant, static, macro, attr, attribute, derive
- Alias `method` mapeia como `fn` / `function`
- Métodos associados como `Runtime::new` resolvem para a página do tipo pai mais `#method.name`
- Payload sempre inclui `item_name`
- `resolved_version` opcional é o SemVer concreto somente do crate alvo, ou o canal da stdlib (`stable`) quando conhecido
- Nunca trate versões de dependências na página do docs.rs como a versão do crate
- `--suggest` em 404 faz request extra de `all.html` e lista símbolos próximos
- `std`, `core` e `alloc` resolvem via doc.rust-lang.org
- Exemplo de canal stdlib: `docsrs-cli readme std --json` → `resolved_version` é `stable` quando conhecido

## Contrato: Regras de search-in-crate
- Default de `--match` é `prefix` (folha exata ou prefixo da folha)
- Use `--match substring` para o comportamento legado de contains
- Modos: `exact`, `prefix`, `substring`
- Hits podem incluir `score` (menor é melhor quando presente)
- Default de `--limit` é 100; hard clamp é 1000 (incluindo dry-run)
- Prove o clamp offline: `docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json` planeja limit 1000
- `--item-type` opcional filtra kinds (`struct`, `fn`, `method`, …) e é ecoado quando definido
- Payload sempre inclui `match_mode` e `cache_hit`
- Query vazia lista itens classificados até `--limit`

## Contrato: Regras de search-crates
- `--page` é 1-based e conflita com `--page-token`
- `--per-page` máximo é 100
- `--sort` aceita relevance, downloads, recent-downloads, recent-updates, new, alphabetical
- Tokens de paginação vêm de `meta.next_page` / `meta.prev_page`
- Devolva tokens com `--page-token` sem inventar query strings à mão
- Payload sempre inclui `cache_hit`

## Contrato: Regras de doctor
- `doctor` default fica offline e checa TLS, paths, concorrência, contact e política de retry
- `doctor --online` adiciona sondas de rede opt-in para crates.io e docs.rs
- Use o modo online antes de lotes grandes de agente quando conectividade importa

## Contrato: Regras de config e path
- Settings de produto: flags CLI > `config.toml` XDG > defaults
- Knobs de produto não são lidos de vars de env `DOCSRS_CLI_*`
- Path sandbox ainda permite `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- User-Agent default é `docsrs-cli/1.1.0 (+https://github.com/danilo-aguiar-br/docsrs-cli)`
- User-Agent: `--user-agent` ou TOML `user_agent`; contact: TOML `contact`
- Dry-run `planned_params` usam `crate_name` (não `crate`)
- Forma do envelope dry-run está em [dry-run.schema.json](schemas/dry-run.schema.json)

## Contrato: Regras de schema
- Schemas de payload existem para search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions
- Contratos compartilhados também cobrem `error` e `dry-run` via `schema --cmd error|dry-run`
- Índice de todos os arquivos: [schemas/README.md](schemas/README.md)
- Prefira schemas vivos de `docsrs-cli schema --cmd <name> --json` antes de hardcodar listas de campos
