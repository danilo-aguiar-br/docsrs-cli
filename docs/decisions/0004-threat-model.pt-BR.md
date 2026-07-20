[English](0004-threat-model.md)

# ADR 0004 — Modelo de ameaças (STRIDE) do docsrs-cli
## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust exigem modelo de ameaças documentado em mudanças de segurança
- `docsrs-cli` é CLI **one-shot** stdin/stdout: sem daemon, sem API keys, sem servidor multi-tenant
- Atacantes conhecem o código-fonte integral (open source); sem segurança por obscuridade

## Ativos
| Ativo | Sensibilidade | Notas |
|-------|---------------|-------|
| Integridade do processo host | Alta | Runner de agente/CI |
| Cache/config XDG local | Média | Corpos públicos; modos `0o600`/`0o700` |
| Egress de rede | Alta | Não pode virar pivô SSRF |
| Conteúdo stdout/stderr | Média | Consumido por agentes; sem segredos |
| Grafo de dependências / binário | Alta | Supply chain |

## Fronteiras de confiança
1. **Não confiável:** argv, env (allowlist path/locale/proxy), `config.toml`, respostas HTTP, cache em disco, DNS, relógio
2. **Confiável após validação:** newtypes de domínio, `Config` com clamp, URLs allowlisted
3. **Fora do produto:** secret managers, OAuth, multi-tenant, orquestração de containers, CI OIDC

## Atacantes
| Ator | Capacidade |
|------|------------|
| Argv / prompt injection malicioso | Nomes, queries, paths e flags hostis |
| Origem comprometida (se bypass de allowlist) | HTML/JSON hostil, redirects, bodies grandes |
| Host multi-usuário local | Ler cache; reescrever config/meta |
| MITM de rede (sem TLS) | Bloqueado por rustls + validação de cert |
| Compromisso de crate | `unsafe` transitivo / build scripts maliciosos |

## Mapa STRIDE (componentes críticos)

| Componente | Controles principais |
|------------|----------------------|
| CLI / parse de domínio | Newtypes, caps de tamanho, rejeição de control/invisíveis |
| Config TOML | Cap 64 KiB, `deny_unknown_fields`, allowlist de origin, UA ASCII |
| Cliente HTTP | Allowlist (config+redirect+request+cache), GET-only, cap de body, timeouts, rustls ≥1.2 |
| Cache em disco | Chaves hex SHA-256, caps+checksum, re-check de `final_url` |
| Scrub HTML | Remove script/style; strip `on*` / `javascript:` |
| Retry | Budget dual; sem retry em 4xx permanente/parse/budget |
| Concorrência | `ConcurrencyBudget`, pool blocking limitado |
| Spawn de processo | **N/A no produto** |

## Riscos aceitos (explícitos)
| Risco | Justificativa |
|-------|---------------|
| Sem mTLS | Origens públicas de docs |
| Sem robots.txt REP | **PROIBIDO** no produto (ADR 0003 Camada Q); cliente one-shot, não crawler |
| Sem Zeroizing de segredos | Produto não guarda credenciais |
| Sem seccomp/Landlock | Responsabilidade do host; CLI é processo de usuário |
| Content-Type ausente aceito | Parse do body falha fechado; CT errado presente é rejeitado |
| NFC não em free-text search | IDs ASCII; chars de formato rejeitados |
| `unsafe` transitivo | Produto `#![forbid(unsafe_code)]`; `cargo audit` recomendado |
| Sem CI/SLSA nesta linha | Fora do escopo da entrega atual |

## Relacionado
- `SECURITY.md`, `src/http.rs`, `src/config.rs`, `src/domain.rs`, `src/cache.rs`
- ADR 0001–0003; inventário gaps camadas G e H
