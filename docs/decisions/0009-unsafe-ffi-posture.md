[Português (pt-BR)](0009-unsafe-ffi-posture.pt-BR.md)

# ADR 0009 — Unsafe code and FFI posture for docsrs-cli

## Status
- Accepted (2026-07-19)

## Context
- Rules Rust require minimizing `unsafe`, documenting every block with `// SAFETY:`, encapsulating FFI, and never using `unsafe` to silence the borrow checker.
- `docsrs-cli` is a **one-shot** HTTPS **client** (GET-only, fixed host allowlist). It has no C ABI surface, no `-sys` crate, no bindgen, and no custom allocator.
- Historical offline mocks used `DOCSRS_CLI_ALLOW_LOCALHOST` (env) plus `unsafe { std::env::set_var(...) }` in integration tests. That violated the product rule **no product knobs via environment variables** and introduced avoidable `unsafe`.

## Decision

### 1. Product code forbids `unsafe`
- `src/lib.rs` and `src/main.rs` use `#![forbid(unsafe_code)]`.
- No `extern "C"`, no raw pointer dereference, no `static mut`, no `union` field access, no `unsafe trait` impls in product code.
- Release profile uses `panic = "abort"` (one-shot CLI). There is no FFI callback path; `catch_unwind` is intentionally out of scope.

### 2. Loopback allowlist is CLI / XDG only
- `Config.allow_loopback: bool` (default `false`).
- Sources: XDG `config.toml` key `allow_loopback = true`, and/or CLI `--allow-loopback` (OR merge; CLI seeds load before TOML origin parse).
- **Never** read from `DOCSRS_CLI_ALLOW_LOCALHOST` or any other env var.
- `is_allowlisted_host(host, allow_loopback)` and `AllowedOrigin::parse_with` carry the policy explicitly.
- HTTP redirect, request gate, and disk-cache `final_url` re-check all use the same flag (defense in depth).

### 3. Harness residual `unsafe` (tests only)
| Site | Why | SAFETY |
|------|-----|--------|
| `tests/signal_term.rs` (`cfg(unix)`) | POSIX `kill(2)` without shelling out to a `kill` CLI (`libc` is dev-dependency only) | Own child PID; standard signal constants; immediate `last_os_error`; ESRCH race accepted |

- Windows: signal tests are skipped via `cfg(not(unix))` stub.
- No `set_var` / env mutation remains in the test tree for allowlist purposes.

### 4. Transitive dependency `unsafe`
- Accepted at the crate boundary. Product still forbids authoring `unsafe`.
- TLS stack is rustls+ring only (ADR 0007 / local `deny.toml`). Operators should run `cargo audit` before deploy (ADR 0004).

### 5. Explicit non-goals (N/A)
| Non-goal | Why |
|----------|-----|
| bindgen / crate `-sys` / `cxx` / `pyo3` / `wasm-bindgen` | No foreign language surface |
| Miri / sanitizers / cargo-vet in CI | Product forbids in-tree CI/CD |
| `catch_unwind` around FFI callbacks | No FFI; `panic=abort` |
| `NonNull` / niche FFI layouts | No C pointers |
| Path sandbox env (`DOCSRS_CLI_HOME`, …) | **Removed in Camada U / 1.1.3** — paths are CLI flags + ProjectDirs only |
| Splitting large integration test files | Does not improve soundness |

### 6. One-shot · memory · parallelism
- **One-shot:** `--allow-loopback` dies with the process; TOML is operator-owned XDG state.
- **Memory:** policy is a `bool` (Copy); no FFI buffers or `Box::from_raw`.
- **Parallelism:** redirect closure captures `allow_loopback: bool` (no new locks); no process-global env mutation races in tests.

## Consequences
- Doctor check `unsafe_posture` reports forbid + loopback source + ADR 0009.
- Offline wiremock: set `allow_loopback = true` in `config.toml` and/or pass `--allow-loopback`.
- Adding product `unsafe` requires an ADR amendment and dedicated soundness review.
- Stabilization of `std::os::unix::process::CommandExt::send_signal` may eventually replace `libc::kill` in the harness (revisit).

## Related
- ADR 0002 error model · ADR 0003 web-fetch · ADR 0004 threat model · ADR 0007 rustls
- Gaps inventory: Camada G SEC-001, Camada P (unsafe/FFI)
- `src/config/allowlist.rs` · `src/http/client.rs` · `tests/signal_term.rs`
