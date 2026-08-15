# 13. Changelog

[← Recipes](12-recipes.md) · [Index](README.md)

What changed in each release, and what it may break for you.

The version is still `0.x`, so a minor bump is allowed to break something. When
one does, the entry says exactly what and how to adapt. Nothing here changes
quietly.

---

## Unreleased

### Changed

- The `pixelcompare` viewer is three panes side by side — the original, the new
  file, and both with the differences painted over them — instead of one pane
  and a choice of modes. Dragging still wipes the new file over the old one, and
  now the two references stay put while you do it, so you can see what the wipe
  is cutting between. The bar is in all three panes at the
  same place and moves in all three at once: in the difference pane it cuts, in
  the other two it is a ruler down the same column of the page. A second, flat
  bar follows the pointer and moves in all three as well: in the difference pane
  the new file shows to the right of the upright bar and below the flat one, so
  where they cross is the corner of what is revealed — and the dot rides the
  upright bar at that height, marking the spot the pointer is holding. The flat
  bar starts at the top, which leaves the upright one a plain full-height wipe
  until it is moved.
- The wheel zooms all three panes together, up to 8×, around the point under the
  pointer, and stops at the fitted page on the way out. The bars keep their
  weight at any zoom. **Reset view** puts the zoom and the bars back where they
  started, and is disabled while there is nothing to undo.
- The dot on the upright bar is translucent, like the bars in the other two
  panes. It sits on the part of the page being looked at, and an opaque disc
  hides exactly what it is pointing at.
- The panes are sized against the window: the whole comparison is on screen
  without scrolling, at any window shape, and each keeps the page's own
  proportions. Where the two files disagree about a page's size, each is shown
  whole inside the shared box rather than stretched to fill it.
- It opens on the pages that differ, with **All** to put the rest back. On a
  long document those pages are the reason it was opened.
- **Flip**, **Fade**, the **Blend** slider and the **Differences** slider are
  gone, along with the `space` and `d` keys that drove them. Three panes answer
  what those modes were for, and the controls now fit one thin bar, which is
  space the pages get instead.

---

## 0.18.0

### Breaks

**The identifier of a `loading` finding changes.** It is derived from the
message, and the message no longer carries the line breaks pdfium put in it.

> Only findings from a document that could not be read are affected — no check
> of your own produces one. If a baseline approves such an identifier, it needs
> to be recorded again. This is what makes the release a minor rather than a
> patch: an identifier is what a baseline is built on, so it does not change
> quietly.

### Fixed

- `pdfl pack` recorded a nested file with the separator of the machine that
  built the package — on Windows, `data\batches.csv`. A package is built on one
  machine and installed on another, so anything but `/` is a package that
  installs and then cannot find its own files; our own verification could not
  find them either. Packages built on Linux and macOS were never affected.
- `--output` and `--output-file` were ignored when the PDF could not be read.
  `run`, `fix` and `compare` printed JSON to stdout whatever was asked for, so
  a pipeline that asked for JUnit in a file got no file at all and a report on
  a stream it was not reading — a corrupt input looked like a run that never
  happened, while the exit code correctly said `10`. The failure report now
  goes out through the same path as every other report.
- A pdfium error reached the report as a pretty-printed Rust enum spread over
  three lines. A diagnostic is one field of a CSV row and one XML attribute, so
  it is now folded onto a single line.

---

## 0.17.0

### Added

- `pdfl pixelcompare` shows a progress bar for each stage — rasterising either
  file, comparing, writing the viewer. It is drawn only when stderr is a
  terminal, because the bar overwrites its own line and a log file has no
  cursor to move; redirected, it stays silent. `--quiet` silences it anywhere.
- `--jobs <n>` on `pdfl pixelcompare` compares that many pages at a time, and
  defaults to one per CPU: 41 pages at 150 dpi go from 3.6s to 1.2s. Only the
  comparison is parallel — pdfium serialises every call behind one global lock,
  so rasterising cannot be, which is why the gain is threefold rather than
  eightfold. The report is unaffected by the value: pages are folded back in
  page order, so the diagnostics, their order and their fingerprints are
  identical, and so are the viewer's files.

  > This defaults to every CPU while `test` and `watch` default to `--jobs 1`.
  > There a job is a child process holding its own document; here the pages are
  > already in memory and the threads share them.

---

## 0.16.0

### Added

- `pdfl pixelcompare` compares two PDFs by what they look like rather than by
  their text, page by page, and reports the share of pixels that differ. It
  aligns a page that only moved before comparing it, so a one-pixel offset does
  not bury the change that matters.
- `--viewer <folder>` on `pixelcompare` writes a self-contained application —
  no CDN, no bundler, no server — to wipe, flip or fade between the two files
  with the differences painted in place: red for ink that is gone, green for
  ink that is new, blue for the same weight in another colour. The page strip
  filters to **Changed only**, and the arrow keys follow the filter — on a long
  document, paging past the pages that did not move is the slow part.

---

## 0.15.0

### Added

- `if` / `else if` / `else`, as an **expression**: its value is the last
  expression of whichever branch ran, the same rule a function already follows.
  So it serves both as a value (`const LIMIT = if coated { 300 } else { 260 }`)
  and as a guard around statements, with no second syntax. A branch that does
  not run yields `null`, and each branch has its own scope — assigning to a
  variable that already exists outside still updates that one.

---

## 0.14.0

### Fixed

- `--var` now reaches `pdfl test` and `pdfl watch`, not just `pdfl run`.
  Neither forwarded it to the children they spawn, so a script reading
  `vars.*` could not be tested or watched at all — every case or file failed
  with "was not provided", regardless of its content.

---

## 0.13.0

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
- `--events` on `pdfl watch` waits on filesystem notifications instead of a
  timer. Opt-in, not the default: inotify on an NFS or SMB mount reports only
  what the local machine writes, so a network hot folder would go quiet. If the
  watcher cannot be created, watch says so and falls back to the timer.
- `--journal <file>` on `pdfl watch`: an append-only record of what was
  validated, one JSON object per file. Re-running with the same journal skips
  the files it covers — a batch interrupted at four thousand of five thousand
  finishes the thousand that are left — while still reporting their verdicts, so
  a resumed batch never claims a folder is clean.
- `--timeout <s>` on `pdfl watch` kills a file's analysis past that many
  seconds and reports it as rejected — one finding, `check_name: "timeout"` —
  instead of leaving the batch stuck. Recursion in a `.pdfl` script is already
  depth-limited, so this is for what a script cannot cause: pdfium looping or
  blocking on a malformed or adversarial PDF.

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

### Fixed

- `pdfl watch` now wakes when the freshest file has settled, instead of up to
  one full interval later. With `--debounce 3000`, a file that arrives is
  reported about 3s later rather than up to 6s.

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
