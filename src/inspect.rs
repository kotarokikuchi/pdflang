//! `pdfl inspect` command — quick summary of a PDF.

use crate::interpreter::DocData;

pub fn inspect(doc: &DocData) -> String {
    let mut out = String::new();
    let mut w = |s: String| {
        out.push_str(&s);
        out.push('\n');
    };

    w(format!("File:     {}", doc.filename));
    w(format!("Size:     {} KB ({} bytes)", doc.file_size / 1024, doc.file_size));
    w(format!("SHA-256:  {}", doc.sha256));
    w(String::new());

    // Pages
    w(format!("Pages:    {}", doc.pages.len()));
    if let Some(first) = doc.pages.first() {
        let uniform = doc.pages.iter().all(|p| p.width == first.width && p.height == first.height);
        let dims = format!("{:.0} x {:.0} pt", first.width, first.height);
        w(format!("Page size: {dims}{}", if uniform { "" } else { " (varies between pages)" }));
        let boxes = [
            ("TrimBox", doc.pages.iter().all(|p| p.boxes.trim.is_some())),
            ("BleedBox", doc.pages.iter().all(|p| p.boxes.bleed.is_some())),
            ("CropBox", doc.pages.iter().all(|p| p.boxes.crop.is_some())),
        ];
        let present: Vec<&str> = boxes.iter().filter(|(_, has)| *has).map(|(n, _)| *n).collect();
        w(format!(
            "Boxes:    MediaBox{}",
            if present.is_empty() { String::new() } else { format!(", {}", present.join(", ")) }
        ));
    }
    w(String::new());

    // Non-empty metadata
    let meta: Vec<String> =
        doc.metadata.iter().filter(|(_, v)| !v.is_empty()).map(|(k, v)| format!("  {k}: {v}")).collect();
    w(format!("Metadata: {}", if meta.is_empty() { "(none)".into() } else { format!("\n{}", meta.join("\n")) }));
    w(String::new());

    // Fonts
    if doc.fonts.is_empty() {
        w("Fonts:    (none)".into());
    } else {
        w(format!("Fonts:    {}", doc.fonts.len()));
        for f in &doc.fonts {
            w(format!("  {} — {}", f.name, if f.is_embedded { "embedded" } else { "NOT embedded" }));
        }
    }

    // Images
    let images: Vec<_> = doc.pages.iter().flat_map(|p| p.images.iter()).collect();
    if images.is_empty() {
        w("Images:   (none)".into());
    } else {
        let min_dpi = images.iter().map(|i| i.dpi_x.min(i.dpi_y)).fold(f64::INFINITY, f64::min);
        let mut spaces: Vec<&str> = images.iter().map(|i| i.color_space.as_str()).collect();
        spaces.sort_unstable();
        spaces.dedup();
        w(format!("Images:   {} (minimum DPI {:.0}, spaces: {})", images.len(), min_dpi, spaces.join(", ")));
    }

    // Estimated TAC
    let tac = doc.pages.iter().map(|p| p.tac_max).fold(0.0, f64::max);
    w(format!("Max. estimated TAC: {tac:.0}% (RGB render approximation)"));
    w(String::new());

    // General warnings
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
    if warnings.is_empty() {
        w("Warnings: none".into());
    } else {
        w("Warnings:".into());
        for warn in warnings {
            w(format!("  ! {warn}"));
        }
    }
    out
}
