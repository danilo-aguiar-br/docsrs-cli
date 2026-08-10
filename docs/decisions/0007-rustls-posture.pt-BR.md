[English](0007-rustls-posture.md)

# ADR 0007 — Postura TLS rustls do docsrs-cli

## Status
- Aceito (2026-07-19)

## Contexto
- Rules Rust exigem TLS **somente rustls** (sem `native-tls` / OpenSSL), piso **TLS 1.2**, bootstrap de `CryptoProvider` no binário e escolha explícita de provider.
- `docsrs-cli` é um **cliente** HTTPS **one-shot** (GET-only contra allowlist fixa). Não é servidor TLS, endpoint ACME nem agente mTLS/SPIFFE.
- HTTP usa **reqwest 0.13** (`default-features = false`, feature `rustls-no-provider`), que puxa `hyper-rustls` / `tokio-rustls` / **rustls 0.23.x** e provider nenhum — o binário instala o **ring** por conta própria.
- O default upstream do rustls é **aws-lc-rs** (+ KX pós-quântico opcional), e o da feature `rustls` do reqwest 0.13 também — que é exatamente por que este produto usa `rustls-no-provider` e instala o **ring** explicitamente no `main`.

## Decisão

### 1. Stack TLS única
- **Somente** rustls no produto. Nunca habilitar `native-tls`, `openssl` / `openssl-sys` ou stacks dual no mesmo binário.
- Pin direto: `rustls` com `default-features = false` e features `std`, `tls12`, `ring` (piso **≥ 0.23.18**; produto é só cliente, mas o floor vale).
- A feature `ring` é o crypto provider nomeado na seção 2, então está ligada de propósito; esta linha dizia **sem feature de cripto** até 2026-08-10, contradizendo o manifesto e a seção seguinte deste mesmo documento.
- Raízes: dependência **direta** de `webpki-roots`, nunca feature do reqwest. O reqwest 0.13 removeu toda feature de webpki-roots, então deixar o conjunto de âncoras a cargo do reqwest o entrega ao repositório do SO; quem o possui é o `src/http/tls.rs`.

### 2. O crypto provider é o `ring`, e a regra sem-C fica como não conformidade registrada

A decisão original escolheu `ring`. Uma revisão anterior desta seção reverteu
isso em favor de um provider puro Rust, e esta revisão reverte de volta — porque
o experimento puro Rust foi abandonado **no código** e este documento ficou
descrevendo o experimento. Entre os dois momentos, o documento normativo de TLS
nomeava um provider que o binário nunca instalou, e declarava como Consequência
que `cargo tree -i ring` deveria imprimir nada enquanto o `ring` estava no grafo.
Isso é pior que nunca ter emendado: quem auditasse a postura TLS concluiria que
este produto não embarca C nenhum. Os dois idiomas desta ADR passaram a ser
conferidos contra `src/main.rs` por um gate, então o par não se separa de novo.

A regra de produto que exige CLI auto-contida e rust-native **não é cumprida**.
O `ring` compila `crypto/*.c` via `cc-rs`, então todo alvo não-host exige um
toolchain C capaz de mirá-lo. O custo é real e agora é limitado em vez de
argumentado: no host do mantenedor o `x86_64-pc-windows-msvc` passa no
cross-check via `cargo-xwin` (logo o `#[cfg(windows)]` **é** type-checked), e em
2026-08-10 o `zig` mais o `cargo-zigbuild` — instaláveis sob `$HOME` sem root —
também construíram `x86_64-pc-windows-gnu` e `aarch64-unknown-linux-gnu` com o
`ring` incluído.

Essa medição estreitou a não conformidade em vez de confirmá-la. **Os alvos
Apple não são bloqueados pelo `ring`:** o zig compila o C do ring para os dois e
o build chega ao passo de link, onde falha em `CoreFoundation`, `Security` e
`SystemConfiguration` — frameworks do SDK da Apple puxados por
`rustls-platform-verifier` e `system-configuration`, que o zig não redistribui.
Remover o provider C não moveria essas duas linhas. O que o `ring` ainda custa é
um toolchain C por alvo cruzado, e esse custo agora está limitado ao que ele
realmente é.

| Escolha | Justificativa |
|---------|---------------|
| **ring** (aceito, não conformidade registrada) | Compila C, o que a regra de produto proíbe. Mantido porque as duas alternativas puras Rust custam mais do que a regra compra: uma estreita o hardware suportado, a outra enfraquece a validação de certificado |
| **graviola** (rejeitado por medição) | Puro Rust e formalmente verificado, mas exige `adx` em x86_64 (Broadwell, 2015+). Neste host Haswell ele compilou, passou em todos os testes offline e abortou o processo no primeiro handshake real — nenhuma suíte offline pega isso |
| **rustls-rustcrypto** (rejeitado por medição) | Puro Rust e sem piso de CPU, mas fixa `rustls-webpki ^0.102` com RUSTSEC-2026-0049/0098/0099/0104, mais `rsa` 0.9.10 com RUSTSEC-2023-0071 e sem versão corrigida. Um cliente TLS não pode trocar dependência de build por validação de certificado enfraquecida |
| **aws-lc-rs** (rejeitado) | Compila C, com build cmake/nasm mais pesado que o `ring`, então perde na mesma regra sem ganhar nada |

- Reabrir quando o `graviola` ganhar fallback sem `adx`, ou quando o
  `rustls-rustcrypto` migrar para um `rustls-webpki` corrigido. Qualquer um dos
  dois entrega provider puro Rust sem estreitar hardware nem enfraquecer
  validação.
- **Não** reabrir por maturidade. Os dois candidatos saíram de alpha; os eixos
  que bloqueiam são piso de hardware e advisories, e nada mais.
- O **binário** chama `rustls::crypto::ring::default_provider().install_default()`
  **uma vez** no topo de `main`, **antes** do runtime Tokio e de qualquer cliente
  HTTP.
- A lib `docsrs_cli` **não** chama `install_default` pelo caminho do binário
  (responsabilidade do consumidor); ela instala um só quando não existe default
  de processo, para chamadores de biblioteca e testes.
- Nunca habilitar um **segundo** provider como feature do rustls neste produto:
  o `ring` é a única exceção à regra sem-C e o `aws-lc-rs` está banido no
  `deny.toml`. Dois providers linkados ao mesmo tempo é como um build deixa de
  usar, em silêncio, o provider que esta ADR descreve.

### 3. Configuração de cliente
- Builder reqwest: `use_preconfigured_tls` com o `ClientConfig` montado por `src/http/tls.rs`. Este item descreveu `.use_rustls_tls()` + `.min_tls_version(TLS_1_2)` por um mês depois de o cliente parar de chamar ambos, e isso era pior que apenas desatualizado: entregar um config pré-montado torna o `min_tls_version` do reqwest **inerte**, então quem seguisse esta ADR definiria um piso que não faz nada. O piso agora vive onde tem efeito, em `with_protocol_versions(&[TLS13, TLS12])` no próprio config. Um gate de política reprova qualquer ADR que nomeie chamada de builder ausente de `src/http/client.rs`.
- Validação de certificado sempre ligada: nunca `danger_accept_invalid_*` / bypass de hostname no produto.
- Sem `KeyLog` / `SSLKEYLOGFILE` em builds de produto.
- **Sem** `https_only(true)` no cliente reqwest: wiremock offline usa `http://127.0.0.1` sob gate de teste. Hosts de produção ainda exigem **https** via allowlist de origem.
- TLS 1.2 permanece (`tls12`) porque o piso do produto é 1.2; TLS 1.3 quando o peer oferecer.

### 4. Confiança, proxy, multi-OS
- **Raízes:** `webpki-roots` como dependência **direta**, montada num `rustls::ClientConfig` pelo `src/http/tls.rs` e entregue ao reqwest via `use_preconfigured_tls`, para ambientes de container e de agente compartilharem o conjunto Mozilla conhecido. Não `rustls-platform-verifier`: ele permanece no grafo porque o único portão público do reqwest para o rustls (`rustls-no-provider`) o puxa sem condicional, mas nenhum caminho de código o consulta.
- **Isso já foi perdido uma vez, em silêncio (GAP-TLS-ROOTS-001).** O reqwest 0.13 removeu toda feature de webpki-roots, então a atualização vinda da 0.12 moveu as âncoras para o repositório do sistema operacional enquanto este item seguia afirmando o contrário — e o `http_client_posture`, a string que o operador lê para auditar justamente isso, imprimia `webpki-roots` a partir de um literal fixo. O binário reportava errado as próprias âncoras de confiança. Um gate de política agora deriva a fonte ativa do `src/http/tls.rs` e reprova qualquer documento, e aquela constante, que nomeie outra.
- **Consequência de raízes compiladas:** um proxy que termina TLS cuja raiz existe só no repositório do SO **não** é confiado, e o handshake dele falha. Esse é o preço da reprodutibilidade e é deliberado — um agente não pode confiar em silêncio no que o host por acaso confia. Atualize o lock periodicamente pela frescura do webpki-roots, porque o conjunto de âncoras agora envelhece com o build e não com o sistema operacional.
- **Proxy de sistema** honrado via `system-proxy`. O **roteamento** por proxy é decisão do operador; a **confiança** no proxy não é, conforme o item acima. A allowlist ainda vale para o host **alvo**.
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
- `cargo tree` sem `native-tls`, `openssl` / `openssl-sys` nem `aws-lc-rs`: `cargo tree -i native-tls`, `cargo tree -i openssl-sys` e `cargo tree -i aws-lc-rs` não podem imprimir nada.
- `cargo tree -i ring` e `cargo tree -i cc` **imprimem**, e isso é a não conformidade registrada, não uma regressão. Uma execução em que eles não imprimem nada significa que o provider mudou e esta ADR está velha — exatamente a condição em que este documento já foi flagrado uma vez.
- O doctor `http_client_posture` reporta o provider, sua maturidade e seu requisito de CPU, para o operador distinguir incompatibilidade de hardware de falha de rede sem ler o fonte.
- Troca de provider exige emenda a este ADR e nota Security no CHANGELOG. Uma troca que entra no código sem as duas é o modo de falha que esta seção foi reescrita para desfazer, então a emenda não é papelada — é o único registro que diz qual provider está vivo.
- `deny.toml` local bloqueia crates TLS alternativos.

## Relacionados
- ADR 0003 · ADR 0004 · `SECURITY.md` · `src/http/client.rs` · `src/main.rs` · `deny.toml`
- Inventário: Camada M (rustls)
