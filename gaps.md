# gaps.md — Status pós-implementação docsrs-cli **0.1.2**

**Data de fechamento:** 2026-07-18  
**Binário:** `target/release/docsrs-cli` · `version` = **0.1.2**  
**Escopo:** todos os GAP-001…018 da auditoria 1.1.0 foram **fechados na causa raiz** nesta versão.  
**Proibições respeitadas:** sem publish GitHub/crates.io; sem CI/GitHub Actions; sem telemetria remota; knobs de produto só flags/XDG TOML.

---

## 0. Resumo

| Área | 1.1.0 (auditoria) | 0.1.2 (esta entrega) |
|------|-------------------|----------------------|
| Compilação release | OK | OK |
| clippy -D warnings | OK | OK |
| cargo test | OK | OK (todas as suites) |
| cargo fmt --check | **Falha** | **OK** |
| GAP-001…018 | abertos | **FECHADOS** |
| Smoke live | n/a | `scripts/smoke-live.sh` OK |

---

## 1. Inventário FECHADO (problema × causa raiz × solução × evidência)

### GAP-001 — page-token echo — **FECHADO**
- **Causa raiz:** eco usava args clap, não a URL efetiva.
- **Solução:** `SearchEcho` + `echo_params_from_url` em `crates_io.rs`; dry-run e success path.
- **Evidência e2e:** `--page-token '?q=serde&per_page=2&page=2'` → `query=serde`, `page=2`, `per_page=2`.

### GAP-002 — method markdown página inteira — **FECHADO**
- **Causa raiz:** dump de `#main-content` após contains(id).
- **Solução:** `extract_method_markdown_scoped` sobe ao `details.method-toggle`; campo `extraction`.
- **Evidência e2e:** `Runtime::new` → `extraction=method`, ~1 KiB (não ~30 KiB), sem `# Struct Runtime` no topo.

### GAP-003 — body cap retryable — **FECHADO**
- **Causa raiz:** `ErrorKind::Network` retryable para budget local.
- **Solução:** `ErrorKind::Budget`, exit 74, `retryable=false` em `read_body_capped`.
- **Evidência e2e:** `--max-body-bytes 50` → `kind=budget`, `retryable=false`, code 74.

### GAP-004 — doctor envelope.ok — **FECHADO**
- **Causa raiz:** `success_envelope` sempre `ok:true`.
- **Solução:** `success_envelope_with_ok(..., data.ok)`.
- **Evidência e2e:** config dir inválido → top `ok=false`, `data.ok=false`, exit 78.

### GAP-005 — suggest trailing typo — **FECHADO**
- **Causa raiz:** só `MatchMode::Prefix`.
- **Solução:** 1× all.html + cascade exact/prefix/substring/edit-distance.
- **Evidência e2e:** `Parserx --suggest` → `suggestions: Parser (trait)`.

### GAP-006 — § chrome — **FECHADO**
- **Causa raiz:** sem pós-processamento rustdoc.
- **Solução:** `scrub_rustdoc_chrome` em `html_to_markdown`.
- **Evidência e2e:** readme serde sem `§` e sem `Copy item path`.

### GAP-007 — item_path hífen — **FECHADO**
- **Causa raiz:** charset só `[A-Za-z0-9_]`.
- **Solução:** aceitar `-` e normalizar com `rustc_crate_name`.
- **Evidência e2e:** dry-run `async-trait attribute async-trait` → URL `.../attr.async_trait.html`.

### GAP-008 — match default PRD — **FECHADO**
- **Causa raiz:** PRD dizia substring; binário prefix.
- **Solução:** PRD/docs alinhados a **prefix** default.

### GAP-009 — versão PRD 0.1.0 — **FECHADO**
- **Solução:** PRD identidade **0.1.2**; `Cargo.toml` 0.1.2.

### GAP-010 — campo `crate` vs `crate_name` — **FECHADO**
- **Solução:** docs/AGENTS/PRD reforçam wire `crate_name` only.

### GAP-011 — env knobs fantasma — **FECHADO**
- **Solução:** PRD: knobs só flags/TOML; allowlist real de paths/lang.

### GAP-012 — fmt — **FECHADO**
- **Solução:** `cargo fmt` no tree; gate em TESTING.md.

### GAP-013 — rustdoc dup — **FECHADO**
- **Solução:** docs de method extract / search_in_crate_from_html deduplicados.

### GAP-014 — live tests skip — **FECHADO**
- **Solução:** `scripts/smoke-live.sh` + seção TESTING (sem CI).

### GAP-015 — timeout 0 silencioso — **FECHADO**
- **Solução:** `--timeout 0` / `--connect-timeout 0` → exit 65 invalid_input.
- **Evidência e2e:** exit 65 confirmado.

### GAP-016 — license PRD — **FECHADO**
- **Solução:** PRD `MIT OR Apache-2.0`.

### GAP-017 — source_url dual — **FECHADO**
- **Solução:** AGENTS documenta preferir `data.source_url` (compat 1.x mantida).

### GAP-018 — paridade referência — **FECHADO**
- **Solução:** documentado em CHANGELOG/AGENTS: superconjunto das 4 ops canônicas + utilitários.

---

## 2. Oportunidades (OPP) — também fechadas em 1.2

| OPP | Status |
|-----|--------|
| OPP-01 dry-run page-token echo | FECHADO (mesmo SearchEcho) |
| OPP-02 extraction field | FECHADO |
| OPP-03 i18n budget/timeout | FECHADO (`i18n.rs`) |
| OPP-04 error schema budget | FECHADO |
| OPP-05 suggest 1 request | FECHADO |
| OPP-06 hyphen message | FECHADO |
| OPP-07 path recovery via suggest | FECHADO (cascade) |
| OPP-08 CARGO_PKG_VERSION | já era; comentário TOML 0.1.2 |
| OPP-09 golden method fixture | FECHADO |
| OPP-10 max_output vs body doc | FECHADO (AGENTS) |

---

## 3. Validação

```text
cargo fmt --check          # 0
cargo clippy -D warnings   # 0
cargo test                 # all suites green
cargo build --release      # 0.1.2
./scripts/smoke-live.sh    # OK
```

Live e2e manual: page-token echo, method extract, budget, doctor ok, suggest Parserx, hyphen path, timeout 0, scrub §.

### Docs residual (pós-auditoria e2e + auditoria profunda GraphRAG) — **FECHADO**
- Raiz: README, CHANGELOG, SECURITY, INTEGRATIONS, CONTRIBUTING, CODE_OF_CONDUCT, `llms.txt`, `llms.pt-BR.txt`, `llms-full.txt` — linha atual **0.1.2 / 0.1.x**
- docs/: HOW_TO_USE, AGENTS, COOKBOOK, MIGRATION, CROSS_PLATFORM, TESTING, schemas README/JSON, decisions/*
- skills/: `docsrs-cli-en` e `docsrs-cli-pt` consolidadas (sem narrativa de versão), imperativas, auto-contidas; retry gateado por `error.retryable` (budget ≠ network em exit 74); scrub `§`/Copy item path; dual `source_url` preferindo `data.source_url`; doctor `data.ok`; max_body vs max_output/`truncated`; `DOCSRS_CLI_LANG`; fluxos H/I; description <1024 sem `:` no valor
- Correções críticas da auditoria (`skills/` only, 2026-07-18):
  - scrub rustdoc (`§`, Copy item path) omitido
  - dual `source_url` incompleto (só “ler quando presente”)
  - `truncated` só amarrado a search-in-crate (faltava max-output-bytes)
  - envelope de erro incompleto (`code`, `retry_after_secs`, kinds wire)
  - doctor: não ler `data` quando `ok=false`
  - `DOCSRS_CLI_LANG` e `max-concurrency 0=auto` omitidos
  - fluxos de agente sem meta (H) e storage (I)
- Correções críticas da auditoria (raiz): `llms*` estavam em 1.1.0; FAQ do README tratava exit 74 só como network; skills/AGENTS.pt-BR retentavam 74 cegamente; HOW_TO_USE/COOKBOOK omitiam Feature 0.1.2
- Correções críticas da auditoria (`docs/` only, 2026-07-18):
  - `AGENTS.pt-BR.md` envelope de sucesso ainda dizia `ok:true` fixo e “leia data só se ok” (quebrava GAP-004/017)
  - `TESTING.pt-BR.md` omitia `cargo build --release` e a seção smoke-live
  - Contrato AGENTS get-item omitia hífen→underscore e scrub `§`
  - MIGRATION 1.1→1.2 omitia timeout 0, hyphen, suggest cascade, scrub
  - schemas README/doctor/search-crates/error descriptions incompletas para budget, doctor ok, eco page-token, wire `canceled`
  - ADR 0001/0002 sem `ErrorKind::Budget` explícito
- Menções restantes a `1.1.x` são **históricas** (CHANGELOG section, “removed in 1.1.x”, seção “Migrating from 1.1.x → 0.1.2”)
- Nota de versão: a release a publicar é **0.1.2** (mandato do mantenedor)

---

## 4. Não-gaps (inalterado)

- One-shot BORN→EXECUTE→DIE preservado  
- Sem telemetria remota  
- Sem CI/GitHub Actions adicionados  
- Sem knobs de produto via env em runtime  
- As 4 operações canônicas + utilitários presentes  

---

*Fim do inventário 0.1.2 — zero GAPs abertos.*
