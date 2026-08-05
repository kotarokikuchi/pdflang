//! Namespace `codes::` — barcodes and QR codes.
//! The scan runs on demand on first use (renders the pages at high
//! resolution and decodes them with rxing).

use crate::interpreter::{BarcodeData, DocData, RuntimeError, Value};
use std::rc::Rc;

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    let codes = barcodes(doc)?;
    match name {
        // ---- detection ----
        "detect_barcodes" => Ok(Value::Bool(!codes.is_empty())),
        "detect_qrcodes" => Ok(Value::Bool(codes.iter().any(|c| c.format == "QR_CODE"))),
        "count_barcodes" => Ok(Value::Int(codes.len() as i64)),
        "get_barcode_type" => Ok(Value::Str(code_arg(codes, args, name)?.format.clone())),
        "get_barcode_location" => {
            let c = code_arg(codes, args, name)?;
            Ok(Value::List(Rc::new(vec![
                Value::Int(c.page_number),
                Value::Float((c.x * 10.0).round() / 10.0),
                Value::Float((c.y * 10.0).round() / 10.0),
            ])))
        }
        // ---- decoding ----
        "decode_barcode" => Ok(Value::Str(code_arg(codes, args, name)?.text.clone())),
        "validate_barcode_checksum" | "validate_gtin" | "validate_ean" => {
            // GTIN check digit (EAN-8/13, UPC-A, GTIN-14).
            // Takes a string or the index of a detected code (default 1).
            let digits = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => code_arg(codes, args, name)?.text.clone(),
            };
            Ok(Value::Bool(gtin_valid(&digits)))
        }
        "validate_code128" => {
            // Code128's checksum is validated by the decoding itself:
            // true = a Code128 was decoded successfully.
            Ok(Value::Bool(codes.iter().any(|c| c.format == "CODE_128")))
        }
        // ---- comparison ----
        "compare_barcode_with_text" => {
            // true = every code's content appears in the document's text.
            let text: String = doc.pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n");
            Ok(Value::Bool(!codes.is_empty() && codes.iter().all(|c| text.contains(&c.text))))
        }
        "validate_barcode_format" => {
            // true = every code's content matches the pattern (regex).
            let pattern = match args.first() {
                Some(Value::Str(s)) => s.clone(),
                _ => return Err(err(format!("codes::{name} expects a pattern (string)"))),
            };
            let re = regex::Regex::new(&pattern)
                .map_err(|e| err(format!("codes::{name}: invalid pattern: {e}")))?;
            Ok(Value::Bool(!codes.is_empty() && codes.iter().all(|c| re.is_match(&c.text))))
        }
        "validate_barcode_position" => {
            // Takes a region or 4 numbers [x0, y0, x1, y1] in points.
            if let Some(Value::Region(r)) = args.first() {
                return Ok(Value::Bool(
                    !codes.is_empty() && codes.iter().all(|c| r.contains_point(c.x, c.y)),
                ));
            }
            let n = |i: usize| match args.get(i) {
                Some(Value::Int(v)) => Ok(*v as f64),
                Some(Value::Float(v)) => Ok(*v),
                _ => Err(err(format!("codes::{name} expects 4 numbers: x0, y0, x1, y1"))),
            };
            let (x0, y0, x1, y1) = (n(0)?, n(1)?, n(2)?, n(3)?);
            Ok(Value::Bool(
                !codes.is_empty()
                    && codes.iter().all(|c| c.x >= x0 && c.x <= x1 && c.y >= y0 && c.y <= y1),
            ))
        }
        _ => Err(err(format!("unknown function: codes::{name}"))),
    }
}

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

/// Scans on demand and stores the result in the document's cache.
fn barcodes(doc: &Rc<DocData>) -> Result<&Vec<Rc<BarcodeData>>, RuntimeError> {
    if doc.barcodes.get().is_none() {
        let scanned = crate::pdf::scan_barcodes(&doc.path)
            .map_err(|e| err(format!("failed to scan barcodes: {e:#}")))?;
        let _ = doc.barcodes.set(scanned);
    }
    Ok(doc.barcodes.get().expect("cache preenchido acima"))
}

/// Optional code index (1-based): without a numeric argument, the first one.
fn code_arg<'a>(
    codes: &'a [Rc<BarcodeData>],
    args: &[Value],
    _name: &str,
) -> Result<&'a Rc<BarcodeData>, RuntimeError> {
    let n = match args.first() {
        Some(Value::Int(n)) => *n,
        _ => 1,
    };
    if n < 1 || n as usize > codes.len() {
        return Err(err(format!("code {n} does not exist (found: {})", codes.len())));
    }
    Ok(&codes[(n - 1) as usize])
}

/// GTIN check digit (mod 10, weights 3/1 from right to left).
fn gtin_valid(code: &str) -> bool {
    let digits: Vec<u32> = code.chars().filter_map(|c| c.to_digit(10)).collect();
    if digits.len() != code.len() || !matches!(digits.len(), 8 | 12 | 13 | 14) {
        return false;
    }
    let sum: u32 = digits[..digits.len() - 1]
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| if i % 2 == 0 { d * 3 } else { *d })
        .sum();
    (10 - sum % 10) % 10 == *digits.last().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gtin() {
        assert!(gtin_valid("7891234567895")); // valid EAN-13
        assert!(gtin_valid("96385074")); // valid EAN-8
        assert!(!gtin_valid("7891234567890")); // wrong check digit
        assert!(!gtin_valid("789123")); // invalid length
        assert!(!gtin_valid("78912345678AB")); // not numeric
    }
}
