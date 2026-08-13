# 1. A linguagem PDFLang

[← Índice](README.md) · [Próximo: Tipos do documento →](02-tipos.md)

PDFLang foi desenhada para ser lida por quem não programa. Não há classes,
herança, tipos declarados nem ponto-e-vírgula. Um script é uma lista de
verificações escritas quase em português.

---

## 1.1 A estrutura de um script

```pdfl
// Comentários começam com duas barras e vão até o fim da linha.

profile "nome-do-perfil" {        // profile é opcional: agrupa e nomeia o
                                  // conjunto; o nome aparece no relatório.

  const LIMITE = 300%             // constantes: convenção de MAIÚSCULAS

  check "Nome do Check" {         // cada check vira uma seção do relatório
    require doc.page_count > 0    // uma validação
  }

  check "Outro Check" {           // quantos checks você quiser
    require doc.title != ""
  }
}
```

O `profile` é opcional — um script pode ter apenas checks soltos:

```pdfl
check "Simples" {
  require doc.page_count > 0
}
```

### Tags nos checks

Tags servem para organizar e filtrar visualmente os checks no relatório:

```pdfl
check "TAC dentro do limite" tags: ["prepress", "cores"] {
  require prepress::validate_tac_limits(300)
}
```

### Severidade de um check

Por padrão um check que falha é **erro** e a execução sai com 2. Um check pode
se declarar consultivo:

```pdfl
check "Resolução das imagens" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

`error` (o padrão), `warning` e `info`. Aviso e informação não reprovam a
execução — saem com 1 e 0 — a menos que você passe `--fail-on warning`, que é
como a CI decide o rigor sem alterar o script.

`tags:` e `severity:` podem vir em qualquer ordem.

> Um erro de execução dentro do check — variável escrita errada, arquivo
> ausente — continua sendo erro, independentemente do que o check declarou. Um
> script quebrado não é consultivo.

---

## 1.2 As duas formas de validar

Toda validação usa `require` ou `assert`. A diferença é só a mensagem que
aparece no relatório quando a validação falha.

```pdfl
check "Comparando as duas formas" {

  // require: a mensagem é gerada da própria expressão.
  // Se falhar, o relatório mostra:
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert: você escreve a mensagem que o usuário final vai ler.
  // Se falhar, o relatório mostra exatamente:
  //   "PDF sem título nos metadados"
  assert doc.title != "", "PDF sem título nos metadados"
}
```

**Regra prática:** use `require` para verificações óbvias (a expressão já se
explica) e `assert` quando quem lê o relatório precisa entender o problema sem
conhecer o script.

### Uma falha não interrompe as outras

```pdfl
check "Três validações independentes" {
  assert doc.page_count > 100, "poucas páginas"    // falha
  assert doc.title != "", "sem título"             // roda mesmo assim
  assert doc.author != "", "sem autor"             // esta também
}
```

O relatório traz **todos** os problemas de uma vez. Isso é proposital: quem
recebe o arquivo de volta quer a lista completa de correções, não uma por vez.

O mesmo vale entre checks — se um check der erro de execução (por exemplo, uma
variável que não existe), ele vira um diagnóstico e os demais continuam rodando.

---

## 1.3 Valores e tipos

### Números e unidades

```pdfl
check "Números" {
  x = 42          // inteiro
  y = 2.5         // número com decimais

  // Unidades de medida viram PONTOS automaticamente (1 pt = 1/72 pol):
  a = 3mm         // 8.5039... pt
  b = 2.5cm       // 70.866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // Porcentagem mantém o valor numérico:
  limite = 300%   // 300

  require a < b            // dá para comparar direto, tudo é ponto
  require c == 72.0
  require limite == 300
}
```

Escrever `3mm` em vez de `8.504` é o ponto: o script fica legível para quem
pensa em milímetros, e a conversão não sai errada.

### Textos

```pdfl
check "Strings" {
  simples = "texto comum"

  // Interpolação: #{...} insere o valor de qualquer expressão
  nome = "documento.pdf"
  mensagem = "Analisando #{nome} com #{doc.page_count} páginas"

  // Escapes: \n (nova linha), \t (tabulação), \" (aspas), \\ (barra)
  com_aspas = "ele disse \"olá\""

  // Barras invertidas desconhecidas passam direto — isso permite escrever
  // expressões regulares sem escape duplo:
  padrao = "\d{3}\.\d{3}\.\d{3}-\d{2}"    // CPF

  require mensagem.contains("páginas")
}
```

### Booleanos e o que é "verdadeiro"

```pdfl
check "Verdadeiro e falso" {
  sim = true
  nao = false

  // Só false e null são falsos. Todo o resto é verdadeiro —
  // inclusive 0, string vazia e lista vazia.
  require 0        // passa (zero é verdadeiro)
  require ""       // passa (string vazia é verdadeira)

  // Por isso, para testar conteúdo, compare explicitamente:
  require doc.title != ""              // certo
  require doc.pages.length > 0         // certo
}
```

Isso importa em funções que devolvem `null` quando não encontram nada:

```pdfl
check "Aproveitando o null" {
  descricao = data::lookup_value("lotes.csv", "L2026-08")
  // null é falso, então isto funciona diretamente:
  assert descricao, "lote não encontrado na tabela"
}
```

### Listas

```pdfl
check "Listas" {
  numeros = [1, 2, 3]
  textos = ["a", "b", "c"]
  misto = [1, "dois", true]

  require numeros.length == 3
  require numeros.contains(2)
  require textos.join(", ") == "a, b, c"

  // Acesso é 1-based: o primeiro item é o item 1
  require numeros.get(1) == 1
  require numeros.first() == 1
  require numeros.last() == 3
}
```

---

## 1.4 Operadores

```pdfl
check "Operadores" {
  // Comparação
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // Aritmética
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // divisão inexata vira número com decimais
  require 10 / 5 == 2          // exata continua inteiro

  // Lógica (com curto-circuito: o lado direito só é avaliado se necessário)
  require true && true
  require false || true
  require !false

  // Curto-circuito na prática: se não há páginas, a segunda parte
  // nem é avaliada — evita erro em documento vazio.
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 Blocos: repetindo para cada item

Blocos são trechos entre chaves que recebem um parâmetro entre barras verticais.
É como se lê em português: "para cada página, faça...".

```pdfl
check "Percorrendo páginas" {

  // each: executa o bloco para cada item
  doc.pages.each { |page|
    assert page.width > 0, "página #{page.number} sem largura"
  }

  // each_with_index: além do item, recebe a posição (0, 1, 2...)
  doc.fonts.each_with_index { |font, i|
    print("fonte", i, ":", font.name)
  }

  // all: verdadeiro se TODOS os itens satisfazem a condição
  require doc.fonts.all { |f| f.is_embedded }

  // any: verdadeiro se ALGUM item satisfaz
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter: devolve só os itens que satisfazem
  sem_texto = doc.pages.filter { |p| p.extract_text() == "" }
  assert sem_texto.length == 0,
    "#{sem_texto.length} página(s) sem texto"

  // map: transforma cada item, devolvendo uma nova lista
  nomes = doc.fonts.map { |f| f.name }
  print("fontes usadas:", nomes.join(", "))
}
```

Blocos podem ser encadeados — **na mesma linha**, sem quebra antes do ponto:

```pdfl
check "Encadeando" {
  // fontes não embutidas, só os nomes, unidos por vírgula
  problemas = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problemas.length == 0,
    "fontes não embutidas: #{problemas.join(", ")}"
}
```

Se a linha ficar longa demais, quebre em etapas nomeadas em vez de quebrar o
encadeamento — fica mais legível de qualquer forma:

```pdfl
check "Etapas nomeadas" {
  soltas = doc.fonts.filter { |f| !f.is_embedded }
  nomes = soltas.map { |f| f.name }
  assert nomes.length == 0, "fontes não embutidas: #{nomes.join(", ")}"
}
```

---

## 1.6 Functions: dando nome às suas regras

Quando a mesma verificação aparece em vários lugares, dê um nome a ela:

```pdfl
// O valor da function é o da ÚLTIMA expressão — não existe "return".
function eh_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function excede_tac(page, limite) {
  page.tac > limite
}

check "Formato e tinta" {
  // agora o check se lê quase como uma frase
  require doc.pages.all { |p| eh_a4(p) }

  doc.pages.each { |page|
    assert !excede_tac(page, 300), "página #{page.number} com tinta demais"
  }
}
```

Regras das functions:

- Os parâmetros existem só dentro da function.
- Podem chamar outras functions.
- Recursão é permitida, mas limitada a 200 chamadas (evita travar o processo).

---

## 1.7 Imports: reaproveitando entre perfis

Coloque as regras comuns em um arquivo e importe onde precisar.

`biblioteca.pdfl`:

```pdfl
// Constantes e functions compartilhadas pela equipe
const TAC_OFFSET = 300%
const SANGRIA_PADRAO = 3mm

function pagina_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`revista.pdfl`:

```pdfl
// O caminho é relativo a ESTE arquivo
import "biblioteca.pdfl"

check "Formato" {
  // TAC_OFFSET e pagina_a4 vieram do import
  require doc.pages.all { |p| pagina_a4(p) }
  require prepress::validate_tac_limits(TAC_OFFSET)
}
```

Cada arquivo é carregado **uma única vez**, mesmo que vários scripts o importem
— então importações circulares não travam.

---

## 1.8 Regras (`rule`): validar página a página

Uma `rule` é um check que roda uma vez para cada página, com a página já
disponível na variável `page`:

```pdfl
// Sem "on": roda em todas as páginas
rule "Toda página tem texto" {
  assert page.extract_text().trim() != "",
    "página #{page.number} está em branco"
}
```

Com `on`, você escolhe em quais páginas a regra se aplica:

```pdfl
rule "Miolo numerado" on doc.pages.filter { |p| p.number > 2 } {
  rodape = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, rodape) != "",
    "página #{page.number} sem numeração no rodapé"
}
```

> **Atenção à sintaxe:** se a seleção do `on` terminar em uma propriedade
> (ex.: `on doc.pages`), envolva-a em parênteses — sem elas, a chave `{` do
> corpo seria interpretada como bloco daquela chamada:
>
> ```pdfl
> rule "Exemplo" on (doc.pages) {     // com parênteses
>   require page.width > 0
> }
> ```

---

## 1.9 Variáveis e escopo

```pdfl
const GLOBAL = 100          // visível no arquivo inteiro

check "Escopo" {
  local = 42                // visível só neste check

  doc.pages.each { |page|
    dentro = page.width     // visível só dentro do bloco
    require dentro > 0
  }

  require local == 42       // ainda visível
  require GLOBAL == 100     // ainda visível
}
```

Convenção: constantes em MAIÚSCULAS, variáveis em minúsculas. A linguagem não
obriga, mas os exemplos e perfis prontos seguem isso.

---

### Valores vindos da linha de comando

`--var nome=valor` no `pdfl run`, `pdfl test` e `pdfl watch` chega ao script como
`vars.nome`, sempre como texto. `test` e `watch` repassam o mesmo valor para
cada caso ou arquivo — um nome de cliente para a execução inteira, não um por
arquivo. É o que evita que um perfil vire cinco cópias quase iguais:

```pdfl
check "Job confere com o pedido" {
  assert doc.title.contains(vars.pedido),
    "o arquivo diz \"#{doc.title}\", o pedido é #{vars.pedido}"
}
```

```bash
pdfl run entrada.pdfl recebido.pdf --var pedido=SO-4471
```

Um nome que não foi passado é **erro, e o erro nomeia a flag que o forneceria** —
não string vazia: um check comparando contra nada passaria e reportaria um
arquivo que ninguém validou.

---

## 1.10 Mensagens que ajudam quem recebe o arquivo

A qualidade do relatório depende das mensagens que você escreve. Compare:

```pdfl
check "Mensagens ruins" {
  require doc.pages.all { |p| p.tac <= 300 }
  // relatório: "requirement not met: doc.pages.all() { ... }"
  // — quem recebe não sabe qual página nem quanto excedeu
}

check "Mensagens boas" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Página #{page.number}: cobertura de tinta #{page.tac}% (máximo 300%)"
  }
  // relatório: "Página 7: cobertura de tinta 324% (máximo 300%)"
  // — o operador sabe exatamente o que corrigir
}
```

Use `print()` para informação de contexto que não é erro. Ela sai no stderr,
então não polui o relatório:

```pdfl
check "Contexto" {
  print("Analisando", doc.page_count, "páginas")
  print("Fontes:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 Erros comuns

As mensagens do `pdfl` são em inglês; a tabela liga cada uma à causa.

| Mensagem | Causa | Correção |
|---|---|---|
| `expected end of line after statement` | dois comandos na mesma linha | um comando por linha |
| `unknown variable: x` | uso antes de atribuir, ou fora do escopo | declare antes, no mesmo nível |
| `unknown function: text::xyz` | nome errado ou função inexistente | veja o capítulo do namespace |
| `fix:: is only available in the 'pdfl fix' command` | `fix::` em `pdfl run` | use `pdfl fix entrada.pdf script.pdfl --output saida.pdf` |
| `unknown unit: 'kg'` | sufixo inválido | use `pt`, `mm`, `cm`, `in` ou `%` |
| `expected '{' with the rule body` | `on` com seleção terminando em propriedade | envolva a seleção em parênteses |
| `unexpected expression: Dot` | encadeamento quebrado em várias linhas | mantenha `.metodo` na mesma linha, ou use variáveis intermediárias |

Antes de rodar, vale sempre:

```bash
pdfl lint meu_perfil.pdfl    # aponta variáveis não usadas, checks duplicados...
pdfl fmt meu_perfil.pdfl     # padroniza a formatação
```

---

[← Índice](README.md) · [Próximo: Tipos do documento →](02-tipos.md)
