//! `pdfl inspect` command — quick summary of a PDF.
//!
//! The summary is built once as data and rendered from there, so the text a
//! person reads and the JSON a script parses cannot drift apart.

use crate::interpreter::DocData;
use serde::Serialize;

#[derive(Serialize)]
pub struct Summary {
    pub file: String,
    pub size_bytes: i64,
    pub sha256: String,
    pub pages: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<PageSize>,
    /// Boxes present on every page. MediaBox is there by definition.
    pub boxes: Vec<String>,
    /// Non-empty metadata entries, in the PDF's own order.
    pub metadata: Vec<MetadataEntry>,
    pub fonts: Vec<Font>,
    pub images: Images,
    /// Highest per-page estimate, from an RGB render — a lower bound on the
    /// real value. `prepress::calculate_exact_tac` is the trustworthy one.
    pub max_estimated_tac: f64,
    pub warnings: Vec<String>,
}

#[derive(Serialize)]
pub struct PageSize {
    pub width: f64,
    pub height: f64,
    /// False when the pages are not all the same size, which makes the width
    /// and height above the first page's rather than the document's.
    pub uniform: bool,
}

#[derive(Serialize)]
pub struct MetadataEntry {
    pub key: String,
    pub value: String,
}

#[derive(Serialize)]
pub struct Font {
    pub name: String,
    pub embedded: bool,
}

#[derive(Serialize)]
pub struct Images {
    pub count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_dpi: Option<f64>,
    pub color_spaces: Vec<String>,
}

pub fn summarize(doc: &DocData) -> Summary {
    let page_size = doc.pages.first().map(|first| PageSize {
        width: first.width,
        height: first.height,
        uniform: doc.pages.iter().all(|p| p.width == first.width && p.height == first.height),
    });

    let optional = [
        ("TrimBox", doc.pages.iter().all(|p| p.boxes.trim.is_some())),
        ("BleedBox", doc.pages.iter().all(|p| p.boxes.bleed.is_some())),
        ("CropBox", doc.pages.iter().all(|p| p.boxes.crop.is_some())),
    ];
    let mut boxes = vec!["MediaBox".to_string()];
    boxes.extend(optional.iter().filter(|(_, has)| *has).map(|(n, _)| n.to_string()));

    let images: Vec<_> = doc.pages.iter().flat_map(|p| p.images.iter()).collect();
    let mut color_spaces: Vec<String> = images.iter().map(|i| i.color_space.clone()).collect();
    color_spaces.sort_unstable();
    color_spaces.dedup();

    let mut warnings = Vec::new();
    if doc.fonts.iter().any(|f| !f.is_embedded) {
        warnings.push("there are non-embedded fonts".to_string());
    }
    if !images.is_empty() {
        let low = images.iter().filter(|i| i.dpi_x.min(i.dpi_y) < 300.0).count();
        if low > 0 {
            warnings.push(format!("{low} image(s) below 300 DPI"));
        }
        if images.iter().any(|i| i.color_space.contains("RGB")) {
            warnings.push("there are RGB images (offset printing requires CMYK)".into());
        }
    }
    if doc.pages.iter().all(|p| p.text.trim().is_empty()) {
        warnings.push("the document has no extractable text".into());
    }
    if doc.pages.iter().any(|p| p.boxes.trim.is_none()) {
        warnings.push("TrimBox missing on some page".into());
    }
    if doc.title.is_empty() {
        warnings.push("no title in the metadata".into());
    }

    Summary {
        file: doc.filename.clone(),
        size_bytes: doc.file_size,
        sha256: doc.sha256.clone(),
        pages: doc.pages.len(),
        page_size,
        boxes,
        metadata: doc
            .metadata
            .iter()
            .filter(|(_, v)| !v.is_empty())
            .map(|(k, v)| MetadataEntry { key: k.clone(), value: v.clone() })
            .collect(),
        fonts: doc
            .fonts
            .iter()
            .map(|f| Font { name: f.name.clone(), embedded: f.is_embedded })
            .collect(),
        images: Images {
            count: images.len(),
            min_dpi: images
                .iter()
                .map(|i| i.dpi_x.min(i.dpi_y))
                .fold(None, |acc: Option<f64>, d| Some(acc.map_or(d, |a| a.min(d)))),
            color_spaces,
        },
        max_estimated_tac: doc.pages.iter().map(|p| p.tac_max).fold(0.0, f64::max),
        warnings,
    }
}

pub fn to_json(s: &Summary) -> String {
    serde_json::to_string_pretty(s).unwrap_or_else(|e| format!("{{\"error\": \"{e}\"}}"))
}

pub fn to_text(s: &Summary) -> String {
    let mut out = String::new();
    let mut w = |line: String| {
        out.push_str(&line);
        out.push('\n');
    };

    w(format!("File:     {}", s.file));
    w(format!("Size:     {} KB ({} bytes)", s.size_bytes / 1024, s.size_bytes));
    w(format!("SHA-256:  {}", s.sha256));
    w(String::new());

    w(format!("Pages:    {}", s.pages));
    if let Some(size) = &s.page_size {
        w(format!(
            "Page size: {:.0} x {:.0} pt{}",
            size.width,
            size.height,
            if size.uniform { "" } else { " (varies between pages)" }
        ));
        w(format!("Boxes:    {}", s.boxes.join(", ")));
    }
    w(String::new());

    let meta: Vec<String> = s.metadata.iter().map(|m| format!("  {}: {}", m.key, m.value)).collect();
    w(format!(
        "Metadata: {}",
        if meta.is_empty() { "(none)".into() } else { format!("\n{}", meta.join("\n")) }
    ));
    w(String::new());

    if s.fonts.is_empty() {
        w("Fonts:    (none)".into());
    } else {
        w(format!("Fonts:    {}", s.fonts.len()));
        for f in &s.fonts {
            w(format!("  {} — {}", f.name, if f.embedded { "embedded" } else { "NOT embedded" }));
        }
    }

    if s.images.count == 0 {
        w("Images:   (none)".into());
    } else {
        w(format!(
            "Images:   {} (minimum DPI {:.0}, spaces: {})",
            s.images.count,
            s.images.min_dpi.unwrap_or(0.0),
            s.images.color_spaces.join(", ")
        ));
    }

    w(format!("Max. estimated TAC: {:.0}% (RGB render approximation)", s.max_estimated_tac));
    w(String::new());

    if s.warnings.is_empty() {
        w("Warnings: none".into());
    } else {
        w("Warnings:".into());
        for warn in &s.warnings {
            w(format!("  ! {warn}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interpreter::{FontData, PageBoxes, PageData};
    use std::rc::Rc;

    fn doc() -> DocData {
        DocData {
            filename: "t.pdf".into(),
            title: String::new(),
            author: String::new(),
            pages: vec![Rc::new(PageData {
                index: 0,
                width: 595.0,
                height: 842.0,
                text: "hello".into(),
                images: vec![],
                boxes: PageBoxes { media: Some([0.0, 0.0, 595.0, 842.0]), crop: None, trim: None, bleed: None, art: None },
                tac_max: 0.0,
                ink_avg: 0.0,
                min_stroke_pt: None,
            })],
            fonts: vec![Rc::new(FontData { name: "Arial".into(), is_embedded: false })],
            metadata: vec![("Title".into(), String::new()), ("Producer".into(), "T".into())],
            file_size: 2048,
            sha256: "abc".into(),
            object_count: 0,
            path: "t.pdf".into(),
            barcodes: Default::default(),
            lowlevel: Default::default(),
            colors: Default::default(),
        }
    }

    /// The two renderings read the same summary, so this pins the thing that
    /// would let them drift: a field present in one and missing from the other.
    #[test]
    fn text_and_json_report_the_same_facts() {
        let s = summarize(&doc());
        let text = to_text(&s);
        let json = to_json(&s);

        assert!(text.contains("Arial"), "{text}");
        assert!(json.contains("Arial"), "{json}");
        // Empty metadata values are dropped from both.
        assert!(!json.contains("\"Title\""), "{json}");
        assert!(text.contains("Producer: T"), "{text}");
        // A missing TrimBox is a warning in both.
        assert!(text.contains("TrimBox missing"), "{text}");
        assert!(json.contains("TrimBox missing"), "{json}");
    }
}
