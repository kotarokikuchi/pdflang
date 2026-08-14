# 13. Mudanças

[← Receitas](12-receitas.md) · [Índice](README.md)

O que mudou em cada versão, e o que isso pode quebrar do seu lado.

A versão ainda é `0.x`, então um salto de minor pode quebrar alguma coisa.
Quando quebra, a entrada diz exatamente o quê e como se adaptar. Nada aqui muda
em silêncio.

---

## 0.18.0

### Quebra

**O identificador de um achado de `loading` muda.** Ele é derivado da mensagem,
e a mensagem não carrega mais as quebras de linha que o pdfium colocava nela.

> Só achados de um documento que não pôde ser lido são afetados — nenhum check
> seu produz um desses. Se alguma baseline aprova esse identificador, ele
> precisa ser registrado de novo. É por isso que esta versão é minor e não
> patch: identificador é a base de uma baseline, então não muda em silêncio.

### Corrigido

- O `pdfl pack` gravava um arquivo aninhado com o separador da máquina que
  montou o pacote — no Windows, `dados\lotes.csv`. Um pacote é montado numa
  máquina e instalado em outra, então qualquer coisa diferente de `/` é um
  pacote que instala e depois não acha os próprios arquivos; nem a nossa
  verificação achava. Pacotes montados em Linux e macOS nunca foram afetados.
- O `--output` e o `--output-file` eram ignorados quando o PDF não podia ser
  lido. O `run`, o `fix` e o `compare` imprimiam JSON no stdout qualquer que
  fosse o pedido, então uma pipeline que pedia JUnit em arquivo não recebia
  arquivo nenhum e sim um relatório num fluxo que ela não estava lendo — um
  arquivo corrompido parecia uma execução que nunca aconteceu, embora o código
  de saída dissesse `10` corretamente. O relatório de falha agora sai pelo
  mesmo caminho de todos os outros.
- Um erro do pdfium chegava ao relatório como um enum do Rust impresso em três
  linhas. Um diagnóstico é um campo de uma linha de CSV e um atributo de XML,
  então agora ele é dobrado numa única linha.

---

## 0.17.0

### Novo

- O `pdfl pixelcompare` mostra uma barra de progresso em cada etapa —
  rasterizar cada arquivo, comparar, gravar o visualizador. Ela só é desenhada
  quando o stderr é um terminal, porque a barra sobrescreve a própria linha e um
  arquivo de log não tem cursor para mover; redirecionada, fica em silêncio. O
  `--quiet` silencia em qualquer caso.
- O `--jobs <n>` no `pdfl pixelcompare` compara essa quantidade de páginas ao
  mesmo tempo, e por padrão usa uma por CPU: 41 páginas a 150 dpi caem de 3,6s
  para 1,2s. Só a comparação é paralela — o pdfium serializa toda chamada atrás
  de um único lock global, então a rasterização não pode ser, e é por isso que o
  ganho é de três vezes e não de oito. O relatório não muda com o valor: as
  páginas são recompostas na ordem de página, então os diagnósticos, sua ordem e
  suas impressões digitais saem idênticos — e os arquivos do visualizador
  também.

  > Aqui o padrão é uma por CPU, enquanto `test` e `watch` usam `--jobs 1`. Lá
  > cada job é um processo filho segurando o próprio documento; aqui as páginas
  > já estão na memória e as threads as compartilham.

---

## 0.16.0

### Novo

- O `pdfl pixelcompare` compara dois PDFs pela aparência, e não pelo texto,
  página por página, e informa a fração de pixels que difere. Ele alinha uma
  página que apenas se deslocou antes de comparar, para que um pixel fora do
  lugar não enterre a mudança que importa.
- O `--viewer <pasta>` do `pixelcompare` grava uma aplicação autocontida — sem
  CDN, sem bundler, sem servidor — para varrer, alternar ou misturar os dois
  arquivos com as diferenças pintadas no lugar: vermelho para tinta que sumiu,
  verde para tinta nova, azul para o mesmo peso em outra cor. A faixa de páginas
  filtra para **Changed only**, e as setas seguem o filtro — num documento longo,
  passar pelas páginas que não mexeram é a parte lenta.

---

## 0.15.0

### Novo

- `if` / `else if` / `else`, como **expressão**: o valor é a última expressão do
  ramo que rodou, a mesma regra que uma função já segue. Serve tanto como valor
  (`const LIMITE = if couche { 300 } else { 260 }`) quanto como guarda em volta
  de comandos, sem uma segunda sintaxe. Um ramo que não roda devolve `null`, e
  cada ramo tem escopo próprio — atribuir a uma variável que já existe fora
  continua atualizando aquela.

---

## 0.14.0

### Corrigido

- O `--var` agora chega ao `pdfl test` e ao `pdfl watch`, não só ao `pdfl run`.
  Nenhum dos dois repassava para os filhos que disparam, então um script que lê
  `vars.*` não podia ser testado nem observado: todo caso ou arquivo falhava com
  "was not provided", não importa o conteúdo.

---

## 0.13.0

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
- `--jobs <n>` também no `pdfl watch`: os arquivos são validados por processos
  filhos, então a passada em lote escala igual (9,5s para 1,2s em oito arquivos
  de 41 páginas). O laudo escrito é idêntico independente do `--jobs`.
- `--events` no `pdfl watch` espera pelas notificações do sistema de arquivos em
  vez de por tempo. É opt-in, não o padrão: o inotify num mount NFS ou SMB só
  reporta o que a máquina local escreve, então uma pasta de rede ficaria muda.
  Se o observador não puder ser criado, o watch avisa e volta para o tempo.
- `--journal <arquivo>` no `pdfl watch`: registro append-only do que foi
  validado, um objeto JSON por arquivo. Rodar de novo com o mesmo journal pula
  os arquivos que ele cobre — um lote interrompido no quatro mil de cinco mil
  termina os mil que faltam — sem deixar de reportar os veredictos, então um
  lote retomado nunca diz que a pasta está limpa.
- `--timeout <s>` no `pdfl watch` mata a análise de um arquivo depois desse
  tanto de segundos e o reporta como recusado — um achado,
  `check_name: "timeout"` — em vez de deixar o lote travado. A recursão num
  script `.pdfl` já tem limite de profundidade, então isto é para o que um
  script não pode causar: o pdfium entrando em loop ou travando num PDF
  malformado ou adversário.

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

### Corrigido

- O `pdfl watch` agora acorda quando o arquivo mais novo terminou de assentar,
  em vez de até um intervalo inteiro depois. Com `--debounce 3000`, um arquivo
  que chega é reportado uns 3s depois, e não até 6s.

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
