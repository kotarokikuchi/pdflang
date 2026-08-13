# 13. Mudanças

[← Receitas](12-receitas.md) · [Índice](README.md)

O que mudou em cada versão, e o que isso pode quebrar do seu lado.

A versão ainda é `0.x`, então um salto de minor pode quebrar alguma coisa.
Quando quebra, a entrada diz exatamente o quê e como se adaptar. Nada aqui muda
em silêncio.

---

## Ainda não publicado

### Quebra

- **O `pdfl pack` não empacota mais planilhas** (`.xlsx`, `.xls`, `.ods`), e diz
  qual arquivo ficou de fora. Nenhuma função `data::` abre uma, então um pacote
  que a carregava instalava direitinho e falhava na primeira consulta. Se você
  empacotava planilha, exporte antes para `.csv` ou `.json`.

### Novo

- `--tags TAG` no `run` filtra quais checks rodam. Repetível; um check roda
  quando carrega qualquer uma das tags informadas.
- `--json` no `inspect` e no `lint`, e `--output json` no `doc`. Todo subcomando
  passa a ser legível por programa.
- `--output sarif` e `--output junit`, onde quer que se escolha um formato de
  relatório — `run`, `compare`, `watch` e `fix`. SARIF é o que o GitHub code
  scanning lê; JUnit é o que o painel de testes de qualquer CI lê.
- `pdfl completions <shell>` imprime o script de autocompletar para bash, zsh,
  fish, elvish ou powershell.
- `--quiet` em todos os comandos silencia progresso e confirmações no stderr. Os
  erros continuam aparecendo, e o `print()` fica intacto — aquilo é a saída do
  próprio script, e sumir com ela mudaria o que o script faz.
- `data::load_dataset` e `data::lookup_value` leem `.json` além de `.csv`: um
  array de arrays, ou um array de objetos cujo primeiro objeto nomeia as colunas
  na ordem em que o arquivo as escreve.
- `pdfl test <script>` roda um script sobre uma pasta de PDFs e compara cada
  relatório com o gravado ao lado, então um perfil que passa a achar outra coisa
  quebra um teste em vez de surpreender alguém lá na frente. O `--update` grava
  os relatórios esperados.
- `--jobs <n>` no `pdfl test` roda essa quantidade de casos ao mesmo tempo, cada
  um em seu processo. Oito arquivos de 41 páginas: 8,9s com `--jobs 1`, 1,1s com
  `--jobs 8`. O padrão segue `1`, já que cada job segura um documento na
  memória; `--jobs 0` dá um por CPU.

### Vale saber

- Uma tag que nenhum check carrega é **erro**, não aprovação vazia. Do contrário
  uma pipeline com a tag escrita errada não validaria nada e reportaria arquivo
  limpo.
- Um `rule` não tem tags, então `--tags` o pula — mesma resposta que um check
  sem tag recebe.
- O relatório JSON ganhou `checks_run`, os checks e rules que rodaram. Ele não
  sobe o `schema_version`, porque quem ignora campo desconhecido sobrevive a
  isso. O JUnit precisa dele: os diagnósticos só nomeiam os checks que acharam
  algo, e uma execução limpa reportada como zero testes é, para uma CI, uma
  execução que não aconteceu.

---

## 0.12.0

### Novo

- Scripts recebem valores pela linha de comando: `--var nome=valor`, lidos como
  `vars.nome`. Um valor ausente nomeia a flag que o forneceria, em vez de
  resolver para nada.
- Quatro exemplos completos de comparação entre dois documentos com `visual::`:
  `proof.pdfl`, `reprint.pdfl`, `scope.pdfl` e `intake.pdfl`.

### Quebra

Nada. Um script que não menciona `vars` se comporta exatamente como antes.

---

## 0.11.0

### Quebra

**Os identificadores de diagnóstico mudaram de forma.** Eram `PDFL-001`, um
contador dentro da execução; agora derivam do próprio achado, como
`PDFL-093751a2`.

> Qualquer coisa que case com `PDFL-\d+` deixa de casar. Em troca, o
> identificador sobrevive a um check inserido acima dele — que é o que torna
> possível manter uma linha de base aprovada.

**Entrada ilegível sai com `10` em vez de `2`.** Arquivo corrompido e arquivo
reprovado eram indistinguíveis para uma pipeline.

> Se a sua CI trata `2` como "este arquivo foi reprovado", ela verá `10` quando
> o arquivo nunca chegou a ser julgado. Os achados seguem em `0`, `1` e `2`;
> erro de sintaxe no script segue em `3`.

### Novo

- Um check pode declarar a severidade dos seus achados:
  `check "..." severity: warning { ... }` — `error` (padrão), `warning` ou
  `info`. É isso que dá ao `--fail-on warning` algo sobre o que agir.
- O relatório JSON abre com `schema_version`, para o consumidor saber que forma
  está lendo. Ele sobe só quando quem lia a saída anterior quebraria;
  acrescentar campo não o faz subir.

---

## 0.10.1

### Corrigido

- O relatório em PDF estava parcialmente em português: o cabeçalho da seção dizia
  `Diagnósticos` e cada diagnóstico trazia `(linha N)`. Ambos estão em inglês
  agora, que é o que a documentação sempre prometeu.

---

## 0.10.0

### Quebra

**Os alvos de release passaram de `x64` para `amd64`**, então todo nome de asset
mudou.

**Os arquivos portáteis deixaram de ser publicados**, exceto um para Linux
amd64.

> Qualquer coisa que baixe `pdfl-<versão>-linux-x64.tar.gz`, ou qualquer
> portátil que não seja o Linux amd64, precisa mudar. Em CI, instale pelo
> `.deb` — as receitas desta documentação foram atualizadas para isso — ou use o
> tarball de Linux amd64 onde instalar não for possível.

### Corrigido

- Duas lacunas encontradas auditando a documentação contra o código:
  `text::detect_personal_data` e `text::detect_pii` aceitam uma string opcional
  que não estava documentada, e `fix::reorder_pages` estava escrito de duas
  formas diferentes entre os idiomas.

---

## 0.9.0

### Novo

- Instaladores para todas as plataformas: `.deb` no Linux, `.dmg` no macOS,
  `-setup.exe` e `.msi` no Windows.
- Builds para macOS Intel, compilados cruzado a partir do runner Apple Silicon.

### Corrigido

- O instalador do Windows era construído com caminhos que resolviam para o
  diretório errado, e por isso nunca produzia arquivo.
- Os pacotes de release carregavam os headers C e os arquivos de build do
  pdfium, que só interessam a quem compila contra ele. Cerca de 550 KB por
  pacote.

---

## 0.8.0

### Novo

- Windows x64 entra nas plataformas publicadas.

> A `pdfium.dll` embutida fica em `pdfium\bin`, não em `pdfium\lib`. Se você
> empacotar o `pdfl` por conta própria, mantenha o layout como distribuído: o
> binário procura a biblioteca ao lado de si mesmo.

---

## 0.7.0

### Quebra

**Os assets de release carregam a versão no nome**, como
`pdfl-<versão>-<alvo>.tar.gz`, e o diretório de dentro também.

> `.../releases/latest/download/<nome>` deixa de resolver, porque esse endpoint
> exige o nome exato. Baixe por padrão em vez disso:
> `gh release download --pattern 'pdfl-*-linux-amd64.*'`.

### Novo

- O código, o README e os exemplos estão em inglês. A documentação segue em sete
  idiomas.

---

## v0.6.1

Primeira versão pública. A linguagem, o interpretador e dez comandos de CLI, com
documentação em sete idiomas.

---

[← Receitas](12-receitas.md) · [Índice](README.md)
