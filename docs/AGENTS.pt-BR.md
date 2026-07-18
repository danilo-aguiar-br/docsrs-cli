[English](AGENTS.md)

# Guia de Agentes para docsrs-cli

> Gaste tokens em respostas, não em raspar HTML à mão.


## Por que Agentes Usam docsrs-cli
- JSON estável vence scraping frágil de HTML
- Um processo por pergunta mantém o estado honesto
- Exit codes tornam a política de retry mecânica


## Economia
- Cache em disco remove downloads repetidos dentro do TTL
- Dry-run valida URLs planejadas sem queimar cota
- Flags de truncagem dizem quando elevar limites


## Soberania
- Não exige daemon MCP sticky
- Nenhuma telemetria de produto sai do host
- Apenas hosts públicos de docs; sem scraping com login


## Agentes e Orquestradores Compatíveis
- Claude Code, Codex, Cursor, OpenCode e qualquer agente que execute binário
- Pipelines de shell e jobs de CI que parseiam JSON
- Pacotes de skill em `skills/docsrs-cli-en` e `skills/docsrs-cli-pt`


## Detalhes de Integração de Agente
- O ciclo de vida é sempre one-shot: BORN, EXECUTE, FINALIZE, DIE
- Stdout é o contrato de dados; stderr é só diagnóstico
- JSON é automático quando stdout não é TTY
- Force JSON com `--json` ou `--format json`
- Force humano com `--format markdown` ou `--format text`
- Prefira `-q` quando stderr não deve poluir transcripts


## Integrações de Crates e Hosts
- crates.io alimenta `search-crates`
- docs.rs alimenta `readme`, `get-item` e `search-in-crate`
- doc.rust-lang.org alimenta `std`, `core` e `alloc`
- A allowlist de hosts é fixa na camada HTTP do produto


## Contrato: Descoberta
- Rode `docsrs-cli commands --json` antes de inventar argv
- Rode `docsrs-cli schema --cmd <name> --json` antes de parsear campos novos
- Rode `docsrs-cli doctor --json` quando paths ou TLS parecerem errados


## Contrato: Envelope de Sucesso
- JSON de sucesso inclui `schema_version`, `ok:true`, `command`, `data`, `duration_ms`
- Leia `data` só depois de `ok` ser true
- Campos de proveniência como `source_url` identificam a página buscada


## Contrato: Envelope de Erro
- JSON de falha inclui `ok:false` e `error` com `kind` e message
- Falhas no caminho humano deixam stdout vazio e escrevem uma linha no stderr
- Ramifique pelo exit code do processo antes de confiar em qualquer campo


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
- Desabilite retries só com `--disable-retry` em incidentes


## Contrato: Catálogo Completo de Comandos
- Superfície tem 11 top-level commands e ações aninhadas de `cache` / `config`
```bash
# search-crates
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates tokio --sort alphabetical --json

# readme
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json

# get-item
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json

# search-in-crate
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --limit 20 --json
docsrs-cli search-in-crate tokio "" --limit 50 --json

# meta
docsrs-cli version --json
docsrs-cli doctor --json
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

# completions
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell

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
- Kinds aceitos incluem module, struct, trait, enum, union, fn, function, type, const, constant, static, macro, attr, attribute, derive
- `std`, `core` e `alloc` resolvem via doc.rust-lang.org


## Contrato: Regras de search-crates
- `--page` é 1-based
- `--per-page` máximo é 100
- `--sort` aceita relevance, downloads, recent-downloads, recent-updates, new, alphabetical


## Contrato: Regras de schema
- Schemas de payload existem para search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config
- `schema` e `completions` não expõem payload schemas via `--cmd`
