# PDFLang Documentation — English

Complete guide to the `.pdfl` language and the `pdfl` CLI — version 0.15.0.

Every example in this documentation is runnable, commented code. If you have
never used the language, start with the manual (chapter 1) and use the rest as
reference.

## Table of contents

| Chapter | Contents |
|---|---|
| [1. The language](01-language.md) | Full manual: checks, assertions, types, units, blocks, functions, imports, rules |
| [2. Document types](02-types.md) | `doc`, `page`, `font`, `image`, `region` — all properties and methods |
| [3. `text::`](03-text.md) | Text: extraction, normalization, search, Brazilian validations, PII |
| [4. `struct::`](04-struct.md) | Structure and metadata: objects, XMP, security, hashing |
| [5. `visual::`](05-visual.md) | Images: resolution, visual comparison, pHash, SSIM, quality |
| [6. `prepress::`](06-prepress.md) | Prepress: ink coverage, separations, spot colors, fonts, boxes |
| [7. `codes::`](07-codes.md) | Barcodes and QR codes: detection, decoding, validation |
| [8. `fix::`](08-fix.md) | Normalization: boxes, pages, watermarks, merge/split, optimization |
| [9. `data::`](09-data.md) | External data: glossaries, datasets and lookup tables |
| [10. Standard library](10-stdlib.md) | List and string methods, global functions |
| [11. CLI commands](11-cli.md) | `run`, `compare`, `pixelcompare`, `watch`, `fix`, `inspect`, `lint`, `fmt`, `doc`, `pack`, `add`, `test`, `completions` |
| [12. Recipes](12-recipes.md) | Complete cases: print shop, legal publisher, lab, CI/CD |
| [13. Changes](13-changelog.md) | What changed in each version, and what it may break |

## Getting started in 30 seconds

Create `my_profile.pdfl`:

```pdfl
// Every script is a list of checks. Each check groups related validations
// and becomes a section of the report.
check "Basic structure" {
  // require: fails with a message generated from the expression itself
  require doc.page_count > 0

  // assert: fails with the message you write
  assert doc.title != "", "PDF has no title in its metadata"
}
```

Run it:

```bash
pdfl run my_profile.pdfl document.pdf
```

The report goes to stdout as JSON. The exit code tells you what happened:
`0` everything passed, `1` warnings only, `2` validation errors, `3` syntax error.

## Conventions used here

- Each function is listed with its **signature**, **what it does**, **what it
  returns** and a **commented example**.
- Arguments in square brackets are optional: `calculate_tac([page])`.
- "1-based" means the first page is `1`, not `0` — the language counts pages the
  way people do, not the way programmers do.
- Measurements are always in **points** (1 pt = 1/72 in). Use unit literals
  (`3mm`, `1in`) and conversion happens automatically.

---

Other languages: [Português (Brasil)](../pt-br/) · [日本語](../ja/) ·
[中文](../zh/) · [Français](../fr/) · [العربية](../ar/) · [Deutsch](../de/)
