//! Namespace `prepress::` — prepress validations.
//! Includes real color separations read from the content stream (exact TAC,
//! spot colors, rich black, overprint) through the colors module.

use crate::interpreter::{DocData, PageData, RuntimeError, Value};
use std::rc::Rc;

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        // ---- TAC and coverage ----
        "calculate_tac" => {
            // Without an argument: the document's highest TAC. With n: page n's.
            match opt_page(doc, args, name)? {
                Some(p) => Ok(Value::Float(round1(p.tac_max))),
                None => Ok(Value::Float(round1(
                    doc.pages.iter().map(|p| p.tac_max).fold(0.0, f64::max),
                ))),
            }
        }
        "calculate_ink_coverage" => match opt_page(doc, args, name)? {
            Some(p) => Ok(Value::Float(round1(p.ink_avg))),
            None => {
                let n = doc.pages.len().max(1) as f64;
                Ok(Value::Float(round1(doc.pages.iter().map(|p| p.ink_avg).sum::<f64>() / n)))
            }
        },
        "calculate_tac_by_region" => {
            // calculate_tac_by_region(page, region) -> [tac_max, coverage]
            let page = match args.first() {
                Some(Value::Int(n)) => *n,
                Some(Value::Page(p)) => p.index + 1,
                _ => return Err(err(format!("prepress::{name} expects the page number and the region"))),
            };
            let region = match args.get(1) {
                Some(Value::Region(r)) => r.clone(),
                _ => return Err(err(format!("prepress::{name} expects a region as the 2nd argument"))),
            };
            if page < 1 || page as usize > doc.pages.len() {
                return Err(err(format!("page {page} does not exist (the PDF has {})", doc.pages.len())));
            }
            let (max, avg) = crate::pdf::tac_in_region(
                &doc.path,
                page,
                [region.x, region.y, region.width, region.height],
            )
            .map_err(|e| err(format!("prepress::{name}: {e:#}")))?;
            Ok(Value::List(Rc::new(vec![Value::Float(round1(max)), Value::Float(round1(avg))])))
        }
        "validate_tac_limits" => {
            let limit = num_arg(args, 0, name).unwrap_or(300.0);
            Ok(Value::Bool(doc.pages.iter().all(|p| p.tac_max <= limit)))
        }
        // ---- linhas finas ----
        "detect_hairlines" => {
            // true = there is a stroke below the limit (default 0.25 pt)
            let limit = num_arg(args, 0, name).unwrap_or(0.25);
            Ok(Value::Bool(min_stroke(doc).is_some_and(|w| w < limit)))
        }
        "detect_fine_lines" => {
            let limit = num_arg(args, 0, name).unwrap_or(1.0);
            Ok(Value::Bool(min_stroke(doc).is_some_and(|w| w < limit)))
        }
        "validate_minimum_stroke_width" => {
            // true = no stroke below the required minimum
            let min = num_arg(args, 0, name)
                .ok_or_else(|| err(format!("prepress::{name} expects the minimum width in points")))?;
            Ok(Value::Bool(min_stroke(doc).is_none_or(|w| w >= min)))
        }
        // ---- cores ----
        "detect_color_mode" => {
            let spaces: Vec<&str> =
                doc.pages.iter().flat_map(|p| p.images.iter()).map(|i| i.color_space.as_str()).collect();
            let has_rgb = spaces.iter().any(|s| s.contains("RGB"));
            let has_cmyk = spaces.iter().any(|s| s.contains("CMYK"));
            Ok(Value::Str(
                match (has_rgb, has_cmyk) {
                    (true, true) => "Mixed",
                    (true, false) => "RGB",
                    (false, true) => "CMYK",
                    (false, false) if spaces.is_empty() => "None",
                    _ => "Other",
                }
                .into(),
            ))
        }
        "validate_color_space" => {
            // true = every image is in the required space (e.g. "DeviceCMYK")
            let wanted = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(err(format!("prepress::{name} expects the required color space (string)"))),
            };
            Ok(Value::Bool(
                doc.pages.iter().flat_map(|p| p.images.iter()).all(|i| i.color_space == wanted),
            ))
        }
        // ---- fontes ----
        "list_fonts" => Ok(Value::List(Rc::new(
            doc.fonts.iter().map(|f| Value::Str(f.name.clone())).collect(),
        ))),
        "validate_font_embedding" => Ok(Value::Bool(doc.fonts.iter().all(|f| f.is_embedded))),
        // ---- pages and boxes ----
        "get_page_size" => {
            let p = page_arg(doc, args, name)?;
            Ok(Value::List(Rc::new(vec![Value::Float(p.width), Value::Float(p.height)])))
        }
        "get_page_boxes" => {
            let p = page_arg(doc, args, name)?;
            let mut out = Vec::new();
            for (label, b) in [
                ("MediaBox", &p.boxes.media),
                ("CropBox", &p.boxes.crop),
                ("TrimBox", &p.boxes.trim),
                ("BleedBox", &p.boxes.bleed),
                ("ArtBox", &p.boxes.art),
            ] {
                if let Some([x0, y0, x1, y1]) = b {
                    out.push(Value::Str(format!("{label}: [{x0}, {y0}, {x1}, {y1}]")));
                }
            }
            Ok(Value::List(Rc::new(out)))
        }
        "validate_media_box" => Ok(Value::Bool(doc.pages.iter().all(|p| p.boxes.media.is_some()))),
        "validate_trim_box" => Ok(Value::Bool(doc.pages.iter().all(|p| p.boxes.trim.is_some()))),
        "validate_bleed_box" => Ok(Value::Bool(doc.pages.iter().all(|p| p.boxes.bleed.is_some()))),
        "check_page_geometry" => {
            // true = on every page the BleedBox exceeds the TrimBox by at least
            // N points on each side (use unit literals: 3mm).
            // Default: 3mm. A page missing either box fails.
            let min_pt = num_arg(args, 0, name).unwrap_or(3.0 * 72.0 / 25.4);
            let ok = doc.pages.iter().all(|p| match (&p.boxes.trim, &p.boxes.bleed) {
                (Some(t), Some(b)) => {
                    t[0] - b[0] >= min_pt - 0.01
                        && t[1] - b[1] >= min_pt - 0.01
                        && b[2] - t[2] >= min_pt - 0.01
                        && b[3] - t[3] >= min_pt - 0.01
                }
                _ => false,
            });
            Ok(Value::Bool(ok))
        }
        // ---- real separations ----
        "calculate_exact_tac" => {
            // TAC of the colors declared in the content stream (exact, not estimated)
            let info = colors(doc)?;
            match opt_page(doc, args, name)? {
                Some(p) => Ok(Value::Float(round1(
                    info.tac_by_page.get(p.index as usize).copied().unwrap_or(0.0),
                ))),
                None => Ok(Value::Float(round1(
                    info.tac_by_page.iter().copied().fold(0.0, f64::max),
                ))),
            }
        }
        "detect_spot_colors" => {
            let info = colors(doc)?;
            Ok(Value::List(Rc::new(
                info.spot_names.iter().cloned().map(Value::Str).collect(),
            )))
        }
        "compare_colors_delta_e" => {
            // compare_colors_delta_e([c,m,y,k], [c,m,y,k]) -> CIE76 Delta-E
            let ink = |i: usize| -> Result<crate::colors::Ink, RuntimeError> {
                match args.get(i) {
                    Some(Value::List(items)) => {
                        let n: Vec<f64> = items
                            .iter()
                            .filter_map(|v| match v {
                                Value::Int(x) => Some(*x as f64),
                                Value::Float(x) => Some(*x),
                                _ => None,
                            })
                            .collect();
                        match n.len() {
                            4 => Ok(crate::colors::Ink::Cmyk(n[0], n[1], n[2], n[3])),
                            3 => Ok(crate::colors::Ink::Rgb(n[0], n[1], n[2])),
                            1 => Ok(crate::colors::Ink::Gray(n[0])),
                            _ => Err(err(format!(
                                "prepress::{name}: the color must have 1 (gray), 3 (RGB) or 4 (CMYK) components"
                            ))),
                        }
                    }
                    _ => Err(err(format!("prepress::{name} expects two colors as lists"))),
                }
            };
            Ok(Value::Float(round1(crate::colors::delta_e(&ink(0)?, &ink(1)?))))
        }
        "detect_rich_black" => Ok(Value::Bool(colors(doc)?.has_rich_black)),
        "validate_overprint_settings" => {
            // true = no overprint enabled (the safe default for offset)
            Ok(Value::Bool(!colors(doc)?.overprint_on))
        }
        "validate_output_intent" => {
            let info = colors(doc)?;
            match args.first() {
                Some(Value::Str(expected)) => Ok(Value::Bool(
                    info.output_intent.as_deref().is_some_and(|i| i.contains(expected.as_str())),
                )),
                _ => Ok(Value::Bool(info.output_intent.is_some())),
            }
        }
        "check_rendering_intent" => {
            let info = colors(doc)?;
            match args.first() {
                Some(Value::Str(expected)) => Ok(Value::Bool(
                    info.rendering_intents.is_empty()
                        || info.rendering_intents.iter().all(|r| r == expected),
                )),
                _ => Ok(Value::List(Rc::new(
                    info.rendering_intents.iter().cloned().map(Value::Str).collect(),
                ))),
            }
        }
        // ---- fonts (file-level details) ----
        "detect_missing_glyphs" => {
            // Fonts without a widths table: the reader has to guess the metrics
            let info = colors(doc)?;
            Ok(Value::List(Rc::new(
                info.fonts
                    .iter()
                    .filter(|(_, f)| f.missing_widths)
                    .map(|(n, _)| Value::Str(n.clone()))
                    .collect(),
            )))
        }
        "detect_text_substitution" => {
            // A non-embedded font = the reader substitutes a similar one
            let info = colors(doc)?;
            Ok(Value::List(Rc::new(
                info.fonts
                    .iter()
                    .filter(|(_, f)| !f.embedded)
                    .map(|(n, _)| Value::Str(n.clone()))
                    .collect(),
            )))
        }
        "subset_fonts" => {
            // true = every embedded font is subsetted (a lean file)
            let info = colors(doc)?;
            Ok(Value::Bool(info.fonts.values().filter(|f| f.embedded).all(|f| f.subset)))
        }
        "check_font_licensing" => {
            // Type3 and non-embedded fonts are the licensing-risk cases
            let info = colors(doc)?;
            Ok(Value::List(Rc::new(
                info.fonts
                    .iter()
                    .filter(|(_, f)| f.font_type == "Type3" || !f.embedded)
                    .map(|(n, f)| {
                        Value::Str(format!(
                            "{n} ({})",
                            if f.font_type == "Type3" { "Type3" } else { "not embedded" }
                        ))
                    })
                    .collect(),
            )))
        }
        "validate_font_size" => {
            // true = no text below the minimum size (default 6pt)
            let min = num_arg(args, 0, name).unwrap_or(6.0);
            let info = colors(doc)?;
            Ok(Value::Bool(info.font_sizes.iter().all(|s| *s >= min)))
        }
        // ---- page geometry ----
        "detect_hairlines_exact" => {
            // A stroke of width 0 is PostScript's classic hairline
            Ok(Value::Bool(colors(doc)?.has_zero_width_stroke))
        }
        _ => Err(err(format!("unknown function: prepress::{name}"))),
    }
}

/// Analyzes the file's separations exactly once.
fn colors(doc: &Rc<DocData>) -> Result<&crate::colors::ColorInfo, RuntimeError> {
    if doc.colors.get().is_none() {
        let info = crate::colors::analyze(&doc.path)
            .map_err(|e| err(format!("color analysis failed: {e}")))?;
        let _ = doc.colors.set(info);
    }
    Ok(doc.colors.get().expect("cache preenchido acima"))
}

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

fn min_stroke(doc: &DocData) -> Option<f64> {
    doc.pages.iter().filter_map(|p| p.min_stroke_pt).fold(None, |acc, w| {
        Some(match acc {
            Some(a) if a < w => a,
            _ => w,
        })
    })
}

fn num_arg(args: &[Value], i: usize, _name: &str) -> Option<f64> {
    match args.get(i) {
        Some(Value::Int(n)) => Some(*n as f64),
        Some(Value::Float(n)) => Some(*n),
        _ => None,
    }
}

/// Optional page (1-based): Some(page) if an argument was given, None = the document.
fn opt_page<'a>(
    doc: &'a DocData,
    args: &[Value],
    name: &str,
) -> Result<Option<&'a Rc<PageData>>, RuntimeError> {
    match args.first() {
        None => Ok(None),
        Some(Value::Int(n)) => {
            if *n < 1 || *n as usize > doc.pages.len() {
                Err(err(format!("page {n} does not exist (the PDF has {})", doc.pages.len())))
            } else {
                Ok(Some(&doc.pages[(*n - 1) as usize]))
            }
        }
        Some(Value::Page(p)) => {
            let idx = p.index as usize;
            Ok(Some(&doc.pages[idx.min(doc.pages.len().saturating_sub(1))]))
        }
        Some(other) => Err(err(format!("prepress::{name} expects the page number, got {}", other.type_name()))),
    }
}

fn page_arg<'a>(doc: &'a DocData, args: &[Value], name: &str) -> Result<&'a Rc<PageData>, RuntimeError> {
    match opt_page(doc, args, name)? {
        Some(p) => Ok(p),
        None => doc.pages.first().ok_or_else(|| err("the PDF has no pages".into())),
    }
}
