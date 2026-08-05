//! Namespace `text::` — text extraction, normalization and validation.
//! The functions work on the document's text by default; most of them take
//! a string as an optional argument.

use crate::interpreter::{DocData, RuntimeError, Value};
use regex::Regex;
use std::rc::Rc;

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        // ---- basics ----
        "extract_all" => Ok(Value::Str(full_text(doc))),
        "extract_from_page" => {
            let n = int_arg(args, 0, name)?;
            let page = doc
                .pages
                .get((n - 1).max(0) as usize)
                .filter(|_| n >= 1)
                .ok_or_else(|| err(format!("page {n} does not exist (the PDF has {})", doc.pages.len())))?;
            Ok(Value::Str(page.text.clone()))
        }
        "normalize" => Ok(Value::Str(normalize(&text_arg(doc, args)))),
        "split_words" => Ok(str_list(words(&text_arg(doc, args)))),
        "split_sentences" => {
            let text = text_arg(doc, args);
            let parts = Regex::new(r"[.!?]+\s+")
                .unwrap()
                .split(&text)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(str_list(parts))
        }
        "split_paragraphs" => {
            let text = text_arg(doc, args);
            let parts = Regex::new(r"\n\s*\n")
                .unwrap()
                .split(&text)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            Ok(str_list(parts))
        }
        "count_words" => Ok(Value::Int(words(&text_arg(doc, args)).len() as i64)),
        "count_characters" => Ok(Value::Int(text_arg(doc, args).chars().count() as i64)),
        "detect_language" => Ok(Value::Str(detect_language(&text_arg(doc, args)).into())),
        // ---- glossary and validation (return a boolean for require/assert) ----
        "require_text" => {
            let needle = str_arg(args, 0, name)?;
            Ok(Value::Bool(normalize(&full_text(doc)).contains(&normalize(&needle))))
        }
        "forbid_text" => {
            let needle = str_arg(args, 0, name)?;
            Ok(Value::Bool(!normalize(&full_text(doc)).contains(&normalize(&needle))))
        }
        "require_match" => Ok(Value::Bool(regex_arg(args, name)?.is_match(&full_text(doc)))),
        "forbid_match" => Ok(Value::Bool(!regex_arg(args, name)?.is_match(&full_text(doc)))),
        "fuzzy_match" => {
            let a = str_arg(args, 0, name)?;
            let b = str_arg(args, 1, name)?;
            Ok(Value::Float(similarity(&normalize(&a), &normalize(&b))))
        }
        "detect_personal_data" | "detect_pii" => {
            let text = text_arg(doc, args);
            let mut found = Vec::new();
            // CPF/CNPJ only count with a valid check digit — a number that
            // "looks like" one but is not (e.g. 111.111.111-12) raises no false alarm.
            for m in Regex::new(r"\b\d{3}\.\d{3}\.\d{3}-\d{2}\b").unwrap().find_iter(&text) {
                if cpf_valid(m.as_str()) {
                    found.push(format!("CPF: {}", m.as_str()));
                }
            }
            for m in Regex::new(r"\b\d{2}\.\d{3}\.\d{3}/\d{4}-\d{2}\b").unwrap().find_iter(&text) {
                if cnpj_valid(m.as_str()) {
                    found.push(format!("CNPJ: {}", m.as_str()));
                }
            }
            for (label, pattern) in [
                ("E-mail", r"[\w.+-]+@[\w-]+\.[\w.-]+"),
                ("Phone", r"\(\d{2}\)\s*9?\d{4}-\d{4}"),
            ] {
                for m in Regex::new(pattern).unwrap().find_iter(&text) {
                    found.push(format!("{label}: {}", m.as_str()));
                }
            }
            Ok(str_list(found))
        }
        // ---- format validations ----
        "validate_cpf" => Ok(Value::Bool(cpf_valid(&str_arg(args, 0, name)?))),
        "validate_cnpj" => Ok(Value::Bool(cnpj_valid(&str_arg(args, 0, name)?))),
        "validate_date_format" => {
            // validate_date_format(s [, "dd/mm/aaaa" | "aaaa-mm-dd"])
            let s = str_arg(args, 0, name)?;
            let format = match args.get(1) {
                Some(Value::Str(f)) => Some(f.clone()),
                _ => None,
            };
            Ok(Value::Bool(date_valid(&s, format.as_deref())?))
        }
        "validate_phone_format" => {
            // Brazilian phone number: (DD) 9XXXX-XXXX or (DD) XXXX-XXXX,
            // with optional punctuation.
            let s = str_arg(args, 0, name)?;
            let digits: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
            let re = Regex::new(r"^\(?\d{2}\)?\s?9?\d{4}-?\d{4}$").unwrap();
            Ok(Value::Bool(re.is_match(s.trim()) && matches!(digits.len(), 10 | 11)))
        }
        "validate_format" => {
            // validate_format(s, regex) — the WHOLE string must match
            let s = str_arg(args, 0, name)?;
            let pattern = str_arg(args, 1, name)?;
            let re = Regex::new(&format!("^(?:{pattern})$"))
                .map_err(|e| err(format!("text::{name}: invalid pattern: {e}")))?;
            Ok(Value::Bool(re.is_match(&s)))
        }
        // ---- advanced operations ----
        "diff" => {
            // diff(a, b) -> lines that changed: "-only in a", "+only in b"
            let a = str_arg(args, 0, name)?;
            let b = str_arg(args, 1, name)?;
            let la: Vec<&str> = a.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            let lb: Vec<&str> = b.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
            let mut out = Vec::new();
            for l in &la {
                if !lb.contains(l) {
                    out.push(format!("-{l}"));
                }
            }
            for l in &lb {
                if !la.contains(l) {
                    out.push(format!("+{l}"));
                }
            }
            Ok(str_list(out))
        }
        "extract_with_normalization" => Ok(Value::Str(normalize(&full_text(doc)))),
        "extract_from_region" => {
            // extract_from_region(page, region)
            let page = match args.first() {
                Some(Value::Int(n)) => *n,
                Some(Value::Page(p)) => p.index + 1,
                _ => return Err(err(format!("text::{name} expects the page number and the region"))),
            };
            let region = match args.get(1) {
                Some(Value::Region(r)) => r.clone(),
                _ => return Err(err(format!("text::{name} expects a region as the 2nd argument"))),
            };
            if page < 1 || page as usize > doc.pages.len() {
                return Err(err(format!("page {page} does not exist (the PDF has {})", doc.pages.len())));
            }
            crate::pdf::extract_text_in_region(
                &doc.path,
                page,
                [region.x, region.y, region.width, region.height],
            )
            .map(Value::Str)
            .map_err(|e| err(format!("text::{name}: {e:#}")))
        }
        "detect_rasterized_text" => {
            // Heuristic: a page with no extractable text but with a large image
            // (>= 50% of the area) = the text is probably rasterized.
            // Real OCR waits until Tesseract is available.
            let suspect = doc.pages.iter().any(|p| {
                let page_area = p.width * p.height;
                p.text.trim().is_empty()
                    && page_area > 0.0
                    && p.images.iter().any(|img| {
                        let w_pt = img.width as f64 / (img.dpi_x.max(1.0) / 72.0);
                        let h_pt = img.height as f64 / (img.dpi_y.max(1.0) / 72.0);
                        w_pt * h_pt >= page_area * 0.5
                    })
            });
            Ok(Value::Bool(suspect))
        }
        _ => Err(err(format!("unknown function: text::{name}"))),
    }
}

// ---- auxiliares ----

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

fn full_text(doc: &DocData) -> String {
    doc.pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n")
}

/// The first string argument, if given; otherwise the document's full text.
fn text_arg(doc: &DocData, args: &[Value]) -> String {
    match args.first() {
        Some(Value::Str(s)) => s.clone(),
        _ => full_text(doc),
    }
}

fn str_arg(args: &[Value], i: usize, name: &str) -> Result<String, RuntimeError> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(other) => Err(err(format!("text::{name} expects a string, got {}", other.type_name()))),
        None => Err(err(format!("text::{name} expects a string argument at position {}", i + 1))),
    }
}

fn int_arg(args: &[Value], i: usize, name: &str) -> Result<i64, RuntimeError> {
    match args.get(i) {
        Some(Value::Int(n)) => Ok(*n),
        Some(other) => Err(err(format!("text::{name} expects an integer, got {}", other.type_name()))),
        None => Err(err(format!("text::{name} expects the page number"))),
    }
}

fn regex_arg(args: &[Value], name: &str) -> Result<Regex, RuntimeError> {
    let pattern = str_arg(args, 0, name)?;
    Regex::new(&pattern).map_err(|e| err(format!("text::{name}: invalid pattern: {e}")))
}

fn str_list(items: Vec<String>) -> Value {
    Value::List(Rc::new(items.into_iter().map(Value::Str).collect()))
}

/// Normalization: lowercase + collapsed whitespace.
fn normalize(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Words: split on whitespace, with edge punctuation removed.
fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect()
}

/// CPF with its check digit (mod 11). Accepted with or without punctuation.
fn cpf_valid(s: &str) -> bool {
    let d: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 11 || d.iter().all(|&x| x == d[0]) {
        return false; // tamanho errado ou todos iguais (111.111.111-11)
    }
    let dv = |n: usize| {
        let sum: u32 = d[..n].iter().enumerate().map(|(i, &x)| x * (n as u32 + 1 - i as u32)).sum();
        let r = (sum * 10) % 11;
        if r == 10 { 0 } else { r }
    };
    dv(9) == d[9] && dv(10) == d[10]
}

/// CNPJ with its check digit (mod 11). Accepted with or without punctuation.
fn cnpj_valid(s: &str) -> bool {
    let d: Vec<u32> = s.chars().filter_map(|c| c.to_digit(10)).collect();
    if d.len() != 14 || d.iter().all(|&x| x == d[0]) {
        return false;
    }
    let dv = |n: usize| {
        let weights: Vec<u32> = (2..=9).cycle().take(n).collect();
        let sum: u32 = d[..n].iter().rev().zip(weights).map(|(&x, w)| x * w).sum();
        let r = sum % 11;
        if r < 2 { 0 } else { 11 - r }
    };
    dv(12) == d[12] && dv(13) == d[13]
}

/// A calendar-valid date. Without a format: accepts dd/mm/yyyy or yyyy-mm-dd.
fn date_valid(s: &str, format: Option<&str>) -> Result<bool, RuntimeError> {
    let s = s.trim();
    let (d, m, y) = match format {
        Some("dd/mm/aaaa") | None if Regex::new(r"^\d{1,2}/\d{1,2}/\d{4}$").unwrap().is_match(s) => {
            let p: Vec<i64> = s.split('/').map(|x| x.parse().unwrap()).collect();
            (p[0], p[1], p[2])
        }
        Some("aaaa-mm-dd") | None if Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap().is_match(s) => {
            let p: Vec<i64> = s.split('-').map(|x| x.parse().unwrap()).collect();
            (p[2], p[1], p[0])
        }
        Some(f) if f != "dd/mm/aaaa" && f != "aaaa-mm-dd" => {
            return Err(err(format!(
                "text::validate_date_format: unknown format '{f}' (use dd/mm/aaaa or aaaa-mm-dd)"
            )))
        }
        _ => return Ok(false),
    };
    if !(1..=12).contains(&m) || !(1..=9999).contains(&y) {
        return Ok(false);
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let max_day = match m {
        2 => if leap { 29 } else { 28 },
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    Ok((1..=max_day).contains(&d))
}

/// Normalized Levenshtein similarity (1.0 = identical).
fn similarity(a: &str, b: &str) -> f64 {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            curr[j + 1] = (prev[j] + cost).min(prev[j + 1] + 1).min(curr[j] + 1);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    1.0 - prev[b.len()] as f64 / a.len().max(b.len()) as f64
}

/// Naive detection by stopwords (pt/en/es).
/// ponytail: a simple counting heuristic; swap for a detection library if accuracy matters
fn detect_language(text: &str) -> &'static str {
    const PT: &[&str] = &["de", "que", "não", "uma", "para", "com", "os", "das", "dos", "isso", "são", "é"];
    const EN: &[&str] = &["the", "of", "and", "to", "in", "is", "that", "it", "was", "for", "with", "are"];
    const ES: &[&str] = &["el", "la", "los", "las", "una", "por", "del", "es", "en", "y", "que", "para"];
    let lower = text.to_lowercase();
    let ws: Vec<&str> = lower.split_whitespace().collect();
    let score = |stop: &[&str]| ws.iter().filter(|w| stop.contains(&w.trim_matches(|c: char| !c.is_alphanumeric()))).count();
    let (pt, en, es) = (score(PT), score(EN), score(ES));
    let max = pt.max(en).max(es);
    if max == 0 {
        "unknown"
    } else if pt == max {
        "pt"
    } else if en == max {
        "en"
    } else {
        "es"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levenshtein_similarity() {
        assert_eq!(similarity("abc", "abc"), 1.0);
        assert_eq!(similarity("", ""), 1.0);
        assert!(similarity("paracetamol", "paracetamoI") > 0.9);
        assert!(similarity("abc", "xyz") < 0.01);
    }

    #[test]
    fn normalization_and_words() {
        assert_eq!(normalize("  Olá   MUNDO \n novo "), "olá mundo novo");
        assert_eq!(words("Olá, mundo! (teste)"), vec!["Olá", "mundo", "teste"]);
    }

    #[test]
    fn cpf() {
        assert!(cpf_valid("529.982.247-25")); // classic valid one
        assert!(cpf_valid("52998224725"));
        assert!(!cpf_valid("529.982.247-26")); // wrong check digit
        assert!(!cpf_valid("111.111.111-11")); // todos iguais
        assert!(!cpf_valid("123"));
    }

    #[test]
    fn cnpj() {
        assert!(cnpj_valid("11.222.333/0001-81")); // classic valid one
        assert!(cnpj_valid("11222333000181"));
        assert!(!cnpj_valid("11.222.333/0001-82"));
        assert!(!cnpj_valid("00.000.000/0000-00"));
    }

    #[test]
    fn dates() {
        assert!(date_valid("29/02/2024", None).unwrap()); // bissexto
        assert!(!date_valid("29/02/2023", None).unwrap());
        assert!(!date_valid("31/04/2026", None).unwrap()); // abril tem 30
        assert!(date_valid("2026-08-02", None).unwrap());
        assert!(date_valid("02/08/2026", Some("dd/mm/aaaa")).unwrap());
        assert!(!date_valid("2026-08-02", Some("dd/mm/aaaa")).unwrap());
        assert!(!date_valid("agosto de 2026", None).unwrap());
        assert!(date_valid("bla", Some("xx")).is_err());
    }

    #[test]
    fn languages() {
        assert_eq!(detect_language("o contrato é válido e não pode ser alterado para os fins"), "pt");
        assert_eq!(detect_language("the quick brown fox is in the yard with the dog"), "en");
        assert_eq!(detect_language("xyzzy 123"), "unknown");
    }
}
