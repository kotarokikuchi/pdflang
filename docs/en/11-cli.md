# 11. CLI commands

[← Standard library](10-stdlib.md) · [Index](README.md) · [Next: Recipes →](12-recipes.md)

Twelve commands: four that work on PDFs, five on scripts, two for
distribution and one for the shell.

| Command | What it does |
|---|---|
| [`run`](#pdfl-run) | Validates a PDF with a script |
| [`compare`](#pdfl-compare) | Compares two versions of a PDF |
| [`watch`](#pdfl-watch) | Watches a folder and validates what arrives |
| [`fix`](#pdfl-fix) | Applies corrections and saves a new PDF |
| [`inspect`](#pdfl-inspect) | Quick summary of a PDF |
| [`lint`](#pdfl-lint) | Analyzes a script without running it |
| [`fmt`](#pdfl-fmt) | Formats a script |
| [`test`](#pdfl-test) | Runs a script against a folder of PDFs and compares each report |
| [`doc`](#pdfl-doc) | Generates documentation from a script |
| [`pack`](#pdfl-pack) | Packages profiles and data files |
| [`add`](#pdfl-add) | Installs a package |
| [`completions`](#pdfl-completions) | Prints a completion script for your shell |

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

## Global options

| Option | What it does |
|---|---|
| `--quiet` | Silences progress and confirmations on stderr |

`--quiet` works before or after the subcommand, and on every one of them. It
removes the lines a person wants and a pipeline does not — `report saved to …`,
`watching …`, the per-file result of `watch`. It does **not** remove errors: a
quiet run that fails still says why.

It does not silence `print()` either. That is the script's own output, and
dropping it would change what the script does. Redirect stderr if you want it
gone.

`--quiet` wins over `--verbose`.

---

## `pdfl run`

Validates a PDF with a script.

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| Option | Default | What it does |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Report format |
| `--output-file <file>` | — | Writes to a file instead of stdout |
| `--fail-on error\|warning` | `error` | With `warning`, warnings also exit 2 |
| `--verbose` | — | Extra information on stderr |
| `--var NAME=VALUE` | — | Value the script reads as `vars.NAME`; repeatable |
| `--tags TAG` | — | Run only checks carrying this tag; repeatable. A tag no check carries is an error, not an empty pass |

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
  "schema_version": 1,
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
  ],
  "checks_run": ["Ink coverage", "Fonts", "Bleed"]
}
```

The same PDF with the same script always produces the **same report, byte for
byte** — so it can be versioned and diffed in CI.

`schema_version` is the first key so a consumer can branch on it before parsing
anything else. It is bumped only when a reader of the previous output would
break; a new field does not bump it.

### SARIF and JUnit

Two more formats, so the result shows up where the team already looks instead of
in a log nobody opens.

```bash
# GitHub code scanning: the findings become annotations on the pull request
pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif

# Any CI's test panel: one test per check, the ones that passed included
pdfl run prepress.pdfl magazine.pdf --output junit --output-file pdfl.xml
```

In SARIF a result is anchored on the **script**, not on the PDF: the line we
know is the line of the check, and the PDF is usually an artifact passing
through CI rather than a file in the repository, so pointing there would
annotate a path that does not exist. The file under validation travels in
`properties.inputFile`, and the diagnostic id in `partialFingerprints` — which
is what lets GitHub recognise a finding it has already seen instead of reopening
it on every run.

In JUnit every check that ran is a test case, including the ones that found
nothing. A format that listed only the failures would report a clean run as zero
tests, and a CI reads that as a run that never happened. An `info` finding does
not fail its case; it is written to `<system-out>`.

```yaml
- name: Preflight
  run: pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif
  # exit 2 is a rejected file, and the upload still has to happen
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

Compares two versions of a PDF: text, structure and metadata.

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| Option | Default | What it does |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format |
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
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Report format |
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
| `--report json\|csv\|html\|pdf\|sarif\|junit` | Report format |
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

`--json` gives the same summary as data.

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

`--json` gives the same warnings as data.

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
pdfl doc <script.pdfl> [--output markdown|html|json]
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

It includes `.pdfl`, `.csv`, `.txt` and `.json` files from the folder
(recursively), plus a `manifest.json` recording the SHA-256 of each file. The
package is deterministic: the same folder produces identical bytes.

A spreadsheet (`.xlsx`, `.xls`, `.ods`) is **not** packaged, and `pack` says
which file it left behind. No `data::` function can open one, so packaging it
would ship a package that installs cleanly and fails at the first lookup.

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

## `pdfl test`

Runs a script against every PDF in a folder and compares each report to the one
recorded beside it. A profile that starts finding something different fails a
test instead of surprising someone downstream.

```bash
pdfl test <script.pdfl> [--dir <folder>] [--update]
```

| Option | Default | What it does |
|---|---|---|
| `--dir <folder>` | `tests/` next to the script | Where the case PDFs live |
| `--update` | — | Records the expected reports instead of comparing them |

A case is a PDF and the report expected of it, side by side:

```
profiles/print-shop/
  prepress.pdfl
  tests/
    approved.pdf
    approved.expected.json
    heavy_ink.pdf
    heavy_ink.expected.json
```

```bash
# First time: record what the script finds today
pdfl test prepress.pdfl --update

# From then on
pdfl test prepress.pdfl
```

```
ok   approved.pdf
FAIL heavy_ink.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Ink coverage (line 12): page 7: 324% ink (limit 300%)
1 passed, 1 failed
```

The failure names what changed — the counts, the verdict, and which findings
appeared or vanished — rather than printing two JSON files side by side.

Recording is always deliberate: a run that refreshed its own baseline would
never fail. Read the diff first, then re-record with `--update` when the change
is the one you meant.

The expected report is the one `pdfl run` produces, with `input_file` reduced to
the file's name — a baseline that changed with the directory you invoked it from
would not be a baseline. A PDF that cannot be opened fails its own case and
leaves the others to run.

Exit codes: `0` all passed, `2` at least one failed, `10` the folder could not
be read or holds no PDF.

---

## `pdfl completions`

Prints a completion script for your shell to stdout.

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash, for the current user
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — anywhere on your $fpath
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

Nothing else is written to stdout, so the output can be redirected straight into
the completion directory. Regenerate it after upgrading: the script is built
from the commands and flags of the binary that printed it.

---

[← Standard library](10-stdlib.md) · [Index](README.md) · [Next: Recipes →](12-recipes.md)
