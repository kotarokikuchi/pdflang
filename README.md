# PDFLang (.pdfl)

[![CI](https://github.com/kotarokikuchi/pdflang/actions/workflows/ci.yml/badge.svg)](https://github.com/kotarokikuchi/pdflang/actions/workflows/ci.yml)

Linguagem de script para validação de PDFs, interpretada pelo CLI `pdfl` (Rust).
Feita para ser legível por pessoas não técnicas — sem orientação a objetos, só
`check`s e asserções.

📖 **Documentação completa** — manual da linguagem, referência de todas as
funções e receitas prontas:
[Português (Brasil)](docs/pt-br/) · [English](docs/en/) · [日本語](docs/ja/) ·
[中文](docs/zh/) · [Français](docs/fr/) · [العربية](docs/ar/) · [Deutsch](docs/de/)

As mensagens do CLI, os diagnósticos e os relatórios são em **inglês**.

## Instalação (binário pronto)

Baixe o pacote da sua plataforma em
[Releases](https://github.com/kotarokikuchi/pdflang/releases) (Linux x64/arm64,
macOS arm64 — a libpdfium já vem dentro), extraia e rode:

```bash
tar -xzf pdfl-v*.tar.gz && cd pdfl-*/
./pdfl inspect documento.pdf
```

## Início rápido (compilando do código)

```bash
./setup_pdfium.sh          # baixa a biblioteca nativa pdfium (uma vez)
cargo build --release
./target/release/pdfl run examples/exemplo.pdfl documento.pdf --output json
```

Exit codes: `0` = OK, `1` = warnings, `2` = erros de validação, `3` = erro de sintaxe.

Formatos de saída (`--output` no `run`/`compare`, `--report` no `fix`/`watch`):
`json` (padrão), `csv` (uma linha por diagnóstico), `html` (autocontido)
e `pdf` (arquivo de auditoria A4). Os formatos de texto saem no stdout ou em
`--output-file`; o `pdf` vai sempre para arquivo (`--output-file` ou
`<entrada>.report.pdf`). `print()` e progresso saem no stderr.

## Exemplo de script

```pdfl
profile "validacao-basica" {
  const MIN_PAGINAS = 1

  check "Estrutura" tags: ["basico"] {
    require doc.page_count >= MIN_PAGINAS
    assert doc.title != "", "PDF has no title"
  }

  check "Fontes" {
    doc.fonts.each { |font|
      assert font.is_embedded, "Font #{font.name} is not embedded"
    }
  }
}
```

- `require expr` — falha com mensagem automática gerada da expressão
- `assert expr, "mensagem"` — falha com mensagem customizada (aceita interpolação `#{...}`)
- **Unidades**: `3mm`, `2.5cm`, `1in`, `10pt` viram pontos automaticamente
  (`const SANGRIA = 3mm`); `300%` mantém o valor numérico
- **Functions**: `function dobro(x) { x * 2 }` — o valor é o da última expressão;
  chame de qualquer check (`require dobro(21) == 42`)
- **Imports**: `import "biblioteca.pdfl"` — caminho relativo ao script; carrega
  functions, constantes e checks (cada arquivo é importado uma única vez)
- `doc` — o PDF carregado: `page_count`, `title`, `author`, `pages`, `fonts`, `extract_text()`
- `page` — `number`, `width`, `height` (pontos), `extract_text()`
- Listas: `each`, `all`, `any`, `filter`, `map`, `length`, `contains`, `join`
- Strings: `contains`, `starts_with`, `ends_with`, `trim`, `length`, `to_uppercase`, `to_lowercase`
- Globais: `min`, `max`, `abs`, `round`, `print`

## Namespace `text::`

Funções sobre o texto do documento (a maioria aceita uma string como argumento
opcional para operar sobre ela em vez do documento):

- Extração: `text::extract_all()`, `text::extract_from_page(n)`
- Normalização: `text::normalize()`, `text::split_words()`, `text::split_sentences()`,
  `text::split_paragraphs()`, `text::count_words()`, `text::count_characters()`,
  `text::detect_language()` (pt/en/es)
- Validação (retornam booleano, para usar com `require`/`assert`):
  `text::require_text("termo")`, `text::forbid_text("termo")`,
  `text::require_match("regex")`, `text::forbid_match("regex")`,
  `text::fuzzy_match(a, b)` (similaridade 0.0–1.0)
- Dados pessoais: `text::detect_personal_data()` / `text::detect_pii()` —
  lista ocorrências de CPF, CNPJ, e-mail e telefone

Exemplo completo em [examples/texto.pdfl](examples/texto.pdfl).

## Namespace `struct::`

Estrutura e metadados do arquivo PDF:

- Metadados: `struct::get_title()`, `struct::get_author()`, `struct::get_producer()`,
  `struct::get_creator()`, `struct::get_subject()`, `struct::get_keywords()`,
  `struct::get_creation_date()`, `struct::get_modification_date()` (datas no formato
  `AAAA-MM-DD HH:MM:SS`), `struct::list_metadata_entries()`
- Objetos e arquivo: `struct::count_objects()`, `struct::file_size()` (bytes),
  `struct::calculate_sha256()`, `struct::detect_file_bloat(kb_por_pagina)` (padrão 1024)

Exemplo completo em [examples/estrutura.pdfl](examples/estrutura.pdfl).

## Namespace `visual::`

Imagens do documento:

- `visual::detect_images()`, `visual::count_images()`
- `visual::get_image_resolution(n)` (DPI efetivo), `visual::get_image_size(n)` (`[largura, altura]` px)
- `visual::detect_image_color_space()` (lista dos espaços presentes) ou `(n)` (da n-ésima imagem)
- `visual::detect_low_resolution(dpi_minimo)` — `true` se alguma imagem está abaixo (padrão 300)

As imagens também são valores: `doc.images` / `page.images`, com `width`, `height`,
`dpi`, `dpi_x`, `dpi_y`, `color_space`, `page_number`, `bits_per_pixel`. O DPI é o
efetivo (pixels ÷ tamanho impresso na página), não o dos metadados.

Exemplo completo em [examples/imagens.pdfl](examples/imagens.pdfl).

## Namespace `prepress::`

Validações de pré-impressão:

- TAC/tinta: `prepress::calculate_tac([pagina])`, `prepress::calculate_ink_coverage([pagina])`,
  `prepress::validate_tac_limits(limite)` (padrão 300). **Atenção**: o TAC é estimado por
  renderização RGB e é um limite *inferior* do real — cores neutras (rich black) colapsam
  em K puro. Para o número confiável use `prepress::calculate_exact_tac([pagina])`,
  que lê as separações declaradas no content stream
- Linhas: `prepress::detect_hairlines(pt)` (padrão 0.25), `prepress::detect_fine_lines(pt)`
  (padrão 1.0), `prepress::validate_minimum_stroke_width(pt)`
- Cores: `prepress::detect_color_mode()` (RGB/CMYK/Mixed/None), `prepress::validate_color_space("DeviceCMYK")`
- Fontes: `prepress::list_fonts()`, `prepress::validate_font_embedding()`
- Páginas: `prepress::get_page_size(n)`, `prepress::get_page_boxes(n)`,
  `prepress::validate_media_box()`/`validate_trim_box()`/`validate_bleed_box()`,
  `prepress::check_page_geometry(mm)` (sangria mínima, padrão 3mm)

Nas páginas: `page.tac`, `page.ink_coverage`, `page.min_stroke_width`,
`page.has_trim_box`/`has_bleed_box`/`has_media_box`/`has_crop_box`/`has_art_box`.

Exemplo completo em [examples/prepress.pdfl](examples/prepress.pdfl).

## Namespace `codes::`

Códigos de barras e QR (decodificados com [rxing](https://crates.io/crates/rxing); o
escaneamento renderiza as páginas e roda só no primeiro uso de `codes::`):

- Detecção: `codes::detect_barcodes()`, `codes::detect_qrcodes()`, `codes::count_barcodes()`,
  `codes::get_barcode_type(n)` (EAN_13, QR_CODE, CODE_128...), `codes::get_barcode_location(n)`
  (`[página, x, y]` em pontos)
- Decodificação: `codes::decode_barcode(n)`, `codes::validate_barcode_checksum(n)` /
  `codes::validate_gtin(s)` / `codes::validate_ean(s)` (dígito verificador GTIN),
  `codes::validate_code128()`
- Comparação: `codes::compare_barcode_with_text()` (conteúdo do código aparece no texto),
  `codes::validate_barcode_format("regex")`, `codes::validate_barcode_position(x0, y0, x1, y1)`

Exemplo completo em [examples/codigos.pdfl](examples/codigos.pdfl).

## Namespace `data::`

Glossários e datasets locais (offline-first — caminhos relativos ao diretório
de execução; arquivos ficam em cache durante a execução):

- `data::load_glossary("termos.txt")` — lista de termos (um por linha, `#` comenta)
- `data::load_dataset("dados.csv")` — lista de linhas (cada linha é uma lista de
  colunas; CSV com aspas padrão)
- `data::lookup_value("dados.csv", chave)` — segunda coluna da linha cuja primeira
  coluna é a chave; `null` se não achar (funciona direto em `assert`)
- `data::validate_against_reference("termos.txt")` — termos do glossário que NÃO
  aparecem no texto do documento (lista vazia = tudo presente)

Listas ganham `get(n)` (1-based), `first()` e `last()` — úteis para as linhas de CSV.
Consultas a bases de referência: `query_gtin`, `query_medicamento`,
`query_postal_code` e `validate_address` — exigem os CSVs em `./dados/`,
`./pdfl_profiles/*/dados/`, `./` ou `$PDFL_DATA_DIR`.

Exemplo completo em [examples/dados.pdfl](examples/dados.pdfl) — inclui cruzamento
do código de barras do PDF com uma tabela local de lotes.

## Namespace `fix::` (comando `pdfl fix`)

Normalização — o único namespace que **escreve** um novo PDF, por isso roda em
comando próprio:

```bash
pdfl fix entrada.pdf script.pdfl --output corrigido.pdf [--dry-run]
```

- Caixas: `fix::set_page_size(w, h)`, `fix::set_crop_box(x0, y0, x1, y1)`,
  `fix::set_trim_box(...)`, `fix::set_bleed_box(...)` (pontos)
- Páginas: `fix::rotate_page([pagina,] graus)` (90/180/270; sem página = todas),
  `fix::delete_page(n)`, `fix::duplicate_page(n)`, `fix::reorder_pages([2, 1, 3])`
- Conteúdo: `fix::add_watermark("texto")`, `fix::add_page_numbers()`

As operações são validadas na hora da chamada (página inexistente, rotação
inválida, ordem incompleta → erro amigável) e aplicadas em sequência no final.
O relatório JSON ganha o campo `fixes` com o que foi aplicado. Em `pdfl run`,
chamadas `fix::` dão erro — normalização só no comando `fix`.

Exemplo completo em [examples/normalizar.pdfl](examples/normalizar.pdfl).

## Comando `pdfl compare`

Compara duas versões de um PDF (texto, estrutura e metadados):

```bash
pdfl compare v1.pdf v2.pdf [--output json|csv|html] [--normalize] \
  [--ignore-dates] [--similarity-threshold 95]
```

- Páginas são alinhadas por similaridade de conteúdo (inserções e remoções são
  detectadas mesmo quando a contagem muda)
- Cada página alinhada ganha um score de similaridade (Levenshtein por palavra)
  e uma amostra das linhas alteradas (`-removida | +adicionada`)
- Metadados alterados viram avisos; mudanças de texto acima do
  `--similarity-threshold` também (abaixo, erros)
- `--ignore-dates` troca datas (dd/mm/aaaa, aaaa-mm-dd, "1 de março de 2026")
  por um marcador antes de comparar; `--normalize` ignora caixa e espaçamento
- O relatório traz `similarity` geral (0–100); exit codes: 0 idênticos,
  1 só avisos, 2 diferenças acima do tolerado

## Comando `pdfl watch`

Monitora uma pasta e valida cada PDF novo ou alterado, gravando o relatório
ao lado do arquivo (ou em `--output-dir`):

```bash
pdfl watch entrada/ --script perfil.pdfl [--pattern "*.pdf"] [--exclude "*_rascunho*"] \
  [--output-dir relatorios/] [--depth 1] [--debounce 1000] [--report json|csv|html] \
  [--fail-fast] [--once]
```

- Polling com debounce: o arquivo só é processado quando para de ser escrito
- `--once` processa o que já está na pasta e sai com o pior exit code
  (0/1/2) — bom para lotes e CI; sem `--once` roda até Ctrl+C
- Relatórios saem como `<nome>.report.json|csv|html`; o log de progresso vai
  para o stderr

## Comandos `pdfl pack` e `pdfl add`

Perfis como código, distribuíveis (offline):

```bash
pdfl pack perfis/grafica --name perfil-grafica --version 1.0.0
# cria perfil-grafica.pdflpkg (scripts .pdfl + datasets, manifest com SHA-256)

pdfl add perfil-grafica.pdflpkg          # instala em ./pdfl_profiles/<nome>@<versão>/
pdfl run pdfl_profiles/perfil-grafica@1.0.0/prepress.pdfl arquivo.pdf
```

O pacote é um tar.gz determinístico; o `add` confere o hash de cada arquivo
contra o manifest (pacote adulterado é recusado). Repositório remoto e
assinatura digital ainda não estão implementados.

## Comandos `pdfl inspect` e `pdfl doc`

```bash
pdfl inspect documento.pdf              # resumo rápido: páginas, caixas, metadados,
                                        # fontes, imagens, TAC estimado e avisos gerais
pdfl doc script.pdfl                    # documentação do script em Markdown
pdfl doc script.pdfl --output html      # ou em HTML autocontido
```

O `doc` gera, a partir do próprio script: perfil, tabela de constantes e, para
cada check, as tags e o que ele valida (mensagens dos `assert` e condições dos
`require`). Scripts com `fix::` ganham o aviso de que rodam via `pdfl fix`.

## Comandos `pdfl lint` e `pdfl fmt`

Qualidade dos scripts `.pdfl`, sem executar:

```bash
pdfl lint script.pdfl           # avisos (exit 1 se houver)
pdfl fmt script.pdfl            # formata no lugar (2 espaços, espaçamento padrão)
pdfl fmt script.pdfl --check    # exit 1 se não está formatado (CI)
```

O lint aponta: variáveis/parâmetros de bloco declarados e nunca usados
(prefixo `_` silencia), checks duplicados ou vazios, namespace desconhecido,
`assert` fora de check e uso de `fix::` fora do comando `pdfl fix`.
O formatador preserva comentários e as quebras de linha do autor.

## Desenvolvimento

```bash
cargo test    # lexer, parser, interpretador e report (não precisa de pdfium)
```

## Licença

[MIT](LICENSE). A biblioteca nativa pdfium, baixada pelo `setup_pdfium.sh` e
embutida nos pacotes de release, vem com as licenças dela: PDFium é BSD de 3
cláusulas, a distribuição binária é MIT, e as dependências transitivas estão em
`pdfium/licenses/`.
