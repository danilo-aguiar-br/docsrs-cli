[English](COOKBOOK.md)

# Cookbook

> Copie a receita, rode o comando, leia o JSON.


## Nota de Latência
- Fetches frios de rede dependem da latência do crates.io e docs.rs
- Hits quentes de cache ficam locais dentro do TTL
- Use `--dry-run` quando só precisar de URLs planejadas


## Referência de Valores Default
- Timeout wall-clock usa orçamento seguro de produto via config
- TTL de cache default é 86400 segundos
- Orçamento soft de cache default é 256 MiB
- Max body default é 10 MiB com hard ceiling
- Max output default é 2 MiB com hard ceiling
- JSON é escolhido automaticamente em stdout non-TTY


## Como Buscar um Crate
- Problema: achar crates por palavra-chave
```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
```


## Como Buscar a Visão Geral de um Crate
- Problema: ler o overview do docs.rs de um crate
```bash
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json
```


## Como Buscar um Item Tipado
- Problema: puxar documentação de uma struct, trait ou função
```bash
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio struct runtime::Runtime --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item async-trait attribute async_trait --json
```


## Como Buscar Símbolos Dentro de um Crate
- Problema: localizar símbolos sem navegar HTML
```bash
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type function --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```


## Como Descobrir a Superfície de Agente
- Problema: aprender comandos e formatos de payload de forma programática
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
docsrs-cli version --json
docsrs-cli doctor --json
```


## Como Gerar Completions de Shell
- Problema: instalar scripts de completion para o seu shell
```bash
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
docsrs-cli completions bash --json
```


## Como Trabalhar Offline ou Sem Efeitos de Rede
- Problema: planejar URLs sem abrir sockets
```bash
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
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
- Problema: falhar fechado antes de muitos turnos de agente
```bash
docsrs-cli doctor --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli config init --json
DOCSRS_CLI_HOME=/tmp/docsrs-audit docsrs-cli doctor --json
```


## Como Cobrir Cada Top-Level Command Uma Vez
- Problema: smoke da superfície completa em um checklist
```bash
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli search-in-crate serde Serialize --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli completions bash >/dev/null
docsrs-cli cache stats --json
docsrs-cli config path --json
```
