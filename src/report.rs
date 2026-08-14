//! Diagnostics, the JSON report and exit codes.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A finding's identity: what it is about, not where it landed in the run.
///
/// Derived from the check name and the message, so inserting a check above a
/// finding no longer renames it — which is what a positional counter did, and
/// what made an approved baseline impossible to keep.
///
/// `line` is deliberately excluded: editing the script above a check shifts it
/// without the finding itself changing.
///
/// `occurrence` disambiguates the honest collision — the same check failing
/// twice with the identical message, one page per failure. Without it the two
/// share an identity and a baseline that approves the first silences the second.
pub fn fingerprint(check_name: &str, message: &str, occurrence: u32) -> String {
    let mut h = Sha256::new();
    // A unit separator, so ("ab", "c") and ("a", "bc") cannot collide.
    h.update(check_name.as_bytes());
    h.update([0x1f]);
    h.update(message.as_bytes());
    h.update([0x1f]);
    h.update(occurrence.to_string().as_bytes());
    format!("PDFL-{:.8}", hex(&h.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub id: String,
    pub severity: Severity,
    pub check_name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub line: Option<usize>,
}

/// Bumped only when a consumer that parsed the previous output would break —
/// a field changing meaning, a value changing shape. Adding a field does not
/// bump it, because a reader that ignores unknown fields survives that.
pub const SCHEMA_VERSION: u32 = 1;

/// Deserialize as well as Serialize: `watch` has its cases analysed by child
/// processes, which speak JSON, and renders every other format from what comes
/// back. The round trip is lossless — every field is in the JSON — so a report
/// rendered from a child is the one the same code would have produced in place.
#[derive(Debug, Serialize, Deserialize)]
pub struct Report {
    /// First key in the JSON, so a consumer can branch on it before parsing
    /// anything else.
    pub schema_version: u32,
    pub script_name: String,
    pub input_file: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub profile: Option<String>,
    pub status: String, // PASS | FAIL
    pub total_pages_analyzed: i64,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// The checks and rules that ran, in order. The diagnostics only name the
    /// ones that found something, so without this a format that counts tests
    /// cannot tell a clean run from an empty one.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub checks_run: Vec<String>,
    /// Applied fix:: operations (the `pdfl fix` command).
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub fixes: Vec<String>,
    /// Overall similarity 0–100 (the `pdfl compare` command).
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub similarity: Option<f64>,
}

impl Report {
    pub fn new(
        script_name: String,
        input_file: String,
        profile: Option<String>,
        total_pages_analyzed: i64,
        diagnostics: Vec<Diagnostic>,
    ) -> Self {
        let count = |s: Severity| diagnostics.iter().filter(|d| d.severity == s).count();
        let error_count = count(Severity::Error);
        let warning_count = count(Severity::Warning);
        let info_count = count(Severity::Info);
        Report {
            schema_version: SCHEMA_VERSION,
            script_name,
            input_file,
            profile,
            status: if error_count == 0 { "PASS" } else { "FAIL" }.into(),
            total_pages_analyzed,
            error_count,
            warning_count,
            info_count,
            diagnostics,
            checks_run: Vec::new(),
            fixes: Vec::new(),
            similarity: None,
        }
    }

    /// Exit codes: 0 = OK, 1 = warnings, 2 = errors.
    /// With --fail-on warning, warnings also drop it to 2.
    pub fn exit_code(&self, fail_on_warning: bool) -> i32 {
        if self.error_count > 0 || (fail_on_warning && self.warning_count > 0) {
            2
        } else if self.warning_count > 0 {
            1
        } else {
            0
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("report is always serializable")
    }

    /// CSV: one line per diagnostic (the header is always present).
    pub fn to_csv(&self) -> String {
        let mut out = String::from("id,severity,check,message,line,script,file,status\n");
        for d in &self.diagnostics {
            let sev = match d.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            let line = d.line.map(|l| l.to_string()).unwrap_or_default();
            for (i, field) in [
                d.id.as_str(),
                sev,
                d.check_name.as_str(),
                d.message.as_str(),
                line.as_str(),
                self.script_name.as_str(),
                self.input_file.as_str(),
                self.status.as_str(),
            ]
            .iter()
            .enumerate()
            {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&csv_field(field));
            }
            out.push('\n');
        }
        out
    }

    /// Self-contained HTML (inline CSS), in English.
    pub fn to_html(&self) -> String {
        let e = html_escape;
        let status_color = if self.status == "PASS" { "#1a7f37" } else { "#c0392b" };
        let mut rows = String::new();
        for d in &self.diagnostics {
            let (sev_label, sev_color) = match d.severity {
                Severity::Error => ("error", "#c0392b"),
                Severity::Warning => ("warning", "#b7791f"),
                Severity::Info => ("info", "#2b6cb0"),
            };
            let line = d.line.map(|l| l.to_string()).unwrap_or_else(|| "—".into());
            rows.push_str(&format!(
                "<tr><td>{}</td><td><span style=\"color:{sev_color};font-weight:600\">{sev_label}</span></td>\
                 <td>{}</td><td>{}</td><td>{line}</td></tr>\n",
                e(&d.id),
                e(&d.check_name),
                e(&d.message),
            ));
        }
        if rows.is_empty() {
            rows = "<tr><td colspan=\"5\" style=\"text-align:center;color:#666\">\
                    No problems found</td></tr>"
                .into();
        }
        let fixes = if self.fixes.is_empty() {
            String::new()
        } else {
            let items: String =
                self.fixes.iter().map(|f| format!("<li>{}</li>\n", e(f))).collect();
            format!("<h2>Applied fixes</h2>\n<ul>{items}</ul>\n")
        };
        let profile = self
            .profile
            .as_ref()
            .map(|p| format!("<p><strong>Profile:</strong> {}</p>\n", e(p)))
            .unwrap_or_default();
        let similarity = self
            .similarity
            .map(|s| format!("<p><strong>Similarity:</strong> {s}%</p>\n"))
            .unwrap_or_default();

        format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>PDFL Report — {script}</title>
<style>
body {{ font-family: system-ui, sans-serif; margin: 2rem auto; max-width: 60rem; color: #222; }}
table {{ border-collapse: collapse; width: 100%; margin-top: 1rem; }}
th, td {{ border: 1px solid #ddd; padding: 0.5rem 0.75rem; text-align: left; vertical-align: top; }}
th {{ background: #f5f5f5; }}
.status {{ display: inline-block; padding: 0.2rem 0.8rem; border-radius: 4px; color: #fff;
          font-weight: 700; background: {status_color}; }}
.resumo span {{ margin-right: 1.5rem; }}
</style>
</head>
<body>
<h1>Validation Report</h1>
<p><strong>Script:</strong> {script} &nbsp;|&nbsp; <strong>File:</strong> {input}</p>
{profile}{similarity}<p><span class="status">{status}</span></p>
<p class="resumo">
<span><strong>{errors}</strong> error(s)</span>
<span><strong>{warnings}</strong> warning(s)</span>
<span><strong>{infos}</strong> info(s)</span>
<span><strong>{pages}</strong> page(s) analyzed</span>
</p>
{fixes}<h2>Diagnostics</h2>
<table>
<thead><tr><th>ID</th><th>Severity</th><th>Check</th><th>Message</th><th>Line</th></tr></thead>
<tbody>
{rows}</tbody>
</table>
</body>
</html>
"#,
            script = e(&self.script_name),
            input = e(&self.input_file),
            status = e(&self.status),
            errors = self.error_count,
            warnings = self.warning_count,
            infos = self.info_count,
            pages = self.total_pages_analyzed,
        )
    }
}

// ---- SARIF and JUnit ----

impl Report {
    fn severity_label(severity: &Severity) -> &'static str {
        match severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }

    /// SARIF 2.1.0, the format GitHub code scanning reads.
    ///
    /// A result is anchored on the script, not on the PDF: the line we know is
    /// the line of the check, and the file under validation is usually an
    /// artifact passing through CI rather than something in the repository, so
    /// anchoring there would annotate a path that does not exist. The input
    /// file travels in `properties` instead, where it stays visible.
    ///
    /// The diagnostic id goes in `partialFingerprints`, which is what lets a
    /// consumer recognise the same finding across runs — the reason the id is
    /// derived from the finding rather than counted.
    pub fn to_sarif(&self) -> String {
        use serde_json::json;

        // A rule per distinct check, in first-seen order: GitHub groups alerts
        // by rule, and an unknown ruleId shows up bare.
        let mut rule_ids: Vec<&str> = Vec::new();
        for d in &self.diagnostics {
            if !rule_ids.contains(&d.check_name.as_str()) {
                rule_ids.push(&d.check_name);
            }
        }
        let rules: Vec<_> = rule_ids
            .iter()
            .map(|id| json!({ "id": id, "shortDescription": { "text": id } }))
            .collect();

        let results: Vec<_> = self
            .diagnostics
            .iter()
            .map(|d| {
                let mut region = serde_json::Map::new();
                if let Some(l) = d.line {
                    region.insert("startLine".into(), json!(l));
                }
                json!({
                    "ruleId": d.check_name,
                    "level": match d.severity {
                        Severity::Error => "error",
                        Severity::Warning => "warning",
                        // SARIF has no "info"; "note" is its equivalent.
                        Severity::Info => "note",
                    },
                    "message": { "text": d.message },
                    "locations": [{
                        "physicalLocation": {
                            "artifactLocation": { "uri": self.script_name },
                            "region": region,
                        }
                    }],
                    "partialFingerprints": { "pdflDiagnostic/v1": d.id },
                    "properties": { "inputFile": self.input_file },
                })
            })
            .collect();

        let sarif = json!({
            "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
            "version": "2.1.0",
            "runs": [{
                "tool": { "driver": {
                    "name": "PDFLang",
                    "informationUri": "https://github.com/kotarokikuchi/pdflang",
                    "version": env!("CARGO_PKG_VERSION"),
                    "rules": rules,
                }},
                "results": results,
            }],
        });
        serde_json::to_string_pretty(&sarif).expect("sarif is always serializable")
    }

    /// JUnit XML, which every CI knows how to display.
    ///
    /// One test case per check that ran — including the ones that passed, which
    /// is why the interpreter records them. A format that reported only the
    /// failures would show a clean run as zero tests, and a CI reads that as a
    /// run that never happened.
    pub fn to_junit(&self) -> String {
        let e = xml_escape;

        // Every check that ran, plus any check that only shows up in the
        // diagnostics — a report from `fix` or `compare` has no check list, and
        // dropping those findings would be worse than an odd-looking suite.
        let mut cases: Vec<&str> = self.checks_run.iter().map(|s| s.as_str()).collect();
        for d in &self.diagnostics {
            if !cases.contains(&d.check_name.as_str()) {
                cases.push(&d.check_name);
            }
        }

        let mut body = String::new();
        let mut failures = 0;
        for case in &cases {
            let found: Vec<&Diagnostic> =
                self.diagnostics.iter().filter(|d| d.check_name == *case).collect();
            let failing: Vec<&&Diagnostic> =
                found.iter().filter(|d| d.severity != Severity::Info).collect();
            body.push_str(&format!(
                "    <testcase name=\"{}\" classname=\"{}\"",
                e(case),
                e(&self.input_file)
            ));
            if failing.is_empty() {
                // Info findings do not fail a case, but they must not vanish.
                let infos: String =
                    found.iter().map(|d| format!("{}\n", e(&describe(d)))).collect();
                if infos.is_empty() {
                    body.push_str("/>\n");
                } else {
                    body.push_str(&format!(
                        ">\n      <system-out>{infos}      </system-out>\n    </testcase>\n"
                    ));
                }
                continue;
            }
            failures += 1;
            let first = failing[0];
            // One <failure> per case: a second one is not portable, so the rest
            // of the findings go in its body rather than in siblings.
            let detail: String = found.iter().map(|d| format!("{}\n", e(&describe(d)))).collect();
            body.push_str(&format!(
                ">\n      <failure message=\"{}\" type=\"{}\">\n\
                 {detail}      </failure>\n    </testcase>\n",
                e(&first.message),
                Self::severity_label(&first.severity),
            ));
        }

        format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <testsuites name=\"pdfl\" tests=\"{n}\" failures=\"{failures}\">\n\
             \x20 <testsuite name=\"{script}\" tests=\"{n}\" failures=\"{failures}\" errors=\"0\" skipped=\"0\">\n\
             {body}  </testsuite>\n\
             </testsuites>\n",
            n = cases.len(),
            script = e(&self.script_name),
        )
    }
}

/// One finding on one line, for a format that carries text rather than fields.
fn describe(d: &Diagnostic) -> String {
    let line = d.line.map(|l| format!(" (line {l})")).unwrap_or_default();
    format!("{} [{}]{} {}", d.id, Report::severity_label(&d.severity), line, d.message)
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ---- PDF ----

impl Report {
    /// PDF report (A4, embedded Helvetica, deterministic).
    pub fn to_pdf(&self) -> Vec<u8> {
        use printpdf::*;

        const TOP: f32 = 277.0; // mm
        const BOTTOM: f32 = 20.0;
        const LEFT: f32 = 20.0;
        const WRAP: usize = 100; // characters per line (Helvetica 9pt across 170mm)

        struct Line {
            text: String,
            size: f32,
            bold: bool,
            color: (f32, f32, f32),
        }
        let black = (0.13, 0.13, 0.13);
        let gray = (0.4, 0.4, 0.4);
        let mut lines: Vec<Line> = Vec::new();
        let mut push = |text: &str, size: f32, bold: bool, color: (f32, f32, f32)| {
            lines.push(Line { text: pdf_sanitize(text), size, bold, color });
        };

        push("Validation Report", 18.0, true, black);
        push("", 6.0, false, black);
        push(&format!("Script: {}   File: {}", self.script_name, self.input_file), 10.0, false, gray);
        if let Some(p) = &self.profile {
            push(&format!("Profile: {p}"), 10.0, false, gray);
        }
        if let Some(s) = self.similarity {
            push(&format!("Similarity: {s}%"), 10.0, false, gray);
        }
        let status_color = if self.status == "PASS" { (0.1, 0.5, 0.22) } else { (0.75, 0.22, 0.17) };
        push(&format!("Status: {}", self.status), 13.0, true, status_color);
        push(
            &format!(
                "{} error(s), {} warning(s), {} info(s) — {} page(s) analyzed",
                self.error_count, self.warning_count, self.info_count, self.total_pages_analyzed
            ),
            10.0,
            false,
            black,
        );

        if !self.fixes.is_empty() {
            push("", 6.0, false, black);
            push("Applied fixes", 13.0, true, black);
            for f in &self.fixes {
                push(&format!("  - {f}"), 10.0, false, black);
            }
        }

        push("", 6.0, false, black);
        push("Diagnostics", 13.0, true, black);
        if self.diagnostics.is_empty() {
            push("No problems found.", 10.0, false, gray);
        }
        for d in &self.diagnostics {
            let (label, color) = match d.severity {
                Severity::Error => ("error", (0.75, 0.22, 0.17)),
                Severity::Warning => ("warning", (0.72, 0.47, 0.12)),
                Severity::Info => ("info", (0.17, 0.42, 0.69)),
            };
            let line_info = d.line.map(|l| format!(" (line {l})")).unwrap_or_default();
            push("", 3.0, false, black);
            push(&format!("{} [{}] {}{}", d.id, label, d.check_name, line_info), 10.0, true, color);
            for chunk in wrap_text(&d.message, WRAP) {
                push(&format!("    {chunk}"), 9.0, false, black);
            }
        }

        // Pagination: one cursor at the top per page; each line moves down via
        // SetLineHeight + AddLineBreak (printpdf's SetTextCursor is relative).
        let start_page = || vec![Op::StartTextSection, Op::SetTextCursor { pos: Point::new(Mm(LEFT), Mm(TOP)) }];
        let mut pages: Vec<PdfPage> = Vec::new();
        let mut ops: Vec<Op> = start_page();
        let mut y = TOP;
        for line in &lines {
            let advance_pt = line.size * 1.2 + 3.0;
            let advance_mm = advance_pt * 0.3528;
            if y - advance_mm < BOTTOM {
                ops.push(Op::EndTextSection);
                pages.push(PdfPage::new(Mm(210.0), Mm(297.0), std::mem::take(&mut ops)));
                ops = start_page();
                y = TOP;
            }
            y -= advance_mm;
            ops.push(Op::SetLineHeight { lh: Pt(advance_pt) });
            ops.push(Op::AddLineBreak);
            if line.text.is_empty() {
                continue;
            }
            let font = if line.bold { BuiltinFont::HelveticaBold } else { BuiltinFont::Helvetica };
            ops.push(Op::SetFont { font: PdfFontHandle::Builtin(font), size: Pt(line.size) });
            ops.push(Op::SetFillColor {
                col: Color::Rgb(Rgb { r: line.color.0, g: line.color.1, b: line.color.2, icc_profile: None }),
            });
            ops.push(Op::ShowText { items: vec![TextItem::Text(line.text.clone())] });
        }
        ops.push(Op::EndTextSection);
        pages.push(PdfPage::new(Mm(210.0), Mm(297.0), ops));

        PdfDocument::new("PDFL Report")
            .with_pages(pages)
            .save(&PdfSaveOptions::default(), &mut Vec::new())
    }
}

/// Embedded fonts use WinAnsi (Latin-1): replaces whatever does not fit.
fn pdf_sanitize(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '→' => "->".chars().collect::<Vec<_>>(),
            '—' | '–' => vec!['-'],
            '…' => "...".chars().collect(),
            '\u{201C}' | '\u{201D}' => vec!['"'],
            '\u{2018}' | '\u{2019}' => vec!['\''],
            c if (c as u32) < 256 => vec![c],
            _ => vec!['?'],
        })
        .collect()
}

/// Wraps on word boundaries into lines of at most `width` characters.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() || out.is_empty() {
        out.push(line);
    }
    out
}

/// Escapes a CSV field (doubled quotes; wraps it if it has , " or a newline).
fn csv_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(sev: Severity) -> Diagnostic {
        Diagnostic {
            id: "PDFL-001".into(),
            severity: sev,
            check_name: "c".into(),
            message: "m".into(),
            line: None,
        }
    }

    /// The PDF report was the one format nothing exercised — no unit test and
    /// no CI step — while being the only one that writes a binary a person
    /// then has to open.
    #[test]
    fn pdf_is_a_pdf_and_carries_the_findings() {
        let mut d = diag(Severity::Error);
        d.message = "page 3 is missing from reprint.pdf".into();
        let r = Report::new("profile.pdfl".into(), "doc.pdf".into(), None, 3, vec![d]);
        let bytes = r.to_pdf();

        assert!(bytes.starts_with(b"%PDF-"), "must be a PDF");
        assert!(bytes.ends_with(b"%%EOF\n") || bytes.ends_with(b"%%EOF"), "must be terminated");
        assert!(bytes.len() > 500, "a report with a finding is not 500 bytes: {}", bytes.len());
    }

    /// A finding is written into the page as a literal, and printpdf builds
    /// that from whatever we hand it. Anything outside Latin-1 has to be
    /// folded first — a report is not the place to discover an encoding.
    #[test]
    fn pdf_folds_what_it_cannot_encode() {
        assert_eq!(pdf_sanitize("a → b"), "a -> b");
        assert_eq!(pdf_sanitize("em—dash"), "em-dash");
        assert_eq!(pdf_sanitize("and so…"), "and so...");
        assert_eq!(pdf_sanitize("\u{201C}quoted\u{201D}"), "\"quoted\"");
        assert_eq!(pdf_sanitize("it\u{2019}s"), "it's");
        // Accented Latin-1 survives; anything above it becomes a placeholder
        // rather than a broken glyph or a panic.
        assert_eq!(pdf_sanitize("reimpressão"), "reimpressão");
        assert_eq!(pdf_sanitize("見本"), "??");

        // And the whole path holds with such a message in it.
        let mut d = diag(Severity::Error);
        d.message = "見本 → \u{201C}proof\u{201D}".into();
        let bytes = Report::new("p.pdfl".into(), "doc.pdf".into(), None, 1, vec![d]).to_pdf();
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn json_declares_its_schema_version() {
        let r = Report::new("s.pdfl".into(), "f.pdf".into(), None, 1, vec![diag(Severity::Error)]);
        let json = r.to_json();
        // First key, so a consumer can branch on it without parsing the rest.
        assert!(json.starts_with("{\n  \"schema_version\": 1"), "{json}");
    }

    #[test]
    fn csv_escapes_fields() {
        let mut d = diag(Severity::Error);
        d.message = "message with \"quotes\", comma\nand a newline".into();
        let r = Report::new("s.pdfl".into(), "f.pdf".into(), None, 1, vec![d]);
        let csv = r.to_csv();
        assert!(csv.starts_with("id,severity,check,message,line,script,file,status\n"));
        assert!(csv.contains("\"message with \"\"quotes\"\", comma\nand a newline\""));
    }

    #[test]
    fn html_escapes_and_summarizes() {
        let mut d = diag(Severity::Warning);
        d.message = "size <6pt> & co".into();
        let r = Report::new("s.pdfl".into(), "f.pdf".into(), Some("perfil-x".into()), 3, vec![d]);
        let html = r.to_html();
        assert!(html.contains("size &lt;6pt&gt; &amp; co"));
        assert!(html.contains("perfil-x"));
        assert!(html.contains("PASS")); // a warning does not drop the status
        assert!(html.contains("<strong>1</strong> warning(s)"));
    }

    #[test]
    fn sarif_carries_the_fingerprint_and_the_rule() {
        let mut d = diag(Severity::Warning);
        d.id = "PDFL-093751a2".into();
        d.check_name = "Ink".into();
        d.line = Some(12);
        let r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, vec![d]);
        let v: serde_json::Value = serde_json::from_str(&r.to_sarif()).unwrap();
        assert_eq!(v["version"], "2.1.0");
        let run = &v["runs"][0];
        assert_eq!(run["tool"]["driver"]["rules"][0]["id"], "Ink");
        let res = &run["results"][0];
        assert_eq!(res["ruleId"], "Ink");
        assert_eq!(res["level"], "warning");
        // The id is what lets a consumer match a finding across runs.
        assert_eq!(res["partialFingerprints"]["pdflDiagnostic/v1"], "PDFL-093751a2");
        // Anchored on the script, because the PDF is not a file the CI can annotate.
        let loc = &res["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "p.pdfl");
        assert_eq!(loc["region"]["startLine"], 12);
        assert_eq!(res["properties"]["inputFile"], "f.pdf");
    }

    #[test]
    fn sarif_maps_info_to_note() {
        let r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, vec![diag(Severity::Info)]);
        let v: serde_json::Value = serde_json::from_str(&r.to_sarif()).unwrap();
        assert_eq!(v["runs"][0]["results"][0]["level"], "note");
    }

    /// A clean run must still count its tests. Reporting zero tests is how a CI
    /// concludes the run never happened.
    #[test]
    fn junit_counts_the_checks_that_passed() {
        let mut d = diag(Severity::Error);
        d.check_name = "Ink".into();
        d.message = "TAC above 300% & rising".into();
        let mut r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, vec![d]);
        r.checks_run = vec!["Ink".into(), "Fonts".into(), "Bleed".into()];
        let xml = r.to_junit();
        assert!(xml.contains("tests=\"3\" failures=\"1\""), "{xml}");
        // The passing ones are self-closing cases; the failing one carries the message.
        assert!(xml.contains("<testcase name=\"Fonts\" classname=\"f.pdf\"/>"), "{xml}");
        assert!(xml.contains("message=\"TAC above 300% &amp; rising\" type=\"error\""), "{xml}");
    }

    /// `fix` and `compare` build a report without ever running a check. Their
    /// findings still have to appear.
    #[test]
    fn junit_keeps_findings_that_have_no_check() {
        let r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, vec![diag(Severity::Error)]);
        let xml = r.to_junit();
        assert!(xml.contains("tests=\"1\" failures=\"1\""), "{xml}");
        assert!(xml.contains("<testcase name=\"c\""), "{xml}");
    }

    #[test]
    fn junit_does_not_fail_on_info() {
        let mut r = Report::new("p.pdfl".into(), "f.pdf".into(), None, 1, vec![diag(Severity::Info)]);
        r.checks_run = vec!["c".into()];
        let xml = r.to_junit();
        assert!(xml.contains("failures=\"0\""), "{xml}");
        // An info finding does not fail the case, but it is not dropped either.
        assert!(xml.contains("<system-out>"), "{xml}");
    }

    #[test]
    fn exit_codes() {
        let pass = Report::new("s".into(), "f".into(), None, 1, vec![]);
        assert_eq!(pass.status, "PASS");
        assert_eq!(pass.exit_code(false), 0);

        let warn = Report::new("s".into(), "f".into(), None, 1, vec![diag(Severity::Warning)]);
        assert_eq!(warn.status, "PASS");
        assert_eq!(warn.exit_code(false), 1);
        assert_eq!(warn.exit_code(true), 2);

        let err = Report::new("s".into(), "f".into(), None, 1, vec![diag(Severity::Error)]);
        assert_eq!(err.status, "FAIL");
        assert_eq!(err.exit_code(false), 2);
    }
}
