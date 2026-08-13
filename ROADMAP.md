# Roadmap

What exists, what does not, and what gets built next.

pdfl is a **solo project with open code**. This document is not a call for
contributions; it is here so that anyone deciding whether to depend on the tool
can see where it is going, and so that anyone who forks it inherits the reasoning
behind what was left out.

Every "exists" claim was verified by reading the source or running the binary,
not by reading the documentation — during this project the documentation was
found to disagree with the code more than once. Where a claim could not be
verified, it is marked **partial** with the doubt stated, never rounded up to a
yes.

State at **0.10.1**.

---

## 1. Summary

The language core is **done and in production**: lexer, parser, AST and a
tree-walking interpreter, all shipping.

| Area | State |
|---|---|
| Language core (lexer → parser → AST → interpreter) | 🟩 complete |
| CLI commands | 10 — see the coverage table for what is missing |
| Standard library | 135 functions across 7 namespaces |
| Domain types | 7 (`Document`, `Page`, `Region`, `Font`, `Image`, `List`, `Str`) + `Diagnostic` |
| Report formats | 4 of 10 (JSON, CSV, HTML, PDF) |
| Distribution | 5 platforms, installers + one portable tarball |
| Documentation | 7 languages, verified in sync with the code |

---

## 2. Coverage

Legend: 🟩 **yes** — implemented and verified · 🟧 **partial** —
some of it, gap noted · 🟥 **no** — absent, verified by grep over `src/`
and `Cargo.toml`.

### `pdfl run`

| Feature | State | Evidence / gap |
|---|---|---|
| Basic invocation, exit by worst severity | 🟩 yes | `src/main.rs`, `src/report.rs` |
| Exit codes 0/1/2 | 🟩 yes | 0 OK, 1 warnings, 2 validation, 3 syntax |
| Infrastructure errors in a separate range (10+) | 🟩 yes | an unreadable document or unwritable file exits 10; findings keep 0–2 |
| `--fail-on warning` | 🟩 yes | `src/main.rs`; checks declare `severity: warning` |
| Custom exit mapping declared in the script | 🟥 no | |
| `--fail-fast` | 🟧 partial | exists in `watch`, not in `run` |
| `--max-findings N` | 🟥 no | |
| stdin input, stdin file list | 🟥 no | |
| Typed script parameters and free variables | 🟧 partial | `--var name=value` reaches the script as `vars.name`; no typed `params` block that declares what a script requires |
| Input by URL | 🟥 no | out of scope until a network namespace exists |
| `--tags` | 🟩 yes | filters checks; a tag no check carries is an error rather than an empty pass |
| `--quiet` / `--verbose` | 🟩 yes | `--quiet` is global and silences progress on stderr, never errors and never `print()`; `run` has `--verbose` |
| Report language selection | 🟥 no | reports are English-only by decision |
| `--dry-run` / execution plan | 🟧 partial | `fix --dry-run` exists; `run` has neither |
| `--profile`, `--explain-skip` | 🟥 no | |
| `--json` on every subcommand | 🟩 yes | `run`/`compare`/`fix`/`watch` via `--output`; `inspect` and `lint` via `--json`; `doc` via `--output json` |
| Baseline runs, run-to-run diff | 🟥 no | unblocked: diagnostic identifiers are now stable |

### Outputs

| Feature | State |
|---|---|
| Self-contained HTML, PDF | 🟩 yes — `src/report.rs`; the PDF is deterministic with embedded Helvetica |
| Canonical JSON/CSV | 🟩 yes — JSON carries `schema_version`, bumped only on a breaking change |
| `schema_version` in CSV | 🟥 no — deliberately: a CSV consumer parses by header, so a format change is already visible there, and a constant column on every row is noise |
| SARIF, JUnit XML | 🟩 yes — `--output sarif\|junit` wherever a report format is chosen: `run`, `compare`, `watch`, `fix` |
| Markdown, NDJSON, XLSX, XML, SQLite artifact | 🟥 no |
| NDJSON progress on stderr | 🟥 no |
| stdout/stderr separation | 🟩 yes — report on stdout, `print()` and progress on stderr |
| Normalized PDF to stdout | 🟥 no — `fix` always writes a file |
| Output packaging, deterministic ZIP, checksum sidecar, filename templates | 🟥 no |
| Input hash recorded in every report | 🟧 partial — `struct::calculate_sha256()` exposes it to scripts; the report does not record it automatically |

### Batch and queues

Essentially **absent**. `watch --once` processes a folder and exits with the
worst code, which covers the simplest case. Everything else — a job type, a
declarative batch block, manifests, priorities, SLAs, dependency graphs, retry,
quarantine, timeouts, incremental hash cache, journal, multi-machine
coordination, queue status, metrics, routing, digests — is unwritten.

`pdfl test` has `--jobs`, and it works by spawning one process per case rather
than by threading — pdfium serialises every call behind a single mutex, so
threads inside one process buy nothing. Batch mode would take the same shape,
but the queue around it is what is missing, not the parallelism.

One design question is worth settling before any of this is built, because it
decides the shape of the rest. A batch that survives a crash needs to remember
what it already did, and this project holds that it keeps no writable state. The
resolution: a journal written *because the user asked for it* — an explicit
`--journal batch.jsonl`, append-only — is an artifact, exactly like the report,
not hidden state the tool maintains behind the user's back. State as an artifact
is in; state as a private database is out.

### Watch mode

| Feature | State |
|---|---|
| Invocation with `--script` | 🟩 yes |
| Multiple folders, each with its own script | 🟥 no |
| Debounce / settle | 🟩 yes — polling with debounce, `src/watch.rs` |
| Sentinel file, manifest trigger, event coalescing, rename detection, double-processing lock | 🟥 no |
| Include/exclude globs, depth limit | 🟩 yes — `--pattern`, `--exclude`, `--depth` |
| Symlink policy, network-share fallback | 🟥 no — though polling is the only mode, so network shares work by accident |
| `--once` | 🟩 yes |
| Parallel batch | 🟩 yes — `--jobs`, one child process per file; the reports are written in the order the files were found |
| `--status`, hot reload, catch-up scan, log rotation, disk guard, status artifact | 🟥 no |
| Service unit generation, watchdog integration, cron overlap lock, jitter, calendar awareness | 🟥 no |

`notify` is not a dependency; the watcher polls. `src/watch.rs:3` records this as
a deliberate choice — portable, no new dependency — with the upgrade path noted.

### Developer tooling

Six of the planned tooling subcommands exist in some form; eleven do not.

| Command | State |
|---|---|
| `fmt` (+ `--check`) | 🟩 yes |
| `lint` | 🟧 partial — 6 rule categories; no config file, no custom rules, no autofix |
| `doc` | 🟧 partial — generates Markdown/HTML from the AST; no docstrings, so descriptions come from assert messages |
| `inspect` | 🟩 yes |
| `pack` | 🟩 yes |
| `add` | 🟩 yes |
| `test` | 🟩 yes — golden-file runner: a folder of PDFs, each with the report expected of it |
| `repl`, `debug`, `bench`, `explain`, `new`, `migrate`, `graph`, `doctor`, `capabilities`, `cache` | 🟥 no |
| Shell completions | 🟩 yes — `pdfl completions <shell>`, via `clap_complete` |
| LSP, editor extension, man pages, corpus runner | 🟥 no |

Shell completions were the cheapest item here and are done; `clap_complete`
generates them from the CLI definition, so they cannot drift from the binary.

### Packages and data

| Feature | State |
|---|---|
| `.pdflpkg` with manifest and SHA-256, reproducible `pdfl pack` | 🟩 yes — determinism has a test |
| Data schema validation at packaging time | 🟥 no |
| Registry, search, publish, signing, dependency resolution, vendoring | 🟥 no |
| `data::` reading CSV, TXT and JSON | 🟩 yes — JSON as an array of arrays or of objects, columns in file order |
| `data::` reading SQLite, XLSX, Parquet, TOML/YAML | 🟥 no |
| Versioned datasets, data dictionary, remote pinning | 🟥 no |

That inconsistency is closed: `pack` and `data::` now agree on the same list of
formats, and a spreadsheet found in the folder is named and left out rather than
packaged into a package that fails at the first lookup.

### CI and integration

All **no**: container image, published CI action, pre-commit hook, an offline
mode that makes network calls fail explicitly, recorded network fixtures, a local
server mode, C ABI, Python/Node bindings, WASM, telemetry.

The repository ships GitHub Actions workflows for its own CI, which is not the
same thing as a published action other people can use.

### Robustness and determinism

| Feature | State |
|---|---|
| Same script + same PDF = same bytes | 🟩 yes — CI asserts it on every push |
| Deterministic parallel output | 🟩 yes — `pdfl test --jobs` judges cases in the order they were found, whichever child finished first; CI compares the parallel and sequential runs |
| Seeded sampling, reproducible builds | 🟥 no — there is no sampling |
| Strict vs lenient parsing, repair diagnostics | 🟥 no — a corrupt PDF fails with one error |
| Partial result on timeout | 🟥 no — there are no timeouts |
| Memory/time limits, recursion limit, path sandbox, large-file guard | 🟧 partial — recursion is bounded and tested; nothing else |
| Memory-mapped reads, page streaming, lazy images | 🟥 no — `visual::` renders on demand and caches, which bounds cost in practice but is not streaming |
| Structured logging, resource report, SBOM, dependency audit | 🟥 no |

### The language itself

| Area | State |
|---|---|
| Core and inspection | 🟩 **yes** — domain types, `check`/`rule`, imports, metadata, SHA-256, normalized text extraction, glossaries, region masks, personal data with valid check digits |
| Diagnostics in more than one language | 🟥 no — output is English-only |
| Comparison | 🟧 partial — text diff with LCS page alignment, pHash, SSIM, pixel diff, Delta-E; **no** typography/table/vector diff, no anchor alignment, no moved-vs-changed semantics, no accept/reject/review triage |
| Preflight | 🟩 **largely yes** — exact TAC from real separations, hairlines, overprint, bleed, spot colors, rich black, output intent, font details |
| PDF/A/X/UA conformance, signature validation, Braille, OCR, spell check | 🟥 no — signatures are detected as present, never validated |
| `fix::` normalization | 🟧 partial — boxes, rotate, delete, duplicate, reorder, watermark, page numbers, split, merge, image downsample/recompress; **no** RGB→CMYK, font embedding, flattening, Bates numbering, redaction, imposition |
| Vertical niches (packaging, regulatory, legal, fiscal) | 🟥 no — beyond the Brazilian pieces already present: CPF/CNPJ, GTIN, postal codes |
| Network namespace | 🟥 no — nothing in the binary touches the network |

---

## 3. Order of work

### Wave 1 — done

Diagnostic identifiers are stable, checks declare their severity, the JSON
report carries a schema version, and infrastructure failures exit outside the
finding range. Baseline runs are no longer blocked.

### Wave 2 — done

`run`, `compare`, `watch` and `fix` write SARIF and JUnit, so a finding lands on
the pull request and in the CI's test panel. The JSON report gained `checks_run`
along the way, because a format that counts tests has to know which checks
passed, and the diagnostics only name the ones that failed.

`pdfl completions <shell>` and a global `--quiet` followed. Completions cost one
dependency, `clap_complete`, the only one this wave added.

Last, `data::` learned to read JSON and `pack` stopped packaging spreadsheets, so
the two halves finally agree on which formats exist.

### Wave 3 — real cost, decided deliberately

Each adds a dependency and a support surface. Waves 1 and 2 are done, and
`pdfl test` is in; this is what is left.

`pdfl test` needed no dependency in the end — it reuses the interpreter and the
report, and compares the JSON it already knows how to produce.

Parallelism needed none either, but not for the expected reason. Threads inside
one process do not help: pdfium serialises every call behind a single mutex, and
a threaded run of eight 41-page files measured *slower* than sequential (12.2s
against 8.3s). Separate processes finished the same work in 1.2s. So `--jobs`
spawns children rather than threads, on `pdfl test` and on `pdfl watch` alike,
and `rayon` never came up.

`watch` was restructured to match: a child analyses each file and this process
renders every format from the JSON that comes back. One code path for all six
formats and every value of `--jobs` — CI checks that a report rendered from a
child is byte-identical to one rendered in place.

12. **Event-based watch**, keeping polling as the fallback for network shares.
13. **Batch as runtime semantics** — the largest single block of work. Worth
    starting only once 10–12 exist.

---

## 4. Out of scope for this project

These are deliberate exclusions, not oversights. A fork is free to decide
otherwise, and the reasoning is recorded here so that decision can be an informed
one.

**Curated regulatory datasets.** Allergen tables, tobacco rules, medical-device
and health-agency lists. The cost is continuous curation, not code. Such lists go
stale silently and then produce confident wrong answers — the worst failure mode
for a tool whose stated principle is that a false alarm is a bug. The
*mechanism* ships: `data::` already reads a user-supplied dataset. The data
belongs to whoever is accountable for it being current.

**A package registry.** Publishing, search, signing, dependency resolution and a
private registry add up to a package manager, and one that competes with `git
clone`. `.pdflpkg` plus `pdfl add` from a local file already covers
distribution. Worth revisiting only if profiles are actually being exchanged.

**A local server mode and a WASM build.** Both are new runtime surfaces with
their own security posture, for a tool that is invoked from a shell and from CI.
A stable C ABI and a Python binding are more defensible than either, and even
those should wait for a caller that wants them.

**Multi-machine coordination.** Nothing in the code has any concept of shared
state, and a claim protocol over a shared filesystem is a distributed system with
all of the failure modes and none of the tooling. If coordination is ever needed,
consuming a queue the user already runs is the smaller and more honest design.

**Report translation.** Output settled on English, documented in seven languages.
Translating every diagnostic means keeping those translations in sync with the
code — the exact maintenance trap that let the documentation drift three releases
behind before it was caught. Language coverage belongs in the documentation, not
in the diagnostics.

**Redaction that claims to remove information.** `fix::` may reorganize and
re-encode a PDF. It will not claim to have removed content, because a redaction
that silently fails leaks precisely what it was asked to hide. Reorganizing is
recoverable; a false guarantee is not.

---

## 5. Re-verifying this document

Each claim maps to a command:

```bash
# commands the binary actually has
./target/debug/pdfl --help

# functions per namespace
grep -cE '^\s+"[a-z0-9_]+"( \| "[a-z0-9_]+")* =>' src/textns.rs   # and the others

# the absences
grep -rin "baseline\|rayon\|serve\|repl" src/ Cargo.toml

```

Beware that a plain grep for `ocr`, `notify` or `xlsx` returns hits which are
comments saying the feature is *absent*, or an extension whitelist rather than a
reader. Every hit recorded here was opened and read first.
