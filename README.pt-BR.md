[English](README.md)

# docsrs-cli
> Busque docs do crates.io e docs.rs em um tiro para agentes.

[![docs.rs](https://img.shields.io/docsrs/docsrs-cli)](https://docs.rs/docsrs-cli)
[![crates.io](https://img.shields.io/crates/v/docsrs-cli)](https://crates.io/crates/docsrs-cli)
[![License](https://img.shields.io/crates/l/docsrs-cli)](LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-orange)](Cargo.toml)
[![Downloads](https://img.shields.io/crates/d/docsrs-cli)](https://crates.io/crates/docsrs-cli)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-blue)](https://www.rust-lang.org/)


## O que é
- CLI one-shot stdin/stdout para busca no crates.io e páginas no docs.rs
- Ciclo de vida sempre BORN, EXECUTE, FINALIZE, DIE
- Sem daemon, sem sessão sticky, sem telemetria de produto
- JSON escolhido automaticamente quando stdout não é TTY
- HTTP público apenas contra allowlist de hosts de documentação
- A linha de produto atual é 1.1.x


## A Dor
- Agentes raspam HTML à mão e gastam tokens em ruído
- Abas de browser e pipelines com curl não formam contrato estável
- Servidores MCP sticky mantêm sockets abertos entre turnos


## Por quê
- Envelopes JSON estáveis no stdout em todo comando
- Caminho Markdown humano quando você força `--format markdown`
- Cache em disco XDG com TTL, orçamento soft e `data.cache_hit`
- Rate limits educados, retry HTTP com full-jitter e shutdown cancel-safe
- Exit codes que o agente ramifica sem parsear prosa
- Match modes ranqueados reduzem falsos positivos ruidosos


## Superpoderes
- `search-crates` no crates.io com paginação, ordenação e `--page-token`
- `readme` com overview no docs.rs e `resolved_version`
- `get-item` para páginas rustdoc tipadas incluindo métodos associados
- `search-in-crate` sobre `all.html` com `--match exact|prefix|substring`
- Fetch de stdlib para `std`, `core` e `alloc` via doc.rust-lang.org
- `commands`, `schema`, `doctor`, `version` para descoberta de agentes
- `doctor --online` para probes DNS opt-in em crates.io e docs.rs
- `cache` e `config` para manutenção XDG sem segredos


## Início Rápido
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme tokio --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio fn runtime::Runtime::new --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli doctor --online --json
```


## Instalação
- Do crates.io: `cargo install docsrs-cli --locked`
- Do checkout local: `cargo install --path . --locked`
- MSRV é Rust 1.88
- Este pacote não tem feature flags de Cargo


## Uso
- Passe `--json` ou pipe stdout para o envelope de agente
- Force saída humana com `--format markdown` ou `--format text`
- Planeje URLs sem rede com `--dry-run`
- Aumente o orçamento wall-clock com `--timeout <seconds>`
- Desabilite o cache em disco com `--no-cache`
- Knobs de produto usam flags CLI ou XDG `config.toml`, nunca env de produto


## Comandos
- Superfície completa tem 11 top-level commands
- `search-crates [query] [--page N] [--per-page N] [--sort KIND] [--page-token TOKEN]`
- A query pode ser omitida quando `--page-token` carrega a query completa
- `--page` conflita com `--page-token`
- `--sort` aceita `relevance`, `downloads`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- `readme <crate> [--crate-version V]` — docblock de overview no docs.rs
- `get-item <crate> <item_type> <item_path> [--crate-version V] [--suggest]`
- `item_type` aceita `module`, `struct`, `trait`, `enum`, `union`, `fn`/`function`/`method`, `type`, `const`/`constant`, `static`, `macro`, `attr`/`attribute`, `derive`
- `method` é alias de `fn` para métodos associados
- Métodos associados resolvem para a página do tipo pai com `#method.name` e `item_name`
- `item_path` aceita separadores `::` ou `/` e prefixo opcional do crate
- `--suggest` em get-item 404 lista símbolos próximos em `all.html`
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N] [--match MODE]`
- `--match` aceita `exact`, `prefix` (padrão), `substring`
- `query` vazio lista itens classificados até `--limit` (clamp em 1000)
- Hits podem incluir `score` quando há query
- `version` — versão do binário
- `doctor` — prontidão local de TLS, paths, concorrência e retry
- `doctor --online` — também sonda DNS de crates.io e docs.rs
- `commands` — árvore completa de comandos para agentes
- `schema --cmd <name>` — JSON Schema do payload de um comando
- Alvos de schema: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `cache`, `config`
- `completions <shell>` — script cru por padrão; JSON só com `--json` explícito
- Shells: `bash`, `zsh`, `fish`, `elvish`, `power-shell` (alias `powershell`)
- `cache stats` — reporta contagem, bytes e orçamento
- `cache clear` — apaga bodies HTTP em cache
- `config path` — imprime dirs resolvidos e camada vencedora
- `config show` — imprime configuração efetiva de runtime
- `config init [--force]` — cria `config.toml` padrão


## Destaques do contrato JSON
- Envelope de sucesso: `schema_version`, `ok`, `command`, `data`, `duration_ms`
- Payloads de rede usam `crate_name` canônico (nunca `crate`)
- Payloads de rede expõem `cache_hit` só para cache local em disco
- `get-item` expõe `item_name` e `resolved_version` opcional
- `readme` expõe `resolved_version` opcional (canal da stdlib é `stable`)
- `search-in-crate` ecoa `match_mode` e `item_type` opcional
- Tokens de paginação de `search-crates` ficam em `data.meta.next_page` / `prev_page`


## Variáveis de Ambiente
- `DOCSRS_CLI_HOME` — raiz de sandbox para config e cache (testes / isolamento)
- `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` — overrides apenas de path
- Knobs de produto (timeout, UA, TTL de cache, retries, concurrency, lang) não são lidos de `DOCSRS_CLI_*` em runtime
- Use flags CLI e XDG `config.toml` para settings de produto
- `RUST_LOG` — filtro de tracing (só stderr; sem telemetria de produto)
- `NO_COLOR` / `CLICOLOR_FORCE` — apenas diagnósticos


## Padrões de Integração
- Subprocesso de agente: `docsrs-cli get-item serde trait Serialize --json`
- Fetch de método: `docsrs-cli get-item tokio fn runtime::Runtime::new --json`
- Símbolos ranqueados: `docsrs-cli search-in-crate serde Serialize --match prefix --json`
- Paginar: leia `meta.next_page` e rode `docsrs-cli search-crates --page-token '...' --json`
- Descoberta primeiro: `docsrs-cli commands --json` depois `schema --cmd get-item --json`
- Plano offline: `docsrs-cli --dry-run readme tokio --json`
- Veja [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md) e [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md)


## Performance
- Um GET primário por comando no caminho feliz
- Runtime Tokio multi-thread com orçamento Semaphore
- `spawn_blocking` para parse HTML pesado
- Cache em disco evita downloads repetidos dentro do TTL
- Respostas de cache quente definem `data.cache_hit` como true


## Requisitos de Memória
- Teto padrão de body é 10 MiB por resposta
- Teto padrão de output é 2 MiB por emissão
- Orçamento soft padrão de cache em disco é 256 MiB
- Eleve tetos só com flags CLI ou XDG `config.toml`, nunca acima do hard ceiling


## FAQ de Troubleshooting
- Exit `66` significa crate ou item não encontrado
- Use `get-item ... --suggest` para listar símbolos próximos após 404
- Exit `69` significa rate limit ou outage temporário
- Exit `74` significa falha de transporte; retente com backoff
- Exit `78` significa falha de config ou prontidão de path local
- Exit `124` significa timeout wall-clock
- Rode `docsrs-cli doctor --json` antes de culpar a rede
- Rode `docsrs-cli doctor --online --json` para sondar DNS de crates.io e docs.rs
- Hits ruidosos: troque `--match substring` por `prefix` ou `exact`


## Mapa de Documentação
- [Como usar](docs/HOW_TO_USE.pt-BR.md)
- [Agentes](docs/AGENTS.pt-BR.md)
- [Cookbook](docs/COOKBOOK.pt-BR.md)
- [Multiplataforma](docs/CROSS_PLATFORM.pt-BR.md)
- [Migração](docs/MIGRATION.pt-BR.md)
- [Testes](docs/TESTING.pt-BR.md)
- [JSON schemas](docs/schemas/README.md)
- [Integrações](INTEGRATIONS.pt-BR.md)
- [llms.pt-BR.txt](llms.pt-BR.txt)


## Contribuindo
- Leia [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md)
- Siga o [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md)


## Segurança
- Leia [SECURITY.pt-BR.md](SECURITY.pt-BR.md)
- Reporte problemas em privado para daniloaguiarbr@proton.me


## Changelog
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para o histórico de versões
- Notas da release 1.1.0 ficam sob `[1.1.0]`


## Licença
- Dual-licensed sob MIT ou Apache-2.0
- Veja [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)
