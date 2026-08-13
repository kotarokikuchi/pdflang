# PDFLang (.pdfl)

[![CI](https://github.com/kotarokikuchi/pdflang/actions/workflows/ci.yml/badge.svg)](https://github.com/kotarokikuchi/pdflang/actions/workflows/ci.yml)

A scripting language for validating PDFs, interpreted by the `pdfl` CLI (Rust).
Built to be readable by non-technical people — no object orientation, just
`check`s and assertions.

📖 **Full documentation** — language manual, reference for every function and
ready-made recipes:
[English](docs/en/) · [Português (Brasil)](docs/pt-br/) · [日本語](docs/ja/) ·
[中文](docs/zh/) · [Français](docs/fr/) · [العربية](docs/ar/) · [Deutsch](docs/de/)

CLI messages, diagnostics and reports are in **English**.

## Installation

Every [release](https://github.com/kotarokikuchi/pdflang/releases) ships an
installer per platform, with pdfium bundled. Each asset has a matching
`.sha256`.

| Platform | Installer |
|---|---|
| Linux amd64 / arm64 | `pdfl_<version>_amd64.deb` · `pdfl_<version>_arm64.deb` |
| macOS Intel / Apple Silicon | `pdfl-<version>-macos-amd64.dmg` · `-arm64.dmg` |
| Windows amd64 | `pdfl-<version>-windows-amd64-setup.exe` · `.msi` |

Linux amd64 also gets a portable `pdfl-<version>-linux-amd64.tar.gz`, for when
installing is not an option — a container without `dpkg`, a distribution that is
not Debian-based, a machine where you are not root.

```bash
# Debian, Ubuntu and derivatives
sudo dpkg -i pdfl_*_amd64.deb
pdfl inspect document.pdf
```

On macOS, open the `.dmg` and copy `pdfl` wherever you keep local binaries. On
Windows, run the `-setup.exe` and keep the "add to PATH" box ticked; then use
`pdfl` from any terminal.

For unattended Windows deployment — Group Policy, Intune, or a scripted rollout
— use the `.msi` instead. It installs to Program Files and adds itself to the
system `PATH`:

```powershell
msiexec /i pdfl-<version>-windows-amd64.msi /qn
```

> The macOS and Windows installers are unsigned, so Gatekeeper and SmartScreen
> will warn on first run. On macOS, right-click → Open; on Windows, "More info"
> → "Run anyway".

The portable tarball needs no privileges at all:

```bash
tar -xzf pdfl-*-linux-amd64.tar.gz && cd pdfl-*/
./pdfl inspect document.pdf
```

In CI, install from the `.deb` — that is what the recipes in the documentation
do. On any other platform, build from source.

## Quick start (building from source)

```bash
./setup_pdfium.sh          # downloads the native pdfium library (once)
cargo build --release
./target/release/pdfl run examples/basic.pdfl document.pdf --output json
```

Exit codes: `0` = OK, `1` = warnings, `2` = validation errors, `3` = syntax
error, `10` = the document could not be read or a file could not be written —
kept out of the 0–2 range so CI can tell a broken input from a rejected one.

Output formats (`--output` on `run`/`compare`, `--report` on `fix`/`watch`):
`json` (default), `csv` (one line per diagnostic), `html` (self-contained) and
`pdf` (A4 audit file). Text formats go to stdout or to `--output-file`; `pdf`
always writes to a file (`--output-file` or `<input>.report.pdf`). `print()` and
progress go to stderr.

## Example script

```pdfl
profile "basic-validation" {
  const MIN_PAGES = 1

  check "Structure" tags: ["basic"] {
    require doc.page_count >= MIN_PAGES
    assert doc.title != "", "PDF has no title"
  }

  check "Fonts" {
    doc.fonts.each { |font|
      assert font.is_embedded, "Font #{font.name} is not embedded"
    }
  }
}
```

- `require expr` — fails with a message generated automatically from the expression
- `assert expr, "message"` — fails with a custom message (accepts `#{...}` interpolation)
- **Units**: `3mm`, `2.5cm`, `1in`, `10pt` become points automatically
  (`const BLEED = 3mm`); `300%` keeps the numeric value
- **Functions**: `function double(x) { x * 2 }` — the value is that of the last
  expression; call it from any check (`require double(21) == 42`)
- **Imports**: `import "library.pdfl"` — path relative to the script; loads
  functions, constants and checks (each file is imported only once)
- `doc` — the loaded PDF: `page_count`, `title`, `author`, `pages`, `fonts`, `extract_text()`
- `page` — `number`, `width`, `height` (points), `extract_text()`
- Lists: `each`, `all`, `any`, `filter`, `map`, `length`, `contains`, `join`
- Strings: `contains`, `starts_with`, `ends_with`, `trim`, `length`, `to_uppercase`, `to_lowercase`
- Globals: `min`, `max`, `abs`, `round`, `print`

## Namespace `text::`

Functions over the document's text (most accept a string as an optional argument
to operate on it instead of the document):

- Extraction: `text::extract_all()`, `text::extract_from_page(n)`
- Normalization: `text::normalize()`, `text::split_words()`, `text::split_sentences()`,
  `text::split_paragraphs()`, `text::count_words()`, `text::count_characters()`,
  `text::detect_language()` (pt/en/es)
- Validation (return a boolean, for use with `require`/`assert`):
  `text::require_text("term")`, `text::forbid_text("term")`,
  `text::require_match("regex")`, `text::forbid_match("regex")`,
  `text::fuzzy_match(a, b)` (similarity 0.0–1.0)
- Personal data: `text::detect_personal_data()` / `text::detect_pii()` —
  lists occurrences of CPF, CNPJ, e-mail and phone numbers

Full example in [examples/text.pdfl](examples/text.pdfl).

## Namespace `struct::`

Structure and metadata of the PDF file:

- Metadata: `struct::get_title()`, `struct::get_author()`, `struct::get_producer()`,
  `struct::get_creator()`, `struct::get_subject()`, `struct::get_keywords()`,
  `struct::get_creation_date()`, `struct::get_modification_date()` (dates in
  `YYYY-MM-DD HH:MM:SS` format), `struct::list_metadata_entries()`
- Objects and file: `struct::count_objects()`, `struct::file_size()` (bytes),
  `struct::calculate_sha256()`, `struct::detect_file_bloat(kb_per_page)` (default 1024)

Full example in [examples/structure.pdfl](examples/structure.pdfl).

## Namespace `visual::`

Images in the document:

- `visual::detect_images()`, `visual::count_images()`
- `visual::get_image_resolution(n)` (effective DPI), `visual::get_image_size(n)` (`[width, height]` px)
- `visual::detect_image_color_space()` (list of the color spaces present) or `(n)` (of the nth image)
- `visual::detect_low_resolution(min_dpi)` — `true` if any image falls below it (default 300)

Images are values too: `doc.images` / `page.images`, with `width`, `height`,
`dpi`, `dpi_x`, `dpi_y`, `color_space`, `page_number`, `bits_per_pixel`. The DPI
is the effective one (pixels ÷ printed size on the page), not the one in the
metadata.

Full example in [examples/images.pdfl](examples/images.pdfl).

Comparing one document against another — SSIM, perceptual hash, pixel diff and
the quality checks — has four worked examples:

| Example | Question it answers |
|---|---|
| [proof.pdfl](examples/proof.pdfl) | did anything move since the client signed off? |
| [reprint.pdfl](examples/reprint.pdfl) | is this reprint the same book, apart from the colophon that *must* differ? |
| [scope.pdfl](examples/scope.pdfl) | did the correction stay within what was agreed — and did anything change at all? |
| [intake.pdfl](examples/intake.pdfl) | reject or accept, and for which reason: reordered, rewritten, or degraded? |

`scope.pdfl` and `intake.pdfl` are meant to run in that order: the first decides
whether a resubmission is worth reviewing, the second says what is wrong with
it.

## Namespace `prepress::`

Prepress validations:

- TAC/ink: `prepress::calculate_tac([page])`, `prepress::calculate_ink_coverage([page])`,
  `prepress::validate_tac_limits(limit)` (default 300). **Note**: this TAC is
  estimated from an RGB render and is a *lower* bound on the real one — neutral
  colors (rich black) collapse into pure K. For the trustworthy number use
  `prepress::calculate_exact_tac([page])`, which reads the separations declared
  in the content stream
- Lines: `prepress::detect_hairlines(pt)` (default 0.25), `prepress::detect_fine_lines(pt)`
  (default 1.0), `prepress::validate_minimum_stroke_width(pt)`
- Colors: `prepress::detect_color_mode()` (RGB/CMYK/Mixed/None), `prepress::validate_color_space("DeviceCMYK")`
- Fonts: `prepress::list_fonts()`, `prepress::validate_font_embedding()`
- Pages: `prepress::get_page_size(n)`, `prepress::get_page_boxes(n)`,
  `prepress::validate_media_box()`/`validate_trim_box()`/`validate_bleed_box()`,
  `prepress::check_page_geometry(mm)` (minimum bleed, default 3mm)

On pages: `page.tac`, `page.ink_coverage`, `page.min_stroke_width`,
`page.has_trim_box`/`has_bleed_box`/`has_media_box`/`has_crop_box`/`has_art_box`.

Full example in [examples/prepress.pdfl](examples/prepress.pdfl).

## Namespace `codes::`

Barcodes and QR codes (decoded with [rxing](https://crates.io/crates/rxing); the
scan renders the pages and runs only on the first use of `codes::`):

- Detection: `codes::detect_barcodes()`, `codes::detect_qrcodes()`, `codes::count_barcodes()`,
  `codes::get_barcode_type(n)` (EAN_13, QR_CODE, CODE_128...), `codes::get_barcode_location(n)`
  (`[page, x, y]` in points)
- Decoding: `codes::decode_barcode(n)`, `codes::validate_barcode_checksum(n)` /
  `codes::validate_gtin(s)` / `codes::validate_ean(s)` (GTIN check digit),
  `codes::validate_code128()`
- Comparison: `codes::compare_barcode_with_text()` (the code's content appears in the text),
  `codes::validate_barcode_format("regex")`, `codes::validate_barcode_position(x0, y0, x1, y1)`

Full example in [examples/barcodes.pdfl](examples/barcodes.pdfl).

## Namespace `data::`

Local glossaries and datasets (offline-first — paths relative to the working
directory; files are cached for the duration of the run):

- `data::load_glossary("terms.txt")` — list of terms (one per line, `#` comments)
- `data::load_dataset("data.csv")` — list of rows (each row is a list of columns;
  CSV with standard quoting)
- `data::lookup_value("data.csv", key)` — second column of the row whose first
  column is the key; `null` if not found (works directly in `assert`)
- `data::validate_against_reference("terms.txt")` — glossary terms that do NOT
  appear in the document's text (empty list = all present)

Lists gain `get(n)` (1-based), `first()` and `last()` — handy for CSV rows.
Reference-base lookups: `query_gtin`, `query_medicamento`, `query_postal_code`
and `validate_address` — they need the CSVs in `./dados/`,
`./pdfl_profiles/*/dados/`, `./` or `$PDFL_DATA_DIR`.

Full example in [examples/data.pdfl](examples/data.pdfl) — includes cross-checking
the PDF's barcode against a local batch table.

## Namespace `fix::` (command `pdfl fix`)

Normalization — the only namespace that **writes** a new PDF, which is why it
runs under its own command:

```bash
pdfl fix input.pdf script.pdfl --output fixed.pdf [--dry-run]
```

- Boxes: `fix::set_page_size(w, h)`, `fix::set_crop_box(x0, y0, x1, y1)`,
  `fix::set_trim_box(...)`, `fix::set_bleed_box(...)` (points)
- Pages: `fix::rotate_page([page,] degrees)` (90/180/270; without a page = all),
  `fix::delete_page(n)`, `fix::duplicate_page(n)`, `fix::reorder_pages([2, 1, 3])`
- Content: `fix::add_watermark("text")`, `fix::add_page_numbers()`

Operations are validated at call time (nonexistent page, invalid rotation,
incomplete ordering → a friendly error) and applied in sequence at the end. The
JSON report gains a `fixes` field with what was applied. Under `pdfl run`,
`fix::` calls are an error — normalization only in the `fix` command.

Full example in [examples/normalize.pdfl](examples/normalize.pdfl).

## Command `pdfl compare`

Compares two versions of a PDF (text, structure and metadata):

```bash
pdfl compare v1.pdf v2.pdf [--output json|csv|html] [--normalize] \
  [--ignore-dates] [--similarity-threshold 95]
```

- Pages are aligned by content similarity (insertions and removals are detected
  even when the count changes)
- Each aligned page gets a similarity score (word-level Levenshtein) and a
  sample of the changed lines (`-removed | +added`)
- Changed metadata becomes warnings; text changes above the
  `--similarity-threshold` do too (below it, errors)
- `--ignore-dates` replaces dates (dd/mm/yyyy, yyyy-mm-dd, "1 de março de 2026")
  with a marker before comparing; `--normalize` ignores case and spacing
- The report carries an overall `similarity` (0–100); exit codes: 0 identical,
  1 warnings only, 2 differences beyond what is tolerated

## Command `pdfl watch`

Watches a folder and validates each new or changed PDF, writing the report next
to the file (or into `--output-dir`):

```bash
pdfl watch input/ --script profile.pdfl [--pattern "*.pdf"] [--exclude "*_draft*"] \
  [--output-dir reports/] [--depth 1] [--debounce 1000] [--report json|csv|html] \
  [--fail-fast] [--once]
```

- Polling with debounce: a file is only processed once it stops being written to
- `--once` processes what is already in the folder and exits with the worst exit
  code (0/1/2) — good for batches and CI; without `--once` it runs until Ctrl+C
- Reports are written as `<name>.report.json|csv|html`; the progress log goes to
  stderr

## Commands `pdfl pack` and `pdfl add`

Profiles as code, distributable (offline):

```bash
pdfl pack profiles/printshop --name printshop-profile --version 1.0.0
# creates printshop-profile.pdflpkg (.pdfl scripts + datasets, manifest with SHA-256)

pdfl add printshop-profile.pdflpkg       # installs into ./pdfl_profiles/<name>@<version>/
pdfl run pdfl_profiles/printshop-profile@1.0.0/prepress.pdfl file.pdf
```

The package is a deterministic tar.gz; `add` checks each file's hash against the
manifest (a tampered package is refused). A remote repository and digital
signatures are not implemented yet.

## Commands `pdfl inspect` and `pdfl doc`

```bash
pdfl inspect document.pdf               # quick summary: pages, boxes, metadata,
                                        # fonts, images, estimated TAC and general warnings
pdfl doc script.pdfl                    # script documentation in Markdown
pdfl doc script.pdfl --output html      # or as self-contained HTML
```

From the script itself, `doc` generates: the profile, a table of constants and,
for each check, its tags and what it validates (the `assert` messages and the
`require` conditions). Scripts using `fix::` get a note that they run via
`pdfl fix`.

## Commands `pdfl lint` and `pdfl fmt`

Quality of `.pdfl` scripts, without running them:

```bash
pdfl lint script.pdfl           # warnings (exit 1 if there are any)
pdfl fmt script.pdfl            # formats in place (2 spaces, standard spacing)
pdfl fmt script.pdfl --check    # exit 1 if it is not formatted (CI)
```

`lint` reports: block variables/parameters declared and never used (an `_`
prefix silences it), duplicate or empty checks, unknown namespace, `assert`
outside a check, and use of `fix::` outside the `pdfl fix` command. The formatter
preserves comments and the author's line breaks.

## Development

```bash
cargo test    # lexer, parser, interpreter and report (no pdfium needed)
```

Releasing means bumping `version` in `Cargo.toml`, then:

```bash
./scripts/sync-version.sh    # rewrites the version line in docs/*/README.md
```

CI runs the same script with `--check` and fails if the two disagree, so a bump
that forgets the documentation cannot merge.

## About this project

pdfl is developed by one person, with the code open. **Pull requests are turned
off on this repository** — not as a judgement on anyone's patch, but so that
nobody writes one expecting it to be read.

Bug reports and questions are welcome as
[issues](https://github.com/kotarokikuchi/pdflang/issues): a reproducible bug is
useful even when nobody else can fix it.

The MIT licence means you can fork this and take it wherever you want, and
[ROADMAP.md](ROADMAP.md) exists so that a fork inherits the reasoning — what is
built, what is planned, and what was deliberately left out and why.

## License

[MIT](LICENSE). The native pdfium library, downloaded by `setup_pdfium.sh` and
bundled into the release packages, comes with its own licenses: PDFium is
3-clause BSD, the binary distribution is MIT, and the transitive dependencies
are listed in `pdfium/licenses/`.
