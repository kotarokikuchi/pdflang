# 13. Changelog

[← Recipes](12-recipes.md) · [Index](README.md)

What changed in each release, and what it may break for you.

The version is still `0.x`, so a minor bump is allowed to break something. When
one does, the entry says exactly what and how to adapt. Nothing here changes
quietly.

---

## Unreleased

### Breaks

- **`pdfl pack` no longer packages spreadsheets** (`.xlsx`, `.xls`, `.ods`), and
  names the file it left behind. No `data::` function can open one, so a package
  carrying it installed cleanly and then failed at the first lookup. If you
  packaged a spreadsheet, export it to `.csv` or `.json` first.

### Added

- `--tags TAG` on `run` filters which checks execute. Repeatable; a check runs
  when it carries any of the tags given.
- `--json` on `inspect` and `lint`, and `--output json` on `doc`. Every
  subcommand can now be read by a program.
- `--output sarif` and `--output junit`, wherever a report format is chosen —
  `run`, `compare`, `watch` and `fix`. SARIF is what GitHub code scanning reads;
  JUnit is what the test panel of any CI reads.
- `pdfl completions <shell>` prints a completion script for bash, zsh, fish,
  elvish or powershell.
- `--quiet` on every command silences progress and confirmations on stderr.
  Errors still appear, and `print()` is untouched — that is the script's own
  output, and dropping it would change what a script does.
- `data::load_dataset` and `data::lookup_value` read `.json` as well as `.csv`:
  an array of arrays, or an array of objects whose first object names the
  columns in the order the file writes them.
- `pdfl test <script>` runs a script against a folder of PDFs and compares each
  report to the one recorded beside it, so a profile that starts finding
  something different fails a test instead of surprising someone downstream.
  `--update` records the expected reports.
- `--jobs <n>` on `pdfl test` runs that many cases at once, each as its own
  process. Eight 41-page files: 8.9s at `--jobs 1`, 1.1s at `--jobs 8`. The
  default stays `1`, since each job holds a document in memory; `--jobs 0` gives
  one per CPU.
- `--jobs <n>` on `pdfl watch` too: files are validated by child processes, so a
  batch pass scales the same way (9.5s to 1.2s on eight 41-page files). The
  report written is identical whatever `--jobs` says.

### Worth knowing

- A `--tags` value that no check carries is an **error**, not an empty pass. A
  pipeline filtering on a misspelled tag would otherwise validate nothing and
  report a clean file.
- A `rule` carries no tags, so `--tags` skips it — the same answer an untagged
  check gets.
- The JSON report gained `checks_run`, the checks and rules that ran. It does
  not bump `schema_version`, because a reader that ignores unknown fields
  survives it. JUnit needs it: the diagnostics only name the checks that found
  something, and a clean run reported as zero tests is a run a CI thinks never
  happened.

---

## 0.12.0

### Added

- Scripts take values from the command line: `--var name=value`, read as
  `vars.name`. A missing one names the flag that would supply it instead of
  resolving to nothing.
- Four worked examples of comparing two documents with `visual::`:
  `proof.pdfl`, `reprint.pdfl`, `scope.pdfl` and `intake.pdfl`.

### Breaks

Nothing. A script that never mentions `vars` behaves exactly as before.

---

## 0.11.0

### Breaks

**Diagnostic identifiers changed shape.** They were `PDFL-001`, a counter
within the run; they are now derived from the finding itself, like
`PDFL-093751a2`.

> Anything matching `PDFL-\d+` stops matching. In exchange, an identifier now
> survives a check being inserted above it, which is what makes an approved
> baseline possible to keep.

**An unreadable input exits `10` instead of `2`.** A corrupt file and a rejected
file used to be indistinguishable to a pipeline.

> If your CI treats `2` as "this file was rejected", it will now see `10` when
> the file was never judged at all. Findings still use `0`, `1` and `2`; a
> script syntax error still uses `3`.

### Added

- A check can declare the severity of its findings:
  `check "..." severity: warning { ... }` — `error` (the default), `warning` or
  `info`. This is what gives `--fail-on warning` something to act on.
- The JSON report opens with `schema_version`, so a consumer can tell which
  shape it is reading. It bumps only when a reader of the previous output would
  break; adding a field does not bump it.

---

## 0.10.1

### Fixed

- The PDF report was partly in Portuguese: the section header read
  `Diagnósticos`, and every diagnostic carried `(linha N)`. Both are English
  now, which is what the documentation always promised.

---

## 0.10.0

### Breaks

**Release targets renamed from `x64` to `amd64`**, so every asset name changed.

**Portable archives are no longer published**, except one for Linux amd64.

> Anything downloading `pdfl-<version>-linux-x64.tar.gz`, or any portable
> archive other than Linux amd64, has to change. Install from the `.deb` in CI —
> the recipes in this documentation were updated to do that — or use the Linux
> amd64 tarball where installing is not an option.

### Fixed

- Two documentation gaps found by auditing the docs against the source:
  `text::detect_personal_data` and `text::detect_pii` accept an optional string
  that was not documented, and `fix::reorder_pages` was written two different
  ways across languages.

---

## 0.9.0

### Added

- Installers for every platform: `.deb` for Linux, `.dmg` for macOS,
  `-setup.exe` and `.msi` for Windows.
- macOS Intel builds, cross-compiled from the Apple Silicon runner.

### Fixed

- The Windows installer was built with paths that resolved against the wrong
  directory, so it never produced a file.
- Release packages carried pdfium's C headers and build files, which only matter
  to someone compiling against pdfium. About 550 KB per package.

---

## 0.8.0

### Added

- Windows x64 joins the released platforms.

> The bundled `pdfium.dll` lives in `pdfium\bin`, not `pdfium\lib`. If you
> package `pdfl` yourself, keep the layout as shipped: the binary looks for the
> library next to itself.

---

## 0.7.0

### Breaks

**Release assets carry the version in their name**, as
`pdfl-<version>-<target>.tar.gz`, and the directory inside carries it too.

> `.../releases/latest/download/<name>` no longer resolves, because that
> endpoint needs an exact file name. Download with a pattern instead:
> `gh release download --pattern 'pdfl-*-linux-amd64.*'`.

### Added

- The codebase, the README and the examples are in English. The documentation
  remains in seven languages.

---

## v0.6.1

First public release. The language, the interpreter and ten CLI commands, with
documentation in seven languages.

---

[← Recipes](12-recipes.md) · [Index](README.md)
