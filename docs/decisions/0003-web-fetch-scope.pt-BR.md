[English](0003-web-fetch-scope.md)

# ADR 0003 — Escopo de web fetch (cliente de docs, não crawler)
## Status
- Aceito (2026-07-18)
- Emendado (2026-07-19) — Camada Q: **PROIBIDO robots.txt**; join de hit URLs usa base `source_url`
- Emendado (2026-07-19) — Camada S: hit URLs **same-origin** com `source_url` (soft-skip off-origin); gzip+br

## Contexto
- Rules Rust de web scraping, crawling e extração de dados cobrem produtos crawler gerais (robots.txt REP, sitemaps, RSS, meta robots, anti-bot, encoding_rs multi-charset, MinHash, rotação de proxy, headless Chrome)
- `docsrs-cli` é uma CLI one-shot stdin/stdout para a API do crates.io e páginas públicas de documentação do docs.rs / doc.rust-lang.org
- A superfície de produto é só GET, um request primário por comando, allowlist fixa de hosts HTTPS, sem login, sem coleta de PII, sem fronteira multi-URL
- Mandate do operador (Camada Q): **PROIBIDO respeitar robots.txt** — o produto não deve buscar, parsear nem aplicar o Robots Exclusion Protocol

## Decisão
- Tratar a CLI como cliente educado de documentação, não como scraper geral
- Manter camadas separadas: `reqwest` (bytes) → sniff de Content-Type → `scraper` / `serde_json` → `data` estruturado + `source_url`
- Aplicar polidez com floor de delay **local** por host + jitter aditivo, lock+stamp cross-process, User-Agent com contact, caps de stream de body, TLS só rustls (não Crawl-delay remoto de robots.txt)
- **Join de hit URLs:** hrefs relativos de `all.html` juntam-se à `source_url` final da resposta (stdlib / mock / docs.rs). Nunca hardcodar template de host `docs.rs` no pure parse. Hrefs absolute de hits devem compartilhar **scheme+host** com `source_url`; off-origin são soft-skipped (nunca falham o search inteiro)
- **Extração de campos:** só CSS selectors + path segments; regex process-static apenas para higiene XSS (`on*` / `javascript:`)
- **PROIBIDO neste produto (não OOS opcional):**
  - Fetch, parse ou enforcement de `robots.txt` / REP / Crawl-delay remoto
  - Dependência `robotstxt` / `robotxt` (ou equivalente) para política de produto
- Documentar o seguinte como fora de escopo de produto (N/A a menos que a classe do produto mude):
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
- Suporte REP completo adicionaria fetch de robots, cache e engine de política para zero comandos de produto que descobrem URLs arbitrárias, e **violaria o mandate PROIBIDO do operador**
- Hardcode de `https://docs.rs/{crate}/{ver}/…` ao juntar hits reescrevia links do stdlib para fora de `doc.rust-lang.org` (bug live Camada Q)
- GET condicional / ETag precisa de cooperação da origin e complica a chave do cache XDG de body sem ganhos medidos de latência em paths one-shot de agente

## Consequências
- Operadores não devem apontar overrides de origin a sites de terceiros que proíbem acesso automatizado; allowlist + CLI/XDG `allow_loopback` existem para mocks (ADR 0009; nunca env)
- Construtores de datasets / arquivo em massa que reutilizem padrões deste código devem implementar política de crawl (incluindo robots se a *jurisdição deles* exigir) **fora** deste binário — este produto não ganhará suporte REP
- Fan-out multi-GET futuro reabriria itens de ADR (frontier, ETag); robots permanece **PROIBIDO** até novo ADR superseder este mandate
- Doctor expõe `web_fetch_posture` resumindo esta decisão

## Relacionados
- `src/http/`, `src/docs_rs/`, `src/crates_io.rs`, `src/retry.rs`, `src/doctor.rs`
- ADR 0001 (política de retry HTTP), ADR 0002 (modelo de erro), ADR 0007 (rustls), ADR 0009 (unsafe/FFI)
