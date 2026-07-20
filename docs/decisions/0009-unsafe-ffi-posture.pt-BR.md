[English](0009-unsafe-ffi-posture.md)

# ADR 0009 — Postura de unsafe e FFI do docsrs-cli

## Status
- Aceito (2026-07-19)

## Contexto
- As Rules Rust exigem minimizar `unsafe`, documentar cada bloco com `// SAFETY:`, encapsular FFI e nunca usar `unsafe` para silenciar o borrow checker.
- `docsrs-cli` é um **cliente** HTTPS **one-shot** (somente GET, allowlist de hosts fixa). Não há ABI C, crate `-sys`, bindgen nem alocador customizado.
- Mocks offline históricos usavam `DOCSRS_CLI_ALLOW_LOCALHOST` (env) e `unsafe { std::env::set_var(...) }` em testes de integração. Isso violava a regra de produto **sem knobs via variáveis de ambiente** e introduzia `unsafe` evitável.

## Decisão

### 1. Código de produto proíbe `unsafe`
- `src/lib.rs` e `src/main.rs` usam `#![forbid(unsafe_code)]`.
- Sem `extern "C"`, sem desreferência de ponteiro cru, sem `static mut`, sem `union`, sem `unsafe trait` no produto.
- Profile release usa `panic = "abort"` (CLI one-shot). Não há callback FFI; `catch_unwind` fica fora de escopo de propósito.

### 2. Allowlist de loopback só via CLI / XDG
- `Config.allow_loopback: bool` (default `false`).
- Fontes: chave XDG `config.toml` `allow_loopback = true` e/ou CLI `--allow-loopback` (OR; a CLI semeia o load antes do parse de origins no TOML).
- **Nunca** lido de `DOCSRS_CLI_ALLOW_LOCALHOST` nem de qualquer outra env.
- `is_allowlisted_host(host, allow_loopback)` e `AllowedOrigin::parse_with` carregam a política explicitamente.
- Redirect HTTP, gate de request e re-check de `final_url` no cache usam o mesmo flag (defesa em profundidade).

### 3. `unsafe` residual do harness (somente testes)
| Site | Por quê | SAFETY |
|------|---------|--------|
| `tests/signal_term.rs` (`cfg(unix)`) | `kill(2)` POSIX sem shell de CLI `kill` (`libc` só em dev-dependency) | PID do próprio filho; constantes de sinal; `last_os_error` imediato; race ESRCH aceita |

- Windows: testes de sinal pulados via stub `cfg(not(unix))`.
- Não resta `set_var` / mutação de env no tree de testes para allowlist.

### 4. `unsafe` transitivo em dependências
- Aceito na fronteira do crate. O produto ainda proíbe escrever `unsafe`.
- Stack TLS é só rustls+ring (ADR 0007 / `deny.toml` local). Operadores devem rodar `cargo audit` antes do deploy (ADR 0004).

### 5. Não-objetivos explícitos (N/A)
| Não-objetivo | Por quê |
|--------------|---------|
| bindgen / crate `-sys` / `cxx` / `pyo3` / `wasm-bindgen` | Sem superfície de linguagem estrangeira |
| Miri / sanitizers / cargo-vet em CI | Produto proíbe CI/CD in-tree |
| `catch_unwind` em callbacks FFI | Sem FFI; `panic=abort` |
| `NonNull` / layouts niche FFI | Sem ponteiros C |
| Env de sandbox de path (`DOCSRS_CLI_HOME`, …) | **Removido na Camada U / 1.1.3** — paths só flags CLI + ProjectDirs |
| Split de monólitos de teste | Não melhora soundness |

### 6. One-shot · memória · paralelismo
- **One-shot:** `--allow-loopback` morre com o processo; TOML é estado XDG do operador.
- **Memória:** política é `bool` (Copy); sem buffers FFI nem `Box::from_raw`.
- **Paralelismo:** closure de redirect captura `allow_loopback: bool` (sem locks novos); sem races de mutação global de env nos testes.

## Consequências
- Doctor `unsafe_posture` reporta forbid + fonte de loopback + ADR 0009.
- Wiremock offline: `allow_loopback = true` no `config.toml` e/ou `--allow-loopback`.
- Adicionar `unsafe` de produto exige emenda de ADR e revisão dedicada de soundness.
- Estabilização de `CommandExt::send_signal` pode substituir `libc::kill` no harness (revisitar).

## Relacionados
- ADR 0002 modelo de erro · ADR 0003 web-fetch · ADR 0004 threat model · ADR 0007 rustls
- Inventário de gaps: Camada G SEC-001, Camada P (unsafe/FFI)
- `src/config/allowlist.rs` · `src/http/client.rs` · `tests/signal_term.rs`
