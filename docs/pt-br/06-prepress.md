# 6. Namespace `prepress::` — pré-impressão

[← `visual::`](05-visual.md) · [Índice](README.md) · [Próximo: `codes::` →](07-codes.md)

30 funções para validar o que a gráfica precisa conferir antes de imprimir:
cobertura de tinta, separações de cor, fontes, traços e caixas de página.

---

## 6.1 Cobertura de tinta (TAC)

TAC (*Total Area Coverage*) é a soma das quatro tintas num ponto. Passar do
limite da impressora causa borrão, secagem lenta e transferência entre folhas.
O limite típico do offset em papel couché é 300%.

Há **duas formas** de medir, e a diferença importa:

### `prepress::calculate_exact_tac([pagina])` — o número confiável

Lê as cores **declaradas no arquivo** (operadores de cor do PDF). É o valor
real.

```pdfl
check "Limite de tinta" {
  tac = prepress::calculate_exact_tac()
  assert tac <= 300,
    "cobertura de tinta de #{tac}% excede o limite de 300%"

  // Por página, para localizar o problema
  doc.pages.each { |page|
    valor = prepress::calculate_exact_tac(page.number)
    assert valor <= 300,
      "página #{page.number}: #{valor}% de tinta"
  }
}
```

### `prepress::calculate_tac([pagina])` — a estimativa

Calcula renderizando a página em RGB. É um **limite inferior**: cores neutras
escuras (preto rico) colapsam para perto de 100% na estimativa.

```pdfl
check "Comparando os dois métodos" {
  print("exato (declarado no arquivo):", prepress::calculate_exact_tac(), "%")
  print("estimado (por renderização):", prepress::calculate_tac(), "%")
  // Em arquivos reais testados: exato 324% vs estimado 299%
  // — só o exato revela que o arquivo passou do limite.
}
```

**Use sempre `calculate_exact_tac` para validar limite de tinta.** A estimativa
serve para uma leitura rápida quando as cores não estão declaradas.

### `prepress::validate_tac_limits([limite])`

Verdadeiro se todas as páginas estão dentro do limite (padrão 300). Usa a
estimativa por renderização.

```pdfl
check "Limite por perfil de papel" {
  // jornal aceita bem menos tinta que couché
  assert prepress::validate_tac_limits(240),
    "excede o limite de 240% do papel jornal"
}
```

### `prepress::calculate_ink_coverage([pagina])`

Cobertura **média** de tinta (%) — indica consumo, não risco de borrão.

```pdfl
check "Consumo de tinta" {
  media = prepress::calculate_ink_coverage()
  print("cobertura média:", media, "%")
  // Páginas muito cobertas encarecem a tiragem
  assert media < 200, "cobertura média alta: #{media}%"
}
```

### `prepress::calculate_tac_by_region(pagina, regiao)`

`[tac_maximo, cobertura_media]` dentro de uma área específica.

```pdfl
check "Tinta na área de dobra" {
  // Excesso de tinta na dobra racha na hora do acabamento
  dobra = region(290, 0, 15, 842, "dobra central")
  medida = prepress::calculate_tac_by_region(1, dobra)

  assert medida.first() < 240,
    "TAC de #{medida.first()}% na dobra (máximo 240%)"
  print("média na dobra:", medida.last(), "%")
}
```

---

## 6.2 Cores e separações

### `prepress::detect_spot_colors()`

Lista as tintas especiais (Pantone, vernizes, facas) declaradas como
`Separation` ou `DeviceN`.

> As separações reservadas `All` e `None` não entram na lista — `All` é marca de
> registro, não tinta.

```pdfl
check "Cores especiais" {
  spots = prepress::detect_spot_colors()
  print("tintas especiais:", spots.join(", "))

  // Trabalho contratado como 4 cores não pode ter tinta extra
  assert spots.length == 0,
    "o arquivo usa tinta especial não contratada: #{spots.join(", ")}"
}

check "Verniz previsto" {
  // Quando a tinta especial É esperada
  spots = prepress::detect_spot_colors()
  assert spots.contains("Verniz"),
    "faltou a camada de verniz localizado"
}
```

### `prepress::detect_color_mode()`

Devolve `"CMYK"`, `"RGB"`, `"Mixed"`, `"None"` ou `"Other"`, com base nas
imagens.

```pdfl
check "Modo de cor do documento" {
  modo = prepress::detect_color_mode()
  assert modo == "CMYK" || modo == "None",
    "documento em #{modo} — impressão offset exige CMYK"
}
```

### `prepress::validate_color_space(espaco)`

Verdadeiro se **todas** as imagens estão no espaço informado.

```pdfl
check "Tudo em CMYK" {
  assert prepress::validate_color_space("DeviceCMYK"),
    "há imagens fora do CMYK"
}
```

### `prepress::compare_colors_delta_e(cor_a, cor_b)`

Diferença perceptual entre duas cores (Delta-E CIE76). As cores são listas:
4 valores = CMYK, 3 = RGB, 1 = cinza.

Referência prática: ΔE abaixo de 1 é imperceptível; até 3, aceitável em
impressão; acima de 5, visivelmente diferente.

```pdfl
check "Cor da marca" {
  // O azul institucional aprovado
  marca = [1.0, 0.6, 0.0, 0.1]
  usada = [1.0, 0.62, 0.0, 0.12]

  diferenca = prepress::compare_colors_delta_e(marca, usada)
  assert diferenca < 3.0,
    "cor da marca fora do padrão (ΔE #{diferenca})"
}
```

### `prepress::detect_rich_black()`

Verdadeiro se há preto composto por várias tintas (K ≥ 60% com C+M+Y ≥ 20%).

```pdfl
check "Preto de texto correto" {
  // Texto pequeno em preto rico fica com registro tremido
  assert !prepress::detect_rich_black(),
    "há preto rico no arquivo — use preto chapado (0/0/0/100) em textos"
}
```

### `prepress::validate_overprint_settings()`

Verdadeiro se **nenhum** overprint está ligado.

```pdfl
check "Overprint" {
  // Overprint acidental faz elementos sumirem na impressão
  assert prepress::validate_overprint_settings(),
    "há overprint ligado — confira se é intencional"
}
```

### `prepress::validate_output_intent([nome])`

Sem argumento: verdadeiro se há Output Intent declarado. Com nome: verdadeiro
se o intent contém o texto informado.

```pdfl
check "Perfil de saída" {
  assert prepress::validate_output_intent(),
    "PDF sem Output Intent — a gráfica não sabe o perfil de cor de destino"

  // Exigindo um perfil específico
  assert prepress::validate_output_intent("Coated FOGRA39"),
    "Output Intent diferente do padrão da gráfica"
}
```

### `prepress::check_rendering_intent([esperado])`

Sem argumento: lista os intents declarados. Com argumento: verdadeiro se todos
são o esperado.

```pdfl
check "Rendering intent" {
  print("intents no arquivo:", prepress::check_rendering_intent().join(", "))

  assert prepress::check_rendering_intent("RelativeColorimetric"),
    "rendering intent diferente do padrão de produção"
}
```

---

## 6.3 Traços e linhas finas

Linhas muito finas somem na impressão ou saem irregulares.

### `prepress::detect_hairlines([limite])`

Verdadeiro se há traço abaixo do limite (padrão 0,25 pt).

```pdfl
check "Sem fios de cabelo" {
  assert !prepress::detect_hairlines(0.25),
    "há traços abaixo de 0,25 pt — vão sumir na impressão"
}
```

### `prepress::detect_hairlines_exact()`

Verdadeiro se há traço com **largura 0** — o hairline clássico do PostScript,
que a impressora renderiza no mínimo possível do equipamento (imprevisível).

```pdfl
check "Traço de largura zero" {
  assert !prepress::detect_hairlines_exact(),
    "há traço com espessura 0 — defina uma espessura real"
}
```

### `prepress::detect_fine_lines([limite])`

Como `detect_hairlines`, com limite maior (padrão 1 pt).

```pdfl
check "Linhas finas em fundo colorido" {
  // Sobre fundo, linhas abaixo de 1pt somem
  assert !prepress::detect_fine_lines(1.0),
    "há linhas abaixo de 1 pt"
}
```

### `prepress::validate_minimum_stroke_width(minimo)`

Verdadeiro se nenhum traço está abaixo do mínimo exigido.

```pdfl
check "Espessura mínima do contrato" {
  assert prepress::validate_minimum_stroke_width(0.5),
    "o contrato com a gráfica exige traços de no mínimo 0,5 pt"
}
```

---

## 6.4 Fontes

### `prepress::list_fonts()`

Nomes das fontes usadas.

```pdfl
check "Inventário de fontes" {
  fontes = prepress::list_fonts()
  print("fontes:", fontes.join(", "))
  assert fontes.length <= 8,
    "#{fontes.length} fontes diferentes — projeto gráfico inconsistente?"
}
```

### `prepress::validate_font_embedding()`

Verdadeiro se todas as fontes estão embutidas.

```pdfl
check "Fontes embutidas" {
  assert prepress::validate_font_embedding(),
    "há fontes não embutidas — o texto vai mudar na RIP"
}
```

### `prepress::detect_text_substitution()`

Lista as fontes **não embutidas**, que o leitor vai substituir por outras.

```pdfl
check "Quais fontes faltam" {
  faltando = prepress::detect_text_substitution()
  assert faltando.length == 0,
    "fontes não embutidas: #{faltando.join(", ")}"
}
```

### `prepress::detect_missing_glyphs()`

Lista fontes sem tabela de larguras — o leitor precisa adivinhar as métricas, o
que desalinha o texto.

```pdfl
check "Métricas completas" {
  problemas = prepress::detect_missing_glyphs()
  assert problemas.length == 0,
    "fontes sem tabela de larguras: #{problemas.join(", ")}"
}
```

### `prepress::subset_fonts()`

Verdadeiro se todas as fontes embutidas estão em subset (só os glifos usados) —
o que mantém o arquivo enxuto.

```pdfl
check "Fontes em subset" {
  assert prepress::subset_fonts(),
    "há fonte embutida inteira — o arquivo fica maior que o necessário"
}
```

### `prepress::check_font_licensing()`

Lista fontes de risco de licença: Type3 ou não embutidas.

```pdfl
check "Licenciamento" {
  risco = prepress::check_font_licensing()
  assert risco.length == 0,
    "fontes com risco de licença: #{risco.join(", ")}"
}
```

### `prepress::validate_font_size([minimo])`

Verdadeiro se nenhum texto está abaixo do tamanho mínimo (padrão 6 pt).

```pdfl
check "Legibilidade" {
  // ANVISA exige mínimo em bulas; contratos têm exigências parecidas
  assert prepress::validate_font_size(6),
    "há texto abaixo de 6 pt — ilegível após impressão"
}
```

---

## 6.5 Páginas e caixas

As caixas do PDF definem as áreas de trabalho: **MediaBox** (folha), **BleedBox**
(sangria), **TrimBox** (corte final), **CropBox** (visualização), **ArtBox**
(conteúdo).

### `prepress::get_page_size([pagina])`

`[largura, altura]` em pontos.

```pdfl
check "Formato" {
  tamanho = prepress::get_page_size(1)
  print("página 1:", tamanho.first(), "x", tamanho.last(), "pt")
  assert abs(tamanho.first() - 595.0) < 5, "largura fora do A4"
}
```

### `prepress::get_page_boxes([pagina])`

Lista das caixas definidas, formatadas como texto.

```pdfl
check "Caixas da primeira página" {
  prepress::get_page_boxes(1).each { |caixa| print(caixa) }
  // Exemplo de saída:
  //   MediaBox: [0, 0, 467.2, 665.6]
  //   TrimBox: [35.2, 35.2, 432, 630.4]
}
```

### `validate_media_box()`, `validate_trim_box()`, `validate_bleed_box()`

Verdadeiro se a caixa existe em **todas** as páginas.

```pdfl
check "Caixas obrigatórias para impressão" {
  require prepress::validate_media_box()
  assert prepress::validate_trim_box(),
    "sem TrimBox — a gráfica não sabe onde cortar"
  assert prepress::validate_bleed_box(),
    "sem BleedBox — não há área de sangria definida"
}
```

### `prepress::check_page_geometry([margem])`

Verdadeiro se a BleedBox excede a TrimBox pela margem informada em **todos os
lados**, em todas as páginas. Padrão: 3 mm.

```pdfl
check "Sangria suficiente" {
  // Use literais de unidade: fica legível e a conversão é automática
  assert prepress::check_page_geometry(3mm),
    "sangria menor que 3 mm em alguma página"

  // Gráficas de embalagem costumam exigir mais
  assert prepress::check_page_geometry(5mm),
    "esta gráfica exige 5 mm de sangria"
}
```

---

## 6.6 Exemplo completo

```pdfl
// offset_revista.pdfl — preflight completo para impressão offset
// Uso: pdfl run offset_revista.pdfl revista.pdf --output html --output-file laudo.html
profile "offset-revista" {

  const TAC_LIMITE = 300%
  const SANGRIA = 3mm
  const DPI_MINIMO = 300

  check "Cobertura de tinta" tags: ["prepress", "cores"] {
    // Sempre o exato para validar limite
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMITE,
        "página #{page.number}: #{tac}% de tinta (limite #{TAC_LIMITE}%)"
    }
    print("cobertura média:", prepress::calculate_ink_coverage(), "%")
  }

  check "Cores" tags: ["prepress", "cores"] {
    assert prepress::detect_color_mode() != "RGB", "documento em RGB"
    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "tinta especial não contratada: #{spots.join(", ")}"
    assert !prepress::detect_rich_black(), "preto rico em texto"
    assert prepress::validate_output_intent(), "sem Output Intent"
  }

  check "Fontes" tags: ["fontes"] {
    faltando = prepress::detect_text_substitution()
    assert faltando.length == 0, "fontes não embutidas: #{faltando.join(", ")}"
    assert prepress::validate_font_size(6), "texto abaixo de 6 pt"
    print("fontes:", prepress::list_fonts().join(", "))
  }

  check "Traços" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "traços abaixo de 0,25 pt"
    assert !prepress::detect_hairlines_exact(), "traço com espessura 0"
  }

  check "Geometria" tags: ["prepress", "caixas"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(SANGRIA),
      "sangria menor que 3 mm"
  }

  check "Imagens" tags: ["imagens"] {
    doc.images.each { |img|
      assert img.dpi >= DPI_MINIMO,
        "imagem na página #{img.page_number}: #{round(img.dpi)} DPI"
      assert img.color_space != "DeviceRGB",
        "imagem RGB na página #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [Índice](README.md) · [Próximo: `codes::` →](07-codes.md)
