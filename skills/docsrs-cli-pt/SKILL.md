---
name: docsrs-cli
description: Esta skill DEVE ativar quando o agente precisar de docsrs-cli, busca no crates.io, fetch no docs.rs, lookup rustdoc, get-item, readme, search-crates, search-in-crate, version, doctor online, doctor data.ok, commands, schema, completions, cache, config, eco page-token, match exact prefix substring, get-item suggest em cascata, extraction de método, hífen em item_path, scrub rustdoc de § e Copy item path, source_url dual preferindo data.source_url, cache_hit, envelopes JSON, dry-run de URL, cache config XDG, timeout zero exit 65, exit 74 budget vs network, budget não retryable, locale via --lang ou DOCSRS_CLI_LANG, ou allowlist de hosts. Ela DEVE ensinar o catálogo dos 11 comandos, argv exato, flags globais, contratos JSON, exit codes, retry por error.retryable, fluxos multi-etapa, fórmulas prontas e ciclo one-shot BORN-EXECUTE-FINALIZE-DIE para obter docs de crates Rust sem raspar HTML.
---

# docsrs-cli
Produto `docsrs-cli`. Repositório canônico `https://github.com/danilo-aguiar-br/docsrs-cli`. CLI one-shot BORN-EXECUTE-FINALIZE-DIE para crates.io e docs.rs. Stdout é o contrato de dados. Stderr é diagnóstico apenas.

## Identidade e Ciclo de Vida
### REQUIRED
- DEVE tratar o binário como sempre `docsrs-cli`
- DEVE tratar cada processo como BORN, EXECUTE, FINALIZE, DIE sem sessão sticky
- DEVE usar `--json` para todo consumidor programático
- DEVE tratar stdout como contrato de dados e stderr como diagnóstico
- DEVE esperar JSON automático quando stdout não é TTY
- DEVE forçar humano com `--format markdown` ou `--format text`
- DEVE descobrir a árvore viva com `commands --json` quando em dúvida
- DEVE usar o User-Agent padrão embutido salvo override concreto obrigatório
- DEVE manter envelopes JSON sempre em inglês
- DEVE forçar locale humano de stderr só com `--lang en`, `--lang pt-BR` ou env `DOCSRS_CLI_LANG`
- DEVE aplicar precedência de knobs flags depois XDG `config.toml` depois defaults embutidos
### FORBIDDEN
- NUNCA assuma daemon, sessão sticky ou telemetria de produto
- NUNCA parseie stderr como JSON de sucesso
- NUNCA reutilize estado de processo entre invocações
- NUNCA invente subcomando fora do catálogo abaixo
- NUNCA use envs de produto `DOCSRS_CLI_USER_AGENT`, `DOCSRS_CLI_CONTACT`, `DOCSRS_CLI_DISABLE_RETRY` ou `DOCSRS_CLI_TIMEOUT`
- NUNCA coloque narrativa de histórico de versões dentro desta skill
### Correct Pattern
```bash
docsrs-cli --version
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli --lang pt-BR doctor --json
DOCSRS_CLI_LANG=pt-BR docsrs-cli doctor --json
```

## Matriz de Decisão — Escolha o Comando
### REQUIRED
- DEVE executar `search-crates` quando o objetivo é achar um crate no crates.io
- DEVE executar `readme` quando o objetivo é o overview docblock do docs.rs (não README do git)
- DEVE executar `search-in-crate` quando o objetivo é listar ou filtrar símbolos no `all.html` de um crate
- DEVE executar `get-item` quando o objetivo é o corpo completo de um item rustdoc tipado
- DEVE executar `get-item` com kind `method` (alias de `fn`) para métodos associados; o produto resolve a página do tipo pai mais `#method.name`
- DEVE executar `doctor --online --json` antes de trabalho em lote na rede quando a saúde da rede é desconhecida
- DEVE executar `schema --cmd <name> --json` antes de parsear payload desconhecido
- DEVE executar `version --json` quando a identidade do binário é exigida
- DEVE executar `cache stats|clear` e `config path|show|init` para storage e knobs
### FORBIDDEN
- NUNCA raspe HTML de docs.rs ou crates.io com regex quando esta CLI resolve a necessidade
- NUNCA chame `get-item` sem kind e path concretos
- NUNCA use `search-in-crate` como busca no crates.io (isso é `search-crates`)
- NUNCA trate `readme` como conteúdo de README de controle de versão
### Correct Pattern
```bash
docsrs-cli search-crates serde --sort downloads --json
docsrs-cli readme serde --json
docsrs-cli search-in-crate serde Serialize --match prefix --limit 20 --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
```

## Catálogo Completo de Comandos
### REQUIRED
- DEVE conhecer os 11 top-level commands
- DEVE conhecer subcomandos `cache clear`, `cache stats`
- DEVE conhecer subcomandos `config path`, `config show`, `config init`
- DEVE conhecer shells de `completions` (script bruto por padrão)
- DEVE usar `--match` com valores `exact|prefix|substring` (padrão `prefix`); o campo JSON é sempre `match_mode`
- DEVE tratar `--limit` como clampado a 1000 (incluindo limit planejado no dry-run)
- DEVE conhecer os nomes de inventário de schema `search-crates`, `readme`, `get-item`, `search-in-crate`, `version`, `doctor`, `commands`, `schema`, `completions`, `error`, `dry-run`, `cache`, `config`
### FORBIDDEN
- NUNCA omita um top-level command do catálogo operacional
- NUNCA invente `--match-mode` (a flag é `--match`)
- NUNCA invente nomes de schema fora do inventário acima
- NUNCA assuma que completions emitem envelope JSON sem `--json` explícito
### Correct Pattern
```bash
# 1) search-crates  (QUERY DEVE ser omitida quando --page-token carrega a query completa)
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates <QUERY> --sort alphabetical --json
docsrs-cli search-crates --page-token '<TOKEN_DE_meta.next_page>' --json
# --sort values: relevance|downloads|recent-downloads|recent-updates|new|alphabetical
# --page conflita com --page-token

# 2) readme
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json
docsrs-cli readme std --json

# 3) get-item
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --suggest --json
# KIND: module|struct|trait|enum|union|fn|function|method|type|const|constant|static|macro|attr|attribute|derive
# method é alias de fn; métodos resolvem para página do tipo pai + #method.name
# PATH: usa :: ou / ; prefixo do crate opcional; segmentos com hífen normalizam para underscore

# 4) search-in-crate
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> "" --limit 50 --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type function --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match prefix --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match substring --json
docsrs-cli search-in-crate <CRATE> <QUERY> --crate-version <VERSION> --limit 100 --json
# --limit clampado a 1000; match padrão é prefix; hits ordenados por score (menor é melhor)

# 5) version
docsrs-cli version --json
docsrs-cli --format markdown version

# 6) doctor
docsrs-cli doctor --json
docsrs-cli doctor --online --json
# --online adiciona sondas de rede online_crates_io e online_docs_rs

# 7) commands
docsrs-cli commands --json

# 8) schema
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# 9) completions  (script de shell bruto por padrão; JSON só com --json explícito)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# 10) cache
docsrs-cli cache stats --json
docsrs-cli cache clear --json

# 11) config
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json
```

## Flags Globais — Knobs de Execução
### REQUIRED
- DEVE usar `--json` ou `--format json` em pipelines de agente
- DEVE aceitar flags globais antes ou depois do subcomando (flags globais do clap)
- DEVE usar `--timeout <SECS>` e `--connect-timeout <SECS>` para limites de wall-clock e connect
- DEVE tratar `--timeout 0` e `--connect-timeout 0` explícitos como fail-closed invalid input (exit 65)
- DEVE aumentar `--max-body-bytes` quando `error.kind=budget` em vez de retentar
- DEVE tratar body acima de `--max-body-bytes` como erro duro `kind=budget` com `retryable=false`
- DEVE tratar emissão acima de `--max-output-bytes` como sucesso com `truncated:true` (não budget)
- DEVE usar `--no-cache` só quando frescor é obrigatório
- DEVE usar `--cache-ttl-secs`, `--max-cache-bytes`, `--cache-dir`, `--config-dir` para controlar cache em disco e raízes de config
- DEVE usar `--max-body-bytes` e `--max-output-bytes` para capar download e emissão (tetos duros do produto se aplicam)
- DEVE usar `--rate-limit-delay-ms` e `--max-concurrency` para polidez e workers de parse
- DEVE tratar `--max-concurrency 0` como auto (CPU e RAM livre)
- DEVE usar `--max-retries`, `--retry-base-ms`, `--retry-max-delay-ms` e `--disable-retry` para controlar retries HTTP
- DEVE usar `-q` / `--quiet` para suprimir ruído não essencial de stderr em pipelines de agente
- DEVE usar `-v` / `--verbose` só quando diagnóstico mais profundo for obrigatório (flag contável)
- DEVE usar `--no-color` quando cor ANSI deve ser desabilitada
- DEVE NUNCA combinar `--json` com `--format markdown` ou `--format text` na mesma invocação
- DEVE tratar `search-crates --per-page` com default 10 e max 100
- DEVE tratar `search-in-crate --limit` com default 100 e clamp duro 1000
- DEVE tratar `--cache-ttl-secs` default 86400 e `--max-cache-bytes` default 268435456 (0 = ilimitado)
- DEVE tratar tetos duros `--max-body-bytes` 10485760 (10 MiB) e `--max-output-bytes` 2097152 (2 MiB)
- DEVE usar `--dry-run` para planejar URLs sem abrir sockets de rede
- DEVE usar `--user-agent` só quando override concreto é obrigatório
- DEVE usar `--lang en` ou `--lang pt-BR` só para locale humano de stderr (JSON permanece em inglês)
- DEVE aceitar envs de path `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR`, `DOCSRS_CLI_CACHE_DIR`
- DEVE aceitar env de locale `DOCSRS_CLI_LANG` só para stderr humano (não timeout, UA ou retry)
### FORBIDDEN
- NUNCA invente knobs de env de produto para timeout, UA, contact ou retry
- NUNCA trate envs de path como portadores de timeout, UA ou política de retry
- NUNCA espere `.env` em runtime de produto
- NUNCA armazene API keys no produto
### Correct Pattern
```bash
docsrs-cli --timeout 30 --connect-timeout 5 -q get-item serde trait Serialize --json
docsrs-cli --no-cache readme tokio --json
docsrs-cli --dry-run --max-retries 0 --retry-base-ms 100 --retry-max-delay-ms 2000 search-crates serde --json
docsrs-cli --disable-retry doctor --online --json
docsrs-cli --lang pt-BR --format markdown doctor
docsrs-cli --no-color --format text version
docsrs-cli --cache-dir /tmp/docsrs-cache --config-dir /tmp/docsrs-cfg config path --json
docsrs-cli --rate-limit-delay-ms 200 --max-concurrency 2 search-in-crate serde "" --limit 50 --json
docsrs-cli --max-concurrency 0 search-in-crate serde Serialize --json
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme serde --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config show --json
DOCSRS_CLI_LANG=en docsrs-cli --format markdown doctor
```

## Comandos de Busca e Fetch
### REQUIRED
- DEVE usar `search-crates` para busca no crates.io
- DEVE usar `readme` para overview do docs.rs (não git README)
- DEVE usar `get-item` para item rustdoc tipado
- DEVE usar `search-in-crate` para símbolos no `all.html`
- DEVE aceitar `item_path` com `::` ou `/`
- DEVE aceitar segmentos com hífen em `item_path` e esperar normalização para underscore nos paths rustc
- DEVE tratar `std`, `core` e `alloc` via doc.rust-lang.org
- DEVE passar `--crate-version` quando a versão concreta for exigida
- DEVE usar `--match prefix` (padrão) para reduzir ruído tipo Serialize; escalar para `exact` ou `substring` de propósito
- DEVE usar `get-item --suggest` em 404 para obter símbolos próximos (um request all.html; cascata exact→prefix→substring→edit-distance)
- DEVE tratar `method` como alias de `fn`; métodos associados resolvem para página do tipo pai mais `#method.name`
- DEVE esperar `data.extraction` opcional `method` ou `item_page` em fetches de método
- DEVE esperar markdown de produto em readme/get-item já com scrub de chrome rustdoc (marcas de seção `§` e strings de UI `Copy item path`)
- DEVE esperar que o eco de `search-crates` após `--page-token` bata com a URL efetiva
- DEVE esperar `resolved_version` de canais stdlib como `stable` quando o produto devolver esse rótulo de canal
- DEVE esperar resolução SemVer de `latest` apenas do crate alvo (não de versões de dependências)
### FORBIDDEN
- NUNCA raspe HTML de docs.rs com regex quando a CLI resolve o item
- NUNCA re-raspe ou re-aplique regex no markdown só para remover `§` ou `Copy item path` (o produto já faz scrub)
- NUNCA invente kinds fora do conjunto suportado
- NUNCA omita `--json` em pipelines de agente
- NUNCA combine `--page` e `--page-token` na mesma invocação de `search-crates`
- NUNCA use substring como padrão em crates ruidosos
### Correct Pattern
```bash
docsrs-cli search-crates serde --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates --page-token '<TOKEN>' --json
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
docsrs-cli readme tokio --crate-version 1.40.0 --json
docsrs-cli readme std --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
docsrs-cli get-item async-trait attribute async-trait --json
docsrs-cli get-item serde trait Serde --suggest --json
docsrs-cli search-in-crate reqwest Client --item-type struct --match prefix --limit 20 --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate std Option --json
```

## Descoberta Doctor Version Schema Completions
### REQUIRED
- DEVE rodar `commands --json` antes de inventar subcomandos
- DEVE rodar `schema --cmd <name> --json` para toda superfície de payload que for parsear
- DEVE rodar `doctor --json` quando paths, TLS ou retry parecerem errados
- DEVE rodar `doctor --online --json` antes de trabalho em lote na rede para sondar `online_crates_io` e `online_docs_rs`
- DEVE rodar `version --json` para identidade do binário
- DEVE gerar completions só com shells suportados
- DEVE esperar script de shell bruto de `completions` salvo `--json` explícito
- DEVE validar top-level `ok` e exit code do processo antes de confiar em sucesso
- DEVE tratar doctor como saudável só quando top-level `ok` e `data.ok` forem ambos true
- DEVE ainda ler `data` do doctor quando `ok` for false para inspecionar checks falhos (exit 78)
### FORBIDDEN
- NUNCA invente flags fora de `commands` e `--help`
- NUNCA ignore `schema_version`
- NUNCA pule `schema --cmd` vivo para `error`, `dry-run`, `schema` ou `completions` ao parsear essas superfícies
- NUNCA pule `doctor --online` antes de fetches em lote quando a saúde da rede é desconhecida
- NUNCA trate doctor com exit 0 e `data.ok=false` como saudável (envelope `ok` espelha `data.ok`)
- NUNCA descarte `data` do doctor só porque `ok` é false
### Correct Pattern
```bash
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli completions zsh
docsrs-cli completions power-shell
```

## Dry-Run Cache Config
### REQUIRED
- DEVE usar `--dry-run` para planejar URLs sem rede
- DEVE esperar envelopes de sucesso dry-run com `dry_run=true` e `data.planned_url` mais `data.planned_params`
- DEVE esperar planned params com `crate_name` canônico (nunca `crate`)
- DEVE esperar planned `limit` de search-in-crate clampado a 1000 mesmo quando `--limit` for maior
- DEVE esperar eco dry-run de search-crates com page-token batendo com a URL planejada efetiva
- DEVE usar `cache stats --json` e `cache clear --json`
- DEVE usar `config path|show|init --json`
- DEVE usar `config init --force` só para sobrescrever `config.toml` existente
- DEVE definir knobs de produto via flags CLI e XDG `config.toml` somente
- DEVE matar retries com `--disable-retry` ou TOML `disable_retry = true` / `max_retries = 0`
### FORBIDDEN
- NUNCA espere `.env` em runtime de produto
- NUNCA armazene API keys no produto
- NUNCA use `config init --force` sem intenção de sobrescrever
- NUNCA invente knobs de env de produto como `DOCSRS_CLI_USER_AGENT`, `DOCSRS_CLI_CONTACT`, `DOCSRS_CLI_DISABLE_RETRY`, `DOCSRS_CLI_TIMEOUT`
### Correct Pattern
```bash
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
docsrs-cli --dry-run search-in-crate serde Serialize --limit 5000 --json
docsrs-cli --dry-run get-item async-trait attribute async-trait --json
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config init --force --json
docsrs-cli --disable-retry doctor --json
```

## Contrato JSON
### REQUIRED
- DEVE esperar sucesso com `schema_version`, `ok`, `command`, `data`, `duration_ms`
- DEVE esperar falha com `ok:false` e objeto `error` no caminho JSON
- DEVE ler `error.code`, `error.kind`, `error.message`, `error.retryable` em toda falha
- DEVE ler `error.retry_after_secs` opcional quando presente e honrá-lo antes de retentar
- DEVE conhecer kinds no wire `usage|invalid_input|not_found|rate_limited|unavailable|timeout|network|budget|parse|config|internal|broken_pipe|canceled`
- DEVE preferir `data.source_url` quando presente; DEVE tratar `source_url` top-level do envelope como espelho só em ops de fetch
- DEVE tratar campos opcionais ausentes como omitidos, NUNCA inventar JSON null
- DEVE tratar `truncated:true` em `search-in-crate` como corte por `--limit`
- DEVE tratar `truncated:true` em `readme` / `get-item` como corte de emissão por `--max-output-bytes` (caminho de sucesso)
- DEVE ler `cache_hit` (bool) nos dados de comandos de rede quando presente
- DEVE interpretar `cache_hit:true` como servido do cache em disco (sem fetch de body na rede)
- DEVE interpretar `cache_hit:false` como miss ou bypass
- DEVE ler `crate_name` canônico em dados de readme / get-item / search-in-crate (NUNCA campo wire `crate`)
- DEVE ler campos de `search-crates` `query`, `page`, `per_page`, `sort`, `hits`, `meta`, `cache_hit`
- DEVE ler campos de `readme` `crate_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `cache_hit`; `resolved_version` opcional
- DEVE ler campos de `get-item` `crate_name`, `item_type`, `item_path`, `item_name`, `version`, `markdown`, `empty`, `truncated`, `source_url`, `title`, `cache_hit`; `resolved_version` opcional; `extraction` opcional (`method`|`item_page`)
- DEVE ler campos de `search-in-crate` `crate_name`, `query`, `version`, `match_mode`, `total`, `emitted`, `hits`, `truncated`, `source_url`, `cache_hit`; `item_type` opcional
- DEVE ler `hits[].name`, `hits[].kind`, `hits[].url` e `hits[].score` opcional em search-in-crate (score menor é melhor)
- DEVE esperar `markdown` de readme/get-item sem chrome rustdoc `§` e sem `Copy item path`
- DEVE paginar search-crates via `data.meta.next_page` / `data.meta.prev_page` como `--page-token`
- DEVE confiar no eco de `query`/`page`/`per_page`/`sort` após `--page-token` como request efetiva
- DEVE parsear envelopes de erro via `schema --cmd error` quando o formato for desconhecido
- DEVE gatear retries por `error.retryable` e `error.kind`, nunca só pelo exit code
- DEVE esperar falhas no caminho humano com stdout vazio e uma linha em stderr
### FORBIDDEN
- NUNCA confie em `data` de sucesso quando `ok` é false exceto inspeção de checks do doctor
- NUNCA misture NDJSON e JSON de envelope no mesmo parse
- NUNCA invente null para campos que o produto omite
- NUNCA renomeie campos do wire (`crate` é proibido; use `crate_name`)
- NUNCA retente `kind=budget`
- NUNCA prefira `source_url` top-level sobre `data.source_url` quando ambos existem
### Correct Pattern
```bash
out=$(docsrs-cli -q get-item serde trait Serialize --json) || code=$?
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.source_url // .source_url // empty'
echo "$out" | jaq -r '.data.crate_name'
echo "$out" | jaq -r '.data.item_name'
echo "$out" | jaq -r '.data.item_type'
echo "$out" | jaq -r '.data.item_path'
echo "$out" | jaq -r '.data.title // empty'
echo "$out" | jaq -r '.data.extraction // empty'
echo "$out" | jaq -r '.data.cache_hit'
echo "$out" | jaq -r '.data.truncated'
echo "$out" | jaq -r '.data.resolved_version // empty'
echo "$out" | jaq -e '(.data.markdown // "") | (contains("§") | not)'
echo "$out" | jaq -e '(.data.markdown // "") | (contains("Copy item path") | not)'

out=$(docsrs-cli -q get-item tokio method runtime::Runtime::new --json) || code=$?
echo "$out" | jaq -r '.data.extraction // empty'
echo "$out" | jaq -r '.data.item_name'

out=$(docsrs-cli -q search-in-crate serde Serialize --match prefix --json) || code=$?
echo "$out" | jaq -r '.data.match_mode'
echo "$out" | jaq -c '.data.hits[0] | {name, kind, url, score}'

out=$(docsrs-cli -q search-crates serde --per-page 5 --json) || code=$?
token=$(echo "$out" | jaq -r '.data.meta.next_page // empty')
[ -n "$token" ] && docsrs-cli -q search-crates --page-token "$token" --json
```

## Fluxos de Agente — Prompts de Execução
### REQUIRED
- DEVE executar o fluxo A com crate desconhecido: `search-crates` → escolher nome → `readme` → `search-in-crate` → `get-item`
- DEVE executar o fluxo B em 404 de get-item: reexecutar com `--suggest`, parsear sugestões de `error.message`, escolher símbolo e chamar `get-item` de novo
- DEVE executar o fluxo C de paginação: ler `meta.next_page`, reinvocar `search-crates --page-token`, parar quando o token estiver vazio
- DEVE executar o fluxo D para nomes ruidosos: `search-in-crate --match prefix`, depois escalar para `exact` quando o leaf for conhecido
- DEVE executar o fluxo E antes de jobs em lote na rede: `doctor --online --json`, abortar ou retentar quando as sondas falharem
- DEVE executar o fluxo F de diagnóstico de cache: ler `cache_hit`; usar `cache stats|clear`; usar `--no-cache` só quando frescor for obrigatório
- DEVE executar o fluxo G de planejamento offline: `--dry-run` primeiro, depois fetch ao vivo após revisão do plano
- DEVE executar o fluxo H de descoberta meta: `commands` → `schema --cmd` → `version` → `completions` opcional
- DEVE executar o fluxo I de knobs de storage: `config path|show|init` e `cache stats|clear` sob `DOCSRS_CLI_HOME` isolado quando em sandbox
- DEVE manter invocações one-shot; encadear fluxos no agente, não em processo longo
- DEVE tratar Fórmulas Prontas como a superfície argv completa dos 11 comandos; fluxos são cadeias multi-etapa por cima
### FORBIDDEN
- NUNCA ignore texto de suggest após 404 quando o usuário ainda precisa do símbolo
- NUNCA passe `--page` e `--page-token` juntos
- NUNCA assuma rede saudável sem doctor quando sondas anteriores falharam
- NUNCA mascare exit codes com `|| true` em pipelines de agente
### Correct Pattern
```bash
# A) descobrir crate → overview → símbolos → item
docsrs-cli -q search-crates async --sort downloads --json
docsrs-cli -q readme tokio --json
docsrs-cli -q search-in-crate tokio Runtime --match prefix --limit 20 --json
docsrs-cli -q get-item tokio struct runtime::Runtime --json

# B) 404 + suggest
set +e
out=$(docsrs-cli -q get-item serde trait Serde --suggest --json)
code=$?
set -e
# se code==66, parseie error.message para suggestions, depois:
docsrs-cli -q get-item serde trait Serialize --json

# C) paginação page-token
page=$(docsrs-cli -q search-crates async --per-page 10 --json)
token=$(echo "$page" | jaq -r '.data.meta.next_page // empty')
while [ -n "$token" ]; do
  page=$(docsrs-cli -q search-crates --page-token "$token" --json)
  token=$(echo "$page" | jaq -r '.data.meta.next_page // empty')
done

# D) match prefix depois exact
docsrs-cli -q search-in-crate serde Serialize --match prefix --limit 20 --json
docsrs-cli -q search-in-crate serde Serialize --match exact --json

# E) doctor online antes de lote
docsrs-cli doctor --online --json
docsrs-cli -q readme tokio --json
docsrs-cli -q get-item tokio struct runtime::Runtime --json

# F) cache_hit + superfície cache
docsrs-cli -q readme serde --json | jaq -r '.data.cache_hit'
docsrs-cli --no-cache -q readme serde --json | jaq -r '.data.cache_hit'
docsrs-cli cache stats --json
docsrs-cli cache clear --json

# G) dry-run depois live
docsrs-cli --dry-run -q get-item tokio method runtime::Runtime::new --json
docsrs-cli -q get-item tokio method runtime::Runtime::new --json

# H) descoberta meta
docsrs-cli commands --json
docsrs-cli schema --cmd error --json
docsrs-cli version --json
docsrs-cli completions bash --json

# I) sandbox de storage
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config path --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config show --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli cache stats --json
```

## Exit Codes e Retry
### REQUIRED
- DEVE ramificar no exit code antes do stdout
- DEVE tratar `0` como sucesso
- DEVE tratar `2` como argv clap inválido
- DEVE tratar `64` usage, `65` invalid input/parse (inclui timeout 0), `66` not found
- DEVE tratar `69` rate limit/unavailable e `124` timeout como retryable quando `error.retryable` é true
- DEVE tratar exit `74` como ambíguo até ler `error.kind`
- DEVE tratar `kind=network` no exit `74` como retryable
- DEVE tratar `kind=budget` no exit `74` como permanente na mesma config (`retryable=false`)
- DEVE tratar `78` config (incluindo doctor unhealthy), `70` internal, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- DEVE retentar só quando `error.retryable` é true (`69`, `74`/`network` retryable, `124`)
- DEVE honrar `error.retry_after_secs` e `Retry-After` do upstream quando presente
- DEVE aumentar `--max-body-bytes` em budget em vez de retentar
- DEVE honrar kill switch `--disable-retry` ou TOML `disable_retry` / `max_retries=0` apenas em incidente ou debug
- DEVE tratar o primeiro Ctrl-C / SIGINT como cancel cooperativo exit `130`; o segundo Ctrl-C em 5s força exit `130`
- DEVE tratar SIGTERM / SIGHUP como terminate exit `143`
- DEVE tratar Ctrl+Break e fechamento de console no Windows como terminate exit `143`
### FORBIDDEN
- NUNCA retente `64`, `65`, `66`, `78` ou `kind=budget` sem mudar inputs/config
- NUNCA trate todo exit `74` como retryable
- NUNCA mascare exit codes com `|| true` em pipelines de agente
### Correct Pattern
```bash
set +e
out=$(docsrs-cli -q --timeout 15 get-item missing-crate-xyz struct Foo --json)
code=$?
set -e
kind=$(echo "$out" | jaq -r '.error.kind // empty')
retryable=$(echo "$out" | jaq -r '.error.retryable // false')
retry_after=$(echo "$out" | jaq -r '.error.retry_after_secs // empty')
case "$code" in
  0) echo ok ;;
  66) echo not_found ;;
  69|124) echo retryable after=${retry_after:-0} ;;
  74)
    if [ "$kind" = budget ] || [ "$retryable" = false ]; then
      echo permanent_budget_or_non_retryable
    else
      echo retryable_network after=${retry_after:-0}
    fi
    ;;
  65) echo invalid_input_or_timeout_zero ;;
  78) echo config_or_doctor_unhealthy ;;
  *) echo fail_$code ;;
esac

# caminho budget — aumente o cap, NUNCA retente cego
set +e
out=$(docsrs-cli -q --max-body-bytes 50 readme serde --json)
code=$?
set -e
echo "$out" | jaq -r '.error.kind // empty'   # budget
echo "$out" | jaq -r '.error.retryable // false'  # false
docsrs-cli -q --max-body-bytes 10485760 readme serde --json
```

## Allowlist de Hosts e Segurança
### REQUIRED
- DEVE aceitar apenas hosts de produto crates.io, docs.rs, static.docs.rs e doc.rust-lang.org
- DEVE manter User-Agent identificável; DEVE usar o padrão embutido salvo override obrigatório
- DEVE respeitar rate-limit delay e polidez do produto
- DEVE usar rustls sem bypass de certificado
### FORBIDDEN
- NUNCA peça login scraping ou bypass de CAPTCHA
- NUNCA desabilite validação TLS
- NUNCA trate a CLI como crawler genérico multi-host
### Correct Pattern
```bash
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli --user-agent 'my-agent (+https://example.com/contact)' version --json
```

## Fórmulas Prontas
### REQUIRED
- DEVE copiar fórmulas abaixo e só substituir placeholders
- DEVE cobrir todos os 11 top-level commands nesta lista
- DEVE cobrir page-token, match, suggest, doctor --online, clamp de limit, expectativas de scrub, source_url dual, flags globais e todos os 13 nomes de schema
```bash
# superfície de fetch
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates --page-token <TOKEN> --json
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --suggest --json
docsrs-cli get-item <CRATE> method <TYPE::method> --json
docsrs-cli get-item <CRATE> attribute <hyphen-or-underscore-name> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type <KIND> --match prefix --limit <N> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match exact --json
docsrs-cli search-in-crate <CRATE> <QUERY> --match substring --json
docsrs-cli search-in-crate <CRATE> "" --limit <N> --json
docsrs-cli --dry-run search-crates <QUERY> --json
docsrs-cli --dry-run search-crates --page-token <TOKEN> --json
docsrs-cli --dry-run readme <CRATE> --json
docsrs-cli --dry-run get-item <CRATE> <KIND> <PATH> --json
docsrs-cli --dry-run search-in-crate <CRATE> <QUERY> --limit 5000 --json

# superfície meta
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# superfície completions (bruto por padrão)
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json

# superfície storage
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json

# knobs globais / sandbox
docsrs-cli --timeout 30 --connect-timeout 5 -q doctor --json
docsrs-cli --no-cache readme <CRATE> --json
docsrs-cli --disable-retry doctor --json
docsrs-cli --max-retries 3 --retry-base-ms 100 --retry-max-delay-ms 2000 doctor --json
docsrs-cli --lang pt-BR --format markdown doctor
docsrs-cli --no-color --format text version
docsrs-cli -v doctor --json
docsrs-cli --rate-limit-delay-ms 200 search-crates <QUERY> --json
docsrs-cli --max-concurrency 0 search-in-crate <CRATE> <QUERY> --json
docsrs-cli --cache-ttl-secs 86400 --max-cache-bytes 268435456 cache stats --json
docsrs-cli --max-body-bytes 10485760 --max-output-bytes 2097152 readme <CRATE> --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config path --json
DOCSRS_CLI_LANG=pt-BR docsrs-cli --format markdown doctor
```
### FORBIDDEN
- NUNCA invente subcomandos fora desta superfície
- NUNCA documente narrativa de changelog histórico dentro desta skill
