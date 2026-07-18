[English](INTEGRATIONS.md)

# Integrações

> Um binário cobre agentes, shells e CI sem servidor sticky.


## Instantâneo de Cobertura
- Integração one-shot por subprocesso para qualquer agente que execute um binário
- JSON automático em stdout non-TTY para pipes e orquestradores
- Completions de shell para bash, zsh, fish, elvish e PowerShell
- Dry-run offline para planejar URLs sem sockets


## Aliases de Flags
- `--json` força o envelope JSON
- `--format json` é alias de `--json`
- `--format markdown` e `--format text` forçam o caminho humano
- Completions aceitam `powershell` e `power-shell`


## Tabela Resumo

| Superfície | Estilo de integração | Contrato primário |
|------------|----------------------|-------------------|
| Claude Code / agentes LLM genéricos | subprocesso + `--json` | envelope JSON no stdout |
| Codex / Cursor / OpenCode | subprocesso + pipe | auto-JSON non-TTY |
| Humanos em shell | Markdown default em TTY | diagnósticos no stderr |
| CI / scripts | pipe non-TTY | exit codes + JSON |
| Completions | `completions <shell>` | scripts de shell |


## Claude Code e Agentes Genéricos
- Invoque como subprocesso one-shot por operação
- Passe `--json` ou confie no auto-JSON non-TTY
- Parseie `ok`, `command`, `data` e `duration_ms` no sucesso
- Parseie `ok:false` e `error.kind` na falha
- Comece com `commands --json` e `schema --cmd <name> --json`


## Codex Cursor e OpenCode
- Mantenha o binário no PATH após `cargo install docsrs-cli --locked`
- Prefira modo quiet com `-q` quando stderr precisa ficar limpo
- Ramifique por exit codes antes de confiar no stdout
- Use `--dry-run` para validar URLs planejadas em sandboxes


## Humanos em Shell
- Saída default em TTY é Markdown
- Use `--format markdown` para forçar saída humana em pipes
- Gere completions com `docsrs-cli completions bash`
- Rode `doctor --json` após mudar paths XDG


## CI e Scripts
- Sempre passe `--json` para parsing estável
- Trate exit `0` como sucesso e non-zero como classes de falha
- Defina `DOCSRS_CLI_HOME` para config e cache isolados
- Não habilite testes live de rede sem intenção


## Pacotes de Skill
- Skill em inglês: `skills/docsrs-cli-en/SKILL.md`
- Skill em português: `skills/docsrs-cli-pt/SKILL.md`
- Skills ensinam argv exato, envelopes e política de retry
