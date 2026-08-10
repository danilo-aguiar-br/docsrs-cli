[English](0010-explicit-target-designation.md)

# ADR 0010 — Designação Explícita de Alvo para verbos destrutivos

## Status
- Aceito (2026-08-10)

## Contexto
- Dois comandos destroem algo: `cache clear` esvazia todo body em cache sob a raiz resolvida, e `config init --force` substitui o `config.toml` no lugar.
- Os dois resolvem o alvo pela mesma resolução em camadas de todos os outros comandos: a flag de CLI quando existe, senão o diretório XDG.
- Essa resolução é correta para uma leitura. Para uma destruição ela produz a forma do delegado confuso: o chamador nomeia o verbo, o ambiente nomeia a vítima, e nada compara os dois.
- Um chamador que nunca passou `--cache-dir` nunca viu o caminho prestes a ser esvaziado. O comando continuava tendo tudo o que precisava para esvaziá-lo.
- O `cache clear` aprendeu a regra primeiro, como um braço de um `match`. O `config init --force` deveria aprender no mesmo plano e não aprendeu.
- A lacuna não era a guarda ausente. Era que nada conseguia perceber a ausência, porque regra que mora dentro de um braço de `match` não pode ser interrogada.
- Um segundo verbo destrutivo foi publicado sem waiver enquanto todo gate da árvore concordava com a árvore.

## Decisão

### 1. Verbo destrutivo precisa do alvo designado no argv
- Nomear o diretório com a flag de alvo do verbo designa: o chamador viu o caminho.
- Passar a flag de waiver aceita o diretório ambiente de propósito: o chamador escolheu não nomeá-lo.
- Não passar nenhum dos dois é recusado: exit 64, `kind=usage`, e nada é destruído.
- Só `PathSource::CliFlag` conta como designação. `Xdg` é a camada ambiente que o chamador nunca nomeou.
- `Unresolved` é pior que ambiente e é recusado por outro motivo: não há alvo algum, então agir seria agir em lugar desconhecido.

### 2. A recusa nunca depende do disco
- `config init --force` recusa alvo ambiente mesmo onde ainda não existe `config.toml`.
- Regra que consultasse o disco responderia diferente em duas máquinas com o mesmo argv, o que torna o contrato inensinável.
- O chamador não tem como saber de antemão se um diretório que ele nunca nomeou guarda um arquivo, então a resposta não pode depender disso.

### 3. A recusa nomeia a vítima e as duas saídas
- A mensagem carrega o verbo, o alvo resolvido, a flag de alvo e a flag de waiver.
- Recusa que esconde qual caminho seria destruído não ensina nada ao chamador sobre o que passar em seguida.

### 4. A regra vive num registro, não numa guarda
- `src/cli/destructive.rs` guarda um `DestructiveVerb` por verbo, com `wire`, `target_flag`, `waiver_flag`, `effect` e `schema_stem`.
- O runtime lê essa lista para decidir se recusa.
- `tests/policy/etd.rs` lê a mesma lista para exigir flag de waiver, `target_source` no schema nomeado, linha nas duas referências de configuração e anúncio nos dois guias de migração.
- Um terceiro verbo que destrói ou está no registro ou o gate de classe reprova. Ele não consegue estar caladamente correto como o `config init --force` esteve.

### 5. O envelope reporta qual camada resolveu o alvo
- Os dois verbos emitem `target_source` com valores `cli`, `xdg` ou `unresolved`.
- O campo tem o mesmo nome e os mesmos valores nos dois verbos, então uma auditoria só lê os dois.
- O `config init` emitia `source` antes; o rename é a quebra 1.3.0 registrada no guia de migração.

## Consequências
- Um script escrito antes desta regra, invocando qualquer dos dois verbos sem alvo e sem waiver, para de funcionar e não destrói nada, que é a falha pretendida.
- Toda receita, entrada de cookbook e runbook de skill que ensine invocação destrutiva precisa passar waiver ou alvo, e um gate varre a prosa atrás das que não passam.
- Acrescentar verbo destrutivo custa uma entrada no registro mais as quatro âncoras de documentação que o gate de classe cobra.
- Fora de escopo: prompt interativo de confirmação. Esta é uma CLI one-shot sem garantia de TTY, então o prompt travaria um agente ou seria pulado em silêncio.

## Relacionados
- ADR 0002 modelo de erro (o kind `usage` e o exit 64) · ADR 0006 postura de sistema de tipos (`PathSource` como tipo de domínio)
- `src/cli/destructive.rs` · `tests/policy/etd.rs` · `docs/schemas/cache-clear.schema.json` · `docs/schemas/config-init.schema.json`
- [Configuração](../CONFIGURATION.pt-BR.md) · [Migração](../MIGRATION.pt-BR.md)
