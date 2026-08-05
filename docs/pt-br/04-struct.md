# 4. Namespace `struct::` — estrutura e metadados

[← `text::`](03-text.md) · [Índice](README.md) · [Próximo: `visual::` →](05-visual.md)

23 funções sobre o arquivo em si: metadados, objetos internos, segurança e
rastreabilidade.

> As funções a partir de `list_objects` leem a estrutura interna do arquivo.
> Essa análise roda **uma única vez**, no primeiro uso, e fica em cache.

---

## 4.1 Metadados

### Leitura direta

| Função | Devolve |
|---|---|
| `struct::get_title()` | Título |
| `struct::get_author()` | Autor |
| `struct::get_subject()` | Assunto |
| `struct::get_keywords()` | Palavras-chave |
| `struct::get_creator()` | Programa que criou o documento original |
| `struct::get_producer()` | Programa que gerou o PDF |

Todas devolvem string vazia quando o campo não existe.

```pdfl
check "Metadados obrigatórios" {
  assert struct::get_title() != "", "PDF sem título"
  assert struct::get_author() != "", "PDF sem autor"

  // Producer revela a ferramenta de origem — útil para rastrear problemas
  print("gerado por:", struct::get_producer())
  print("criado em:", struct::get_creator())
}

check "Origem confiável" {
  // Alguns fluxos só aceitam PDFs de ferramentas homologadas
  produtor = struct::get_producer()
  assert produtor.contains("Adobe") || produtor.contains("Ghostscript"),
    "PDF gerado por ferramenta não homologada: #{produtor}"
}
```

### `struct::get_creation_date()` e `struct::get_modification_date()`

Datas já convertidas do formato interno do PDF (`D:20260802173622-03'00'`) para
`AAAA-MM-DD HH:MM:SS`.

```pdfl
check "Datas do arquivo" {
  criacao = struct::get_creation_date()
  assert criacao != "", "PDF sem data de criação"
  print("criado em:", criacao)
  print("modificado em:", struct::get_modification_date())

  // Comparação de texto funciona porque o formato é ordenável
  assert criacao > "2026-01-01", "arquivo antigo demais para esta campanha"
}
```

### `struct::list_metadata_entries()`

Lista com todas as entradas preenchidas, no formato `"Chave: valor"`.

```pdfl
check "Inventário de metadados" {
  entradas = struct::list_metadata_entries()
  print("metadados:", entradas.join(" | "))
  require entradas.length >= 2
}
```

### `struct::extract_xmp()`

Metadados XMP (XML) do catálogo. String vazia se não houver.

```pdfl
check "XMP presente" {
  xmp = struct::extract_xmp()
  assert xmp != "", "PDF sem metadados XMP"

  // XMP é XML — dá para procurar campos específicos
  assert xmp.contains("pdfaid"), "sem identificação PDF/A no XMP"
  print("XMP tem", xmp.length, "caracteres")
}
```

---

## 4.2 Arquivo e rastreabilidade

### `struct::file_size()`

Tamanho em bytes.

```pdfl
check "Tamanho para envio por e-mail" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "arquivo com #{round(mb)} MB (limite de 10 MB para e-mail)"
}
```

### `struct::calculate_sha256()`

Hash SHA-256 do arquivo — a impressão digital para trilha de auditoria.

```pdfl
check "Registro de auditoria" {
  // O hash entra no relatório e prova qual arquivo exato foi aprovado
  hash = struct::calculate_sha256()
  print("SHA-256:", hash)
  require hash.length == 64
}
```

### `struct::detect_file_bloat([kb_por_pagina])`

Verdadeiro se o arquivo está "inchado" — acima do limite de KB por página
(padrão 1024).

```pdfl
check "Arquivo enxuto" {
  assert !struct::detect_file_bloat(1024),
    "arquivo pesado: #{struct::file_size() / 1024} KB para #{doc.page_count} páginas"

  // Limite mais rígido para publicação web
  assert !struct::detect_file_bloat(200),
    "pesado demais para publicação web"
}
```

---

## 4.3 Objetos internos

### `struct::count_objects()`

Quantidade de objetos de conteúdo (texto, imagem, traço) nas páginas.

```pdfl
check "Documento não está vazio" {
  require struct::count_objects() > 0
  print("objetos de conteúdo:", struct::count_objects())
}
```

### `struct::list_objects()`

Lista todos os objetos do arquivo no formato `"número: tipo"`.

```pdfl
check "Inventário do arquivo" {
  objetos = struct::list_objects()
  print("total de objetos:", objetos.length)

  // Quantos são fontes?
  fontes = objetos.filter { |o| o.contains("Font") }
  print("objetos de fonte:", fontes.length)
}
```

### `struct::detect_unreferenced_objects()`

Objetos que não são alcançáveis a partir do trailer — peso morto no arquivo.

> Objetos de infraestrutura (`ObjStm`, `XRef`) não entram na lista: eles nunca
> são referenciados pelo trailer por definição, e reportá-los seria alarme falso.

```pdfl
check "Sem lixo no arquivo" {
  soltos = struct::detect_unreferenced_objects()
  assert soltos.length == 0,
    "#{soltos.length} objeto(s) não referenciado(s): #{soltos.join(", ")}"
}
```

### `struct::detect_orphaned_resources()`

Como o anterior, mas só recursos (fontes, imagens, XObjects) — o tipo de sobra
que mais pesa no arquivo.

```pdfl
check "Recursos órfãos" {
  orfaos = struct::detect_orphaned_resources()
  assert orfaos.length == 0,
    "recursos embutidos sem uso: #{orfaos.join(", ")} — rode 'pdfl fix' com remove_unused_resources()"
}
```

### `struct::measure_object_size(numero)`

Tamanho aproximado de um objeto específico, em bytes.

```pdfl
check "Objeto mais pesado" {
  // Combinando com list_objects para investigar o que ocupa espaço
  struct::list_objects().each { |entrada| print(entrada) }
  print("tamanho do objeto 5:", struct::measure_object_size(5), "bytes")
}
```

---

## 4.4 Segurança

### `struct::detect_javascript()`

Verdadeiro se há JavaScript embutido no PDF.

```pdfl
check "Sem código executável" {
  // JavaScript em PDF é vetor de ataque comum e desnecessário
  // em documentos de produção gráfica
  assert !struct::detect_javascript(),
    "PDF contém JavaScript embutido"
}
```

### `struct::detect_suspicious_actions()`

Lista ações de risco encontradas: `JavaScript`, `Launch` (executa programa),
`URI`, `SubmitForm`, `ImportData`, `GoToR` — cada uma com o objeto de origem.

```pdfl
check "Ações do documento" {
  acoes = struct::detect_suspicious_actions()
  assert acoes.length == 0,
    "ações suspeitas no PDF: #{acoes.join("; ")}"
}

check "Só links são aceitáveis" {
  // Se links externos são permitidos, filtre só o que preocupa
  perigosas = struct::detect_suspicious_actions().filter { |a|
    a.contains("Launch") || a.contains("JavaScript")
  }
  assert perigosas.length == 0, "ações perigosas: #{perigosas.join("; ")}"
}
```

### `struct::check_encryption()`

Verdadeiro se o documento está criptografado.

```pdfl
check "Arquivo aberto para produção" {
  // PDF criptografado pode falhar na RIP da gráfica
  assert !struct::check_encryption(),
    "PDF criptografado — remova a proteção antes de enviar para impressão"
}
```

### `struct::validate_permissions()`

Verdadeiro se **não** há restrições de permissão (documento livre para
processar).

```pdfl
check "Permissões" {
  assert struct::validate_permissions(),
    "PDF com restrições de permissão que podem impedir o processamento"
}
```

### `struct::validate_signatures()`

Verdadeiro se há campos de assinatura digital no documento.

> Esta função detecta a **presença** dos campos. A validação criptográfica da
> cadeia de certificados não é feita nesta versão.

```pdfl
check "Documento assinado" {
  assert struct::validate_signatures(),
    "documento sem campo de assinatura digital"
}
```

---

## 4.5 Exemplo completo

```pdfl
// auditoria.pdfl — verificação de conformidade e segurança
profile "auditoria-de-arquivo" {

  check "Identificação" tags: ["metadados"] {
    assert struct::get_title() != "", "sem título"
    assert struct::get_author() != "", "sem autor"
    assert struct::get_creation_date() != "", "sem data de criação"
    print("documento:", struct::get_title())
    print("gerado por:", struct::get_producer())
  }

  check "Rastreabilidade" tags: ["auditoria"] {
    // O hash prova exatamente qual arquivo foi validado
    print("SHA-256:", struct::calculate_sha256())
    print("tamanho:", struct::file_size() / 1024, "KB")
  }

  check "Segurança" tags: ["seguranca"] {
    assert !struct::detect_javascript(), "JavaScript embutido"
    assert !struct::check_encryption(), "arquivo criptografado"
    acoes = struct::detect_suspicious_actions()
    assert acoes.length == 0, "ações suspeitas: #{acoes.join("; ")}"
  }

  check "Higiene do arquivo" tags: ["otimizacao"] {
    orfaos = struct::detect_orphaned_resources()
    assert orfaos.length == 0, "recursos sem uso: #{orfaos.join(", ")}"
    assert !struct::detect_file_bloat(1024), "arquivo inchado"
  }
}
```

---

[← `text::`](03-text.md) · [Índice](README.md) · [Próximo: `visual::` →](05-visual.md)
