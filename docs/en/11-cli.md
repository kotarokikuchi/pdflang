# 11. CLI commands

[← Standard library](10-stdlib.md) · [Index](README.md) · [Next: Recipes →](12-recipes.md)

Ten commands: four that work on PDFs, four on scripts and two for distribution.

| Command | What it does |
|---|---|
| [`run`](#pdfl-run) | Validates a PDF with a script |
| [`compare`](#pdfl-compare) | Compares two versions of a PDF |
| [`watch`](#pdfl-watch) | Watches a folder and validates what arrives |
| [`fix`](#pdfl-fix) | Applies corrections and saves a new PDF |
| [`inspect`](#pdfl-inspect) | Quick summary of a PDF |
| [`lint`](#pdfl-lint) | Analyzes a script without running it |
| [`fmt`](#pdfl-fmt) | Formats a script |
| [`doc`](#pdfl-doc) | Generates documentation from a script |
| [`pack`](#pdfl-pack) | Packages profiles and data files |
| [`add`](#pdfl-add) | Installs a package |

---

## Exit codes

Every validating command uses the same convention:

| Code | Meaning |
|---|---|
| `0` | Everything passed |
| `1` | Warnings only |
| `2` | Validation errors |
| `3` | Syntax error in the script |
| `10` | The document could not be read, or a file could not be written — no verdict was reached |

In shell scripts:

```bash
pdfl run profile.pdfl file.pdf > report.json
case $? in
  0) echo "approved" ;;
  1) echo "approved with warnings" ;;
  2) echo "rejected — see report.json" ;;
  3) echo "error in the validation script" ;;
esac
```

---

## `pdfl run`

Validates a PDF with a script.

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| Option | Default | What it does |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Report format |
| `--output-file <file>` | — | Writes to a file instead of stdout |
| `--fail-on error\|warning` | `error` | With `warning`, warnings also exit 2 |
| `--verbose` | — | Extra information on stderr |
| `--var NAME=VALUE` | — | Value the script reads as `vars.NAME`; repeatable |

```bash
# JSON report in the terminal
pdfl run prepress.pdfl magazine.pdf

# HTML to send back to the client
pdfl run prepress.pdfl magazine.pdf --output html --output-file report.html

# Audit PDF (the pdf format always writes to a file)
pdfl run prepress.pdfl magazine.pdf --output pdf --output-file report.pdf

# CSV for a spreadsheet
pdfl run prepress.pdfl magazine.pdf --output csv --output-file findings.csv

# Strict: warnings fail too
pdfl run prepress.pdfl magazine.pdf --fail-on warning
```

### The JSON report

```json
{
  "script_name": "prepress.pdfl",
  "input_file": "magazine.pdf",
  "profile": "offset-magazine",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Ink coverage",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ]
}
```

The same PDF with the same script always produces the **same report, byte for
byte** — so it can be versioned and diffed in CI.

---

## `pdfl compare`

Compares two versions of a PDF: text, structure and metadata.

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| Option | Default | What it does |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Format |
| `--output-file <file>` | — | Writes to a file |
| `--normalize` | — | Ignores case and spacing |
| `--ignore-dates` | — | Masks dates before comparing |
| `--similarity-threshold <0-100>` | `100` | Minimum acceptable similarity |

```bash
# Straight comparison
pdfl compare approved_v1.pdf new_v2.pdf

# Tolerating small formatting and date differences
pdfl compare approved_v1.pdf new_v2.pdf --normalize --ignore-dates

# Accepts up to 1% difference; below that it becomes an error
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### How it works

- Pages are **aligned by content**, not by number: if a page was inserted in the
  middle, the comparison notices instead of flagging everything after it as
  different. It scales to documents of over a thousand pages.
- Each aligned page gets a similarity score and a sample of the lines that
  changed (`-` removed, `+` added).
- Changed metadata becomes a **warning**; changed text becomes an **error** when
  it falls below the threshold and a **warning** when above it.
- The report carries a `similarity` field with the overall score.

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

Watches a folder and validates every PDF that arrives or changes.

```bash
pdfl watch <folder> --script <script.pdfl> [options]
```

| Option | Default | What it does |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | Which files to process |
| `--exclude <glob>` | — | Which to skip |
| `--output-dir <folder>` | next to the PDF | Where to write reports |
| `--depth <n>` | `1` | Subfolder levels |
| `--debounce <ms>` | `1000` | Waits for the file to stop being copied |
| `--report json\|csv\|html\|pdf` | `json` | Report format |
| `--fail-fast` | — | Stops at the first error |
| `--once` | — | Processes what is already there and exits |

```bash
# The print shop's inbox, running continuously
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# Batch mode for CI: process everything and exit with the worst code
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"

# Skipping drafts
pdfl watch inbox/ --script preflight.pdfl \
  --pattern "*.pdf" --exclude "*_draft*"
```

**Debounce** exists because large files arrive in pieces: watch only processes a
file once it stops changing, so it never reads half a PDF.

Reports are written as `<name>.report.json` (or `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Applies `fix::` operations and saves a new PDF. Details in
[chapter 8](08-fix.md).

```bash
pdfl fix <input.pdf> <script.pdfl> --output <output.pdf> [options]
```

| Option | What it does |
|---|---|
| `--output <file>` | Output PDF (required) |
| `--dry-run` | Lists the operations without saving |
| `--report json\|csv\|html\|pdf` | Report format |
| `--report-file <file>` | Writes the report to a file |

```bash
# See what would happen, touching nothing
pdfl fix original.pdf normalize.pdfl --output out.pdf --dry-run

# Apply for real
pdfl fix original.pdf normalize.pdfl --output fixed.pdf
```

---

## `pdfl inspect`

Quick summary of a PDF, no script needed.

```bash
pdfl inspect <file.pdf>
```

```
File:     magazine.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Example Magazine
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

This is the first command to run when a new file lands: within seconds you know
whether it is worth opening.

---

## `pdfl lint`

Analyzes a script without running it, reporting quality issues.

```bash
pdfl lint <script.pdfl>
```

It detects:

- variables, block parameters and functions that are declared and **never used**
  (prefix with `_` to silence: `_page`)
- **duplicate** or **empty** checks
- unknown namespaces (`text::`, `struct::`, `visual::`, `prepress::`, `codes::`,
  `fix::`, `data::`)
- `assert`/`require` outside any check
- use of `fix::` (which only runs under `pdfl fix`)

```bash
$ pdfl lint profile.pdfl
profile.pdfl: warning: variable 'LIMIT' declared and never used
profile.pdfl: warning: check "Fonts" declared 2 times
```

Exits with `1` when there are warnings — usable in CI.

---

## `pdfl fmt`

Formats the script: two-space indentation, consistent spacing, collapsed blank
lines. Comments and units are preserved (`3mm` stays `3mm`).

```bash
pdfl fmt <script.pdfl>            # formats in place
pdfl fmt <script.pdfl> --check    # changes nothing; exits 1 if unformatted
```

```bash
# In CI, enforcing a team standard
for f in profiles/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

Generates documentation for a script from the code itself.

```bash
pdfl doc <script.pdfl> [--output markdown|html]
```

It produces: the profile, a table of constants, functions, imports and — for each
check — its tags and what it validates (the `assert` messages become the
description).

```bash
# Markdown for the repository
pdfl doc prepress.pdfl > docs/prepress-profile.md

# HTML to hand to people who do not read code
pdfl doc prepress.pdfl --output html > profile.html
```

This is the artifact that lets a production manager understand what a profile
validates without opening the script.

---

## `pdfl pack`

Packages scripts and data files into a distributable `.pdflpkg`.

```bash
pdfl pack <folder> [--name <name>] [--version <version>] [--output <file>]
```

It includes `.pdfl`, `.csv`, `.txt`, `.json` and `.xlsx` files from the folder
(recursively), plus a `manifest.json` recording the SHA-256 of each file. The
package is deterministic: the same folder produces identical bytes.

```bash
pdfl pack profiles/print-shop --name print-profile --version 1.0.0
# creates print-profile.pdflpkg
```

---

## `pdfl add`

Installs a local package, verifying the manifest hashes.

```bash
pdfl add <package.pdflpkg> [--dir <folder>]
```

```bash
pdfl add print-profile.pdflpkg
# installs into ./pdfl_profiles/print-profile@1.0.0/

pdfl run pdfl_profiles/print-profile@1.0.0/prepress.pdfl file.pdf
```

If any file's hash differs from the recorded one, installation is **refused** —
a corrupted or tampered package never lands.

> A remote repository and digital signatures are not part of this version: `add`
> installs from local files.

---

[← Standard library](10-stdlib.md) · [Index](README.md) · [Next: Recipes →](12-recipes.md)
