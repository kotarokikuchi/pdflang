//! Semantic comparison between two PDFs (the `pdfl compare` command).
//! Out of scope here: --granularity char, region masks (--ignore-regions),
//! explicit reorder detection (it shows up as removal+insertion) and
//! visual comparison — not implemented yet.

use crate::interpreter::DocData;
use crate::report::{Diagnostic, Severity};
use regex::Regex;

pub struct CompareOptions {
    pub normalize: bool,
    pub ignore_dates: bool,
    /// Minimum similarity (0–100) for the document to pass.
    pub similarity_threshold: f64,
}

pub struct CompareResult {
    pub diagnostics: Vec<Diagnostic>,
    /// Overall text similarity, 0–100.
    pub similarity: f64,
}

pub fn compare_documents(a: &DocData, b: &DocData, opts: &CompareOptions) -> CompareResult {
    let mut diags = Vec::new();
    let mut next_id = 1usize;
    let mut push = |diags: &mut Vec<Diagnostic>, severity: Severity, check: &str, message: String| {
        diags.push(Diagnostic {
            id: format!("PDFL-{:03}", next_id),
            severity,
            check_name: check.into(),
            message,
            line: None,
        });
        next_id += 1;
    };

    // ---- metadata (a change is a warning, not an error) ----
    for (key, va) in &a.metadata {
        let vb = b.metadata.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or("");
        if va != vb && !(va.is_empty() && vb.is_empty()) {
            push(&mut diags, Severity::Warning, "metadata", format!("{key}: \"{va}\" → \"{vb}\""));
        }
    }

    // ---- page alignment by text similarity (LCS) ----
    let pages_a: Vec<String> = a.pages.iter().map(|p| prepare(&p.text, opts)).collect();
    let pages_b: Vec<String> = b.pages.iter().map(|p| prepare(&p.text, opts)).collect();
    let pairs = align_pages(&pages_a, &pages_b);

    if a.pages.len() != b.pages.len() {
        push(
            &mut diags,
            Severity::Error,
            "structure",
            format!("page count changed: {} → {}", a.pages.len(), b.pages.len()),
        );
    }

    let matched_a: Vec<usize> = pairs.iter().map(|(i, _)| *i).collect();
    let matched_b: Vec<usize> = pairs.iter().map(|(_, j)| *j).collect();
    for i in 0..pages_a.len() {
        if !matched_a.contains(&i) {
            push(&mut diags, Severity::Error, "structure", format!("page {} removed", i + 1));
        }
    }
    for j in 0..pages_b.len() {
        if !matched_b.contains(&j) {
            push(&mut diags, Severity::Error, "structure", format!("page {} inserted", j + 1));
        }
    }

    // ---- text diff per aligned page ----
    let mut total_sim = 0.0;
    for (i, j) in &pairs {
        let sim = word_similarity(&pages_a[*i], &pages_b[*j]);
        total_sim += sim;
        if sim < 1.0 {
            let sample = line_diff_sample(&pages_a[*i], &pages_b[*j]);
            // Above the threshold the change is tolerated: it becomes an informative warning.
            let severity = if sim * 100.0 < opts.similarity_threshold {
                Severity::Error
            } else {
                Severity::Warning
            };
            push(
                &mut diags,
                severity,
                "text",
                format!(
                    "page {} → {}: similarity {:.1}%{sample}",
                    i + 1,
                    j + 1,
                    sim * 100.0
                ),
            );
        }
    }

    // Overall similarity: aligned pairs count their average; unpaired pages count 0.
    let denom = pairs.len() + (pages_a.len() - pairs.len()) + (pages_b.len() - pairs.len());
    let similarity = if denom == 0 { 100.0 } else { (total_sim / denom as f64) * 100.0 };
    let similarity = (similarity * 10.0).round() / 10.0;

    if similarity < opts.similarity_threshold {
        push(
            &mut diags,
            Severity::Error,
            "similarity",
            format!("overall similarity {similarity}% below the minimum of {}%", opts.similarity_threshold),
        );
    }

    CompareResult { diagnostics: diags, similarity }
}

/// Optional normalizations applied before comparing.
fn prepare(text: &str, opts: &CompareOptions) -> String {
    let mut t = text.to_string();
    if opts.ignore_dates {
        // dd/mm/yyyy, yyyy-mm-dd, "12 de março de 2026", PDF dates D:...
        for pat in [
            r"\b\d{1,2}/\d{1,2}/\d{2,4}\b",
            r"\b\d{4}-\d{2}-\d{2}\b",
            r"\b\d{1,2} de [a-zç]+ de \d{4}\b",
            r"D:\d{8,14}",
        ] {
            t = Regex::new(pat).unwrap().replace_all(&t, "<data>").into_owned();
        }
    }
    if opts.normalize {
        t = t.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase();
    }
    t
}

/// LCS over pages: pairs (i, j) are aligned when similar enough.
/// To scale to documents of hundreds/thousands of pages, matching uses a
/// cheap test (text equality, otherwise Jaccard over precomputed word sets)
/// and a diagonal band — the fine-grained similarity (Levenshtein) only runs
/// on pairs that are already aligned.
fn align_pages(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    const MATCH: f64 = 0.6;
    let (n, m) = (a.len(), b.len());
    let sets_a: Vec<std::collections::HashSet<&str>> =
        a.iter().map(|t| t.split_whitespace().collect()).collect();
    let sets_b: Vec<std::collections::HashSet<&str>> =
        b.iter().map(|t| t.split_whitespace().collect()).collect();
    // ponytail: diagonal band — page shifts larger than this show up as
    // removal+insertion instead of an aligned pair
    let band = n.abs_diff(m) + 25;

    let is_match = |i: usize, j: usize| -> bool {
        if i.abs_diff(j) > band {
            return false;
        }
        if a[i] == b[j] {
            return true;
        }
        let (sa, sb) = (&sets_a[i], &sets_b[j]);
        if sa.is_empty() && sb.is_empty() {
            return true;
        }
        let inter = sa.intersection(sb).count();
        let union = sa.len() + sb.len() - inter;
        union > 0 && inter as f64 / union as f64 >= MATCH
    };

    // dp[i][j] = length of the best alignment of a[i..] with b[j..]
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if is_match(i, j) {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if dp[i][j] == dp[i + 1][j + 1] + 1 && is_match(i, j) {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

/// Levenshtein similarity over WORDS (1.0 = identical).
/// ponytail: fixed at word level; --granularity char|line is left for later
fn word_similarity(a: &str, b: &str) -> f64 {
    let wa: Vec<&str> = a.split_whitespace().collect();
    let wb: Vec<&str> = b.split_whitespace().collect();
    if wa.is_empty() && wb.is_empty() {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=wb.len()).collect();
    let mut curr = vec![0; wb.len() + 1];
    for (i, x) in wa.iter().enumerate() {
        curr[0] = i + 1;
        for (j, y) in wb.iter().enumerate() {
            let cost = if x == y { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    1.0 - prev[wb.len()] as f64 / wa.len().max(wb.len()) as f64
}

/// Sample of the first lines that changed (up to 3 on each side).
fn line_diff_sample(a: &str, b: &str) -> String {
    let la: Vec<&str> = a.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let lb: Vec<&str> = b.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let removed: Vec<&&str> = la.iter().filter(|l| !lb.contains(l)).take(3).collect();
    let added: Vec<&&str> = lb.iter().filter(|l| !la.contains(l)).take(3).collect();
    let mut out = String::new();
    for l in removed {
        out.push_str(&format!(" | -{l}"));
    }
    for l in added {
        out.push_str(&format!(" | +{l}"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{DocData, PageBoxes, PageData};
    use std::rc::Rc;

    fn doc(texts: &[&str], title: &str) -> DocData {
        DocData {
            filename: "t.pdf".into(),
            title: title.into(),
            author: "".into(),
            pages: texts
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    Rc::new(PageData {
                        index: i as i64,
                        width: 595.0,
                        height: 842.0,
                        text: t.to_string(),
                        images: vec![],
                        tac_max: 0.0,
                        ink_avg: 0.0,
                        min_stroke_pt: None,
                        boxes: PageBoxes::default(),
                    })
                })
                .collect(),
            fonts: vec![],
            metadata: vec![("Title".into(), title.into())],
            file_size: 0,
            sha256: String::new(),
            object_count: 0,
            path: std::path::PathBuf::new(),
            barcodes: std::cell::OnceCell::new(),
            lowlevel: std::cell::OnceCell::new(),
            colors: std::cell::OnceCell::new(),
        }
    }

    fn opts() -> CompareOptions {
        CompareOptions { normalize: false, ignore_dates: false, similarity_threshold: 100.0 }
    }

    #[test]
    fn identical() {
        let a = doc(&["one two three", "four five"], "T");
        let r = compare_documents(&a, &doc(&["one two three", "four five"], "T"), &opts());
        assert_eq!(r.similarity, 100.0);
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn text_changed() {
        let a = doc(&["one two three four"], "T");
        let b = doc(&["one two THREE four"], "T");
        let r = compare_documents(&a, &b, &opts());
        assert_eq!(r.similarity, 75.0);
        assert!(r.diagnostics.iter().any(|d| d.check_name == "text"));
        assert!(r.diagnostics.iter().any(|d| d.check_name == "similarity"));
    }

    #[test]
    fn page_inserted() {
        let a = doc(&["first page content", "last page content"], "T");
        let b = doc(&["first page content", "new page in between", "last page content"], "T");
        let r = compare_documents(&a, &b, &opts());
        let msgs: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("page 2 inserted")), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("page count changed: 2 → 3")));
    }

    #[test]
    fn metadata_changed() {
        let a = doc(&["same text here"], "Version 1");
        let b = doc(&["same text here"], "Version 2");
        let r = compare_documents(&a, &b, &opts());
        assert_eq!(r.similarity, 100.0);
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].severity, Severity::Warning);
        assert!(r.diagnostics[0].message.contains("Version 1"));
    }

    #[test]
    fn ignores_dates_and_normalizes() {
        let a = doc(&["Issued on 01/02/2026 TOTAL AMOUNT"], "T");
        let b = doc(&["Issued on 15/03/2026 total  amount"], "T");
        let com = CompareOptions { normalize: true, ignore_dates: true, similarity_threshold: 100.0 };
        let r = compare_documents(&a, &b, &com);
        assert_eq!(r.similarity, 100.0, "{:?}", r.diagnostics);
        let sem = compare_documents(&a, &b, &opts());
        assert!(sem.similarity < 100.0);
    }

    #[test]
    fn threshold_tolerates_changes() {
        let a = doc(&["one two three four"], "T");
        let b = doc(&["one two THREE four"], "T"); // 75% similarity
        let tolerante = CompareOptions { normalize: false, ignore_dates: false, similarity_threshold: 50.0 };
        let r = compare_documents(&a, &b, &tolerante);
        assert!(r.diagnostics.iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics);
        assert!(r.diagnostics.iter().any(|d| d.check_name == "text"));
    }

    #[test]
    fn diff_sample() {
        let a = doc(&["line one\nline two\nline three"], "T");
        let b = doc(&["line one\nline TWO\nline three"], "T");
        let r = compare_documents(&a, &b, &opts());
        let msg = &r.diagnostics.iter().find(|d| d.check_name == "text").unwrap().message;
        assert!(msg.contains("-line two"), "{msg}");
        assert!(msg.contains("+line TWO"), "{msg}");
    }
}
