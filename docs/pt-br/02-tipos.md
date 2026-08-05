# 2. Tipos do documento

[← A linguagem](01-linguagem.md) · [Índice](README.md) · [Próximo: `text::` →](03-text.md)

Todo script recebe automaticamente a variável `doc`, que representa o PDF em
análise. A partir dela você chega às páginas, fontes e imagens.

---

## 2.1 `doc` — o documento

### Propriedades

| Propriedade | Tipo | O que é |
|---|---|---|
| `doc.page_count` | número | Quantidade de páginas |
| `doc.title` | texto | Título dos metadados (vazio se ausente) |
| `doc.author` | texto | Autor dos metadados (vazio se ausente) |
| `doc.filename` | texto | Nome do arquivo analisado |
| `doc.pages` | lista | Todas as páginas |
| `doc.fonts` | lista | Todas as fontes usadas |
| `doc.images` | lista | Todas as imagens de todas as páginas |

```pdfl
check "Propriedades do documento" {
  print("arquivo:", doc.filename)
  print("páginas:", doc.page_count)
  print("título:", doc.title)

  // As coleções são listas comuns — aceitam todos os métodos de lista
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0
  print("imagens no documento inteiro:", doc.images.length)
}
```

### Métodos

#### `doc.extract_text()`

Todo o texto do documento, com as páginas separadas por quebra de linha.

```pdfl
check "Texto do documento" {
  texto = doc.extract_text()
  assert texto.trim() != "", "PDF sem texto extraível (só imagens?)"
  require texto.contains("Contrato")
  print("caracteres no total:", texto.length)
}
```

---

## 2.2 `page` — a página

Páginas vêm de `doc.pages` (dentro de blocos) ou da variável `page` (dentro de
uma `rule`).

### Propriedades

| Propriedade | Tipo | O que é |
|---|---|---|
| `page.number` | número | Número da página, começando em **1** |
| `page.index` | número | Índice da página, começando em **0** |
| `page.width` | número | Largura em pontos |
| `page.height` | número | Altura em pontos |
| `page.images` | lista | Imagens desta página |
| `page.tac` | número | Cobertura de tinta máxima estimada (%) |
| `page.ink_coverage` | número | Cobertura média de tinta estimada (%) |
| `page.min_stroke_width` | número/null | Menor espessura de traço (pt); `null` se não há traços |
| `page.has_media_box` | booleano | Tem MediaBox definida |
| `page.has_crop_box` | booleano | Tem CropBox definida |
| `page.has_trim_box` | booleano | Tem TrimBox definida |
| `page.has_bleed_box` | booleano | Tem BleedBox definida |
| `page.has_art_box` | booleano | Tem ArtBox definida |

```pdfl
check "Formato das páginas" {
  doc.pages.each { |page|
    // number é o que o usuário vê; index é para cálculos internos
    assert page.width > 100mm,
      "página #{page.number} estreita demais: #{page.width}pt"

    // Caixas: essenciais para impressão
    assert page.has_trim_box,
      "página #{page.number} sem TrimBox (área de corte)"
    assert page.has_bleed_box,
      "página #{page.number} sem BleedBox (sangria)"
  }
}

check "Tinta e traços" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "página #{page.number}: #{page.tac}% de tinta (limite 300%)"

    // min_stroke_width pode ser null (página sem traços) —
    // null é falso, então este teste é seguro:
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "página #{page.number} tem traço fino demais"
  }
}
```

### Métodos

#### `page.extract_text()`

Texto apenas desta página.

```pdfl
check "Páginas em branco" {
  brancas = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert brancas.length == 0,
    "#{brancas.length} página(s) em branco: #{brancas.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — a fonte

Fontes vêm de `doc.fonts`.

| Propriedade | Tipo | O que é |
|---|---|---|
| `font.name` | texto | Nome da fonte |
| `font.is_embedded` | booleano | Está embutida no arquivo |

```pdfl
check "Fontes embutidas" {
  // Fonte não embutida é substituída pelo leitor — o texto muda de aparência
  doc.fonts.each { |font|
    assert font.is_embedded,
      "fonte '#{font.name}' não está embutida no PDF"
  }
}

check "Relatório de fontes" {
  print("fontes usadas:", doc.fonts.map { |f| f.name }.join(", "))
  faltando = doc.fonts.filter { |f| !f.is_embedded }
  print("não embutidas:", faltando.length)
}
```

---

## 2.4 `image` — a imagem

Imagens vêm de `doc.images` (todas) ou `page.images` (de uma página).

| Propriedade | Tipo | O que é |
|---|---|---|
| `image.width` | número | Largura em **pixels** |
| `image.height` | número | Altura em **pixels** |
| `image.dpi` | número | Resolução efetiva (o menor entre dpi_x e dpi_y) |
| `image.dpi_x` | número | Resolução horizontal efetiva |
| `image.dpi_y` | número | Resolução vertical efetiva |
| `image.color_space` | texto | `DeviceRGB`, `DeviceCMYK`, `Indexed`... |
| `image.page_number` | número | Página onde aparece (1-based) |
| `image.bits_per_pixel` | número | Bits por pixel |

> **O DPI é o efetivo**, calculado como pixels ÷ tamanho impresso na página —
> não o valor nominal gravado nos metadados. É o número que importa para
> qualidade de impressão: uma imagem de 1000 px esticada para ocupar 20 cm tem
> DPI baixo, mesmo que os metadados digam outra coisa.

```pdfl
profile "imagens-para-offset" {
  const DPI_MINIMO = 300

  check "Resolução" {
    doc.images.each { |img|
      assert img.dpi >= DPI_MINIMO,
        "imagem #{img.width}x#{img.height}px na página #{img.page_number}: #{img.dpi} DPI (mínimo #{DPI_MINIMO})"
    }
  }

  check "Espaço de cor" {
    // Impressão offset trabalha em CMYK; RGB precisa de conversão
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "imagem RGB na página #{img.page_number} — converter para CMYK"
    }
  }

  check "Imagens por página" {
    doc.pages.each { |page|
      // page.images traz só as imagens daquela página
      print("página", page.number, "tem", page.images.length, "imagem(ns)")
    }
  }
}
```

---

## 2.5 `region` — área da página

Regiões delimitam áreas retangulares para validar partes específicas da página:
rodapé, cabeçalho, área do código de barras, tarja de medicamento.

### Criando

```pdfl
// region(x, y, largura, altura [, "nome"])
// A origem (0,0) é o canto INFERIOR esquerdo, como no PDF.
cabecalho = region(0, 742, 595, 100, "cabeçalho")
rodape = region(0, 0, 595, 60, "rodapé")
tarja = region(20mm, 250mm, 60mm, 15mm, "tarja vermelha")
```

### Propriedades

| Propriedade | O que é |
|---|---|
| `region.name` | Nome dado na criação (vazio se omitido) |
| `region.x` / `region.y` | Canto inferior esquerdo |
| `region.width` / `region.height` | Dimensões |
| `region.right` / `region.top` | Bordas direita e superior (calculadas) |
| `region.area` | Área em pontos quadrados |

### Métodos

| Método | O que faz |
|---|---|
| `region.contains_point(x, y)` | O ponto está dentro? |
| `region.intersects(outra)` | As duas regiões se sobrepõem? |
| `region.expand(pt)` | Nova região maior em todos os lados |
| `region.inset(pt)` | Nova região menor em todos os lados |
| `region.export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Trabalhando com regiões" {
  rodape = region(0, 0, 595, 60, "rodapé")

  require rodape.name == "rodapé"
  require rodape.top == 60.0
  require rodape.right == 595.0
  require rodape.area == 35700.0

  // Um ponto no rodapé?
  require rodape.contains_point(300, 30)
  require !rodape.contains_point(300, 500)

  // Sobreposição: útil para detectar elementos invadindo áreas
  cabecalho = region(0, 780, 595, 62)
  require !rodape.intersects(cabecalho)

  // expand/inset devolvem NOVAS regiões (a original não muda)
  folga = rodape.expand(5mm)      // 5mm maior de cada lado
  seguro = rodape.inset(3mm)      // 3mm menor de cada lado
  require folga.area > rodape.area
  require seguro.area < rodape.area
}
```

### Usando regiões nas validações

```pdfl
profile "bula-farmaceutica" {

  check "Tarja de tarja vermelha" {
    // A tarja precisa estar no topo, com texto legal
    tarja = region(0, 700, 595, 142, "tarja")
    conteudo = text::extract_from_region(1, tarja)
    assert conteudo.contains("VENDA SOB PRESCRIÇÃO"),
      "tarja sem o texto obrigatório"
  }

  check "Tinta na área de dobra" {
    // Excesso de tinta na dobra causa problemas de acabamento
    dobra = region(290, 0, 15, 842, "dobra central")
    medida = prepress::calculate_tac_by_region(1, dobra)
    assert medida.first() < 240,
      "tinta demais na dobra: #{medida.first()}%"
  }

  check "Código de barras no lugar certo" {
    area_codigo = region(400, 20, 180, 80, "área do código")
    assert codes::validate_barcode_position(area_codigo),
      "código de barras fora da área reservada"
  }
}
```

---

[← A linguagem](01-linguagem.md) · [Índice](README.md) · [Próximo: `text::` →](03-text.md)
