# 11. Comandos do CLI

[← Biblioteca padrão](10-stdlib.md) · [Índice](README.md) · [Próximo: Receitas →](12-receitas.md)

Dez comandos: quatro que trabalham com PDFs, quatro sobre os scripts e dois de
distribuição.

| Comando | O que faz |
|---|---|
| [`run`](#pdfl-run) | Valida um PDF com um script |
| [`compare`](#pdfl-compare) | Compara duas versões de um PDF |
| [`watch`](#pdfl-watch) | Monitora uma pasta e valida o que chega |
| [`fix`](#pdfl-fix) | Aplica correções e salva um PDF novo |
| [`inspect`](#pdfl-inspect) | Resumo rápido de um PDF |
| [`lint`](#pdfl-lint) | Analisa um script sem executar |
| [`fmt`](#pdfl-fmt) | Formata um script |
| [`doc`](#pdfl-doc) | Gera documentação de um script |
| [`pack`](#pdfl-pack) | Empacota perfis e bases |
| [`add`](#pdfl-add) | Instala um pacote |

---

## Códigos de saída

Todos os comandos que validam usam a mesma convenção:

| Código | Significado |
|---|---|
| `0` | Tudo passou |
| `1` | Apenas avisos |
| `2` | Erros de validação |
| `3` | Erro de sintaxe no script |
| `10` | O documento não pôde ser lido, ou um arquivo não pôde ser escrito — nenhum veredito foi dado |

Em scripts de shell:

```bash
pdfl run perfil.pdfl arquivo.pdf > relatorio.json
case $? in
  0) echo "aprovado" ;;
  1) echo "aprovado com ressalvas" ;;
  2) echo "reprovado — veja relatorio.json" ;;
  3) echo "erro no script de validação" ;;
esac
```

---

## `pdfl run`

Valida um PDF com um script.

```bash
pdfl run <script.pdfl> <entrada.pdf> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Formato do relatório |
| `--output-file <arquivo>` | — | Grava em arquivo em vez do stdout |
| `--fail-on error\|warning` | `error` | Com `warning`, avisos também dão exit 2 |
| `--verbose` | — | Informação extra no stderr |
| `--var NOME=VALOR` | — | Valor que o script lê como `vars.NOME`; repetível |
| `--tags TAG` | — | Roda só os checks com essa tag; repetível. Tag que nenhum check tem é erro, não aprovação vazia |

```bash
# Relatório JSON no terminal
pdfl run prepress.pdfl revista.pdf

# HTML para enviar ao cliente
pdfl run prepress.pdfl revista.pdf --output html --output-file laudo.html

# PDF de auditoria (o formato pdf sempre grava em arquivo)
pdfl run prepress.pdfl revista.pdf --output pdf --output-file laudo.pdf

# CSV para planilha
pdfl run prepress.pdfl revista.pdf --output csv --output-file achados.csv

# Rigoroso: avisos também reprovam
pdfl run prepress.pdfl revista.pdf --fail-on warning
```

### O relatório JSON

```json
{
  "script_name": "prepress.pdfl",
  "input_file": "revista.pdf",
  "profile": "offset-revista",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Cobertura de tinta",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ]
}
```

O mesmo PDF com o mesmo script sempre gera o **mesmo relatório, byte a byte** —
dá para versionar e comparar em CI.

---

## `pdfl compare`

Compara duas versões de um PDF: texto, estrutura e metadados.

```bash
pdfl compare <v1.pdf> <v2.pdf> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Formato |
| `--output-file <arquivo>` | — | Grava em arquivo |
| `--normalize` | — | Ignora maiúsculas e espaçamento |
| `--ignore-dates` | — | Mascara datas antes de comparar |
| `--similarity-threshold <0-100>` | `100` | Similaridade mínima aceitável |

```bash
# Comparação simples
pdfl compare aprovado_v1.pdf novo_v2.pdf

# Tolerando pequenas diferenças de formatação e datas
pdfl compare aprovado_v1.pdf novo_v2.pdf --normalize --ignore-dates

# Aceita até 1% de diferença; abaixo disso vira erro
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### Como funciona

- As páginas são **alinhadas por conteúdo**, não por número: se uma página foi
  inserida no meio, o comparador percebe em vez de acusar tudo depois dela como
  diferente. Funciona em documentos de mais de mil páginas.
- Cada página alinhada recebe uma nota de similaridade e uma amostra das linhas
  que mudaram (`-` saiu, `+` entrou).
- Metadados diferentes viram **aviso**; texto alterado vira **erro** se ficar
  abaixo do threshold, **aviso** se acima.
- O relatório traz o campo `similarity` com a nota geral.

```
page 4 → 4: similarity 97.8% | -título original contos | +título revisado
```

---

## `pdfl watch`

Monitora uma pasta e valida cada PDF que chega ou muda.

```bash
pdfl watch <pasta> --script <script.pdfl> [opções]
```

| Opção | Padrão | O que faz |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | Quais arquivos processar |
| `--exclude <glob>` | — | Quais ignorar |
| `--output-dir <pasta>` | ao lado do PDF | Onde gravar os relatórios |
| `--depth <n>` | `1` | Níveis de subpasta |
| `--debounce <ms>` | `1000` | Espera o arquivo parar de ser copiado |
| `--report json\|csv\|html\|pdf` | `json` | Formato dos relatórios |
| `--fail-fast` | — | Para no primeiro erro |
| `--once` | — | Processa o que já está lá e sai |

```bash
# Pasta de entrada da gráfica, rodando continuamente
pdfl watch entrada/ --script preflight.pdfl --output-dir laudos/ --report html

# Modo lote para CI: processa tudo e sai com o pior código
pdfl watch entrada/ --script preflight.pdfl --once
echo "resultado: $?"

# Ignorando rascunhos
pdfl watch entrada/ --script preflight.pdfl \
  --pattern "*.pdf" --exclude "*_rascunho*"
```

O **debounce** existe porque arquivos grandes chegam aos poucos: o watch só
processa quando o arquivo para de mudar, evitando ler um PDF pela metade.

Os relatórios saem como `<nome>.report.json` (ou `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Aplica operações `fix::` e salva um PDF novo. Detalhes no
[capítulo 8](08-fix.md).

```bash
pdfl fix <entrada.pdf> <script.pdfl> --output <saida.pdf> [opções]
```

| Opção | O que faz |
|---|---|
| `--output <arquivo>` | PDF de saída (obrigatório) |
| `--dry-run` | Lista as operações sem salvar |
| `--report json\|csv\|html\|pdf` | Formato do relatório |
| `--report-file <arquivo>` | Grava o relatório em arquivo |

```bash
# Ver o que seria feito, sem tocar em nada
pdfl fix original.pdf normalizar.pdfl --output saida.pdf --dry-run

# Aplicar de verdade
pdfl fix original.pdf normalizar.pdfl --output corrigido.pdf
```

---

## `pdfl inspect`

Resumo rápido de um PDF, sem script.

```bash
pdfl inspect <arquivo.pdf>
```

```
File:     revista.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Revista Exemplo
  Creator: Adobe InDesign 19.3

Fonts:    26
  ABCDEF+Helvetica — embedded
  Arial — NOT embedded
Images:   81 (minimum DPI 136, spaces: DeviceCMYK, Indexed)
Max. estimated TAC: 300% (RGB render approximation)

Warnings:
  ! there are non-embedded fonts
  ! 3 image(s) below 300 DPI
```

É o primeiro comando a rodar quando um arquivo novo chega: em segundos você sabe
se vale a pena abrir.

---

## `pdfl lint`

Analisa um script sem executar, apontando problemas de qualidade.

```bash
pdfl lint <script.pdfl>
```

Detecta:

- variáveis, parâmetros de bloco e functions declarados e **nunca usados**
  (prefixe com `_` para silenciar: `_page`)
- checks **duplicados** ou **vazios**
- namespace desconhecido (`text::`, `struct::`, `visual::`, `prepress::`,
  `codes::`, `fix::`, `data::`)
- `assert`/`require` fora de qualquer check
- uso de `fix::` (que só roda em `pdfl fix`)

```bash
$ pdfl lint perfil.pdfl
perfil.pdfl: warning: variable 'LIMITE' declared and never used
perfil.pdfl: warning: check "Fontes" declared 2 times
```

Sai com código `1` se houver avisos — dá para usar em CI.

---

## `pdfl fmt`

Formata o script: indentação de 2 espaços, espaçamento consistente, linhas em
branco colapsadas. Preserva comentários e unidades (`3mm` continua `3mm`).

```bash
pdfl fmt <script.pdfl>            # formata no lugar
pdfl fmt <script.pdfl> --check    # não altera; sai com 1 se estiver fora do padrão
```

```bash
# Em CI, garantindo padrão na equipe
for f in perfis/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

Gera a documentação de um script a partir do próprio código.

```bash
pdfl doc <script.pdfl> [--output markdown|html]
```

Produz: perfil, tabela de constantes, functions, imports e — para cada check —
as tags e o que ele valida (as mensagens dos `assert` viram a descrição).

```bash
# Markdown para o repositório
pdfl doc prepress.pdfl > docs/perfil-prepress.md

# HTML para enviar a quem não lê código
pdfl doc prepress.pdfl --output html > perfil.html
```

É o artefato para o gerente de produção entender o que o perfil valida sem abrir
o script.

---

## `pdfl pack`

Empacota scripts e bases em um arquivo `.pdflpkg` distribuível.

```bash
pdfl pack <pasta> [--name <nome>] [--version <versão>] [--output <arquivo>]
```

Inclui `.pdfl`, `.csv`, `.txt`, `.json` e `.xlsx` da pasta (recursivamente), com
um `manifest.json` que registra o SHA-256 de cada arquivo. O pacote é
determinístico: mesma pasta gera bytes idênticos.

```bash
pdfl pack perfis/grafica --name perfil-grafica --version 1.0.0
# cria perfil-grafica.pdflpkg
```

---

## `pdfl add`

Instala um pacote local, conferindo os hashes do manifesto.

```bash
pdfl add <pacote.pdflpkg> [--dir <pasta>]
```

```bash
pdfl add perfil-grafica.pdflpkg
# instala em ./pdfl_profiles/perfil-grafica@1.0.0/

pdfl run pdfl_profiles/perfil-grafica@1.0.0/prepress.pdfl arquivo.pdf
```

Se algum arquivo tiver hash diferente do registrado, a instalação é **recusada**
— pacote corrompido ou adulterado não entra.

> Repositório remoto e assinatura digital não fazem parte desta versão: o `add`
> instala a partir de arquivos locais.

---

[← Biblioteca padrão](10-stdlib.md) · [Índice](README.md) · [Próximo: Receitas →](12-receitas.md)
