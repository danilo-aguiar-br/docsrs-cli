[English](TESTING.md)

# Testes
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
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live
```
- Deixe essas vars unset nos loops locais default
- Use-as só quando pretender atingir hosts públicos

## Mocks Offline Com config.toml
- Origins de produto não são definidos via `DOCSRS_CLI_CRATES_IO_ORIGIN` ou `DOCSRS_CLI_DOCS_RS_ORIGIN`
- Aponte testes a wiremock escrevendo TOML sob um home sandbox:
```bash
export DOCSRS_CLI_HOME=/tmp/docsrs-test-home
export DOCSRS_CLI_ALLOW_LOCALHOST=1
mkdir -p "$DOCSRS_CLI_HOME/config"
cat > "$DOCSRS_CLI_HOME/config/config.toml" <<'TOML'
crates_io_origin = "http://127.0.0.1:PORT"
docs_rs_origin = "http://127.0.0.1:PORT"
rate_limit_delay_ms = 0
max_retries = 1
TOML
```
- `DOCSRS_CLI_HOME` isola config e cache para o processo
- `DOCSRS_CLI_ALLOW_LOCALHOST` permite origins mock locais em testes controlados
- Mantenha knobs de produto (timeouts, retries, UA, origins) só em flags ou TOML

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
- Afirma eco de page-token, `budget` não retryable, timeout 0 fail-closed, version 0.1.2
- Exige rede; fail-open se hosts estiverem fora

## Variáveis de Ambiente
- `DOCSRS_CLI_NETWORK_TESTS` habilita testes live de crates.io e docs.rs
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` habilita testes live de doc.rust-lang.org
- `DOCSRS_CLI_ALLOW_LOCALHOST` permite origins mock locais em testes controlados
- `DOCSRS_CLI_HOME` isola config e cache durante testes
- Overrides de path `DOCSRS_CLI_CONFIG_DIR` e `DOCSRS_CLI_CACHE_DIR` permanecem disponíveis
- Não há knobs de produto por env para origins, retries, UA ou timeouts em 0.1.x

## Troubleshooting
- Se testes live falharem, confirme rede e disponibilidade do host primeiro
- Se testes offline falharem, não habilite flags live para esconder a falha
- Se golden tests falharem, inspecione mudanças intencionais de contrato de render
- Se origins mock forem recusados, defina `DOCSRS_CLI_ALLOW_LOCALHOST=1` e coloque origins em `config.toml`
- Se testes de sinal flakarem, garanta que nenhum processo externo roube o terminal controlador
