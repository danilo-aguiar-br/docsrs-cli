[English](CROSS_PLATFORM.md)

# Multiplataforma
> Um binário Rust, seis targets documentados, zero hacks de plataforma no argv.

## A Dor Que Você Já Conhece
- Separadores de path hardcoded quebram no Windows
- Stacks TLS nativas diferem por host e surpreendem agentes
- Semântica de sinais difere entre Unix e consoles Windows

## Matriz de Suporte
| Target | Plataforma | build docs.rs | cross-check local |
|--------|------------|---------------|-------------------|
| `x86_64-unknown-linux-gnu` | Linux glibc | default-target | sim |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | não (ring compila C) | sim (via cargo-zigbuild) |
| `x86_64-pc-windows-gnu` | Windows GNU | não (ring compila C) | sim (via cargo-zigbuild) |
| `x86_64-apple-darwin` | macOS Intel | não (ring compila C) | não (frameworks do SDK Apple) |
| `aarch64-apple-darwin` | macOS Apple Silicon | não (ring compila C) | não (frameworks do SDK Apple) |
| `x86_64-pc-windows-msvc` | Windows MSVC | não (ring compila C) | sim (via cargo-xwin) |

- A coluna `build docs.rs` dizia `sim` para quatro alvos enquanto o `Cargo.toml` trazia `targets = []`
- Esses quatro produziam 404 permanentes, e o manifesto estava certo enquanto a matriz estava errada
- A coluna `build docs.rs` deriva de uma causa: o `ring` compila C, e a imagem do docs.rs não traz cross-compiler C
- A coluna `cross-check local` já não deriva dessa causa, e afirmar que derivava sobreviveu um dia à medição: o `zig` fornece o cross-compiler C que faltava
- Das cinco linhas não-host, três agora compilam **com** o `ring`: `aarch64-unknown-linux-gnu`, `x86_64-pc-windows-gnu` e `x86_64-pc-windows-msvc`
- As duas linhas Apple são o resto, e param no passo de link, não na compilação do C
- Medido em 2026-08 com um provider puro Rust no lugar, os quatro passaram no cross-check sem compilador instalado
- Essa medição é o custo do GAP-TOOLCHAIN-001, e é por isso que a coluna diz `não` e não `sem suporte`
- `cross-check local` vem do `scripts/check-targets.sh`, que falha fechado com cobertura zero
- A linha do msvc dizia `não (sem SDK MSVC aqui)` até 2026-08-10, quando o gate parou de sondar `lib.exe` — archiver que não pode existir em Linux — e passou a sondar `cargo-xwin`, `clang-cl` e `llvm-lib`, que já estavam instalados. Quem estava errado era a linha, não o host
- Portanto o `#[cfg(windows)]` É type-checked aqui, e em 2026-08-10 o `zig` mais o `cargo-zigbuild` — ambos instaláveis sob `$HOME` sem root — levaram `x86_64-pc-windows-gnu` e `aarch64-unknown-linux-gnu` de pulados a compilados, com o ring junto
- **As linhas Apple NÃO são bloqueadas pelo `ring`, e este documento afirmava que eram.** O zig compila o C do ring para os dois alvos Apple e o build chega ao LINK, onde falha com `unable to find framework 'CoreFoundation' / 'Security' / 'SystemConfiguration'`
- Esses frameworks são puxados pelo `rustls-platform-verifier` (via `security-framework` e `core-foundation`) e pelo `system-configuration` do reqwest — e o primeiro é o crate que o reqwest força na árvore sem condicional pelo `rustls-no-provider`, que nenhum caminho de código deste produto consulta
- O bloqueio Apple é portanto de frameworks do SDK Apple em tempo de link, que o zig não redistribui; remover o `ring` não moveria essa linha

## Ferramentas de Host Que Decidem a Cobertura
- Cobertura de alvo é propriedade do HOST, nunca deste crate; estas ferramentas a decidem
- `zig` e `cargo-zigbuild` dirigem `x86_64-pc-windows-gnu` e `aarch64-unknown-linux-gnu`
- `cargo-xwin`, `clang-cl` e `llvm-lib` dirigem `x86_64-pc-windows-msvc`
- `oa64-clang` é a sonda dos dois alvos Apple, e exige um SDK Apple real por trás
- Todas instalam sob `$HOME` sem root, exceto o próprio SDK Apple
- O `cross_checked` conta apenas os quatro alvos não-Linux, então `aarch64-unknown-linux-gnu` nunca o eleva
- Remova o `zig` e o `cross_checked` cai de 2 para 1 sem nenhuma mudança neste repositório, porque o `zig` cobre exatamente um alvo contado
- Um gate de política deriva esta lista de `cross_tools_for()` para que as duas não divirjam

## Postura TLS
- O provider de cripto é o `ring`, que compila C e portanto contradiz a regra rust-native do produto
- As duas alternativas puras Rust foram remedidas em 2026-08-10 e seguem rejeitadas
- O `graviola` exige `adx`, `bmi2` e `avx2` em x86_64, que o próprio README dele data em ~2014
- O `rustls-graviola` 0.4.0 vem do autor do rustls, então a barreira é alcance de CPU, nunca confiança
- O `rustls-rustcrypto` nunca saiu do `0.0.2-alpha`, e um alpha não é dependência de TLS
- O `rustls-webpki` carrega RUSTSEC-2026-0098 e RUSTSEC-2026-0104 na mesma janela
- Trocar dependência de build por validação de certificado enfraquecida não é troca que um cliente TLS possa fazer
- `docsrs-cli doctor --json` imprime o provider e por que o caminho puro Rust está bloqueado

## Notas Linux
- Config default em `$XDG_CONFIG_HOME/docsrs-cli` ou `~/.config/docsrs-cli`
- Cache default em `$XDG_CACHE_HOME/docsrs-cli` ou `~/.cache/docsrs-cli`
- Modos privados preferem dirs `0o700` e arquivos `0o600`
- SIGINT é 130; SIGTERM e SIGHUP são 143

## Notas macOS
- Paths vêm do layout de plataforma do crate `directories`
- TLS é apenas rustls (`provider=ring`); sem OpenSSL, mas o `ring` compila C (ADR 0007)
- Completions funcionam para bash, zsh e fish comuns no macOS

## Notas Windows
- Use completions PowerShell via `docsrs-cli completions powershell`
- Ctrl+C mapeia para exit 130
- Ctrl+Break e fechamento de console mapeiam para exit 143
- Cache e config herdam ACLs do pai em vez de modos Unix

## Containers
- Instale com `cargo install docsrs-cli --locked` no build da imagem
- Passe `--config-dir` / `--cache-dir` para volumes graváveis (ou confie no XDG/AppData sob home gravável)
- Forneça raízes CA para o rustls validar HTTPS público
- Prefira usuários non-root com home gravável ou dirs explícitos de config/cache
- Knobs de produto vêm só de flags e `config.toml` (nunca env de produto `DOCSRS_CLI_*`)

## Suporte a Shell
- Completions: bash, zsh, fish, elvish, powershell, power-shell
- Completions emitem scripts de shell brutos por default mesmo em non-TTY
- Passe `--json` só quando quiser envelope JSON para completions
- Markdown humano é o default em TTY para os demais comandos
- JSON é escolhido automaticamente em pipes non-TTY para comandos de rede e meta

## Paths de Arquivo e XDG
- Nunca hardcode separadores; a CLI usa `PathBuf`
- Precedência de path:
  1. Flags CLI `--config-dir` / `--cache-dir`
  2. ProjectDirs / XDG (Linux), AppData (Windows), Library (macOS)
- Knobs de produto (timeouts, retries, UA, origins, cache TTL, …) usam só flags + TOML
- Knobs e paths de produto não são lidos de variáveis `DOCSRS_CLI_*`
- `config path --json` imprime a camada vencedora para auditorias

## Performance por Target
- Trabalho de rede I/O-bound domina o wall time em todos os targets
- Offload de parse CPU usa `spawn_blocking` sob o orçamento de concorrência
- Scans de hit são sequenciais em todo target e não puxam crate de paralelismo de dados
- Comportamento de cache em disco é idêntico nas plataformas suportadas
- Semântica de `cache_hit` é a mesma em cada classe de host

## Agentes Validados por Plataforma
- Qualquer agente que execute binário e parseie JSON funciona em todos os targets
- Use as skills empacotadas como fonte operacional de verdade
- Valide offline com `docsrs-cli doctor --json` em cada classe de host que você envia
- Valide conectividade live com `docsrs-cli doctor --online --json` em cada classe de host
- Confirme que `docsrs-cli version --json` reporta `1.3.0` (ou 1.3.x mais novo) após o deploy
