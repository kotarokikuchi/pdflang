//! Comparação semântica entre dois PDFs (comando `pdfl compare`).
//! Fora desta fatia: --granularity char, máscaras de região (--ignore-regions),
//! detecção explícita de reordenação (aparece como remoção+inserção) e
//! comparação visual — ainda não implementados.

use crate::interpreter::DocData;
use crate::report::{Diagnostic, Severity};
use regex::Regex;

pub struct CompareOptions {
    pub normalize: bool,
    pub ignore_dates: bool,
    /// Similaridade mínima (0–100) para o documento passar.
    pub similarity_threshold: f64,
}

pub struct CompareResult {
    pub diagnostics: Vec<Diagnostic>,
    /// Similaridade geral do texto, 0–100.
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

    // ---- metadados (mudança = aviso, não erro) ----
    for (key, va) in &a.metadata {
        let vb = b.metadata.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str()).unwrap_or("");
        if va != vb && !(va.is_empty() && vb.is_empty()) {
            push(&mut diags, Severity::Warning, "metadata", format!("{key}: \"{va}\" → \"{vb}\""));
        }
    }

    // ---- alinhamento de páginas por similaridade de texto (LCS) ----
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

    // ---- diff de texto por página alinhada ----
    let mut total_sim = 0.0;
    for (i, j) in &pairs {
        let sim = word_similarity(&pages_a[*i], &pages_b[*j]);
        total_sim += sim;
        if sim < 1.0 {
            let sample = line_diff_sample(&pages_a[*i], &pages_b[*j]);
            // Acima do threshold a mudança é tolerada: vira aviso informativo.
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

    // Similaridade geral: pares alinhados contam a média; páginas sem par contam 0.
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

/// Normalizações opcionais aplicadas antes de comparar.
fn prepare(text: &str, opts: &CompareOptions) -> String {
    let mut t = text.to_string();
    if opts.ignore_dates {
        // dd/mm/aaaa, aaaa-mm-dd, "12 de março de 2026", datas PDF D:...
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

/// LCS sobre páginas: pares (i, j) alinhados quando parecidos o bastante.
/// Para escalar a documentos de centenas/milhares de páginas, o casamento
/// usa um teste barato (igualdade de texto, senão Jaccard de conjuntos de
/// palavras pré-computados) e uma banda diagonal — a similaridade fina
/// (Levenshtein) só roda nos pares já alinhados.
fn align_pages(a: &[String], b: &[String]) -> Vec<(usize, usize)> {
    const MATCH: f64 = 0.6;
    let (n, m) = (a.len(), b.len());
    let sets_a: Vec<std::collections::HashSet<&str>> =
        a.iter().map(|t| t.split_whitespace().collect()).collect();
    let sets_b: Vec<std::collections::HashSet<&str>> =
        b.iter().map(|t| t.split_whitespace().collect()).collect();
    // ponytail: banda diagonal — deslocamentos de página maiores que isso
    // aparecem como remoção+inserção em vez de par alinhado
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

    // dp[i][j] = tamanho do melhor alinhamento de a[i..] com b[j..]
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

/// Similaridade Levenshtein sobre PALAVRAS (1.0 = idênticas).
/// ponytail: nível de palavra fixo; --granularity char|line fica para depois
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

/// Amostra das primeiras linhas que mudaram (até 3 de cada lado).
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
    fn identicos() {
        let a = doc(&["um dois três", "quatro cinco"], "T");
        let r = compare_documents(&a, &doc(&["um dois três", "quatro cinco"], "T"), &opts());
        assert_eq!(r.similarity, 100.0);
        assert!(r.diagnostics.is_empty());
    }

    #[test]
    fn texto_mudou() {
        let a = doc(&["um dois três quatro"], "T");
        let b = doc(&["um dois TRÊS quatro"], "T");
        let r = compare_documents(&a, &b, &opts());
        assert_eq!(r.similarity, 75.0);
        assert!(r.diagnostics.iter().any(|d| d.check_name == "text"));
        assert!(r.diagnostics.iter().any(|d| d.check_name == "similarity"));
    }

    #[test]
    fn pagina_inserida() {
        let a = doc(&["primeira página conteúdo", "última página conteúdo"], "T");
        let b = doc(&["primeira página conteúdo", "página nova no meio", "última página conteúdo"], "T");
        let r = compare_documents(&a, &b, &opts());
        let msgs: Vec<&str> = r.diagnostics.iter().map(|d| d.message.as_str()).collect();
        assert!(msgs.iter().any(|m| m.contains("page 2 inserted")), "{msgs:?}");
        assert!(msgs.iter().any(|m| m.contains("page count changed: 2 → 3")));
    }

    #[test]
    fn metadados_mudaram() {
        let a = doc(&["texto igual aqui"], "Versão 1");
        let b = doc(&["texto igual aqui"], "Versão 2");
        let r = compare_documents(&a, &b, &opts());
        assert_eq!(r.similarity, 100.0);
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].severity, Severity::Warning);
        assert!(r.diagnostics[0].message.contains("Versão 1"));
    }

    #[test]
    fn ignora_datas_e_normaliza() {
        let a = doc(&["Emitido em 01/02/2026 VALOR TOTAL"], "T");
        let b = doc(&["Emitido em 15/03/2026 valor  total"], "T");
        let com = CompareOptions { normalize: true, ignore_dates: true, similarity_threshold: 100.0 };
        let r = compare_documents(&a, &b, &com);
        assert_eq!(r.similarity, 100.0, "{:?}", r.diagnostics);
        let sem = compare_documents(&a, &b, &opts());
        assert!(sem.similarity < 100.0);
    }

    #[test]
    fn threshold_tolera_mudancas() {
        let a = doc(&["um dois três quatro"], "T");
        let b = doc(&["um dois TRÊS quatro"], "T"); // 75% de similaridade
        let tolerante = CompareOptions { normalize: false, ignore_dates: false, similarity_threshold: 50.0 };
        let r = compare_documents(&a, &b, &tolerante);
        assert!(r.diagnostics.iter().all(|d| d.severity != Severity::Error), "{:?}", r.diagnostics);
        assert!(r.diagnostics.iter().any(|d| d.check_name == "text"));
    }

    #[test]
    fn amostra_de_diff() {
        let a = doc(&["linha um\nlinha dois\nlinha três"], "T");
        let b = doc(&["linha um\nlinha DOIS\nlinha três"], "T");
        let r = compare_documents(&a, &b, &opts());
        let msg = &r.diagnostics.iter().find(|d| d.check_name == "text").unwrap().message;
        assert!(msg.contains("-linha dois"), "{msg}");
        assert!(msg.contains("+linha DOIS"), "{msg}");
    }
}
