[English](COOKBOOK.md)

# Cookbook
> Copie uma receita, rode o comando, leia o JSON.

## Nota de Latência
- Fetches frios de rede dependem da latência de crates.io e docs.rs
- Hits quentes de cache ficam locais dentro do TTL e reportam `cache_hit: true`
- Use `--dry-run` quando só precisar de URLs planejadas

## Referência de Valores Default
- Timeout wall-clock usa um orçamento seguro de produto via config
- TTL de cache default é 86400 segundos
- Orçamento soft de cache default é 256 MiB
- Max body default é teto duro de 10 MiB (valores acima do hard max falham fechados com exit 65)
- Max output default é teto duro de 2 MiB (valores acima do hard max falham fechados com exit 65)
- `search-in-crate --match` default é `prefix`
- `search-in-crate --limit` default é 100 e clamp em 1000
- JSON é escolhido automaticamente em stdout non-TTY (exceto `completions` bruto)

## Como Buscar um Crate
- Problema: achar crates por palavra-chave
```bash
docsrs-cli search-crates serde --json
docsrs-cli search-crates async --page 1 --per-page 20 --sort downloads --json
```

## Como Paginar Com page-token
- Problema: percorrer resultados do crates.io sem montar query strings à mão
```bash
docsrs-cli search-crates async --page 1 --per-page 20 --json
# leia data.meta.next_page em NEXT, depois:
docsrs-cli search-crates --page-token "$NEXT" --json
# eco de query/page/per_page/sort bate com a URL efetiva (não argv obsoleto)
docsrs-cli --dry-run search-crates --page-token '?q=serde&per_page=2&page=2' --json
```

## Como Buscar a Visão Geral de um Crate
- Problema: ler o overview do docs.rs para um crate
```bash
docsrs-cli readme tokio --json
docsrs-cli readme clap --crate-version 4.5.0 --json
docsrs-cli readme tokio --crate-version latest --json
# resolved_version é o SemVer só do crate alvo, nunca de uma dependência
```

## Como Buscar Overview da Stdlib
- Problema: obter docs de std/core/alloc sem adivinhar HTML
```bash
docsrs-cli readme std --json
docsrs-cli readme core --json
docsrs-cli readme alloc --json
# resolved_version é o nome do canal como stable quando conhecido
```

## Como Observar o Clamp de Limit Offline
- Problema: provar o hard clamp de search-in-crate --limit sem rede
```bash
docsrs-cli --dry-run search-in-crate tokio "" --limit 5000 --json
# planned_params.limit é 1000 (hard clamp, inclusive no dry-run)
```

## Como Buscar um Item Tipado
- Problema: puxar documentação de uma struct, trait ou função
```bash
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio struct runtime::Runtime --json
docsrs-cli get-item clap trait clap::Parser --json
docsrs-cli get-item async-trait attribute async_trait --json
```

## Como Resolver Métodos Associados
- Problema: abrir docs de métodos como `Runtime::new` sem adivinhar HTML
```bash
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli get-item tokio fn runtime::Runtime::new --json
# página do tipo pai + #method.new; payload inclui item_name e resolved_version opcional
# data.extraction é method em sucesso; #method.X ausente é not_found (exit 66), nunca sucesso item_page
```

## Como Abrir Item Associado de Trait
- Problema: `Iterator::Item` e `Duration::MAX` não têm página própria
```bash
docsrs-cli get-item std type iter::Iterator::Item --json
docsrs-cli get-item std const time::Duration::MAX --json
docsrs-cli get-item std method iter::Iterator::next --json
# o rustdoc emite um prefixo de âncora por categoria de membro, todos na página do pai:
#   method.NOME · tymethod.NOME · associatedtype.NOME · associatedconstant.NOME
# source_url ecoa a âncora que existe, não a que foi planejada
docsrs-cli get-item std const u32::MAX --json
# pai em minúscula (primitivo ou módulo) segue item livre em página própria
docsrs-cli get-item std type iter::Iterator::item --suggest --json
# só a caixa do PAI escolhe a rota; um typo na folha ainda alcança a página do
# pai e falha com not_found (exit 66) trazendo os nomes reais dos membros, nunca
# um 404 sobre caminho inventado
```

## Como Recuperar de Typo em Method
- Problema: o agente digitou `Runtime::neww` e não pode aceitar sucesso falso com página pai
```bash
docsrs-cli get-item tokio method Runtime::neww --suggest --json
# exit 66, ok=false, error.kind=not_found; error.suggestions traz os leaves ranqueados como {path, kind}
# envelope de erro no topo tem command, duration_ms e error aninhado
# nunca trate extraction=item_page como sucesso de method (removido em 1.2.0)
# anchor_family traz a família real: variant, structfield, associatedtype, associatedconstant, tymethod, method

# transforme a melhor sugestão direto no próximo comando — sem parsear texto
docsrs-cli get-item tokio method Runtime::neww --suggest --json \
  | jaq -r '.error.suggestions[0] | "docsrs-cli get-item tokio \(.kind) \(.path) --json"'
# o campo fica ausente quando o ranking não achou nada, então proteja com // empty em script
```

## Como Planejar URL de Method Offline
- Problema: inspecionar parent kind e probes planejados sem rede
```bash
docsrs-cli --dry-run get-item tokio method runtime::Runtime::neww --json
# planned_params.validation=url_shape_only; planned_parent_kind + parent_kind_probe presentes
# dry-run não prova que a âncora remota existe
```

## Como Falhar Fechado em Overshoot do Hard Max de Budget
- Problema: o agente não deve aceitar clamp silencioso quando flags passam do hard max
```bash
docsrs-cli --max-body-bytes 999999999 version --json
docsrs-cli --max-output-bytes 999999999 version --json
# ambos saem com exit 65 invalid_input (sem clamp silencioso para 10 MiB / 2 MiB)
```

## Como Reduzir Payload Sem Pós-Processador de JSON
- Problema: contexto de agente é caro e o envelope inteiro é maior que a resposta
```bash
docsrs-cli --dry-run --select planned_url readme serde --json
# data traz só planned_url; o corte acontece antes da serialização (sem estágio jq/jaq)
docsrs-cli --dry-run --fields planned_url readme serde --json
# --fields é alias de --select; CSV e flag repetida funcionam igual
docsrs-cli --dry-run --count-only readme serde --json
# data é {"count":1}; a contagem roda depois de --filter e --dedupe-by
docsrs-cli --dry-run --truncate-content 5 readme serde --json
# strings acima de 5 caracteres são encurtadas; agent_surface.content_truncated fica true
docsrs-cli --dry-run --select nao_existe readme serde --json
# data é {}; chave ausente é pulada, nunca emitida como null
docsrs-cli --select name search-in-crate serde Serialize --limit 3 --json
# com array de resultados, --select projeta os ELEMENTOS: hits:[{"name":…}]
docsrs-cli --dedupe-by name search-in-crate serde Serialize --limit 5 --json
# agent_surface mostra input_count 5 e output_count 4 quando um name repete
```

## Como Pegar o Top N de um Conjunto Filtrado
- Problema: `--filter` estreita a lista mas a resposta ainda são as primeiras linhas
- Instinto errado: canalizar o envelope por `jaq` — isso devolve o trabalho ao agente
```bash
docsrs-cli --filter kind=struct --sort-by name --max-items 5 --select name \
  search-in-crate serde "" --limit 200 --json
# --limit 200 limita a CONSULTA (quanto do all.html é classificado)
# --max-items 5 limita a EMISSÃO (quanto chega ao stdout)
# a ordem é fixa: filter, sort-by, dedupe-by, max-items, select
docsrs-cli --sort-by downloads search-crates serde --per-page 20 --json
# número compara numericamente: 9 vem antes de 10, nunca depois
docsrs-cli --sort-by chave_inexistente search-crates serde --json
# chave que ninguém carrega é no-op, não erro; a ordem upstream sobrevive
docsrs-cli --max-items 5 --count-only search-in-crate serde "" --limit 200 --json
# {"count":5}: a contagem descreve o recorte, não o conjunto filtrado
docsrs-cli --max-items 999 search-in-crate serde "" --limit 20 --json
# agent_surface.limited é false: conjunto pequeno nunca parece conjunto cortado
```

## Como Confiar nos Contadores Depois de uma Redução
- Problema: `emitted` é documentado como hits realmente emitidos, e a redução encolhe o array
```bash
docsrs-cli --filter kind=struct search-in-crate serde "" --limit 200 --json
# data.emitted bate com o tamanho de hits: campo que nomeia o array acompanha o array
# data.total mantém a contagem upstream, porque descreve o docs.rs e não este envelope
docsrs-cli schema --cmd agent-surface --json
# o contrato completo do agent_surface, incluindo limited e as duas flags de truncagem
```

## Como Falhar Fechado em Filtro Malformado
- Problema: typo em filtro nunca pode parecer resultado vazio honesto
```bash
docsrs-cli --dry-run --filter 'chave sem operador' readme serde --json
# exit 65, ok=false, error.kind=invalid_input; a mensagem nomeia a gramática esperada
docsrs-cli --dry-run --filter '=semvalor' readme serde --json
# exit 65 também; lado esquerdo vazio é rejeitado em vez de casar nada em silêncio
docsrs-cli --dry-run --filter 'command=readme' --filter 'ok!=false' readme serde --json
# formas válidas são chave=valor, chave!=valor, chave~substring; repita a flag para AND
```

## Como Confirmar Scrub de Chrome do Rustdoc
- Problema: o markdown não deve carregar chrome de UI do rustdoc no contexto do agente
```bash
docsrs-cli readme serde --json
# data.markdown não tem marcadores de seção § nem strings de UI "Copy item path"
```

## Como Passar Segmentos de item_path Com Hífen
- Problema: nomes estilo crates.io usam hífen, mas paths rustc usam underscore
```bash
docsrs-cli --dry-run get-item async-trait attribute async-trait --json
# URL planejada usa async_trait; get-item live aceita hífen da mesma forma
docsrs-cli get-item async-trait attribute async-trait --json
```

## Como Sugerir Símbolos Próximos em 404
- Problema: recuperar de um path de item errado sem raspar all.html você mesmo
```bash
docsrs-cli get-item serde struct Serde --suggest --json
docsrs-cli get-item tokio struct RuntimeX --suggest --json
# sugestões ranqueiam exact → prefix → substring → edit-distance (um fetch de all.html)
# typos como Parserx podem trazer Parser (trait) na mensagem de erro
```

## Como Tratar Budget de Body Sem Tempestade de Retry
- Problema: a resposta passa de `--max-body-bytes` e o agente não deve girar
```bash
docsrs-cli --max-body-bytes 50 readme serde --json
# exit 74, error.kind=budget, error.retryable=false
# aumente --max-body-bytes (dentro do hard ceiling) em vez de retentar
```

## Como Falhar Fechado em Timeout Zero Explícito
- Problema: provar que timeout 0 é rejeitado em vez de travar para sempre
```bash
docsrs-cli --timeout 0 version --json
docsrs-cli --connect-timeout 0 doctor --json
# ambos saem com exit 65 invalid_input
```

## Como Buscar Símbolos Dentro de um Crate
- Problema: localizar símbolos sem navegar HTML
```bash
docsrs-cli search-in-crate reqwest Client --json
docsrs-cli search-in-crate clap Parser --item-type trait --json
docsrs-cli search-in-crate tokio "" --limit 50 --json
```
- Filtre pelo kind que o símbolo realmente é: `Parser` no clap é trait e macro derive, nunca função
- Filtro que não casa nada continua sendo sucesso: exit `0` com `total` e `emitted` em `0`, não erro
- O `item_type` ecoado é a grafia canônica, então `--item-type function` volta como `fn`

## Como Escolher Modos de Match
- Problema: apertar ou afrouxar o ranking da busca de símbolos
```bash
docsrs-cli search-in-crate serde Serialize --match exact --json
docsrs-cli search-in-crate serde Ser --match prefix --json
docsrs-cli search-in-crate serde de --match substring --limit 20 --json
# default é prefix; use substring para o comportamento legado de contains
```

## Como Detectar cache_hit
- Problema: saber se um payload veio do cache local em disco
```bash
docsrs-cli readme serde --json
docsrs-cli readme serde --json
# a segunda chamada costuma mostrar data.cache_hit true dentro do TTL
docsrs-cli --no-cache readme serde --json
# caminho forçado de rede reporta cache_hit false
docsrs-cli cache path --json
docsrs-cli cache stats --json
```

## Como Descobrir a Superfície de Agente
- Problema: aprender comandos e formas de payload de forma programática
```bash
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli schema --cmd search-crates --json
docsrs-cli schema --cmd readme --json
docsrs-cli schema --cmd search-in-crate --json
docsrs-cli schema --cmd version --json
docsrs-cli schema --cmd doctor --json
docsrs-cli schema --cmd commands --json
docsrs-cli schema --cmd cache --json
docsrs-cli schema --cmd cache-path --json
docsrs-cli schema --cmd cache-clear --json
docsrs-cli schema --cmd cache-stats --json
docsrs-cli schema --cmd config --json
docsrs-cli schema --cmd config-path --json
docsrs-cli schema --cmd config-show --json
docsrs-cli schema --cmd config-init --json
docsrs-cli schema --cmd schema --json
docsrs-cli schema --cmd completions --json
docsrs-cli schema --cmd error --json
docsrs-cli schema --cmd dry-run --json
docsrs-cli schema --cmd agent-surface --json
docsrs-cli schema --cmd all --json      # os vinte numa chamada só
docsrs-cli version --json
docsrs-cli doctor --json
```
- Peça o nome específico, não o guarda-chuva: `--cmd config-show` responde com a forma que `config show` realmente emite
- Os nomes guarda-chuva `cache` e `config` descrevem a família inteira, então carregam todas as variantes de uma vez

## Como Gerar Completions de Shell
- Problema: instalar scripts de completion do seu shell
```bash
docsrs-cli completions bash
docsrs-cli completions zsh
docsrs-cli completions fish
docsrs-cli completions elvish
docsrs-cli completions power-shell
docsrs-cli completions powershell
# shell bruto por default mesmo em non-TTY; JSON só quando explícito:
docsrs-cli completions bash --json
```

## Como Trabalhar Offline ou Sem Efeitos de Rede
- Problema: planejar URLs sem abrir sockets
```bash
docsrs-cli --dry-run search-crates serde --json
docsrs-cli --dry-run readme serde --json
docsrs-cli --dry-run get-item tokio fn task::spawn --json
docsrs-cli --dry-run search-in-crate reqwest Client --json
# planned_params usam crate_name (não crate)
```

## Como Gerir Cache e Config
- Problema: inspecionar saúde de storage e resetar estado local
```bash
docsrs-cli cache stats --json
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
docsrs-cli config init --force --yes --json
```

## Como Auditar Prontidão Antes de um Lote
- Problema: falhar fechado antes de muitas rodadas de agente
```bash
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache config init --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache doctor --json
docsrs-cli --config-dir /tmp/docsrs-audit/config --cache-dir /tmp/docsrs-audit/cache doctor --online --json
# trate como saudável só quando exit for 0 e top-level ok e data.ok forem ambos true
```

## Como Cobrir Cada Top-Level Command Uma Vez
- Problema: smoke da superfície completa de comandos em um checklist
```bash
docsrs-cli search-crates serde --json
docsrs-cli readme serde --json
docsrs-cli get-item serde trait Serialize --json
docsrs-cli get-item tokio method runtime::Runtime::new --json
docsrs-cli search-in-crate serde Serialize --match prefix --json
docsrs-cli version --json
docsrs-cli doctor --json
docsrs-cli doctor --online --json
docsrs-cli commands --json
docsrs-cli schema --cmd get-item --json
docsrs-cli completions bash >/dev/null
docsrs-cli cache path --json
docsrs-cli cache stats --json
docsrs-cli cache clear --yes --json
docsrs-cli config path --json
docsrs-cli config show --json
docsrs-cli config init --json
# quinze caminhos invocáveis: nove top-level mais os seis aninhados de cache e config
# os dois verbos destrutivos exigem alvo designado, por isso o --yes acima
# smoke live humano opcional (sem CI):
# ./scripts/smoke-live.sh
```

## Como Abrir uma Variante de Enum ou um Campo de Struct
- Problema: `Option::Some` e `Range::start` existem apenas como âncoras na página do pai
```bash
docsrs-cli get-item std variant option::Option::Some --json
docsrs-cli get-item std variant result::Result::Ok --json
docsrs-cli get-item std structfield ops::Range::start --json
# field é alias de structfield; o eco item_type no wire é sempre structfield
docsrs-cli get-item std field ops::Range::end --json
# os dois kinds EXIGEM caminho Pai::membro — o rustdoc não serve variant.X.html
docsrs-cli get-item std variant Some --json
# exit 65, invalid_input: nomeia os kinds de pai que podem hospedar o membro
docsrs-cli get-item std variant option::Option::some --suggest --json
# exit 66 com os nomes reais das variantes, rotulados (variant) e prontos para colar
```

## Como Dirigir o Diagnóstico Sem Variável de Ambiente
- Problema: o produto não lê variável de ambiente de produto, então `RUST_LOG` não tem efeito
```bash
docsrs-cli -v version              # verbosidade por invocação
docsrs-cli config init             # depois defina log_directive no config.toml
# log_directive = "docsrs_cli=debug,docsrs_cli::http=trace"
docsrs-cli config show --json | jaq -r '.data.log_directive // "nao definida"'
# a chave só é ecoada depois de definida: config show a omite enquanto não está,
# então ler o campo num config recém-criado devolve null, e não o default
docsrs-cli -q version              # a flag explícita sempre vence o arquivo
# NO_COLOR / TERM / CLICOLOR_FORCE seguem valendo: descrevem o dispositivo de
# terminal, nunca configuração de produto, e --no-color vence as três
```
