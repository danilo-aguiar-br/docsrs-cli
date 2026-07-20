[Português (pt-BR)](0004-threat-model.pt-BR.md)

# ADR 0004 — Threat model (STRIDE) for docsrs-cli
## Status
- Accepted (2026-07-19)

## Context
- Rules Rust require a documented threat model before (and after) security-relevant design work
- `docsrs-cli` is a **one-shot** stdin/stdout CLI: no daemon, no API keys, no multi-tenant server
- Attackers are assumed to know the full source (open source); security does not rely on obscurity

## Assets
| Asset | Sensitivity | Notes |
|-------|-------------|-------|
| Host process integrity | High | Agent/CI runner running the binary |
| Local XDG cache/config | Medium | Public docs bodies; modes `0o600`/`0o700` preferred |
| Network egress | High | Must not become SSRF pivot to internal hosts |
| stdout/stderr content | Medium | Agent-consumed; no secrets by design |
| Dependency graph / binary | High | Supply chain of crates.io crates |

## Trust boundaries
1. **Untrusted:** argv, env (path/locale/proxy allowlist only), XDG `config.toml`, HTTP responses, disk cache files, DNS, system clock
2. **Trusted after validation:** domain newtypes (`CrateName`, `ItemPath`, …), clamped `Config`, allowlisted `Url`s
3. **Out of product:** secrets managers, OAuth, multi-tenant auth, container orchestration, CI OIDC

## Attackers
| Actor | Capability |
|-------|------------|
| Malicious argv / agent prompt injection | Hostile crate names, queries, paths, flags |
| Malicious or compromised origin (if allowlist bypassed) | Hostile HTML/JSON, redirects, large bodies |
| Local multi-user host | Read cache; rewrite `config.toml` / cache meta |
| Network MITM (without TLS) | Blocked by rustls + cert validation |
| Supply-chain crate compromise | Transitive `unsafe` / malicious build scripts |

## STRIDE map (critical components)

| Component | S | T | R | I | D | E | Primary controls |
|-----------|---|---|---|---|---|---|------------------|
| CLI / domain parse | | ✓ spoofed input | | ✓ invalid types | ✓ huge args | | Newtypes, length caps, control/invisible reject |
| Config TOML | | ✓ typo keys | | ✓ evil origins | ✓ TOML bomb | | Cap 64 KiB, `deny_unknown_fields`, origin allowlist, UA ASCII |
| HTTP client | | ✓ SSRF / redirect | | ✓ MIME confusion | ✓ slowloris/body | | Host allowlist (config+redirect+request+cache), GET-only, body stream cap, timeouts, rustls ≥1.2 |
| Disk cache | | ✓ poisoned meta | | ✓ path key | ✓ disk fill | | SHA-256 keys hex-only, body/meta caps+checksum, `final_url` re-check, max_bytes eviction |
| HTML scrub | | | | ✓ XSS in MD | | | Drop script/style; strip `on*` / `javascript:` |
| Retry | | | | | ✓ retry storm | | Dual budget, no retry on permanent 4xx/parse/budget |
| Concurrency | | | | | ✓ task explosion | | `ConcurrencyBudget`, bounded blocking pool |
| Process spawn | | | | | | | **N/A product** (no `Command`); tests use sanitized helper |

Legend: S spoofing · T tampering · R repudiation · I information disclosure · D denial of service · E elevation of privilege

## Accepted risks (explicit)
| Risk | Justification |
|------|----------------|
| No mTLS / client certs | Public docs origins; no mutual auth needed |
| No robots.txt REP | **PROIBIDO** in product (ADR 0003 Camada Q); one-shot docs client, not a crawler |
| No Zeroizing secrets | Product stores no API keys or credentials |
| No seccomp/Landlock sandbox | Host OS responsibility for agent runners; CLI is unprivileged user process |
| Missing Content-Type still accepted | Body parse fails closed; present-but-wrong CT rejected |
| NFC not applied to free-text search | Domain IDs are ASCII; search is not an identity key — format/invisible chars rejected instead |
| Transitive `unsafe` in deps | Product `#![forbid(unsafe_code)]`; operators should run `cargo audit` before deploy |
| No CI/SLSA in this delivery line | Publish/CI out of scope for current audit constraint |

## CVSS usage
- Discovered product vulns are triaged with CVSS v4 in `SECURITY.md` SLAs
- Prioritize by real exposure: SSRF/allowlist bypass and unbounded allocation rank above cosmetic issues

## Related
- `SECURITY.md`, `src/http.rs`, `src/config.rs`, `src/domain.rs`, `src/cache.rs`
- ADR 0001 (retry), 0002 (errors), 0003 (web fetch scope)
- Gaps inventory: Camada G (defensive) + Camada H (security development)
