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

State at **0.18.0**.

---

## 1. Summary

The language core is **done and in production**: lexer, parser, AST and a
tree-walking interpreter, all shipping.

| Area | State |
|---|---|
| Language core (lexer → parser → AST → interpreter) | 🟩 complete |
| CLI commands | 13 — see the coverage table for what is missing |
| Standard library | 135 functions across 7 namespaces |
| Domain types | 7 (`Document`, `Page`, `Region`, `Font`, `Image`, `List`, `Str`) + `Diagnostic` |
| Report formats | 6 of 10 (JSON, CSV, HTML, PDF, SARIF, JUnit) |
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
| Typed script parameters and free variables | 🟧 partial | `--var name=value` reaches the script as `vars.name`, on `run`, `test` and `watch` alike; no typed `params` block that declares what a script requires |
| Input by URL | 🟥 no | out of scope until a network namespace exists |
| `--tags` | 🟩 yes | filters checks; a tag no check carries is an error rather than an empty pass |
| `--quiet` / `--verbose` | 🟩 yes | `--quiet` is global and silences progress on stderr, never errors and never `print()`; `run` has `--verbose` |
| Report language selection | 🟥 no | reports are English-only by decision |
| `--dry-run` / execution plan | 🟧 partial | `fix --dry-run` exists; `run` has neither |
| `--profile`, `--explain-skip` | 🟥 no | |
| `--json` on every validating command | 🟩 yes | `run`/`compare`/`fix`/`watch` via `--output`; `inspect` and `lint` via `--json`; `doc` via `--output json`. `pack`, `add`, `fmt`, `test` and `completions` print human-readable text — they confirm an action or report pass/fail counts, not a document verdict, so JSON was never added there |
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

`watch --once` processes a folder, `--jobs` decides how many at a time,
`--journal` makes an interrupted batch resumable, and `--timeout` kills a file
that runs too long instead of letting it hang the rest. That covers the case a
print shop actually has: a folder, a script, and a run that has to survive both
a crash and one adversarial file.

Unwritten, and mostly on purpose: a job type, a declarative batch block,
manifests, priorities, SLAs, dependency graphs, retry, quarantine,
multi-machine coordination, queue status, metrics, routing, digests. Those
describe a queue product; this is a validator with a folder mode.

A timed-out file is reported exactly like an unreadable one — one finding,
`check_name: "timeout"` — so it flows through `write_report` and the journal
with no special case: same exit-code path, same "a resumed batch must still see
the rejection" guarantee. There is nothing in `.pdfl` a script can use to hang
the interpreter on purpose (recursion is depth-limited), so the flag exists for
what a script cannot cause — pdfium looping or blocking on a malformed or
adversarial PDF.

`pdfl test` has `--jobs`, and it works by spawning one process per case rather
than by threading — pdfium serialises every call behind a single mutex, so
threads inside one process buy nothing. Batch mode would take the same shape,
but the queue around it is what is missing, not the parallelism.

The design question that decided the shape of this — a batch that survives a
crash has to remember what it did, and this project keeps no writable state —
was settled the way it was framed: a journal written *because the user asked for
it*, an explicit `--journal batch.jsonl`, append-only, is an artifact exactly
like the report. State as an artifact is in; state as a private database is out.
Nothing is written without the flag.

Two things fell out of building it. A file is matched by its bytes rather than
its name or its timestamp, so a replaced file is validated again. And a skipped
file still contributes its recorded verdict to the exit code — a resumed batch
that stayed quiet about a rejection it had already seen would be the worst bug
this tool could have.

### Watch mode

| Feature | State |
|---|---|
| Invocation with `--script` | 🟩 yes |
| Multiple folders, each with its own script | 🟥 no |
| Debounce / settle | 🟩 yes — polling with debounce, `src/watch.rs` |
| Filesystem notifications | 🟩 yes — `--events`, opt-in; polling stays the default because inotify does not see a network share |
| Sentinel file, manifest trigger, rename detection, double-processing lock | 🟥 no — though a burst of events coalesces into one pass |
| Include/exclude globs, depth limit | 🟩 yes — `--pattern`, `--exclude`, `--depth` |
| Symlink policy, network-share fallback | 🟥 no — though polling is the only mode, so network shares work by accident |
| `--once` | 🟩 yes |
| Parallel batch | 🟩 yes — `--jobs`, one child process per file; the reports are written in the order the files were found |
| Resumable batch | 🟩 yes — `--journal`, append-only, matched by content hash; a skipped file still counts its recorded verdict |
| Per-file timeout | 🟩 yes — `--timeout`, kills the child and reports the file as rejected rather than stalling the batch |
| `--status`, hot reload, catch-up scan, log rotation, disk guard, status artifact | 🟥 no |
| Service unit generation, watchdog integration, cron overlap lock, jitter, calendar awareness | 🟥 no |

`notify` is a dependency, and `--events` opts into it. Polling stays the default,
which the measurement argues for: a folder of 10,000 files listed every 200ms
costs no measurable CPU, and the settle time dominates the latency so completely
that the two modes finish within a hundredth of a second of each other on a
local folder. Where notifications would win — a hot folder fed over the network
— is where they are broken, since inotify on an NFS or SMB mount reports what
the local machine writes and nothing else. A watcher that goes quiet without
saying so is the failure this project exists to avoid, so that behaviour is not
something a default should be able to reach. A watcher that cannot be created
says so and falls back.

The measurement also found the latency that was really there: the loop waited a
whole interval before looking again. It now sleeps only until the freshest file
has settled.

### Developer tooling

Eight of the planned tooling subcommands exist in some form; ten do not.

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

That CI runs the unit tests on Linux, macOS and Windows — the three platforms
the releases ship — and the end-to-end smoke on Linux only, since that part
needs pdfium and the fixtures. Until this was added, nothing exercised macOS or
Windows before a tag was pushed, so a break on either surfaced as a failed
release rather than a failed push.

### Robustness and determinism

| Feature | State |
|---|---|
| Same script + same PDF = same bytes | 🟩 yes — CI asserts it on every push |
| Deterministic parallel output | 🟩 yes — `pdfl test --jobs` and `pdfl watch --jobs` judge files in the order they were found, whichever child finished first, and `pixelcompare --jobs` folds pages back in page order so even the fingerprints are unchanged; CI compares a parallel run against a sequential one for each |
| Seeded sampling, reproducible builds | 🟥 no — there is no sampling |
| Strict vs lenient parsing, repair diagnostics | 🟥 no — a corrupt PDF fails with one error |
| Partial result on timeout | 🟧 partial — `watch --timeout` kills a hung file and substitutes a "timeout" finding for it; `pdfl run` alone has no timeout, and there is no version that keeps whatever diagnostics ran before the hang |
| Memory/time limits, recursion limit, path sandbox, large-file guard | 🟧 partial — recursion is bounded and tested; `watch --timeout` bounds one file's analysis inside a batch by killing the child process; a lone `pdfl run` still has no time limit of its own, and there is no memory limit or path sandbox anywhere |
| Memory-mapped reads, page streaming, lazy images | 🟥 no — `visual::` renders on demand and caches, which bounds cost in practice but is not streaming |
| Structured logging, resource report, SBOM, dependency audit | 🟥 no |

### The language itself

| Area | State |
|---|---|
| Core and inspection | 🟩 **yes** — domain types, `check`/`rule`, imports, metadata, SHA-256, normalized text extraction, glossaries, region masks, personal data with valid check digits |
| Conditionals | 🟩 yes — `if`/`else if`/`else` as an **expression**, so a function can finally yield one value or another instead of only composing booleans |
| Diagnostics in more than one language | 🟥 no — output is English-only |
| Comparison | 🟧 partial — text diff with LCS page alignment, pHash, SSIM, pixel diff, Delta-E; `pixelcompare` adds a rendered per-page diff with global-shift alignment, added/removed/recoloured classification and a three-pane viewer — original, new and the difference, sharing one crosshair, a synchronised zoom and pan, and opening on the pages that differ; **no** typography/table/vector diff, no anchor alignment, no per-element moved-vs-changed semantics, no accept/reject/review triage |
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

### Wave 3 — done

Five items — `pdfl test`, `--jobs`, `--events`, `--journal`, `--timeout` — each
expected to cost a dependency and a support surface. Only one of the five
actually did.

`pdfl test` needed no dependency at all — it reuses the interpreter and the
report, and compares the JSON it already knows how to produce.

Parallelism needed none either, but not for the expected reason. Threads inside
one process do not help *for work that goes through pdfium*: it serialises every
call behind a single mutex, and a threaded run of eight 41-page files measured
*slower* than sequential (12.2s against 8.3s). Separate processes finished the
same work in 1.2s. So `--jobs` spawns children rather than threads, on
`pdfl test` and on `pdfl watch` alike, and `rayon` never came up.

The qualifier turned out to matter. `pixelcompare` was measured stage by stage
and the shape is inverted: rasterising is a fifth of the run, the pixel
comparison four fifths, and the comparison never enters pdfium — it is our own
arithmetic over buffers already in memory. There threads are exactly right, and
`pixelcompare --jobs` uses them (3.6s to 1.2s on 41 pages), with `std::thread`
rather than a dependency. The rule is not "threads never help"; it is "pdfium
cannot be threaded", which leaves the stage in front of it free.

`watch` was restructured to match: a child analyses each file and this process
renders every format from the JSON that comes back. One code path for all six
formats and every value of `--jobs` — CI checks that a report rendered from a
child is byte-identical to one rendered in place.

`--events` is the one real dependency the wave added, `notify`, and it is
opt-in rather than the default: the measurement did not support switching, and
inotify goes silent on a network share instead of failing loud. `--journal`
answered the design question the roadmap flagged as deciding batch mode's
shape — a resumable run needs to remember what it did, and a journal the user
asked for by name is an artifact, not state the tool keeps on its own.
`--timeout` closed the last hole: one adversarial PDF hanging pdfium no longer
hangs the batch around it, since the child is killed and the file gets a
`"timeout"` finding like any other rejection.

What Wave 3 did not build is the batch-as-a-product direction — a job type,
priorities, SLAs, retry, quarantine, multi-machine coordination. See "Batch and
queues" above: that is not a deferred item waiting on a prerequisite, it is a
deliberate boundary. This project is a validator with a folder mode, not a queue.

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

Beware that a plain grep for `ocr` or `xlsx` returns hits which are comments
saying the feature is *absent*, or an extension whitelist rather than a reader
(`src/pack.rs`'s `UNREADABLE` list names `xlsx` precisely to refuse it). `notify`
used to belong on this warning list too: this document itself said, in full
sentences, "`notify` is not a dependency; the watcher polls" — a true statement
at the time, and exactly the kind of confident-sounding hit that goes stale the
moment a feature ships. `--events` shipped, and the sentence became wrong
without anyone editing it, until this pass caught it. It is a real dependency
now (`Cargo.toml`, `src/watch.rs`). Every hit recorded here was opened and read
first — including, this time, the ones inside this file.
