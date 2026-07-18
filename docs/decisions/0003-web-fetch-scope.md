[Português (pt-BR)](0003-web-fetch-scope.pt-BR.md)

# ADR 0003 — Web fetch scope (docs client, not a crawler)
## Status
- Accepted (2026-07-18)

## Context
- Rules Rust for web scraping, crawling, and data extraction cover general crawler products (robots.txt REP, sitemaps, RSS, meta robots, anti-bot, encoding_rs multi-charset, MinHash, proxy rotation, headless Chrome)
- `docsrs-cli` is a one-shot stdin/stdout agent CLI for crates.io API and docs.rs / doc.rust-lang.org public documentation pages
- Product surface is GET-only, one primary request per command, fixed HTTPS host allowlist, no login, no PII harvest, no multi-URL frontier

## Decision
- Treat the CLI as a polite documentation client, not a general scraper
- Keep layers separated: `reqwest` (bytes) → Content-Type sniff → `scraper` / `serde_json` → structured `data` + `source_url`
- Enforce politeness with per-host delay floor + additive jitter, cross-process lock+stamp, User-Agent with contact, body stream caps, rustls-only TLS
- Document the following as out of product scope unless a future command becomes a multi-URL crawler:
  - robots.txt REP parser and Crawl-delay from remote robots files
  - Meta robots / X-Robots-Tag / rel=nofollow link following
  - HTTP conditional revalidation (ETag / If-None-Match / If-Modified-Since)
  - RSS/Atom, sitemap XML, JSON-LD Schema.org harvest
  - `encoding_rs` non-UTF-8 pipelines (targets serve UTF-8)
  - Anti-bot evasion (wreq / BoringSSL fingerprint, CAPTCHA, headless Chrome)
  - Proxy health checks and multi-proxy rotation
  - Content MinHash / Bloom URL dedup across crawls
  - WebSocket / GraphQL
  - GDPR PII collection pipelines (product does not scrape personal profiles)

## Rationale
- Official hosts exist to serve public crate metadata and rustdoc HTML
- One primary GET per process is not a crawl; rate limit + allowlist + UA already demonstrate good faith without a robots.txt state machine
- Full REP support would add a robots fetch, cache, and policy engine for zero product commands that discover arbitrary URLs
- Conditional GET / ETag needs origin cooperation and complicates the XDG body cache key without measured latency wins on one-shot agent paths

## Consequences
- Operators must not point origin overrides at third-party sites that forbid automated access; allowlist + `DOCSRS_CLI_ALLOW_LOCALHOST` exist for mocks
- Dataset builders reusing this binary for bulk archive work must add their own crawl policy (robots, retention, ToS) outside this product
- Future multi-GET fan-out would reopen ADR items (robots, ETag, frontier)

## Related
- `src/http.rs`, `src/docs_rs.rs`, `src/crates_io.rs`, `src/retry.rs`
- ADR 0001 (HTTP retry policy), ADR 0002 (error model)
