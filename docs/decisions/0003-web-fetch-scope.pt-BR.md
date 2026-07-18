[English](0003-web-fetch-scope.md)

# ADR 0003 — Escopo de web fetch (cliente de docs, não crawler)
## Status
- Aceito (2026-07-18)

## Contexto
- Rules Rust de web scraping, crawling e extração de dados cobrem produtos crawler gerais (robots.txt REP, sitemaps, RSS, meta robots, anti-bot, encoding_rs multi-charset, MinHash, rotação de proxy, headless Chrome)
- `docsrs-cli` é uma CLI one-shot stdin/stdout para a API do crates.io e páginas públicas de documentação do docs.rs / doc.rust-lang.org
- A superfície de produto é só GET, um request primário por comando, allowlist fixa de hosts HTTPS, sem login, sem coleta de PII, sem fronteira multi-URL

## Decisão
- Tratar a CLI como cliente educado de documentação, não como scraper geral
- Manter camadas separadas: `reqwest` (bytes) → sniff de Content-Type → `scraper` / `serde_json` → `data` estruturado + `source_url`
- Aplicar polidez com floor de delay por host + jitter aditivo, lock+stamp cross-process, User-Agent com contact, caps de stream de body, TLS só rustls
- Documentar o seguinte como fora de escopo de produto a menos que um comando futuro vire crawler multi-URL:
  - parser REP de robots.txt e Crawl-delay de arquivos robots remotos
  - Meta robots / X-Robots-Tag / seguir links rel=nofollow
  - revalidação HTTP condicional (ETag / If-None-Match / If-Modified-Since)
  - RSS/Atom, sitemap XML, harvest JSON-LD Schema.org
  - pipelines `encoding_rs` non-UTF-8 (alvos servem UTF-8)
  - evasão anti-bot (wreq / fingerprint BoringSSL, CAPTCHA, headless Chrome)
  - health checks de proxy e rotação multi-proxy
  - MinHash de conteúdo / dedup Bloom de URL entre crawls
  - WebSocket / GraphQL
  - pipelines de coleta GDPR de PII (o produto não raspa perfis pessoais)

## Justificativa
- Hosts oficiais existem para servir metadados públicos de crates e HTML rustdoc
- Um GET primário por processo não é crawl; rate limit + allowlist + UA já demonstram boa-fé sem máquina de estados de robots.txt
- Suporte REP completo adicionaria fetch de robots, cache e engine de política para zero comandos de produto que descobrem URLs arbitrárias
- GET condicional / ETag precisa de cooperação da origin e complica a chave do cache XDG de body sem ganhos medidos de latência em paths one-shot de agente

## Consequências
- Operadores não devem apontar overrides de origin a sites de terceiros que proíbem acesso automatizado; allowlist + `DOCSRS_CLI_ALLOW_LOCALHOST` existem para mocks
- Construtores de datasets que reutilizem este binário para arquivo em massa devem adicionar a própria política de crawl (robots, retenção, ToS) fora deste produto
- Fan-out multi-GET futuro reabriria itens de ADR (robots, ETag, frontier)

## Relacionados
- `src/http.rs`, `src/docs_rs.rs`, `src/crates_io.rs`, `src/retry.rs`
- ADR 0001 (política de retry HTTP), ADR 0002 (modelo de erro)
