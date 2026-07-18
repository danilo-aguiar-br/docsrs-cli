[English](COOKBOOK.md)

# Cookbook
> Copie uma receita, rode o comando, leia o JSON.

## Nota de Latência
- Fetches frios de rede dependem da latência de crates.io e docs.rs
- Hits quentes de cache ficam locais dentro do TTL e reportam `cache_hit: true`
- Use `--dry-run` quando só precisar de URLs planejadas

## Referência de Valores Default
- Timeout wall-clock usa um orçamento seguro de produto via config
- TTL de cache default é 86400 segundos
- Orçamento soft de cache default é 256 MiB
- Max body default é teto duro de 10 MiB
- Max output default é teto duro de 2 MiB
- `search-in-crate --match` default é `prefix`
- `search-in-crate --limit` default é 100 e clamp em 1000
- JSON é escolhido automaticamente em stdout non-TTY (exceto `completions` bruto)

## Como Buscar um Crate
- Problema: achar crates por palavra-chave
```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
```

## Como Paginar Com page-token
- Problema: percorrer resultados do crates.io sem montar query strings à mão
```bash
docsrs-cli search-crates async --page 1 --per-page 20 --json
# leia data.meta.next_page em NEXT, depois:
docsrs-cli search-crates --page-token "$NEXT" --json
```

## Como Buscar a Visão Geral de um Crate
- Problema: ler o overview do docs.rs para um crate
```bash
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json
docsrs-cli readme tokio --crate-version latest --json
# resolved_version é o SemVer só do crate alvo, nunca de uma dependência
```

## Como Buscar Overview da Stdlib
- Problema: obter docs de std/core/alloc sem adivinhar HTML
```bash
docsrs-cli readme std --json
docsrs-cli readme core --json
docsrs-cli readme alloc --json
# resolved_version é o nome do canal como stable quando conhecido
```

## Como Observar o Clamp de Limit Offline
- Problema: provar o hard clamp de search-in-crate --limit sem rede
```bash
docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json
# planned_params.limit é 1000 (hard clamp, inclusive no dry-run)
```

## Como Buscar um Item Tipado
- Problema: puxar documentação de uma struct, trait ou função
```bash
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio struct runtime::Runtime --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item async-trait attribute async_trait --json
```

## Como Resolver Métodos Associados
- Problema: abrir docs de métodos como `Runtime::new` sem adivinhar HTML
```bash
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio fn runtime::Runtime::new --json
# página do tipo pai + #method.new; payload inclui item_name e resolved_version opcional
```

## Como Sugerir Símbolos Próximos em 404
- Problema: recuperar de um path de item errado sem raspar all.html você mesmo
```bash
docsrs-cli get-item serde struct Serde --suggest --json
docsrs-cli get-item tokio struct RuntimeX --suggest --json
```

## Como Buscar Símbolos Dentro de um Crate
- Problema: localizar símbolos sem navegar HTML
```bash
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```

## Como Escolher Modos de Match
- Problema: apertar ou afrouxar o ranking da busca de símbolos
```bash
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate serde Ser --match prefix --json
docsrs-cli search-in-crate serde de --match substring --limit 20 --json
# default é prefix; use substring para o comportamento legado de contains
```

## Como Detectar cache_hit
- Problema: saber se um payload veio do cache local em disco
```bash
docsrs-cli readme serde --json
docsrs-cli readme serde --json
# a segunda chamada costuma mostrar data.cache_hit true dentro do TTL
docsrs-cli --no-cache readme serde --json
# caminho forçado de rede reporta cache_hit false
docsrs-cli cache stats --json
```

## Como Descobrir a Superfície de Agente
- Problema: aprender comandos e formas de payload de forma programática
```bash
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
docsrs-cli version --json
docsrs-cli doctor --json
```

## Como Gerar Completions de Shell
- Problema: instalar scripts de completion do seu shell
```bash
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
# shell bruto por default mesmo em non-TTY; JSON só quando explícito:
docsrs-cli completions bash --json
```

## Como Trabalhar Offline ou Sem Efeitos de Rede
- Problema: planejar URLs sem abrir sockets
```bash
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
# planned_params usam crate_name (não crate)
```

## Como Gerir Cache e Config
- Problema: inspecionar saúde de storage e resetar estado local
```bash
docsrs-cli cache stats --json
docsrs-cli cache clear --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --json
```

## Como Auditar Prontidão Antes de um Lote
- Problema: falhar fechado antes de muitas rodadas de agente
```bash
docsrs-cli doctor --json
docsrs-cli doctor --online --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --online --json
```

## Como Cobrir Cada Top-Level Command Uma Vez
- Problema: smoke da superfície completa de comandos em um checklist
```bash
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli completions bash >/dev/null
docsrs-cli cache stats --json
docsrs-cli config path --json
```
