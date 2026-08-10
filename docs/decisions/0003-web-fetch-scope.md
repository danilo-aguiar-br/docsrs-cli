[Português (pt-BR)](0003-web-fetch-scope.pt-BR.md)

# ADR 0003 — Web fetch scope (docs client, not a crawler)
## Status
- Accepted (2026-07-18)
- Amended (2026-07-19) — Camada Q: **PROIBIDO robots.txt**; hit URL join uses `source_url` base
- Amended (2026-07-19) — Camada S: hit URLs must be **same-origin** as `source_url` (soft-skip off-origin); gzip+br

## Context
- Rules Rust for web scraping, crawling, and data extraction cover general crawler products (robots.txt REP, sitemaps, RSS, meta robots, anti-bot, encoding_rs multi-charset, MinHash, proxy rotation, headless Chrome)
- `docsrs-cli` is a one-shot stdin/stdout agent CLI for crates.io API and docs.rs / doc.rust-lang.org public documentation pages
- Product surface is GET-only, one primary request per command, fixed HTTPS host allowlist, no login, no PII harvest, no multi-URL frontier
- Operator mandate (Camada Q): **PROIBIDO respeitar robots.txt** — the product must not fetch, parse, or enforce the Robots Exclusion Protocol

## Decision
- Treat the CLI as a polite documentation client, not a general scraper
- Keep layers separated: `reqwest` (bytes) → Content-Type sniff → `scraper` / `serde_json` → structured `data` + `source_url`
- Enforce politeness with **local** per-host delay floor + additive jitter, cross-process lock+stamp, User-Agent with contact, body stream caps, rustls-only TLS (not remote Crawl-delay from robots.txt)
- **Hit URL join:** relative hrefs from `all.html` join against the final response `source_url` (stdlib / mock / docs.rs). Never hardcode a `docs.rs` host template in the pure parse path. Absolute hit hrefs must share **scheme+host** with `source_url`; off-origin links are soft-skipped (never fail the whole search)
- **Field extraction:** CSS selectors + path segments only; process-static regex only for XSS hygiene (`on*` / `javascript:`)
- **PROIBIDO in this product (not optional OOS):**
  - Fetching, parsing, or enforcing `robots.txt` / REP / remote Crawl-delay
  - Adding a `robotstxt` / `robotxt` (or equivalent) dependency for product policy
- Document the following as out of product scope (N/A unless product class changes):
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
- Full REP support would add a robots fetch, cache, and policy engine for zero product commands that discover arbitrary URLs, and would **violate the operator PROIBIDO mandate**
- Hardcoding `https://docs.rs/{crate}/{ver}/…` when joining hits rewrote stdlib links off `doc.rust-lang.org` (Camada Q live bug)
- Conditional GET / ETag needs origin cooperation and complicates the XDG body cache key without measured latency wins on one-shot agent paths

## Consequences
- Operators must not point origin overrides at third-party sites that forbid automated access; allowlist + CLI/XDG `allow_loopback` exist for mocks (ADR 0009; never env)
- Bulk archive / dataset builders that reuse patterns from this codebase must implement crawl policy (including robots if required by *their* jurisdiction) **outside** this binary — this product will not grow REP support
- Future multi-GET fan-out would reopen ADR items (frontier, ETag); robots remains **PROIBIDO** unless a new ADR supersedes this mandate
- Doctor exposes `web_fetch_posture` summarizing this decision

## Related
- `src/http/`, `src/docs_rs/`, `src/crates_io/`, `src/retry/`, `src/doctor.rs`
- ADR 0001 (HTTP retry policy), ADR 0002 (error model), ADR 0007 (rustls), ADR 0009 (unsafe/FFI)
