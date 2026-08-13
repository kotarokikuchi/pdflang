//! The `pdfl watch` command — watches a folder and validates each new or
//! alterado.
//! ponytail: polling with debounce instead of inotify — portable, no new
//! dependency; swap for the `notify` crate if latency starts to matter.
//! Out of scope here: --paired (waiting for a v1+v2 pair) — not implemented yet.

use crate::report::{Diagnostic, Report, Severity};
use crate::{ast, interpreter, pdf};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

pub struct WatchOptions {
    /// The script's directory (the base for imports).
    pub script_dir: PathBuf,
    pub pattern: String,
    pub exclude: Option<String>,
    pub output_dir: Option<PathBuf>,
    pub depth: usize,
    pub debounce_ms: u64,
    pub fail_fast: bool,
    /// Processes the files already present and exits (useful for batches and CI).
    pub once: bool,
    pub format: crate::OutputFormat,
}

pub fn watch(folder: &Path, program: &[ast::Stmt], script_name: &str, opts: &WatchOptions) -> u8 {
    let mut processed: HashMap<PathBuf, SystemTime> = HashMap::new();
    let mut worst: u8 = 0;
    crate::note(format!(
        "watching {} (pattern {}, debounce {}ms){}",
        folder.display(),
        opts.pattern,
        opts.debounce_ms,
        if opts.once { " — single pass" } else { "" }
    ));

    loop {
        let mut files = Vec::new();
        collect_files(folder, opts.depth, &mut files);

        for file in files {
            let name = file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !wildcard_match(&opts.pattern, &name) {
                continue;
            }
            if let Some(ex) = &opts.exclude {
                if wildcard_match(ex, &name) {
                    continue;
                }
            }
            let Ok(mtime) = file.metadata().and_then(|m| m.modified()) else { continue };
            if processed.get(&file) == Some(&mtime) {
                continue;
            }
            // Debounce: only processes once the file has stopped being written.
            if !opts.once {
                let age = SystemTime::now().duration_since(mtime).unwrap_or_default();
                if age < Duration::from_millis(opts.debounce_ms) {
                    continue;
                }
            }
            processed.insert(file.clone(), mtime);

            let exit = process_file(&file, program, script_name, opts);
            worst = worst.max(exit);
            if opts.fail_fast && exit >= 2 {
                crate::note(format!("--fail-fast: stopping at the first error ({})", file.display()));
                return exit;
            }
        }

        if opts.once {
            return worst;
        }
        std::thread::sleep(Duration::from_millis(opts.debounce_ms.max(200)));
    }
}

/// Validates a file and writes the report next to it (or into --output-dir).
/// Returns the exit code `pdfl run` would have given.
fn process_file(file: &Path, program: &[ast::Stmt], script_name: &str, opts: &WatchOptions) -> u8 {
    let report = match pdf::load_document(file) {
        Ok(doc) => {
            let total_pages = doc.pages.len() as i64;
            let mut interp = interpreter::Interpreter::new();
            interp.script_dir = opts.script_dir.clone();
            match interp.run(program, doc) {
                Ok(()) => {
                    let mut r = Report::new(
                        script_name.into(),
                        file.to_string_lossy().into_owned(),
                        interp.profile_name.clone(),
                        total_pages,
                        interp.diagnostics,
                    );
                    r.checks_run = interp.checks_run;
                    r
                }
                Err(e) => error_report(script_name, file, format!("{e}")),
            }
        }
        Err(e) => error_report(script_name, file, format!("{e:#}")),
    };

    let (content, ext) = match opts.format {
        crate::OutputFormat::Json => (report.to_json().into_bytes(), "json"),
        crate::OutputFormat::Csv => (report.to_csv().into_bytes(), "csv"),
        crate::OutputFormat::Html => (report.to_html().into_bytes(), "html"),
        crate::OutputFormat::Pdf => (report.to_pdf(), "pdf"),
        crate::OutputFormat::Sarif => (report.to_sarif().into_bytes(), "sarif"),
        crate::OutputFormat::Junit => (report.to_junit().into_bytes(), "xml"),
    };
    let dir = opts.output_dir.clone().unwrap_or_else(|| file.parent().unwrap_or(Path::new(".")).to_path_buf());
    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let out_path = dir.join(format!("{stem}.report.{ext}"));
    if let Err(e) = std::fs::write(&out_path, content) {
        eprintln!("error writing {}: {e}", out_path.display());
        return 2;
    }

    let exit = report.exit_code(false) as u8;
    crate::note(format!(
        "{} → {} ({}, {} error(s), {} warning(s))",
        file.display(),
        out_path.display(),
        report.status,
        report.error_count,
        report.warning_count
    ));
    exit
}

fn error_report(script_name: &str, file: &Path, message: String) -> Report {
    Report::new(
        script_name.into(),
        file.to_string_lossy().into_owned(),
        None,
        0,
        vec![Diagnostic {
            id: "PDFL-000".into(),
            severity: Severity::Error,
            check_name: "loading".into(),
            message,
            line: None,
        }],
    )
}

/// Lists files up to `depth` levels (1 = the folder only).
fn collect_files(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort(); // deterministic order
    for path in entries {
        if path.is_dir() {
            collect_files(&path, depth - 1, out);
        } else {
            out.push(path);
        }
    }
}

/// Simple `*` wildcard matching (enough for *.pdf, draft_*.pdf).
fn wildcard_match(pattern: &str, name: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == name;
    }
    let mut rest = name;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(pos) => rest = &rest[pos + part.len()..],
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards() {
        assert!(wildcard_match("*.pdf", "file.pdf"));
        assert!(!wildcard_match("*.pdf", "file.txt"));
        assert!(wildcard_match("prova_*.pdf", "prova_01.pdf"));
        assert!(!wildcard_match("prova_*.pdf", "final_01.pdf"));
        assert!(wildcard_match("*relatorio*", "meu_relatorio_final.txt"));
        assert!(wildcard_match("exato.pdf", "exato.pdf"));
        assert!(!wildcard_match("exato.pdf", "outro.pdf"));
    }
}
