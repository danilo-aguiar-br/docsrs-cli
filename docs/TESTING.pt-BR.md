[English](TESTING.md)

# Testes
> **Dogfood 1.3.0:** invoque sempre `./target/release/docsrs-cli` (ou `cargo run --release --`) para checks de produto. Instalação no PATH pode atrasar o tree (GAP-W-005). Prefira `cargo audit --no-fetch` se o index do advisory DB travar (GAP-W-010). Instale deste tree com `cargo install --path . --force` (crates.io pode listar versão antiga — GAP-X-009).

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
cargo test --locked --test policy_gates
cargo test --locked --test e2e_offline
cargo test --locked --test http_docs_rs
cargo test --locked --test golden_render
cargo test --locked --test golden_diff
cargo test --locked --test signal_term
cargo test --locked --test lib_dispatch
cargo test --locked --test etd_target_designation
```

## Perfis Live de Rede
```bash
cargo test --locked --test network_live -- --ignored
```
- `#[ignore]` é o único gate; `cargo test` puro nunca abre socket externo
- Rode isto só quando pretender atingir hosts públicos
- A suíte exigia `DOCSRS_CLI_NETWORK_TESTS=1` além do `--ignored`
- Sem ela todo teste retornava cedo e ainda era contado como aprovado
- Um segundo gate que esvazia o primeiro em silêncio é pior que gate nenhum

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
./scripts/check-all.sh
```
- O `check-all.sh` roda `cargo test --test policy_gates` (gates de i18n, anti-env, flags de agente, postura TLS, trilha do gaps e doc-versus-manifesto), depois descobre todo `scripts/check-*.sh` no diretório e falha fechado: `check-docs.sh` (`RUSTDOCFLAGS='-D warnings' cargo doc`), `check-supply.sh` (`cargo deny` + `cargo audit`), `check-targets.sh` (cross `cargo check`, cobertura zero em não-Linux reprova)
- O `check-docs.sh` existe porque a suíte de política lê o fonte como texto e não sabe se um link de documentação resolve. Um gate de pré-publish achou 83 links intra-doc quebrados em 29 de 36 arquivos com todos os outros gates verdes (GAP-DOC-LINKS-001)
- Os gates de política são Rust, em `tests/policy_gates.rs`. Eram um script bash de 540 linhas envolvendo 260 linhas de Python inline, o que quebrava a regra full-stack Rust e tornava a política de uma CLI de três plataformas verificável em uma só
- Acrescente `--allow-no-cross` em host sem toolchain mingw ou Apple; o msvc ainda passa no cross-check ali via `cargo-xwin`
- O `check-all.sh` precisa do `fd` no PATH para descobrir os gates irmãos, e aborta com exit 1 quando ele falta
- Esse abort é deliberado: sem descoberta a execução reportaria verde pulando todo gate que nunca encontrou
- Rode a suíte de política direto com `cargo test --locked --test policy_gates` em host sem `fd`

## Smoke live humano (pré-release, sem CI)
```bash
cargo build --release
./scripts/smoke-live.sh
```
- Usa dirs temp `--config-dir` / `--cache-dir` (XDG via flags; sem knobs de produto por env)
- Afirma eco de page-token, `budget` não retryable, timeout 0 fail-closed, version do binário (dogfood `./target/release/docsrs-cli` para 1.3.0)
- Exige rede; fail-open se hosts estiverem fora

## Variáveis de Ambiente
- Nenhuma variável de ambiente controla teste algum; `#[ignore]` controla, e a suíte `policy_gates` varre `tests/` para manter assim
- Testes de integração isolam storage com `--config-dir` / `--cache-dir` (tempdir)
- Testes live de rede são `#[ignore]`; habilite com `cargo test -- --ignored`
- **Não** há knobs de produto por env para origins, retries, UA, timeouts ou allowlist de loopback (use CLI/XDG)

## Troubleshooting
- Se testes live falharem, confirme rede e disponibilidade do host primeiro
- Se testes offline falharem, não habilite flags live para esconder a falha
- Se golden tests falharem, inspecione mudanças intencionais de contrato de render
- Se origins mock forem recusados, defina `allow_loopback = true` no `config.toml` e/ou passe `--allow-loopback`
- Se testes de sinal flakarem, garanta que nenhum processo externo roube o terminal controlador
