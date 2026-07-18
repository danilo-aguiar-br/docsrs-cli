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
- Confirme que `data.version` de `docsrs-cli version --json` é `1.1.0` na linha 1.1

## Feature 1.1 → Guia
| Feature 1.1 | Seção do guia |
|-------------|---------------|
| `--match exact\|prefix\|substring` (default `prefix`) | Modos de Match |
| `search-crates --page-token` | Paginação Com page-token |
| `data.cache_hit` | Conceito de cache_hit |
| `item_name` / `resolved_version` / `match_mode` | Campos JSON Que Agentes Devem Ler |
| Alias `method` em `get-item` + `#method.name` | Comandos Centrais |
| `get-item --suggest` | Comandos Centrais |
| `doctor --online` | Outros Subcomandos |
| Dry-run `planned_params.crate_name` | Padrões Avançados |
| Completions shell bruto (JSON só com `--json`) | Padrões Avançados |
| Knobs de produto: só flags + TOML (sem `DOCSRS_CLI_*`) | Configuração |
| Schemas `schema` / `completions` / `error` / `dry-run` | Superfície Completa de Comandos |
| Upgrade de 0.1.x | [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md) |

## Comandos Centrais
- Busque no registry: `docsrs-cli search-crates tokio --json`
- Paginação e sort: `docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json`
- Continue com page token: `docsrs-cli search-crates --page-token "$NEXT" --json`
- Busque visão geral do crate: `docsrs-cli readme tokio --json`
- Fixe versão do overview: `docsrs-cli readme clap --crate-version 4.5.0 --json`
- Resolva o SemVer latest só do crate alvo: `docsrs-cli readme tokio --crate-version latest --json`
- Busque overview da stdlib (canal em `resolved_version`): `docsrs-cli readme std --json`
- Busque item tipado: `docsrs-cli get-item clap trait clap::Parser --json`
- Busque método associado: `docsrs-cli get-item tokio method runtime::Runtime::new --json`
- Sugira símbolos próximos em miss: `docsrs-cli get-item serde struct Serde --suggest --json`
- Busque símbolos em um crate: `docsrs-cli search-in-crate reqwest Client --json`
- Escolha o match mode: `docsrs-cli search-in-crate serde Serialize --match exact --json`
- Liste símbolos com query vazia: `docsrs-cli search-in-crate tokio "" --limit 50 --json`
- Descubra a árvore: `docsrs-cli commands --json`
- Imprima schema de payload: `docsrs-cli schema --cmd get-item --json`

## Superfície Completa de Comandos
- `search-crates` com `--page`, `--per-page`, `--sort`, `--page-token`
- `readme` com `--crate-version` opcional
- `get-item` com `--crate-version` e `--suggest` opcionais
- `search-in-crate` com `--crate-version`, `--item-type`, `--limit`, `--match` opcionais
- `version`, `doctor`, `doctor --online`, `commands`
- `schema --cmd` para search-crates, readme, get-item, search-in-crate, version, doctor, commands, cache, config, schema, completions, error, dry-run
- `completions` para bash, zsh, fish, elvish, power-shell, powershell (shell bruto por default; `--json` só quando explícito)
- `cache stats` e `cache clear`
- `config path`, `config show`, `config init`, `config init --force`

## Daemon
- docsrs-cli não tem daemon
- Toda invocação é BORN, EXECUTE, FINALIZE, DIE
- Não espere sessões sticky nem workers em background

## Modos de Match
- O default de `search-in-crate` é `--match prefix`
- `exact` mantém só matches de folha exata (ou path completo exato)
- `prefix` ranqueia folha exata primeiro, depois prefixos da folha
- `substring` restaura o comportamento legado de contains no path
- Hits podem incluir `score`; scores menores ranqueiam melhor quando presentes
- Exemplo exact: `docsrs-cli search-in-crate serde Serialize --match exact --json`
- Exemplo substring: `docsrs-cli search-in-crate serde de --match substring --limit 20 --json`

## Paginação Com page-token
- Primeira página: `docsrs-cli search-crates async --page 1 --per-page 20 --json`
- Leia `data.meta.next_page` quando presente
- Próxima página: `docsrs-cli search-crates --page-token "$NEXT" --json`
- `--page` e `--page-token` conflitam; escolha um por invocação
- Tokens são query strings opacas da resposta anterior; não os invente à mão

## Conceito de cache_hit
- Payloads de comandos de rede incluem `data.cache_hit`
- `true` significa que o body HTTP veio do cache local em disco dentro do TTL
- `false` significa que um fetch de rede preencheu (ou contornou) o cache
- Use `--no-cache` para forçar rede; use `cache stats` / `cache clear` para inspecionar ou resetar

## Campos JSON Que Agentes Devem Ler
- `crate_name` é o campo canônico do crate no wire
- `get-item` sempre emite `item_name`
- `readme` e `get-item` podem emitir `resolved_version` opcional
- Na stdlib, `resolved_version` é o nome do canal como `stable` (exemplo: `docsrs-cli readme std --json`)
- Em crates, `resolved_version` é o SemVer somente do crate alvo (nunca a versão de uma dependência raspada da página)
- `search-in-crate` sempre ecoa `match_mode`
- Hits ranqueados podem incluir `score` quando há query
- Nomes de campos JSON e mensagens técnicas permanecem em inglês mesmo com stderr localizado

## Padrões Avançados
- Planeje sem rede: `docsrs-cli --dry-run get-item tokio struct runtime::Runtime --json`
- Dry-run de busca: `docsrs-cli --dry-run search-crates serde --json`
- Params planejados do dry-run usam `crate_name` (não `crate`)
- Dry-run limita `search-in-crate --limit` a 1000: `docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json`
- Force Markdown humano em pipe: `docsrs-cli --format markdown version`
- Isole storage: `DOCSRS_CLI_HOME=/tmp/docsrs-sandbox docsrs-cli doctor --json`
- Prontidão online: `docsrs-cli doctor --online --json`
- Inspecione cache: `docsrs-cli cache stats --json`
- Limpe cache: `docsrs-cli cache clear --json`
- Crie config padrão: `docsrs-cli config init --json`
- Sobrescreva config: `docsrs-cli config init --force --json`
- Gere completions (script bruto): `docsrs-cli completions bash`
- Completions como JSON só quando pedido: `docsrs-cli completions bash --json`
- Outros shells: `docsrs-cli completions zsh`, `completions fish`, `completions elvish`, `completions power-shell`, `completions powershell`

## Configuração
- Precedência de settings de produto: flags CLI > `config.toml` XDG > defaults embutidos
- Knobs de produto não são lidos de variáveis de ambiente `DOCSRS_CLI_*`
- Path sandbox ainda usa `DOCSRS_CLI_HOME`, `DOCSRS_CLI_CONFIG_DIR` e `DOCSRS_CLI_CACHE_DIR`
- Mostre config efetiva: `docsrs-cli config show --json`
- Imprima paths resolvidos: `docsrs-cli config path --json`
- User-Agent default é `docsrs-cli/1.1.0 (+https://github.com/danilo-aguiar-br/docsrs-cli)`
- Sobrescreva User-Agent com `--user-agent` ou TOML `user_agent`
- Contact do UA default vem de TOML `contact` (sem `DOCSRS_CLI_USER_AGENT` / `DOCSRS_CLI_CONTACT`)
- Origins para mocks/testes: TOML `crates_io_origin` / `docs_rs_origin` sob um home sandbox

## Outros Subcomandos
- `version` imprime a identidade do binário (`1.1.0` nesta linha)
- `doctor` valida TLS, paths, concorrência, contact e política de retry
- `doctor --online` adiciona sondas de rede opt-in a crates.io e docs.rs
- `completions <shell>` emite scripts de completion brutos por default
- `config path|show|init` gerencia config XDG sem segredos
- `cache stats|clear` gerencia o cache HTTP em disco

## Integração Com Agentes de IA
- Prefira sempre `--json` para consumidores máquina
- Parseie o exit code antes de ler o stdout
- JSON no stdout permanece em inglês; stderr humano pode seguir `--lang` ou `DOCSRS_CLI_LANG`
- Leia [AGENTS.pt-BR.md](AGENTS.pt-BR.md) e as skills empacotadas [docsrs-cli-en](../skills/docsrs-cli-en/SKILL.md) / [docsrs-cli-pt](../skills/docsrs-cli-pt/SKILL.md)
- Leia schemas máquina em [schemas/README.md](schemas/README.md)
- Leia [MIGRATION.pt-BR.md](MIGRATION.pt-BR.md) ao atualizar de 0.1.x
