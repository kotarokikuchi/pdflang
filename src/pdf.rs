//! PDF loading via pdfium-render → the interpreter's structures.

use crate::interpreter::{BarcodeData, DocData, FontData, ImageData, PageBoxes, PageData};
use anyhow::{Context, Result};
use pdfium_render::prelude::*;
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

/// Locates libpdfium: ./pdfium/ (from setup_pdfium.sh), next to the executable
/// (release package) and finally the system's one.
///
/// `bin` is checked because the Windows build of pdfium-binaries ships
/// pdfium.dll there — `lib/` holds only the import library, which cannot be
/// loaded at runtime.
fn bind_pdfium() -> Result<Pdfium> {
    let mut candidates = vec![
        std::path::PathBuf::from("./pdfium/lib/"),
        std::path::PathBuf::from("./pdfium/bin/"),
        std::path::PathBuf::from("./pdfium/"),
    ];
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        candidates.push(exe_dir.join("pdfium/lib"));
        candidates.push(exe_dir.join("pdfium/bin"));
        candidates.push(exe_dir.join("pdfium"));
    }
    for dir in &candidates {
        if let Ok(b) = Pdfium::bind_to_library(Pdfium::pdfium_platform_library_name_at_path(dir)) {
            return Ok(Pdfium::new(b));
        }
    }
    let bindings = Pdfium::bind_to_system_library().context(
        "pdfium library not found — run ./setup_pdfium.sh to download it",
    )?;
    Ok(Pdfium::new(bindings))
}

/// Applies the fix:: operations in sequence and saves the result.
/// Returns the description of each applied operation.
pub fn apply_fixes(
    input: &Path,
    ops: &[crate::fixns::FixOp],
    output: &Path,
) -> Result<Vec<String>> {
    use crate::fixns::FixOp;

    let pdfium = bind_pdfium()?;
    let mut document = pdfium
        .load_pdf_from_file(input, None)
        .with_context(|| format!("could not reopen PDF: {}", input.display()))?;
    let mut applied = Vec::new();

    // Scratch area for duplicating/reordering (imports pages from a snapshot).
    let temp = std::env::temp_dir().join(format!("pdfl-fix-{}.pdf", std::process::id()));

    for op in ops {
        match op {
            FixOp::SetPageSize { width, height } => {
                let rect = points_rect([0.0, 0.0, *width, *height]);
                for mut page in document.pages_mut().iter() {
                    page.boundaries_mut().set_media(rect)?;
                }
            }
            FixOp::SetCropBox { rect } => {
                for mut page in document.pages_mut().iter() {
                    page.boundaries_mut().set_crop(points_rect(*rect))?;
                }
            }
            FixOp::SetTrimBox { rect } => {
                for mut page in document.pages_mut().iter() {
                    page.boundaries_mut().set_trim(points_rect(*rect))?;
                }
            }
            FixOp::SetBleedBox { rect } => {
                for mut page in document.pages_mut().iter() {
                    page.boundaries_mut().set_bleed(points_rect(*rect))?;
                }
            }
            FixOp::RotatePage { page, degrees } => {
                let rotation = match degrees {
                    90 => PdfPageRenderRotation::Degrees90,
                    180 => PdfPageRenderRotation::Degrees180,
                    _ => PdfPageRenderRotation::Degrees270,
                };
                for (i, mut p) in document.pages_mut().iter().enumerate() {
                    if *page == 0 || i as i64 + 1 == *page {
                        p.set_rotation(rotation);
                    }
                }
            }
            FixOp::DeletePage { page } => {
                document.pages_mut().get((*page - 1) as PdfPageIndex)?.delete()?;
            }
            FixOp::DuplicatePage { page } => {
                document.save_to_file(&temp)?;
                let snapshot = pdfium.load_pdf_from_file(&temp, None)?;
                document.pages_mut().copy_pages_from_document(
                    &snapshot,
                    &page.to_string(),
                    *page as PdfPageIndex, // inserts right after the original
                )?;
            }
            FixOp::ReorderPages { order } => {
                document.save_to_file(&temp)?;
                let snapshot = pdfium.load_pdf_from_file(&temp, None)?;
                let new_doc = pdfium.create_new_pdf()?;
                let mut new_doc = new_doc;
                for (i, page_n) in order.iter().enumerate() {
                    new_doc.pages_mut().copy_pages_from_document(
                        &snapshot,
                        &page_n.to_string(),
                        i as PdfPageIndex,
                    )?;
                }
                document = new_doc;
            }
            FixOp::AddWatermark { text } => {
                let font = document.fonts_mut().helvetica_bold();
                for mut page in document.pages_mut().iter() {
                    let (w, h) = (page.width(), page.height());
                    let mut obj = page.objects_mut().create_text_object(
                        PdfPoints::ZERO,
                        PdfPoints::ZERO,
                        text,
                        font,
                        PdfPoints::new(48.0),
                    )?;
                    obj.set_fill_color(PdfColor::new(180, 180, 180, 120))?;
                    obj.rotate_counter_clockwise_degrees(45.0)?;
                    obj.translate(PdfPoints::new(w.value * 0.25), PdfPoints::new(h.value * 0.35))?;
                }
            }
            FixOp::AddPageNumbers => {
                let font = document.fonts_mut().helvetica();
                let total = document.pages().len();
                for (i, mut page) in document.pages_mut().iter().enumerate() {
                    let label = format!("{} / {}", i + 1, total);
                    let w = page.width();
                    page.objects_mut().create_text_object(
                        PdfPoints::new(w.value / 2.0 - 15.0),
                        PdfPoints::new(20.0),
                        &label,
                        font,
                        PdfPoints::new(10.0),
                    )?;
                }
            }
            FixOp::SplitDocument { from, to, output } => {
                document.save_to_file(&temp)?;
                let snapshot = pdfium.load_pdf_from_file(&temp, None)?;
                let mut part = pdfium.create_new_pdf()?;
                part.pages_mut().copy_pages_from_document(
                    &snapshot,
                    &format!("{from}-{to}"),
                    0,
                )?;
                part.save_to_file(output)?;
            }
            FixOp::MergeDocuments { path } => {
                let other = pdfium.load_pdf_from_file(std::path::Path::new(path), None)?;
                let count = other.pages().len();
                let at = document.pages().len();
                document.pages_mut().copy_pages_from_document(
                    &other,
                    &format!("1-{count}"),
                    at,
                )?;
            }
            FixOp::AddStamp { text } => {
                let font = document.fonts_mut().helvetica_bold();
                for mut page in document.pages_mut().iter() {
                    let w = page.width();
                    let h = page.height();
                    let mut obj = page.objects_mut().create_text_object(
                        PdfPoints::new(w.value - 140.0),
                        PdfPoints::new(h.value - 40.0),
                        text,
                        font,
                        PdfPoints::new(12.0),
                    )?;
                    obj.set_fill_color(PdfColor::new(200, 30, 30, 255))?;
                }
            }
            FixOp::RemoveAnnotations => {
                for mut page in document.pages_mut().iter() {
                    // always removes the first one until empty (the index shifts)
                    while page.annotations().len() > 0 {
                        let annotation = page.annotations().get(0)?;
                        page.annotations_mut().delete_annotation(annotation)?;
                    }
                }
            }
            FixOp::RemoveAttachments => {
                while document.attachments().len() > 0 {
                    document.attachments_mut().delete_at_index(0)?;
                }
            }
            // Low-level passes: applied after the save (see below)
            FixOp::FlattenLayers
            | FixOp::RemoveUnusedResources
            | FixOp::DownsampleImages { .. }
            | FixOp::CompressImages { .. } => {}
        }
        applied.push(op.to_string());
    }

    let _ = std::fs::remove_file(&temp);
    document
        .save_to_file(output)
        .with_context(|| format!("could not save {}", output.display()))?;
    drop(document);

    // Low-level passes (lopdf) over the already-saved file.
    let needs_lowlevel = ops.iter().any(|op| {
        matches!(
            op,
            FixOp::FlattenLayers
                | FixOp::RemoveUnusedResources
                | FixOp::DownsampleImages { .. }
                | FixOp::CompressImages { .. }
        )
    });
    if needs_lowlevel {
        lowlevel_pass(output, ops)?;
    }
    Ok(applied)
}

/// Optimizations and flattening that require editing the file's structure.
fn lowlevel_pass(path: &Path, ops: &[crate::fixns::FixOp]) -> Result<()> {
    use crate::fixns::FixOp;
    use lopdf::{Document, Object};

    let mut pdf = Document::load(path).map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut changed = false;

    for op in ops {
        match op {
            FixOp::FlattenLayers => {
                // Flattening = removing the optional-content layer structure,
                // leaving all content permanently visible.
                if let Ok(catalog) = pdf.catalog_mut() {
                    catalog.remove(b"OCProperties");
                }
                let ids: Vec<_> = pdf.objects.keys().cloned().collect();
                for id in ids {
                    if let Some(obj) = pdf.objects.get_mut(&id) {
                        match obj {
                            Object::Stream(s) => {
                                s.dict.remove(b"OC");
                            }
                            Object::Dictionary(d) => {
                                d.remove(b"OC");
                            }
                            _ => {}
                        }
                    }
                }
                changed = true;
            }
            FixOp::RemoveUnusedResources => {
                pdf.prune_objects();
                changed = true;
            }
            FixOp::DownsampleImages { dpi } => {
                if resample_images(&mut pdf, Some(*dpi), None)? {
                    changed = true;
                }
            }
            FixOp::CompressImages { quality } => {
                if resample_images(&mut pdf, None, Some(*quality))? {
                    changed = true;
                }
            }
            _ => {}
        }
    }

    if changed {
        // Optimizations only count if the file shrinks: write to a temporary,
        // compare, and keep the original when the rewrite comes out larger.
        let so_otimiza = ops.iter().all(|op| {
            matches!(
                op,
                FixOp::RemoveUnusedResources
                    | FixOp::DownsampleImages { .. }
                    | FixOp::CompressImages { .. }
            )
        });
        if so_otimiza {
            let antes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(u64::MAX);
            let temp = path.with_extension("pdfl-tmp");
            pdf.save(&temp).map_err(|e| anyhow::anyhow!("{e}"))?;
            let depois = std::fs::metadata(&temp).map(|m| m.len()).unwrap_or(u64::MAX);
            if depois < antes {
                std::fs::rename(&temp, path)?;
            } else {
                let _ = std::fs::remove_file(&temp);
            }
        } else {
            pdf.save(path).map_err(|e| anyhow::anyhow!("{e}"))?;
        }
    }
    Ok(())
}

/// [x0, y0, x1, y1] in points → PdfRect.
fn points_rect(r: [f64; 4]) -> PdfRect {
    PdfRect::new(
        PdfPoints::new(r[1] as f32), // bottom
        PdfPoints::new(r[0] as f32), // left
        PdfPoints::new(r[3] as f32), // top
        PdfPoints::new(r[2] as f32), // right
    )
}

/// Resamples and/or re-encodes the PDF's images, replacing each XObject's
/// stream. `max_dpi` caps the resolution (using the printed size declared in
/// the XObject); `jpeg_quality` re-encodes as JPEG.
/// Returns true if anything changed.
fn resample_images(
    pdf: &mut lopdf::Document,
    max_dpi: Option<f64>,
    jpeg_quality: Option<u8>,
) -> Result<bool> {
    use image::{DynamicImage, ImageEncoder};
    use lopdf::{Object, ObjectId};

    // Printed size of each image (to compute the real DPI): scans the content
    // streams looking for `cm ... /Name Do`.
    let mut printed_width: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
    for (_, page_id) in pdf.get_pages() {
        let content_bytes = pdf.get_page_content(page_id);
        let Ok(content) = lopdf::content::Content::decode(&content_bytes) else { continue };
        let mut ctm_width = 0.0f64;
        for op in content.operations.iter() {
            match op.operator.as_str() {
                "cm" => {
                    if let Some(a) = op.operands.first().and_then(|o| match o {
                        Object::Integer(n) => Some(*n as f64),
                        Object::Real(n) => Some(*n as f64),
                        _ => None,
                    }) {
                        ctm_width = a.abs();
                    }
                }
                "Do" => {
                    if let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) {
                        // The largest use wins: the image needs resolution for
                        // the biggest area it is drawn into.
                        let key = String::from_utf8_lossy(name).into_owned();
                        let entry = printed_width.entry(key).or_insert(0.0);
                        if ctm_width > *entry {
                            *entry = ctm_width;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // XObject name -> object id
    let mut name_of: std::collections::HashMap<ObjectId, String> = std::collections::HashMap::new();
    for (_, page_id) in pdf.get_pages() {
        if let Ok((Some(res), _)) = pdf.get_page_resources(page_id) {
            if let Ok(Object::Dictionary(xobjects)) =
                res.get(b"XObject").map(|v| match v {
                    Object::Reference(id) => pdf.get_object(*id).cloned().unwrap_or(Object::Null),
                    other => other.clone(),
                })
            {
                for (name, value) in xobjects.iter() {
                    if let Object::Reference(id) = value {
                        name_of.insert(*id, String::from_utf8_lossy(name).into_owned());
                    }
                }
            }
        }
    }

    let ids: Vec<ObjectId> = pdf.objects.keys().cloned().collect();
    let mut changed = false;
    for id in ids {
        let Some(Object::Stream(stream)) = pdf.objects.get(&id) else { continue };
        if stream.dict.get(b"Subtype").ok().and_then(|s| s.as_name().ok()) != Some(b"Image".as_slice()) {
            continue;
        }
        let get_int = |key: &[u8]| stream.dict.get(key).ok().and_then(|v| v.as_i64().ok());
        let (Some(w), Some(h)) = (get_int(b"Width"), get_int(b"Height")) else { continue };
        if w < 2 || h < 2 {
            continue;
        }
        // decode: JPEG (DCTDecode) or raw data after Flate
        let filter = stream
            .dict
            .get(b"Filter")
            .ok()
            .and_then(|f| f.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_default();
        let raw = stream.decompressed_content().unwrap_or_else(|_| stream.content.clone());
        let decoded: Option<DynamicImage> = if filter == "DCTDecode" {
            image::load_from_memory_with_format(&stream.content, image::ImageFormat::Jpeg).ok()
        } else {
            let comps = raw.len() / (w as usize * h as usize).max(1);
            match comps {
                3 => image::RgbImage::from_raw(w as u32, h as u32, raw.clone()).map(DynamicImage::ImageRgb8),
                1 => image::GrayImage::from_raw(w as u32, h as u32, raw.clone()).map(DynamicImage::ImageLuma8),
                _ => None, // CMYK and others: out of scope for this version
            }
        };
        let Some(image) = decoded else { continue };

        // resize target
        let printed_pt = name_of.get(&id).and_then(|n| printed_width.get(n)).copied().unwrap_or(0.0);
        let mut target = image.clone();
        let mut resized = false;
        if let Some(dpi) = max_dpi {
            if printed_pt > 1.0 {
                let inches = printed_pt / 72.0;
                let current_dpi = w as f64 / inches;
                if current_dpi > dpi {
                    let new_w = ((inches * dpi).round() as u32).max(1);
                    let new_h = ((new_w as f64) * (h as f64 / w as f64)).round().max(1.0) as u32;
                    target = image.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
                    resized = true;
                }
            }
        }
        if !resized && jpeg_quality.is_none() {
            continue;
        }

        // always re-encodes as JPEG (accepted by any PDF reader)
        let mut jpeg = Vec::new();
        let rgb = target.to_rgb8();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
            &mut jpeg,
            jpeg_quality.unwrap_or(90),
        );
        if encoder
            .write_image(rgb.as_raw(), rgb.width(), rgb.height(), image::ExtendedColorType::Rgb8)
            .is_err()
        {
            continue;
        }
        // only swaps if it got smaller (never inflate the file)
        if jpeg.len() >= stream.content.len() {
            continue;
        }
        if let Some(Object::Stream(stream)) = pdf.objects.get_mut(&id) {
            stream.dict.set("Width", Object::Integer(rgb.width() as i64));
            stream.dict.set("Height", Object::Integer(rgb.height() as i64));
            stream.dict.set("ColorSpace", Object::Name(b"DeviceRGB".to_vec()));
            stream.dict.set("BitsPerComponent", Object::Integer(8));
            stream.dict.set("Filter", Object::Name(b"DCTDecode".to_vec()));
            stream.dict.remove(b"DecodeParms");
            stream.dict.remove(b"SMask");
            stream.set_content(jpeg);
            changed = true;
        }
    }
    Ok(changed)
}

/// Text contained in a region of the page (coordinates in points, origin at
/// the bottom-left corner). `page` is 1-based.
pub fn extract_text_in_region(
    path: &Path,
    page: i64,
    rect: [f64; 4], // [x, y, largura, altura]
) -> Result<String> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("could not reopen PDF: {}", path.display()))?;
    let pages = document.pages();
    let page = pages
        .get((page - 1) as PdfPageIndex)
        .map_err(|_| anyhow::anyhow!("page {page} does not exist"))?;
    let text = page.text().map_err(|e| anyhow::anyhow!("{e}"))?;
    let bounds = PdfRect::new(
        PdfPoints::new(rect[1] as f32),
        PdfPoints::new(rect[0] as f32),
        PdfPoints::new((rect[1] + rect[3]) as f32),
        PdfPoints::new((rect[0] + rect[2]) as f32),
    );
    // A region with no characters is a legitimate result (empty string), not an error.
    match text.chars_inside_rect(bounds) {
        Ok(chars) => Ok(chars.iter().filter_map(|c| c.unicode_char()).collect()),
        Err(PdfiumError::NoCharsInRect) => Ok(String::new()),
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

/// Approximate maximum TAC inside a region of the page.
pub fn tac_in_region(path: &Path, page: i64, rect: [f64; 4]) -> Result<(f64, f64)> {
    let pdfium = bind_pdfium()?;
    let document = pdfium.load_pdf_from_file(path, None)?;
    let pages = document.pages();
    let pdf_page = pages
        .get((page - 1) as PdfPageIndex)
        .map_err(|_| anyhow::anyhow!("page {page} does not exist"))?;
    let (pw, ph) = (pdf_page.width().value as f64, pdf_page.height().value as f64);
    let bitmap = pdf_page.render_with_config(&PdfRenderConfig::new().set_target_width(600))?;
    let (bw, bh) = (bitmap.width() as usize, bitmap.height() as usize);
    let bytes = bitmap.as_rgba_bytes();

    // region (origin at the bottom) -> bitmap pixels (origin at the top)
    let px_x0 = ((rect[0] / pw) * bw as f64).max(0.0) as usize;
    let px_x1 = (((rect[0] + rect[2]) / pw) * bw as f64).min(bw as f64) as usize;
    let px_y0 = ((1.0 - (rect[1] + rect[3]) / ph) * bh as f64).max(0.0) as usize;
    let px_y1 = ((1.0 - rect[1] / ph) * bh as f64).min(bh as f64) as usize;

    let (mut max_tac, mut sum, mut count) = (0.0f64, 0.0f64, 0u64);
    for y in px_y0..px_y1 {
        for x in px_x0..px_x1 {
            let i = (y * bw + x) * 4;
            if i + 2 >= bytes.len() {
                continue;
            }
            let (r, g, b) =
                (bytes[i] as f64 / 255.0, bytes[i + 1] as f64 / 255.0, bytes[i + 2] as f64 / 255.0);
            let k = 1.0 - r.max(g).max(b);
            let tac = if k >= 1.0 {
                100.0
            } else {
                let c = (1.0 - r - k) / (1.0 - k);
                let m = (1.0 - g - k) / (1.0 - k);
                let yy = (1.0 - b - k) / (1.0 - k);
                (c + m + yy + k) * 100.0
            };
            if tac > max_tac {
                max_tac = tac;
            }
            sum += tac;
            count += 1;
        }
    }
    Ok(if count == 0 { (0.0, 0.0) } else { (max_tac, sum / count as f64) })
}

/// Renders pages in greyscale for visual analysis.
/// An empty `pages` means all of them. Returns (page number, width, height, pixels).
pub fn render_pages_gray(
    path: &Path,
    pages: &[i64],
    target_width: u16,
) -> Result<Vec<(i64, u32, u32, Vec<u8>)>> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("could not reopen PDF: {}", path.display()))?;
    let config = PdfRenderConfig::new().set_target_width(target_width as i32);

    let mut out = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let n = index as i64 + 1;
        if !pages.is_empty() && !pages.contains(&n) {
            continue;
        }
        let Ok(bitmap) = page.render_with_config(&config) else { continue };
        let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
        let gray: Vec<u8> = bitmap
            .as_rgba_bytes()
            .chunks_exact(4)
            .map(|px| ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8)
            .collect();
        out.push((n, w, h, gray));
    }
    Ok(out)
}

/// Renders pages in colour, at `dpi` (72 dpi = the PDF's own point grid).
/// An empty `pages` means all of them. Returns (page number, width, height,
/// RGBA pixels).
///
/// Greyscale is enough to *measure* a difference, which is why
/// `render_pages_gray` exists; showing one to a person is a different job —
/// a colour swapped for another of the same luminance is invisible in grey.
pub struct RenderedPage {
    pub number: i64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// `on_page` is called once the page count is known and once per page
/// rendered, so a caller can show progress. Rendering is where the time goes —
/// minutes for a long document at 300 dpi — and it is the caller, not this
/// function, that decides whether anyone is watching.
pub fn render_pages_rgba(
    path: &Path,
    pages: &[i64],
    dpi: u16,
    on_page: &mut dyn FnMut(RenderEvent),
) -> Result<Vec<RenderedPage>> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("could not open PDF: {}", path.display()))?;
    let config = PdfRenderConfig::new().scale_page_by_factor(dpi as f32 / 72.0);

    let selected = if pages.is_empty() {
        document.pages().len() as usize
    } else {
        // Only the requested pages that the document actually has: a range
        // running past the end must not inflate the total.
        pages.iter().filter(|n| **n >= 1 && **n <= document.pages().len() as i64).count()
    };
    on_page(RenderEvent::Starting { pages: selected });

    let mut out = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let n = index as i64 + 1;
        if !pages.is_empty() && !pages.contains(&n) {
            continue;
        }
        let bitmap = page
            .render_with_config(&config)
            .with_context(|| format!("could not render page {n} of {}", path.display()))?;
        out.push(RenderedPage {
            number: n,
            width: bitmap.width() as u32,
            height: bitmap.height() as u32,
            rgba: bitmap.as_rgba_bytes(),
        });
        on_page(RenderEvent::Rendered);
    }
    Ok(out)
}

/// What `render_pages_rgba` reports as it goes.
pub enum RenderEvent {
    /// The document is open and the page count is known — which it is not
    /// until then, so a bar cannot be sized any earlier.
    Starting { pages: usize },
    /// One more page is in memory.
    Rendered,
}

/// Scans barcodes/QR codes by rendering each page at high resolution.
/// Called on demand by the `codes::` namespace.
pub fn scan_barcodes(path: &Path) -> Result<Vec<Rc<BarcodeData>>> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("could not reopen PDF: {}", path.display()))?;

    let mut out = Vec::new();
    for (index, page) in document.pages().iter().enumerate() {
        let bitmap = match page.render_with_config(&PdfRenderConfig::new().set_target_width(1200)) {
            Ok(b) => b,
            Err(_) => continue,
        };
        let (w, h) = (bitmap.width() as u32, bitmap.height() as u32);
        let luma: Vec<u8> = bitmap
            .as_rgba_bytes()
            .chunks_exact(4)
            .map(|px| ((px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000) as u8)
            .collect();
        let scale = page.width().value as f64 / w as f64; // px -> pontos
        let page_height = page.height().value as f64;

        if let Ok(results) = rxing::helpers::detect_multiple_in_luma(luma, w, h) {
            for r in results {
                let points = r.getPoints();
                let (px, py) = if points.is_empty() {
                    (0.0, 0.0)
                } else {
                    (
                        points.iter().map(|p| p.x as f64).sum::<f64>() / points.len() as f64,
                        points.iter().map(|p| p.y as f64).sum::<f64>() / points.len() as f64,
                    )
                };
                out.push(Rc::new(BarcodeData {
                    page_number: index as i64 + 1,
                    format: format!("{:?}", r.getBarcodeFormat()),
                    text: r.getText().to_string(),
                    x: px * scale,
                    // the PDF's Y axis grows upward; the bitmap's grows downward
                    y: page_height - py * scale,
                }));
            }
        }
    }
    Ok(out)
}

/// Approximate TAC (max % and average coverage) via a low-resolution render
/// and an RGB→CMYK conversion with maximum GCR.
/// ponytail: this is a LOWER BOUND on the real TAC — neutral colors collapse
/// into pure K (a rich black 90/90/90/90 estimates ~100%), so it never raises
/// a false alarm, but values above ~300% are only detectable with real
/// separations — that is what `prepress::calculate_exact_tac` is for.
fn approximate_tac(page: &PdfPage) -> (f64, f64) {
    let Ok(bitmap) = page.render_with_config(&PdfRenderConfig::new().set_target_width(300)) else {
        return (0.0, 0.0);
    };
    let bytes = bitmap.as_rgba_bytes();
    let (mut max_tac, mut sum, mut count) = (0.0f64, 0.0f64, 0u64);
    for px in bytes.chunks_exact(4) {
        let (r, g, b) = (px[0] as f64 / 255.0, px[1] as f64 / 255.0, px[2] as f64 / 255.0);
        let k = 1.0 - r.max(g).max(b);
        let tac = if k >= 1.0 {
            100.0
        } else {
            let c = (1.0 - r - k) / (1.0 - k);
            let m = (1.0 - g - k) / (1.0 - k);
            let y = (1.0 - b - k) / (1.0 - k);
            (c + m + y + k) * 100.0
        };
        if tac > max_tac {
            max_tac = tac;
        }
        sum += tac;
        count += 1;
    }
    if count == 0 {
        (0.0, 0.0)
    } else {
        (max_tac, sum / count as f64)
    }
}

pub fn load_document(path: &Path) -> Result<Rc<DocData>> {
    let pdfium = bind_pdfium()?;
    let document = pdfium
        .load_pdf_from_file(path, None)
        .with_context(|| format!("could not open PDF: {}", path.display()))?;

    let bytes = std::fs::read(path).with_context(|| format!("could not read {}", path.display()))?;
    let file_size = bytes.len() as i64;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    drop(bytes);

    let doc_metadata = document.metadata();
    let tag_value = |tag: PdfDocumentMetadataTagType| {
        doc_metadata.get(tag).map(|t| t.value().to_string()).unwrap_or_default()
    };
    let tags = [
        ("Title", PdfDocumentMetadataTagType::Title),
        ("Author", PdfDocumentMetadataTagType::Author),
        ("Subject", PdfDocumentMetadataTagType::Subject),
        ("Keywords", PdfDocumentMetadataTagType::Keywords),
        ("Creator", PdfDocumentMetadataTagType::Creator),
        ("Producer", PdfDocumentMetadataTagType::Producer),
        ("CreationDate", PdfDocumentMetadataTagType::CreationDate),
        ("ModificationDate", PdfDocumentMetadataTagType::ModificationDate),
    ];
    let metadata: Vec<(String, String)> =
        tags.iter().map(|(name, tag)| (name.to_string(), tag_value(*tag))).collect();
    let title = tag_value(PdfDocumentMetadataTagType::Title);
    let author = tag_value(PdfDocumentMetadataTagType::Author);

    let mut object_count: i64 = 0;
    let mut pages = Vec::new();
    let mut fonts: Vec<Rc<FontData>> = Vec::new();
    let mut seen_fonts = HashSet::new();

    for (index, page) in document.pages().iter().enumerate() {
        let text = page.text().map(|t| t.all()).unwrap_or_default();
        let mut images = Vec::new();
        let mut min_stroke_pt: Option<f64> = None;

        object_count += page.objects().len() as i64;
        for object in page.objects().iter() {
            if object.as_path_object().is_some() {
                if let Ok(w) = object.stroke_width() {
                    let w = w.value as f64;
                    if w > 0.0 && min_stroke_pt.is_none_or(|m| w < m) {
                        min_stroke_pt = Some(w);
                    }
                }
            }
            if let Some(text_object) = object.as_text_object() {
                let font = text_object.font();
                let name = font.family().to_string();
                if seen_fonts.insert(name.clone()) {
                    fonts.push(Rc::new(FontData { name, is_embedded: font.is_embedded().unwrap_or(false) }));
                }
            }
            if let Some(image_object) = object.as_image_object() {
                let px_w = image_object.width().map(|p| p as i64).unwrap_or(0);
                let px_h = image_object.height().map(|p| p as i64).unwrap_or(0);
                // Effective DPI = pixels / size on the page in inches.
                // Falls back to the metadata DPI if the bounds are unavailable.
                let meta_dpi = (
                    image_object.horizontal_dpi().unwrap_or(0.0) as f64,
                    image_object.vertical_dpi().unwrap_or(0.0) as f64,
                );
                let (dpi_x, dpi_y) = match object.bounds() {
                    Ok(b) => {
                        let w_in = (b.width().value as f64) / 72.0;
                        let h_in = (b.height().value as f64) / 72.0;
                        if w_in > 0.0 && h_in > 0.0 {
                            (px_w as f64 / w_in, px_h as f64 / h_in)
                        } else {
                            meta_dpi
                        }
                    }
                    Err(_) => meta_dpi,
                };
                images.push(Rc::new(ImageData {
                    page_number: index as i64 + 1,
                    width: px_w,
                    height: px_h,
                    dpi_x,
                    dpi_y,
                    color_space: image_object
                        .color_space()
                        .map(|cs| format!("{cs:?}"))
                        .unwrap_or_else(|_| "Unknown".into()),
                    bits_per_pixel: image_object.bits_per_pixel().unwrap_or(0) as i64,
                }));
            }
        }

        let (tac_max, ink_avg) = approximate_tac(&page);

        let boundaries = page.boundaries();
        let rect = |b: std::result::Result<PdfPageBoundaryBox, PdfiumError>| {
            b.ok().map(|bb| {
                [
                    bb.bounds.left().value as f64,
                    bb.bounds.bottom().value as f64,
                    bb.bounds.right().value as f64,
                    bb.bounds.top().value as f64,
                ]
            })
        };
        let boxes = PageBoxes {
            media: rect(boundaries.media()),
            crop: rect(boundaries.crop()),
            trim: rect(boundaries.trim()),
            bleed: rect(boundaries.bleed()),
            art: rect(boundaries.art()),
        };

        pages.push(Rc::new(PageData {
            index: index as i64,
            width: page.width().value as f64,
            height: page.height().value as f64,
            text,
            images,
            tac_max,
            ink_avg,
            min_stroke_pt,
            boxes,
        }));
    }

    Ok(Rc::new(DocData {
        filename: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        title,
        author,
        pages,
        fonts,
        metadata,
        file_size,
        sha256,
        object_count,
        path: path.to_path_buf(),
        barcodes: std::cell::OnceCell::new(),
        lowlevel: std::cell::OnceCell::new(),
        colors: std::cell::OnceCell::new(),
    }))
}
