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


## A Dor
- Agentes raspam HTML à mão e gastam tokens em ruído
- Abas de browser e pipelines com curl não formam contrato estável
- Servidores MCP sticky mantêm sockets abertos entre turnos


## Por quê
- Envelopes JSON estáveis no stdout em todo comando
- Caminho Markdown humano quando você força `--format markdown`
- Cache em disco XDG com TTL e orçamento soft
- Rate limits educados, retry HTTP com full-jitter e shutdown cancel-safe
- Exit codes que o agente ramifica sem parsear prosa


## Superpoderes
- `search-crates` no crates.io com paginação e ordenação
- `readme` com o docblock de visão geral no docs.rs
- `get-item` para páginas rustdoc tipadas por kind e path
- `search-in-crate` sobre o índice `all.html` do crate
- Fetch de stdlib para `std`, `core` e `alloc` via doc.rust-lang.org
- `commands`, `schema`, `doctor`, `version` para descoberta de agentes
- `cache` e `config` para manutenção XDG sem segredos


## Início Rápido
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme tokio --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli doctor --json
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


## Comandos
- Superfície completa tem 11 top-level commands
- `search-crates <query> [--page N] [--per-page N] [--sort KIND]` — busca no crates.io
- `--sort` aceita `relevance`, `downloads`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- `readme <crate> [--crate-version V]` — docblock de overview no docs.rs
- `get-item <crate> <item_type> <item_path> [--crate-version V]` — item rustdoc tipado
- `item_type` aceita `module`, `struct`, `trait`, `enum`, `union`, `fn`/`function`, `type`, `const`/`constant`, `static`, `macro`, `attr`/`attribute`, `derive`
- `item_path` aceita separadores `::` ou `/` e prefixo opcional do crate
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N]` — busca em `all.html`
- `query` vazio lista itens classificados até `--limit`
- `version` — versão do binário
- `doctor` — prontidão local de TLS, paths, concorrência e retry
- `commands` — árvore completa de comandos para agentes
- `schema --cmd <name>` — JSON Schema do payload de um comando
- Alvos de schema: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `cache`, `config`
- `completions <shell>` — `bash`, `zsh`, `fish`, `elvish`, `power-shell` (alias `powershell`)
- Exemplos: `docsrs-cli completions bash`, `completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`
- `cache stats` — reporta contagem, bytes e orçamento
- `cache clear` — apaga bodies HTTP em cache
- `config path` — imprime dirs resolvidos e camada vencedora
- `config show` — imprime configuração efetiva de runtime
- `config init [--force]` — cria `config.toml` padrão
- Exemplo de sobrescrita: `docsrs-cli config init --force --json`


## Variáveis de Ambiente
- `DOCSRS_CLI_HOME` — raiz de sandbox para config e cache
- `DOCSRS_CLI_CONFIG_DIR` / `DOCSRS_CLI_CACHE_DIR` — overrides de path
- `DOCSRS_CLI_TIMEOUT_SECS` — timeout wall-clock
- `DOCSRS_CLI_USER_AGENT` / `DOCSRS_CLI_CONTACT` — headers de identidade
- `DOCSRS_CLI_CACHE_TTL_SECS` / `DOCSRS_CLI_MAX_CACHE_BYTES` / `DOCSRS_CLI_NO_CACHE`
- `DOCSRS_CLI_MAX_BODY_BYTES` / `DOCSRS_CLI_MAX_OUTPUT_BYTES` — tetos rígidos
- `DOCSRS_CLI_MAX_CONCURRENCY` — orçamento de workers de parse (`0` = auto)
- `DOCSRS_CLI_MAX_RETRIES` / `DOCSRS_CLI_RETRY_BASE_MS` / `DOCSRS_CLI_RETRY_MAX_DELAY_MS`
- `DOCSRS_CLI_DISABLE_RETRY` — kill switch de retries HTTP
- `DOCSRS_CLI_LANG` — locale de stderr humano (`en` ou `pt-BR`)
- `RUST_LOG` / `NO_COLOR` / `CLICOLOR_FORCE` — apenas diagnósticos


## Padrões de Integração
- Subprocesso de agente: `docsrs-cli get-item serde trait Serialize --json`
- Descoberta primeiro: `docsrs-cli commands --json` depois `schema --cmd get-item --json`
- Plano offline: `docsrs-cli --dry-run readme tokio --json`
- Veja [INTEGRATIONS.pt-BR.md](INTEGRATIONS.pt-BR.md) e [docs/AGENTS.pt-BR.md](docs/AGENTS.pt-BR.md)


## Performance
- Um GET primário por comando no caminho feliz
- Runtime Tokio multi-thread com orçamento Semaphore
- `spawn_blocking` para parse HTML pesado
- Cache em disco evita downloads repetidos dentro do TTL


## Requisitos de Memória
- Teto padrão de body é 10 MiB por resposta
- Teto padrão de output é 2 MiB por emissão
- Orçamento soft padrão de cache em disco é 256 MiB
- Eleve tetos só com flags ou env explícitos, nunca acima do hard ceiling


## FAQ de Troubleshooting
- Exit `66` significa crate ou item não encontrado
- Exit `69` significa rate limit ou outage temporário
- Exit `74` significa falha de transporte; retente com backoff
- Exit `78` significa falha de config ou prontidão de path local
- Exit `124` significa timeout wall-clock
- Rode `docsrs-cli doctor --json` antes de culpar a rede


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


## Licença
- Dual-licensed sob MIT ou Apache-2.0
- Veja [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)
