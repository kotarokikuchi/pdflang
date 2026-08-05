# 7. Namespace `codes::` — códigos de barras e QR

[← `prepress::`](06-prepress.md) · [Índice](README.md) · [Próximo: `fix::` →](08-fix.md)

13 funções para detectar, decodificar e validar códigos de barras e QR codes
impressos no documento.

> O escaneamento renderiza as páginas em alta resolução e roda **uma única vez**,
> no primeiro uso de qualquer função `codes::`. Scripts que não usam o namespace
> não pagam esse custo.

Formatos reconhecidos: EAN-8/13, UPC-A/E, Code 128, Code 39, ITF, QR Code,
Data Matrix, Aztec, PDF417, entre outros.

---

## 7.1 Detecção

### `codes::detect_barcodes()` e `codes::detect_qrcodes()`

```pdfl
check "Embalagem tem código" {
  assert codes::detect_barcodes(),
    "nenhum código de barras encontrado na arte"

  // QR de rastreabilidade
  assert codes::detect_qrcodes(),
    "faltou o QR code de rastreabilidade"
}
```

### `codes::count_barcodes()`

Quantidade total de códigos detectados (barras + QR).

```pdfl
check "Quantidade de códigos" {
  total = codes::count_barcodes()
  print("códigos detectados:", total)

  assert total == 2,
    "esperava 2 códigos (EAN + QR), encontrei #{total}"
}
```

### `codes::get_barcode_type(n)`

Formato do n-ésimo código (1-based): `"EAN_13"`, `"QR_CODE"`, `"CODE_128"`...

```pdfl
check "Tipo do código principal" {
  tipo = codes::get_barcode_type(1)
  assert tipo == "EAN_13",
    "o código principal deveria ser EAN-13, é #{tipo}"
}

check "Listando todos" {
  // Percorre pelos índices, de 1 até a quantidade
  print("primeiro:", codes::get_barcode_type(1))
  print("segundo:", codes::get_barcode_type(2))
}
```

### `codes::get_barcode_location(n)`

Onde o código está: `[pagina, x, y]` em pontos (origem no canto inferior
esquerdo).

```pdfl
check "Posição do código" {
  local = codes::get_barcode_location(1)
  print("página:", local.get(1), "x:", local.get(2), "y:", local.get(3))

  // O código deve estar na primeira página
  assert local.first() == 1,
    "código de barras não está na capa"
}
```

---

## 7.2 Decodificação e validação

### `codes::decode_barcode(n)`

O conteúdo decodificado do n-ésimo código.

```pdfl
check "Conteúdo do código" {
  codigo = codes::decode_barcode(1)
  print("código lido:", codigo)
  assert codigo.starts_with("789"),
    "GTIN não é brasileiro (deveria começar com 789): #{codigo}"
}
```

### `codes::validate_barcode_checksum(n)`

Valida o dígito verificador GTIN do n-ésimo código detectado.

```pdfl
check "Dígito verificador" {
  // Um GTIN com dígito errado é rejeitado no caixa do supermercado
  assert codes::validate_barcode_checksum(1),
    "dígito verificador inválido no código #{codes::decode_barcode(1)}"
}
```

### `codes::validate_gtin(texto)` e `codes::validate_ean(texto)`

Sinônimos. Validam o dígito verificador de uma **string** (EAN-8/13, UPC-A,
GTIN-14) — útil para conferir um número vindo de outra fonte.

```pdfl
check "GTIN informado no texto" {
  require codes::validate_gtin("7891234567895")
  require !codes::validate_gtin("7891234567890")   // dígito errado

  // Conferindo o número impresso abaixo das barras
  impresso = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(impresso),
    "o número impresso não é um GTIN válido: #{impresso}"
}
```

### `codes::validate_code128()`

Verdadeiro se há algum Code 128 decodificado com sucesso (o checksum do Code 128
é validado na própria decodificação).

```pdfl
check "Código logístico" {
  assert codes::validate_code128(),
    "faltou o Code 128 de logística"
}
```

---

## 7.3 Conferência cruzada

### `codes::compare_barcode_with_text()`

Verdadeiro se o conteúdo de **todos** os códigos aparece no texto do documento.

Este é o teste que pega o erro mais caro da indústria: o código de barras
apontando para um produto e o texto impresso dizendo outro.

```pdfl
check "Código confere com o texto impresso" {
  assert codes::compare_barcode_with_text(),
    "o número do código de barras não aparece no texto — arte com dados trocados?"
}
```

### `codes::validate_barcode_format(regex)`

Verdadeiro se o conteúdo de todos os códigos casa com a expressão regular.

```pdfl
check "Formato esperado" {
  // Só EAN-13: exatamente 13 dígitos
  assert codes::validate_barcode_format("^\d{13}$"),
    "há código fora do padrão EAN-13"
}

check "QR aponta para o site oficial" {
  assert codes::validate_barcode_format("^https://empresa\.com\.br/.*"),
    "QR code aponta para endereço não autorizado"
}
```

### `codes::validate_barcode_position(regiao)` ou `(x0, y0, x1, y1)`

Verdadeiro se todos os códigos estão dentro da área. Aceita uma `region` ou
quatro números em pontos.

```pdfl
check "Código na área reservada" {
  // Com região nomeada — mais legível
  area = region(400, 20, 180, 80, "área do código")
  assert codes::validate_barcode_position(area),
    "código de barras fora da área reservada da embalagem"
}

check "Com coordenadas diretas" {
  // x0, y0, x1, y1 em pontos
  assert codes::validate_barcode_position(400, 20, 580, 100),
    "código fora da posição especificada"
}
```

---

## 7.4 Exemplo completo

```pdfl
// bula_farmaceutica.pdfl — validação de código de lote em bula
// Uso: pdfl run bula_farmaceutica.pdfl bula.pdf
profile "bula-anvisa" {

  check "Presença dos códigos" tags: ["codes"] {
    assert codes::detect_barcodes(), "bula sem código de barras"
    assert codes::count_barcodes() >= 1,
      "esperava ao menos o EAN do produto"
  }

  check "Integridade do código" tags: ["codes"] {
    codigo = codes::decode_barcode(1)
    tipo = codes::get_barcode_type(1)
    print("código:", tipo, "=", codigo)

    assert tipo == "EAN_13", "código principal não é EAN-13 (é #{tipo})"
    assert codes::validate_barcode_checksum(1),
      "dígito verificador inválido: #{codigo}"
    assert codigo.starts_with("789"),
      "GTIN não é brasileiro: #{codigo}"
  }

  check "Conferência com o texto" tags: ["codes", "critico"] {
    // O erro mais caro: código de um produto, texto de outro
    assert codes::compare_barcode_with_text(),
      "número do código não aparece no texto da bula"
  }

  check "Posição na arte" tags: ["codes", "layout"] {
    area_reservada = region(400, 20, 180, 90, "área do código")
    assert codes::validate_barcode_position(area_reservada),
      "código fora da área reservada — pode ser cortado no acabamento"
  }

  check "Cruzamento com a base de produtos" tags: ["dados"] {
    // Integra com data:: — veja o capítulo 9
    codigo = codes::decode_barcode(1)
    produto = data::query_gtin(codigo)
    assert produto,
      "GTIN #{codigo} não consta na base de produtos homologados"
    print("produto:", produto.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [Índice](README.md) · [Próximo: `fix::` →](08-fix.md)
