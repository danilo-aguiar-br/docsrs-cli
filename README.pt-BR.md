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
- A linha de produto atual é 1.3.x (release 1.3.0)


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
- `--page-token` ecoa `query`/`page`/`per_page`/`sort` da URL efetiva
- `readme` com overview no docs.rs e `resolved_version`
- `get-item` para páginas rustdoc tipadas incluindo métodos associados
- Fetch de método define `data.extraction` como `method` em sucesso; `#method.X` ausente é `not_found` (exit 66), nunca sucesso falso com página pai
- `--suggest` em 404 de method ranqueia leaves de método na página do tipo pai (e all.html para outros kinds)
- Orçamentos acima do hard max falham fechados com exit 65 (sem clamp silencioso)
- Envelopes de erro incluem `command` e `duration_ms` como os de sucesso
- `search-in-crate` sobre `all.html` com `--match exact|prefix|substring`
- `--suggest` ranqueia exact → prefix → substring → edit-distance em get-item 404, devolvido em `error.suggestions[{path,kind}]`
- `item_path` aceita hífens e normaliza para underscores nos paths rustc
- Scrub de markdown remove chrome rustdoc (`§`, “Copy item path”)
- Corpo acima do teto é `error.kind=budget` (exit 74, `retryable=false`)
- Fetch de stdlib para `std`, `core` e `alloc` via doc.rust-lang.org
- `commands`, `schema`, `doctor`, `version` para descoberta de agentes
- `doctor --online` para probes DNS opt-in em crates.io e docs.rs
- `doctor` top-level `ok` espelha `data.ok`
- `cache` e `config` para manutenção XDG sem segredos


## Novidades em 1.3.0
- `--sort-by` e `--max-items` completam o pipeline de redução: filter → sort-by → dedupe-by → max-items → select → count-only → truncate-content → max-output-bytes
- `agent_surface` ganhou `limited`, separando resultado pequeno de resultado cortado
- Schema `agent-surface` publicado; `schema --cmd all` carrega 20 nomes wire
- `get-item` alcança variantes de enum e campos de struct (`variant`, `structfield`), além de itens associados de trait e métodos requeridos
- `error.suggestions` publica o ranking de `--suggest` como dado, sem agente algum parseando a prosa de `error.message`
- `anchor_family` nomeia a família real de âncora do rustdoc por trás de uma extração `method`
- Chave `log_directive` controla a verbosidade do stderr; valor não parseável é rejeitado na carga (exit 78)
- `RUST_LOG` deixou de ser lido: ele vencia a CLI, o que é knob de produto morando em ambiente
- Âncoras de confiança TLS voltaram ao `webpki-roots` embutido, depois que um upgrade do `reqwest` as moveu em silêncio para o repositório do sistema
- Os gates de política são testes de integração Rust (`cargo test --test policy_gates`), então rodam igualmente em Linux, macOS e Windows
- Notas completas em [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) seção `[1.3.0]`

## Destaques anteriores (1.2.0)
- Method fail-closed: `#method.X` ausente retorna `not_found` (exit 66), não sucesso falso com página pai
- `--suggest` ranqueia leaves de método no HTML do tipo pai em typos
- Envelopes de erro carregam `command` + `duration_ms` (paridade com sucesso)
- Valores acima do hard max de body/output falham com exit 65 (sem clamp silencioso)
- URL 404 de method mantém o primeiro kind de probe (`struct`), não o último
- Dry-run documenta `validation=url_shape_only` e probes de parent kind
- Schemas offline batem com `schema --cmd all` (19 nomes wire incluindo aliases naquele release)
- Notas completas em [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) seção `[1.2.0]`

## Destaques anteriores (1.1.x)
- Paths curtos de método (`Runtime::new`) resolvem via all.html
- Açúcar `crate@version` em readme / get-item / search-in-crate
- Eco de URL efetiva em `--page-token`, `--suggest` em cascata, normalização de hífen
- `--timeout 0` / `--connect-timeout 0` fail-closed (exit 65)
- Budget local não retryable (`kind=budget`, exit 74)
- Knobs de produto: flags CLI + XDG apenas (sem env `DOCSRS_CLI_*` de produto)


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


## TLS
- **Somente rustls** (sem `native-tls` / OpenSSL em runtime); provider crypto **`ring`**
- Protocolo mínimo **TLS 1.2**; peers podem negociar TLS 1.3
- Trust store: **webpki-roots** (Mozilla); validação de certificado sempre ligada
- Hosts de produção exigem **HTTPS** (allowlist); testes offline podem usar HTTP em loopback
- Sem `danger_accept_invalid_*`, sem KeyLog de produto, sem mTLS (origens públicas de docs)
- Postura em runtime: `docsrs-cli doctor --json` → check `http_client_posture`
- Decisão: [`docs/decisions/0007-rustls-posture.pt-BR.md`](docs/decisions/0007-rustls-posture.pt-BR.md)


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
- `item_type` aceita `module`/`mod`, `struct`, `trait`, `enum`, `union`, `fn`/`function`/`method`, `type`, `const`/`constant`, `static`, `macro`, `attr`/`attribute`, `derive`, `variant`, `structfield`/`field`
- `variant` e `structfield` não têm página própria: qualifique como `Pai::folha` ou o exit 65 nomeia os kinds pais
- `method` é alias de `fn` para métodos associados
- Métodos associados resolvem para a página do tipo pai com `#method.name` e `item_name`
- Sucesso de method define `data.extraction` como `method`; âncoras ausentes são `not_found` (exit 66)
- `item_path` aceita separadores `::` ou `/` e prefixo opcional do crate
- `--suggest` em get-item 404 lista símbolos próximos (leaves de method na página pai; outros kinds em `all.html`)
- `search-in-crate <crate> [query] [--crate-version V] [--item-type K] [--limit N] [--match MODE]`
- `--match` aceita `exact`, `prefix` (padrão), `substring`
- `query` vazio lista itens classificados até `--limit` (clamp em 1000)
- Hits podem incluir `score` quando há query
- `version` — versão do binário
- `doctor` — prontidão local de TLS, paths, concorrência e retry
- `doctor --online` — também sonda DNS de crates.io e docs.rs
- `commands` — árvore completa de comandos para agentes
- `schema --cmd <name>` — JSON Schema do payload de um comando
- Alvos de schema: `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `agent-surface`, `cache`, `config` mais aliases (`cache-path`, `cache-clear`, `cache-stats`, `config-path`, `config-show`, `config-init`); use `schema --cmd all --json` para o bundle completo
- `completions <shell>` — script cru por padrão; JSON só com `--json` explícito
- Shells: `bash`, `zsh`, `fish`, `elvish`, `power-shell` (alias `powershell`)
- `cache path` — imprime a raiz de cache resolvida, a camada vencedora (`cli` / `xdg` / `unresolved`) e `no_cache`
- `cache stats` — reporta contagem, bytes e orçamento
- `cache clear` — apaga bodies HTTP em cache
- `config path` — imprime dirs resolvidos e camada vencedora
- `config show` — imprime configuração efetiva de runtime
- `config init [--force]` — cria `config.toml` padrão


## Destaques do contrato JSON
- Envelope de sucesso: `schema_version`, `ok`, `command`, `data`, `duration_ms`
- Payloads de rede usam `crate_name` canônico (nunca `crate`)
- Payloads de rede expõem `cache_hit` só para cache local em disco
- `get-item` expõe `item_name`, `resolved_version` opcional; sucesso de method inclui `extraction=method`
- Agentes DEVEM rejeitar sucesso de method quando `extraction` estiver ausente ou for o valor legado `item_page`
- `anchor_family` traz a família rustdoc real, porque `extraction` reporta `method` também para variantes e campos de struct
- `readme` expõe `resolved_version` opcional (canal da stdlib é `stable`)
- `search-in-crate` ecoa `match_mode` e `item_type` opcional
- Tokens de paginação de `search-crates` ficam em `data.meta.next_page` / `prev_page`
- Após `--page-token`, o eco de `query`/`page`/`per_page`/`sort` bate com a URL efetiva
- Envelopes de falha expõem `schema_version`, `ok:false`, `command`, `duration_ms` e `error` aninhado (`kind`, `retryable`, …)
- Nunca retente `kind=budget` (exit 74); aumente `--max-body-bytes` só dentro do hard max (acima do hard max é exit 65)


## Variáveis de Ambiente
- Paths: use `--config-dir` / `--cache-dir` (ou XDG da plataforma via `directories`)
- Locale: use `--lang` ou TOML `lang` (nunca env de produto)
- Knobs de produto (timeout, UA, TTL de cache, retries, concurrency, lang, paths) **nunca** são lidos de `DOCSRS_CLI_*` em runtime
- Use flags CLI e XDG `config.toml` para settings de produto
- `RUST_LOG` **não** é lido: a verbosidade de stderr vem de `-q` / `-v` ou da chave TOML `log_directive`
- Só capacidade do terminal: `NO_COLOR`, `TERM`, `CLICOLOR_FORCE` — descrevem o *dispositivo*, como `isatty`, e não carregam configuração de produto
- Só transporte: `HTTP(S)_PROXY` / `NO_PROXY`, honrados pelo próprio `reqwest`, nunca por knob de produto


## Redução de Payload
- A CLI corta o payload antes da serialização; não é preciso estágio `jq` / `jaq`
- Projete chaves: `docsrs-cli --select planned_url --dry-run readme serde --json` (alias `--fields`)
- Chaves ausentes são puladas, nunca emitidas como null
- Filtre: `chave=valor`, `chave!=valor`, `chave~substring`; repita `--filter` para AND
- `--filter` malformado sai com exit `65`; um typo nunca parece resultado vazio
- Ordene com `--sort-by <CHAVE>` (estável, ascendente; sem a chave vai para o fim)
- Número compara numericamente, então `9` vem antes de `10`, nunca depois
- Limite a emissão com `--max-items <N>`; ele limita a saída, não a consulta
- `search-in-crate --limit` limita a consulta: decide quanto é classificado
- Os dois existem porque são limites distintos, e o clap rejeita duas flags com o mesmo nome
- Deduplique com `--dedupe-by <CHAVE>`; conte com `--count-only`
- Encurte strings com `--truncate-content <N>` (caracteres, sem partir UTF-8)
- Limite o envelope inteiro com `--max-output-bytes <N>`: ele descarta hits inteiros, nunca bytes, então o JSON segue parseável
- Medido em `search-in-crate serde "" --limit 200`: `--max-output-bytes 2000` emite 1973 bytes e 12 dos 62 hits
- Sozinha ela não ativa o pipeline, portanto `agent_surface` fica ausente e o sinal é `data.truncated`
- Ordem: filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- Ordenar antes do dedupe decide qual duplicata sobrevive; limitar depois protege as vagas
- `--count-only` conta, portanto, o que sobreviveu ao filtro, ao dedupe e ao limite
- `agent_surface` reporta `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- `limited` separa resultado genuinamente pequeno de resultado cortado por `--max-items`
- Contador irmão que nomeia o array acompanha o array: `emitted` é reescrito, `total` nunca
- Top-N sem pipe: `docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name search-in-crate serde "" --limit 200 --json`
- Contrato completo: `docsrs-cli schema --cmd agent-surface --json`


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
- Eleve tetos só com flags CLI ou XDG `config.toml`; valores acima do hard ceiling falham fechados (exit 65), nunca clamp silencioso


## FAQ de Troubleshooting
- Exit `65` significa input inválido (incluindo `--timeout 0` / `--connect-timeout 0` explícitos, ou flags de budget acima do hard max)
- Exit `66` significa crate ou item não encontrado (incluindo âncoras de method ausentes)
- Use `get-item ... --suggest` para listar símbolos próximos após 404 (typos de method incluem leaves da página pai)
- Exit `69` significa rate limit ou outage temporário (retryable)
- Exit `74` com `error.kind=network` significa falha de transporte; retente com backoff
- Exit `74` com `error.kind=budget` significa teto local de body; não retente — aumente `--max-body-bytes`
- Sempre leia `error.retryable` antes de retentar qualquer exit non-zero
- Exit `78` significa falha de config ou prontidão de path local
- Exit `124` significa timeout wall-clock
- Rode `docsrs-cli doctor --json` antes de culpar a rede
- Trate doctor como saudável só quando top-level `ok` e `data.ok` forem ambos true
- Rode `docsrs-cli doctor --online --json` para sondar DNS de crates.io e docs.rs
- Hits ruidosos: troque `--match substring` por `prefix` ou `exact`
- Paths com hífen: passe segmentos `async-trait`; a CLI normaliza para `async_trait`


## Mapa de Documentação
- [Como usar](docs/HOW_TO_USE.pt-BR.md)
- [Agentes](docs/AGENTS.pt-BR.md)
- [Cookbook](docs/COOKBOOK.pt-BR.md)
- [Configuração](docs/CONFIGURATION.pt-BR.md) — toda flag e toda chave do `config.toml`
- [Multiplataforma](docs/CROSS_PLATFORM.pt-BR.md)
- [Migração](docs/MIGRATION.pt-BR.md)
- [Testes](docs/TESTING.pt-BR.md)
- [JSON schemas](docs/schemas/README.md)
- [Decisões de arquitetura](docs/decisions/) — nove ADRs, cada uma com par pt-BR
- [Integrações](INTEGRATIONS.pt-BR.md)
- [Política de segurança](SECURITY.pt-BR.md)
- [Contribuindo](CONTRIBUTING.pt-BR.md)
- [Changelog](CHANGELOG.pt-BR.md)
- [llms.pt-BR.txt](llms.pt-BR.txt)


## Contribuindo
- Leia [CONTRIBUTING.pt-BR.md](CONTRIBUTING.pt-BR.md)
- Siga o [Código de Conduta](CODE_OF_CONDUCT.pt-BR.md)


## Segurança
- Leia [SECURITY.pt-BR.md](SECURITY.pt-BR.md)
- Reporte problemas em privado para daniloaguiarbr@proton.me


## Changelog
- Veja [CHANGELOG.pt-BR.md](CHANGELOG.pt-BR.md) para o histórico de versões
- Notas da release 1.3.0 ficam sob `[1.3.0]`


## Licença
- Dual-licensed sob MIT ou Apache-2.0
- Veja [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT) e [LICENSE-APACHE](LICENSE-APACHE)
