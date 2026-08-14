# Documentação do PDFLang — Português (Brasil)

Guia completo da linguagem `.pdfl` e do CLI `pdfl` — versão 0.18.0.

Todo exemplo desta documentação é código executável e comentado. Se você nunca
usou a linguagem, comece pelo manual (capítulo 1) e depois consulte a referência
conforme a necessidade.

> **Idioma da ferramenta.** As mensagens do `pdfl` — diagnósticos, erros, ajuda
> do CLI e rótulos dos relatórios — são em **inglês**, o padrão para ferramentas
> de linha de comando. Esta documentação está em português, mas um check que
> falha reporta algo como `page 7: 324% ink (limit 300%)`. As mensagens que
> **você escreve** nos seus scripts saem no idioma em que as escrever.

## Índice

| Capítulo | Conteúdo |
|---|---|
| [1. A linguagem](01-linguagem.md) | Manual completo: checks, asserções, tipos, unidades, blocos, functions, imports, regras |
| [2. Tipos do documento](02-tipos.md) | `doc`, `page`, `font`, `image`, `region` — todas as propriedades e métodos |
| [3. `text::`](03-text.md) | Texto: extração, normalização, busca, validações brasileiras, PII |
| [4. `struct::`](04-struct.md) | Estrutura e metadados: objetos, XMP, segurança, hash |
| [5. `visual::`](05-visual.md) | Imagens: resolução, comparação visual, pHash, SSIM, qualidade |
| [6. `prepress::`](06-prepress.md) | Pré-impressão: TAC, separações, spot colors, fontes, caixas |
| [7. `codes::`](07-codes.md) | Códigos de barras e QR: detecção, decodificação, validação |
| [8. `fix::`](08-fix.md) | Normalização: caixas, páginas, marca d'água, merge/split, otimizações |
| [9. `data::`](09-data.md) | Dados externos: glossários, datasets e bases de consulta |
| [10. Biblioteca padrão](10-stdlib.md) | Métodos de listas e strings, funções globais |
| [11. Comandos do CLI](11-cli.md) | `run`, `compare`, `pixelcompare`, `watch`, `fix`, `inspect`, `lint`, `fmt`, `doc`, `pack`, `add`, `test`, `completions` |
| [12. Receitas](12-receitas.md) | Casos completos: gráfica, editora jurídica, laboratório, CI/CD |
| [13. Mudanças](13-changelog.md) | O que mudou em cada versão, e o que isso pode quebrar |

## Começando em 30 segundos

Crie `meu_perfil.pdfl`:

```pdfl
// Todo script é uma lista de checks. Cada check agrupa validações
// relacionadas e vira uma seção do relatório.
check "Estrutura básica" {
  // require: falha com mensagem gerada automaticamente da expressão
  require doc.page_count > 0

  // assert: falha com a mensagem que você escreve
  assert doc.title != "", "PDF sem título nos metadados"
}
```

Execute:

```bash
pdfl run meu_perfil.pdfl documento.pdf
```

O relatório sai em JSON no stdout. O código de saída diz o que aconteceu:
`0` tudo passou, `1` só avisos, `2` erros de validação, `3` erro de sintaxe.

## Convenções desta documentação

- Cada função aparece com **assinatura**, **o que faz**, **o que devolve** e um
  **exemplo comentado**.
- Argumentos entre colchetes são opcionais: `calculate_tac([pagina])`.
- "1-based" significa que a primeira página é `1`, não `0` — a linguagem é feita
  para quem conta páginas como pessoas contam, não como programadores.
- Medidas são sempre em **pontos** (1 pt = 1/72 pol). Use literais de unidade
  (`3mm`, `1in`) e a conversão é automática.

---

Outros idiomas: [English](../en/) · [日本語](../ja/) · [中文](../zh/) ·
[Français](../fr/) · [العربية](../ar/) · [Deutsch](../de/)
