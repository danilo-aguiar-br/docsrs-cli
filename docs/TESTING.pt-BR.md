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
```


## Perfis Live de Rede
```bash
DOCSRS_CLI_NETWORK_TESTS=1 cargo test --locked --test network_live
DOCSRS_CLI_STDLIB_NETWORK_TESTS=1 cargo test --locked --test network_live
```
- Deixe essas vars unset nos loops locais default
- Use-as só quando pretender atingir hosts públicos


## Perfis de CI
- Este repositório não envia workflows GitHub Actions por política
- Validação local é o gate antes do publish
- Gate local recomendado:
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
```


## Variáveis de Ambiente
- `DOCSRS_CLI_NETWORK_TESTS` habilita testes live de crates.io e docs.rs
- `DOCSRS_CLI_STDLIB_NETWORK_TESTS` habilita testes live de doc.rust-lang.org
- `DOCSRS_CLI_ALLOW_LOCALHOST` permite origins mock locais em testes controlados
- `DOCSRS_CLI_CRATES_IO_ORIGIN` e `DOCSRS_CLI_DOCS_RS_ORIGIN` sobrescrevem bases para mocks
- `DOCSRS_CLI_HOME` isola config e cache durante testes


## Troubleshooting
- Se testes live falharem, confirme rede e disponibilidade do host primeiro
- Se testes offline falharem, não habilite flags live para esconder a falha
- Se golden tests falharem, inspecione mudanças intencionais de contrato de render
- Se testes de sinal flakarem, garanta que nenhum processo externo roube o terminal controlador
