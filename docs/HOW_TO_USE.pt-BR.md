[English](HOW_TO_USE.md)

# Como Usar o docsrs-cli

> Vá da instalação a um fetch real de docs em menos de 60 segundos.


## Pré-requisitos
- Instale Rust 1.88 ou mais novo com rustup
- Garanta HTTPS de saída para crates.io e docs.rs
- Prefira um terminal com PATH funcional após o cargo install


## Primeiro Comando em 60 Segundos
```bash
cargo install docsrs-cli --locked
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
```
- Confirme exit code 0 após cada comando
- Confirme que stdout é um objeto JSON com `"ok":true`


## Comandos Centrais
- Busque no registry: `docsrs-cli search-crates tokio --json`
- Paginação e sort: `docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json`
- Busque visão geral do crate: `docsrs-cli readme tokio --json`
- Fixe versão do overview: `docsrs-cli readme clap --crate-version 4.5.0 --json`
- Busque item tipado: `docsrs-cli get-item clap trait clap::Parser --json`
- Busque símbolos em um crate: `docsrs-cli search-in-crate reqwest Client --json`
- Liste símbolos com query vazia: `docsrs-cli search-in-crate tokio "" --limit 50 --json`
- Descubra a árvore: `docsrs-cli commands --json`
- Imprima schema de payload: `docsrs-cli schema --cmd get-item --json`


## Superfície Completa de Comandos
- `search-crates` com `--page`, `--per-page`, `--sort`
- `readme` com `--crate-version` opcional
- `get-item` com `--crate-version` opcional
- `search-in-crate` com `--crate-version`, `--item-type`, `--limit` opcionais
- `version`, `doctor`, `commands`
- `schema --cmd` para search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config
- `completions` para bash, zsh, fish, elvish, power-shell, powershell
- `cache stats` e `cache clear`
- `config path`, `config show`, `config init`, `config init --force`


## Daemon
- docsrs-cli não tem daemon
- Toda invocação é BORN, EXECUTE, FINALIZE, DIE
- Não espere sessões sticky nem workers em background


## Padrões Avançados
- Planeje sem rede: `docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json`
- Dry-run de busca: `docsrs-cli --dry-run search-crates serde --json`
- Force Markdown humano em pipe: `docsrs-cli --format markdown version`
- Isole storage: `DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli doctor --json`
- Inspecione cache: `docsrs-cli cache stats --json`
- Limpe cache: `docsrs-cli cache clear --json`
- Crie config padrão: `docsrs-cli config init --json`
- Sobrescreva config: `docsrs-cli config init --force --json`
- Gere completions: `docsrs-cli completions bash`
- Outros shells: `docsrs-cli completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`


## Configuração
- Prefira flags, depois allowlist de env, depois `config.toml` XDG, depois defaults
- Mostre config efetiva: `docsrs-cli config show --json`
- Imprima paths resolvidos: `docsrs-cli config path --json`
- Contact e User-Agent vêm de `--user-agent`, `DOCSRS_CLI_USER_AGENT` ou `DOCSRS_CLI_CONTACT`


## Outros Subcomandos
- `version` imprime a identidade do binário
- `doctor` valida TLS, paths, concorrência e política de retry
- `completions <shell>` emite scripts de completion
- `config path|show|init` gerencia config XDG sem segredos
- `cache stats|clear` gerencia o cache HTTP em disco


## Integração Com Agentes de IA
- Prefira sempre `--json` para consumidores máquina
- Parseie o exit code antes de ler o stdout
- Leia [AGENTS.pt-BR.md](AGENTS.pt-BR.md) e as skills empacotadas em `skills/`
