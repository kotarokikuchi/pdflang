//! The `pdfl test` command — a golden-file runner for .pdfl scripts.
//!
//! A test case is a PDF next to an expected report: `invoice.pdf` and
//! `invoice.expected.json`. The script runs against each PDF and the report is
//! compared to the file recorded beside it, so a change in what a profile
//! *finds* shows up as a failing test instead of as a surprise in production.
//!
//! `--update` writes the expected files. Recording is a deliberate act: a run
//! that silently refreshed its own baseline would never fail.

use crate::note;
use std::path::{Path, PathBuf};

/// Runs one case as a child `pdfl run` and returns its JSON report.
///
/// A child rather than a call, for two reasons. pdfium serialises every call
/// behind one mutex, so threads sharing this process cannot run two documents
/// at once — measured on eight 41-page files, threads were *slower* than
/// sequential (12.2s against 8.3s) while separate processes finished in 1.2s.
/// And a document that kills the library takes only its own case down.
///
/// `input_file` is reduced to the file's name: the full path depends on the
/// directory the runner was invoked from, and a baseline that changes with the
/// caller's shell is not a baseline.
fn run_case(exe: &Path, script: &Path, pdf_path: &Path) -> Result<String, String> {
    let output = std::process::Command::new(exe)
        .arg("run")
        .arg(script)
        .arg(pdf_path)
        .args(["--output", "json", "--quiet"])
        .output()
        .map_err(|e| format!("could not run {}: {e}", exe.display()))?;

    // A verdict of any severity still prints a report; so does an unreadable
    // input, whose report carries the reason as a finding. Only the absence of
    // a report is a case that could not be judged.
    let mut value: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|_| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let reason = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        if reason.is_empty() {
            format!("no report ({})", output.status)
        } else {
            reason
        }
    })?;
    value["input_file"] = serde_json::Value::String(crate::file_name(pdf_path));
    Ok(serde_json::to_string_pretty(&value).expect("a parsed report serialises"))
}

/// Runs the cases `jobs` at a time and returns the reports in the order the
/// cases were given, so what is printed never depends on which child finished
/// first.
fn run_cases(exe: &Path, script: &Path, pdfs: &[PathBuf], jobs: usize) -> Vec<Result<String, String>> {
    if jobs <= 1 || pdfs.len() <= 1 {
        return pdfs.iter().map(|p| run_case(exe, script, p)).collect();
    }
    let next = std::sync::atomic::AtomicUsize::new(0);
    let done: std::sync::Mutex<Vec<(usize, Result<String, String>)>> =
        std::sync::Mutex::new(Vec::with_capacity(pdfs.len()));
    std::thread::scope(|scope| {
        for _ in 0..jobs.min(pdfs.len()) {
            scope.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let Some(pdf_path) = pdfs.get(i) else { return };
                let result = run_case(exe, script, pdf_path);
                done.lock().expect("no panic holds this lock").push((i, result));
            });
        }
    });
    let mut results = done.into_inner().expect("the threads are joined");
    results.sort_by_key(|(i, _)| *i);
    results.into_iter().map(|(_, r)| r).collect()
}

pub fn test(script: &Path, dir: &Path, update: bool, jobs: usize) -> u8 {
    let mut pdfs: Vec<PathBuf> = match std::fs::read_dir(dir) {
        Ok(entries) => entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("pdf"))
            })
            .collect(),
        Err(e) => {
            eprintln!("error: could not read {}: {e}", dir.display());
            return crate::EXIT_INFRASTRUCTURE;
        }
    };
    pdfs.sort();
    if pdfs.is_empty() {
        eprintln!("error: no .pdf in {} — a test case is a PDF and its expected report", dir.display());
        return crate::EXIT_INFRASTRUCTURE;
    }

    let exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: could not find the pdfl binary to run the cases with: {e}");
            return crate::EXIT_INFRASTRUCTURE;
        }
    };

    let mut failed = 0;
    let mut written = 0;
    // The cases run first and are judged afterwards, in order: what a run
    // prints must not depend on which child happened to finish first.
    let reports = run_cases(&exe, script, &pdfs, jobs);
    for (pdf_path, result) in pdfs.iter().zip(reports) {
        let name = crate::file_name(pdf_path);
        let expected_path = expected_path(pdf_path);
        let actual = match result {
            Ok(r) => r,
            Err(e) => {
                // A case that could not be judged is a failing case, not a
                // crashed run: the others still have something to say.
                println!("FAIL {name}");
                println!("     {e}");
                failed += 1;
                continue;
            }
        };

        if update {
            match std::fs::write(&expected_path, format!("{actual}\n")) {
                Ok(()) => {
                    written += 1;
                    note(format!("recorded {}", crate::file_name(&expected_path)));
                }
                Err(e) => {
                    eprintln!("error: could not write {}: {e}", expected_path.display());
                    return crate::EXIT_INFRASTRUCTURE;
                }
            }
            continue;
        }

        let expected = match std::fs::read_to_string(&expected_path) {
            Ok(s) => s,
            Err(_) => {
                println!("FAIL {name}");
                println!(
                    "     no {} yet — run `pdfl test {} --update` to record it",
                    crate::file_name(&expected_path),
                    script.display()
                );
                failed += 1;
                continue;
            }
        };
        let differences = diff(&expected, &actual);
        if differences.is_empty() {
            println!("ok   {name}");
        } else {
            println!("FAIL {name}");
            for line in differences {
                println!("     {line}");
            }
            failed += 1;
        }
    }

    if update {
        note(format!("{written} expected report(s) recorded in {}", dir.display()));
        return 0;
    }
    let passed = pdfs.len() - failed;
    println!("{passed} passed, {failed} failed");
    if failed > 0 {
        2
    } else {
        0
    }
}

fn expected_path(pdf_path: &Path) -> PathBuf {
    let stem = pdf_path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    pdf_path.with_file_name(format!("{stem}.expected.json"))
}

/// What changed, in the terms the author wrote the test in — the counts, the
/// verdict, and which findings appeared or vanished. A textual diff of the two
/// JSON files would be longer and say less.
fn diff(expected: &str, actual: &str) -> Vec<String> {
    let (e, a): (serde_json::Value, serde_json::Value) = match (
        serde_json::from_str(expected),
        serde_json::from_str(actual),
    ) {
        (Ok(e), Ok(a)) => (e, a),
        (Err(err), _) => return vec![format!("the expected report is not valid JSON: {err}")],
        (_, Err(err)) => return vec![format!("the report produced is not valid JSON: {err}")],
    };

    let mut out = Vec::new();
    for field in [
        "schema_version",
        "status",
        "error_count",
        "warning_count",
        "info_count",
        "total_pages_analyzed",
        "profile",
    ] {
        let (want, got) = (&e[field], &a[field]);
        if want != got {
            out.push(format!("{field}: expected {want}, got {got}"));
        }
    }

    let empty = Vec::new();
    let want = e["diagnostics"].as_array().unwrap_or(&empty);
    let got = a["diagnostics"].as_array().unwrap_or(&empty);
    for d in want {
        if !got.contains(d) {
            out.push(format!("missing:    {}", describe(d)));
        }
    }
    for d in got {
        if !want.contains(d) {
            out.push(format!("unexpected: {}", describe(d)));
        }
    }

    // Only reported when nothing above explains the difference, so the useful
    // lines are not buried under a list of check names.
    if out.is_empty() && e["checks_run"] != a["checks_run"] {
        out.push(format!(
            "checks that ran: expected {}, got {}",
            e["checks_run"], a["checks_run"]
        ));
    }
    out
}

fn describe(d: &serde_json::Value) -> String {
    let text = |k: &str| d[k].as_str().unwrap_or("?").to_string();
    let line = d["line"].as_u64().map(|l| format!(" (line {l})")).unwrap_or_default();
    format!("{} [{}] {}{}: {}", text("id"), text("severity"), text("check_name"), line, text("message"))
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = r#"{
      "schema_version": 1,
      "script_name": "p.pdfl",
      "input_file": "a.pdf",
      "status": "FAIL",
      "total_pages_analyzed": 3,
      "error_count": 1,
      "warning_count": 0,
      "info_count": 0,
      "diagnostics": [
        {"id": "PDFL-aaaa1111", "severity": "error", "check_name": "Ink",
         "message": "324% ink", "line": 12}
      ],
      "checks_run": ["Ink", "Fonts"]
    }"#;

    #[test]
    fn an_identical_report_has_no_differences() {
        assert!(diff(BASE, BASE).is_empty());
    }

    /// The path a case was invoked with is not part of what is being tested.
    #[test]
    fn the_input_path_is_not_compared() {
        let moved = BASE.replace("\"a.pdf\"", "\"/somewhere/else/a.pdf\"");
        assert!(diff(BASE, &moved).is_empty(), "{:?}", diff(BASE, &moved));
    }

    #[test]
    fn a_finding_that_disappeared_is_named() {
        let fixed = BASE.replace(r#"{"id": "PDFL-aaaa1111", "severity": "error", "check_name": "Ink",
         "message": "324% ink", "line": 12}"#, "");
        let fixed = fixed.replace("\"error_count\": 1", "\"error_count\": 0");
        let d = diff(BASE, &fixed);
        assert!(d.iter().any(|l| l.contains("error_count: expected 1, got 0")), "{d:?}");
        assert!(d.iter().any(|l| l.starts_with("missing:") && l.contains("324% ink")), "{d:?}");
    }

    #[test]
    fn a_new_finding_is_named() {
        let extra = BASE.replace(
            r#""message": "324% ink", "line": 12}"#,
            r#""message": "324% ink", "line": 12},
        {"id": "PDFL-bbbb2222", "severity": "warning", "check_name": "Bleed",
         "message": "bleed below 3mm"}"#,
        );
        let d = diff(BASE, &extra);
        assert!(d.iter().any(|l| l.starts_with("unexpected:") && l.contains("bleed below 3mm")), "{d:?}");
    }

    /// A check that stopped running while the findings stayed the same is worth
    /// hearing about — it usually means a tag or a rename went wrong.
    #[test]
    fn a_check_that_stopped_running_is_reported() {
        let fewer = BASE.replace(r#"["Ink", "Fonts"]"#, r#"["Ink"]"#);
        let d = diff(BASE, &fewer);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].starts_with("checks that ran:"), "{d:?}");
    }

    #[test]
    fn the_expected_file_next_to_the_pdf() {
        assert_eq!(
            expected_path(Path::new("cases/invoice.pdf")),
            Path::new("cases/invoice.expected.json")
        );
    }
}
