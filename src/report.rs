//! Diagnostics, the JSON report and exit codes.

use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, PartialEq)]
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

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub id: String,
    pub severity: Severity,
    pub check_name: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// Bumped only when a consumer that parsed the previous output would break —
/// a field changing meaning, a value changing shape. Adding a field does not
/// bump it, because a reader that ignores unknown fields survives that.
pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Serialize)]
pub struct Report {
    /// First key in the JSON, so a consumer can branch on it before parsing
    /// anything else.
    pub schema_version: u32,
    pub script_name: String,
    pub input_file: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    pub status: String, // PASS | FAIL
    pub total_pages_analyzed: i64,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    /// Applied fix:: operations (the `pdfl fix` command).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fixes: Vec<String>,
    /// Overall similarity 0–100 (the `pdfl compare` command).
    #[serde(skip_serializing_if = "Option::is_none")]
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

// ---- PDF ----

impl Report {
    /// PDF report (A4, embedded Helvetica, deterministic).
    pub fn to_pdf(&self) -> Vec<u8> {
        use printpdf::*;

        const TOP: f32 = 277.0; // mm
        const BOTTOM: f32 = 20.0;
        const LEFT: f32 = 20.0;
        const WRAP: usize = 100; // caracteres por linha (Helvetica 9pt em 170mm)

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
