# 10. Biblioteca padrão

[← `data::`](09-data.md) · [Índice](README.md) · [Próximo: Comandos do CLI →](11-cli.md)

Métodos de listas e strings, e as funções globais disponíveis em qualquer lugar
do script.

---

## 10.1 Métodos de lista

### Percorrendo

#### `lista.each { |item| ... }`

Executa o bloco para cada item.

```pdfl
check "each" {
  doc.pages.each { |page|
    assert page.width > 0, "página #{page.number} sem largura"
  }
}
```

#### `lista.each_with_index { |item, i| ... }`

Como `each`, mas o segundo parâmetro recebe a posição (começando em **0**).

```pdfl
check "each_with_index" {
  doc.fonts.each_with_index { |font, i|
    print("fonte", i + 1, "de", doc.fonts.length, ":", font.name)
  }
}
```

### Testando

#### `lista.all { |item| ... }`

Verdadeiro se **todos** os itens satisfazem a condição. Lista vazia devolve
verdadeiro.

```pdfl
check "all" {
  require doc.fonts.all { |f| f.is_embedded }
  require doc.pages.all { |p| p.has_trim_box }
}
```

#### `lista.any { |item| ... }`

Verdadeiro se **algum** item satisfaz. Lista vazia devolve falso.

```pdfl
check "any" {
  assert doc.pages.any { |p| p.extract_text() != "" },
    "documento inteiro sem texto"
}
```

#### `lista.contains(valor)`

Verdadeiro se o valor está na lista.

```pdfl
check "contains" {
  require [1, 2, 3].contains(2)
  require prepress::detect_spot_colors().contains("Verniz")
}
```

### Transformando

#### `lista.filter { |item| ... }`

Nova lista só com os itens que satisfazem a condição.

```pdfl
check "filter" {
  ruins = doc.images.filter { |img| img.dpi < 300 }
  assert ruins.length == 0,
    "#{ruins.length} imagem(ns) com resolução baixa"

  // Encadeando: filtrar e depois transformar
  nomes = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  print("fontes soltas:", nomes.join(", "))
}
```

#### `lista.map { |item| ... }`

Nova lista com o resultado do bloco para cada item.

```pdfl
check "map" {
  numeros = doc.pages.map { |p| p.number }
  larguras = doc.pages.map { |p| p.width }
  print("páginas:", numeros.join(", "))
}
```

### Acessando

#### `lista.length`

Quantidade de itens. Funciona como propriedade ou método: `lista.length` e
`lista.length()` são equivalentes.

```pdfl
check "length" {
  require doc.pages.length == doc.page_count
  print("fontes:", doc.fonts.length)
}
```

#### `lista.get(n)`

O n-ésimo item, **1-based**. Erro amigável se o índice não existe.

```pdfl
check "get" {
  linha = data::load_dataset("dados/lotes.csv").get(2)   // segunda linha
  print("primeira coluna:", linha.get(1))
}
```

#### `lista.first()` e `lista.last()`

Primeiro e último item. Devolvem `null` em lista vazia (sem erro).

```pdfl
check "first e last" {
  primeira = doc.pages.first()
  ultima = doc.pages.last()
  print("da página", primeira.number, "até a", ultima.number)

  // Seguro em lista vazia: null é falso
  spots = prepress::detect_spot_colors()
  assert !spots.first() || spots.first() == "Verniz",
    "tinta especial inesperada: #{spots.first()}"
}
```

#### `lista.join([separador])`

Junta os itens em texto. Separador padrão: `", "`.

```pdfl
check "join" {
  print(doc.fonts.map { |f| f.name }.join(", "))
  print(prepress::get_page_boxes(1).join(" | "))
  print([1, 2, 3].join(" -> "))
}
```

---

## 10.2 Métodos de string

| Método | O que faz |
|---|---|
| `texto.contains(sub)` | Contém o trecho? |
| `texto.starts_with(sub)` | Começa com? |
| `texto.ends_with(sub)` | Termina com? |
| `texto.trim()` | Remove espaços das pontas |
| `texto.to_uppercase()` | Tudo em maiúsculas |
| `texto.to_lowercase()` | Tudo em minúsculas |
| `texto.length` | Quantidade de caracteres |

```pdfl
check "Métodos de string" {
  titulo = doc.title

  require titulo.length > 0
  require titulo.trim() == titulo          // sem espaços sobrando
  assert !titulo.to_lowercase().contains("rascunho"),
    "título ainda marcado como rascunho"

  codigo = codes::decode_barcode(1)
  assert codigo.starts_with("789"), "GTIN não brasileiro"

  arquivo = doc.filename
  assert arquivo.ends_with(".pdf"), "extensão inesperada"
}
```

Diferença entre `contains` de string e de lista:

```pdfl
check "contains em cada tipo" {
  // string: procura um TRECHO dentro do texto
  require "documento final".contains("final")

  // lista: procura um ITEM inteiro
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" não é item da lista
}
```

---

## 10.3 Funções globais

### `min(a, b)` e `max(a, b)`

```pdfl
check "min e max" {
  larguras = doc.pages.map { |p| p.width }
  // Reduzindo uma lista com each
  menor = 99999
  doc.pages.each { |p| menor = min(menor, p.width) }
  print("página mais estreita:", menor, "pt")
}
```

### `abs(x)`

Valor absoluto — essencial para comparar dimensões com tolerância.

```pdfl
check "abs" {
  const A4_LARGURA = 595.0
  const TOLERANCIA = 5.0

  doc.pages.each { |page|
    // "a diferença, para mais ou para menos, é menor que a tolerância"
    assert abs(page.width - A4_LARGURA) < TOLERANCIA,
      "página #{page.number} fora do A4: #{page.width}pt"
  }
}
```

### `round(x)`

Arredonda para o inteiro mais próximo. Útil para deixar as mensagens legíveis.

```pdfl
check "round" {
  doc.images.each { |img|
    // sem round: "217.4453125 DPI" — com round: "217 DPI"
    assert img.dpi >= 300,
      "imagem na página #{img.page_number}: #{round(img.dpi)} DPI"
  }

  mb = struct::file_size() / 1024 / 1024
  print("tamanho:", round(mb), "MB")
}
```

### `print(...)`

Imprime valores separados por espaço. **Sai no stderr**, então não polui o
relatório no stdout — dá para usar `> relatorio.json` sem misturar.

```pdfl
check "print" {
  print("documento:", doc.filename)
  print("páginas:", doc.page_count, "| fontes:", doc.fonts.length)

  // Útil para investigar antes de escrever a validação definitiva
  doc.images.each { |img|
    print("imagem", img.width, "x", img.height, "@", round(img.dpi), "DPI")
  }
}
```

### `region(x, y, largura, altura [, nome])`

Cria uma região. Documentada no [capítulo 2](02-tipos.md#25-region--área-da-página).

---

## 10.4 Padrões úteis

### Contar quantos itens falham

```pdfl
check "Contagem de problemas" {
  ruins = doc.images.filter { |i| i.dpi < 300 }
  assert ruins.length == 0,
    "#{ruins.length} de #{doc.images.length} imagens abaixo de 300 DPI"
}
```

### Listar os itens que falharam na mensagem

```pdfl
check "Lista na mensagem" {
  // Encadeamentos ficam na mesma linha: o ponto precisa vir logo
  // após o valor anterior, sem quebra de linha no meio.
  problemas = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }

  assert problemas.length == 0,
    "páginas sem TrimBox: #{problemas.join(", ")}"
}
```

### Validar com tolerância

```pdfl
function proximo(valor, alvo, tolerancia) {
  abs(valor - alvo) < tolerancia
}

check "Com tolerância" {
  doc.pages.each { |page|
    assert proximo(page.width, 595.0, 2.0),
      "página #{page.number}: largura #{page.width}pt (esperado 595 ± 2)"
  }
}
```

### Evitar erro em documento vazio

```pdfl
check "Defensivo" {
  // O curto-circuito evita avaliar first() em lista vazia
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "primeira página sem largura"
}
```

---

[← `data::`](09-data.md) · [Índice](README.md) · [Próximo: Comandos do CLI →](11-cli.md)
