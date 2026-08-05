//! Namespace `fix::` — normalização do PDF.
//! As chamadas apenas ENFILEIRAM operações (validadas contra o documento);
//! quem aplica e salva é `pdf::apply_fixes`, no comando `pdfl fix`.
//! Fora desta fatia: downsample/compressão de imagens e subset de fontes
//! (o pdfium-render não expõe objetos de página de forma mutável, então
//! não é possível substituir imagens/fontes existentes) e linearização
//! (nenhuma das bibliotecas atuais gera PDF linearizado).

use crate::interpreter::{DocData, RuntimeError, Value};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum FixOp {
    SetPageSize { width: f64, height: f64 },
    SetCropBox { rect: [f64; 4] },
    SetTrimBox { rect: [f64; 4] },
    SetBleedBox { rect: [f64; 4] },
    /// `page` 0 = todas as páginas.
    RotatePage { page: i64, degrees: i64 },
    DeletePage { page: i64 },
    DuplicatePage { page: i64 },
    ReorderPages { order: Vec<i64> },
    AddWatermark { text: String },
    AddPageNumbers,
    /// Salva as páginas do intervalo em outro arquivo (o original segue intacto).
    SplitDocument { from: i64, to: i64, output: String },
    /// Anexa as páginas de outro PDF ao final.
    MergeDocuments { path: String },
    /// Texto no canto superior direito de cada página.
    AddStamp { text: String },
    FlattenLayers,
    RemoveAnnotations,
    RemoveAttachments,
    RemoveUnusedResources,
    /// Reamostra imagens acima do DPI alvo (substitui o stream via lopdf).
    DownsampleImages { dpi: f64 },
    /// Recodifica imagens como JPEG na qualidade dada.
    CompressImages { quality: u8 },
}

impl fmt::Display for FixOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FixOp::SetPageSize { width, height } => write!(f, "page size set to {width}x{height}pt"),
            FixOp::SetCropBox { rect } => write!(f, "CropBox set to {rect:?}"),
            FixOp::SetTrimBox { rect } => write!(f, "TrimBox set to {rect:?}"),
            FixOp::SetBleedBox { rect } => write!(f, "BleedBox set to {rect:?}"),
            FixOp::RotatePage { page: 0, degrees } => write!(f, "all pages rotated {degrees}°"),
            FixOp::RotatePage { page, degrees } => write!(f, "page {page} rotated {degrees}°"),
            FixOp::DeletePage { page } => write!(f, "page {page} removed"),
            FixOp::DuplicatePage { page } => write!(f, "page {page} duplicated"),
            FixOp::ReorderPages { order } => write!(f, "pages reordered to {order:?}"),
            FixOp::AddWatermark { text } => write!(f, "watermark \"{text}\" added"),
            FixOp::AddPageNumbers => write!(f, "page numbering added"),
            FixOp::SplitDocument { from, to, output } => {
                write!(f, "pages {from}-{to} saved to {output}")
            }
            FixOp::MergeDocuments { path } => write!(f, "pages from {path} appended at the end"),
            FixOp::AddStamp { text } => write!(f, "stamp \"{text}\" added"),
            FixOp::FlattenLayers => write!(f, "layers flattened"),
            FixOp::RemoveAnnotations => write!(f, "annotations removed"),
            FixOp::RemoveAttachments => write!(f, "attachments removed"),
            FixOp::RemoveUnusedResources => write!(f, "unused resources removed"),
            FixOp::DownsampleImages { dpi } => write!(f, "images resampled to at most {dpi} DPI"),
            FixOp::CompressImages { quality } => {
                write!(f, "images re-encoded as JPEG at quality {quality}")
            }
        }
    }
}

pub fn queue(doc: &DocData, name: &str, args: &[Value]) -> Result<FixOp, RuntimeError> {
    let page_count = doc.pages.len() as i64;
    match name {
        "set_page_size" => {
            let (w, h) = (num(args, 0, name)?, num(args, 1, name)?);
            if w <= 0.0 || h <= 0.0 {
                return Err(err(format!("fix::{name}: dimensions must be positive")));
            }
            Ok(FixOp::SetPageSize { width: w, height: h })
        }
        "set_crop_box" => Ok(FixOp::SetCropBox { rect: rect4(args, name)? }),
        "set_trim_box" => Ok(FixOp::SetTrimBox { rect: rect4(args, name)? }),
        "set_bleed_box" => Ok(FixOp::SetBleedBox { rect: rect4(args, name)? }),
        "rotate_page" => {
            // rotate_page(graus) = todas; rotate_page(pagina, graus) = uma
            let (page, degrees) = if args.len() >= 2 {
                (num(args, 0, name)? as i64, num(args, 1, name)? as i64)
            } else {
                (0, num(args, 0, name)? as i64)
            };
            if !matches!(degrees, 90 | 180 | 270) {
                return Err(err(format!("fix::{name}: rotation must be 90, 180 or 270 (got {degrees})")));
            }
            if page != 0 {
                check_page(page, page_count, name)?;
            }
            Ok(FixOp::RotatePage { page, degrees })
        }
        "delete_page" => {
            let page = num(args, 0, name)? as i64;
            check_page(page, page_count, name)?;
            if page_count == 1 {
                return Err(err(format!("fix::{name}: cannot remove the only page of the PDF")));
            }
            Ok(FixOp::DeletePage { page })
        }
        "duplicate_page" => {
            let page = num(args, 0, name)? as i64;
            check_page(page, page_count, name)?;
            Ok(FixOp::DuplicatePage { page })
        }
        "reorder_pages" => {
            let order = match args.first() {
                Some(Value::List(items)) => items
                    .iter()
                    .map(|v| match v {
                        Value::Int(n) => Ok(*n),
                        other => Err(err(format!("fix::{name}: the list must contain numbers, found {}", other.type_name()))),
                    })
                    .collect::<Result<Vec<i64>, _>>()?,
                _ => return Err(err(format!("fix::{name} expects a list with the new order, e.g. [2, 1, 3]"))),
            };
            let mut sorted = order.clone();
            sorted.sort_unstable();
            if sorted != (1..=page_count).collect::<Vec<_>>() {
                return Err(err(format!(
                    "fix::{name}: the order must use each page from 1 to {page_count} exactly once"
                )));
            }
            Ok(FixOp::ReorderPages { order })
        }
        "add_watermark" => match args.first() {
            Some(Value::Str(s)) if !s.trim().is_empty() => Ok(FixOp::AddWatermark { text: s.clone() }),
            _ => Err(err(format!("fix::{name} expects the watermark text"))),
        },
        "add_page_numbers" => Ok(FixOp::AddPageNumbers),
        // ---- documento ----
        "split_document" => {
            // split_document(de, ate, "saida.pdf")
            let from = num(args, 0, name)? as i64;
            let to = num(args, 1, name)? as i64;
            let output = match args.get(2) {
                Some(Value::Str(s)) if !s.trim().is_empty() => s.clone(),
                _ => return Err(err(format!("fix::{name} expects the output file as the 3rd argument"))),
            };
            check_page(from, page_count, name)?;
            check_page(to, page_count, name)?;
            if to < from {
                return Err(err(format!("fix::{name}: invalid range ({from} to {to})")));
            }
            Ok(FixOp::SplitDocument { from, to, output })
        }
        "merge_documents" => match args.first() {
            Some(Value::Str(p)) if !p.trim().is_empty() => {
                if !std::path::Path::new(p.as_str()).exists() {
                    return Err(err(format!("fix::{name}: file not found: {p}")));
                }
                Ok(FixOp::MergeDocuments { path: p.clone() })
            }
            _ => Err(err(format!("fix::{name} expects the path of the PDF to append"))),
        },
        "add_stamps" | "add_stamp" => match args.first() {
            Some(Value::Str(s)) if !s.trim().is_empty() => Ok(FixOp::AddStamp { text: s.clone() }),
            _ => Err(err(format!("fix::{name} expects the stamp text"))),
        },
        "flatten_layers" => Ok(FixOp::FlattenLayers),
        "remove_annotations" => Ok(FixOp::RemoveAnnotations),
        "remove_attachments" => Ok(FixOp::RemoveAttachments),
        "remove_unused_resources" => Ok(FixOp::RemoveUnusedResources),
        "downsample_images" => {
            let dpi = num(args, 0, name).unwrap_or(300.0);
            if dpi <= 0.0 {
                return Err(err(format!("fix::{name}: DPI must be positive")));
            }
            Ok(FixOp::DownsampleImages { dpi })
        }
        "compress_images" => {
            let quality = num(args, 0, name).unwrap_or(85.0);
            if !(1.0..=100.0).contains(&quality) {
                return Err(err(format!("fix::{name}: quality must be between 1 and 100")));
            }
            Ok(FixOp::CompressImages { quality: quality as u8 })
        }
        _ => Err(err(format!("unknown function: fix::{name}"))),
    }
}

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

fn num(args: &[Value], i: usize, name: &str) -> Result<f64, RuntimeError> {
    match args.get(i) {
        Some(Value::Int(n)) => Ok(*n as f64),
        Some(Value::Float(n)) => Ok(*n),
        _ => Err(err(format!("fix::{name} expects a number at position {}", i + 1))),
    }
}

fn rect4(args: &[Value], name: &str) -> Result<[f64; 4], RuntimeError> {
    let r = [num(args, 0, name)?, num(args, 1, name)?, num(args, 2, name)?, num(args, 3, name)?];
    if r[2] <= r[0] || r[3] <= r[1] {
        return Err(err(format!("fix::{name}: invalid rectangle (expected x0, y0, x1, y1 with x1 > x0 and y1 > y0)")));
    }
    Ok(r)
}

fn check_page(page: i64, page_count: i64, name: &str) -> Result<(), RuntimeError> {
    if page < 1 || page > page_count {
        Err(err(format!("fix::{name}: page {page} does not exist (the PDF has {page_count})")))
    } else {
        Ok(())
    }
}
