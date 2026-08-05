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
mod prepressns;
mod report;
mod structns;
mod textns;
mod visualns;
mod watch;

use clap::{Parser, Subcommand, ValueEnum};
use report::{Diagnostic, Report, Severity};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "pdfl", version, about = "Runs .pdfl scripts to validate and normalize PDFs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
    },
    /// Shows a quick summary of a PDF
    Inspect {
        /// Input PDF
        input: PathBuf,
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
    },
    /// Formats a .pdfl script (2 spaces, consistent spacing)
    Fmt {
        /// The .pdfl script
        script: PathBuf,
        /// Does not write: fails (exit 1) if the file is not formatted
        #[arg(long)]
        check: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Csv,
    Html,
    Pdf,
}

/// Writes the report: stdout for text formats (or --output-file); PDF always
/// goes to a file (--output-file or <input>.report.pdf).
fn emit_report(report: &Report, format: OutputFormat, file: Option<&PathBuf>, input: &Path) {
    let bytes: Vec<u8> = match format {
        OutputFormat::Json => report.to_json().into_bytes(),
        OutputFormat::Csv => report.to_csv().into_bytes(),
        OutputFormat::Html => report.to_html().into_bytes(),
        OutputFormat::Pdf => report.to_pdf(),
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
            Ok(()) => eprintln!("report saved to {}", path.display()),
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

/// Loads the PDF; a failure becomes a FAIL report with exit 2.
fn load_doc(
    input: &Path,
    script_name: &str,
) -> Result<std::rc::Rc<interpreter::DocData>, ExitCode> {
    pdf::load_document(input).map_err(|e| {
        let diag = Diagnostic {
            id: "PDFL-000".into(),
            severity: Severity::Error,
            check_name: "loading".into(),
            message: format!("{e:#}"),
            line: None,
        };
        let report =
            Report::new(script_name.into(), input.to_string_lossy().into_owned(), None, 0, vec![diag]);
        println!("{}", report.to_json());
        ExitCode::from(2)
    })
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Run { script, input, output, output_file, fail_on, verbose } => {
            run_cmd(&script, &input, output, output_file.as_ref(), fail_on, verbose)
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
        } => {
            let program = match load_program(&script) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let opts = watch::WatchOptions {
                script_dir: script.parent().map(|d| d.to_path_buf()).unwrap_or_default(),
                pattern,
                exclude,
                output_dir,
                depth,
                debounce_ms: debounce,
                fail_fast,
                once,
                format: report_format,
            };
            ExitCode::from(watch::watch(&folder, &program, &file_name(&script), &opts))
        }
        Command::Inspect { input } => {
            match pdf::load_document(&input) {
                Ok(doc) => {
                    print!("{}", inspect::inspect(&doc));
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {e:#}");
                    ExitCode::from(2)
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
                    ExitCode::from(2)
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
                ExitCode::from(2)
            }
        },
        Command::Lint { script } => {
            let program = match load_program(&script) {
                Ok(p) => p,
                Err(code) => return code,
            };
            let warnings = lint::lint(&program);
            for w in &warnings {
                println!("{}: warning: {w}", script.display());
            }
            if warnings.is_empty() {
                eprintln!("{}: no problems found", script.display());
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
                eprintln!("{}: already formatted", script.display());
                ExitCode::SUCCESS
            } else if let Err(e) = std::fs::write(&script, &formatted) {
                eprintln!("error writing {}: {e}", script.display());
                ExitCode::from(2)
            } else {
                eprintln!("{}: formatted", script.display());
                ExitCode::SUCCESS
            }
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
    if verbose {
        eprintln!("PDF loaded: {} page(s), {} font(s)", doc.pages.len(), doc.fonts.len());
    }

    let mut interp = interpreter::Interpreter::new();
    if let Some(dir) = script.parent() {
        interp.script_dir = dir.to_path_buf();
    }
    let total_pages = doc.pages.len() as i64;
    if let Err(e) = interp.run(&program, doc) {
        eprintln!("{}: {e}", script.display());
        return ExitCode::from(3);
    }

    let report = Report::new(
        script_name,
        input.to_string_lossy().into_owned(),
        interp.profile_name.clone(),
        total_pages,
        interp.diagnostics,
    );
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
                return ExitCode::from(2);
            }
        }
    };
    if !dry_run && !fixes.is_empty() {
        eprintln!("normalized PDF saved to {}", output.display());
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
