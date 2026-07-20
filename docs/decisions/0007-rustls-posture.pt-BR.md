[English](0007-rustls-posture.md)

# ADR 0007 — Postura TLS rustls do docsrs-cli

## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust exigem TLS **somente rustls** (sem `native-tls` / OpenSSL), piso **TLS 1.2**, bootstrap de `CryptoProvider` no binário e escolha explícita de provider.
- `docsrs-cli` é um **cliente** HTTPS **one-shot** (GET-only contra allowlist fixa). Não é servidor TLS, endpoint ACME nem agente mTLS/SPIFFE.
- HTTP usa **reqwest 0.12** (`default-features = false`, feature `rustls-tls`), que puxa `hyper-rustls` / `tokio-rustls` / **rustls 0.23.x** e, via `__rustls-ring`, o provider **ring**.
- O default upstream do rustls é **aws-lc-rs** (+ KX pós-quântico opcional). O path `rustls-tls` do reqwest 0.12 habilita **ring** de propósito.

## Decisão

### 1. Stack TLS única
- **Somente** rustls no produto. Nunca habilitar `native-tls`, `openssl` / `openssl-sys` ou stacks dual no mesmo binário.
- Pin direto: `rustls` com `default-features = false` e features `std`, `tls12`, `ring` (piso **≥ 0.23.18**; produto é só cliente, mas o floor vale).
- Transitivo: `webpki-roots` via `rustls-tls-webpki-roots` (raízes Mozilla; reprodutível em containers).

### 2. Crypto provider = ring (Opção A)
| Escolha | Justificativa |
|---------|---------------|
| **ring** (aceito) | Alinha com reqwest 0.12 `rustls-tls` / `__rustls-ring`; um único provider no grafo; portátil Linux/macOS/Windows e builds musl sem cmake/nasm do aws-lc |
| **aws-lc-rs** (rejeitado por ora) | Preferível para defaults PQ upstream, mas muda o grafo de features e encarece cross/musl para uma CLI de docs one-shot |

- O **binário** chama `rustls::crypto::ring::default_provider().install_default()` **uma vez** no topo de `main`, **antes** do runtime Tokio e de qualquer cliente HTTP.
- A lib `docsrs_cli` **não** chama `install_default` (responsabilidade do consumidor/binário).
- Nunca habilitar `ring` e `aws_lc_rs` juntos neste produto.

### 3. Configuração de cliente
- Builder reqwest: `.use_rustls_tls()` + `.min_tls_version(TLS_1_2)`.
- Validação de certificado sempre ligada: nunca `danger_accept_invalid_*` / bypass de hostname no produto.
- Sem `KeyLog` / `SSLKEYLOGFILE` em builds de produto.
- **Sem** `https_only(true)` no cliente reqwest: wiremock offline usa `http://127.0.0.1` sob gate de teste. Hosts de produção ainda exigem **https** via allowlist de origem.
- TLS 1.2 permanece (`tls12`) porque o piso do produto é 1.2; TLS 1.3 quando o peer oferecer.

### 4. Confiança, proxy, multi-OS
- **Raízes:** webpki-roots (não `rustls-platform-verifier`) para ambientes de container/agente compartilharem o conjunto Mozilla. Atualizar o lock periodicamente.
- **Proxy de sistema** honrado via `system-proxy`. Proxy corporativo que termina TLS é **decisão de confiança do operador**; a allowlist ainda vale para o host **alvo**.
- Mesmo stack rustls+ring em Linux, macOS e Windows (sem OpenSSL em runtime).

### 5. One-shot · memória · paralelismo
- **One-shot:** handshake TLS por invocação é esperado (sem daemon de session tickets).
- **Pool:** idle minúsculo (constantes nomeadas) — anti-daemon.
- **Memória:** caps de body **após** TLS; produto não guarda chaves privadas nem client certs.
- **Paralelismo:** Tokio multi-thread; `HttpClient` com `&self`; sem mutação global de TLS após o bootstrap.

### 6. Não-objetivos (N/A)
| Não-objetivo | Por quê |
|--------------|---------|
| Servidor TLS / `Acceptor` | Só cliente; RUSTSEC-2024-0399 não aplica ao código de produto |
| mTLS / cert cliente / SPIFFE / HSM / ACME | Origens públicas de docs |
| ECH / FIPS / prefer-post-quantum | Sem requisito; PQ exigiria migração aws-lc (reabrir ADR) |
| QUIC / HTTP/3 / gRPC / WebSocket / DTLS | GET HTTP/1.1+h2 (ADR 0003) |
| Gates CI / cosign / SBOM | CI/CD fora do escopo; `deny.toml` local para mantenedores |
| Knobs TLS via `DOCSRS_CLI_TLS_*` | Paths usam flags CLI + ProjectDirs XDG; postura TLS é compile-time + código |

## Consequências
- `cargo tree` sem `native-tls` / `openssl` e com **um** provider crypto sob rustls (**ring**).
- Doctor reporta `provider=ring` e o floor rustls.
- Upgrade major de rustls ou troca de provider exige emenda a este ADR e nota Security no CHANGELOG.
- `deny.toml` local bloqueia crates TLS alternativos.

## Relacionados
- ADR 0003 · ADR 0004 · `SECURITY.md` · `src/http/client.rs` · `src/main.rs` · `deny.toml`
- Inventário: Camada M (rustls)
