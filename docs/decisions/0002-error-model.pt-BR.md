[English](0002-error-model.md)

# ADR 0002 — Modelo de erro estruturado do docsrs-cli
## Status
- Aceito (2026-07-18)

## Contexto
- docsrs-cli é ao mesmo tempo library (`docsrs_cli`) e CLI one-shot para agentes
- Agentes parseiam envelopes JSON de erro e exit codes
- Operadores precisam de SemVer estável para `ErrorKind` e mensagens técnicas em inglês sem segredos
- Rules Rust (error handling) exigem: `Result` tipado, erros de library com `thiserror`, cadeias `source`, sem `.unwrap()` em produção, `.expect` justificado, Display em minúsculas sem ponto final e exit codes estruturados

## Decisão
- `E` público: `AppError` via `thiserror`, nunca `anyhow` / `String` / `Box<dyn Error>` nu como tipo de erro do `Result` da library
- Kind + payload: `ErrorKind` (discriminante JSON snake_case estável + exit code estilo UNIX) pareado com string técnica `message` e opcionais `retry_after_secs` / `source`
- SemVer: `ErrorKind` e `AppError` são `#[non_exhaustive]` para que novos kinds ou variants não forcem major bump em matchers externos
- Cadeia de causa: `with_source` guarda `Arc<dyn Error + Send + Sync>`; Display mostra só `message`; callers percorrem `Error::source` até a causa raiz
- Clone: `AppError: Clone` compartilha o source via `Arc` para retries e logging reterem o erro original com baixo custo
- Classificação: `ErrorKind::is_retryable` / `is_permanent` e métodos espelhados em `AppError` para contratos de agente (alinhado ao retry do ADR 0001)
- Tetos locais de body/output usam `ErrorKind::Budget` (exit 74, permanente na mesma config); falhas de transporte mantêm `ErrorKind::Network` (exit 74, retryable)
- Caminho de emissão: toda falha de domínio na CLI passa por `emit_error` (envelope JSON ou stderr localizado)
- Falhas de load de config não devem usar `?` nu caindo em path hardcoded de exit 70
- Política de panic: só invariantes estáticos (regex / seletores CSS hardcoded) usam `.expect("… valid by construction")`
- I/O externo, parse e config sempre retornam `AppResult`
- Segurança: mensagens e envelopes JSON nunca incluem credenciais, bodies brutos de resposta ou paths de cache com segredos
- `from_http_status` aceita apenas um label curto de contexto não sensível

## Consequências
- Agentes obtêm `error.kind`, `error.code`, `retryable` e `retry_after_secs` opcional estáveis sem raspar texto livre
- Adicionar um novo `ErrorKind` é mudança SemVer minor para consumidores externos
- Desenvolvedores devem rotear falhas precoces da CLI por `emit_error`, não por `?` no handler de último recurso de `main`
- Fora de escopo neste produto: Sentry/OTLP, mapeamento de status de HTTP server, FFI `catch_unwind`, rollback de database, circuit breaker (ver ADR 0001 para a lista OOS de retry)
