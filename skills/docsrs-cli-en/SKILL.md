---
name: docsrs-cli
description: This skill MUST activate for docsrs-cli, crates.io search, docs.rs fetch, rustdoc item lookup, get-item, readme, search-crates, search-in-crate, version, doctor, commands, schema, completions, cache, config, agent JSON envelopes, dry-run URL planning, and XDG cache or config work. It MUST teach the full command catalog, exact argv, stdout JSON contracts, exit codes, retry policy, host allowlist, and one-shot BORN-EXECUTE-FINALIZE-DIE lifecycle so agents fetch Rust crate docs without scraping HTML by hand.
---

# docsrs-cli

## Identity and Lifecycle
### REQUIRED
- DEVE tratar o binário como sempre `docsrs-cli`
- DEVE tratar cada processo como BORN, EXECUTE, FINALIZE, DIE
- DEVE preferir `--json` para todo consumidor programático
- DEVE tratar stdout como contrato de dados e stderr como diagnóstico
- DEVE esperar JSON automático quando stdout não é TTY
- DEVE forçar humano com `--format markdown` ou `--format text`
- DEVE descobrir a árvore viva com `commands --json` quando em dúvida
### FORBIDDEN
- NUNCA assuma daemon, sessão sticky ou telemetria de produto
- NUNCA parseie stderr como JSON de sucesso
- NUNCA reutilize estado de processo entre invocações
- NUNCA invente subcomando fora do catálogo abaixo
### Correct Pattern
```bash
docsrs-cli --version
docsrs-cli doctor --json
docsrs-cli commands --json
```

## Full Command Catalog
### REQUIRED
- DEVE conhecer os 11 top-level commands
- DEVE conhecer subcomandos `cache clear`, `cache stats`
- DEVE conhecer subcomandos `config path`, `config show`, `config init`
- DEVE conhecer shells de `completions`
- DEVE conhecer flags de produto por comando listadas abaixo
### FORBIDDEN
- NUNCA omita um top-level command do catálogo operacional
- NUNCA use `schema --cmd schema` ou `schema --cmd completions` (sem schema de payload)
### Correct Pattern
```bash
# 1) search-crates
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli search-crates <QUERY> --sort alphabetical --json
# --sort values: relevance|downloads|recent-downloads|recent-updates|new|alphabetical

# 2) readme
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json

# 3) get-item
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
# KIND: module|struct|trait|enum|union|fn|function|type|const|constant|static|macro|attr|attribute|derive
# PATH: uses :: or / ; optional leading crate prefix

# 4) search-in-crate
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> "" --limit 50 --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type function --json
docsrs-cli search-in-crate <CRATE> <QUERY> --crate-version <VERSION> --limit 100 --json

# 5) version
docsrs-cli version --json
docsrs-cli --format markdown version

# 6) doctor
docsrs-cli doctor --json

# 7) commands
docsrs-cli commands --json

# 8) schema  (payload schemas for data-bearing commands)
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# 9) completions
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

## Search and Fetch Commands
### REQUIRED
- DEVE usar `search-crates` para busca no crates.io
- DEVE usar `readme` para overview do docs.rs (não git README)
- DEVE usar `get-item` para item rustdoc tipado
- DEVE usar `search-in-crate` para símbolos no `all.html`
- DEVE aceitar `item_path` com `::` ou `/`
- DEVE tratar `std`, `core` e `alloc` via doc.rust-lang.org
- DEVE passar `--crate-version` quando a versão concreta for exigida
### FORBIDDEN
- NUNCA scrape HTML de docs.rs com regex quando a CLI resolve o item
- NUNCA invente kinds fora do conjunto suportado
- NUNCA omita `--json` em pipelines de agente
### Correct Pattern
```bash
docsrs-cli search-crates serde --page 1 --per-page 20 --sort downloads --json
docsrs-cli readme tokio --crate-version 1.40.0 --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item tokio struct runtime/Runtime --crate-version latest --json
docsrs-cli search-in-crate reqwest Client --item-type struct --limit 20 --json
docsrs-cli search-in-crate std Option --json
```

## Discovery Doctor Version Schema Completions
### REQUIRED
- DEVE rodar `commands --json` antes de inventar subcomandos
- DEVE rodar `schema --cmd <name> --json` para payloads com schema
- DEVE rodar `doctor --json` quando paths, TLS ou retry parecerem errados
- DEVE rodar `version --json` para identidade do binário
- DEVE gerar completions só com shells suportados
- DEVE validar `ok == true` antes de ler `data`
### FORBIDDEN
- NUNCA invente flags fora de `commands` e `--help`
- NUNCA ignore `schema_version`
- NUNCA peça schema para `schema` ou `completions`
### Correct Pattern
```bash
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli completions zsh
docsrs-cli completions power-shell
```

## Dry-Run Cache and Config
### REQUIRED
- DEVE usar `--dry-run` para planejar URLs sem rede
- DEVE usar `cache stats --json` e `cache clear --json`
- DEVE usar `config path|show|init --json`
- DEVE usar `config init --force` só para sobrescrever `config.toml` existente
- DEVE isolar storage com `DOCSRS_CLI_HOME` em sandboxes
### FORBIDDEN
- NUNCA espere `.env` em runtime de produto
- NUNCA armazene API keys no produto
- NUNCA use `config init --force` sem intenção de sobrescrever
### Correct Pattern
```bash
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-crates serde --json
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli config init --force --json
```

## JSON Contract
### REQUIRED
- DEVE esperar sucesso com `schema_version`, `ok`, `command`, `data`, `duration_ms`
- DEVE esperar falha com `ok:false` e `error.kind` no caminho JSON
- DEVE ler `source_url` como proveniência quando presente
- DEVE tratar `truncated:true` em `search-in-crate` como corte por `--limit`
### FORBIDDEN
- NUNCA confie em `data` quando `ok` é false
- NUNCA misture NDJSON e JSON de envelope no mesmo parse
### Correct Pattern
```bash
out=$(docsrs-cli -q get-item serde trait Serialize --json) || code=$?
echo "$out" | jaq -e '.ok == true'
echo "$out" | jaq -r '.data.source_url'
```

## Exit Codes and Retry
### REQUIRED
- DEVE ramificar no exit code antes do stdout
- DEVE tratar `0` como sucesso
- DEVE tratar `2` como argv clap inválido
- DEVE tratar `64` usage, `65` invalid input/parse, `66` not found
- DEVE tratar `69` rate limit/unavailable, `74` network, `124` timeout como retryable
- DEVE tratar `78` config, `70` internal, `130` SIGINT, `141` broken pipe, `143` SIGTERM
- DEVE retentar só `69`, `74` e `124` com backoff
- DEVE honrar kill switch `--disable-retry` apenas em incidente
### FORBIDDEN
- NUNCA retente `64`, `65`, `66` ou `78` sem mudar inputs
- NUNCA mascare exit codes com `|| true` em pipelines de agente
### Correct Pattern
```bash
set +e
docsrs-cli -q get-item missing-crate-xyz struct Foo --json
code=$?
set -e
case "$code" in
  0) echo ok ;;
  66) echo not_found ;;
  69|74|124) echo retryable ;;
  *) echo fail_$code ;;
esac
```

## Host Allowlist and Safety
### REQUIRED
- DEVE aceitar apenas hosts de produto crates.io, docs.rs, static.docs.rs e doc.rust-lang.org
- DEVE manter User-Agent identificável
- DEVE respeitar rate-limit delay e politeness do produto
- DEVE usar rustls sem bypass de certificado
### FORBIDDEN
- NUNCA peça login scraping ou bypass de CAPTCHA
- NUNCA desabilite validação TLS
- NUNCA trate a CLI como crawler genérico multi-host
### Correct Pattern
```bash
docsrs-cli doctor --json
docsrs-cli --user-agent 'docsrs-cli/0.1.0 (+https://github.com/docsrs-cli/docsrs-cli)' version --json
```

## Ready Formulas
### REQUIRED
- DEVE copiar fórmulas abaixo e só substituir placeholders
- DEVE cobrir todos os 11 top-level commands nesta lista
```bash
# fetch surface
docsrs-cli search-crates <QUERY> --json
docsrs-cli search-crates <QUERY> --page 1 --per-page 20 --sort downloads --json
docsrs-cli readme <CRATE> --json
docsrs-cli readme <CRATE> --crate-version <VERSION> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --json
docsrs-cli get-item <CRATE> <KIND> <PATH> --crate-version <VERSION> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --json
docsrs-cli search-in-crate <CRATE> <QUERY> --item-type <KIND> --limit <N> --json
docsrs-cli search-in-crate <CRATE> "" --limit <N> --json
docsrs-cli --dry-run search-crates <QUERY> --json
docsrs-cli --dry-run readme <CRATE> --json
docsrs-cli --dry-run get-item <CRATE> <KIND> <PATH> --json
docsrs-cli --dry-run search-in-crate <CRATE> <QUERY> --json

# meta surface
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli commands --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd config --json

# completions surface
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell

# storage surface
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json
```
### FORBIDDEN
- NUNCA invente subcomandos fora desta superfície
- NUNCA documente changelog histórico dentro desta skill
