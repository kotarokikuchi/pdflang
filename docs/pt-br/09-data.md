# 9. Namespace `data::` — dados externos

[← `fix::`](08-fix.md) · [Índice](README.md) · [Próximo: Biblioteca padrão →](10-stdlib.md)

8 funções para cruzar o conteúdo do PDF com listas e tabelas suas. Tudo local:
nenhum dado sai da máquina.

---

## 9.1 Onde ficam os arquivos

Glossários e datasets aceitam **caminho relativo ao diretório de execução**:

```pdfl
data::load_glossary("termos/juridicos.txt")
data::load_dataset("dados/lotes.csv")
```

As bases de consulta (`query_gtin`, `query_medicamento`, `query_postal_code`) têm
nome fixo e são procuradas nesta ordem:

1. `$PDFL_DATA_DIR` (variável de ambiente)
2. `./dados/`
3. `./`
4. Perfis instalados por `pdfl add` (`pdfl_profiles/*/dados/`)
5. Ao lado do PDF analisado

```bash
# Apontando explicitamente para a pasta das bases
PDFL_DATA_DIR=/opt/bases pdfl run perfil.pdfl documento.pdf
```

Se a base não for encontrada, a mensagem de erro diz onde colocá-la.

Para distribuir bases junto com os perfis, use `pdfl pack` — veja o
[capítulo 11](11-cli.md#pdfl-pack).

---

## 9.2 Glossários

Um glossário é um arquivo de texto com um termo por linha. Linhas vazias e
começadas com `#` são ignoradas.

`termos/obrigatorios.txt`:

```
# Termos que toda apólice precisa conter
prazo de carência
cobertura contratada
condições gerais
```

### `data::load_glossary(arquivo)`

Carrega o glossário como lista de termos.

```pdfl
check "Glossário carregado" {
  termos = data::load_glossary("termos/obrigatorios.txt")
  print("termos no glossário:", termos.length)
  require termos.contains("condições gerais")
}
```

### `data::validate_against_reference(arquivo)`

O caminho mais direto: devolve a lista dos termos do glossário que **não**
aparecem no documento. Lista vazia significa que está tudo lá.

```pdfl
check "Cláusulas obrigatórias" {
  faltando = data::validate_against_reference("termos/obrigatorios.txt")
  assert faltando.length == 0,
    "cláusulas ausentes na apólice: #{faltando.join("; ")}"
}
```

A comparação ignora maiúsculas e espaçamento — "CONDIÇÕES  GERAIS" satisfaz
"condições gerais".

---

## 9.3 Datasets (CSV e JSON)

### `data::load_dataset(arquivo)`

Carrega um CSV ou um JSON como lista de linhas; cada linha é uma lista de
colunas. No CSV as aspas são tratadas conforme o padrão (campo entre aspas pode
conter vírgula); o JSON está descrito abaixo.

`dados/lotes.csv`:

```csv
lote,descricao,validade
L2026-08,Lote homologado agosto/2026,2028-08-01
L2026-09,Lote homologado setembro/2026,2028-09-01
```

```pdfl
check "Percorrendo a tabela" {
  linhas = data::load_dataset("dados/lotes.csv")

  // A primeira linha é o cabeçalho
  print("colunas:", linhas.first().join(" | "))
  print("registros:", linhas.length - 1)

  // get(n) é 1-based: get(1) é a primeira coluna
  linhas.each { |linha|
    print(linha.get(1), "->", linha.get(2))
  }
}
```

### Bases em JSON

Um arquivo terminado em `.json` é lido como JSON — tanto pelo `load_dataset`
quanto pelo `lookup_value`. Duas formas são aceitas, porque são as duas em que
uma base realmente é escrita.

Um array de arrays são as linhas como estão:

```json
[["lote", "descricao"],
 ["L2026-08", "Lote aprovado agosto/2026"]]
```

Um array de objetos vira uma linha de cabeçalho mais uma linha por objeto. As
colunas seguem a ordem em que o **primeiro** objeto as escreve, não a ordem
alfabética, então a primeira chave continua sendo a que o `lookup_value`
procura:

```json
[{"lote": "L2026-08", "descricao": "Lote aprovado agosto/2026"},
 {"lote": "L2026-09", "descricao": "Lote aprovado setembro/2026"}]
```

Uma chave ausente num objeto posterior deixa uma **célula vazia**, nunca uma
linha deslocada: buraco aparece no relatório, deslocamento não. Números mantêm
os dígitos com que foram escritos, e `null` é célula vazia — o mesmo que um
campo vazio de CSV significa.

Misturar as duas formas no mesmo arquivo é erro, e o erro diz em qual linha.

### `data::lookup_value(arquivo, chave)`

Procura a chave na primeira coluna e devolve o valor da **segunda**, tanto num
CSV quanto num JSON. Devolve `null` se não encontrar — e como `null` é falso, dá
para testar direto.

```pdfl
check "Lote homologado" {
  lote = text::extract_from_region(1, region(400, 50, 150, 20)).trim()

  descricao = data::lookup_value("dados/lotes.csv", lote)
  assert descricao,
    "lote #{lote} não consta na tabela de homologados"

  print("lote reconhecido:", descricao)
}
```

---

## 9.4 Bases de consulta

Estas funções procuram arquivos de nome fixo nas pastas descritas em 9.1 e
devolvem a **linha inteira** como lista (ou `null`).

### `data::query_gtin(codigo)`

Consulta `gtin.csv`. Ignora pontuação do código.

`dados/gtin.csv`:

```csv
gtin,descricao,fabricante
7891234567895,Dipirona Sódica 500mg 20cp,Lab Exemplo
```

```pdfl
check "Produto homologado" {
  // Cruzando com o código lido da própria embalagem
  codigo = codes::decode_barcode(1)
  produto = data::query_gtin(codigo)

  assert produto,
    "GTIN #{codigo} não consta na base de produtos"

  print("produto:", produto.get(2))
  print("fabricante:", produto.get(3))
}
```

### `data::query_medicamento(registro_ou_nome)`

Consulta `medicamentos.csv`. Aceita o número de registro (primeira coluna) ou
parte do nome (segunda coluna).

`dados/medicamentos.csv`:

```csv
registro,nome,principio_ativo,tarja
1.0298.0123,Dipirona Sódica,dipirona monoidratada,livre
1.0298.0456,Amoxicilina,amoxicilina tri-hidratada,vermelha
```

```pdfl
check "Tarja correta na bula" {
  registro = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicamento = data::query_medicamento(registro)

  assert medicamento,
    "registro #{registro} não encontrado na base ANVISA"

  // Se a tarja é vermelha, o texto obrigatório precisa estar na arte
  tarja = medicamento.get(4)
  print("medicamento:", medicamento.get(2), "| tarja:", tarja)

  assert tarja != "vermelha" || text::require_text("VENDA SOB PRESCRIÇÃO"),
    "medicamento de tarja vermelha sem o texto obrigatório"
}
```

### `data::query_postal_code(cep)`

Consulta `ceps.csv`. Aceita CEP com ou sem hífen; exige 8 dígitos.

`dados/ceps.csv`:

```csv
cep,logradouro,bairro,cidade,uf
01310100,Avenida Paulista,Bela Vista,Sao Paulo,SP
```

```pdfl
check "Endereço do fabricante" {
  endereco = data::query_postal_code("01310-100")
  assert endereco, "CEP não encontrado na base"

  print("logradouro:", endereco.get(2))
  print("cidade:", endereco.get(4), "-", endereco.get(5))
}
```

### `data::validate_address(cep, "trecho")`

Confere se o trecho informado aparece no endereço daquele CEP.

```pdfl
check "Endereço impresso confere com o CEP" {
  // O endereço na embalagem precisa bater com o CEP declarado
  assert data::validate_address("01310100", "Avenida Paulista"),
    "endereço impresso não corresponde ao CEP informado"
}
```

---

## 9.5 Exemplo completo

```pdfl
// bula_com_bases.pdfl — validação cruzando PDF com bases locais
// Uso: PDFL_DATA_DIR=./bases pdfl run bula_com_bases.pdfl bula.pdf
profile "bula-com-referencias"  {

  check "Termos obrigatórios ANVISA" tags: ["glossario"] {
    faltando = data::validate_against_reference("bases/termos_anvisa.txt")
    assert faltando.length == 0,
      "textos obrigatórios ausentes: #{faltando.join("; ")}"
  }

  check "Produto na base" tags: ["dados", "critico"] {
    codigo = codes::decode_barcode(1)
    produto = data::query_gtin(codigo)
    assert produto, "GTIN #{codigo} não homologado"

    // O nome na base tem que aparecer impresso na bula
    nome = produto.get(2)
    assert text::require_text(nome),
      "o nome '#{nome}' da base não aparece na bula"
    print("produto conferido:", nome)
  }

  check "Registro e tarja" tags: ["anvisa"] {
    registro = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(registro)
    assert med, "registro #{registro} não encontrado"

    assert med.get(4) != "vermelha" || text::require_text("VENDA SOB PRESCRIÇÃO"),
      "tarja vermelha exige o texto de prescrição"
  }

  check "Endereço do fabricante" tags: ["dados"] {
    assert data::validate_address("01310100", "Avenida Paulista"),
      "endereço do fabricante não confere com o CEP"
  }
}
```

---

[← `fix::`](08-fix.md) · [Índice](README.md) · [Próximo: Biblioteca padrão →](10-stdlib.md)
