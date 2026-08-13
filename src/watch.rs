//! The `pdfl watch` command — watches a folder and validates each new or
//! changed file.
//! ponytail: polling with debounce instead of inotify — portable, no new
//! dependency; swap for the `notify` crate if latency starts to matter.
//! Out of scope here: --paired (waiting for a v1+v2 pair) — not implemented yet.
//!
//! Each file is analysed by a child `pdfl run`, and this process renders the
//! report. pdfium serialises every call behind one mutex, so threads sharing a
//! process cannot analyse two documents at once — only separate processes can.
//! The parent doing the rendering keeps one code path for every format and
//! every value of --jobs.

use crate::report::Report;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, SystemTime};

pub struct WatchOptions {
    /// The script under which every file is validated.
    pub script: PathBuf,
    /// Files analysed at the same time, each in its own process.
    pub jobs: usize,
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

pub fn watch(folder: &Path, opts: &WatchOptions) -> u8 {
    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: could not find the pdfl binary to validate with: {e}");
            return crate::EXIT_INFRASTRUCTURE;
        }
    };
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

        let mut due = Vec::new();
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
            due.push(file);
        }
        due.sort();

        // Analysed together, reported in order: what a batch prints must not
        // depend on which child finished first.
        let analysed = analyse_all(&exe, &due, opts);
        for (file, report) in due.iter().zip(analysed) {
            let Some(report) = report else { continue };
            let exit = write_report(file, &report, opts);
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

/// Analyses one file in a child `pdfl run` and returns its report.
///
/// A verdict of any severity still prints a report, and so does an unreadable
/// input — its report carries the reason as a finding, which is exactly what
/// belongs in the file written for that document. Only the absence of a report
/// leaves nothing to write.
fn analyse(exe: &Path, script: &Path, file: &Path) -> Option<Report> {
    let output = std::process::Command::new(exe)
        .arg("run")
        .arg(script)
        .arg(file)
        .args(["--output", "json", "--quiet"])
        .output();
    let output = match output {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: could not run {}: {e}", exe.display());
            return None;
        }
    };
    match serde_json::from_slice(&output.stdout) {
        Ok(report) => Some(report),
        Err(_) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let reason = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
            eprintln!(
                "error: {} produced no report ({}){}{}",
                file.display(),
                output.status,
                if reason.is_empty() { "" } else { ": " },
                reason
            );
            None
        }
    }
}

/// Runs the batch `jobs` at a time, keeping the order of `files`.
fn analyse_all(exe: &Path, files: &[PathBuf], opts: &WatchOptions) -> Vec<Option<Report>> {
    if opts.jobs <= 1 || files.len() <= 1 {
        return files.iter().map(|f| analyse(exe, &opts.script, f)).collect();
    }
    let next = AtomicUsize::new(0);
    // With --fail-fast, no new file is started once one has failed. The ones
    // already running finish: killing them would leave half-written reports.
    let stop = AtomicBool::new(false);
    let done: std::sync::Mutex<Vec<(usize, Option<Report>)>> =
        std::sync::Mutex::new(Vec::with_capacity(files.len()));
    std::thread::scope(|scope| {
        for _ in 0..opts.jobs.min(files.len()) {
            scope.spawn(|| loop {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                let i = next.fetch_add(1, Ordering::Relaxed);
                let Some(file) = files.get(i) else { return };
                let report = analyse(exe, &opts.script, file);
                if opts.fail_fast && report.as_ref().is_some_and(|r| r.exit_code(false) >= 2) {
                    stop.store(true, Ordering::Relaxed);
                }
                done.lock().expect("no panic holds this lock").push((i, report));
            });
        }
    });
    let mut results = done.into_inner().expect("the threads are joined");
    results.sort_by_key(|(i, _)| *i);
    // A file skipped by --fail-fast has no report and nothing to write; the
    // padding keeps the results aligned with the files they came from.
    let mut out: Vec<Option<Report>> = (0..files.len()).map(|_| None).collect();
    for (i, report) in results {
        out[i] = report;
    }
    out
}

/// Renders the report in the chosen format and writes it next to the file (or
/// into --output-dir). Returns the exit code `pdfl run` would have given.
fn write_report(file: &Path, report: &Report, opts: &WatchOptions) -> u8 {
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

    crate::note(format!(
        "{} → {} ({}, {} error(s), {} warning(s))",
        file.display(),
        out_path.display(),
        report.status,
        report.error_count,
        report.warning_count
    ));
    report.exit_code(false) as u8
}

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
