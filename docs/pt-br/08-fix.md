# 8. Namespace `fix::` — normalização

[← `codes::`](07-codes.md) · [Índice](README.md) · [Próximo: `data::` →](09-data.md)

19 operações que **modificam** o PDF e salvam um arquivo novo. O original nunca
é alterado.

---

## 8.1 Como usar

`fix::` é o único namespace que escreve, por isso roda em comando próprio:

```bash
pdfl fix entrada.pdf script.pdfl --output corrigido.pdf
```

Opções:

| Opção | O que faz |
|---|---|
| `--output <arquivo>` | PDF de saída (obrigatório) |
| `--dry-run` | Lista as operações sem salvar nada |
| `--report json\|csv\|html\|pdf` | Formato do relatório |
| `--report-file <arquivo>` | Grava o relatório em arquivo |

Em `pdfl run`, qualquer chamada `fix::` gera erro orientando a usar o comando
certo — assim ninguém aplica correções achando que está só validando.

### Como as operações funcionam

```pdfl
// Este script NÃO precisa de checks: são comandos, executados na ordem.
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("RASCUNHO")
```

Cada chamada é **validada na hora** (página inexistente, rotação inválida,
arquivo ausente) e só depois aplicada. O relatório traz o campo `fixes` com o
que foi feito:

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"RASCUNHO\" added"
]
```

Nada impede misturar validação e correção no mesmo script:

```pdfl
// Valida antes de corrigir — se a pré-condição falhar, aparece no relatório
check "Pré-condições" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "arquivo criptografado, não dá para corrigir"
}

fix::add_page_numbers()
```

---

## 8.2 Caixas de página

### `fix::set_page_size(largura, altura)`

Define a MediaBox de todas as páginas.

```pdfl
// A4 em pontos — ou use unidades e deixe a conversão por conta da linguagem
fix::set_page_size(595, 842)
fix::set_page_size(210mm, 297mm)    // idêntico, e mais legível
```

### `fix::set_crop_box(x0, y0, x1, y1)`, `set_trim_box`, `set_bleed_box`

Definem a caixa correspondente em todas as páginas. Coordenadas em pontos, do
canto inferior esquerdo ao superior direito.

```pdfl
// Arquivo veio da editora sem as caixas de produção:
// TrimBox = área final; BleedBox = com 3 mm de sangria em volta
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 Páginas

### `fix::rotate_page([pagina,] graus)`

Rotaciona em 90, 180 ou 270 graus. Sem o número da página, rotaciona todas.

```pdfl
fix::rotate_page(90)        // todas as páginas
fix::rotate_page(3, 180)    // só a página 3
```

### `fix::delete_page(n)` e `fix::duplicate_page(n)`

```pdfl
fix::delete_page(1)         // remove a capa de rascunho
fix::duplicate_page(1)      // duplica a capa (a cópia entra logo depois)
```

Remover a única página do documento é recusado com mensagem clara.

### `fix::reorder_pages([nova, ordem])`

Nova ordem das páginas. A lista deve usar cada página exatamente uma vez.

```pdfl
// Documento de 4 páginas com a capa no fim: traz para o começo
fix::reorder_pages([4, 1, 2, 3])
```

### `fix::split_document(de, ate, "saida.pdf")`

Salva um intervalo de páginas em outro arquivo. O documento em edição continua
intacto.

```pdfl
// Separa o miolo da capa para envio a fornecedores diferentes
fix::split_document(1, 2, "capa.pdf")
fix::split_document(3, 50, "miolo.pdf")
```

### `fix::merge_documents("outro.pdf")`

Anexa as páginas de outro PDF ao final.

```pdfl
fix::merge_documents("anexos/termo_de_garantia.pdf")
fix::merge_documents("anexos/tabela_de_medidas.pdf")
```

---

## 8.4 Conteúdo

### `fix::add_watermark("texto")`

Marca d'água diagonal cinza, em todas as páginas.

```pdfl
fix::add_watermark("RASCUNHO — NÃO IMPRIMIR")
```

### `fix::add_stamps("texto")`

Selo em vermelho no canto superior direito de cada página.

```pdfl
fix::add_stamps("APROVADO 02/08/2026")
```

### `fix::add_page_numbers()`

Numeração `n / total` no rodapé de cada página.

```pdfl
fix::add_page_numbers()
```

### `fix::remove_annotations()` e `fix::remove_attachments()`

Removem anotações (comentários, marcações de revisão) e arquivos anexados.

```pdfl
// Antes de enviar para a gráfica: comentários de revisão não podem
// aparecer, e anexos só aumentam o arquivo
fix::remove_annotations()
fix::remove_attachments()
```

### `fix::flatten_layers()`

Remove a estrutura de camadas opcionais (OCG), deixando todo o conteúdo
permanentemente visível.

```pdfl
// Camadas com "versão inglês" desligada podem ser reativadas por engano
// na gráfica — achatar elimina o risco
fix::flatten_layers()
```

---

## 8.5 Otimização

> As operações desta seção **só gravam se o arquivo encolher**. Se a
> reescrita resultar em arquivo maior, o original é mantido.

### `fix::remove_unused_resources()`

Descarta objetos que não são alcançáveis a partir do trailer.

```pdfl
fix::remove_unused_resources()
```

### `fix::downsample_images([dpi])`

Reamostra imagens acima do DPI alvo (padrão 300). O DPI é calculado pelo
**tamanho impresso real** da imagem na página.

```pdfl
// Versão para aprovação por e-mail não precisa de 300 DPI
fix::downsample_images(96)

// Versão para impressão digital
fix::downsample_images(200)
```

> **Imagens CMYK são preservadas.** Reamostrá-las exigiria converter para RGB, o
> que destruiria as separações de pré-impressão. Em arquivos de gráfica, o ganho
> vem das imagens RGB.

### `fix::compress_images([qualidade])`

Recodifica as imagens em JPEG na qualidade informada (1 a 100, padrão 85).

```pdfl
fix::compress_images(70)
```

### Não disponíveis

`subset_fonts` e `linearize_document` **não** existem como operações de `fix::` e
geram erro de função desconhecida:

- **subset_fonts**: foi implementado e medido. Geradores profissionais já
  embutem apenas os glifos usados, então o ganho medido foi de 0,5% no melhor
  caso e nulo nos demais — não compensa o risco de corromper fontes. Para
  *verificar* se as fontes estão em subset, use
  [`prepress::subset_fonts()`](06-prepress.md#prepresssubset_fonts).
- **linearize_document**: exige gerar hint tables (§7.14 da especificação PDF).
  Nenhuma biblioteca Rust faz isso, e uma versão parcial não seria reconhecida
  como "Fast Web View" pelos leitores.

---

## 8.6 Exemplos completos

### Preparar arquivo da editora para a gráfica

```pdfl
// preparar_para_grafica.pdfl
// Uso: pdfl fix editora.pdf preparar_para_grafica.pdfl --output grafica.pdf

check "Pré-condições" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "arquivo criptografado — peça a versão aberta à editora"
}

// Caixas de produção que a editora não definiu
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Limpeza: comentários de revisão e anexos não vão para a impressão
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

### Versão leve para aprovação por e-mail

```pdfl
// versao_email.pdfl
// Uso: pdfl fix final.pdf versao_email.pdfl --output aprovacao.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROVA — NÃO É VERSÃO FINAL")
fix::add_page_numbers()
```

Conferindo o resultado com o próprio `pdfl`:

```bash
pdfl fix final.pdf versao_email.pdfl --output aprovacao.pdf
pdfl inspect aprovacao.pdf          # tamanho, DPI e avisos do arquivo gerado
```

### Separar um livro em capa e miolo

```pdfl
// separar.pdfl
// Uso: pdfl fix livro.pdf separar.pdfl --output livro_processado.pdf

check "Estrutura esperada" {
  assert doc.page_count > 4,
    "livro com apenas #{doc.page_count} páginas — estrutura inesperada"
}

fix::split_document(1, 2, "saida/capa.pdf")
fix::split_document(3, doc.page_count, "saida/miolo.pdf")
```

---

[← `codes::`](07-codes.md) · [Índice](README.md) · [Próximo: `data::` →](09-data.md)
