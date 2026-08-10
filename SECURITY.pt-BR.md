[English](SECURITY.md)

# Política de Segurança
## Versões Suportadas
- `1.3.x` (linha atual) recebe correções de segurança
- `0.1.x` é histórico e não recebe novas correções de segurança
- Experimentos pré-release fora de tags não são suportados


## Reportar uma Vulnerabilidade
- Reporte em privado para daniloaguiarbr@proton.me
- Não abra issue pública para vulnerabilidades não corrigidas
- Inclua descrição de impacto, passos de reprodução e versão ou commit afetado
- Inclua OS, versão da CLI via `docsrs-cli version --json` e logs redigidos


## SLA de Resposta
- Crítico (CVSS 9.0-10.0): reconhecimento em até 2 dias úteis
- Alto (CVSS 7.0-8.9): reconhecimento em até 3 dias úteis
- Médio (CVSS 4.0-6.9): reconhecimento em até 5 dias úteis
- Baixo (CVSS 0.1-3.9): reconhecimento em até 10 dias úteis


## SLA de Correção
- Crítico: meta de fix ou mitigação em 14 dias após confirmação
- Alto: meta de fix em 30 dias após confirmação
- Médio: meta de fix em 60 dias após confirmação
- Baixo: meta de fix na próxima janela regular de manutenção


## Política de Divulgação
- Coordene a divulgação depois que o fix estiver disponível ou a mitigação documentada
- Credite repórteres que desejarem reconhecimento público no Hall of Fame
- Não exija NDA para relatos de boa-fé


## Política de Atualizações de Segurança
- Embarque fixes de segurança na linha minor suportada quando possível
- Documente mudanças de segurança no CHANGELOG sob Security
- Prefira defaults de menor privilégio e configuração fail-closed


## Notas de Escopo
- HTTP de produto é GET-only contra `crates.io`, `docs.rs`, `static.docs.rs` e `doc.rust-lang.org`
- Overrides de origin, redirects, gate de request e `final_url` do cache compartilham uma allowlist de hosts (porta SSRF)
- TLS usa apenas rustls (mín. TLS 1.2, provider crypto **ring**, piso rustls ≥ 0.23.18); sem `danger_accept_invalid_*`
- Raízes: webpki-roots; `HTTP(S)_PROXY` do sistema é confiança do operador quando usado (allowlist do alvo ainda vale)
- Postura TLS (provider, não-objetivos): [`docs/decisions/0007-rustls-posture.pt-BR.md`](docs/decisions/0007-rustls-posture.pt-BR.md)
- O crate de produto compila com `#![forbid(unsafe_code)]`
- Sem telemetria de produto
- O produto não armazena API keys
- Cache em disco guarda apenas bodies HTTP públicos
- Modos privados Unix preferem dirs `0o700` e arquivos `0o600` nas escritas da CLI
- Knobs de produto vêm só de flags CLI e XDG `config.toml`, nunca de env `DOCSRS_CLI_*` de produto
- `config.toml` tem teto de tamanho e rejeita chaves desconhecidas; User-Agent deve ser ASCII visível
- Modelo de ameaças (STRIDE, riscos aceitos): [`docs/decisions/0004-threat-model.pt-BR.md`](docs/decisions/0004-threat-model.pt-BR.md)


## Hall of Fame
- Pesquisadores de segurança que reportarem issues válidas podem ser listados aqui após o fix
- Ainda sem entradas para 1.3.x


## Boas Práticas para Usuários
- Instale com `cargo install docsrs-cli --locked`
- Mantenha o binário atualizado na linha minor suportada
- Não passe segredos em argv quando stdin ou env estiver disponível
- Rode `docsrs-cli doctor --json` após mudar paths de config
- Rode `docsrs-cli doctor --online --json` quando precisar de probes DNS para crates.io e docs.rs
- Trate o conteúdo do cache como snapshots públicos de documentação, não como segredos
