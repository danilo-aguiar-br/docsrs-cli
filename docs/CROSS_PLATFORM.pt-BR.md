[English](CROSS_PLATFORM.md)

# Multiplataforma

> Um binário Rust, cinco targets documentados, zero hacks de plataforma no argv.


## A Dor Que Você Já Conhece
- Separadores de path hardcoded quebram no Windows
- Stacks TLS nativas diferem por host e surpreendem agentes
- Semântica de sinais difere entre Unix e consoles Windows


## Matriz de Suporte

| Target | Plataforma | build docs.rs |
|--------|------------|---------------|
| `x86_64-unknown-linux-gnu` | Linux glibc | default-target |
| `aarch64-unknown-linux-gnu` | Linux ARM64 | yes |
| `x86_64-apple-darwin` | macOS Intel | yes |
| `aarch64-apple-darwin` | macOS Apple Silicon | yes |
| `x86_64-pc-windows-msvc` | Windows MSVC | yes |


## Notas Linux
- Config default em `$XDG_CONFIG_HOME/docsrs-cli` ou `~/.config/docsrs-cli`
- Cache default em `$XDG_CACHE_HOME/docsrs-cli` ou `~/.cache/docsrs-cli`
- Modos privados preferem dirs `0o700` e arquivos `0o600`
- SIGINT é 130; SIGTERM e SIGHUP são 143


## Notas macOS
- Paths vêm do layout de plataforma do crate `directories`
- TLS é apenas rustls; sem dependência OpenSSL em runtime
- Completions funcionam para bash, zsh e fish comuns no macOS


## Notas Windows
- Use completions PowerShell via `docsrs-cli completions powershell`
- Ctrl+C mapeia para exit 130
- Ctrl+Break e fechamento de console mapeiam para exit 143
- Cache e config herdam ACLs do pai em vez de modos Unix


## Containers
- Instale com `cargo install docsrs-cli --locked` no build da imagem
- Defina `DOCSRS_CLI_HOME` para volume sandbox gravável
- Forneça CA roots para rustls validar HTTPS público
- Prefira usuários non-root com home gravável ou dirs explícitos de config/cache


## Suporte a Shell
- Completions: bash, zsh, fish, elvish, powershell, power-shell
- Markdown humano é o default em TTY
- JSON é escolhido automaticamente em pipes non-TTY


## Paths de Arquivo e XDG
- Nunca hardcode separadores; a CLI usa `PathBuf`
- Precedência: flags CLI, allowlist de env, `DOCSRS_CLI_HOME`, ProjectDirs
- `config path --json` imprime a camada vencedora para auditorias


## Performance por Target
- Trabalho de rede I/O-bound domina o wall time em todos os targets
- Offload de parse CPU usa `spawn_blocking` e scans rayon opcionais
- Comportamento de cache em disco é idêntico nas plataformas suportadas


## Agentes Validados por Plataforma
- Qualquer agente que execute binário e parseie JSON funciona em todos os targets
- Use as skills empacotadas como fonte operacional de verdade
- Valide com `docsrs-cli doctor --json` em cada classe de host que você envia
