//! The `pdfl` CLI — interpreter for the PDFLang language (.pdfl).

mod ast;
mod codesns;
mod colors;
mod compare;
mod datans;
mod doccmd;
mod fixns;
mod inspect;
mod fmt;
mod lint;
mod pack;
mod interpreter;
mod lexer;
mod parser;
mod pdf;
mod pixelcmp;
mod progress;
mod prepressns;
mod report;
mod structns;
mod testcmd;
mod textns;
mod viewer;
mod visualns;
mod watch;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use report::{Diagnostic, Report, Severity};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};

/// Something went wrong that is not a verdict on the document: the PDF could not
/// be read, a file could not be written, an operation failed.
///
/// Kept out of the 0–2 range on purpose. A corrupt input and a rejected input
/// both used to exit 2, so CI could not tell "this file is broken" from "this
/// file failed the checks" — opposite situations needing opposite reactions.
const EXIT_INFRASTRUCTURE: u8 = 10;

/// Set once from --quiet, then only read. Informational stderr passes through
/// `note()`; errors never do, so a quiet run still says why it failed.
static QUIET: AtomicBool = AtomicBool::new(false);

/// Progress and confirmations on stderr — the lines a person wants and a
/// pipeline does not.
pub fn note(message: String) {
    if !QUIET.load(Ordering::Relaxed) {
        eprintln!("{message}");
    }
}

/// Whether `--quiet` was given. The progress bar asks, because a bar is
/// exactly the kind of stderr chatter that flag exists to remove.
pub fn quiet() -> bool {
    QUIET.load(Ordering::Relaxed)
}

#[derive(Parser)]
#[command(name = "pdfl", version, about = "Runs .pdfl scripts to validate and normalize PDFs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Silences progress and confirmations on stderr; errors still appear. Wins
    /// over --verbose
    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Runs a .pdfl script against a PDF
    Run {
        /// The .pdfl script
        script: PathBuf,
        /// Input PDF
        input: PathBuf,
        /// Output format
        #[arg(long, default_value = "json")]
        output: OutputFormat,
        /// Writes the report to a file (required in practice for pdf)
        #[arg(long)]
        output_file: Option<PathBuf>,
        /// Severity that forces exit code 2
        #[arg(long, default_value = "error")]
        fail_on: FailOn,
        /// Verbose output on stderr
        #[arg(long)]
        verbose: bool,
        /// Value the script reads as `vars.<name>`; repeatable
        #[arg(long = "var", value_name = "NAME=VALUE")]
        vars: Vec<String>,
        /// Run only checks carrying this tag; repeatable
        #[arg(long = "tags", value_name = "TAG")]
        tags: Vec<String>,
    },
    /// Applies fix:: operations from a script and saves a new PDF
    Fix {
        /// Input PDF
        input: PathBuf,
        /// The .pdfl script with fix:: calls
        script: PathBuf,
        /// Output PDF
        #[arg(long)]
        output: PathBuf,
        /// Only lists the operations, without saving
        #[arg(long)]
        dry_run: bool,
        /// Report format
        #[arg(long = "report", default_value = "json")]
        report_format: OutputFormat,
        /// Writes the report to a file
        #[arg(long)]
        report_file: Option<PathBuf>,
    },
    /// Compares two PDFs (text, structure and metadata)
    Compare {
        /// Original version
        v1: PathBuf,
        /// New version
        v2: PathBuf,
        /// Output format
        #[arg(long, default_value = "json")]
        output: OutputFormat,
        /// Writes the report to a file
        #[arg(long)]
        output_file: Option<PathBuf>,
        /// Normalizes text before comparing (lowercase, spacing)
        #[arg(long)]
        normalize: bool,
        /// Ignores dates (dd/mm/yyyy, yyyy-mm-dd, "1 de março de 2026")
        #[arg(long)]
        ignore_dates: bool,
        /// Minimum similarity (0-100) to pass
        #[arg(long, default_value_t = 100.0)]
        similarity_threshold: f64,
    },
    /// Compares two PDFs pixel by pixel, page by page
    Pixelcompare {
        /// Original version
        v1: PathBuf,
        /// New version
        v2: PathBuf,
        /// Output format
        #[arg(long, default_value = "json")]
        output: OutputFormat,
        /// Writes the report to a file
        #[arg(long)]
        output_file: Option<PathBuf>,
        /// Folder for a self-contained viewer: the rendered pages, the
        /// difference overlays and an index.html to wipe and flip between them
        #[arg(long)]
        viewer: Option<PathBuf>,
        /// Render resolution. Higher sees more and costs more
        #[arg(long, default_value_t = 150)]
        dpi: u16,
        /// Per-pixel colour distance (0.0-1.0) above which two pixels differ
        #[arg(long, default_value_t = 0.05)]
        threshold: f64,
        /// Percentage of changed pixels a page may have before it is reported
        /// as an error
        #[arg(long, default_value_t = 0.0)]
        max_diff: f64,
        /// Pages to compare, e.g. 1-10 or 1,3,7-12 (default: all)
        #[arg(long)]
        pages: Option<String>,
        /// Do not compensate a global shift between the pages
        #[arg(long)]
        no_align: bool,
        /// Box-blur radius applied before comparing, to absorb anti-aliasing
        #[arg(long, default_value_t = 0)]
        blur: u32,
    },
    /// Watches a folder and validates each new or changed PDF
    Watch {
        /// Folder to watch
        folder: PathBuf,
        /// The validation .pdfl script
        #[arg(long)]
        script: PathBuf,
        /// File pattern (wildcard *)
        #[arg(long, default_value = "*.pdf")]
        pattern: String,
        /// Pattern to exclude
        #[arg(long)]
        exclude: Option<String>,
        /// Folder for the reports (default: next to the PDF)
        #[arg(long)]
        output_dir: Option<PathBuf>,
        /// Recursion depth (1 = the folder only)
        #[arg(long, default_value_t = 1)]
        depth: usize,
        /// Wait (ms) for the file to settle before processing
        #[arg(long, default_value_t = 1000)]
        debounce: u64,
        /// Stops at the first file with an error
        #[arg(long)]
        fail_fast: bool,
        /// Processes the files present and exits (batch/CI)
        #[arg(long)]
        once: bool,
        /// Format of the written reports
        #[arg(long = "report", default_value = "json")]
        report_format: OutputFormat,
        /// Files to validate at the same time (0 = one per CPU)
        #[arg(long, default_value_t = 1)]
        jobs: usize,
        /// Wake on filesystem events instead of listing the folder on a timer.
        /// Not for network shares: inotify does not see what another machine
        /// writes to an NFS or SMB mount
        #[arg(long)]
        events: bool,
        /// Append-only record of what has been validated. Re-running with the
        /// same journal skips the files it already covers
        #[arg(long)]
        journal: Option<PathBuf>,
        /// Kills a file's analysis past this many seconds and reports it as
        /// rejected instead of leaving the batch stuck. Unset waits forever
        #[arg(long)]
        timeout: Option<u64>,
        /// Value every file reads as `vars.<name>`; repeatable
        #[arg(long = "var", value_name = "NAME=VALUE")]
        vars: Vec<String>,
    },
    /// Shows a quick summary of a PDF
    Inspect {
        /// Input PDF
        input: PathBuf,
        /// Emit the summary as JSON instead of text
        #[arg(long)]
        json: bool,
    },
    /// Generates documentation for a .pdfl script
    Doc {
        /// The .pdfl script
        script: PathBuf,
        /// Documentation format
        #[arg(long, default_value = "markdown")]
        output: DocFormat,
    },
    /// Packages scripts and datasets into a .pdflpkg (with manifest and hashes)
    Pack {
        /// Folder with .pdfl scripts and datasets (.csv, .txt, .json, .xlsx)
        folder: PathBuf,
        /// Output file (default: <folder>.pdflpkg)
        #[arg(long)]
        output: Option<PathBuf>,
        /// Package name (default: folder name)
        #[arg(long)]
        name: Option<String>,
        /// Package version
        #[arg(long, default_value = "0.1.0")]
        version: String,
    },
    /// Installs a local .pdflpkg package (verifies the manifest hashes)
    Add {
        /// The .pdflpkg file
        package: PathBuf,
        /// Installation folder
        #[arg(long, default_value = "pdfl_profiles")]
        dir: PathBuf,
    },
    /// Analyzes a .pdfl script without running it (quality warnings)
    Lint {
        /// The .pdfl script
        script: PathBuf,
        /// Emit the warnings as JSON instead of text
        #[arg(long)]
        json: bool,
    },
    /// Formats a .pdfl script (2 spaces, consistent spacing)
    Fmt {
        /// The .pdfl script
        script: PathBuf,
        /// Does not write: fails (exit 1) if the file is not formatted
        #[arg(long)]
        check: bool,
    },
    /// Runs a script against a folder of PDFs and compares each report to the
    /// one recorded beside it
    Test {
        /// The .pdfl script under test
        script: PathBuf,
        /// Folder with the case PDFs (default: tests/ next to the script)
        #[arg(long)]
        dir: Option<PathBuf>,
        /// Records the expected reports instead of comparing them
        #[arg(long)]
        update: bool,
        /// Cases to run at the same time (0 = one per CPU)
        #[arg(long, default_value_t = 1)]
        jobs: usize,
        /// Value every case reads as `vars.<name>`; repeatable
        #[arg(long = "var", value_name = "NAME=VALUE")]
        vars: Vec<String>,
    },
    /// Prints a completion script for a shell (bash, zsh, fish, elvish, powershell)
    Completions {
        /// The shell to generate for
        shell: clap_complete::Shell,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Csv,
    Html,
    Pdf,
    /// SARIF 2.1.0, for GitHub code scanning.
    Sarif,
    /// JUnit XML, for the test panel of any CI.
    Junit,
}

/// Writes the report: stdout for text formats (or --output-file); PDF always
/// goes to a file (--output-file or <input>.report.pdf).
fn emit_report(report: &Report, format: OutputFormat, file: Option<&PathBuf>, input: &Path) {
    let bytes: Vec<u8> = match format {
        OutputFormat::Json => report.to_json().into_bytes(),
        OutputFormat::Csv => report.to_csv().into_bytes(),
        OutputFormat::Html => report.to_html().into_bytes(),
        OutputFormat::Pdf => report.to_pdf(),
        OutputFormat::Sarif => report.to_sarif().into_bytes(),
        OutputFormat::Junit => report.to_junit().into_bytes(),
    };
    let default_pdf = || {
        let stem = input.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        PathBuf::from(format!("{stem}.report.pdf"))
    };
    let target = match (format, file) {
        (OutputFormat::Pdf, f) => Some(f.cloned().unwrap_or_else(default_pdf)),
        (_, Some(f)) => Some(f.clone()),
        (_, None) => None,
    };
    match target {
        Some(path) => match std::fs::write(&path, bytes) {
            Ok(()) => note(format!("report saved to {}", path.display())),
            Err(e) => eprintln!("error writing {}: {e}", path.display()),
        },
        None => println!("{}", String::from_utf8_lossy(&bytes)),
    }
}

#[derive(Clone, Copy, ValueEnum)]
enum FailOn {
    Error,
    Warning,
}

#[derive(Clone, Copy, ValueEnum)]
enum DocFormat {
    Markdown,
    Html,
    Json,
}

/// `--var NAME=VALUE`, split on the first `=` so a value may contain more.
fn parse_vars(raw: &[String]) -> Result<std::collections::HashMap<String, String>, String> {
    raw.iter()
        .map(|s| match s.split_once('=') {
            Some((k, v)) if !k.is_empty() => Ok((k.to_string(), v.to_string())),
            _ => Err(s.clone()),
        })
        .collect()
}

/// Loads and parses the script; an error means exit 3.
fn load_program(script: &Path) -> Result<Vec<ast::Stmt>, ExitCode> {
    let source = std::fs::read_to_string(script).map_err(|e| {
        eprintln!("error: could not read script {}: {e}", script.display());
        ExitCode::from(3)
    })?;
    parser::parse(&source).map_err(|e| {
        eprintln!("{}: {e}", script.display());
        ExitCode::from(3)
    })
}

/// Loads the PDF; a failure still prints a report, but exits in the
/// infrastructure range — the document was never judged.
fn load_doc(
    input: &Path,
    script_name: &str,
) -> Result<std::rc::Rc<interpreter::DocData>, ExitCode> {
    pdf::load_document(input).map_err(|e| {
        let diag = Diagnostic {
            id: report::fingerprint("loading", &format!("{e:#}"), 1),
            severity: Severity::Error,
            check_name: "loading".into(),
            message: format!("{e:#}"),
            line: None,
        };
        let report =
            Report::new(script_name.into(), input.to_string_lossy().into_owned(), None, 0, vec![diag]);
        println!("{}", report.to_json());
        ExitCode::from(EXIT_INFRASTRUCTURE)
    })
}

/// `--jobs 0` means "one per CPU". Anything else is taken as written: a person
/// who says 2 on a 32-core machine is asking for 2, usually because something
/// else is running.
fn resolve_jobs(requested: usize) -> usize {
    if requested > 0 {
        return requested;
    }
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    QUIET.store(cli.quiet, Ordering::Relaxed);
    match cli.command {
        Command::Run { script, input, output, output_file, fail_on, verbose, vars, tags } => {
            run_cmd(&script, &input, output, output_file.as_ref(), fail_on, verbose, &vars, &tags)
        }
        Command::Fix { input, script, output, dry_run, report_format, report_file } => {
            fix_cmd(&input, &script, &output, dry_run, report_format, report_file.as_ref())
        }
        Command::Compare { v1, v2, output, output_file, normalize, ignore_dates, similarity_threshold } => {
            compare_cmd(&v1, &v2, output, output_file.as_ref(), compare::CompareOptions {
                normalize,
                ignore_dates,
                similarity_threshold,
            })
        }
        Command::Pixelcompare {
            v1,
            v2,
            output,
            output_file,
            viewer,
            dpi,
            threshold,
            max_diff,
            pages,
            no_align,
            blur,
        } => {
            let pages = match pages.as_deref().map(parse_page_range) {
                Some(Ok(p)) => p,
                Some(Err(bad)) => {
                    eprintln!("error: --pages expects something like 1-10 or 1,3,7-12, got '{bad}'");
                    return ExitCode::from(EXIT_INFRASTRUCTURE);
                }
                None => Vec::new(),
            };
            pixelcompare_cmd(
                &v1,
                &v2,
                output,
                output_file.as_ref(),
                viewer.as_deref(),
                dpi,
                &pages,
                max_diff,
                pixelcmp::Options {
                    threshold,
                    align: !no_align,
                    blur,
                    ..Default::default()
                },
            )
        }
        Command::Watch {
            folder,
            script,
            pattern,
            exclude,
            output_dir,
            depth,
            debounce,
            fail_fast,
            once,
            report_format,
            jobs,
            events,
            journal,
            timeout,
            vars,
        } => {
            // Parsed here only to fail fast: a syntax error, or a malformed
            // --var, is one message, not one per file in the folder.
            if let Err(code) = load_program(&script) {
                return code;
            }
            if let Err(bad) = parse_vars(&vars) {
                eprintln!("error: --var expects NAME=VALUE, got '{bad}'");
                return ExitCode::from(EXIT_INFRASTRUCTURE);
            }
            let opts = watch::WatchOptions {
                script,
                jobs: resolve_jobs(jobs),
                pattern,
                exclude,
                output_dir,
                depth,
                debounce_ms: debounce,
                fail_fast,
                timeout: timeout.map(std::time::Duration::from_secs),
                vars,
                journal,
                events,
                once,
                format: report_format,
            };
            ExitCode::from(watch::watch(&folder, &opts))
        }
        Command::Inspect { input, json } => {
            match pdf::load_document(&input) {
                Ok(doc) => {
                    let summary = inspect::summarize(&doc);
                    if json {
                        println!("{}", inspect::to_json(&summary));
                    } else {
                        print!("{}", inspect::to_text(&summary));
                    }
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    ExitCode::from(EXIT_INFRASTRUCTURE)
                }
            }
        }
        Command::Doc { script, output } => {
            let program = match load_program(&script) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let name = file_name(&script);
            match output {
                DocFormat::Markdown => print!("{}", doccmd::markdown(&name, &program)),
                DocFormat::Html => print!("{}", doccmd::html(&name, &program)),
                DocFormat::Json => println!("{}", doccmd::json(&name, &program)),
            }
            ExitCode::SUCCESS
        }
        Command::Pack { folder, output, name, version } => {
            let name = name.unwrap_or_else(|| file_name(&folder));
            let output = output.unwrap_or_else(|| PathBuf::from(format!("{name}.pdflpkg")));
            match pack::pack(&folder, &output, &name, &version) {
                Ok(manifest) => {
                    eprintln!(
                        "package {}@{} created at {} ({} file(s))",
                        manifest.name,
                        manifest.version,
                        output.display(),
                        manifest.files.len()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    ExitCode::from(EXIT_INFRASTRUCTURE)
                }
            }
        }
        Command::Add { package, dir } => match pack::add(&package, &dir) {
            Ok((manifest, target)) => {
                eprintln!(
                    "package {}@{} installed at {} (hashes verified)",
                    manifest.name,
                    manifest.version,
                    target.display()
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e:#}");
                ExitCode::from(EXIT_INFRASTRUCTURE)
            }
        },
        Command::Lint { script, json } => {
            let program = match load_program(&script) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let warnings = lint::lint(&program);
            if json {
                // The script is named alongside the warnings so a caller
                // linting a folder can merge the outputs without losing which
                // file each came from.
                let report = serde_json::json!({
                    "script": script.display().to_string(),
                    "warnings": warnings,
                });
                println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
            } else {
                for w in &warnings {
                    println!("{}: warning: {w}", script.display());
                }
                if warnings.is_empty() {
                    note(format!("{}: no problems found", script.display()));
                }
            }
            if warnings.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Command::Fmt { script, check } => {
            let source = match std::fs::read_to_string(&script) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: could not read {}: {e}", script.display());
                    return ExitCode::from(3);
                }
            };
            let formatted = match fmt::format_source(&source) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("{}: {e}", script.display());
                    return ExitCode::from(3);
                }
            };
            if check {
                if formatted == source {
                    ExitCode::SUCCESS
                } else {
                    eprintln!("{}: not formatted (run pdfl fmt)", script.display());
                    ExitCode::from(1)
                }
            } else if formatted == source {
                note(format!("{}: already formatted", script.display()));
                ExitCode::SUCCESS
            } else if let Err(e) = std::fs::write(&script, &formatted) {
                eprintln!("error writing {}: {e}", script.display());
                ExitCode::from(EXIT_INFRASTRUCTURE)
            } else {
                note(format!("{}: formatted", script.display()));
                ExitCode::SUCCESS
            }
        }
        Command::Test { script, dir, update, jobs, vars } => {
            // Parsed here only to fail fast: a syntax error, or a malformed
            // --var, is one message, not one per case.
            if let Err(code) = load_program(&script) {
                return code;
            }
            if let Err(bad) = parse_vars(&vars) {
                eprintln!("error: --var expects NAME=VALUE, got '{bad}'");
                return ExitCode::from(EXIT_INFRASTRUCTURE);
            }
            let dir = dir.unwrap_or_else(|| {
                script.parent().unwrap_or(Path::new(".")).join("tests")
            });
            ExitCode::from(testcmd::test(&script, &dir, update, resolve_jobs(jobs), &vars))
        }
        Command::Completions { shell } => {
            // stdout, so it can be piped straight into the shell's completion
            // directory — which is why nothing else is printed here.
            clap_complete::generate(shell, &mut Cli::command(), "pdfl", &mut std::io::stdout());
            ExitCode::SUCCESS
        }
    }
}

fn run_cmd(
    script: &Path,
    input: &Path,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
    fail_on: FailOn,
    verbose: bool,
    vars: &[String],
    tags: &[String],
) -> ExitCode {
    let vars = match parse_vars(vars) {
        Ok(v) => v,
        Err(bad) => {
            eprintln!("error: --var expects NAME=VALUE, got '{bad}'");
            return ExitCode::from(EXIT_INFRASTRUCTURE);
        }
    };
    let program = match load_program(script) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let script_name = file_name(script);
    let doc = match load_doc(input, &script_name) {
        Ok(d) => d,
        Err(code) => return code,
    };
    if verbose {
        note(format!("PDF loaded: {} page(s), {} font(s)", doc.pages.len(), doc.fonts.len()));
    }

    let mut interp = interpreter::Interpreter::new();
    interp.vars = vars;
    if !tags.is_empty() {
        interp.tag_filter = Some(tags.to_vec());
    }
    if let Some(dir) = script.parent() {
        interp.script_dir = dir.to_path_buf();
    }
    let total_pages = doc.pages.len() as i64;
    if let Err(e) = interp.run(&program, doc) {
        eprintln!("{}: {e}", script.display());
        return ExitCode::from(3);
    }

    // A filter that matches nothing would otherwise run no checks and report a
    // pass, so a misspelled tag in a pipeline would look like a clean file.
    if !tags.is_empty() && interp.checks_run.is_empty() {
        eprintln!("error: no check carries any of these tags: {}", tags.join(", "));
        eprintln!("nothing was validated — check the spelling, or drop --tags");
        return ExitCode::from(EXIT_INFRASTRUCTURE);
    }

    let mut report = Report::new(
        script_name,
        input.to_string_lossy().into_owned(),
        interp.profile_name.clone(),
        total_pages,
        interp.diagnostics,
    );
    report.checks_run = interp.checks_run;
    emit_report(&report, format, output_file, input);
    ExitCode::from(report.exit_code(matches!(fail_on, FailOn::Warning)) as u8)
}

fn fix_cmd(
    input: &Path,
    script: &Path,
    output: &Path,
    dry_run: bool,
    format: OutputFormat,
    report_file: Option<&PathBuf>,
) -> ExitCode {
    let program = match load_program(script) {
        Ok(p) => p,
        Err(code) => return code,
    };
    let script_name = file_name(script);
    let doc = match load_doc(input, &script_name) {
        Ok(d) => d,
        Err(code) => return code,
    };
    let total_pages = doc.pages.len() as i64;

    let mut interp = interpreter::Interpreter::new();
    interp.allow_fixes = true;
    if let Some(dir) = script.parent() {
        interp.script_dir = dir.to_path_buf();
    }
    if let Err(e) = interp.run(&program, doc) {
        eprintln!("{}: {e}", script.display());
        return ExitCode::from(3);
    }

    let fixes: Vec<String> = if dry_run {
        interp.fix_ops.iter().map(|op| format!("[dry-run] {op}")).collect()
    } else if interp.fix_ops.is_empty() {
        Vec::new()
    } else {
        match pdf::apply_fixes(input, &interp.fix_ops, output) {
            Ok(applied) => applied,
            Err(e) => {
                eprintln!("error applying fixes: {e:#}");
                return ExitCode::from(EXIT_INFRASTRUCTURE);
            }
        }
    };
    if !dry_run && !fixes.is_empty() {
        note(format!("normalized PDF saved to {}", output.display()));
    }

    let mut report = Report::new(
        script_name,
        input.to_string_lossy().into_owned(),
        interp.profile_name.clone(),
        total_pages,
        interp.diagnostics,
    );
    report.fixes = fixes;
    emit_report(&report, format, report_file, input);
    ExitCode::from(report.exit_code(false) as u8)
}

fn compare_cmd(
    v1: &Path,
    v2: &Path,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
    opts: compare::CompareOptions,
) -> ExitCode {
    let doc1 = match load_doc(v1, "compare") {
        Ok(d) => d,
        Err(code) => return code,
    };
    let doc2 = match load_doc(v2, "compare") {
        Ok(d) => d,
        Err(code) => return code,
    };

    let result = compare::compare_documents(&doc1, &doc2, &opts);
    let total_pages = doc1.pages.len().max(doc2.pages.len()) as i64;
    let mut report = Report::new(
        "compare".into(),
        format!("{} → {}", v1.display(), v2.display()),
        None,
        total_pages,
        result.diagnostics,
    );
    report.similarity = Some(result.similarity);
    emit_report(&report, format, output_file, v1);
    ExitCode::from(report.exit_code(false) as u8)
}

/// `1-10` or `1,3,7-12` into a sorted list of page numbers. Returns the
/// offending fragment on bad input rather than silently comparing everything —
/// a typo that quietly widened the job is the kind of thing nobody notices.
fn parse_page_range(spec: &str) -> Result<Vec<i64>, String> {
    let mut pages = Vec::new();
    for part in spec.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        match part.split_once('-') {
            Some((from, to)) => {
                let (from, to) = (from.trim(), to.trim());
                let (Ok(a), Ok(b)) = (from.parse::<i64>(), to.parse::<i64>()) else {
                    return Err(part.to_string());
                };
                if a < 1 || b < a {
                    return Err(part.to_string());
                }
                pages.extend(a..=b);
            }
            None => match part.parse::<i64>() {
                Ok(n) if n >= 1 => pages.push(n),
                _ => return Err(part.to_string()),
            },
        }
    }
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

#[allow(clippy::too_many_arguments)]
fn pixelcompare_cmd(
    v1: &Path,
    v2: &Path,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
    viewer_dir: Option<&Path>,
    dpi: u16,
    pages: &[i64],
    max_diff: f64,
    opts: pixelcmp::Options,
) -> ExitCode {
    // Rasterising is where the minutes go, so each file gets its own bar. The
    // page count is not known until the document is open, which is why the bar
    // is created from inside the callback rather than before the call.
    let render = |path: &Path| {
        let mut bar: Option<progress::Progress> = None;
        let result = pdf::render_pages_rgba(path, pages, dpi, &mut |event| match event {
            pdf::RenderEvent::Starting { pages } => {
                bar = Some(progress::Progress::new(
                    format!("rasterising {}", file_name(path)),
                    pages,
                ));
            }
            pdf::RenderEvent::Rendered => {
                if let Some(b) = bar.as_mut() {
                    b.tick();
                }
            }
        });
        if let Some(b) = bar.as_mut() {
            b.finish();
        }
        result
    };
    let (before, after) = match (render(v1), render(v2)) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("error: {e:#}");
            return ExitCode::from(EXIT_INFRASTRUCTURE);
        }
    };

    if before.is_empty() && after.is_empty() {
        eprintln!("error: no page to compare — check --pages against the documents' length");
        return ExitCode::from(EXIT_INFRASTRUCTURE);
    }

    // Compared by page number, not by position: with --pages the two lists
    // hold the same numbers, and without it a document that is short is
    // missing pages rather than shifted.
    let numbers: Vec<i64> = {
        let mut n: Vec<i64> = before.iter().chain(after.iter()).map(|p| p.number).collect();
        n.sort_unstable();
        n.dedup();
        n
    };
    let find = |list: &[pdf::RenderedPage], want: i64| {
        list.iter()
            .find(|p| p.number == want)
            .map(|p| pixelcmp::Page::new(p.width, p.height, p.rgba.clone()))
    };

    let mut diagnostics = Vec::new();
    let mut diffs = Vec::new();
    let mut assets = Vec::new();
    let mut seen = std::collections::HashMap::new();

    // Comparing is the second slow stage: every pixel of every page, plus the
    // shift search. With a viewer it also encodes three PNGs per page, which
    // costs more than the comparison — so the label says so rather than
    // leaving someone to wonder why "comparing" is taking that long.
    let mut comparing = progress::Progress::new(
        if viewer_dir.is_some() { "comparing and encoding" } else { "comparing" },
        numbers.len(),
    );
    for n in numbers {
        let (a, b) = (find(&before, n), find(&after, n));
        let (a, b) = match (a, b) {
            (Some(a), Some(b)) => (a, b),
            // A page in one file and not the other is a finding on its own;
            // there is nothing to compare it against.
            (only, _) => {
                let which = if only.is_some() { v2 } else { v1 };
                let message = format!("page {n} is missing from {}", file_name(which));
                let occurrence = bump(&mut seen, &message);
                diagnostics.push(Diagnostic {
                    id: report::fingerprint("page count", &message, occurrence),
                    severity: Severity::Error,
                    check_name: "page count".into(),
                    message,
                    line: None,
                });
                comparing.tick();
                continue;
            }
        };

        let diff = pixelcmp::compare(&a, &b, opts, n);
        if diff.diff_percent > max_diff {
            let message = format!(
                "page {n}: {:.2}% of the pixels differ, in {} area(s){}",
                diff.diff_percent,
                diff.regions.len(),
                match diff.shift {
                    (0, 0) => String::new(),
                    (dx, dy) => format!(" (aligned by {dx}, {dy} px)"),
                }
            );
            let occurrence = bump(&mut seen, &message);
            diagnostics.push(Diagnostic {
                id: report::fingerprint("pixel difference", &message, occurrence),
                severity: Severity::Error,
                check_name: "pixel difference".into(),
                message,
                line: None,
            });
        }

        if viewer_dir.is_some() {
            let encode = |w, h, px: &[u8]| viewer::encode_png(w, h, px);
            match (
                encode(a.width, a.height, &a.rgba),
                encode(b.width, b.height, &b.rgba),
                encode(diff.width, diff.height, &diff.overlay),
            ) {
                (Ok(before_png), Ok(after_png), Ok(overlay_png)) => {
                    assets.push(viewer::PageAssets {
                        page: n,
                        before: before_png,
                        after: after_png,
                        overlay: overlay_png,
                    });
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    eprintln!("error: {e:#}");
                    return ExitCode::from(EXIT_INFRASTRUCTURE);
                }
            }
        }
        diffs.push(diff);
        comparing.tick();
    }
    comparing.finish();

    if let Some(dir) = viewer_dir {
        let mut writing = progress::Progress::new("writing the viewer", assets.len());
        let result =
            viewer::write(dir, &file_name(v1), &file_name(v2), &diffs, &assets, &mut || {
                writing.tick();
            });
        writing.finish();
        if let Err(e) = result {
            eprintln!("error: {e:#}");
            return ExitCode::from(EXIT_INFRASTRUCTURE);
        }
        note(format!("viewer written to {}", dir.join("index.html").display()));
    }

    let compared = diffs.len();
    let mut report = Report::new(
        "pixelcompare".into(),
        format!("{} → {}", v1.display(), v2.display()),
        None,
        compared as i64,
        diagnostics,
    );
    // The mean, not the worst: "how alike are these documents" is the question
    // this answers, and one rebuilt page out of two hundred should not read as
    // a different document. The per-page figures are in the diagnostics.
    report.similarity = Some(if compared == 0 {
        0.0
    } else {
        100.0 - diffs.iter().map(|d| d.diff_percent).sum::<f64>() / compared as f64
    });
    emit_report(&report, format, output_file, v1);
    ExitCode::from(report.exit_code(false) as u8)
}

/// How many times this exact message has already been produced — the
/// occurrence that keeps two identical findings from sharing an identifier.
fn bump(seen: &mut std::collections::HashMap<String, u32>, message: &str) -> u32 {
    let n = seen.entry(message.to_string()).or_insert(0);
    *n += 1;
    *n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_ranges() {
        assert_eq!(parse_page_range("1-4").unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(parse_page_range("3").unwrap(), vec![3]);
        assert_eq!(parse_page_range("1,3,7-9").unwrap(), vec![1, 3, 7, 8, 9]);
        // Overlaps collapse rather than comparing a page twice.
        assert_eq!(parse_page_range("1-3,2-4").unwrap(), vec![1, 2, 3, 4]);
        assert_eq!(parse_page_range(" 2 , 1 ").unwrap(), vec![1, 2]);
    }

    /// A typo must not quietly widen the job to every page.
    #[test]
    fn a_bad_page_range_names_the_fragment() {
        assert_eq!(parse_page_range("1-x").unwrap_err(), "1-x");
        assert_eq!(parse_page_range("4-2").unwrap_err(), "4-2");
        assert_eq!(parse_page_range("0").unwrap_err(), "0");
        assert_eq!(parse_page_range("abc").unwrap_err(), "abc");
    }

    /// clap's own consistency check: catches a flag that collides with another,
    /// which is easy to introduce with `global = true`.
    #[test]
    fn the_cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn completions_are_generated_for_every_shell() {
        for shell in clap_complete::Shell::value_variants() {
            let mut out = Vec::new();
            clap_complete::generate(*shell, &mut Cli::command(), "pdfl", &mut out);
            let script = String::from_utf8(out).expect("a completion script is text");
            // Naming a subcommand is the cheapest proof it walked the whole CLI
            // rather than emitting a stub.
            assert!(script.contains("inspect"), "{shell}: {script}");
        }
    }
}
