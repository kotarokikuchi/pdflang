# 11. Comandos do CLI

[← Biblioteca padrão](10-stdlib.md) · [Índice](README.md) · [Próximo: Receitas →](12-receitas.md)

Treze comandos: seis que trabalham com PDFs, quatro sobre os scripts, dois de
distribuição e um para o shell.

| Comando | O que faz |
|---|---|
| [`run`](#pdfl-run) | Valida um PDF com um script |
| [`compare`](#pdfl-compare) | Compara duas versões de um PDF |
| [`pixelcompare`](#pdfl-pixelcompare) | Compara dois PDFs pixel a pixel, com um visualizador para ver a mudança |
| [`watch`](#pdfl-watch) | Monitora uma pasta e valida o que chega |
| [`fix`](#pdfl-fix) | Aplica correções e salva um PDF novo |
| [`inspect`](#pdfl-inspect) | Resumo rápido de um PDF |
| [`lint`](#pdfl-lint) | Analisa um script sem executar |
| [`fmt`](#pdfl-fmt) | Formata um script |
| [`test`](#pdfl-test) | Roda um script sobre uma pasta de PDFs e compara cada relatório |
| [`doc`](#pdfl-doc) | Gera documentação de um script |
| [`pack`](#pdfl-pack) | Empacota perfis e bases |
| [`add`](#pdfl-add) | Instala um pacote |
| [`completions`](#pdfl-completions) | Imprime o script de autocompletar do seu shell |

---

## Códigos de saída

Todos os comandos que validam usam a mesma convenção:

| Código | Significado |
|---|---|
| `0` | Tudo passou |
| `1` | Apenas avisos |
| `2` | Erros de validação |
| `3` | Erro de sintaxe no script |
| `10` | O documento não pôde ser lido, ou um arquivo não pôde ser escrito — nenhum veredito foi dado |

Em scripts de shell:

```bash
pdfl run perfil.pdfl arquivo.pdf > relatorio.json
case $? in
  0) echo "aprovado" ;;
  1) echo "aprovado com ressalvas" ;;
  2) echo "reprovado — veja relatorio.json" ;;
  3) echo "erro no script de validação" ;;
esac
```

---

## Opções globais

| Opção | O que faz |
|---|---|
| `--quiet` | Silencia progresso e confirmações no stderr |

`--quiet` funciona antes ou depois do subcomando, e em todos eles. Tira as linhas
que uma pessoa quer e uma pipeline não — `report saved to …`, `watching …`, o
resultado por arquivo do `watch`. Ele **não** tira erros: uma execução silenciosa
que falha continua dizendo por quê.

Também não silencia o `print()`. Aquilo é a saída do próprio script, e sumir com
ela mudaria o que o script faz. Redirecione o stderr se quiser se livrar dela.

`--quiet` vence o `--verbose`.

---

## `pdfl run`

Valida um PDF com um script.

```bash
pdfl run <script.pdfl> <entrada.pdf> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Formato do relatório |
| `--output-file <arquivo>` | — | Grava em arquivo em vez do stdout |
| `--fail-on error\|warning` | `error` | Com `warning`, avisos também dão exit 2 |
| `--verbose` | — | Informação extra no stderr |
| `--var NOME=VALOR` | — | Valor que o script lê como `vars.NOME`; repetível |
| `--tags TAG` | — | Roda só os checks com essa tag; repetível. Tag que nenhum check tem é erro, não aprovação vazia |

```bash
# Relatório JSON no terminal
pdfl run prepress.pdfl revista.pdf

# HTML para enviar ao cliente
pdfl run prepress.pdfl revista.pdf --output html --output-file laudo.html

# PDF de auditoria (o formato pdf sempre grava em arquivo)
pdfl run prepress.pdfl revista.pdf --output pdf --output-file laudo.pdf

# CSV para planilha
pdfl run prepress.pdfl revista.pdf --output csv --output-file achados.csv

# Rigoroso: avisos também reprovam
pdfl run prepress.pdfl revista.pdf --fail-on warning
```

### O relatório JSON

```json
{
  "schema_version": 1,
  "script_name": "prepress.pdfl",
  "input_file": "revista.pdf",
  "profile": "offset-revista",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Cobertura de tinta",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ],
  "checks_run": ["Ink coverage", "Fonts", "Bleed"]
}
```

O mesmo PDF com o mesmo script sempre gera o **mesmo relatório, byte a byte** —
dá para versionar e comparar em CI.

`schema_version` é a primeira chave para o consumidor decidir antes de parsear o
resto. Ela sobe só quando quem lia a saída anterior quebraria; acrescentar um
campo não a faz subir.

### SARIF e JUnit

Mais dois formatos, para o resultado aparecer onde a equipe já olha, e não num
log que ninguém abre.

```bash
# GitHub code scanning: os achados viram anotações no pull request
pdfl run prepress.pdfl revista.pdf --output sarif --output-file pdfl.sarif

# Painel de testes de qualquer CI: um teste por check, incluindo os que passaram
pdfl run prepress.pdfl revista.pdf --output junit --output-file pdfl.xml
```

No SARIF o achado é ancorado no **script**, não no PDF: a linha que se conhece é
a do check, e o PDF costuma ser um artefato de passagem na CI, não um arquivo do
repositório — apontar para lá anotaria um caminho que não existe. O arquivo
validado viaja em `properties.inputFile`, e o id do diagnóstico em
`partialFingerprints`, que é o que permite ao GitHub reconhecer um achado já
visto em vez de reabri-lo a cada execução.

No JUnit todo check que rodou é um caso de teste, inclusive os que não acharam
nada. Um formato que listasse só as falhas reportaria uma execução limpa como
zero testes, e uma CI lê isso como execução que não aconteceu. Um achado `info`
não reprova o caso; ele vai para `<system-out>`.

```yaml
- name: Preflight
  run: pdfl run prepress.pdfl revista.pdf --output sarif --output-file pdfl.sarif
  # exit 2 é arquivo reprovado, e o upload ainda precisa acontecer
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

Compara duas versões de um PDF: texto, estrutura e metadados.

```bash
pdfl compare <v1.pdf> <v2.pdf> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Formato |
| `--output-file <arquivo>` | — | Grava em arquivo |
| `--normalize` | — | Ignora maiúsculas e espaçamento |
| `--ignore-dates` | — | Mascara datas antes de comparar |
| `--similarity-threshold <0-100>` | `100` | Similaridade mínima aceitável |

```bash
# Comparação simples
pdfl compare aprovado_v1.pdf novo_v2.pdf

# Tolerando pequenas diferenças de formatação e datas
pdfl compare aprovado_v1.pdf novo_v2.pdf --normalize --ignore-dates

# Aceita até 1% de diferença; abaixo disso vira erro
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### Como funciona

- As páginas são **alinhadas por conteúdo**, não por número: se uma página foi
  inserida no meio, o comparador percebe em vez de acusar tudo depois dela como
  diferente. Funciona em documentos de mais de mil páginas.
- Cada página alinhada recebe uma nota de similaridade e uma amostra das linhas
  que mudaram (`-` saiu, `+` entrou).
- Metadados diferentes viram **aviso**; texto alterado vira **erro** se ficar
  abaixo do threshold, **aviso** se acima.
- O relatório traz o campo `similarity` com a nota geral.

```
page 4 → 4: similarity 97.8% | -título original contos | +título revisado
```

---

## `pdfl pixelcompare`

Compara dois PDFs por como eles *aparecem*, página por página.

```bash
pdfl pixelcompare <original.pdf> <novo.pdf> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Formato do relatório |
| `--output-file <arquivo>` | — | Grava o relatório em arquivo |
| `--viewer <pasta>` | — | Grava um visualizador autocontido: as páginas, as diferenças e um `index.html` para olhar |
| `--dpi <n>` | `150` | Resolução de renderização. Mais alta enxerga mais e custa mais |
| `--threshold <0.0-1.0>` | `0.05` | Distância de cor a partir da qual dois pixels são diferentes |
| `--max-diff <percentual>` | `0.0` | Quanto de uma página pode mudar antes de virar achado |
| `--pages <intervalo>` | todas | `1-10` ou `1,3,7-12` |
| `--no-align` | — | Não compensa um deslocamento global entre as páginas |
| `--blur <raio>` | `0` | Desfoque antes de comparar, para absorver o antialiasing |
| `--jobs <n>` | um por CPU | Páginas comparadas ao mesmo tempo |

O `pdfl compare` responde "o texto ou a estrutura mudaram". Este responde outra
pergunta — "continua com a mesma cara" — e as duas discordam mais do que se
imagina. Um logo deslocado 2mm, um fio de cabelo que sumiu, uma cor especial
trocada por uma composição CMYK dela: nos três casos o texto é idêntico.

```bash
# O documento inteiro, em JSON
pdfl pixelcompare aprovado.pdf reimpressao.pdf

# Com um lugar para de fato olhar as diferenças
pdfl pixelcompare aprovado.pdf reimpressao.pdf --viewer diff/

# Tolerar um pouco de ruído e olhar com mais cuidado o que sobrar
pdfl pixelcompare aprovado.pdf reimpressao.pdf --max-diff 0.1 --dpi 300
```

Um achado por página que mudou, com a fração de pixels e em quantas áreas
separadas eles caem:

```
page 7: 0.51% of the pixels differ, in 29 area(s)
```

Uma página que existe num arquivo e não no outro é um achado à parte — não há
com o que compará-la. O `similarity` do relatório é a média das páginas
comparadas, então uma página refeita em duzentas não faz o documento parecer
outro; os números por página estão nos diagnósticos.

### Alinhamento, e por que ele vem ligado

Um arquivo exportado de novo da mesma origem costuma sair um ou dois pixels
fora do lugar. Sem compensar isso, toda borda de glifo da página fica
"diferente" e a única mudança que importa se perde no meio. O `pixelcompare`
procura um deslocamento global único — primeiro grosseiro, numa cópia reduzida,
depois refinado — e informa quando encontra:

```
page 3: 2.10% of the pixels differ, in 44 area(s) (aligned by 2, -1 px)
```

Desligue com `--no-align` quando a posição *for* justamente o que se está
conferindo.

### O visualizador

O `--viewer diff/` grava uma pasta com três PNGs por página e um `index.html`.
Sem dependência de nenhum tipo — sem CDN, sem bundler, sem servidor. Abra o
arquivo, ou compacte a pasta e mande para quem precisa aprovar a reimpressão.

Três painéis, lado a lado, sempre na mesma página:

| Painel | O que mostra |
|---|---|
| **Original** | a página do primeiro arquivo, intacta |
| **New** | a página do segundo arquivo, intacta |
| **Difference** | os dois, com o que mudou pintado por cima — arraste para o wipe |

Os três painéis têm o mesmo par de barras — uma em pé, uma deitada — na mesma
posição, e elas se movem nos três de uma vez. A barra em pé é arrastada; a
deitada acompanha o ponteiro, pressionado ou não. Onde elas se cruzam é o canto
do que está sendo revelado, e o círculo fica na barra em pé nessa altura, ou
seja, marca o ponto onde o ponteiro está segurando.

No painel **Difference** as barras cortam: o arquivo novo aparece à direita da
barra em pé e abaixo da deitada, e o original em todo o resto. Intacta, a barra
deitada fica no topo, o que faz da barra em pé um wipe simples de altura
inteira — desça a deitada quando a mudança que você persegue estiver numa faixa
em vez de numa coluna. Nos outros dois painéis as barras são réguas sobre a
mesma coluna e a mesma linha da página, então dá para achar em cada original
aquilo que o wipe está cortando, sem medir no olho.

As duas posições são porcentagens da página, não de um painel, então sobrevivem
a trocar de página e a redimensionar a janela.

A roda do mouse dá zoom, até 8×, e os três painéis ampliam juntos em torno do
ponto sob o ponteiro — então o que você estava olhando continua onde estava.
Diminuir para no ajuste à página: abaixo disso não há nada de útil, o painel já
tem o tamanho de segurar a página inteira. As barras mantêm a espessura em
qualquer zoom. O **Reset view** devolve o zoom à página inteira e as barras à
posição inicial; ele fica desabilitado enquanto não houver o que desfazer.

As diferenças são pintadas no lugar, e a cor diz de que tipo:

| Cor | Significado |
|---|---|
| Vermelho | Tinta que sumiu no arquivo novo |
| Verde | Tinta que é nova nele |
| Azul | Mesmo peso, cor diferente |

Os três painéis são dimensionados contra a janela, então a comparação inteira
cabe na tela sem rolagem, e eles mantêm a proporção da página em qualquer
formato de janela. Onde os dois arquivos discordam do tamanho de uma página —
uma virou paisagem, digamos — cada uma é mostrada inteira dentro da caixa
comum, em vez de esticada para preenchê-la.

**Ele abre nas páginas que diferem.** Num documento de duzentas páginas em que
três mudaram, essas três são o motivo de você ter aberto; o **All** traz as
outras de volta. As setas e `←` `→` seguem o filtro, pulando o que a faixa está
escondendo em vez de pousar em cima. Quando nada difere, o botão do filtro diz
isso e fica desabilitado, em vez de reduzir a faixa a nada.

### Progresso

Rasterizar um documento longo a 300 dpi leva minutos, então cada etapa desenha
uma barra no stderr: uma para cada arquivo sendo rasterizado, uma para a
comparação e uma para gravar o visualizador.

```
rasterising aprovado.pdf  [############------------]  98/207
```

Ela só é desenhada quando o stderr é um terminal. A barra funciona voltando ao
início da linha e sobrescrevendo-a; um arquivo de log não tem cursor para mover,
então uma execução redirecionada acumularia milhares de fragmentos. Redirecionada,
ela fica em silêncio e as mensagens normais continuam saindo. O `--quiet`
silencia em qualquer caso.

### Velocidade

A comparação usa todas as CPUs por padrão. Em 41 páginas a 150 dpi:

| `--jobs` | Tempo |
|---|---|
| `1` | 3,6s |
| `4` | 1,7s |
| `8` | 1,2s |
| `20` | 1,3s |

Para de melhorar por volta de oito porque essa etapa é limitada pela banda de
memória, não pela aritmética — ela varre páginas inteiras pela CPU — então
daí em diante as threads apenas disputam a mesma memória. Pedir mais não faz
mal, só não adianta.

Repare no que **não** é paralelo: a rasterização. O pdfium serializa toda
chamada atrás de um único lock global, então uma segunda thread na frente dele
só espera. Isso cria um piso para a execução — cerca de 0,8s dos números acima
— e é por isso que `--jobs 8` é três vezes mais rápido, e não oito.

Aqui o padrão é uma por CPU, enquanto `pdfl test` e `pdfl watch` usam
`--jobs 1`. A diferença é real: lá cada job é um processo filho segurando o
próprio documento, ou seja, mais um documento na memória. Aqui as páginas já
estão na memória e as threads as compartilham, então um job custa o espaço de
trabalho de uma página. Reduza se a máquina for compartilhada.

Códigos de saída: `0` nenhuma página mudou mais que `--max-diff`, `2` pelo
menos uma mudou, `10` um arquivo não pôde ser lido ou o visualizador não pôde
ser gravado.

O relatório não depende de `--jobs`. As páginas são recompostas na ordem de
página, então os diagnósticos, sua ordem e suas impressões digitais saem
idênticos com qualquer valor — há um teste que garante isso, e os arquivos do
visualizador saem byte a byte iguais.

---

## `pdfl watch`

Monitora uma pasta e valida cada PDF que chega ou muda.

```bash
pdfl watch <pasta> --script <script.pdfl> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | Quais arquivos processar |
| `--exclude <glob>` | — | Quais ignorar |
| `--output-dir <pasta>` | ao lado do PDF | Onde gravar os relatórios |
| `--depth <n>` | `1` | Níveis de subpasta |
| `--debounce <ms>` | `1000` | Espera o arquivo parar de ser copiado |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Formato dos relatórios |
| `--fail-fast` | — | Para no primeiro erro |
| `--events` | — | Acorda com as notificações do sistema em vez de por tempo — não em pasta de rede |
| `--journal <arquivo>` | — | Registro append-only do que foi validado; rodar de novo pula o que ele cobre |
| `--timeout <s>` | — | Mata a análise de um arquivo depois desse tanto de segundos e o reporta como recusado |
| `--var NOME=VALOR` | — | Valor que todo arquivo lê como `vars.NOME`; repetível |
| `--jobs <n>` | `1` | Arquivos validados ao mesmo tempo; `0` é um por CPU |
| `--once` | — | Processa o que já está lá e sai |

```bash
# Pasta de entrada da gráfica, rodando continuamente
pdfl watch entrada/ --script preflight.pdfl --output-dir laudos/ --report html

# Modo lote para CI: processa tudo e sai com o pior código
pdfl watch entrada/ --script preflight.pdfl --once
echo "resultado: $?"

# Ignorando rascunhos
pdfl watch entrada/ --script preflight.pdfl \
  --pattern "*.pdf" --exclude "*_rascunho*"
```

O `--jobs` vale para o que a passada tiver que vencer, tanto em lote quanto numa
rajada de chegadas. Cada arquivo é validado pelo seu próprio processo `pdfl` — o
mesmo motivo do `pdfl test` — e é este processo que renderiza os laudos, então o
arquivo escrito é idêntico independente do `--jobs`. Em oito arquivos de 41
páginas: 9,5s com `--jobs 1`, 1,2s com `--jobs 0`.

Com `--fail-fast`, nenhum arquivo novo começa depois que um falhou; os que já
estavam rodando terminam, porque matá-los deixaria laudos pela metade. Os laudos
são escritos na ordem em que os arquivos foram encontrados, então um lote imprime
as mesmas linhas não importa quantos rodaram juntos.

O **debounce** existe porque arquivos grandes chegam aos poucos: o watch só
processa quando o arquivo para de mudar, evitando ler um PDF pela metade. A
espera termina exatamente quando o arquivo mais novo assentou, então um arquivo
que chega durante uma espera não fica retido um intervalo inteiro a mais.

Por padrão a pasta é listada por tempo; com `--events` o watch espera pelas
notificações do sistema operacional. O padrão é o tempo, e isso foi medido:
listar 10.000 arquivos a cada 200ms não custa CPU mensurável, e o tempo de
assentamento domina a latência dos dois jeitos — numa pasta local, os dois modos
terminam com centésimos de segundo de diferença.

Não use `--events` em pasta de rede. O inotify num mount NFS ou SMB reporta o
que a máquina local escreve e mais nada, então arquivo vindo de fora nunca seria
notado — e o watch não diria nada a respeito. Onde compensa é numa máquina
observando muitas pastas, ou onde listar o diretório é caro. Se o observador não
puder ser criado, o watch avisa e volta para o tempo, em vez de ficar mudo.

### O journal: terminar um lote que foi interrompido

Cinco mil arquivos, e a máquina reinicia no quatro mil. Sem registro, a próxima
rodada começa do primeiro.

```bash
pdfl watch entrada/ --script offset.pdfl --once --journal lote.jsonl
```

Um objeto JSON por arquivo, acrescentado conforme cada um é validado:

```json
{"input":"entrada/capa.pdf","sha256":"9f2b…","status":"FAIL","errors":2,"warnings":0,"exit":2}
```

Rode de novo com o mesmo journal e os arquivos que ele cobre são pulados. Os
veredictos, não: um lote retomado que pula um arquivo reprovado ainda sai com
`2`, porque o journal é o registro do lote e o código de saída é o veredicto
dele. Um lote reportando limpo porque já tinha visto a falha seria o pior bug que
esta ferramenta poderia ter.

O arquivo é reconhecido **pelos bytes**, não pelo nome nem pela data. Troque
`capa.pdf` por outro `capa.pdf` e ele é validado de novo — o hash não é o que
está registrado.

Nada é escrito sem `--journal`. A ferramenta não guarda estado próprio; este é um
arquivo que você pediu pelo nome, igual a um laudo. E não há data na linha: o
journal diz *se* o arquivo foi validado e no que deu, o laudo ao lado diz *o
quê*, e o sistema de arquivos diz *quando* — o que mantém uma re-execução byte a
byte igual à primeira, como todo o resto aqui.

As linhas são gravadas uma a uma, então o que uma queda deixa para trás é verdade
até onde vai. Um journal que não pode ser lido para a execução dizendo em qual
linha; pular arquivos por causa de um registro mal lido seria pior do que começar
de novo.

### `--timeout`: um arquivo ruim não pode travar o lote

```bash
pdfl watch entrada/ --script offset.pdfl --once --timeout 60
```

Um arquivo cuja análise passa de `60` segundos é morto e reportado do mesmo jeito
que um PDF ilegível — um laudo com um achado, `check_name: "timeout"` — então ele
imprime, grava em disco e entra no journal exatamente como qualquer outro
veredicto. Nada é pulado em silêncio, e o lote segue para o próximo arquivo em
vez de travar nesse.

```json
{"input":"entrada/adversario.pdf","sha256":"7a1c…","status":"FAIL","errors":1,"warnings":0,"exit":2}
```

Não existe nada na linguagem `.pdfl` que um script possa usar para travar o
interpretador de propósito — a recursão tem limite de profundidade — então o
`--timeout` existe para o que um script não pode causar: o pdfium entrando em
loop ou travando num PDF malformado ou adversário. Sem a flag, a análise de um
arquivo espera o tempo que precisar, que era o único comportamento antes dela
existir.

`--var` chega a cada arquivo sem mudar — um valor para a execução inteira, útil
para algo constante numa pasta (um nome de cliente) e não algo que varia por
arquivo (um número de pedido). Sem ele, um script que lê `vars.*` nunca poderia
ser observado: todo arquivo falharia com "was not provided".

Os relatórios saem como `<nome>.report.json` (ou `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Aplica operações `fix::` e salva um PDF novo. Detalhes no
[capítulo 8](08-fix.md).

```bash
pdfl fix <entrada.pdf> <script.pdfl> --output <saida.pdf> [opções]
```

| Opção | O que faz |
|---|---|
| `--output <arquivo>` | PDF de saída (obrigatório) |
| `--dry-run` | Lista as operações sem salvar |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | Formato do relatório |
| `--report-file <arquivo>` | Grava o relatório em arquivo |

```bash
# Ver o que seria feito, sem tocar em nada
pdfl fix original.pdf normalizar.pdfl --output saida.pdf --dry-run

# Aplicar de verdade
pdfl fix original.pdf normalizar.pdfl --output corrigido.pdf
```

---

## `pdfl inspect`

Resumo rápido de um PDF, sem script.

```bash
pdfl inspect <arquivo.pdf>
```

`--json` devolve o mesmo resumo como dado.

```
File:     revista.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Revista Exemplo
  Creator: Adobe InDesign 19.3

Fonts:    26
  ABCDEF+Helvetica — embedded
  Arial — NOT embedded
Images:   81 (minimum DPI 136, spaces: DeviceCMYK, Indexed)
Max. estimated TAC: 300% (RGB render approximation)

Warnings:
  ! there are non-embedded fonts
  ! 3 image(s) below 300 DPI
```

É o primeiro comando a rodar quando um arquivo novo chega: em segundos você sabe
se vale a pena abrir.

---

## `pdfl lint`

Analisa um script sem executar, apontando problemas de qualidade.

```bash
pdfl lint <script.pdfl>
```

`--json` devolve os mesmos avisos como dado.

Detecta:

- variáveis, parâmetros de bloco e functions declarados e **nunca usados**
  (prefixe com `_` para silenciar: `_page`)
- checks **duplicados** ou **vazios**
- namespace desconhecido (`text::`, `struct::`, `visual::`, `prepress::`,
  `codes::`, `fix::`, `data::`)
- `assert`/`require` fora de qualquer check
- uso de `fix::` (que só roda em `pdfl fix`)

```bash
$ pdfl lint perfil.pdfl
perfil.pdfl: warning: variable 'LIMITE' declared and never used
perfil.pdfl: warning: check "Fontes" declared 2 times
```

Sai com código `1` se houver avisos — dá para usar em CI.

---

## `pdfl fmt`

Formata o script: indentação de 2 espaços, espaçamento consistente, linhas em
branco colapsadas. Preserva comentários e unidades (`3mm` continua `3mm`).

```bash
pdfl fmt <script.pdfl>            # formata no lugar
pdfl fmt <script.pdfl> --check    # não altera; sai com 1 se estiver fora do padrão
```

```bash
# Em CI, garantindo padrão na equipe
for f in perfis/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

Gera a documentação de um script a partir do próprio código.

```bash
pdfl doc <script.pdfl> [--output markdown|html|json]
```

Produz: perfil, tabela de constantes, functions, imports e — para cada check —
as tags e o que ele valida (as mensagens dos `assert` viram a descrição).

```bash
# Markdown para o repositório
pdfl doc prepress.pdfl > docs/perfil-prepress.md

# HTML para enviar a quem não lê código
pdfl doc prepress.pdfl --output html > perfil.html
```

É o artefato para o gerente de produção entender o que o perfil valida sem abrir
o script.

---

## `pdfl pack`

Empacota scripts e bases em um arquivo `.pdflpkg` distribuível.

```bash
pdfl pack <pasta> [--name <nome>] [--version <versão>] [--output <arquivo>]
```

Inclui `.pdfl`, `.csv`, `.txt` e `.json` da pasta (recursivamente), com um
`manifest.json` que registra o SHA-256 de cada arquivo. O pacote é
determinístico: mesma pasta gera bytes idênticos.

Planilha (`.xlsx`, `.xls`, `.ods`) **não** entra no pacote, e o `pack` diz qual
arquivo ficou de fora. Nenhuma função `data::` abre uma, então empacotá-la
entregaria um pacote que instala direitinho e falha na primeira consulta.

```bash
pdfl pack perfis/grafica --name perfil-grafica --version 1.0.0
# cria perfil-grafica.pdflpkg
```

---

## `pdfl add`

Instala um pacote local, conferindo os hashes do manifesto.

```bash
pdfl add <pacote.pdflpkg> [--dir <pasta>]
```

```bash
pdfl add perfil-grafica.pdflpkg
# instala em ./pdfl_profiles/perfil-grafica@1.0.0/

pdfl run pdfl_profiles/perfil-grafica@1.0.0/prepress.pdfl arquivo.pdf
```

Se algum arquivo tiver hash diferente do registrado, a instalação é **recusada**
— pacote corrompido ou adulterado não entra.

> Repositório remoto e assinatura digital não fazem parte desta versão: o `add`
> instala a partir de arquivos locais.

---

## `pdfl test`

Roda um script sobre cada PDF de uma pasta e compara cada relatório com o que
está gravado ao lado. Um perfil que passa a achar outra coisa quebra um teste,
em vez de surpreender alguém lá na frente.

```bash
pdfl test <script.pdfl> [--dir <pasta>] [--update]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--dir <pasta>` | `tests/` ao lado do script | Onde estão os PDFs dos casos |
| `--update` | — | Grava os relatórios esperados em vez de comparar |
| `--jobs <n>` | `1` | Casos rodando ao mesmo tempo; `0` é um por CPU |
| `--var NOME=VALOR` | — | Valor que todo caso lê como `vars.NOME`; repetível |

Um caso é um PDF e o relatório esperado dele, lado a lado:

```
perfis/grafica/
  prepress.pdfl
  tests/
    aprovado.pdf
    aprovado.expected.json
    tinta_pesada.pdf
    tinta_pesada.expected.json
```

```bash
# Na primeira vez: grave o que o script acha hoje
pdfl test prepress.pdfl --update

# Daí em diante
pdfl test prepress.pdfl
```

```
ok   aprovado.pdf
FAIL tinta_pesada.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Cobertura de tinta (line 12): página 7: 324% de tinta (limite 300%)
1 passed, 1 failed
```

A falha diz o que mudou — as contagens, o veredito e quais achados surgiram ou
sumiram — em vez de imprimir dois JSON lado a lado.

Gravar é sempre um ato deliberado: uma execução que atualizasse a própria linha
de base nunca falharia. Leia a diferença primeiro e regrave com `--update`
quando a mudança for a que você queria.

O relatório esperado é o que o `pdfl run` produz, com o `input_file` reduzido ao
nome do arquivo — uma linha de base que mudasse conforme o diretório de onde se
chamou não seria linha de base. Um PDF que não abre reprova o próprio caso e
deixa os outros rodarem.

Códigos de saída: `0` todos passaram, `2` pelo menos um falhou, `10` a pasta não
pôde ser lida ou não tem PDF.

### Rodando casos ao mesmo tempo

Cada caso roda como um processo `pdfl` próprio, então o `--jobs` transforma a
suíte em trabalho paralelo de verdade: em oito arquivos de 41 páginas, `--jobs 1`
levou 8,9s e `--jobs 8` levou 1,1s. Threads dentro de um processo não dariam
conta — o pdfium serializa toda chamada atrás de um único mutex, e a versão com
threads mediu *mais lenta* que a sequencial.

O padrão é `1` porque cada job é um processo segurando um documento na memória, e
esta ferramenta existe para arquivos que podem ser enormes. Aumente quando os
casos forem comuns: `--jobs 0` dá um por CPU.

A ordem da saída nunca muda com o `--jobs`: os casos são julgados na ordem em que
foram encontrados, não importa qual filho terminou primeiro.

Um caso cujo PDF não abre é julgado como qualquer outro — o relatório dele traz o
motivo como achado, então "este arquivo tem que ser recusado como ilegível"
também pode ser um teste. Esse relatório nomeia o arquivo como ele foi passado,
então grave as linhas de base com `--dir` **relativo** se elas forem versionadas.

`--var` chega a cada caso sem mudar — um valor para a execução inteira, não um
por arquivo. Sem ele, um script que lê `vars.*` nunca poderia ser testado: todo
caso falharia com "was not provided", não importa o PDF.

---

## `pdfl completions`

Imprime no stdout o script de autocompletar do seu shell.

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash, para o usuário atual
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — em qualquer lugar do seu $fpath
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

Nada mais vai para o stdout, então a saída pode ser redirecionada direto para a
pasta de autocompletar. Gere de novo depois de atualizar: o script é construído a
partir dos comandos e flags do binário que o imprimiu.

---

[← Biblioteca padrão](10-stdlib.md) · [Índice](README.md) · [Próximo: Receitas →](12-receitas.md)
