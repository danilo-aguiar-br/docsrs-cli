[English](CONFIGURATION.md)

# Configuração

## Por que este documento existe
- Todo knob abaixo é ajustável, e onze deles não estavam documentados em nenhum lugar que o usuário lê
- As flags de redução tinham um gate exigindo presença em todo documento de contrato; os knobs de transporte não tinham nenhum
- Knob que existe e nunca é ensinado é knob que ninguém usa

## Precedência
- Flags da CLI vencem o `config.toml`
- O `config.toml` vence os defaults compilados
- Knobs de produto **nunca** são lidos de variáveis de ambiente `DOCSRS_CLI_*`
- `RUST_LOG` não é lido: use `-q` / `-v` ou a chave `log_directive`
- `NO_COLOR`, `TERM` e `CLICOLOR_FORCE` descrevem o terminal, não o produto
- `HTTP_PROXY` / `HTTPS_PROXY` / `NO_PROXY` são honrados pelo transporte do `reqwest`, jamais como knob de produto

## Onde a configuração vive
- `docsrs-cli config path --json` imprime os diretórios resolvidos e a camada vencedora
- `docsrs-cli config show --json` imprime os valores efetivos após todas as camadas
- `docsrs-cli config init` grava um `config.toml` padrão; `--force` sobrescreve
- `config init --force` designa o alvo no argv: passe `--config-dir <DIR>`, ou `--yes` para aceitar a raiz XDG
- Sem um dos dois sai com exit 64 e não grava nada, nomeando o arquivo que se recusou a substituir
- O envelope carrega `target_source` (`cli` ou `xdg`), o mesmo campo que `cache clear` reporta
- A dispensa vale mesmo sem arquivo no destino: você não sabe disso sobre um diretório que nunca nomeou
- `--config-dir <DIR>` sobrescreve o diretório de configuração
- `--cache-dir <DIR>` sobrescreve o diretório de cache HTTP
- Chave desconhecida no `config.toml` falha fechado com exit 78; um typo nunca é ignorado em silêncio

## Formato de saída
- `--json` emite o envelope JSON no stdout
- `--format json|markdown|text` escolhe a renderização (`json` é alias de `--json`)
- JSON é automático quando o stdout não é TTY; force saída humana com `--format markdown`
- `--lang en|pt-BR` força o locale humano do stderr; o JSON permanece em inglês
- `--no-color` desliga ANSI no stderr
- `-v` / `--verbose` aumenta a verbosidade do stderr e é repetível
- `-q` / `--quiet` suprime prosa não essencial

## Redução de payload
- `--select CHAVES` projeta chaves pontilhadas (alias `--fields`); chave ausente é pulada, nunca vira null
- `--filter EXPR` mantém elementos com `key=value`, `key!=value`, `key~substring`; repita para AND
- `--filter` malformado falha com exit 65 em vez de devolver conjunto vazio
- `--sort-by CHAVE` ordena ascendente e estável; elementos sem a chave vão para o fim
- `--dedupe-by CHAVE` descarta elementos posteriores que repetem o valor; sem a chave, o elemento é mantido
- `--max-items N` emite no máximo N elementos, contados após `--filter` e `--dedupe-by`
- `--count-only` substitui o payload por `{"count": N}`
- `--truncate-content N` encurta strings acima de N caracteres, sem partir UTF-8
- `--max-output-bytes N` limita o payload emitido; máximo duro `2097152` (2 MiB)
- Ordem fixa: filter, sort-by, dedupe-by, max-items, select, count-only, truncate-content, max-output-bytes
- `--max-items` limita a emissão; `search-in-crate --limit` limita a consulta

## Timeouts
- `--timeout <SEGS>` é o orçamento de relógio; TOML `timeout_secs`
- `--connect-timeout <SEGS>` é o orçamento de conexão; TOML `connect_timeout_secs`
- Ambos devem ser no mínimo 1 quando definidos; `0` falha fechado, e o exit code nomeia a camada que o carregou
- Na flag é exit 65 `kind=invalid_input`; no `config.toml` é exit 78 `kind=config`
- A falha do arquivo acontece na carga, então um `timeout_secs = 0` herdado quebra todo comando, não só o que você rodou
- Os defaults são `timeout_secs` 30 e `connect_timeout_secs` 10

## Retry
- `--max-retries N` limita as retentativas após a primeira tentativa; TOML `max_retries`
- `--retry-base-ms N` é o atraso base de backoff; TOML `retry_base_ms`
- `--retry-max-delay-ms N` limita uma única espera de retry; TOML `retry_max_delay_ms`
- `--retry-max-elapsed-ms N` é o orçamento total de retry; `0` deriva do `--timeout`
- `--disable-retry` é o kill switch de incidente; TOML `disable_retry = true`
- Definir `max_retries = 0` desliga o retry pela camada de arquivo
- Só falhas idempotentes de GET retentam: 408, 429, 5xx e erros de transporte
- Nunca retente `kind=budget` (exit 74); essa falha é local e permanente para os mesmos ajustes

## Cortesia de rede
- `--rate-limit-delay-ms N` é o atraso mínimo entre requisições ao mesmo host
- TOML `rate_limit_delay_ms`
- O atraso carrega jitter aditivo e vale entre processos por lock e stamp

## Concorrência
- `--max-concurrency N` limita workers de parse na CPU; TOML `max_concurrency`
- `0` significa automático, derivado do número de CPUs e da RAM livre
- Isso limita o parse, não os sockets: os comandos de produto emitem um GET primário por vez

## Orçamentos de body e de saída
- `--max-body-bytes N` limita o body baixado; máximo duro `10485760` (10 MiB)
- `--max-output-bytes N` limita o payload emitido; máximo duro `2097152` (2 MiB)
- Valor acima do máximo duro falha fechado: exit 65 na flag, exit 78 `kind=config` no `config.toml`
- Esses dois tetos são os únicos que recusam; outros sete knobs são clampados em silêncio
- Clampados sem aviso: `max_redirects` para 20, `timeout_secs` para 600, `connect_timeout_secs` para 120
- Também clampados: `rate_limit_delay_ms` para 60000, `max_retries` para 10, e `retry_base_ms` elevado ao piso 50
- Por fim, `connect_timeout_secs` é reduzido a `timeout_secs` sempre que o excederia
- Releia qualquer um deles com `config show --json`, que reporta o valor que valeu, não o que você escreveu
- Demais defaults: `max_redirects` 5, `rate_limit_delay_ms` 1000, `max_retries` 3, `retry_base_ms` 200, `retry_max_delay_ms` 30000
- Estouro de body é `kind=budget`, exit 74, e não é retryable

## Cache em disco
- `--no-cache` desliga o cache e sempre vai à rede; TOML `no_cache`
- `--cache-ttl-secs N` define o TTL da entrada; padrão `86400` (24 h); TOML `cache_ttl_secs`
- `--max-cache-bytes N` é teto suave; padrão `268435456` (256 MiB); `0` significa ilimitado
- `docsrs-cli cache path --json` reporta `root`, `source` e `no_cache`
- `docsrs-cli cache stats --json` reporta entradas, bytes e o orçamento
- `docsrs-cli cache clear --yes --json` apaga bodies em cache e reporta o que foi liberado
- `cache clear` designa o alvo no argv: passe `--cache-dir <DIR>`, ou `--yes` para aceitar a raiz XDG
- Sem um dos dois sai com exit 64 e não apaga nada, nomeando o diretório que se recusou a esvaziar
- O envelope carrega `target_source` (`cli` ou `xdg`), então a auditoria vê qual camada escolheu o caminho
- Payloads de rede expõem `cache_hit`, que descreve apenas o disco local

## Identidade
- `--user-agent <STRING>` sobrescreve o User-Agent; TOML `user_agent`
- A chave TOML `contact` acrescenta um contato à identidade padrão
- `contact` deve ser ASCII imprimível não vazio, sem caracteres de controle e de comprimento limitado
- Valor de header inválido falha com `kind=config`

## Origens e loopback
- As chaves TOML `crates_io_origin` e `docs_rs_origin` sobrescrevem as origens permitidas
- Nenhuma das duas tem flag de CLI; ambas existem para bancadas de teste offline
- `--allow-loopback` permite `127.0.0.1` e `localhost`; TOML `allow_loopback = true`
- A allowlist de hosts continua valendo para toda URL alvo

## Diagnóstico
- A chave TOML `log_directive` define o filtro do `tracing`, por exemplo `docsrs_cli=debug,docsrs_cli::http=trace`
- `-q` e `-v` vencem a chave; o piso compilado é `error`
- Diretiva não parseável é recusada na carga com exit 78
- `config show --json` ecoa `log_directive` quando definida, e a omite quando não está

## Planejar sem sockets
- `--dry-run` planeja URLs e não abre socket de rede
- O payload carrega `planned_url` e `planned_params`; `validation` vive DENTRO de `planned_params`, nunca ao lado
- URL planejada é apenas forma de URL; nunca é prova de que a âncora existe ao vivo

## Chaves completas do `config.toml`
- `timeout_secs`, `connect_timeout_secs`
- `max_body_bytes`, `max_output_bytes`, `max_redirects`
- `max_retries`, `retry_base_ms`, `retry_max_delay_ms`, `retry_max_elapsed_ms`, `disable_retry`
- `rate_limit_delay_ms`, `max_concurrency`
- `user_agent`, `contact`, `lang`, `log_directive`
- `crates_io_origin`, `docs_rs_origin`, `allow_loopback`
- `cache_ttl_secs`, `max_cache_bytes`, `no_cache`
- Cinco chaves não têm flag de CLI e só são ajustáveis pela camada de arquivo
- São `max_redirects`, `contact`, `log_directive`, `crates_io_origin` e `docs_rs_origin`

## Verifique seus ajustes
```bash
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli doctor --json
docsrs-cli --config-dir /tmp/rig config show --json
```

## Veja também
- [Como usar](HOW_TO_USE.pt-BR.md)
- [Agentes](AGENTS.pt-BR.md)
- [Cookbook](COOKBOOK.pt-BR.md)
- [Schemas JSON](schemas/README.md)
