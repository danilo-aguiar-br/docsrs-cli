[Português (pt-BR)](0007-rustls-posture.pt-BR.md)

# ADR 0007 — rustls TLS posture for docsrs-cli

## Status
- Accepted (2026-07-19)

## Context
- Rules Rust mandate **rustls-only** TLS (no `native-tls` / OpenSSL), floor **TLS 1.2**, `CryptoProvider` bootstrap in the binary, and an explicit provider choice.
- `docsrs-cli` is a **one-shot** HTTPS **client** (GET-only against a fixed host allowlist). It is not a TLS server, ACME endpoint, or mTLS workload identity agent.
- HTTP is implemented with **reqwest 0.13** (`default-features = false`, feature `rustls-no-provider`), which pulls `hyper-rustls` / `tokio-rustls` / **rustls 0.23.x** and no provider at all — the binary installs **ring** itself.
- Upstream rustls defaults to **aws-lc-rs** (+ optional post-quantum KX), and so does reqwest 0.13’s `rustls` feature — which is exactly why this product uses `rustls-no-provider` and installs **ring** explicitly in `main`.

## Decision

### 1. Single TLS stack
- **Only** rustls for product TLS. Never enable `native-tls`, `openssl` / `openssl-sys`, or dual stacks in the same binary.
- Direct dependency pin: `rustls` with `default-features = false` and features `std`, `tls12`, `ring` (floor **≥ 0.23.18** for the Acceptor panic advisory era; product is client-only but the floor still applies).
- The `ring` feature is the crypto provider named in section 2, so it is enabled on purpose; this line read **no crypto feature** until 2026-08-10, contradicting both the manifest and the next section of this same document.
- Roots: **direct** `webpki-roots` dependency, not a reqwest feature. reqwest 0.13 removed every webpki-roots feature, so leaving the anchor set to reqwest hands it to the OS store; `src/http/tls.rs` owns it instead.

### 2. Crypto provider is `ring`, and the no-C rule is a recorded non-conformance

The original decision selected `ring`. An earlier revision of this section
reversed that in favour of a pure-Rust provider, and this revision reverses it
back — because the pure-Rust experiment was abandoned **in the code** and this
document was left describing it. Between those two moments the normative TLS
document named a provider the binary never installed, and stated as a
Consequence that `cargo tree -i ring` must print nothing while `ring` sat in the
graph. That is worse than never amending it: a reader auditing TLS posture would
have concluded this product ships no C. Both languages of this ADR are now
checked against `src/main.rs` by a gate, so the pair cannot separate again.

The product rule that this CLI be self-contained and Rust-native is **not met**.
`ring` compiles `crypto/*.c` through `cc-rs`, so every non-host target needs a C
toolchain that can target it. The cost is real and is now bounded rather than
argued: on the maintainer host `x86_64-pc-windows-msvc` cross-checks clean
through `cargo-xwin` (so `#[cfg(windows)]` **is** type-checked), and on
2026-08-10 `zig` plus `cargo-zigbuild` — installable under `$HOME` with no root —
also built `x86_64-pc-windows-gnu` and `aarch64-unknown-linux-gnu` with `ring`
included.

That measurement narrowed the non-conformance rather than confirming it. **The
Apple targets are not blocked by `ring`:** zig compiles ring's C for both and the
build reaches the link step, where it fails on `CoreFoundation`, `Security` and
`SystemConfiguration` — Apple SDK frameworks pulled by `rustls-platform-verifier`
and `system-configuration`, which zig does not redistribute. Dropping the C
provider would not move those two rows. What `ring` still costs is a C toolchain
per cross target, and that cost is now bounded to what it actually is.

| Choice | Rationale |
|--------|-----------|
| **ring** (accepted, non-conformance recorded) | Compiles C, which the product rule forbids. Kept because both pure-Rust alternatives cost more than the rule buys: one narrows supported hardware, the other weakens certificate validation |
| **graviola** (rejected on measurement) | Pure Rust and formally verified, but requires x86_64 `adx` (Broadwell, 2015+). On this Haswell host it compiled, passed all offline tests, then aborted the process at the first live handshake — no offline suite can catch that |
| **rustls-rustcrypto** (rejected on measurement) | Pure Rust with no CPU floor, but pins `rustls-webpki ^0.102` carrying RUSTSEC-2026-0049/0098/0099/0104, plus `rsa` 0.9.10 with RUSTSEC-2023-0071 and no patched release. A TLS client may not trade a build-time dependency for weakened certificate validation |
| **aws-lc-rs** (rejected) | Compiles C, with a heavier cmake/nasm build than `ring`, so it loses on the same rule without winning anything |

- Reopen when `graviola` gains a non-`adx` fallback, or when `rustls-rustcrypto`
  moves to a patched `rustls-webpki`. Either event supplies a pure-Rust provider
  that neither narrows hardware support nor weakens validation.
- Do **not** reopen on maturity grounds alone. Both candidates left alpha; the
  blocking axes are hardware floor and advisories, and nothing else.
- The **binary** calls `rustls::crypto::ring::default_provider().install_default()`
  **once** at the top of `main`, **before** building the Tokio runtime or any
  HTTP client.
- Libraries / `docsrs_cli` lib code **must not** call `install_default` from the
  binary path (consumer/binary responsibility); the lib installs one only when
  no process default exists, for library callers and tests.
- Never enable a **second** provider feature on rustls in this product: `ring`
  is the one exception to the no-C rule and `aws-lc-rs` is banned outright in
  `deny.toml`. Two providers linked at once is how a build silently stops using
  the one this ADR describes.

### 3. Client configuration
- reqwest builder: `use_preconfigured_tls` with the `ClientConfig` built by `src/http/tls.rs`. This bullet described `.use_rustls_tls()` + `.min_tls_version(TLS_1_2)` for a month after the client stopped calling either, which was worse than merely stale: supplying a preconfigured config makes reqwest's `min_tls_version` **inert**, so a reader following this ADR would have set a floor that does nothing. The floor now lives where it has effect, in `with_protocol_versions(&[TLS13, TLS12])` on the config itself. A policy gate rejects any ADR naming a builder call `src/http/client.rs` does not make.
- Certificate validation always on: never `danger_accept_invalid_certs` / hostname bypass / custom `NoVerifier` in product code.
- No `KeyLog` / `SSLKEYLOGFILE` integration in product builds.
- **Not** `https_only(true)` on the reqwest client: offline wiremock uses `http://127.0.0.1` under the localhost test gate. Production hosts still require **https** via the shared origin allowlist (`is_allowed_origin_scheme_host`).
- TLS 1.2 remains enabled (`tls12` feature) because the product floor is 1.2 and public docs CDNs may still negotiate it; TLS 1.3 is used when the peer offers it.

### 4. Trust, proxy, multi-OS
- **Roots:** `webpki-roots` as a **direct** dependency, assembled into a `rustls::ClientConfig` by `src/http/tls.rs` and handed to reqwest through `use_preconfigured_tls`, so container and agent environments share a known Mozilla set. Not `rustls-platform-verifier`: it remains in the graph because reqwest's only public rustls gateway (`rustls-no-provider`) pulls it unconditionally, but no code path consults it.
- **This was lost once, silently (GAP-TLS-ROOTS-001).** reqwest 0.13 removed every webpki-roots feature, so the 0.12 upgrade moved the anchors to the operating system store while this bullet went on claiming the opposite — and `http_client_posture`, the string an operator reads to audit precisely this, printed `webpki-roots` from a hard-coded literal. The binary misreported its own trust anchors. A policy gate now derives the active source from `src/http/tls.rs` and rejects any document, and that constant, naming a different one.
- **Consequence of compiled-in roots:** a TLS-terminating proxy whose root exists only in the OS store is **not** trusted, and its handshake fails. That is the price of reproducibility and it is deliberate — an agent must not silently trust whatever the host happens to trust. Refresh the lockfile periodically for webpki-roots freshness, because the anchor set now ages with the build rather than with the operating system.
- **System proxy** (`HTTP_PROXY` / `HTTPS_PROXY` / `ALL_PROXY` / `NO_PROXY`) is honored via reqwest `system-proxy`. Proxy **routing** is an operator decision; proxy **trust** is not, per the bullet above. The product still applies the **target** host allowlist after proxy resolution.
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
- `cargo tree` must show **no** `native-tls`, `openssl` / `openssl-sys` or `aws-lc-rs`: `cargo tree -i native-tls`, `cargo tree -i openssl-sys` and `cargo tree -i aws-lc-rs` must all print nothing.
- `cargo tree -i ring` and `cargo tree -i cc` **do** print, and that is the recorded non-conformance, not a regression. A run where they print nothing means the provider changed and this ADR is stale — the condition this document was caught in once already.
- Doctor `http_client_posture` reports the provider, its maturity and its CPU requirement, so an operator can distinguish a hardware mismatch from a network failure without reading the source.
- A provider switch requires an ADR amendment and a CHANGELOG Security note. A switch that lands in the code without both is the failure mode this section was rewritten to undo, so the amendment is not paperwork — it is the only record that says which provider is live.
- Local `deny.toml` bans alternate TLS crates to fail closed on accidental feature pulls.

## Related
- ADR 0003 web-fetch scope · ADR 0004 threat model · `SECURITY.md` · `src/http/client.rs` · `src/main.rs` · `deny.toml`
- Gaps inventory: Camada M (rustls)
