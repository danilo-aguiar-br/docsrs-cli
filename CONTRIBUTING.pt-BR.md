[English](CONTRIBUTING.md)

# Contribuindo
## Bem-vindo
- Obrigado por melhorar o docsrs-cli
- Mantenha diffs cirúrgicos e focados no produto
- Não adicione telemetria de produto


## Início Rápido
```bash
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets
```


## Setup de Desenvolvimento
- Instale Rust 1.88 ou mais novo via rustup
- Clone o repositório e trabalhe na raiz do checkout
- Prefira `cargo run -q -- <args>` durante o desenvolvimento local
- Valide rustdoc com:
```bash
RUSTDOCFLAGS='-D missing_docs -D rustdoc::broken_intra_doc_links' cargo doc --no-deps --locked
```


## Estratégia de Branches
- Crie branch a partir de `main` em toda mudança
- Use nomes curtos como `fix/rate-limit` ou `docs/agents`
- Mantenha um único concern por branch


## Convenção de Commits
- Escreva subjects no imperativo
- Explique o porquê no body quando o diff não for óbvio
- Nunca adicione trailers `Co-authored-by`


## Processo de PR
- Abra um pull request focado com resumo claro
- Linke issues relacionadas quando existirem
- Espere revisão de estabilidade de contrato, pares de docs e testes
- Não publique no crates.io nem faça push de tags de release sem autorização do mantenedor


## Testes
- Rode a suíte completa com `cargo test --locked --all-targets`
- Testes live de rede ficam atrás de `DOCSRS_CLI_NETWORK_TESTS`
- Testes live de stdlib ficam atrás de `DOCSRS_CLI_STDLIB_NETWORK_TESTS`
- Veja [docs/TESTING.pt-BR.md](docs/TESTING.pt-BR.md)


## Documentação
- Atualize docs públicos em inglês e português na mesma entrega
- Mantenha tokens técnicos sem tradução
- Indexe todo JSON schema novo em `docs/schemas/README.md`
- Espelhe conhecimento operacional em `skills/docsrs-cli-en` e `skills/docsrs-cli-pt`


## Reportar Bugs
- Abra issue com comando, flags, exit code e stderr redigido
- Inclua OS, versão do Rust e saída de `docsrs-cli version --json`
- Nunca cole segredos em issues


## Solicitar Features
- Descreva o workflow de agente que a feature desbloqueia
- Prefira contratos JSON estáveis a prosa humana
- Declare se a mudança permanece one-shot compatível


## Processo de Release
- Atualize a versão em `Cargo.toml` com SemVer
- Atualize `CHANGELOG.md` e `CHANGELOG.pt-BR.md`
- Sincronize docs públicos bilíngues, `llms*.txt` e skills com a nova superfície de comandos
- Atualize a linha de versões suportadas em `SECURITY.md` / `SECURITY.pt-BR.md`
- Confirme que `docs/MIGRATION.pt-BR.md` cobre os breakings do release
- Rode gates offline: `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test --locked --all-targets`
- Smoke live humano opcional: `./scripts/smoke-live.sh` (sem CI)
- Faça tag só após aprovação do mantenedor
- Publique no crates.io só com autorização explícita do mantenedor


## Reconhecimento
- Contribuidores aparecem no histórico git e nas notas de release quando relevante
- Repórteres de segurança podem entrar no Hall of Fame do SECURITY após o fix


## Perguntas
- Prefira GitHub issues para perguntas de produto
- Mail de segurança vai apenas para daniloaguiarbr@proton.me
