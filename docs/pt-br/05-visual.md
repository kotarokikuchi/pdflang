# 5. Namespace `visual::` — imagens e comparação visual

[← `struct::`](04-struct.md) · [Índice](README.md) · [Próximo: `prepress::` →](06-prepress.md)

16 funções sobre as imagens do documento e sobre a aparência renderizada das
páginas.

> As funções de comparação e qualidade **renderizam a página** em escala de
> cinza. Cada página é renderizada uma única vez e o resultado fica em cache.

---

## 5.1 Inventário de imagens

### `visual::detect_images()` e `visual::count_images()`

```pdfl
check "Imagens no documento" {
  require visual::detect_images()
  print("total de imagens:", visual::count_images())

  // Um catálogo sem imagem provavelmente está errado
  assert visual::count_images() >= 10,
    "catálogo com apenas #{visual::count_images()} imagem(ns)"
}
```

### `visual::get_image_resolution(n)`

DPI efetivo da n-ésima imagem (1-based).

```pdfl
check "Resolução da imagem de capa" {
  dpi = visual::get_image_resolution(1)
  assert dpi >= 300, "imagem de capa com #{round(dpi)} DPI (mínimo 300)"
}
```

### `visual::get_image_size(n)`

Dimensões em pixels: `[largura, altura]`.

```pdfl
check "Tamanho da imagem" {
  tamanho = visual::get_image_size(1)
  print("primeira imagem:", tamanho.first(), "x", tamanho.last(), "pixels")
  require tamanho.first() >= 1000
}
```

### `visual::detect_image_color_space([n])`

Sem argumento: lista dos espaços de cor presentes no documento.
Com `n`: o espaço da n-ésima imagem.

```pdfl
check "Espaços de cor usados" {
  espacos = visual::detect_image_color_space()
  print("espaços presentes:", espacos.join(", "))

  // Para offset, tudo deve ser CMYK
  assert !espacos.contains("DeviceRGB"),
    "há imagens RGB — converter para CMYK antes de imprimir"

  // Conferindo uma imagem específica
  require visual::detect_image_color_space(1) == "DeviceCMYK"
}
```

### `visual::detect_low_resolution([dpi_minimo])`

Verdadeiro se **alguma** imagem está abaixo do mínimo (padrão 300).

```pdfl
check "Resolução geral" {
  assert !visual::detect_low_resolution(300),
    "há imagens abaixo de 300 DPI"

  // Para impressão de grande formato o limite é outro
  assert !visual::detect_low_resolution(150),
    "há imagens abaixo de 150 DPI (mínimo para banner)"
}
```

> Para saber **quais** imagens estão ruins (e não apenas se existem), percorra
> `doc.images` — veja o [capítulo 2](02-tipos.md#24-image--a-imagem):
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300,
>     "imagem na página #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 Comparação visual entre arquivos

Estas funções comparam uma página deste documento com uma página de **outro
arquivo**. A assinatura geral é:

```
funcao(pagina_daqui, "outro.pdf" [, pagina_de_la])
```

Se a página do outro arquivo for omitida, usa o mesmo número. Páginas de
tamanhos diferentes são reamostradas antes da comparação.

### `visual::measure_ssim(pagina, "outro.pdf" [, pagina_b])`

Similaridade estrutural, de `0.0` a `1.0`. É a medida que mais se aproxima da
percepção humana de "são a mesma página".

```pdfl
check "Prova aprovada x arquivo final" {
  aprovado = "aprovados/revista_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, aprovado)
    assert ssim > 0.99,
      "página #{page.number} mudou visualmente (SSIM #{ssim})"
  }
}
```

### `visual::compare_images(...)` e `visual::diff_pages(...)`

A mesma comparação, em escala de 0 a 100. São sinônimos — use o nome que fizer
mais sentido no seu script.

```pdfl
check "Similaridade em porcentagem" {
  nota = visual::diff_pages(1, "versao_anterior.pdf")
  assert nota > 95, "capa mudou #{round(100 - nota)}% desde a última versão"
}
```

### `visual::pixel_diff(pagina, "outro.pdf" [, pagina_b, tolerancia])`

Porcentagem de pixels que diferem. A tolerância (padrão 10, de 0 a 255) ignora
variações mínimas de renderização.

```pdfl
check "Quanto da página mudou" {
  percentual = visual::pixel_diff(4, "anterior.pdf")
  print("pixels alterados na página 4:", percentual, "%")

  // Tolerância maior para ignorar antialiasing
  suave = visual::pixel_diff(4, "anterior.pdf", 4, 30)
  assert suave < 1.0, "mudança significativa na página 4"
}
```

### `visual::calculate_perceptual_hash([pagina])`

Impressão digital visual da página: 64 bits em hexadecimal. Páginas parecidas
têm hashes parecidos.

```pdfl
check "Impressão digital das páginas" {
  doc.pages.each { |page|
    print("página", page.number, "->", visual::calculate_perceptual_hash(page.number))
  }
}
```

### `visual::detect_image_replacement(pagina, "outro.pdf" [, pagina_b, distancia])`

Verdadeiro se a página mudou visualmente além do tolerado. Compara os hashes
perceptuais; `distancia` é quantos bits podem diferir (padrão 10 de 64).

```pdfl
check "Nenhuma imagem trocada entre versões" {
  anterior = "aprovados/catalogo_v1.pdf"

  doc.pages.each { |page|
    assert !visual::detect_image_replacement(page.number, anterior),
      "página #{page.number}: conteúdo visual substituído"
  }
}
```

---

## 5.3 Qualidade da imagem

### `visual::detect_image_artifacts([pagina])`

Verdadeiro se a página apresenta blocagem típica de JPEG muito comprimido.

```pdfl
check "Sem artefatos de compressão" {
  doc.pages.each { |page|
    assert !visual::detect_image_artifacts(page.number),
      "página #{page.number} com blocagem de compressão visível"
  }
}
```

### `visual::estimate_image_quality([pagina])`

Nota de 0 a 100 derivada da blocagem detectada.

```pdfl
check "Nota de qualidade" {
  doc.pages.each { |page|
    nota = visual::estimate_image_quality(page.number)
    assert nota >= 70,
      "página #{page.number} com qualidade #{nota}/100 — recomprimida demais?"
  }
}
```

### `visual::detect_posterization([pagina])`

Verdadeiro se há poucos níveis de tom numa página com faixa tonal ampla — sinal
de imagem salva com profundidade de cor insuficiente.

```pdfl
check "Degradês íntegros" {
  doc.pages.each { |page|
    assert !visual::detect_posterization(page.number),
      "página #{page.number}: possível posterização (poucos tons)"
  }
}
```

### `visual::detect_banding([pagina])`

Verdadeiro se há degradê em degraus visíveis em vez de transição suave.

> A detecção exige progressão monotônica com platôs largos — páginas de texto
> comum, que têm transições abruptas, **não** disparam alarme.

```pdfl
check "Degradês sem faixas" {
  // Fundos com degradê são onde o banding aparece
  doc.pages.each { |page|
    assert !visual::detect_banding(page.number),
      "página #{page.number} com banding no degradê"
  }
}
```

---

## 5.4 Exemplo completo

```pdfl
// aprovacao_visual.pdfl — compara o arquivo com a versão aprovada
// Uso: pdfl run aprovacao_visual.pdfl nova_versao.pdf
profile "aprovacao-visual" {

  const APROVADO = "aprovados/catalogo_v1.pdf"
  const DPI_MINIMO = 300

  check "Inventário" tags: ["imagens"] {
    require visual::detect_images()
    print("imagens:", visual::count_images())
    print("espaços de cor:", visual::detect_image_color_space().join(", "))
  }

  check "Resolução" tags: ["imagens", "prepress"] {
    doc.images.each { |img|
      assert img.dpi >= DPI_MINIMO,
        "imagem na página #{img.page_number}: #{round(img.dpi)} DPI (mínimo #{DPI_MINIMO})"
    }
  }

  check "Qualidade" tags: ["imagens"] {
    doc.pages.each { |page|
      assert !visual::detect_image_artifacts(page.number),
        "página #{page.number} com artefatos de compressão"
      assert !visual::detect_banding(page.number),
        "página #{page.number} com banding"
    }
  }

  check "Fidelidade à versão aprovada" tags: ["aprovacao"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APROVADO)
      assert ssim > 0.99,
        "página #{page.number} diferente da aprovada (SSIM #{ssim}, #{visual::pixel_diff(page.number, APROVADO)}% dos pixels)"
    }
  }
}
```

---

[← `struct::`](04-struct.md) · [Índice](README.md) · [Próximo: `prepress::` →](06-prepress.md)
