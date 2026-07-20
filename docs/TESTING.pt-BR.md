[English](TESTING.md)

# Testes
> **Dogfood 1.2.0:** invoque sempre `./target/release/docsrs-cli` (ou `cargo run --release --`) para checks de produto. Instalação no PATH pode atrasar o tree (GAP-W-005). Prefira `cargo audit --no-fetch` se o index do advisory DB travar (GAP-W-010). Instale deste tree com `cargo install --path . --force` (crates.io pode listar versão antiga — GAP-X-009).

## Por que Testes Categorizados
- Testes unitários e de integração offline devem ficar verdes sem rede
- Testes live de rede são intencionais e gated
- Testes de sinal e smoke de CLI protegem o contrato de agente

## Categorias de Teste
- Unit tests dentro de `src/**` para lógica pura
- Integration tests em `tests/` com wiremock e fixtures offline
- Golden render e golden diff para formatos estáveis de saída
- Smoke de CLI para argv e comportamento de exit
- Testes de sinal para cancel e terminate
- Testes live opcionais de rede para crates.io, docs.rs e stdlib

## Como Rodar
```bash
cargo test --locked --all-targets
cargo test --locked --test cli_smoke
cargo test --locked --test e2e_offline
cargo test --locked --test http_integration
cargo test --locked --test golden_render
cargo test --locked --test golden_diff
cargo test --locked --test signal_term
cargo test --locked --test lib_dispatch
```

## Perfis Live de Rede
```bash
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live -- --ignored
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live -- --ignored
```
- Deixe essas vars unset nos loops locais default
- Use-as só quando pretender atingir hosts públicos

## Mocks Offline Com config.toml
- Origins de produto não são definidos via env vars
- Aponte testes a wiremock escrevendo TOML sob um home sandbox (ou use `--config-dir` + `--allow-loopback`):
```bash
CFG="$(mktemp -d)"
CACHE="$(mktemp -d)"
cat > "$CFG/config.toml" <<'TOML'
allow_loopback = true
crates_io_origin = "http://127.0.0.1:PORT"
docs_rs_origin = "http://127.0.0.1:PORT"
rate_limit_delay_ms = 0
max_retries = 1
TOML
docsrs-cli --allow-loopback --config-dir "$CFG" --cache-dir "$CACHE" doctor --json
```
- Isole config/cache com `--config-dir` / `--cache-dir` (produto nunca lê env de path)
- Mocks em loopback exigem `allow_loopback = true` no TOML e/ou CLI `--allow-loopback` (nunca env; ADR 0009)
- Mantenha knobs de produto (timeouts, retries, UA, origins, loopback) só em flags ou TOML

## Perfis de CI
- Este repositório não envia workflows GitHub Actions por política
- Validação local é o gate antes do publish
- Gate local recomendado:
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
cargo build --release
```

## Smoke live humano (pré-release, sem CI)
```bash
cargo build --release
./scripts/smoke-live.sh
```
- Usa dirs temp `--config-dir` / `--cache-dir` (XDG via flags; sem knobs de produto por env)
- Afirma eco de page-token, `budget` não retryable, timeout 0 fail-closed, version do binário (dogfood `./target/release/docsrs-cli` para 1.2.0)
- Exige rede; fail-open se hosts estiverem fora

## Variáveis de Ambiente
- `DOCSRS_CLI_NETWORK_TESTS` habilita testes live de crates.io e docs.rs (só harness)
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` habilita testes live de doc.rust-lang.org (só harness)
- Testes de integração isolam storage com `--config-dir` / `--cache-dir` (tempdir)
- Testes live de rede são `#[ignore]`; habilite com env de harness + `cargo test -- --ignored`
- **Não** há knobs de produto por env para origins, retries, UA, timeouts ou allowlist de loopback (use CLI/XDG)

## Troubleshooting
- Se testes live falharem, confirme rede e disponibilidade do host primeiro
- Se testes offline falharem, não habilite flags live para esconder a falha
- Se golden tests falharem, inspecione mudanças intencionais de contrato de render
- Se origins mock forem recusados, defina `allow_loopback = true` no `config.toml` e/ou passe `--allow-loopback`
- Se testes de sinal flakarem, garanta que nenhum processo externo roube o terminal controlador
