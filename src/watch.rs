//! The `pdfl watch` command — watches a folder and validates each new or
//! changed file.
//! Polling is the default and `--events` opts into the operating system's
//! notifications. That way round on purpose: inotify on an NFS or SMB mount
//! reports what this machine writes and nothing else, so a hot folder fed over
//! the network would go quiet without ever saying so — and going quiet is the
//! failure this project exists to avoid. Measured before choosing: listing
//! 10,000 files every 200ms costs no measurable CPU, and the settle time
//! dominates the latency, so on a local folder the two modes finish within a
//! hundredth of a second of each other.
//!
//! Events never replace the settle logic — a large file arrives in pieces and
//! fires its first event on its first byte. They only decide when to look
//! again, which is why the loop below is the same in both modes. It wakes when
//! the freshest file has settled, not a whole interval later, which is where
//! the latency actually was.
//! Out of scope here: --paired (waiting for a v1+v2 pair) — not implemented yet.
//!
//! Each file is analysed by a child `pdfl run`, and this process renders the
//! report. pdfium serialises every call behind one mutex, so threads sharing a
//! process cannot analyse two documents at once — only separate processes can.
//! The parent doing the rendering keeps one code path for every format and
//! every value of --jobs.

use crate::report::{Diagnostic, Report, Severity};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant, SystemTime};

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
    /// Kills a file's analysis if it runs longer than this and reports the
    /// file as rejected instead. `None` waits forever — the behaviour before
    /// this flag existed.
    pub timeout: Option<Duration>,
    /// `--var NAME=VALUE`, forwarded to every file. Without this, a script
    /// reading `vars.*` could never be watched: every file would fail with
    /// "was not provided", regardless of its content.
    pub vars: Vec<String>,
    /// Append-only record of what has been validated, and the only way a batch
    /// survives being interrupted.
    pub journal: Option<PathBuf>,
    /// Wake on filesystem notifications instead of listing the folder on a
    /// timer. Opt-in: see the note at the top of this file.
    pub events: bool,
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
    // What a previous run already got through. Keyed by content, not by name or
    // by time: a file replaced with different bytes under the same name has not
    // been validated, whatever its timestamp says.
    let mut journal = match Journal::open(opts.journal.as_deref()) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("error: {e}");
            return crate::EXIT_INFRASTRUCTURE;
        }
    };
    if let Some(done) = journal.resumed() {
        crate::note(format!("journal: {done} file(s) already validated, skipping"));
    }
    let mut worst: u8 = 0;

    // Kept alive as long as the loop runs: dropping the watcher ends the
    // notifications.
    let (_watcher, events) = if opts.events && !opts.once { subscribe(folder) } else { (None, None) };
    crate::note(format!(
        "watching {} (pattern {}, {}, settle {}ms){}",
        folder.display(),
        opts.pattern,
        if events.is_some() { "events" } else { "polling" },
        opts.debounce_ms,
        if opts.once { " — single pass" } else { "" }
    ));

    loop {
        let mut files = Vec::new();
        collect_files(folder, opts.depth, &mut files);

        let mut due = Vec::new();
        // How long until the freshest file seen has settled, so the wait below
        // ends exactly then rather than a whole interval later.
        let mut settles_in: Option<Duration> = None;
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
                let settle = Duration::from_millis(opts.debounce_ms);
                if age < settle {
                    let remaining = settle - age;
                    settles_in = Some(settles_in.map_or(remaining, |s: Duration| s.min(remaining)));
                    continue;
                }
            }
            if let Some(exit) = journal.verdict(&file) {
                // Already validated, and its verdict still counts. --fail-fast
                // is not triggered by it: nothing new failed, and there is no
                // work to stop.
                worst = worst.max(exit);
                processed.insert(file.clone(), mtime);
                continue;
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
            if let Err(e) = journal.record(file, &report, exit) {
                eprintln!("error writing the journal: {e}");
                return crate::EXIT_INFRASTRUCTURE;
            }
            worst = worst.max(exit);
            if opts.fail_fast && exit >= 2 {
                crate::note(format!("--fail-fast: stopping at the first error ({})", file.display()));
                return exit;
            }
        }

        if opts.once {
            return worst;
        }
        // Nothing settling: wait a full interval. Otherwise wake exactly when
        // the freshest file is ready — a shorter sleep, never a longer one.
        let idle = Duration::from_millis(opts.debounce_ms.max(200));
        let wait = settles_in.map_or(idle, |s| s.min(idle).max(Duration::from_millis(20)));
        match &events {
            // Nothing settling and nothing to poll for: sleep until the folder
            // moves. This is the whole of what --events buys.
            Some(rx) if settles_in.is_none() => {
                let _ = rx.recv();
                while rx.try_recv().is_ok() {}
            }
            // A file is still being written. Its last write already fired its
            // event, so this wait has to end by itself.
            Some(rx) => {
                let _ = rx.recv_timeout(wait);
                while rx.try_recv().is_ok() {}
            }
            None => std::thread::sleep(wait),
        }
    }
}

/// Subscribes to the folder's notifications, or says why it could not and
/// leaves the caller polling. A watcher that fails falls back out loud: silence
/// is the one outcome this must not have.
fn subscribe(
    folder: &Path,
) -> (Option<notify::RecommendedWatcher>, Option<std::sync::mpsc::Receiver<()>>) {
    use notify::Watcher;
    let (tx, rx) = std::sync::mpsc::channel();
    let watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        // The event's contents are not used: it says something moved, and the
        // loop re-reads the folder to find out what. Coalescing a burst into
        // one pass is the point.
        if res.is_ok() {
            let _ = tx.send(());
        }
    });
    let mut watcher = match watcher {
        Ok(w) => w,
        Err(e) => {
            crate::note(format!("--events unavailable ({e}) — polling instead"));
            return (None, None);
        }
    };
    if let Err(e) = watcher.watch(folder, notify::RecursiveMode::Recursive) {
        crate::note(format!("--events unavailable on {} ({e}) — polling instead", folder.display()));
        return (None, None);
    }
    (Some(watcher), Some(rx))
}

/// Analyses one file in a child `pdfl run` and returns its report.
///
/// A verdict of any severity still prints a report, and so does an unreadable
/// input — its report carries the reason as a finding, which is exactly what
/// belongs in the file written for that document. A file whose analysis was
/// killed for running past `timeout` gets the same treatment: a report with a
/// finding, so a hung document is a rejection the batch can act on rather than
/// a silence that leaves the file unwritten and the batch stuck. Only a child
/// that could not be started at all leaves nothing to write.
fn analyse(exe: &Path, script: &Path, file: &Path, timeout: Option<Duration>, vars: &[String]) -> Option<Report> {
    match run_bounded(exe, script, file, timeout, vars) {
        RunOutcome::SpawnFailed(e) => {
            eprintln!("error: could not run {}: {e}", exe.display());
            None
        }
        RunOutcome::TimedOut => {
            Some(timeout_report(script, file, timeout.expect("only reachable with a timeout set")))
        }
        RunOutcome::Finished(status, stdout, stderr) => match serde_json::from_slice(&stdout) {
            Ok(report) => Some(report),
            Err(_) => {
                let stderr = String::from_utf8_lossy(&stderr);
                let reason = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
                eprintln!(
                    "error: {} produced no report ({status}){}{}",
                    file.display(),
                    if reason.is_empty() { "" } else { ": " },
                    reason
                );
                None
            }
        },
    }
}

enum RunOutcome {
    Finished(std::process::ExitStatus, Vec<u8>, Vec<u8>),
    TimedOut,
    SpawnFailed(std::io::Error),
}

/// Builds the `pdfl run` command for one file and hands it to `spawn_bounded`.
/// A malformed PDF is what `--timeout` exists for — pdfium can loop or block
/// on adversarial input — not the script: recursion in a `.pdfl` function is
/// already depth-limited by the interpreter, so there is no hang to defend
/// against on that side.
fn run_bounded(
    exe: &Path,
    script: &Path,
    file: &Path,
    timeout: Option<Duration>,
    vars: &[String],
) -> RunOutcome {
    let mut cmd = std::process::Command::new(exe);
    cmd.arg("run").arg(script).arg(file).args(["--output", "json", "--quiet"]);
    cmd.args(vars.iter().flat_map(|v| ["--var", v]));
    spawn_bounded(cmd, timeout)
}

/// Runs `cmd`, killing it if it outlives `timeout`.
///
/// `Command::output()` cannot be bounded — it blocks until the child exits,
/// which is exactly what a hung child defeats. This spawns the child instead
/// and polls `try_wait` on an interval, so a deadline can cut in. stdout and
/// stderr are drained on their own threads throughout, the same way `output()`
/// does it internally: a child that fills its pipe buffer before anyone reads
/// it deadlocks against a parent that is only polling for exit.
fn spawn_bounded(mut cmd: std::process::Command, timeout: Option<Duration>) -> RunOutcome {
    let mut child = match cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return RunOutcome::SpawnFailed(e),
    };

    let mut child_stdout = child.stdout.take().expect("stdout was piped");
    let mut child_stderr = child.stderr.take().expect("stderr was piped");
    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut child_stdout, &mut buf);
        buf
    });
    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = std::io::Read::read_to_end(&mut child_stderr, &mut buf);
        buf
    });

    let deadline = timeout.map(|t| Instant::now() + t);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_thread.join().unwrap_or_default();
                let stderr = stderr_thread.join().unwrap_or_default();
                return RunOutcome::Finished(status, stdout, stderr);
            }
            Ok(None) => {
                if deadline.is_some_and(|d| Instant::now() >= d) {
                    // Best-effort: the child may have exited in the instant
                    // between the check above and this kill, in which case its
                    // real report is discarded in favour of the timeout one.
                    // Losing one verdict at the exact deadline is a fair price
                    // for never blocking a batch forever.
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return RunOutcome::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return RunOutcome::SpawnFailed(e);
            }
        }
    }
}

/// A report for a file whose analysis was killed for running past `timeout`,
/// shaped exactly like the "could not open" report `pdfl run` builds when a
/// PDF cannot be loaded at all — one finding, so it prints, writes and journals
/// through the same path as every other verdict.
fn timeout_report(script: &Path, file: &Path, timeout: Duration) -> Report {
    let message = format!("analysis did not finish within {}s and was killed", timeout.as_secs());
    let diag = Diagnostic {
        id: crate::report::fingerprint("timeout", &message, 1),
        severity: Severity::Error,
        check_name: "timeout".into(),
        message,
        line: None,
    };
    Report::new(crate::file_name(script), file.to_string_lossy().into_owned(), None, 0, vec![diag])
}

/// Runs the batch `jobs` at a time, keeping the order of `files`.
fn analyse_all(exe: &Path, files: &[PathBuf], opts: &WatchOptions) -> Vec<Option<Report>> {
    if opts.jobs <= 1 || files.len() <= 1 {
        return files.iter().map(|f| analyse(exe, &opts.script, f, opts.timeout, &opts.vars)).collect();
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
                let report = analyse(exe, &opts.script, file, opts.timeout, &opts.vars);
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


/// The batch journal: one JSON object per line, appended as each file is
/// validated, and read back on the next run to skip what is already done.
///
/// It exists because a batch of five thousand files that dies at four thousand
/// must not start over — and it is a file the user asked for by name, never
/// state the tool keeps behind their back. Nothing is written without
/// `--journal`.
///
/// There is no timestamp in a line. The journal answers *whether* a file was
/// validated and what came of it; the report next to it answers *what*, and the
/// filesystem answers *when*. Leaving time out keeps a re-run over the same
/// inputs byte-identical to the first, which is the property everything else in
/// this tool is held to.
struct Journal {
    file: Option<std::fs::File>,
    /// path → the bytes that were validated, and the verdict they got.
    done: HashMap<String, (String, u8)>,
    resumed: Option<usize>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct JournalEntry {
    input: String,
    sha256: String,
    status: String,
    errors: usize,
    warnings: usize,
    exit: u8,
}

impl Journal {
    fn open(path: Option<&Path>) -> Result<Self, String> {
        let Some(path) = path else {
            return Ok(Journal { file: None, done: HashMap::new(), resumed: None });
        };
        let mut done = HashMap::new();
        let mut lines = 0;
        if path.is_file() {
            let text = std::fs::read_to_string(path)
                .map_err(|e| format!("could not read the journal {}: {e}", path.display()))?;
            for (n, line) in text.lines().enumerate() {
                if line.trim().is_empty() {
                    continue;
                }
                let entry: JournalEntry = serde_json::from_str(line).map_err(|e| {
                    format!("{}: line {} is not a journal entry: {e}", path.display(), n + 1)
                })?;
                // Append-only, so a later line for the same file wins: that is
                // what re-validating a changed file looks like.
                done.insert(entry.input.clone(), (entry.sha256, entry.exit));
                lines += 1;
            }
        }
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("could not open the journal {}: {e}", path.display()))?;
        Ok(Journal { file: Some(file), done, resumed: (lines > 0).then_some(lines) })
    }

    fn resumed(&self) -> Option<usize> {
        self.resumed
    }

    /// The verdict this exact file — same path, same bytes — already got, if it
    /// is recorded.
    ///
    /// The caller folds it into the batch's exit code. A resumed run that
    /// skipped a rejected file must not report a clean batch: the journal is
    /// the record of the batch, and the exit code is its verdict.
    fn verdict(&self, file: &Path) -> Option<u8> {
        if self.file.is_none() || self.done.is_empty() {
            return None;
        }
        let (recorded, exit) = self.done.get(&file.to_string_lossy().into_owned())?;
        (sha256_of(file).as_ref() == Some(recorded)).then_some(*exit)
    }

    fn record(&mut self, file: &Path, report: &Report, exit: u8) -> std::io::Result<()> {
        let Some(handle) = self.file.as_mut() else { return Ok(()) };
        let entry = JournalEntry {
            input: file.to_string_lossy().into_owned(),
            // A file that could not be hashed is recorded with an empty hash,
            // so the next run treats it as unvalidated rather than as done.
            sha256: sha256_of(file).unwrap_or_default(),
            status: report.status.clone(),
            errors: report.error_count,
            warnings: report.warning_count,
            exit,
        };
        let line = serde_json::to_string(&entry).expect("an entry is always serializable");
        // Flushed per line: what a crash leaves behind has to be true.
        use std::io::Write;
        writeln!(handle, "{line}")?;
        handle.flush()?;
        self.done.insert(entry.input, (entry.sha256, entry.exit));
        Ok(())
    }
}

fn sha256_of(file: &Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(file).ok()?;
    Some(format!("{:x}", Sha256::digest(&bytes)))
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

    fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pdfl-journal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn report(status: &str, errors: usize) -> Report {
        let mut r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, Vec::new());
        r.status = status.into();
        r.error_count = errors;
        r
    }

    #[test]
    fn a_recorded_file_keeps_its_verdict_across_runs() {
        let dir = tempdir("resume");
        let pdf = dir.join("a.pdf");
        std::fs::write(&pdf, b"bytes").unwrap();
        let path = dir.join("batch.jsonl");

        let mut journal = Journal::open(Some(&path)).unwrap();
        assert_eq!(journal.verdict(&pdf), None, "nothing is recorded yet");
        journal.record(&pdf, &report("FAIL", 1), 2).unwrap();

        // A second run reads the file back: the rejection still counts, or a
        // resumed batch would report a clean folder.
        let reopened = Journal::open(Some(&path)).unwrap();
        assert_eq!(reopened.verdict(&pdf), Some(2));
        assert_eq!(reopened.resumed(), Some(1));
    }

    /// Same name, different bytes: not the file that was validated.
    #[test]
    fn changed_bytes_are_not_done() {
        let dir = tempdir("changed");
        let pdf = dir.join("a.pdf");
        std::fs::write(&pdf, b"first").unwrap();
        let path = dir.join("batch.jsonl");
        let mut journal = Journal::open(Some(&path)).unwrap();
        journal.record(&pdf, &report("PASS", 0), 0).unwrap();

        std::fs::write(&pdf, b"second").unwrap();
        assert_eq!(Journal::open(Some(&path)).unwrap().verdict(&pdf), None);
    }

    /// Append-only: the last line for a file is the one that counts.
    #[test]
    fn the_last_entry_for_a_file_wins() {
        let dir = tempdir("append");
        let pdf = dir.join("a.pdf");
        std::fs::write(&pdf, b"bytes").unwrap();
        let path = dir.join("batch.jsonl");
        let mut journal = Journal::open(Some(&path)).unwrap();
        journal.record(&pdf, &report("FAIL", 1), 2).unwrap();
        journal.record(&pdf, &report("PASS", 0), 0).unwrap();

        assert_eq!(Journal::open(Some(&path)).unwrap().verdict(&pdf), Some(0));
        let lines = std::fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 2, "an entry is never rewritten");
    }

    #[test]
    fn without_the_flag_nothing_is_written_or_remembered() {
        let dir = tempdir("off");
        let pdf = dir.join("a.pdf");
        std::fs::write(&pdf, b"bytes").unwrap();
        let mut journal = Journal::open(None).unwrap();
        journal.record(&pdf, &report("FAIL", 1), 2).unwrap();
        assert_eq!(journal.verdict(&pdf), None);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1, "only the pdf");
    }

    /// A journal that cannot be trusted must not be treated as empty: skipping
    /// files on a misread line is the one outcome worse than starting over.
    #[test]
    fn a_damaged_journal_names_the_line() {
        let dir = tempdir("damaged");
        let path = dir.join("batch.jsonl");
        std::fs::write(&path, "{\"input\":\"a.pdf\",\"sha256\":\"x\",\"status\":\"PASS\",\"errors\":0,\"warnings\":0,\"exit\":0}\nnot json\n").unwrap();
        let e = match Journal::open(Some(&path)) {
            Err(e) => e,
            Ok(_) => panic!("a damaged journal must not open"),
        };
        assert!(e.contains("line 2"), "{e}");
    }

    /// The real kill path, exercised against `sleep` rather than a hung `pdfl
    /// run` — nothing in the language can hang the interpreter on purpose
    /// (recursion is depth-limited), so a slow *process* is the honest stand-in
    /// for what actually hangs: pdfium on adversarial input.
    // Drives a real subprocess through a POSIX shell, so it runs where
    // there is one. The behaviour itself is not platform-specific; the
    // stand-in for `pdfl` is.
    #[cfg(unix)]
    #[test]
    fn a_slow_child_is_killed_at_the_deadline() {
        let mut cmd = std::process::Command::new("sleep");
        cmd.arg("5");
        let start = Instant::now();
        let outcome = spawn_bounded(cmd, Some(Duration::from_millis(200)));
        let elapsed = start.elapsed();
        assert!(matches!(outcome, RunOutcome::TimedOut));
        // Loose bound: proves it was killed near the deadline, not that it ran
        // the full 5s (a timing assertion tight enough to be exact would be
        // flaky under CI load).
        assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    }

    // Drives a real subprocess through a POSIX shell, so it runs where
    // there is one. The behaviour itself is not platform-specific; the
    // stand-in for `pdfl` is.
    #[cfg(unix)]
    #[test]
    fn a_child_finishing_before_the_deadline_is_not_killed() {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", "echo hi"]);
        let outcome = spawn_bounded(cmd, Some(Duration::from_secs(5)));
        match outcome {
            RunOutcome::Finished(status, stdout, _) => {
                assert!(status.success());
                assert_eq!(stdout, b"hi\n");
            }
            _ => panic!("expected the child to finish"),
        }
    }

    // Drives a real subprocess through a POSIX shell, so it runs where
    // there is one. The behaviour itself is not platform-specific; the
    // stand-in for `pdfl` is.
    #[cfg(unix)]
    #[test]
    fn no_timeout_waits_for_a_slow_child_to_finish() {
        let mut cmd = std::process::Command::new("sh");
        cmd.args(["-c", "sleep 0.2; echo done"]);
        let outcome = spawn_bounded(cmd, None);
        match outcome {
            RunOutcome::Finished(status, stdout, _) => {
                assert!(status.success());
                assert_eq!(stdout, b"done\n");
            }
            _ => panic!("expected the child to finish"),
        }
    }

    /// Without this, a script reading `vars.*` could never be watched at all —
    /// every file would fail with "was not provided", regardless of its
    /// content. Checked against a real subprocess standing in for `pdfl`,
    /// consistent with how the rest of this module is tested.
    // Drives a real subprocess through a POSIX shell, so it runs where
    // there is one. The behaviour itself is not platform-specific; the
    // stand-in for `pdfl` is.
    #[cfg(unix)]
    #[test]
    fn run_bounded_forwards_vars_to_the_child() {
        let dir = tempdir("run-bounded-vars");
        let echo_var = dir.join("fake-pdfl.sh");
        std::fs::write(
            &echo_var,
            "#!/bin/sh\n\
             val=\"\"\n\
             while [ $# -gt 0 ]; do\n\
             \x20 if [ \"$1\" = \"--var\" ]; then val=\"$2\"; fi\n\
             \x20 shift\n\
             done\n\
             printf '%s' \"$val\"\n",
        )
        .unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&echo_var, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Retried past ETXTBSY. This test writes an executable and then runs
        // it, inside a process where other tests are spawning their own
        // children: a fork that lands between our write and its close inherits
        // the write handle, and Linux refuses to exec a file any process holds
        // open for writing. It made the suite fail about one run in three.
        // Nothing to do with what is under test — in use the child is the
        // pdfl binary, which nobody is writing to.
        let mut outcome = RunOutcome::TimedOut;
        for _ in 0..50 {
            outcome = run_bounded(
                &echo_var,
                Path::new("p.pdfl"),
                Path::new("f.pdf"),
                None,
                &["client=AcmeCorp".to_string()],
            );
            match &outcome {
                RunOutcome::SpawnFailed(e)
                    if e.kind() == std::io::ErrorKind::ExecutableFileBusy =>
                {
                    std::thread::sleep(Duration::from_millis(10));
                }
                _ => break,
            }
        }
        match outcome {
            RunOutcome::Finished(_, stdout, _) => assert_eq!(stdout, b"client=AcmeCorp"),
            _ => panic!("expected the fake child to finish"),
        }
    }

    #[test]
    fn the_timeout_report_is_a_rejection_naming_the_deadline() {
        let report = timeout_report(Path::new("p.pdfl"), Path::new("huge.pdf"), Duration::from_secs(120));
        assert_eq!(report.status, "FAIL");
        assert_eq!(report.error_count, 1);
        assert_eq!(report.diagnostics[0].check_name, "timeout");
        assert!(report.diagnostics[0].message.contains("120s"), "{}", report.diagnostics[0].message);
        // Flows through the same exit-code path as every other verdict: no
        // special case in write_report or the journal for a timeout.
        assert_eq!(report.exit_code(false), 2);
    }

    /// A resumed batch that skips a timed-out file must not forget it was
    /// rejected — the same contract already held for an ordinary failure, now
    /// checked for the timeout path specifically.
    #[test]
    fn a_timed_out_file_is_journaled_as_rejected() {
        let dir = tempdir("timeout-journal");
        let pdf = dir.join("huge.pdf");
        std::fs::write(&pdf, b"bytes").unwrap();
        let path = dir.join("batch.jsonl");

        let report = timeout_report(Path::new("p.pdfl"), &pdf, Duration::from_secs(60));
        let exit = report.exit_code(false) as u8;
        assert_eq!(exit, 2);
        let mut journal = Journal::open(Some(&path)).unwrap();
        journal.record(&pdf, &report, exit).unwrap();

        let resumed = Journal::open(Some(&path)).unwrap();
        assert_eq!(resumed.verdict(&pdf), Some(2), "a resumed batch must still see the rejection");
    }
}
