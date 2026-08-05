# 3. Namespace `text::` — texto

[← Tipos](02-tipos.md) · [Índice](README.md) · [Próximo: `struct::` →](04-struct.md)

25 funções para extrair, normalizar, buscar e validar o texto do documento.

> Nas funções marcadas com `[texto]`, o argumento é **opcional**: sem ele, a
> função trabalha sobre o texto do documento inteiro; com ele, sobre a string
> que você passar.

---

## 3.1 Extração

### `text::extract_all()`

Todo o texto do documento (páginas unidas por quebra de linha).

```pdfl
check "Documento tem conteúdo" {
  texto = text::extract_all()
  assert texto.trim() != "", "PDF sem texto extraível"
  print("total de caracteres:", texto.length)
}
```

### `text::extract_from_page(pagina)`

Texto de uma página (1-based). Erro amigável se a página não existe.

```pdfl
check "Capa e contracapa" {
  capa = text::extract_from_page(1)
  assert capa.contains("Manual do Usuário"), "capa sem o título esperado"

  ultima = text::extract_from_page(doc.page_count)
  assert ultima.contains("ISBN"), "última página sem ISBN"
}
```

### `text::extract_from_region(pagina, regiao)`

Texto contido em uma área específica. Devolve string vazia se a região não tem
texto (não é erro).

```pdfl
check "Rodapé técnico não pode sobrar" {
  // Rodapés de produção (nome do arquivo .indd, data de exportação) às vezes
  // escapam para o arquivo final
  rodape = region(0, 0, 467, 40, "rodapé")

  doc.pages.each { |page|
    conteudo = text::extract_from_region(page.number, rodape)
    assert !conteudo.contains(".indd"),
      "página #{page.number} com marca de produção no rodapé: #{conteudo.trim()}"
  }
}
```

### `text::extract_with_normalization()`

O texto do documento já normalizado (minúsculas, espaços colapsados). Atalho
para `text::normalize(text::extract_all())`.

```pdfl
check "Busca sem se preocupar com maiúsculas" {
  texto = text::extract_with_normalization()
  require texto.contains("condições gerais")   // acha "CONDIÇÕES  GERAIS"
}
```

---

## 3.2 Normalização e divisão

### `text::normalize([texto])`

Minúsculas e espaços colapsados (múltiplos espaços viram um só).

```pdfl
check "Normalização" {
  require text::normalize("  OLÁ   Mundo  ") == "olá mundo"

  // Sem argumento, normaliza o documento inteiro
  print("documento normalizado tem", text::normalize().length, "caracteres")
}
```

### `text::split_words([texto])`

Divide em palavras, removendo pontuação das bordas.

```pdfl
check "Palavras" {
  palavras = text::split_words("Olá, mundo! (teste)")
  require palavras.length == 3
  require palavras.first() == "Olá"
  require palavras.contains("teste")
}
```

### `text::split_sentences([texto])`

Divide em sentenças (separadas por `.`, `!` ou `?` seguidos de espaço).

```pdfl
check "Sentenças longas demais" {
  // Bulas e contratos têm limite prático de legibilidade
  text::split_sentences().each { |frase|
    assert frase.length < 400,
      "sentença com #{frase.length} caracteres — difícil de ler"
  }
}
```

### `text::split_paragraphs([texto])`

Divide em parágrafos (separados por linha em branco).

```pdfl
check "Estrutura do documento" {
  paragrafos = text::split_paragraphs()
  print("parágrafos:", paragrafos.length)
  require paragrafos.length >= 3
}
```

### `text::count_words([texto])` e `text::count_characters([texto])`

```pdfl
check "Volume de texto" {
  require text::count_words() > 100
  require text::count_characters() > 500

  // Também funcionam sobre uma string qualquer
  resumo = text::extract_from_page(1)
  assert text::count_words(resumo) <= 250,
    "resumo com #{text::count_words(resumo)} palavras (máximo 250)"
}
```

### `text::detect_language([texto])`

Devolve `"pt"`, `"en"`, `"es"` ou `"unknown"` (heurística por palavras comuns).

```pdfl
check "Idioma do documento" {
  idioma = text::detect_language()
  assert idioma == "pt",
    "documento deveria estar em português, detectei: #{idioma}"
}
```

---

## 3.3 Busca e conteúdo obrigatório

### `text::require_text(termo)` e `text::forbid_text(termo)`

Devolvem verdadeiro/falso. A comparação ignora maiúsculas e espaçamento.

```pdfl
profile "contrato" {
  check "Cláusulas obrigatórias" {
    assert text::require_text("foro da comarca"),
      "contrato sem cláusula de foro"
    assert text::require_text("prazo de vigência"),
      "contrato sem prazo de vigência"
  }

  check "Termos proibidos" {
    assert text::forbid_text("RASCUNHO"),
      "documento ainda marcado como rascunho"
    assert text::forbid_text("lorem ipsum"),
      "texto de preenchimento não substituído"
  }
}
```

### `text::require_match(regex)` e `text::forbid_match(regex)`

Como os anteriores, mas com expressão regular.

```pdfl
check "Padrões no documento" {
  // Precisa ter um número de contrato no formato 2026/0001
  assert text::require_match("\d{4}/\d{4}"),
    "número de contrato não encontrado"

  // Não pode ter data no formato americano
  assert text::forbid_match("\d{2}-\d{2}-\d{4}"),
    "data em formato americano encontrada"
}
```

### `text::fuzzy_match(a, b)`

Similaridade entre dois textos, de `0.0` (nada a ver) a `1.0` (idênticos).
Útil quando erros de digitação ou OCR são esperados.

```pdfl
check "Nome do produto com tolerância" {
  esperado = "Paracetamol 750mg"
  encontrado = text::extract_from_region(1, region(50, 700, 300, 40))

  similaridade = text::fuzzy_match(esperado, encontrado)
  assert similaridade > 0.9,
    "nome do produto diferente do esperado (#{round(similaridade * 100)}% de similaridade)"
}
```

---

## 3.4 Dados pessoais (LGPD)

### `text::detect_personal_data([texto])` e `text::detect_pii([texto])`

São sinônimos. Devolvem a **lista** de dados pessoais encontrados: CPF, CNPJ,
e-mail e telefone.

> CPF e CNPJ só entram na lista se o **dígito verificador for válido**. Um
> número que apenas se parece com CPF (ex.: `111.111.111-12`) não gera alarme.

```pdfl
check "Documento público não pode ter dados pessoais" {
  achados = text::detect_personal_data()
  assert achados.length == 0,
    "dados pessoais expostos: #{achados.join("; ")}"
}

check "Relatório do que foi encontrado" {
  // Cada item vem no formato "CPF: 529.982.247-25"
  text::detect_pii().each { |item|
    print("encontrado:", item)
  }
}
```

---

## 3.5 Validações brasileiras

### `text::validate_cpf(texto)` e `text::validate_cnpj(texto)`

Validam o dígito verificador (mod 11). Aceitam com ou sem pontuação e rejeitam
sequências repetidas (`111.111.111-11`).

```pdfl
check "CPF do titular" {
  cpf = text::extract_from_region(1, region(100, 600, 200, 20)).trim()
  assert text::validate_cpf(cpf),
    "CPF inválido no cadastro: #{cpf}"
}

check "CNPJ da empresa" {
  require text::validate_cnpj("11.222.333/0001-81")
  require !text::validate_cnpj("11.222.333/0001-82")   // dígito errado
}
```

### `text::validate_date_format(texto [, formato])`

Verifica se é uma data **válida no calendário** (considera bissexto e dias por
mês). Formatos aceitos: `"dd/mm/aaaa"` e `"aaaa-mm-dd"`; sem o segundo
argumento, aceita os dois.

```pdfl
check "Datas do documento" {
  require text::validate_date_format("29/02/2024")     // 2024 é bissexto
  require !text::validate_date_format("29/02/2023")    // 2023 não é
  require !text::validate_date_format("31/04/2026")    // abril tem 30 dias

  // Exigindo um formato específico
  require text::validate_date_format("02/08/2026", "dd/mm/aaaa")
  require !text::validate_date_format("2026-08-02", "dd/mm/aaaa")
}
```

### `text::validate_phone_format(texto)`

Telefone brasileiro: `(DD) 9XXXX-XXXX` ou `(DD) XXXX-XXXX`, com pontuação
opcional.

```pdfl
check "Telefone de contato" {
  require text::validate_phone_format("(11) 98765-4321")
  require text::validate_phone_format("1198765432")
  require !text::validate_phone_format("12345")
}
```

### `text::validate_format(texto, regex)`

Verdadeiro se a string **inteira** casa com a expressão regular.

```pdfl
check "Código de lote no padrão da fábrica" {
  lote = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(lote, "L\d{4}-\d{2}"),
    "código de lote fora do padrão L0000-00: #{lote}"
}
```

---

## 3.6 Comparação e diagnóstico

### `text::diff(a, b)`

Lista as linhas que mudaram entre dois textos: `-` para as que saíram, `+` para
as que entraram.

```pdfl
check "Comparando duas páginas" {
  antes = text::extract_from_page(1)
  depois = text::extract_from_page(2)

  mudancas = text::diff(antes, depois)
  print("linhas alteradas:", mudancas.length)
  mudancas.each { |linha| print(linha) }
}
```

> Para comparar dois **arquivos**, use o comando `pdfl compare` — ele alinha as
> páginas automaticamente. Veja o [capítulo 11](11-cli.md).

### `text::detect_rasterized_text()`

Verdadeiro se alguma página não tem texto extraível mas tem imagem cobrindo
metade ou mais da área — sinal de texto convertido em imagem.

```pdfl
check "Texto precisa ser texto" {
  // Página escaneada ou com texto vetorizado não permite busca,
  // acessibilidade nem correção ortográfica
  assert !text::detect_rasterized_text(),
    "há páginas com texto rasterizado (escaneado ou convertido em imagem)"
}
```

---

## 3.7 Exemplo completo

```pdfl
// documento_juridico.pdfl — validação de contrato
profile "contrato-padrao" {

  check "Conteúdo obrigatório" tags: ["juridico"] {
    assert text::require_text("foro da comarca"), "sem cláusula de foro"
    assert text::require_text("prazo de vigência"), "sem prazo de vigência"
    assert text::require_match("\d{4}/\d{4}"), "sem número de contrato"
  }

  check "Nada de rascunho" tags: ["juridico"] {
    assert text::forbid_text("RASCUNHO"), "marcado como rascunho"
    assert text::forbid_text("lorem ipsum"), "texto de preenchimento presente"
    assert text::forbid_match("XXX+"), "campos não preenchidos (XXX)"
  }

  check "LGPD" tags: ["compliance"] {
    achados = text::detect_personal_data()
    assert achados.length == 0,
      "dados pessoais no documento público: #{achados.join("; ")}"
  }

  check "Qualidade do texto" tags: ["texto"] {
    assert text::detect_language() == "pt", "documento não está em português"
    assert !text::detect_rasterized_text(), "texto rasterizado impede busca"
    require text::count_words() > 200
  }
}
```

---

[← Tipos](02-tipos.md) · [Índice](README.md) · [Próximo: `struct::` →](04-struct.md)
