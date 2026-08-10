[English](0001-http-retry-policy.md)

# ADR 0001 — Política de retry HTTP do docsrs-cli
## Status
- Aceito (2026-07-18)

## Contexto
- A CLI emite GETs HTTPS one-shot contra crates.io, docs.rs, static.docs.rs e doc.rust-lang.org
- CDNs e APIs públicas retornam `429` / `5xx` transitórios e erros de transporte
- Agentes precisam de recuperação automática sem tempestade de retry nem retry em erros permanentes de cliente
- Rules Rust (retry/backoff) exigem: política nomeada explícita, classificação só de transitórios, backoff exponencial com jitter, respeito a `Retry-After`, kill switch, uma única camada no stack e documentação

## Decisão
- Tipo de política: `docsrs_cli::retry::RetryConfig` construído a partir de `Config`
- Default ligado: retries habilitados para GETs de produto (idempotentes)
- Defaults: `max_retries=3`, `retry_base_ms=200` (piso 50ms), `retry_max_delay_ms=30000`
- Orçamento duplo: `max_attempts` **e** `max_elapsed_ms` (default deriva de `timeout_secs`; teto 300s)
- Sleep planejado que excederia o budget restante aborta o retry (sem dormir além do budget)
- Kill switch: apenas `--disable-retry` / TOML `disable_retry` / `max_retries=0`
- Settings de produto não são lidos de variáveis de ambiente `DOCSRS_CLI_*`
- Conjunto de retry: `408`, `429`, `500`, `502`, `503`, `504` e erros de transporte reqwest timeout / connect / request
- Nunca fazer retry de `4xx` permanente (incl. 401/403/422), parse, body cap (`ErrorKind::Budget`, exit 74, `retryable=false`) ou cancel
- Exit `74` é compartilhado por três kinds: `network` retryable, `budget` permanente e `io`, cuja retryability depende da causa
- Agentes devem ramificar em `error.retryable`, nunca só no exit code, e nem só no `kind` — `io` é o kind que quebra o mapeamento de kind para retryability
- Backoff: full jitter `uniform(0..=min(base*2^n, max_delay))` com `tokio::time::sleep` monotônico e checks de cancel entre fatias
- `Retry-After`: delta-seconds **ou** HTTP-date via `httpdate` (datas passadas ≤1s de skew → espera zero; mais antigas → fórmula)
- Honrar `Retry-After` para `429` e `503` quando presente (sem jitter extra no hint absoluto); senão usar a fórmula
- Classificação: `RetryClass` (status), `RetryKind` (API de erro), `ErrorLayer` (camada de transporte)
- Camadas: um único loop dentro de `HttpClient::request`
- Sem reqwest-middleware e sem reexecução cega no nível do agente para kinds permanentes
- Observabilidade: span `tracing` `retry_attempt` (target `docsrs_cli::retry`); JSON `retryable` + `retry_after_secs`; doctor `retry_policy`

## Consequências
- Agentes obtêm fetches resilientes sem loops de retry customizados para erros transitórios
- Operadores podem desligar retries durante incidentes sem rebuild
- `Retry-After` em HTTP-date é honrado com dependência leve `httpdate`
- Circuit breaker / token-bucket / hedged requests permanecem fora de escopo para uma CLI one-shot de GET único
- Documentação do kill switch permanece alinhada a flags + config TOML

## Alternativas consideradas
- Middleware reqwest-retry: deps extras; integração de cancel mais difícil; rejeitado
- Default desligado (só opt-in): pior UX de agente em blips de CDN pública; rejeitado dada a superfície só-GET e o kill switch
- Parse HTTP-date com chrono: dep mais pesada; substituído por `httpdate` leve
- Kill switch por env `DOCSRS_CLI_DISABLE_RETRY`: removido em 1.1.x junto com outros knobs de produto por env; só flags + TOML
