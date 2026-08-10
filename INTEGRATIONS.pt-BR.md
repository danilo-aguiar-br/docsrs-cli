[English](INTEGRATIONS.md)

# Integrações
> Um binário cobre agentes, shells e CI sem servidor sticky.


## Instantâneo de Cobertura
- Integração one-shot por subprocesso para qualquer agente que execute um binário
- JSON automático em stdout non-TTY para pipes e orquestradores
- Completions de shell para bash, zsh, fish, elvish e PowerShell
- Dry-run offline para planejar URLs sem sockets
- Probes online do doctor para crates.io e docs.rs quando opt-in


## Superfície de Comandos
- Comandos de dados: `search-crates`, `readme`, `get-item`, `search-in-crate`
- Comandos de descoberta: `version`, `doctor`, `commands`, `schema`, `completions`
- Comandos de armazenamento: `cache path`, `cache stats`, `cache clear`
- Comandos de configuração: `config path`, `config show`, `config init`
- Onze comandos de topo, dezessete caminhos invocáveis contando subcomandos
- Toda flag e toda chave do `config.toml`: [Configuração](docs/CONFIGURATION.pt-BR.md)


## Endurecimento de Contrato em 1.3.0
- `--sort-by` e `--max-items` completam o pipeline de redução; a ordenação roda antes do limite
- `agent_surface` expõe `limited`, então o chamador distingue resultado pequeno de resultado cortado
- `schema --cmd agent-surface` publica o contrato do relatório de redução
- `error.suggestions` carrega o ranking de `--suggest` como dado; nenhum agente parseia `error.message`
- `anchor_family` nomeia a família real do rustdoc por trás de `extraction=method`
- `get-item` alcança `variant` e `structfield`, além de itens associados de trait e métodos requeridos
- Chave `log_directive` no `config.toml`; valor não parseável falha na carga com exit 78
- `RUST_LOG` não é lido; ele vencia a CLI, o que é knob de produto em ambiente
- As âncoras de confiança TLS voltaram ao `webpki-roots` embutido, depois que um upgrade do `reqwest` as moveu para o repositório do sistema


## Flags Adicionadas em 1.1.x (retidas)
- `--match exact|prefix|substring` em `search-in-crate` (padrão `prefix`)
- `--page-token` em `search-crates` para paginação opaca de `meta.next_page`
- `--suggest` em `get-item` para listar símbolos próximos após 404
- `doctor --online` para probes DNS opt-in
- Payloads de rede expõem `data.cache_hit`, `crate_name` canônico e `score` ranqueado
- Métodos associados resolvem para a página do tipo pai com `#method.name` e `item_name`
- Knobs de produto usam flags CLI e XDG `config.toml` apenas, não env `DOCSRS_CLI_*` de produto
- `resolved_version` opcional em readme e get-item (canal da stdlib é `stable`)

## Endurecimento de Contrato em 1.2.0 (Camada Y)
- Method com `#method.X` ausente é `not_found` (exit 66), nunca sucesso falso com página pai
- Sucesso de method define `data.extraction` apenas como `method`
- Agentes DEVEM rejeitar sucesso de method se `extraction` estiver ausente ou for o legado `item_page`
- `--suggest` em 404 de method ranqueia leaves de método na página do tipo pai
- Envelopes de erro incluem `command` e `duration_ms` no topo (paridade com sucesso)
- Valores de budget acima do hard max falham fechados com exit 65 (sem clamp silencioso)
- Corpo acima de `--max-body-bytes` configurado (dentro do hard max) é `error.kind=budget` (exit 74, `retryable=false`)
- URL 404 de method mantém o primeiro kind de probe (`struct`), não o último
- Dry-run reporta `validation=url_shape_only` e probes de parent kind para methods
- `docs/schemas` offline bate com `schema --cmd all` (19 nomes wire incluindo aliases naquele release; o `agent-surface` chegou em 1.3.0)

## Endurecimento retido de 1.1.x
- `--page-token` ecoa `query` / `page` / `per_page` / `sort` efetivos da URL planejada
- `doctor` top-level `ok` espelha `data.ok` (exit 78 quando unhealthy)
- `--suggest` ranqueia exact → prefix → substring → edit-distance
- `--timeout 0` / `--connect-timeout 0` explícitos fail-closed (exit 65)
- Script humano de smoke: `scripts/smoke-live.sh`


## Flags de Redução de Payload
- Oito flags globais cortam o envelope JSON antes da escrita, dispensando etapa jq
- `--select CHAVES` projeta chaves pontilhadas (alias `--fields`); chave ausente é pulada, nunca null
- `--filter EXPR` mantém elementos com `chave=valor`, `chave!=valor`, `chave~substring`; repita para AND
- Um `--filter` malformado falha fechado com exit 65, nunca devolve conjunto vazio
- `--sort-by CHAVE` ordena ascendente e de forma estável; elementos sem a chave vão para o fim
- `--dedupe-by CHAVE` descarta elementos posteriores que repetem o valor
- `--max-items N` limita a EMISSÃO; `search-in-crate --limit` limita a consulta
- `--count-only` substitui o payload por `{"count": N}`
- `--truncate-content N` encurta strings acima de N caracteres, sem partir UTF-8
- `--max-output-bytes N` limita o payload emitido; máximo duro 2097152 (2 MiB)
- Ordem fixa: filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes

## Aliases de Flags
- `--json` força o envelope JSON
- `--format json` é alias de `--json`
- `--format markdown` e `--format text` forçam o caminho humano
- Completions aceitam `powershell` e `power-shell`
- Kind `method` é alias de `fn` para métodos associados


## Contrato de Completions
- `completions <shell>` sempre emite script cru por padrão
- JSON para completions exige `--json` explícito
- Esta é uma exceção intencional ao auto-JSON em non-TTY


## Tabela Resumo
| Superfície | Estilo de integração | Contrato primário |
|------------|----------------------|-------------------|
| Claude Code / agentes LLM genéricos | subprocesso + `--json` | envelope JSON no stdout |
| Codex / Cursor / OpenCode | subprocesso + pipe | auto-JSON non-TTY |
| Humanos em shell | Markdown padrão em TTY | diagnósticos no stderr |
| CI / scripts | pipe non-TTY | exit codes + JSON |
| Completions | `completions <shell>` | scripts de shell crus |


## Claude Code e Agentes Genéricos
- Invoque como subprocesso one-shot por operação
- Passe `--json` ou confie no auto-JSON non-TTY
- Parseie `ok`, `command`, `data` e `duration_ms` em sucesso
- Parseie `ok:false`, `command`, `duration_ms`, `error.kind` e `error.retryable` em falha
- Nunca retente `kind=budget` (exit 74); aumente `--max-body-bytes` só dentro do hard max (acima do hard max é exit 65)
- Leia `data.cache_hit`, `data.crate_name`, `data.item_name`, `data.match_mode`; sucesso de method exige `data.extraction=method`
- Trate doctor como saudável só quando top-level `ok` e `data.ok` forem ambos true
- Comece com `commands --json` e `schema --cmd <name> --json`
- Prefira `--match prefix` ou `exact` para lookup preciso de símbolos
- Paginate com `data.meta.next_page` em `--page-token` e confie no eco da URL efetiva
- Recupere 404s com `get-item ... --suggest` (match em cascata na mensagem de erro)


## Codex Cursor e OpenCode
- Mantenha o binário no PATH após `cargo install docsrs-cli --locked`
- Prefira modo quiet com `-q` quando stderr precisa ficar limpo
- Ramifique no exit code antes de confiar no stdout
- Use `--dry-run` para validar URLs planejadas em sandboxes
- Use `doctor --online --json` antes de batches online grandes


## Humanos em Shell
- Saída padrão em TTY é Markdown
- Use `--format markdown` para forçar saída humana em pipes
- Gere completions com `docsrs-cli completions bash`
- Rode `doctor --json` após mudar paths XDG
- Rode `doctor --online --json` quando a prontidão de rede importar


## CI e Scripts
- Sempre passe `--json` para parse estável
- Trate exit `0` como sucesso e non-zero como classes de falha
- Use `--config-dir` / `--cache-dir` para config e cache isolados
- Knobs de produto vêm de flags ou XDG `config.toml`, não de env de produto
- Não habilite testes live de rede a menos que seja intencional


## Pacotes de Skill
- Skill em inglês: `skills/docsrs-cli-en/SKILL.md`
- Skill em português: `skills/docsrs-cli-pt/SKILL.md`
- Skills ensinam argv exato, envelopes, match modes, page tokens e política de retry
