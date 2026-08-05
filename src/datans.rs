//! Namespace `data::` — glossários e datasets locais.
//! Offline-first: tudo vem de arquivos locais, caminhos relativos ao
//! diretório de execução. As consultas a bases de referência
//! (query_gtin/query_medicamento/query_postal_code/validate_address) exigem
//! CSVs em ./dados/, ./pdfl_profiles/*/dados/, ./ ou $PDFL_DATA_DIR.

use crate::interpreter::{DocData, RuntimeError, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    /// Cache por caminho: scripts consultam o mesmo arquivo em loop.
    static GLOSSARIES: RefCell<HashMap<String, Rc<Vec<String>>>> = RefCell::new(HashMap::new());
    static DATASETS: RefCell<HashMap<String, Rc<Vec<Vec<String>>>>> = RefCell::new(HashMap::new());
}

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    match name {
        "load_glossary" => {
            let terms = glossary(&path_arg(args, name)?)?;
            Ok(Value::List(Rc::new(terms.iter().map(|t| Value::Str(t.clone())).collect())))
        }
        "load_dataset" => {
            let rows = dataset(&path_arg(args, name)?)?;
            Ok(Value::List(Rc::new(
                rows.iter()
                    .map(|r| Value::List(Rc::new(r.iter().map(|c| Value::Str(c.clone())).collect())))
                    .collect(),
            )))
        }
        "lookup_value" => {
            // lookup_value(arquivo.csv, chave) -> segunda coluna da linha cuja
            // primeira coluna é a chave; Null se não achar.
            let rows = dataset(&path_arg(args, name)?)?;
            let key = match args.get(1) {
                Some(v) => v.to_string(),
                None => return Err(err(format!("data::{name} expects the key as the second argument"))),
            };
            Ok(rows
                .iter()
                .find(|r| r.first() == Some(&key))
                .and_then(|r| r.get(1))
                .map(|v| Value::Str(v.clone()))
                .unwrap_or(Value::Null))
        }
        "validate_against_reference" => {
            // Termos do glossário que NÃO aparecem no texto do documento
            // (lista vazia = tudo presente). Comparação sem caixa/espaços.
            let terms = glossary(&path_arg(args, name)?)?;
            let text = normalize(
                &doc.pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n"),
            );
            let missing: Vec<Value> = terms
                .iter()
                .filter(|t| !text.contains(&normalize(t)))
                .map(|t| Value::Str(t.clone()))
                .collect();
            Ok(Value::List(Rc::new(missing)))
        }
        // ---- consultas a bases de referência locais ----
        // As bases são CSVs com cabeçalho, procurados em (nesta ordem):
        // ./dados/, ./pdfl_profiles/*/dados/, ./ e $PDFL_DATA_DIR.
        "query_gtin" => {
            let code: String =
                arg_str(args, 0, name)?.chars().filter(|c| c.is_ascii_digit()).collect();
            query_base(doc, "gtin.csv", &code, name)
        }
        "query_medicamento" => {
            let termo = arg_str(args, 0, name)?;
            query_base(doc, "medicamentos.csv", &termo, name)
        }
        "query_postal_code" => {
            let cep: String = arg_str(args, 0, name)?.chars().filter(|c| c.is_ascii_digit()).collect();
            if cep.len() != 8 {
                return Err(err(format!("data::{name}: postal code must have 8 digits (got \"{cep}\")")));
            }
            query_base(doc, "ceps.csv", &cep, name)
        }
        "validate_address" => {
            // validate_address(cep, "trecho do endereço") -> bool
            let cep: String = arg_str(args, 0, name)?.chars().filter(|c| c.is_ascii_digit()).collect();
            let esperado = arg_str(args, 1, name)?;
            let encontrado = match query_base(doc, "ceps.csv", &cep, name)? {
                Value::List(campos) => campos.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "),
                _ => String::new(),
            };
            if encontrado.is_empty() {
                return Ok(Value::Bool(false));
            }
            Ok(Value::Bool(normalize(&encontrado).contains(&normalize(&esperado))))
        }
        _ => Err(err(format!("unknown function: data::{name}"))),
    }
}

fn arg_str(args: &[Value], i: usize, name: &str) -> Result<String, RuntimeError> {
    match args.get(i) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(Value::Int(n)) => Ok(n.to_string()),
        _ => Err(err(format!("data::{name} expects a lookup value at position {}", i + 1))),
    }
}

/// Procura o arquivo da base nos diretórios padrão.
fn find_base(doc: &DocData, file: &str) -> Option<std::path::PathBuf> {
    let mut candidates: Vec<std::path::PathBuf> = vec![
        std::path::PathBuf::from("dados").join(file),
        std::path::PathBuf::from(file),
    ];
    if let Ok(dir) = std::env::var("PDFL_DATA_DIR") {
        candidates.insert(0, std::path::PathBuf::from(dir).join(file));
    }
    // perfis instalados por `pdfl add`
    if let Ok(entries) = std::fs::read_dir("pdfl_profiles") {
        for entry in entries.flatten() {
            candidates.push(entry.path().join("dados").join(file));
            candidates.push(entry.path().join(file));
        }
    }
    // ao lado do PDF analisado
    if let Some(parent) = doc.path.parent() {
        candidates.push(parent.join("dados").join(file));
        candidates.push(parent.join(file));
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// Busca a chave na 1ª coluna e devolve a linha inteira (lista), ou Null.
fn query_base(
    doc: &Rc<DocData>,
    file: &str,
    key: &str,
    name: &str,
) -> Result<Value, RuntimeError> {
    let path = find_base(doc, file).ok_or_else(|| {
        err(format!(
            "data::{name}: table \"{file}\" not found — put the file in ./dados/, \
             install a profile with `pdfl add` or set PDFL_DATA_DIR"
        ))
    })?;
    let rows = dataset(&path.to_string_lossy())?;
    let alvo = normalize(key);
    let achado = rows.iter().skip(1).find(|row| {
        row.first().is_some_and(|c| normalize(c) == alvo)
            // busca por termo também na 2ª coluna (nome do medicamento etc.)
            || (alvo.len() >= 3 && row.get(1).is_some_and(|c| normalize(c).contains(&alvo)))
    });
    Ok(match achado {
        Some(row) => Value::List(Rc::new(row.iter().map(|c| Value::Str(c.clone())).collect())),
        None => Value::Null,
    })
}

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

fn path_arg(args: &[Value], name: &str) -> Result<String, RuntimeError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(s.clone()),
        _ => Err(err(format!("data::{name} expects the file path (string)"))),
    }
}

fn normalize(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Glossário: um termo por linha; linhas vazias e começadas em # ignoradas.
fn glossary(path: &str) -> Result<Rc<Vec<String>>, RuntimeError> {
    GLOSSARIES.with(|cache| {
        if let Some(g) = cache.borrow().get(path) {
            return Ok(g.clone());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| err(format!("could not read glossary {path}: {e}")))?;
        let terms: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect();
        let rc = Rc::new(terms);
        cache.borrow_mut().insert(path.to_string(), rc.clone());
        Ok(rc)
    })
}

/// Dataset CSV com aspas padrão (campo entre aspas pode ter vírgula/quebra).
fn dataset(path: &str) -> Result<Rc<Vec<Vec<String>>>, RuntimeError> {
    DATASETS.with(|cache| {
        if let Some(d) = cache.borrow().get(path) {
            return Ok(d.clone());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| err(format!("could not read dataset {path}: {e}")))?;
        let rc = Rc::new(parse_csv(&content));
        cache.borrow_mut().insert(path.to_string(), rc.clone());
        Ok(rc)
    })
}

fn parse_csv(content: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let mut chars = content.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if in_quotes => {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    in_quotes = false;
                }
            }
            '"' if field.is_empty() => in_quotes = true,
            ',' if !in_quotes => row.push(std::mem::take(&mut field)),
            '\r' if !in_quotes => {}
            '\n' if !in_quotes => {
                row.push(std::mem::take(&mut field));
                if !(row.len() == 1 && row[0].is_empty()) {
                    rows.push(std::mem::take(&mut row));
                }
                row.clear();
            }
            _ => field.push(c),
        }
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_com_aspas() {
        let rows = parse_csv("a,b\n\"x, y\",\"com \"\"aspas\"\"\"\n\nfim,1\n");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[1], vec!["x, y", "com \"aspas\""]);
        assert_eq!(rows[2], vec!["fim", "1"]);
    }
}
