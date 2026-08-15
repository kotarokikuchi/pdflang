# 12. Receitas

[← Comandos do CLI](11-cli.md) · [Índice](README.md) · [Próximo: Mudanças →](13-changelog.md)

Casos completos, prontos para adaptar. Cada um resolve um problema real de
produção.

---

## 12.1 Gráfica: preflight de revista em offset

**Problema:** o arquivo chega do cliente e alguém precisa conferir tinta,
fontes, imagens e sangria antes de mandar para a chapa. Um erro descoberto
depois custa a tiragem inteira.

`perfis/offset.pdfl`:

```pdfl
profile "offset-revista" {

  const TAC_LIMITE = 300%      // limite de tinta do couché
  const SANGRIA = 3mm          // exigência da imposição
  const DPI_MINIMO = 300

  check "Cobertura de tinta" tags: ["prepress"] {
    // O TAC exato lê as cores declaradas no arquivo — a estimativa
    // por renderização subestima preto rico e deixa passar excesso
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMITE,
        "página #{page.number}: #{tac}% de tinta (limite #{TAC_LIMITE}%)"
    }
  }

  check "Cores" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "documento em RGB — converter para CMYK"

    spots = prepress::detect_spot_colors()
    assert spots.length == 0,
      "tinta especial não contratada: #{spots.join(", ")}"

    assert !prepress::detect_rich_black(),
      "preto rico detectado — em textos use 0/0/0/100"
  }

  check "Fontes" tags: ["fontes"] {
    soltas = prepress::detect_text_substitution()
    assert soltas.length == 0,
      "fontes não embutidas (o texto vai mudar na RIP): #{soltas.join(", ")}"

    assert prepress::validate_font_size(6),
      "há texto abaixo de 6 pt — ilegível impresso"
  }

  check "Traços" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25),
      "traços abaixo de 0,25 pt somem na impressão"
    assert !prepress::detect_hairlines_exact(),
      "há traço com espessura 0 — defina uma espessura real"
  }

  check "Imagens" tags: ["imagens"] {
    doc.images.each { |img|
      assert img.dpi >= DPI_MINIMO,
        "imagem na página #{img.page_number}: #{round(img.dpi)} DPI (mínimo #{DPI_MINIMO})"
      assert img.color_space != "DeviceRGB",
        "imagem RGB na página #{img.page_number}"
    }
  }

  check "Geometria" tags: ["prepress"] {
    assert prepress::validate_trim_box(),
      "sem TrimBox — a imposição não sabe onde cortar"
    assert prepress::validate_bleed_box(),
      "sem BleedBox — não há sangria definida"
    assert prepress::check_page_geometry(SANGRIA),
      "sangria menor que 3 mm em alguma página"
  }
}
```

**Uso no balcão:**

```bash
# Laudo em HTML para devolver ao cliente
pdfl run perfis/offset.pdfl cliente.pdf --output html --output-file laudo.html
```

**Uso em watch folder:** o operador larga o arquivo na pasta e o laudo aparece
ao lado.

```bash
pdfl watch entrada/ --script perfis/offset.pdfl \
  --output-dir laudos/ --report html
```

---

## 12.2 Editora jurídica: contrato antes de publicar

**Problema:** contratos e apólices precisam ter cláusulas obrigatórias, não podem
ter texto de rascunho nem expor dados pessoais, e o texto precisa ser
pesquisável.

`perfis/juridico.pdfl`:

```pdfl
profile "contrato-padrao" {

  check "Cláusulas obrigatórias" tags: ["juridico"] {
    // Glossário mantido pelo departamento jurídico
    faltando = data::validate_against_reference("termos/clausulas.txt")
    assert faltando.length == 0,
      "cláusulas ausentes: #{faltando.join("; ")}"
  }

  check "Nada de rascunho" tags: ["juridico"] {
    assert text::forbid_text("RASCUNHO"), "documento marcado como rascunho"
    assert text::forbid_text("lorem ipsum"), "texto de preenchimento presente"
    assert text::forbid_match("X{3,}"), "campos não preenchidos (XXX)"
  }

  check "LGPD" tags: ["compliance"] {
    // CPF/CNPJ só entram na lista se o dígito verificador for válido,
    // então número de exemplo não gera alarme falso
    achados = text::detect_personal_data()
    assert achados.length == 0,
      "dados pessoais no documento: #{achados.join("; ")}"
  }

  check "Numeração e rubrica" tags: ["juridico"] {
    doc.pages.each { |page|
      rodape = region(0, 0, page.width, 60, "rodapé")
      conteudo = text::extract_from_region(page.number, rodape).trim()
      assert conteudo != "",
        "página #{page.number} sem numeração/rubrica no rodapé"
    }
  }

  check "Texto pesquisável" tags: ["acessibilidade"] {
    assert !text::detect_rasterized_text(),
      "há páginas escaneadas — o texto não pode ser pesquisado nem lido por leitor de tela"
    assert text::detect_language() == "pt",
      "documento não está em português"
  }
}
```

**Uso:**

```bash
pdfl run perfis/juridico.pdfl contrato.pdf --output pdf --output-file parecer.pdf
```

---

## 12.3 Laboratório: bula com código de lote

**Problema:** a bula precisa ter os textos exigidos pela ANVISA, e o código de
barras precisa corresponder ao produto certo — trocar o código entre produtos é
o erro mais caro do setor.

`perfis/bula.pdfl`:

```pdfl
profile "bula-anvisa" {

  check "Textos obrigatórios" tags: ["anvisa"] {
    faltando = data::validate_against_reference("bases/textos_anvisa.txt")
    assert faltando.length == 0,
      "textos obrigatórios ausentes: #{faltando.join("; ")}"
  }

  check "Legibilidade" tags: ["anvisa"] {
    // A ANVISA exige tamanho mínimo de corpo na bula
    assert prepress::validate_font_size(6),
      "há texto abaixo de 6 pt"
  }

  check "Código de barras" tags: ["codes", "critico"] {
    assert codes::detect_barcodes(), "bula sem código de barras"

    codigo = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1),
      "dígito verificador inválido: #{codigo}"

    // Este check pega o erro mais caro: código de um produto,
    // texto de outro
    assert codes::compare_barcode_with_text(),
      "o número do código não aparece no texto da bula"
  }

  check "Produto homologado" tags: ["dados", "critico"] {
    codigo = codes::decode_barcode(1)
    produto = data::query_gtin(codigo)
    assert produto,
      "GTIN #{codigo} não consta na base de produtos"

    // O nome cadastrado precisa aparecer impresso
    nome = produto.get(2)
    assert text::require_text(nome),
      "o nome '#{nome}' não aparece na bula"
    print("produto conferido:", nome)
  }

  check "Posição do código" tags: ["layout"] {
    area = region(400, 20, 180, 90, "área do código")
    assert codes::validate_barcode_position(area),
      "código fora da área reservada — risco de corte no acabamento"
  }
}
```

**Uso com as bases:**

```bash
PDFL_DATA_DIR=./bases pdfl run perfis/bula.pdfl bula_v3.pdf
```

---

## 12.4 Aprovação: comparar com a versão aprovada

**Problema:** o cliente aprovou a v1; chegou a v2 dizendo "só mudei uma palavra".
Confiar é caro.

```bash
# O que mudou de fato, em HTML para o cliente ver
pdfl compare aprovados/catalogo_v1.pdf recebidos/catalogo_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file diferencas.html

echo "exit: $?"   # 0 idênticos · 1 só metadados · 2 conteúdo mudou
```

Para conferir também a **aparência** (e não só o texto), um script:

`perfis/fidelidade.pdfl`:

```pdfl
profile "fidelidade-visual" {

  const APROVADO = "aprovados/catalogo_v1.pdf"

  check "Páginas visualmente idênticas" tags: ["aprovacao"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APROVADO)
      assert ssim > 0.99,
        "página #{page.number} mudou visualmente (SSIM #{ssim}, #{visual::pixel_diff(page.number, APROVADO)}% dos pixels)"
    }
  }

  check "Nenhuma imagem substituída" tags: ["aprovacao"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, APROVADO),
        "página #{page.number}: imagem trocada em relação à aprovada"
    }
  }
}
```

E uma pasta para olhar, para quem tem de assinar a reimpressão:

```bash
pdfl pixelcompare aprovado/catalogo_v1.pdf recebido/catalogo_v2.pdf \
  --max-diff 0.05 --viewer prova/ --output-file pixels.json

zip -r prova.zip prova/    # um index.html e três PNGs por página, mais nada
```

A pasta não precisa de servidor nem de rede: quem abrir o `index.html` vê o
original, o arquivo novo e os dois juntos com as diferenças pintadas por cima,
e ela abre nas páginas que diferem — num catálogo em que duas páginas mudaram
de noventa, são essas duas que interessam. Mover o mouse varre o arquivo novo
sobre o antigo; a roda dá zoom nos três painéis juntos e arrastar move os três,
então um fio de cabelo se resolve a 8× sem sair da página.

---

## 12.5 CI/CD: validando um lote inteiro

**Problema:** todo arquivo que entra no repositório precisa passar no preflight,
sem ninguém rodando nada à mão.

`.github/workflows/preflight.yml`:

```yaml
name: Preflight dos PDFs

on: [push, pull_request]

jobs:
  validar:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Instalar o pdfl
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # token automático do Actions, nada a configurar
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl_*_amd64.deb'
          sudo dpkg -i pdfl_*_amd64.deb

      - name: Conferir os próprios scripts
        run: |
          for f in perfis/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight de todos os PDFs
        run: |
          # --once processa o que está na pasta e sai com o pior código
          pdfl watch arquivos/ --script perfis/offset.pdfl \
            --output-dir laudos/ --once

      - name: Publicar os laudos
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: laudos
          path: laudos/

      # Um artefato é algo que alguém precisa ir lá abrir. Uma anotação no pull
      # request, não.
      - name: Achados no pull request
        run: |
          pdfl run perfis/offset.pdfl arquivos/capa.pdf \
            --output sarif --output-file pdfl.sarif
        continue-on-error: true          # exit 2 é arquivo recusado; o upload ainda tem que rodar
      - uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: pdfl.sarif
```

Em shell puro, com controle por arquivo:

```bash
#!/usr/bin/env bash
# valida_lote.sh — valida uma pasta e monta um resumo
set -uo pipefail

reprovados=0
for arquivo in entrada/*.pdf; do
  nome=$(basename "$arquivo" .pdf)
  if pdfl run perfis/offset.pdfl "$arquivo" \
       --output json --output-file "laudos/$nome.json"; then
    echo "OK      $nome"
  else
    echo "FALHOU  $nome"
    reprovados=$((reprovados + 1))
  fi
done

echo "---"
echo "$reprovados arquivo(s) reprovado(s)"
exit $((reprovados > 0))
```

---

## 12.6 Preparar arquivo da editora para a gráfica

**Problema:** o arquivo vem sem caixas de produção, com comentários de revisão e
camadas que podem ser reativadas por engano.

`perfis/preparar.pdfl`:

```pdfl
// Valida antes de mexer: se a pré-condição falhar, aparece no relatório
check "Pré-condições" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "arquivo criptografado — peça a versão aberta"
}

// Caixas de produção que a editora não definiu
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Limpeza
fix::remove_annotations()      // comentários de revisão
fix::remove_attachments()      // anexos que só pesam
fix::flatten_layers()          // camadas não podem ser reativadas
fix::remove_unused_resources() // sobras do arquivo
```

```bash
pdfl fix editora.pdf perfis/preparar.pdfl --output grafica.pdf --dry-run  # conferir
pdfl fix editora.pdf perfis/preparar.pdfl --output grafica.pdf            # aplicar
pdfl run perfis/offset.pdfl grafica.pdf                                   # validar
```

---

## 12.7 Distribuir perfis para a equipe

**Problema:** cinco máquinas precisam usar exatamente os mesmos perfis e bases,
com garantia de que ninguém alterou nada.

```bash
# Na máquina que mantém os perfis
pdfl pack perfis/ --name perfil-grafica --version 1.2.0
# gera perfil-grafica.pdflpkg (scripts + bases + manifesto com SHA-256)

# Nas máquinas de produção
pdfl add perfil-grafica.pdflpkg
# instala em ./pdfl_profiles/perfil-grafica@1.2.0/ conferindo cada hash

pdfl run pdfl_profiles/perfil-grafica@1.2.0/offset.pdfl arquivo.pdf
```

Se o pacote tiver sido alterado no caminho, o `add` **recusa a instalação**.

---

## 12.8 Investigando um arquivo problemático

Sequência prática quando algo está errado e não se sabe o quê:

```bash
# 1. Panorama em segundos
pdfl inspect suspeito.pdf

# 2. Um script exploratório, só com print()
cat > investigar.pdfl <<'EOF'
check "Raio-X" {
  print("TAC exato:", prepress::calculate_exact_tac(), "%")
  print("TAC estimado:", prepress::calculate_tac(), "%")
  print("spots:", prepress::detect_spot_colors().join(", "))
  print("preto rico?", prepress::detect_rich_black())
  print("overprint ok?", prepress::validate_overprint_settings())
  print("fontes soltas:", prepress::detect_text_substitution().join(", "))

  doc.images.each { |img|
    print("imagem pág", img.page_number, ":", img.width, "x", img.height,
          "@", round(img.dpi), "DPI", img.color_space)
  }
}
EOF

pdfl run investigar.pdfl suspeito.pdf > /dev/null
# o print() sai no stderr, então o relatório vai para /dev/null
# e você vê só a investigação
```

## 12.9 Testando um perfil antes que ele custe uma tiragem

**Problema:** um perfil é código, e alguém edita. Um limite muda, um check é
renomeado, e ninguém percebe até um arquivo que devia ter sido recusado ir para
a chapa.

Guarde os arquivos que te ensinaram a regra e congele o que o perfil diz sobre
eles:

```
perfis/grafica/
  offset.pdfl
  tests/
    aprovado.pdf              # passou, e tem que continuar passando
    aprovado.expected.json
    tinta_324.pdf             # o arquivo que custou uma reimpressão em março
    tinta_324.expected.json
    fontes_nao_embutidas.pdf
    fontes_nao_embutidas.expected.json
```

```bash
# Uma vez, quando os casos forem os que você quer
pdfl test perfis/grafica/offset.pdfl --update

# Daí em diante — na CI, e antes de todo commit no perfil
pdfl test perfis/grafica/offset.pdfl --jobs 0
```

Um arquivo recusado é um caso tão bom quanto um aprovado: o que fica gravado é o
relatório inteiro, então o teste falha com a mesma força se o perfil parar de
reclamar dos 324% de tinta ou se começar a reclamar de um arquivo que está bom.

```yaml
      - name: Os perfis ainda acham o que achavam
        run: pdfl test perfis/grafica/offset.pdfl --jobs 0
```

Leia a falha antes de regravar. O `--update` é o momento em que você decide que
o comportamento novo está certo — não existe outro momento.

---

---

[← Comandos do CLI](11-cli.md) · [Índice](README.md) · [Próximo: Mudanças →](13-changelog.md)
