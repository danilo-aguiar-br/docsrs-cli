---
name: docsrs-cli-pt
description: Esta skill DEVE ativar quando o agente precisar de docsrs-cli, busca de crates no crates.io, fetch de documentação no docs.rs, lookup de item rustdoc, overview readme, listagem de símbolos com search-in-crate, extração tipada get-item com method fail-closed, âncoras de variant e structfield, paginação por page-token, modos de match, sondas doctor, descoberta commands, inventário schema, completions, cache path stats clear, config path show init, designação explícita de alvo em verbos destrutivos, planejamento dry-run, chaves do config.toml XDG, parsing de envelope JSON, redução agent-native de payload, kinds de erro, retry por retryable, ramificação por exit code ou tetos hard-max. Ela DEVE ensinar o catálogo de onze comandos, argv exato, toda flag global, contratos de dados no stdout, política de retry, fluxos e fórmulas prontas para o agente extrair documentação de crates Rust por chamadas estruturadas e NUNCA por raspagem de HTML.
---


# docsrs-cli


## Identidade e Contrato de Execução
### OBRIGATÓRIO
- DEVE invocar o binário como `docsrs-cli` e NUNCA por apelido
- DEVE tratar cada processo como BORN, EXECUTE, FINALIZE, DIE
- DEVE passar `--json` em toda invocação programática
- DEVE parsear stdout como contrato e stderr como diagnóstico
- DEVE esperar JSON automático quando stdout não for um TTY
- DEVE forçar saída humana com `--format markdown` ou `--format text`
- DEVE resolver superfície desconhecida com `commands --json` antes de inventar argv
- DEVE carregar `schema --cmd all --json` quando o payload for desconhecido
- DEVE aplicar precedência flags CLI, depois `config.toml` XDG, depois defaults
- DEVE aceitar somente crates.io, docs.rs, static.docs.rs, doc.rust-lang.org
- DEVE manter o User-Agent embutido salvo override concretamente exigido
- DEVE confiar em rustls sem bypass de certificado
### PROIBIDO
- NUNCA assuma daemon, sessão sticky, telemetria ou estado reaproveitado
- NUNCA parseie stderr como JSON de sucesso
- NUNCA configure knobs por variáveis de ambiente, `.env` de runtime ou chaves de API
- NUNCA raspe HTML de docs.rs ou crates.io quando um comando resolve
- NUNCA peça scraping autenticado, bypass de CAPTCHA, crawling multi-host ou bypass de TLS
- NUNCA escreva histórico de releases nem narrativa de migração na saída


## Seleção e Catálogo de Comandos
### OBRIGATÓRIO
- DEVE executar `search-crates` para descobrir um crate no crates.io
- DEVE executar `readme` para o docblock de overview do docs.rs
- DEVE executar `search-in-crate` para listar símbolos de um crate
- DEVE executar `get-item` para o corpo de um item rustdoc tipado
- DEVE executar `doctor --online --json` antes de trabalho de rede em lote
- DEVE executar `schema --cmd <nome> --json` antes de parsear payload desconhecido
- DEVE executar `version --json` quando a identidade do binário for exigida
- DEVE executar `cache path|stats|clear` e `config path|show|init` para storage e knobs
- DEVE conhecer os onze comandos `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `cache`, `config`
- DEVE passar `--sort` com `relevance|downloads|recent-downloads|recent-updates|new|alphabetical`
- DEVE tratar a query de `search-crates` como opcional só quando `--page-token` a carrega
- DEVE passar kinds de `get-item` entre `module|mod|struct|trait|enum|union|fn|function|method|type|const|constant|static|macro|attr|attribute|derive|variant|structfield|field`
- DEVE tratar `method` como alias de `fn` que ecoa `method`, `mod` de `module`, `const` de `constant`, `field` de `structfield`
- DEVE saber que `variant` e `structfield` NÃO têm página e EXIGEM caminho `Pai::membro`
- DEVE esperá-los em `enum.Option.html#variant.Some` e `struct.Range.html#structfield.start`
- DEVE esperar exit 65 com dica de reescrita quando vierem sem o pai
- NUNCA espere `variant.Name.html`, que o rustdoc jamais serviu
- DEVE dirigir stderr com `-q` ou `-v` ou pela chave XDG `log_directive`, pois `RUST_LOG` NÃO é lido
- DEVE esperar `log_directive` impossível de parsear falhar fechado no load com exit 78
- DEVE esperar `timeout_secs = 0` no TOML sair 78 enquanto o mesmo zero na flag sai 65
- DEVE saber que o locale do host dirige só a prosa de stderr, nunca um knob
- DEVE saber que o produto NÃO lê variável de ambiente, e `--no-color` vence sinais do terminal
- DEVE esperar `error.message` em inglês sob qualquer `--lang`
- DEVE escrever caminhos com `::` ou `/`, permitir prefixo do crate, e esperar hífen virar underscore
- DEVE alcançar `std`, `core`, `alloc` por doc.rust-lang.org automaticamente
- DEVE fixar release com `--crate-version` ou açúcar `nome@versão`
- DEVE passar `--match` com `exact|prefix|substring`, ler o eco `match_mode`, e conhecer aliases `contains` e `substr`
- DEVE esperar token desconhecido de `--match` falhar fechado
- DEVE iniciar lookup em `--match prefix` e escalar deliberadamente
- DEVE passar `--item-type` para filtrar hits de `search-in-crate` por kind
- DEVE passar `--suggest` no miss e ler `error.suggestions` como `{path, kind}`, nunca parseando a mensagem
- DEVE conhecer os vinte schemas `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `agent-surface`, `cache`, `cache-path`, `cache-clear`, `cache-stats`, `config`, `config-path`, `config-show`, `config-init`
- DEVE passar shells de `completions` entre `bash|zsh|fish|elvish|power-shell` com `powershell` como alias
### PROIBIDO
- NUNCA chame `get-item` sem kind e caminho concretos
- NUNCA use `search-in-crate` como busca no crates.io
- NUNCA trate `readme` como README de controle de versão
- NUNCA combine `nome@versão` com `--crate-version` conflitante
- NUNCA combine `--page` com `--page-token`
- NUNCA invente `--match-mode`, pois a flag é `--match`
- NUNCA invente kinds, valores de sort, shells ou nomes de schema fora destas listas
- NUNCA espere `completions` emitir JSON sem `--json`
- NUNCA use `--match substring` como padrão em crates ruidosos


## Designação Explícita de Alvo
### OBRIGATÓRIO
- DEVE saber que dois verbos destroem e recusam alvo ambiente que nunca receberam
- DEVE designar `cache clear` com `--cache-dir <DIR>` ou aceitar com `--yes`
- DEVE designar `config init --force` com `--config-dir <DIR>` ou aceitar com `--yes`
- DEVE esperar exit 64 kind `usage` quando faltarem a flag de alvo e a de aceite
- DEVE ler a recusa, que nomeia o caminho vítima e as duas saídas
- DEVE ler `data.target_source` como `cli` quando o argv nomeou e `xdg` quando houve aceite
- DEVE saber que `cache clear` e `config init` trazem `target_source` e `cache path` traz `source`
- DEVE preferir nomear o diretório sempre que o chamador puder computá-lo
### PROIBIDO
- NUNCA aceite alvo ambiente para alcançar diretório que o chamador nunca resolveu
- NUNCA leia a recusa como defeito, pois o argv nomeia o verbo e o ambiente nomeia a vítima
- NUNCA espere `--yes` em verbo fora destes dois


## Flags Globais
### OBRIGATÓRIO
- DEVE posicionar flags globais antes ou depois do subcomando livremente
- DEVE passar `--json` ou `--format json` para consumo por máquina
- DEVE passar `--format` com `json|markdown|text` e nenhum outro token
- DEVE limitar tempo com `--timeout <SEGS>` e conexão com `--connect-timeout <SEGS>`
- DEVE tratar qualquer um em `0` como entrada inválida fail-closed
- DEVE limitar download com `--max-body-bytes`, teto duro 10485760
- DEVE limitar emissão com `--max-output-bytes`, teto duro 2097152
- DEVE tratar valor acima do teto como fail-closed, nunca clamp silencioso
- DEVE reduzir payload no binário e NUNCA canalizar por `jq` ou `jaq`
- DEVE projetar com `--select <CHAVES>` em CSV ou flag repetida, alias `--fields`
- DEVE tratar chave ausente de `data` como pulada, nunca null emitido
- DEVE saber que `--select` projeta os ELEMENTOS do array quando há resultados
- DEVE passar `--select name` e NUNCA `--select hits`, que devolve objetos vazios
- DEVE filtrar com `--filter` em `chave=valor`, `chave!=valor`, `chave~substring`, com `==` sinônimo
- DEVE repetir `--filter` para conjugar com AND
- DEVE tratar `--filter` malformado como exit `65`, nunca conjunto vazio
- DEVE ordenar com `--sort-by <CHAVE>`, que aceita os caminhos pontilhados de `--select`
- DEVE esperar ordenação ESTÁVEL, comparação numérica, e chave ausente indo para o FIM
- DEVE tratar `--sort-by` em chave inexistente como no-op, nunca erro
- DEVE limitar elementos emitidos com `--max-items <N>`
- DEVE distinguir `--max-items`, que limita a EMISSÃO, de `--limit`, que limita a CONSULTA
- NUNCA DEVE esperar `--limit` global, pois `search-in-crate` é dono desse nome
- DEVE tratar `--max-items 0` como array vazio com `ok: true`
- DEVE parear `--max-items` com `--sort-by`, pois recorte sem ordem é arbitrário
- DEVE descartar repetidos com `--dedupe-by <CHAVE>`, mantendo quem não a tem
- DEVE substituir o payload por `{"count": N}` com `--count-only`, contado após filter e limit
- DEVE encurtar strings com `--truncate-content <N>`, sem partir UTF-8
- DEVE aplicar a ordem filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- DEVE ler `agent_surface` de topo com `input_count`, `output_count`, `limited`, `content_truncated`, `output_truncated`
- DEVE ler `limited` para distinguir resultado pequeno de recorte por `--max-items`
- DEVE esperar `emitted` reescrito para o array reduzido e `total` intacto
- DEVE controlar cache com `--no-cache`, `--cache-ttl-secs`, `--max-cache-bytes`
- DEVE tratar defaults 86400 e 268435456, e `--max-cache-bytes 0` como ilimitado
- DEVE isolar storage com `--config-dir` e `--cache-dir`
- DEVE regular ritmo com `--rate-limit-delay-ms` e dimensionar com `--max-concurrency`, onde `0` autodimensiona
- DEVE ajustar retries com `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms`, `--retry-max-elapsed-ms`
- DEVE tratar `--retry-max-elapsed-ms 0` como derivado do timeout
- DEVE matar retries com `--disable-retry` somente em incidente
- DEVE planejar sem sockets usando `--dry-run`
- DEVE sobrescrever identidade com `--user-agent` só quando concretamente exigido
- DEVE definir locale de stderr com `--lang en` ou `--lang pt-BR` enquanto o JSON fica em inglês
- DEVE silenciar stderr com `-q`, aprofundar tracing com `-v` contável, desligar ANSI com `--no-color`
- DEVE habilitar `--allow-loopback` só para origens locais de teste
- DEVE tratar `--per-page` com padrão 10 e faixa legal de 1 até 100
- DEVE tratar `--per-page` ou `--page` explícitos fora da faixa como fail-closed
- DEVE esperar paginação por `--page-token` clampada em vez de rejeitada
- DEVE tratar `--limit` com padrão 100 e clamp silencioso em 1000
### PROIBIDO
- NUNCA espere `--per-page 500` ser clampado, pois argv explícito falha fechado
- NUNCA passe `--page 0`, pois a página mínima é um
- NUNCA combine `--json` com `--format markdown` ou `--format text`
- NUNCA invente knobs de ambiente para timeout, identidade, caminhos ou retry
- NUNCA use `--allow-loopback` contra hosts de produção
- NUNCA eleve um teto de budget acima do limite duro


## Contrato JSON e Campos de Dados
### OBRIGATÓRIO
- DEVE esperar envelope de sucesso com `schema_version`, `ok`, `command`, `data`, `duration_ms`
- DEVE esperar envelope de falha com as mesmas chaves mais `error` aninhado
- DEVE ler `error.code`, `error.kind`, `error.message`, `error.retryable` em toda falha
- DEVE ler `error.retry_after_secs` só quando presente e honrá-lo
- DEVE conhecer os kinds `usage`, `invalid_input`, `not_found`, `rate_limited`, `unavailable`, `timeout`, `network`, `budget`, `parse`, `config`, `io`, `internal`, `broken_pipe`, `canceled`
- DEVE tratar campo opcional omitido como ausente e NUNCA inventar null
- DEVE ler o canônico `crate_name` e NUNCA o nome de wire `crate`
- DEVE preferir `data.source_url` a qualquer espelho de topo
- DEVE ler `cache_hit` true em serve de disco e false em miss ou bypass
- DEVE ler em `search-crates` os campos `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`, `truncated`
- DEVE ler os campos obrigatórios de hit `name`, `description`, `downloads`, `version`
- DEVE ler os opcionais `exact_match`, `yanked`, `recent_downloads`, `max_version`, `max_stable_version`, `default_version`, `homepage`, `documentation`, `repository`
- DEVE usar `exact_match` para achar o nome literal dentro de lista ranqueada
- DEVE usar `yanked` para rejeitar release retirado antes de buscá-lo
- DEVE usar `max_stable_version` para fixar em vez de adivinhar por `version`
- DEVE ler `data.meta.total` como a contagem upstream
- DEVE paginar com `meta.next_page` e `meta.prev_page` realimentados em `--page-token`
- DEVE esperar `prev_page` omitido na primeira e `next_page` omitido na última
- DEVE ler em `readme` os campos `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`
- DEVE ler em `get-item` os campos `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`
- DEVE ler os opcionais `resolved_version` e `resolved_item_path`
- DEVE exigir `data.extraction` igual a `method` em fetch de método
- DEVE ler `data.anchor_family` para a família real, pois `extraction` diz `method` para todas
- DEVE tratar âncora ausente como `not_found`, nunca sucesso da página pai
- DEVE ler em `search-in-crate` os campos `crate_name`, `query`, `version`, `match_mode`, `item_type`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`
- DEVE comparar `emitted` com `total` para ver quanto o índice foi cortado
- DEVE ler `hits[].name`, `hits[].kind`, `hits[].url` e o opcional `hits[].score`, onde menor ranqueia melhor
- DEVE ler em `version` os campos `name`, `version`, `msrv`, `os`, `arch`
- DEVE ler em `commands` os campos `name`, `version`, `msrv`, `schema_version`, `commands`, `agent_notes`
- DEVE ler em `agent_notes` as chaves `stdout`, `stderr`, `json_auto`, `lifecycle`
- DEVE ler em `completions` os campos `shell` e `script`
- DEVE ler em `doctor` os campos `ok` e `checks` com `name`, `ok`, `detail`
- DEVE esperar vinte checks offline mais exatamente `online_crates_io` e `online_docs_rs` sob `--online`
- DEVE ler em `cache path` os campos `root`, `source` como `cli|xdg|unresolved`, e `no_cache`
- DEVE ler em `cache stats` os campos `root`, `layout`, `entries`, `total_bytes`, `max_bytes`, `ttl_secs`, `parser_version`
- DEVE ler em `cache clear` os campos `root`, `removed_entries`, `freed_bytes`, `target_source`
- DEVE ler em `config path` os campos `config_dir`, `config_file`, `config_source`, `config_file_exists`, `config_toml_loaded`, `cache_dir`, `cache_source`, `dotenv_runtime`, `secrets_layers`
- DEVE ler em `config init` os campos `path`, `config_dir`, `target_source`, `created`, `overwritten`
- NUNCA DEVE ler `config init` como `source`, pois o campo é `target_source`
- DEVE ler `config show` como knobs efetivos após defaults, TOML e flags
- DEVE ler em `config show` os números `timeout_secs`, `connect_timeout_secs`, `max_body_bytes`, `max_output_bytes`, `max_redirects`, `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `rate_limit_delay_ms`, `max_concurrency`, `cache_ttl_secs`, `max_cache_bytes`
- DEVE ler em `config show` o resto `user_agent`, `config_dir`, `cache_dir`, `crates_io_origin`, `docs_rs_origin`, `disable_retry`, `no_cache`, `allow_loopback`, `config_path_source`, `cache_path_source`, `config_toml_loaded`
- DEVE esperar `lang`, `log_directive` e `contact` OMITIDOS de `config show` até serem definidos
- DEVE esperar markdown de readme e get-item já limpo de chrome do rustdoc
- DEVE condicionar retry a `error.retryable` e `error.kind`, nunca só ao exit code
### PROIBIDO
- NUNCA confie em `data` com `ok` false, salvo checks do doctor
- NUNCA aceite `extraction` ausente ou `item_page` como sucesso de método
- NUNCA reprocesse markdown para remover marcas já retiradas
- NUNCA renomeie campos de wire nem misture envelope JSON com NDJSON
- NUNCA aplique fallback ao ler `retryable`, `ok`, `cache_hit`, `truncated`, `empty`
- NUNCA confunda um false legítimo com campo ausente


## Exit Codes e Retry
### OBRIGATÓRIO
- DEVE ramificar no exit code antes de confiar no stdout
- DEVE tratar `0` como sucesso
- DEVE tratar `64` como usage, cobrindo argv inválido, subcomando desconhecido, conflito de flags e alvo ambiente recusado
- DEVE tratar `65` como entrada inválida ou parse, incluindo timeout zero e estouro de teto por flag
- DEVE tratar `66` como não encontrado, incluindo âncora de método ausente
- DEVE tratar `69` como rate limit ou indisponibilidade, portanto retryable
- DEVE tratar `74` como ambíguo até ler `error.kind`
- DEVE tratar `network` no `74` como retryable e `budget` no `74` como permanente
- DEVE tratar `io` no `74` como falha local de sistema de arquivos vinda do ambiente
- DEVE ler `error.retryable` no `io`, pois disco cheio passa e permissão negada não
- NUNCA espere `internal` para falha de arquivo, pois `internal` significa defeito deste binário
- DEVE tratar `78` como falha de configuração, incluindo doctor não saudável e estouro no TOML
- DEVE tratar `70` interno, `124` timeout, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- DEVE retentar somente quando `error.retryable` for true
- DEVE honrar `error.retry_after_secs` antes da próxima tentativa
- DEVE resolver falha de budget elevando o teto rumo ao máximo duro
- DEVE ler `data` do doctor mesmo com `ok` false para inspecionar checks
- DEVE tratar a primeira interrupção como cancelamento e a repetida como saída forçada
### PROIBIDO
- NUNCA retente `64`, `65`, `66`, `78` ou `budget` sem mudar as entradas
- NUNCA trate todo `74` como retryable
- NUNCA mascare exit codes com fallback de sucesso
- NUNCA capture o status depois de um pipe, pois ele reporta o último estágio


## Dry-Run Cache e Config XDG
### OBRIGATÓRIO
- DEVE planejar offline com `--dry-run`, lendo `dry_run` de topo e `data.planned_url`
- DEVE ler `data.planned_params`, cujas chaves mudam por comando
- DEVE esperar `crate_name`, `item_type`, `item_path`, `version` em `get-item` e `search-in-crate`
- DEVE esperar `q`, `per_page`, `sort`, `page` em `search-crates`, que NÃO carrega `crate_name`
- DEVE esperar o limite planejado de `search-in-crate` já clampado em 1000
- DEVE ler `validation`, `planned_parent_kind`, `parent_kind_probe`, `planned_method_anchors` DENTRO de `planned_params`
- DEVE ler `planned_method_anchors` como as âncoras ordenadas que um fetch vivo sondaria
- DEVE esperar `validation` igual a `url_shape_only`, por isso dry-run nunca prova âncora
- DEVE esperar os valores aninhados `cache-path`, `cache-stats`, `cache-clear`, `config-path`, `config-show`, `config-init`
- DEVE esperar em `schema --cmd <nome>` os campos `command`, `schema`, `schema_version`
- DEVE esperar em `schema --cmd all` os campos `mode`, `commands`, `items`, `schema_version`
- DEVE criar o arquivo opcional com `config init` e sobrescrever só com `--force`
- DEVE inspecionar knobs efetivos com `config show --json`
- DEVE definir knobs persistentes só por flags CLI e `config.toml` XDG
- DEVE conhecer as chaves TOML `timeout_secs`, `connect_timeout_secs`, `max_body_bytes`, `max_output_bytes`, `max_redirects`, `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `disable_retry`, `rate_limit_delay_ms`, `max_concurrency`, `user_agent`, `contact`, `lang`, `log_directive`, `crates_io_origin`, `docs_rs_origin`, `allow_loopback`, `cache_ttl_secs`, `max_cache_bytes`, `no_cache`
- DEVE usar `max_redirects`, `contact`, `crates_io_origin` e `docs_rs_origin` pelo TOML, pois nenhuma flag os expõe
- DEVE matar retries de forma persistente com `disable_retry = true` ou `max_retries = 0`
- DEVE habilitar loopback de forma persistente com `allow_loopback = true`
### PROIBIDO
- NUNCA trate URL planejada como prova de âncora viva
- NUNCA execute `config init --force` sem intenção de sobrescrever
- NUNCA declare knob fora da lista de chaves acima


## Fluxos de Execução
### OBRIGATÓRIO
- DEVE resolver crate desconhecido encadeando `search-crates`, `readme`, `search-in-crate`, `get-item`
- DEVE recuperar miss reexecutando com `--suggest`, lendo sugestões e refazendo o fetch
- DEVE paginar lendo `meta.next_page`, alimentando `--page-token`, parando com token vazio
- DEVE estreitar símbolos ruidosos de `--match prefix` para `--match exact`
- DEVE sondar com `doctor --online --json` e abortar o lote quando as sondas falharem
- DEVE diagnosticar cache lendo `cache_hit`, inspecionando `cache stats`, limpando só quando obsoleto
- DEVE planejar fetch arriscado com `--dry-run` antes da chamada viva
- DEVE descobrir superfície encadeando `commands`, `schema --cmd all`, e o schema alvo
- DEVE isolar experimentos pareando `--config-dir` com `--cache-dir` em cada chamada
- DEVE corrigir falha de budget elevando `--max-body-bytes`, e estouro reduzindo-o
- DEVE corrigir alvo recusado nomeando o diretório em vez de recorrer ao aceite
- DEVE capturar stdout e o status de saída separadamente antes de parsear
- DEVE encadear fluxos no agente, pois o processo nunca persiste estado
### PROIBIDO
- NUNCA ignore sugestões quando o chamador ainda precisa do símbolo
- NUNCA assuma rede saudável sem doctor após falha de sonda
- NUNCA trate saída de dry-run como evidência de método vivo


## Fórmulas Prontas
### OBRIGATÓRIO
- DEVE copiar estas fórmulas e substituir apenas os placeholders
- DEVE executar `docsrs-cli search-crates <QUERY> --json`
- DEVE executar `docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --json`
- DEVE executar `docsrs-cli search-crates <QUERY> --sort downloads --json`, trocando por `relevance`, `recent-downloads`, `recent-updates`, `new`, `alphabetical`
- DEVE executar `docsrs-cli search-crates --page-token <TOKEN> --json`
- DEVE executar `docsrs-cli readme <CRATE> --json`
- DEVE executar `docsrs-cli readme <CRATE>@<VERSION> --json`, igual a `--crate-version <VERSION>`
- DEVE executar `docsrs-cli readme std --json`
- DEVE executar `docsrs-cli get-item <CRATE> <KIND> <PATH> --json`
- DEVE executar `docsrs-cli get-item <CRATE>@<VERSION> <KIND> <PATH> --suggest --json`
- DEVE executar `docsrs-cli get-item serde trait Serialize --json`
- DEVE executar `docsrs-cli get-item tokio struct runtime::Runtime --json`
- DEVE executar `docsrs-cli get-item tokio struct runtime/Runtime --json`
- DEVE executar `docsrs-cli get-item tokio method runtime::Runtime::new --json`
- DEVE executar `docsrs-cli get-item std method iter::Iterator::next --json`
- DEVE executar `docsrs-cli get-item std type iter::Iterator::Item --json`
- DEVE executar `docsrs-cli get-item std const time::Duration::MAX --json`
- DEVE executar `docsrs-cli get-item std const u32::MAX --json`
- DEVE executar `docsrs-cli get-item std variant option::Option::Some --json`
- DEVE executar `docsrs-cli get-item std structfield ops::Range::start --json`
- DEVE executar `docsrs-cli get-item async-trait attribute async-trait --json`
- DEVE executar `docsrs-cli get-item tokio fn task::spawn --json`
- DEVE executar `docsrs-cli get-item tokio mod runtime --json`
- DEVE executar `docsrs-cli get-item serde derive Serialize --json`
- DEVE saber que só a caixa do segmento PAI roteia `method`, `type` e `const`
- DEVE ler pai em maiúscula como tipo que cai em âncora do pai, pois módulo maiúsculo dispara `non_snake_case`
- DEVE esperar pai em minúscula manter página própria, e a caixa da FOLHA não mudar nada
- DEVE ler typo de caixa como not-found com sugestões, nunca como forma não suportada
- DEVE executar `docsrs-cli search-in-crate <CRATE> <QUERY> --json`
- DEVE executar `docsrs-cli search-in-crate <CRATE> "" --limit 50 --json`
- DEVE executar `docsrs-cli search-in-crate <CRATE> <QUERY> --item-type struct --json`
- DEVE executar `docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json`, trocando por `prefix`, `substring`, `contains`, `substr`
- DEVE executar `docsrs-cli search-in-crate <CRATE> <QUERY> --crate-version <VERSION> --limit 25 --json`
- DEVE executar `docsrs-cli version --json`
- DEVE executar `docsrs-cli doctor --json`
- DEVE executar `docsrs-cli doctor --online --json`
- DEVE executar `docsrs-cli commands --json`
- DEVE executar `docsrs-cli schema --cmd all --json`
- DEVE executar `docsrs-cli schema --cmd get-item --json`, trocando por qualquer dos vinte nomes
- DEVE executar `docsrs-cli schema --cmd agent-surface --json` para o contrato da redução
- DEVE executar `docsrs-cli completions bash`, trocando por `zsh`, `fish`, `elvish`, `power-shell`
- DEVE executar `docsrs-cli completions bash --json` para envelopar o script
- DEVE executar `docsrs-cli cache path --json`
- DEVE executar `docsrs-cli cache stats --json`
- DEVE executar `docsrs-cli cache clear --cache-dir <DIR> --json` para limpar raiz nomeada
- DEVE executar `docsrs-cli cache clear --yes --json` para limpar a raiz ambiente de propósito
- DEVE executar `docsrs-cli config path --json`
- DEVE executar `docsrs-cli config show --json`
- DEVE executar `docsrs-cli config init --json`
- DEVE executar `docsrs-cli config init --force --config-dir <DIR> --json` para sobrescrever diretório nomeado
- DEVE executar `docsrs-cli config init --force --yes --json` para sobrescrever o ambiente de propósito
- DEVE executar `docsrs-cli --dry-run readme <CRATE> --json`
- DEVE executar `docsrs-cli --dry-run get-item <CRATE> method <TYPE::method> --json`
- DEVE executar `docsrs-cli --dry-run search-crates <QUERY> --json`
- DEVE executar `docsrs-cli --dry-run search-in-crate <CRATE> <QUERY> --limit 5000 --json` para provar o clamp
- DEVE executar `docsrs-cli --timeout 30 --connect-timeout 5 -q readme <CRATE> --json`
- DEVE executar `docsrs-cli --no-cache readme <CRATE> --json`
- DEVE executar `docsrs-cli --cache-ttl-secs 86400 --max-cache-bytes 268435456 cache stats --json`
- DEVE executar `docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme <CRATE> --json`
- DEVE executar `docsrs-cli --select planned_url --dry-run readme <CRATE> --json`, ou `--fields` como alias
- DEVE executar `docsrs-cli --count-only --dry-run readme <CRATE> --json`
- DEVE executar `docsrs-cli --truncate-content 200 readme <CRATE> --json`
- DEVE executar `docsrs-cli --dedupe-by name search-in-crate <CRATE> <QUERY> --json`
- DEVE executar `docsrs-cli --sort-by downloads search-crates <QUERY> --json`
- DEVE executar `docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name search-in-crate <CRATE> "" --limit 200 --json`
- DEVE executar `docsrs-cli --max-items 5 --count-only search-in-crate <CRATE> "" --limit 200 --json`
- DEVE executar `docsrs-cli --filter 'command=readme' --filter 'ok!=false' --dry-run readme <CRATE> --json`
- DEVE executar `docsrs-cli --filter 'chave sem operador' --dry-run readme <CRATE> --json` para provar filtro fail-closed
- DEVE executar `docsrs-cli --rate-limit-delay-ms 200 search-crates <QUERY> --json`
- DEVE executar `docsrs-cli --max-concurrency 0 search-in-crate <CRATE> <QUERY> --json`
- DEVE executar `docsrs-cli --max-retries 3 --retry-base-ms 100 --retry-max-delay-ms 2000 --retry-max-elapsed-ms 10000 doctor --json`
- DEVE executar `docsrs-cli --disable-retry doctor --json`
- DEVE executar `docsrs-cli --user-agent 'my-agent (+https://example.com/contact)' version --json`
- DEVE executar `docsrs-cli --lang pt-BR --format markdown doctor`
- DEVE executar `docsrs-cli --no-color --format text version`
- DEVE executar `docsrs-cli -v doctor --json`
- DEVE executar `docsrs-cli --config-dir <CFG> --cache-dir <CACHE> config path --json`
- DEVE executar `docsrs-cli --allow-loopback doctor --json`
### PROIBIDO
- NUNCA invente argv fora desta superfície
- NUNCA documente histórico de releases dentro desta skill
