//! Namespace `struct::` — the PDF's structure and metadata.
//! The low-level functions (objects, XMP, security) use lopdf and are
//! analyzed exactly once, on demand.

use crate::interpreter::{DocData, RuntimeError, Value};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

/// Result of the file's low-level analysis (via lopdf).
#[derive(Debug)]
pub struct StructInfo {
    /// (object number, type description), ordered by number.
    pub objects: Vec<(u32, String)>,
    /// Objects unreachable from the trailer.
    pub unreferenced: Vec<String>,
    /// Unreachable objects that are resources (fonts, images, XObjects).
    pub orphaned: Vec<String>,
    /// Approximate size per object number (for streams: the content's bytes).
    pub sizes: HashMap<u32, i64>,
    pub xmp: String,
    pub has_javascript: bool,
    pub encrypted: bool,
    pub suspicious_actions: Vec<String>,
    /// Presence of signature fields (cryptographic validation: a future phase).
    pub has_signature_fields: bool,
}

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        // ---- metadados ----
        "get_title" => Ok(Value::Str(doc.title.clone())),
        "get_author" => Ok(Value::Str(doc.author.clone())),
        "get_producer" => Ok(Value::Str(meta(doc, "Producer"))),
        "get_creator" => Ok(Value::Str(meta(doc, "Creator"))),
        "get_subject" => Ok(Value::Str(meta(doc, "Subject"))),
        "get_keywords" => Ok(Value::Str(meta(doc, "Keywords"))),
        "get_creation_date" => Ok(Value::Str(format_pdf_date(&meta(doc, "CreationDate")))),
        "get_modification_date" => Ok(Value::Str(format_pdf_date(&meta(doc, "ModificationDate")))),
        "list_metadata_entries" => Ok(Value::List(Rc::new(
            doc.metadata
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .map(|(k, v)| Value::Str(format!("{k}: {v}")))
                .collect(),
        ))),
        // ---- objects and resources ----
        "count_objects" => Ok(Value::Int(doc.object_count)),
        "file_size" => Ok(Value::Int(doc.file_size)),
        "calculate_sha256" => Ok(Value::Str(doc.sha256.clone())),
        "detect_file_bloat" => {
            // true = bloated. Limit in KB per page (default 1024).
            let limit_kb = match args.first() {
                Some(Value::Int(n)) => *n,
                Some(Value::Float(n)) => *n as i64,
                None => 1024,
                Some(other) => {
                    return Err(RuntimeError {
                        message: format!("struct::detect_file_bloat expects a number (KB per page), got {}", other.type_name()),
                    })
                }
            };
            let pages = doc.pages.len().max(1) as i64;
            Ok(Value::Bool(doc.file_size / 1024 / pages > limit_kb))
        }
        // ---- low level (lopdf, analyzed on demand) ----
        "list_objects" => {
            let info = lowlevel(doc)?;
            Ok(Value::List(Rc::new(
                info.objects.iter().map(|(id, t)| Value::Str(format!("{id}: {t}"))).collect(),
            )))
        }
        "detect_unreferenced_objects" => {
            let info = lowlevel(doc)?;
            Ok(Value::List(Rc::new(info.unreferenced.iter().cloned().map(Value::Str).collect())))
        }
        "detect_orphaned_resources" => {
            let info = lowlevel(doc)?;
            Ok(Value::List(Rc::new(info.orphaned.iter().cloned().map(Value::Str).collect())))
        }
        "measure_object_size" => {
            let n = match args.first() {
                Some(Value::Int(n)) => *n,
                _ => return Err(err_msg("struct::measure_object_size expects the object number")),
            };
            let info = lowlevel(doc)?;
            info.sizes
                .get(&(n as u32))
                .map(|s| Value::Int(*s))
                .ok_or_else(|| err_msg(&format!("object {n} does not exist")))
        }
        "extract_xmp" => Ok(Value::Str(lowlevel(doc)?.xmp.clone())),
        "detect_javascript" => Ok(Value::Bool(lowlevel(doc)?.has_javascript)),
        "check_encryption" => Ok(Value::Bool(lowlevel(doc)?.encrypted)),
        "validate_permissions" => {
            // true = no restrictions (the document is not encrypted)
            Ok(Value::Bool(!lowlevel(doc)?.encrypted))
        }
        "detect_suspicious_actions" => {
            let info = lowlevel(doc)?;
            Ok(Value::List(Rc::new(
                info.suspicious_actions.iter().cloned().map(Value::Str).collect(),
            )))
        }
        "validate_signatures" => {
            // Presence of digital signature fields.
            // ponytail: cryptographic chain validation is left for a future phase
            Ok(Value::Bool(lowlevel(doc)?.has_signature_fields))
        }
        _ => Err(RuntimeError { message: format!("unknown function: struct::{name}") }),
    }
}

fn err_msg(m: &str) -> RuntimeError {
    RuntimeError { message: m.into() }
}

/// Analyzes the file with lopdf exactly once and caches it on the document.
fn lowlevel(doc: &Rc<DocData>) -> Result<&StructInfo, RuntimeError> {
    if doc.lowlevel.get().is_none() {
        let info = analyze(&doc.path)
            .map_err(|e| err_msg(&format!("structural analysis failed: {e}")))?;
        let _ = doc.lowlevel.set(info);
    }
    Ok(doc.lowlevel.get().expect("cache preenchido acima"))
}

fn analyze(path: &std::path::Path) -> Result<StructInfo, String> {
    use lopdf::{Document, Object, ObjectId};

    let pdf = Document::load(path).map_err(|e| e.to_string())?;

    // Reachability from the trailer (Root, Info, ...)
    let mut reachable: HashSet<ObjectId> = HashSet::new();
    let mut stack: Vec<Object> = pdf.trailer.iter().map(|(_, v)| v.clone()).collect();
    while let Some(obj) = stack.pop() {
        match obj {
            Object::Reference(id) => {
                if reachable.insert(id) {
                    if let Ok(o) = pdf.get_object(id) {
                        stack.push(o.clone());
                    }
                }
            }
            Object::Array(items) => stack.extend(items),
            Object::Dictionary(dict) => stack.extend(dict.iter().map(|(_, v)| v.clone())),
            Object::Stream(s) => stack.extend(s.dict.iter().map(|(_, v)| v.clone())),
            _ => {}
        }
    }

    let describe = |obj: &Object| -> String {
        let type_of = |d: &lopdf::Dictionary| {
            d.get(b"Type")
                .ok()
                .and_then(|t| t.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).into_owned())
        };
        match obj {
            Object::Stream(s) => {
                let t = type_of(&s.dict).unwrap_or_else(|| "Stream".into());
                let sub = s
                    .dict
                    .get(b"Subtype")
                    .ok()
                    .and_then(|x| x.as_name().ok())
                    .map(|n| format!("/{}", String::from_utf8_lossy(n)))
                    .unwrap_or_default();
                format!("{t}{sub}")
            }
            Object::Dictionary(d) => type_of(d).unwrap_or_else(|| "Dictionary".into()),
            Object::Array(_) => "Array".into(),
            Object::String(..) => "String".into(),
            Object::Integer(_) | Object::Real(_) => "Number".into(),
            Object::Name(_) => "Name".into(),
            Object::Boolean(_) => "Boolean".into(),
            Object::Null => "Null".into(),
            Object::Reference(_) => "Reference".into(),
        }
    };
    let is_resource = |desc: &str| {
        desc.contains("Font") || desc.contains("/Image") || desc.contains("XObject") || desc.contains("/Form")
    };

    let mut objects: Vec<(u32, String)> = Vec::new();
    let mut sizes: HashMap<u32, i64> = HashMap::new();
    let mut unreferenced = Vec::new();
    let mut orphaned = Vec::new();
    let mut has_javascript = false;
    let mut suspicious: Vec<String> = Vec::new();
    let mut has_signature_fields = false;

    let mut ids: Vec<ObjectId> = pdf.objects.keys().cloned().collect();
    ids.sort_unstable();
    for id in ids {
        let obj = &pdf.objects[&id];
        let desc = describe(obj);
        objects.push((id.0, desc.clone()));
        let size = match obj {
            Object::Stream(s) => s.content.len() as i64,
            other => format!("{other:?}").len() as i64,
        };
        sizes.insert(id.0, size);
        // ObjStm/XRef are file infrastructure: they are never referenced by the
        // trailer, so reporting them would be a false alarm.
        let infra = desc == "ObjStm" || desc == "XRef";
        if !reachable.contains(&id) && !infra {
            let entry = format!("{}: {desc}", id.0);
            unreferenced.push(entry.clone());
            if is_resource(&desc) {
                orphaned.push(entry);
            }
        }
        // Actions: dictionaries with /S or a /JS key
        let dict = match obj {
            Object::Dictionary(d) => Some(d),
            Object::Stream(s) => Some(&s.dict),
            _ => None,
        };
        if let Some(d) = dict {
            if d.get(b"JS").is_ok() {
                has_javascript = true;
            }
            if let Ok(s) = d.get(b"S").and_then(|s| s.as_name()) {
                let action = String::from_utf8_lossy(s).into_owned();
                match action.as_str() {
                    "JavaScript" => {
                        has_javascript = true;
                        push_unique(&mut suspicious, format!("JavaScript (objeto {})", id.0));
                    }
                    "Launch" | "URI" | "SubmitForm" | "ImportData" | "GoToR" => {
                        push_unique(&mut suspicious, format!("{action} (objeto {})", id.0));
                    }
                    _ => {}
                }
            }
            if let Ok(ft) = d.get(b"FT").and_then(|f| f.as_name()) {
                if ft == b"Sig" {
                    has_signature_fields = true;
                }
            }
        }
    }
    suspicious.sort();

    // XMP: Catalog -> /Metadata
    let xmp = pdf
        .catalog()
        .ok()
        .and_then(|cat| cat.get(b"Metadata").ok().cloned())
        .and_then(|m| match m {
            Object::Reference(id) => pdf.get_object(id).ok().cloned(),
            other => Some(other),
        })
        .and_then(|o| match o {
            Object::Stream(s) => {
                Some(String::from_utf8_lossy(&s.decompressed_content().unwrap_or(s.content.clone())).into_owned())
            }
            _ => None,
        })
        .unwrap_or_default();

    Ok(StructInfo {
        objects,
        unreferenced,
        orphaned,
        sizes,
        xmp,
        has_javascript,
        encrypted: pdf.trailer.get(b"Encrypt").is_ok(),
        suspicious_actions: suspicious,
        has_signature_fields,
    })
}

fn push_unique(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

fn meta(doc: &DocData, key: &str) -> String {
    doc.metadata
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.clone())
        .unwrap_or_default()
}

/// Converts the PDF date `D:20260802173622-03'00'` into `2026-08-02 17:36:22`.
/// If the format is unexpected, returns the original value.
fn format_pdf_date(raw: &str) -> String {
    let digits = raw.strip_prefix("D:").unwrap_or(raw);
    if digits.len() < 8 || !digits[..8].chars().all(|c| c.is_ascii_digit()) {
        return raw.to_string();
    }
    let part = |range: std::ops::Range<usize>, default: &str| {
        digits
            .get(range)
            .filter(|s| s.chars().all(|c| c.is_ascii_digit()))
            .unwrap_or(default)
            .to_string()
    };
    format!(
        "{}-{}-{} {}:{}:{}",
        &digits[..4],
        &digits[4..6],
        &digits[6..8],
        part(8..10, "00"),
        part(10..12, "00"),
        part(12..14, "00"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_of_real_fixture() {
        let info = analyze(std::path::Path::new("tests/fixtures/minimal.pdf")).unwrap();
        assert!(!info.objects.is_empty());
        // clean fixture: nothing suspicious, no encryption, no signature
        assert!(!info.has_javascript);
        assert!(!info.encrypted);
        assert!(!info.has_signature_fields);
        assert!(info.suspicious_actions.is_empty());
        // object 5 of the fixture is the content stream ("BT ... ET")
        assert!(info.sizes.get(&5).is_some_and(|s| *s > 10));
        // every object in the fixture is reachable — Info is in the trailer,
        // so everything is reachable:
        assert!(info.unreferenced.is_empty(), "{:?}", info.unreferenced);
    }

    #[test]
    fn analysis_detects_orphans_and_javascript() {
        // fixture with an orphan image (never referenced), a loose Launch action
        // and an OpenAction with JavaScript
        let info = analyze(std::path::Path::new("tests/fixtures/suspeito.pdf")).unwrap();
        assert!(info.has_javascript);
        assert!(info.suspicious_actions.iter().any(|s| s.starts_with("JavaScript")), "{:?}", info.suspicious_actions);
        // object 4 (the image) is an orphan; object 5 (Launch) is unreachable too
        assert!(info.orphaned.iter().any(|s| s.contains("/Image")), "{:?}", info.orphaned);
        assert!(info.unreferenced.len() >= 2, "{:?}", info.unreferenced);
        assert!(!info.encrypted);
    }

    #[test]
    fn pdf_dates() {
        assert_eq!(format_pdf_date("D:20260802173622-03'00'"), "2026-08-02 17:36:22");
        assert_eq!(format_pdf_date("D:20260802"), "2026-08-02 00:00:00");
        assert_eq!(format_pdf_date(""), "");
        assert_eq!(format_pdf_date("texto qualquer"), "texto qualquer");
    }
}
