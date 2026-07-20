[Português (pt-BR)](0007-rustls-posture.pt-BR.md)

# ADR 0007 — rustls TLS posture for docsrs-cli

## Status
- Accepted (2026-07-19)

## Context
- Rules Rust mandate **rustls-only** TLS (no `native-tls` / OpenSSL), floor **TLS 1.2**, `CryptoProvider` bootstrap in the binary, and an explicit provider choice.
- `docsrs-cli` is a **one-shot** HTTPS **client** (GET-only against a fixed host allowlist). It is not a TLS server, ACME endpoint, or mTLS workload identity agent.
- HTTP is implemented with **reqwest 0.12** (`default-features = false`, feature `rustls-tls`), which pulls `hyper-rustls` / `tokio-rustls` / **rustls 0.23.x** and, via `__rustls-ring`, the **ring** crypto provider.
- Upstream rustls defaults to **aws-lc-rs** (+ optional post-quantum KX). reqwest 0.12’s `rustls-tls` feature path deliberately enables **ring** instead.

## Decision

### 1. Single TLS stack
- **Only** rustls for product TLS. Never enable `native-tls`, `openssl` / `openssl-sys`, or dual stacks in the same binary.
- Direct dependency pin: `rustls` with `default-features = false` and features `std`, `tls12`, `ring` (floor **≥ 0.23.18** for the Acceptor panic advisory era; product is client-only but the floor still applies).
- Transitively: `webpki-roots` via reqwest `rustls-tls-webpki-roots` (Mozilla roots; reproducible in containers).

### 2. Crypto provider = ring (Option A)
| Choice | Rationale |
|--------|-----------|
| **ring** (accepted) | Aligns with reqwest 0.12 `rustls-tls` / `__rustls-ring`; single provider in the resolve graph; portable Linux/macOS/Windows and musl-friendly builds without aws-lc cmake/nasm requirements |
| **aws-lc-rs** (rejected for now) | Preferable for PQ defaults upstream, but changes the feature graph (`*-no-provider` + explicit aws-lc deps) and raises cross/musl cost for a one-shot docs CLI |

- The **binary** calls `rustls::crypto::ring::default_provider().install_default()` **once** at the top of `main`, **before** building the Tokio runtime or any HTTP client.
- Libraries / `docsrs_cli` lib code **must not** call `install_default` (consumer/binary responsibility).
- Never enable both `ring` and `aws_lc_rs` features on rustls in this product.

### 3. Client configuration
- reqwest builder: `.use_rustls_tls()` + `.min_tls_version(TLS_1_2)`.
- Certificate validation always on: never `danger_accept_invalid_certs` / hostname bypass / custom `NoVerifier` in product code.
- No `KeyLog` / `SSLKEYLOGFILE` integration in product builds.
- **Not** `https_only(true)` on the reqwest client: offline wiremock uses `http://127.0.0.1` under the localhost test gate. Production hosts still require **https** via the shared origin allowlist (`is_allowed_origin_scheme_host`).
- TLS 1.2 remains enabled (`tls12` feature) because the product floor is 1.2 and public docs CDNs may still negotiate it; TLS 1.3 is used when the peer offers it.

### 4. Trust, proxy, multi-OS
- **Roots:** webpki-roots (not `rustls-platform-verifier`) so container and agent environments share a known Mozilla set. Operators should refresh the lockfile periodically (webpki-roots freshness).
- **System proxy** (`HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`) is honored via reqwest `system-proxy`. A corporate TLS-terminating proxy is an **operator trust decision**; the product still applies the **target** host allowlist after proxy resolution.
- Same rustls+ring stack on Linux, macOS, and Windows (no OpenSSL runtime).

### 5. One-shot · memory · parallelism (TLS cross-cuts)
- **One-shot:** one process lifecycle per command; a full TLS handshake per invocation is expected (no long-lived session ticket daemon).
- **Pool:** tiny idle connection pool (named constants) — anti-daemon, not a connection multiplexer service.
- **Memory:** body stream caps (`try_reserve*`, `max_body_bytes`) apply **after** TLS; product stores no private keys or client certs.
- **Parallelism:** multi-thread Tokio for I/O; `HttpClient` is `&self`-shareable; no process-global TLS config mutation after bootstrap.

### 6. Explicit non-goals (N/A)
| Non-goal | Why |
|----------|-----|
| TLS server / `Acceptor` / `LazyConfigAcceptor` | Client-only; RUSTSEC-2024-0399 server path does not apply to product code |
| mTLS / client certificates / SPIFFE / HSM / ACME | Public docs origins; no mutual auth |
| ECH / FIPS / prefer-post-quantum | No product requirement; PQ would require aws-lc migration (revisit with ADR) |
| QUIC / HTTP/3 / gRPC / WebSocket / DTLS | GET HTTP/1.1+h2 only (ADR 0003) |
| CI deny gates / cosign / SBOM pipelines | CI/CD out of product delivery constraint; local `deny.toml` still used by maintainers |
| Product knobs via `DOCSRS_CLI_TLS_*` env | Paths use CLI flags + ProjectDirs XDG only; TLS posture is compile-time + code, not runtime env knobs |

## Consequences
- `cargo tree` must show **no** `native-tls` / `openssl` and **exactly one** of {ring, aws-lc} under rustls (today: **ring**).
- Doctor `http_client_posture` reports `provider=ring` and rustls floor for agent audits.
- Major rustls upgrades or a provider switch (ring → aws-lc) require an ADR amendment and CHANGELOG Security note.
- Local `deny.toml` bans alternate TLS crates to fail closed on accidental feature pulls.

## Related
- ADR 0003 web-fetch scope · ADR 0004 threat model · `SECURITY.md` · `src/http/client.rs` · `src/main.rs` · `deny.toml`
- Gaps inventory: Camada M (rustls)
