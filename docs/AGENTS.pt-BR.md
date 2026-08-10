[English](AGENTS.md)

# Guia de Agentes para docsrs-cli
> Gaste tokens em respostas, não em raspar HTML à mão.

## Por que Agentes Usam docsrs-cli
- JSON estável vence scraping frágil de HTML
- Um processo por pergunta mantém o estado honesto
- Exit codes tornam a política de retry mecânica
- A linha de produto é `1.3.x` (`version` reporta `1.3.0`)

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
- Stderr humano pode localizar pela flag `--lang` ou pela chave TOML `lang` (pt-BR / en); a flag vence a chave
- JSON é automático quando stdout não é TTY na maioria dos comandos, a menos que `--format markdown|text` sobrescreva
- O `commands --json` devolve essa regra em `data.agent_notes.json_auto`, então o chamador lê a política em vez de supô-la
- O mesmo bloco carrega `stdout`, `stderr` e `lifecycle`, que é o modelo one-shot: BORN, EXECUTE, FINALIZE, DIE
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
- Confirme que `docsrs-cli version --json` reporta `1.3.0` (ou `1.3.x` mais novo)

## Contrato: Envelope de Sucesso
- JSON de sucesso inclui `schema_version`, `ok`, `command`, `data`, `duration_ms`
- Na maioria dos comandos, sucesso significa `ok:true`
- Exceção (`doctor`): top-level `ok` espelha `data.ok` (pode ser `false` com exit 78 quando os checks falham)
- Leia `data` depois de inspecionar `ok` e o exit code do processo
- Prefira `data.source_url` quando presente; `source_url` no topo do envelope é espelho nas ops de fetch
- Sucesso de dry-run pode incluir `dry_run:true` e campos de URL planejada

## Contrato: Campos JSON de data
- data de `search-crates`: `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit` — campos de eco sempre batem com a URL efetiva (incluindo `--page-token`)
- meta de `search-crates` pode incluir `next_page` / `prev_page` para `--page-token`
- data de `readme`: `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; `resolved_version` opcional
- data de `get-item`: `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; `resolved_version` opcional; sucesso de method inclui apenas `extraction=method`
- DEVE rejeitar sucesso de method quando `extraction` estiver ausente ou for o legado `item_page` (fail-closed desde 1.2.0)
- `extraction=method` significa "veio da âncora do membro", nunca "o membro é uma função"
- `anchor_family` nomeia a família real: `method`, `tymethod`, `associatedtype`, `associatedconstant`, `variant`, `structfield`
- Leia `anchor_family` para distinguir variante de enum de função; `iter::Iterator::next` devolve `extraction=method` com `anchor_family=tymethod`
- Os dois campos são omitidos para itens com página própria (nunca JSON null)
- Tipo e constante associados reportam o mesmo valor, então o check fail-closed não muda
- `method`, `type` e `const` aceitam `Pai::membro`; o rustdoc ancora o membro na página do pai
- Um prefixo de âncora por categoria: `method.` · `tymethod.` · `associatedtype.` · `associatedconstant.`
- `source_url` ecoa a âncora que existe na página, não a que o construtor de URL planejou
- Pai em minúscula (`u32::MAX`) segue item livre em página própria, nunca âncora
- Âncora de membro ausente é `not_found` (exit 66), nunca sucesso falso com página pai
- data de `search-in-crate`: `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; `item_type` opcional
- default de `--match` em `search-in-crate` é `prefix` (use `substring` para contains legado)
- hits de `search-in-crate`: `name`, `kind`, `url`; `score` opcional
- `cache_hit` é só cache local em disco; nunca telemetria remota
- Markdown de readme/get-item remove chrome rustdoc (`§`, "Copy item path")
- Campos opcionais são omitidos quando ausentes (nunca JSON null)
- Campo de wire é sempre `crate_name` (nunca `crate`)

## Contrato: Redução de Payload
- Oito flags globais cortam o payload antes da serialização; não é preciso estágio `jq` / `jaq`
- `--select <CHAVES>` projeta só estas chaves pontilhadas (CSV ou repetida); o alias é `--fields`
- Chave ausente de `data` é pulada, nunca emitida como JSON null
- Quando o payload tem array de resultados, `--select` projeta os ELEMENTOS do array, não `data`
- Medido: `--select name` em `search-in-crate` devolve `hits:[{"name":…}]` e mantém as outras chaves de `data`
- Medido: `--select hits` devolve `hits:[{},{},{}]` porque nenhum elemento tem chave chamada `hits`
- `--filter <EXPR>` mantém elementos que casam: `chave=valor`, `chave!=valor`, `chave~substring`
- `==` é sinônimo de `=`; repita `--filter` para conjugar com AND
- `--filter` malformado falha fechado com exit `65` (`kind=invalid_input`), nunca com conjunto vazio
- `--sort-by <CHAVE>` ordena os elementos de forma ascendente pela chave; sem a chave, o elemento vai para o fim
- `--dedupe-by <CHAVE>` descarta elementos posteriores que repetem a chave; sem a chave, o elemento fica
- `--max-items <N>` emite no máximo N elementos; ele limita a EMISSÃO, nunca a consulta
- `search-in-crate --limit` é o limite da consulta; são orçamentos diferentes
- `--count-only` substitui o payload por `{"count": N}`, contado depois de filter, dedupe-by e max-items
- Medido em `search-in-crate tokio "" --limit 200`: `--filter kind=struct --count-only` devolve 164,
  e acrescentar `--max-items 5` devolve 5 — o limite entra na contagem, não vem depois dela
- `--truncate-content <N>` encurta toda string acima de N caracteres (nunca bytes; UTF-8 nunca é partido)
- `--max-output-bytes <N>` continua limitando o payload emitido (hard max 2097152)
- `--max-output-bytes` sozinho não ativa o pipeline; o orçamento por comando já o impõe antes
- A ordem de aplicação é filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- Ordenar antes do dedupe decide QUAL duplicata sobrevive; limitar depois nunca gasta vaga com uma delas
- Leia `agent_surface` para `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- `limited` separa "o conjunto era pequeno" de "o conjunto foi cortado": só então elevar `--max-items` traz mais
- A truncagem nunca é silenciosa: `content_truncated` / `output_truncated` dizem quando um teto bateu

## Contrato: Envelope de Erro
- JSON de falha é envelope de topo: `schema_version`, `ok:false`, `command`, `duration_ms`, `error`
- `error` sempre tem `code`, `kind`, `message` e `retryable`
- `error.retry_after_secs` opcional é omitido quando ausente (nunca JSON null)
- A mensagem é inglês técnico; nunca segredos nem bodies crus de resposta
- Falhas no caminho humano deixam stdout vazio e escrevem uma linha no stderr
- Ramifique pelo exit code do processo antes de confiar em qualquer campo
- Retry só quando `error.retryable` é true (tipicamente rate_limited/unavailable/timeout/network)
- Não retente `kind=budget` (body acima de `--max-body-bytes`; aumente o teto só dentro do hard max)
- Flags de budget acima do hard max falham fechado com exit `65` (sem clamp silencioso)
- Exit `74` é compartilhado por `network` (retryable), `budget` (nunca) e `io` (depende) — leia sempre `error.kind` / `error.retryable`
- `kind=io` é falha local de sistema de arquivos causada pelo ambiente, nunca defeito deste binário
- A retryabilidade de `io` vem da causa do sistema operacional: disco cheio retenta, permissão negada não
- Nunca deduza `retryable` do kind em `io`; leia o campo
- `--timeout 0` / `--connect-timeout 0` explícitos falham fechado com exit `65`
- `max_output_bytes` trunca payloads de sucesso (`truncated:true`); body acima do teto é erro duro (`budget`)
- `get-item --suggest` pode enriquecer paths not-found com símbolos próximos (estruturado em `error.suggestions`, também escrito na mensagem; cascata exact→prefix→substring→edit-distance)
- Schema máquina: [error.schema.json](schemas/error.schema.json)

## Contrato: Exit Codes
- `0` sucesso
- `64` usage, que é também toda falha de parse do clap; o binário nunca sai `2`
- `65` input inválido ou parse (inclui timeout 0 explícito)
- `66` not found
- `69` rate limited ou unavailable
- `70` internal
- `74` network, budget ou io (desambigue com `error.kind`)
- `78` config (doctor não saudável também sai 78; top-level `ok` espelha `data.ok`)
- `124` timeout
- `130` SIGINT
- `141` broken pipe no stdout
- `143` SIGTERM ou SIGHUP

## Contrato: Retry
- Retente só quando `error.retryable` é true — tipicamente exit `69`, `74` retryable (`kind=network`) e `124`
- Honre `Retry-After` quando o upstream enviar
- Não retente `64`, `65`, `66`, `78` ou `kind=budget` sem mudar inputs/config
- Nunca trate todo exit `74` como retryable
- Desabilite retries com `--disable-retry`, TOML `disable_retry` ou `max_retries=0`
- Não há kill switch de retry por env de produto
- Modele o backoff com `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms`, `--retry-max-elapsed-ms`
- `--retry-max-elapsed-ms 0` deriva o teto do `--timeout` em vez de valer por si

## Contrato: Flags de Ajuste
- Todo knob abaixo é flag de CLI com chave equivalente no `config.toml`; a flag vence o arquivo
- Ritme as requisições com `--rate-limit-delay-ms` e dimensione o pool com `--max-concurrency`
- `--max-concurrency 0` significa automático: o pool é derivado das CPUs e da memória livre
- Dirija o cache em disco com `--cache-ttl-secs` e `--max-cache-bytes`, ou contorne com `--no-cache`
- `--max-cache-bytes 0` significa ilimitado, então remove o teto em vez de fechar o cache
- `--allow-loopback` permite origem local de teste; não é bypass de TLS e nunca relaxa a verificação
- Releia o valor efetivo de qualquer um deles com `config show --json`, que ecoa o knob resolvido

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
docsrs-cli search-in-crate clap Parser --item-type trait --limit 20 --json
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
docsrs-cli schema --cmd agent-surface --json
docsrs-cli schema --cmd cache-path --json
docsrs-cli schema --cmd cache-stats --json
docsrs-cli schema --cmd cache-clear --json
docsrs-cli schema --cmd config-path --json
docsrs-cli schema --cmd config-show --json
docsrs-cli schema --cmd config-init --json
docsrs-cli schema --cmd all --json   # o pacote inteiro: 20 schemas numa chamada

# completions (shell bruto por default)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# cache / config
docsrs-cli cache path --json
docsrs-cli cache stats --json
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --yes --json

# dry-run
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme tokio --json
docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
```

## Contrato: Regras de get-item
- `item_path` aceita separadores `::` ou `/`
- `item_path` aceita `-` e normaliza segmentos no estilo rustc (`async-trait` → `async_trait` na URL)
- Prefixo opcional do crate é permitido
- Kinds aceitos são module, mod, struct, trait, enum, union, fn, function, method, type, const, constant, static, macro, attr, attribute, derive, variant, structfield, field
- `variant`, `structfield` e `field` exigem caminho qualificado, porque nomeiam membro de um pai: `Option::Some`, `Range::end`
- `field` é alias que o eco normaliza para `structfield`, a grafia que o rustdoc usa na âncora
- Alias `method` mapeia como `fn` / `function`
- Métodos associados como `Runtime::new` resolvem para a página do tipo pai mais `#method.name`
- Sucesso de method define `extraction` apenas como `method`; âncoras ausentes são `not_found` (exit 66), nunca fallback de página pai
- Payload sempre inclui `item_name`
- `resolved_version` opcional é o SemVer concreto somente do crate alvo, ou o canal da stdlib (`stable`) quando conhecido
- Nunca trate versões de dependências na página do docs.rs como a versão do crate
- `--suggest` em 404 faz request extra de `all.html` e lista símbolos próximos (cascata exact→prefix→substring→edit-distance)
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
- Isole storage com flags CLI `--config-dir` / `--cache-dir` (nunca env de produto `DOCSRS_CLI_*`)
- User-Agent default é `docsrs-cli/<version> (+https://github.com/danilo-aguiar-br/docsrs-cli)` (versão = binário)
- User-Agent: `--user-agent` ou TOML `user_agent`; contact: TOML `contact`
- Quando o `planned_params` de dry-run nomeia um crate, ele usa `crate_name` (nunca `crate`): `readme`, `get-item`, `search-in-crate`
- `search-crates` planeja uma consulta, então seu `planned_params` traz `q`, `per_page`, `sort`, `page`, `page_token`, sem nenhum `crate_name`
- Dry-run `planned_params` pode incluir `validation=url_shape_only`, `planned_parent_kind`, `parent_kind_probe` e `planned_method_anchors` para methods
- Forma do envelope dry-run está em [dry-run.schema.json](schemas/dry-run.schema.json)

## Contrato: Regras de schema
- Schemas de payload existem para search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions
- Contratos compartilhados também cobrem `error` e `dry-run` via `schema --cmd error|dry-run`
- Índice de todos os arquivos: [schemas/README.md](schemas/README.md)
- Prefira schemas vivos de `docsrs-cli schema --cmd <name> --json` antes de hardcodar listas de campos
