//! Tree-walking interpreter for the PDFLang language.

use crate::ast::*;
use crate::report::{Diagnostic, Severity};
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// ---- document data (filled in by pdf.rs, or mocked in the tests) ----

#[derive(Debug)]
pub struct DocData {
    pub filename: String,
    pub title: String,
    pub author: String,
    pub pages: Vec<Rc<PageData>>,
    pub fonts: Vec<Rc<FontData>>,
    /// Every metadata entry in the PDF's order: (key, value).
    pub metadata: Vec<(String, String)>,
    pub file_size: i64,
    pub sha256: String,
    /// Total content objects (text, image, path...) across the pages.
    pub object_count: i64,
    /// Path of the original file (for on-demand scans).
    pub path: std::path::PathBuf,
    /// Barcodes/QR codes — filled on demand on the first use of `codes::`
    /// (the scan renders the pages and is expensive).
    pub barcodes: std::cell::OnceCell<Vec<Rc<BarcodeData>>>,
    /// Low-level structural analysis (lopdf) — on demand, on the first use of
    /// the struct:: functions that need the object table.
    pub lowlevel: std::cell::OnceCell<crate::structns::StructInfo>,
    /// Color separations read from the content stream — on demand, on the first
    /// use of the prepress:: functions that need real color.
    pub colors: std::cell::OnceCell<crate::colors::ColorInfo>,
}

#[derive(Debug)]
pub struct BarcodeData {
    pub page_number: i64,    // 1-based
    pub format: String,      // EAN_13, QR_CODE, CODE_128, ...
    pub text: String,        // decoded content
    pub x: f64,              // position on the page, in points
    pub y: f64,
}

#[derive(Debug)]
pub struct PageData {
    pub index: i64,
    pub width: f64,  // pontos
    pub height: f64, // pontos
    pub text: String,
    pub images: Vec<Rc<ImageData>>,
    /// Approximate maximum TAC of the page, in % (0–400).
    /// ponytail: approximated via an RGB render + naive conversion to CMYK;
    /// exact TAC needs real separations: `prepress::calculate_exact_tac`
    pub tac_max: f64,
    /// Approximate average ink coverage, in %.
    pub ink_avg: f64,
    /// Smallest stroke width among the page's paths, in points.
    pub min_stroke_pt: Option<f64>,
    pub boxes: PageBoxes,
}

/// Page boxes in points: [left, bottom, right, top].
#[derive(Debug, Default)]
pub struct PageBoxes {
    pub media: Option<[f64; 4]>,
    pub crop: Option<[f64; 4]>,
    pub trim: Option<[f64; 4]>,
    pub bleed: Option<[f64; 4]>,
    pub art: Option<[f64; 4]>,
}

#[derive(Debug)]
pub struct ImageData {
    pub page_number: i64, // 1-based
    pub width: i64,       // pixels
    pub height: i64,      // pixels
    pub dpi_x: f64,       // effective DPI on the page
    pub dpi_y: f64,
    pub color_space: String, // DeviceRGB, DeviceCMYK, ...
    pub bits_per_pixel: i64,
}

#[derive(Debug)]
pub struct FontData {
    pub name: String,
    pub is_embedded: bool,
}

/// A declarative area on the page, in points (origin at the bottom-left
/// corner, as in the PDF).
#[derive(Debug, Clone)]
pub struct RegionData {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl RegionData {
    pub fn right(&self) -> f64 {
        self.x + self.width
    }
    pub fn top(&self) -> f64 {
        self.y + self.height
    }
    pub fn contains_point(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.right() && y >= self.y && y <= self.top()
    }
    pub fn intersects(&self, other: &RegionData) -> bool {
        self.x < other.right()
            && other.x < self.right()
            && self.y < other.top()
            && other.y < self.top()
    }
}

// ---- valores ----

#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    List(Rc<Vec<Value>>),
    Doc(Rc<DocData>),
    Page(Rc<PageData>),
    Font(Rc<FontData>),
    Image(Rc<ImageData>),
    Region(Rc<RegionData>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(n) => write!(f, "{n}"),
            Value::Float(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "{s}"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Doc(d) => write!(f, "<document {}>", d.filename),
            Value::Page(p) => write!(f, "<page {}>", p.index + 1),
            Value::Font(x) => write!(f, "<font {}>", x.name),
            Value::Image(img) => write!(f, "<image {}x{} on page {}>", img.width, img.height, img.page_number),
            Value::Region(r) => {
                let label = if r.name.is_empty() { String::new() } else { format!("{} ", r.name) };
                write!(f, "<region {label}{}x{} at ({}, {})>", r.width, r.height, r.x, r.y)
            }
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        !matches!(self, Value::Null | Value::Bool(false))
    }

    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Int(_) => "integer",
            Value::Float(_) => "number",
            Value::Str(_) => "string",
            Value::List(_) => "list",
            Value::Doc(_) => "document",
            Value::Page(_) => "page",
            Value::Font(_) => "font",
            Value::Image(_) => "image",
            Value::Region(_) => "region",
        }
    }
}

// ---- runtime error ----

#[derive(Debug)]
pub struct RuntimeError {
    pub message: String,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "runtime error: {}", self.message)
    }
}

impl std::error::Error for RuntimeError {}

fn rerr<T>(message: String) -> Result<T, RuntimeError> {
    Err(RuntimeError { message })
}

// ---- interpretador ----

pub struct Interpreter {
    scopes: Vec<HashMap<String, Value>>,
    pub diagnostics: Vec<Diagnostic>,
    pub profile_name: Option<String>,
    current_check: String,
    /// Severity a failing assertion reports, from the enclosing check.
    current_severity: Severity,
    /// How many times each (check, message) pair has already fired this run —
    /// the occurrence that disambiguates an otherwise identical finding.
    seen: HashMap<String, u32>,
    /// fix:: operations are only allowed in the `pdfl fix` command.
    pub allow_fixes: bool,
    pub fix_ops: Vec<crate::fixns::FixOp>,
    /// User-defined functions (a global registry).
    functions: HashMap<String, Rc<(Vec<String>, Vec<Stmt>)>>,
    call_depth: usize,
    /// The script's directory (the base for relative imports).
    pub script_dir: std::path::PathBuf,
    imported: std::collections::HashSet<std::path::PathBuf>,
}

impl Interpreter {
    pub fn new() -> Self {
        Interpreter {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            profile_name: None,
            current_check: String::new(),
            current_severity: Severity::Error,
            seen: HashMap::new(),
            allow_fixes: false,
            fix_ops: Vec::new(),
            functions: HashMap::new(),
            call_depth: 0,
            script_dir: std::path::PathBuf::from("."),
            imported: std::collections::HashSet::new(),
        }
    }

    /// Runs the program with `doc` available as a global variable.
    /// An error outside a check aborts everything; inside a check it becomes a
    /// diagnostic and execution continues at the next check.
    pub fn run(&mut self, program: &[Stmt], doc: Rc<DocData>) -> Result<(), RuntimeError> {
        self.scopes[0].insert("doc".into(), Value::Doc(doc));
        self.exec_stmts(program)
    }

    fn exec_stmts(&mut self, stmts: &[Stmt]) -> Result<(), RuntimeError> {
        for stmt in stmts {
            self.exec_stmt(stmt)?;
        }
        Ok(())
    }

    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Profile { name, body } => {
                self.profile_name = Some(name.clone());
                self.scopes.push(HashMap::new());
                let r = self.exec_stmts(body);
                self.scopes.pop();
                r
            }
            Stmt::Check { name, tags: _, severity, body } => {
                self.current_check = name.clone();
                self.current_severity = severity.clone();
                self.scopes.push(HashMap::new());
                if let Err(e) = self.exec_stmts(body) {
                    self.emit(Severity::Error, format!("error in check: {}", e.message), None);
                }
                self.scopes.pop();
                self.current_check.clear();
                self.current_severity = Severity::Error;
                Ok(())
            }
            Stmt::Rule { name, pages, body } => {
                // A rule is a check applied page by page, with `page` bound in
                // the body's scope.
                let previous_check = std::mem::replace(&mut self.current_check, name.clone());
                let selected: Vec<Value> = match pages {
                    Some(expr) => match self.eval(expr) {
                        Ok(Value::List(items)) => items.as_ref().clone(),
                        Ok(Value::Page(p)) => vec![Value::Page(p)],
                        Ok(other) => {
                            self.emit(
                                Severity::Error,
                                format!(
                                    "rule \"{name}\": 'on' expects a list of pages, got {}",
                                    other.type_name()
                                ),
                                None,
                            );
                            self.current_check = previous_check;
                            return Ok(());
                        }
                        Err(e) => {
                            self.emit(Severity::Error, format!("error in rule: {}", e.message), None);
                            self.current_check = previous_check;
                            return Ok(());
                        }
                    },
                    None => {
                        let doc = self.lookup_doc()?;
                        doc.pages.iter().map(|p| Value::Page(p.clone())).collect()
                    }
                };
                for page in selected {
                    self.scopes.push(HashMap::new());
                    self.scopes.last_mut().unwrap().insert("page".into(), page);
                    if let Err(e) = self.exec_stmts(body) {
                        self.emit(Severity::Error, format!("error in rule: {}", e.message), None);
                    }
                    self.scopes.pop();
                }
                self.current_check = previous_check;
                Ok(())
            }
            Stmt::Function { name, params, body } => {
                self.functions.insert(name.clone(), Rc::new((params.clone(), body.clone())));
                Ok(())
            }
            Stmt::Import { path } => {
                let full = self.script_dir.join(path);
                let canonical = full.canonicalize().map_err(|e| RuntimeError {
                    message: format!("import \"{path}\": could not open {}: {e}", full.display()),
                })?;
                if !self.imported.insert(canonical.clone()) {
                    return Ok(()); // already imported (this also breaks cycles)
                }
                let source = std::fs::read_to_string(&canonical)
                    .map_err(|e| RuntimeError { message: format!("import \"{path}\": {e}") })?;
                let program = crate::parser::parse(&source)
                    .map_err(|e| RuntimeError { message: format!("import \"{path}\": {e}") })?;
                // nested imports resolve relative to the imported file
                let previous_dir = std::mem::replace(
                    &mut self.script_dir,
                    canonical.parent().map(|d| d.to_path_buf()).unwrap_or_default(),
                );
                let result = self.exec_stmts(&program);
                self.script_dir = previous_dir;
                result
            }
            Stmt::Const { name, value } | Stmt::Assign { name, value } => {
                let v = self.eval(value)?;
                if let Stmt::Assign { .. } = stmt {
                    for scope in self.scopes.iter_mut().rev() {
                        if let Some(slot) = scope.get_mut(name) {
                            *slot = v;
                            return Ok(());
                        }
                    }
                }
                self.scopes.last_mut().unwrap().insert(name.clone(), v);
                Ok(())
            }
            Stmt::Assert { cond, message, source, line } => {
                let ok = self.eval(cond)?.truthy();
                if !ok {
                    let msg = match message {
                        Some(m) => self.eval(m)?.to_string(),
                        None => format!("requirement not met: {source}"),
                    };
                    self.emit(self.current_severity.clone(), msg, Some(*line));
                }
                Ok(())
            }
            Stmt::Expr(e) => {
                self.eval(e)?;
                Ok(())
            }
        }
    }

    fn emit(&mut self, severity: Severity, message: String, line: Option<usize>) {
        let key = format!("{}\u{1f}{}", self.current_check, message);
        let occurrence = self.seen.entry(key).and_modify(|n| *n += 1).or_insert(1);
        let id = crate::report::fingerprint(&self.current_check, &message, *occurrence);
        self.diagnostics.push(Diagnostic {
            id,
            severity,
            check_name: self.current_check.clone(),
            message,
            line,
        });
    }

    // ---- expression evaluation ----

    fn eval(&mut self, expr: &Expr) -> Result<Value, RuntimeError> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(n) => Ok(Value::Float(*n)),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Str(parts) => {
                let mut s = String::new();
                for p in parts {
                    match p {
                        StrPart::Lit(l) => s.push_str(l),
                        StrPart::Interp(e) => s.push_str(&self.eval(e)?.to_string()),
                    }
                }
                Ok(Value::Str(s))
            }
            Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for e in items {
                    out.push(self.eval(e)?);
                }
                Ok(Value::List(Rc::new(out)))
            }
            Expr::Ident(name) => {
                for scope in self.scopes.iter().rev() {
                    if let Some(v) = scope.get(name) {
                        return Ok(v.clone());
                    }
                }
                rerr(format!("unknown variable: {name}"))
            }
            Expr::Member { recv, name } => {
                let v = self.eval(recv)?;
                self.member(&v, name)
            }
            Expr::Unary { op, expr } => {
                let v = self.eval(expr)?;
                match op {
                    UnOp::Not => Ok(Value::Bool(!v.truthy())),
                    UnOp::Neg => match v {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(n) => Ok(Value::Float(-n)),
                        other => rerr(format!("cannot negate {}", other.type_name())),
                    },
                }
            }
            Expr::Binary { op, left, right } => self.binary(*op, left, right),
            Expr::Call { recv, name, args, block } => {
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    arg_vals.push(self.eval(&a.value)?);
                }
                match recv {
                    Some(r) => {
                        let v = self.eval(r)?;
                        self.method(&v, name, &arg_vals, block.as_ref())
                    }
                    None => self.global_fn(name, &arg_vals),
                }
            }
        }
    }

    fn binary(&mut self, op: BinOp, left: &Expr, right: &Expr) -> Result<Value, RuntimeError> {
        // Curto-circuito
        match op {
            BinOp::And => {
                let l = self.eval(left)?;
                return if l.truthy() { self.eval(right) } else { Ok(l) };
            }
            BinOp::Or => {
                let l = self.eval(left)?;
                return if l.truthy() { Ok(l) } else { self.eval(right) };
            }
            _ => {}
        }
        let l = self.eval(left)?;
        let r = self.eval(right)?;
        match op {
            BinOp::Eq => Ok(Value::Bool(values_eq(&l, &r))),
            BinOp::NotEq => Ok(Value::Bool(!values_eq(&l, &r))),
            BinOp::Add => match (&l, &r) {
                (Value::Str(a), b) => Ok(Value::Str(format!("{a}{b}"))),
                _ => numeric(op, &l, &r),
            },
            BinOp::Sub | BinOp::Mul | BinOp::Div => numeric(op, &l, &r),
            BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => {
                let (a, b) = to_floats(&l, &r)?;
                Ok(Value::Bool(match op {
                    BinOp::Lt => a < b,
                    BinOp::LtEq => a <= b,
                    BinOp::Gt => a > b,
                    BinOp::GtEq => a >= b,
                    _ => unreachable!(),
                }))
            }
            BinOp::And | BinOp::Or => unreachable!(),
        }
    }

    // ---- membros (propriedades) ----

    fn member(&mut self, v: &Value, name: &str) -> Result<Value, RuntimeError> {
        match (v, name) {
            (Value::Doc(d), "page_count") => Ok(Value::Int(d.pages.len() as i64)),
            (Value::Doc(d), "title") => Ok(Value::Str(d.title.clone())),
            (Value::Doc(d), "author") => Ok(Value::Str(d.author.clone())),
            (Value::Doc(d), "filename") => Ok(Value::Str(d.filename.clone())),
            (Value::Doc(d), "pages") => {
                Ok(Value::List(Rc::new(d.pages.iter().map(|p| Value::Page(p.clone())).collect())))
            }
            (Value::Doc(d), "fonts") => {
                Ok(Value::List(Rc::new(d.fonts.iter().map(|x| Value::Font(x.clone())).collect())))
            }
            (Value::Doc(d), "images") => Ok(Value::List(Rc::new(
                d.pages.iter().flat_map(|p| p.images.iter().map(|i| Value::Image(i.clone()))).collect(),
            ))),
            (Value::Page(p), "images") => {
                Ok(Value::List(Rc::new(p.images.iter().map(|i| Value::Image(i.clone())).collect())))
            }
            (Value::Image(i), "width") => Ok(Value::Int(i.width)),
            (Value::Image(i), "height") => Ok(Value::Int(i.height)),
            (Value::Image(i), "dpi") => Ok(Value::Float(i.dpi_x.min(i.dpi_y))),
            (Value::Image(i), "dpi_x") => Ok(Value::Float(i.dpi_x)),
            (Value::Image(i), "dpi_y") => Ok(Value::Float(i.dpi_y)),
            (Value::Image(i), "color_space") => Ok(Value::Str(i.color_space.clone())),
            (Value::Image(i), "page_number") => Ok(Value::Int(i.page_number)),
            (Value::Image(i), "bits_per_pixel") => Ok(Value::Int(i.bits_per_pixel)),
            (Value::Region(r), "name") => Ok(Value::Str(r.name.clone())),
            (Value::Region(r), "x") => Ok(Value::Float(r.x)),
            (Value::Region(r), "y") => Ok(Value::Float(r.y)),
            (Value::Region(r), "width") => Ok(Value::Float(r.width)),
            (Value::Region(r), "height") => Ok(Value::Float(r.height)),
            (Value::Region(r), "right") => Ok(Value::Float(r.right())),
            (Value::Region(r), "top") => Ok(Value::Float(r.top())),
            (Value::Region(r), "area") => Ok(Value::Float(r.width * r.height)),
            (Value::Page(p), "tac") => Ok(Value::Float(p.tac_max)),
            (Value::Page(p), "ink_coverage") => Ok(Value::Float(p.ink_avg)),
            (Value::Page(p), "min_stroke_width") => {
                Ok(p.min_stroke_pt.map(Value::Float).unwrap_or(Value::Null))
            }
            (Value::Page(p), "has_media_box") => Ok(Value::Bool(p.boxes.media.is_some())),
            (Value::Page(p), "has_crop_box") => Ok(Value::Bool(p.boxes.crop.is_some())),
            (Value::Page(p), "has_trim_box") => Ok(Value::Bool(p.boxes.trim.is_some())),
            (Value::Page(p), "has_bleed_box") => Ok(Value::Bool(p.boxes.bleed.is_some())),
            (Value::Page(p), "has_art_box") => Ok(Value::Bool(p.boxes.art.is_some())),
            (Value::Page(p), "index") => Ok(Value::Int(p.index)),
            (Value::Page(p), "number") => Ok(Value::Int(p.index + 1)),
            (Value::Page(p), "width") => Ok(Value::Float(p.width)),
            (Value::Page(p), "height") => Ok(Value::Float(p.height)),
            (Value::Font(x), "name") => Ok(Value::Str(x.name.clone())),
            (Value::Font(x), "is_embedded") => Ok(Value::Bool(x.is_embedded)),
            (Value::List(items), "length") => Ok(Value::Int(items.len() as i64)),
            (Value::Str(s), "length") => Ok(Value::Int(s.chars().count() as i64)),
            _ => rerr(format!("{} has no property '{name}'", v.type_name())),
        }
    }

    // ---- methods ----

    fn method(
        &mut self,
        v: &Value,
        name: &str,
        args: &[Value],
        block: Option<&Block>,
    ) -> Result<Value, RuntimeError> {
        match (v, name) {
            // Listas
            (Value::List(items), "each") => {
                let b = need_block(block, "each")?;
                for item in items.iter() {
                    self.run_block(b, &[item.clone()])?;
                }
                Ok(Value::Null)
            }
            (Value::List(items), "each_with_index") => {
                let b = need_block(block, "each_with_index")?;
                for (i, item) in items.iter().enumerate() {
                    self.run_block(b, &[item.clone(), Value::Int(i as i64)])?;
                }
                Ok(Value::Null)
            }
            (Value::List(items), "all") => {
                let b = need_block(block, "all")?;
                for item in items.iter() {
                    if !self.run_block(b, &[item.clone()])?.truthy() {
                        return Ok(Value::Bool(false));
                    }
                }
                Ok(Value::Bool(true))
            }
            (Value::List(items), "any") => {
                let b = need_block(block, "any")?;
                for item in items.iter() {
                    if self.run_block(b, &[item.clone()])?.truthy() {
                        return Ok(Value::Bool(true));
                    }
                }
                Ok(Value::Bool(false))
            }
            (Value::List(items), "filter") => {
                let b = need_block(block, "filter")?;
                let mut out = Vec::new();
                for item in items.iter() {
                    if self.run_block(b, &[item.clone()])?.truthy() {
                        out.push(item.clone());
                    }
                }
                Ok(Value::List(Rc::new(out)))
            }
            (Value::List(items), "map") => {
                let b = need_block(block, "map")?;
                let mut out = Vec::with_capacity(items.len());
                for item in items.iter() {
                    out.push(self.run_block(b, &[item.clone()])?);
                }
                Ok(Value::List(Rc::new(out)))
            }
            (Value::List(items), "length") => Ok(Value::Int(items.len() as i64)),
            (Value::List(items), "get") => {
                // 1-based, meant for non-programmers: get(1) is the first one
                let n = match one_arg(args, "get")? {
                    Value::Int(n) => *n,
                    other => return rerr(format!("get expects the item number, got {}", other.type_name())),
                };
                if n < 1 || n as usize > items.len() {
                    return rerr(format!("item {n} does not exist (the list has {})", items.len()));
                }
                Ok(items[(n - 1) as usize].clone())
            }
            (Value::List(items), "first") => Ok(items.first().cloned().unwrap_or(Value::Null)),
            (Value::List(items), "last") => Ok(items.last().cloned().unwrap_or(Value::Null)),
            (Value::List(items), "contains") => {
                let target = one_arg(args, "contains")?;
                Ok(Value::Bool(items.iter().any(|i| values_eq(i, target))))
            }
            (Value::List(items), "join") => {
                let sep = match args.first() {
                    Some(Value::Str(s)) => s.clone(),
                    None => ", ".into(),
                    Some(other) => return rerr(format!("join expects a string, got {}", other.type_name())),
                };
                Ok(Value::Str(items.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(&sep)))
            }
            // Strings
            (Value::Str(s), "contains") => match one_arg(args, "contains")? {
                Value::Str(sub) => Ok(Value::Bool(s.contains(sub.as_str()))),
                other => rerr(format!("contains expects a string, got {}", other.type_name())),
            },
            (Value::Str(s), "starts_with") => match one_arg(args, "starts_with")? {
                Value::Str(sub) => Ok(Value::Bool(s.starts_with(sub.as_str()))),
                other => rerr(format!("starts_with expects a string, got {}", other.type_name())),
            },
            (Value::Str(s), "ends_with") => match one_arg(args, "ends_with")? {
                Value::Str(sub) => Ok(Value::Bool(s.ends_with(sub.as_str()))),
                other => rerr(format!("ends_with expects a string, got {}", other.type_name())),
            },
            (Value::Str(s), "trim") => Ok(Value::Str(s.trim().to_string())),
            (Value::Str(s), "to_uppercase") => Ok(Value::Str(s.to_uppercase())),
            (Value::Str(s), "to_lowercase") => Ok(Value::Str(s.to_lowercase())),
            (Value::Str(s), "length") => Ok(Value::Int(s.chars().count() as i64)),
            // Regions
            (Value::Region(r), "contains_point") => {
                let num = |i: usize| match args.get(i) {
                    Some(Value::Int(v)) => Ok(*v as f64),
                    Some(Value::Float(v)) => Ok(*v),
                    _ => rerr("contains_point expects x and y".to_string()),
                };
                Ok(Value::Bool(r.contains_point(num(0)?, num(1)?)))
            }
            (Value::Region(r), "intersects") => match args.first() {
                Some(Value::Region(other)) => Ok(Value::Bool(r.intersects(other))),
                _ => rerr("intersects expects another region".into()),
            },
            (Value::Region(r), "expand") => {
                let by = match args.first() {
                    Some(Value::Int(v)) => *v as f64,
                    Some(Value::Float(v)) => *v,
                    _ => return rerr("expand expects the margin in points".into()),
                };
                Ok(Value::Region(Rc::new(RegionData {
                    name: r.name.clone(),
                    x: r.x - by,
                    y: r.y - by,
                    width: r.width + 2.0 * by,
                    height: r.height + 2.0 * by,
                })))
            }
            (Value::Region(r), "inset") => {
                let by = match args.first() {
                    Some(Value::Int(v)) => *v as f64,
                    Some(Value::Float(v)) => *v,
                    _ => return rerr("inset expects the margin in points".into()),
                };
                if r.width - 2.0 * by <= 0.0 || r.height - 2.0 * by <= 0.0 {
                    return rerr(format!("an inset of {by}pt would leave the region with no area"));
                }
                Ok(Value::Region(Rc::new(RegionData {
                    name: r.name.clone(),
                    x: r.x + by,
                    y: r.y + by,
                    width: r.width - 2.0 * by,
                    height: r.height - 2.0 * by,
                })))
            }
            (Value::Region(r), "export_coordinates") => Ok(Value::List(Rc::new(vec![
                Value::Float(r.x),
                Value::Float(r.y),
                Value::Float(r.right()),
                Value::Float(r.top()),
            ]))),
            // Document / page
            (Value::Page(p), "extract_text") => Ok(Value::Str(p.text.clone())),
            (Value::Doc(d), "extract_text") => {
                Ok(Value::Str(d.pages.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n")))
            }
            // A property called as a no-arg method: doc.pages() etc.
            (_, _) if args.is_empty() && block.is_none() => self.member(v, name),
            _ => rerr(format!("{} has no method '{name}'", v.type_name())),
        }
    }

    fn global_fn(&mut self, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
        // User functions take precedence (namespaces use :: and do not collide)
        if let Some(def) = self.functions.get(name).cloned() {
            if self.call_depth >= 200 {
                return rerr(format!("recursion too deep in '{name}' (limit: 200 calls)"));
            }
            let (params, body) = def.as_ref();
            self.call_depth += 1;
            self.scopes.push(HashMap::new());
            for (i, param) in params.iter().enumerate() {
                let v = args.get(i).cloned().unwrap_or(Value::Null);
                self.scopes.last_mut().unwrap().insert(param.clone(), v);
            }
            let mut result = Ok(Value::Null);
            for stmt in body {
                result = match stmt {
                    Stmt::Expr(e) => self.eval(e),
                    other => self.exec_stmt(other).map(|_| Value::Null),
                };
                if result.is_err() {
                    break;
                }
            }
            self.scopes.pop();
            self.call_depth -= 1;
            return result;
        }
        if let Some(func) = name.strip_prefix("text::") {
            let doc = self.lookup_doc()?;
            return crate::textns::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("struct::") {
            let doc = self.lookup_doc()?;
            return crate::structns::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("visual::") {
            let doc = self.lookup_doc()?;
            return crate::visualns::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("prepress::") {
            let doc = self.lookup_doc()?;
            return crate::prepressns::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("codes::") {
            let doc = self.lookup_doc()?;
            return crate::codesns::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("data::") {
            let doc = self.lookup_doc()?;
            return crate::datans::call(&doc, func, args);
        }
        if let Some(func) = name.strip_prefix("fix::") {
            if !self.allow_fixes {
                return rerr("fix:: is only available in the 'pdfl fix' command (which saves a new PDF)".into());
            }
            let doc = self.lookup_doc()?;
            let op = crate::fixns::queue(&doc, func, args)?;
            self.fix_ops.push(op);
            return Ok(Value::Bool(true));
        }
        if let Some((ns, _)) = name.split_once("::") {
            return rerr(format!("unknown namespace: {ns}"));
        }
        match name {
            "min" | "max" => {
                if args.len() != 2 {
                    return rerr(format!("{name} expects 2 arguments"));
                }
                let (a, b) = to_floats(&args[0], &args[1])?;
                let pick_first = if name == "min" { a <= b } else { a >= b };
                Ok(args[if pick_first { 0 } else { 1 }].clone())
            }
            "abs" => match one_arg(args, "abs")? {
                Value::Int(n) => Ok(Value::Int(n.abs())),
                Value::Float(n) => Ok(Value::Float(n.abs())),
                other => rerr(format!("abs expects a number, got {}", other.type_name())),
            },
            "round" => match one_arg(args, "round")? {
                Value::Int(n) => Ok(Value::Int(*n)),
                Value::Float(n) => Ok(Value::Int(n.round() as i64)),
                other => rerr(format!("round expects a number, got {}", other.type_name())),
            },
            "region" => {
                // region(x, y, width, height [, "name"]) — in points
                let n = |i: usize| -> Result<f64, RuntimeError> {
                    match args.get(i) {
                        Some(Value::Int(v)) => Ok(*v as f64),
                        Some(Value::Float(v)) => Ok(*v),
                        _ => rerr("region expects 4 numbers: x, y, width, height".into()),
                    }
                };
                let (x, y, width, height) = (n(0)?, n(1)?, n(2)?, n(3)?);
                if width <= 0.0 || height <= 0.0 {
                    return rerr("region: width and height must be positive".into());
                }
                let name = match args.get(4) {
                    Some(Value::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                Ok(Value::Region(Rc::new(RegionData { name, x, y, width, height })))
            }
            "print" => {
                // stderr: stdout is reserved for the report (JSON/CSV/HTML)
                eprintln!("{}", args.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(" "));
                Ok(Value::Null)
            }
            _ => rerr(format!("unknown function: {name}")),
        }
    }

    fn lookup_doc(&self) -> Result<Rc<DocData>, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(Value::Doc(d)) = scope.get("doc") {
                return Ok(d.clone());
            }
        }
        rerr("no document loaded (variable 'doc' not found)".into())
    }

    /// Runs a `{ |params| ... }` block; returns the value of the last statement
    /// if it is an expression, otherwise Null.
    fn run_block(&mut self, block: &Block, args: &[Value]) -> Result<Value, RuntimeError> {
        self.scopes.push(HashMap::new());
        for (i, param) in block.params.iter().enumerate() {
            let v = args.get(i).cloned().unwrap_or(Value::Null);
            self.scopes.last_mut().unwrap().insert(param.clone(), v);
        }
        let mut result = Value::Null;
        let mut err = None;
        for stmt in &block.body {
            let r = match stmt {
                Stmt::Expr(e) => self.eval(e).map(|v| result = v),
                other => self.exec_stmt(other).map(|_| result = Value::Null),
            };
            if let Err(e) = r {
                err = Some(e);
                break;
            }
        }
        self.scopes.pop();
        match err {
            Some(e) => Err(e),
            None => Ok(result),
        }
    }
}

// ---- auxiliares ----

fn need_block<'a>(block: Option<&'a Block>, name: &str) -> Result<&'a Block, RuntimeError> {
    block.ok_or_else(|| RuntimeError { message: format!("{name} requires a block {{ |x| ... }}") })
}

fn one_arg<'a>(args: &'a [Value], name: &str) -> Result<&'a Value, RuntimeError> {
    if args.len() == 1 {
        Ok(&args[0])
    } else {
        rerr(format!("{name} expects 1 argument, got {}", args.len()))
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| values_eq(a, b))
        }
        _ => match (as_float(a), as_float(b)) {
            (Some(x), Some(y)) => x == y,
            _ => false,
        },
    }
}

fn as_float(v: &Value) -> Option<f64> {
    match v {
        Value::Int(n) => Some(*n as f64),
        Value::Float(n) => Some(*n),
        _ => None,
    }
}

fn to_floats(l: &Value, r: &Value) -> Result<(f64, f64), RuntimeError> {
    match (as_float(l), as_float(r)) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => rerr(format!(
            "numeric operation between {} and {} is not valid",
            l.type_name(),
            r.type_name()
        )),
    }
}

fn numeric(op: BinOp, l: &Value, r: &Value) -> Result<Value, RuntimeError> {
    if let (Value::Int(a), Value::Int(b)) = (l, r) {
        return match op {
            BinOp::Add => Ok(Value::Int(a + b)),
            BinOp::Sub => Ok(Value::Int(a - b)),
            BinOp::Mul => Ok(Value::Int(a * b)),
            BinOp::Div => {
                if *b == 0 {
                    rerr("division by zero".into())
                } else if a % b == 0 {
                    Ok(Value::Int(a / b))
                } else {
                    Ok(Value::Float(*a as f64 / *b as f64))
                }
            }
            _ => unreachable!(),
        };
    }
    let (a, b) = to_floats(l, r)?;
    match op {
        BinOp::Add => Ok(Value::Float(a + b)),
        BinOp::Sub => Ok(Value::Float(a - b)),
        BinOp::Mul => Ok(Value::Float(a * b)),
        BinOp::Div => {
            if b == 0.0 {
                rerr("division by zero".into())
            } else {
                Ok(Value::Float(a / b))
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    /// Serializes the tests that touch environment variables (globals).
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn mock_doc() -> Rc<DocData> {
        Rc::new(DocData {
            filename: "test.pdf".into(),
            title: "Document Title".into(),
            author: "".into(),
            pages: vec![
                Rc::new(PageData {
                    index: 0,
                    width: 595.0,
                    height: 842.0,
                    text: "Hello world".into(),
                    images: vec![
                        Rc::new(ImageData {
                            page_number: 1,
                            width: 1200,
                            height: 800,
                            dpi_x: 350.0,
                            dpi_y: 350.0,
                            color_space: "DeviceCMYK".into(),
                            bits_per_pixel: 32,
                        }),
                        Rc::new(ImageData {
                            page_number: 1,
                            width: 100,
                            height: 100,
                            dpi_x: 72.0,
                            dpi_y: 72.0,
                            color_space: "DeviceRGB".into(),
                            bits_per_pixel: 24,
                        }),
                    ],
                    tac_max: 280.0,
                    ink_avg: 42.0,
                    min_stroke_pt: Some(0.1),
                    boxes: PageBoxes {
                        media: Some([0.0, 0.0, 595.0, 842.0]),
                        crop: None,
                        trim: Some([8.5, 8.5, 586.5, 833.5]),
                        bleed: Some([0.0, 0.0, 595.0, 842.0]),
                        art: None,
                    },
                }),
                Rc::new(PageData {
                    index: 1,
                    width: 595.0,
                    height: 842.0,
                    text: "".into(),
                    images: vec![],
                    tac_max: 320.0,
                    ink_avg: 55.0,
                    min_stroke_pt: None,
                    boxes: PageBoxes {
                        media: Some([0.0, 0.0, 595.0, 842.0]),
                        ..Default::default()
                    },
                }),
            ],
            fonts: vec![
                Rc::new(FontData { name: "Helvetica".into(), is_embedded: true }),
                Rc::new(FontData { name: "Arial".into(), is_embedded: false }),
            ],
            metadata: vec![
                ("Title".into(), "Document Title".into()),
                ("Author".into(), "".into()),
                ("Producer".into(), "TestPDF 1.0".into()),
                ("CreationDate".into(), "D:20260802173622-03'00'".into()),
            ],
            file_size: 4096,
            sha256: "abc123".into(),
            object_count: 7,
            path: std::path::PathBuf::new(),
            barcodes: {
                let cell = std::cell::OnceCell::new();
                cell.set(vec![
                    Rc::new(BarcodeData {
                        page_number: 1,
                        format: "EAN_13".into(),
                        text: "7891234567895".into(),
                        x: 150.0,
                        y: 650.0,
                    }),
                    Rc::new(BarcodeData {
                        page_number: 2,
                        format: "QR_CODE".into(),
                        text: "https://exemplo.com".into(),
                        x: 300.0,
                        y: 400.0,
                    }),
                ])
                .unwrap();
                cell
            },
            lowlevel: std::cell::OnceCell::new(),
            colors: std::cell::OnceCell::new(),
        })
    }

    fn run(src: &str) -> Interpreter {
        let prog = parse(src).unwrap();
        let mut interp = Interpreter::new();
        interp.run(&prog, mock_doc()).unwrap();
        interp
    }

    #[test]
    fn assert_passes_and_fails() {
        let i = run(r#"
check "Pages" {
  require doc.page_count > 0
  assert doc.page_count > 10, "expected more than 10 pages"
}
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert_eq!(i.diagnostics[0].message, "expected more than 10 pages");
        assert_eq!(i.diagnostics[0].check_name, "Pages");
        assert!(i.diagnostics[0].id.starts_with("PDFL-"), "{}", i.diagnostics[0].id);
    }

    #[test]
    fn check_declares_the_severity_of_its_findings() {
        let i = run(r#"
check "Advisory" severity: warning {
  assert false, "low resolution image"
}
check "Blocking" {
  assert false, "no title"
}
"#);
        assert_eq!(i.diagnostics.len(), 2);
        assert_eq!(i.diagnostics[0].severity, Severity::Warning);
        // Without a declaration the default stays Error, so existing scripts
        // keep the exit code they had.
        assert_eq!(i.diagnostics[1].severity, Severity::Error);
    }

    #[test]
    fn severity_accepts_info_and_survives_alongside_tags() {
        let i = run(r#"
check "Note" tags: ["x"] severity: info {
  assert false, "just so you know"
}
check "Other" severity: info tags: ["y"] {
  assert false, "either order parses"
}
"#);
        assert_eq!(i.diagnostics.len(), 2);
        assert!(i.diagnostics.iter().all(|d| d.severity == Severity::Info));
    }

    /// A broken script is not advisory: a runtime error inside a check stays an
    /// error even when the check declared itself a warning.
    #[test]
    fn a_runtime_error_ignores_the_declared_severity() {
        let i = run(r#"
check "Advisory" severity: warning {
  require nonexistent_variable
}
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert_eq!(i.diagnostics[0].severity, Severity::Error);
    }

    /// The property the identifier exists for: it names the finding, so it
    /// survives an edit that only moves the finding down the report.
    #[test]
    fn diagnostic_id_survives_a_check_inserted_above() {
        let before = run(r#"
check "Fonts" {
  assert false, "Font Arial is not embedded"
}
"#);
        let after = run(r#"
check "Pages" {
  assert false, "too few pages"
}
check "Fonts" {
  assert false, "Font Arial is not embedded"
}
"#);
        assert_eq!(after.diagnostics.len(), 2);
        // The Fonts finding moved from first to second and kept its identity;
        // a positional counter would have renamed it from 001 to 002.
        assert_eq!(before.diagnostics[0].id, after.diagnostics[1].id);
        assert_ne!(after.diagnostics[0].id, after.diagnostics[1].id);
    }

    /// Two findings that differ only in the value inside the message are
    /// different findings — approving one must not approve the other.
    #[test]
    fn diagnostic_id_distinguishes_the_interpolated_value() {
        let i = run(r#"
check "Fonts" {
  assert false, "Font Arial is not embedded"
  assert false, "Font Helvetica is not embedded"
}
"#);
        assert_eq!(i.diagnostics.len(), 2);
        assert_ne!(i.diagnostics[0].id, i.diagnostics[1].id);
    }

    /// The honest collision: the same check failing twice with the identical
    /// message. Without an occurrence counter both would share an identity and
    /// a baseline approving the first would silence the second.
    #[test]
    fn identical_findings_get_distinct_ids() {
        let i = run(r#"
check "Boxes" {
  assert false, "TrimBox missing"
  assert false, "TrimBox missing"
}
"#);
        assert_eq!(i.diagnostics.len(), 2);
        assert_eq!(i.diagnostics[0].message, i.diagnostics[1].message);
        assert_ne!(i.diagnostics[0].id, i.diagnostics[1].id);
    }

    #[test]
    fn require_generates_automatic_message() {
        let i = run("check \"Autor\" { require doc.author != \"\" }");
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("doc.author != \"\""));
    }

    #[test]
    fn each_with_interpolation() {
        let i = run(r#"
check "Fontes" {
  doc.fonts.each { |font|
    assert font.is_embedded, "Font #{font.name} is not embedded"
  }
}
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert_eq!(i.diagnostics[0].message, "Font Arial is not embedded");
    }

    #[test]
    fn profile_const_and_arithmetic() {
        let i = run(r#"
profile "teste" {
  const MIN = 2 * 300
  check "Dimensions" {
    doc.pages.each { |p|
      require p.width * 2 > MIN
    }
  }
}
"#);
        assert_eq!(i.profile_name.as_deref(), Some("teste"));
        assert!(i.diagnostics.is_empty());
    }

    #[test]
    fn all_any_filter() {
        let i = run(r#"
check "Listas" {
  require doc.fonts.all { |f| f.is_embedded }
  require doc.fonts.any { |f| f.is_embedded }
  require doc.fonts.filter { |f| !f.is_embedded }.length == 1
}
"#);
        assert_eq!(i.diagnostics.len(), 1); // only `all` fails
        assert!(i.diagnostics[0].message.contains("all"));
    }

    #[test]
    fn text_and_strings() {
        let i = run(r#"
check "Texto" {
  page1 = doc.pages.filter { |p| p.index == 0 }
  require doc.extract_text().contains("Hello")
  assert doc.title.length > 0, "no title"
}
"#);
        assert!(i.diagnostics.is_empty());
    }

    #[test]
    fn error_in_one_check_does_not_stop_the_next() {
        let i = run(r#"
check "Quebrado" { require variavel_inexistente > 0 }
check "OK" { require doc.page_count == 2 }
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("variavel_inexistente"));
        assert_eq!(i.diagnostics[0].check_name, "Quebrado");
    }

    #[test]
    fn namespace_text_basics() {
        let i = run(r#"
check "Texto" {
  require text::require_text("hello WORLD")
  require text::forbid_text("forbidden word")
  require text::count_words() == 2
  require text::count_characters("abc") == 3
  require text::extract_from_page(1).contains("Hello")
  require text::split_words().length == 2
  require text::normalize("  HELLO   World ") == "hello world"
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_text_regex_and_fuzzy() {
        let i = run(r#"
check "Patterns" {
  require text::require_match("Hello \w+")
  require text::forbid_match("\d{3}-\d{2}")
  require text::fuzzy_match("paracetamol", "paracetamol") == 1.0
  require text::fuzzy_match("dipirona", "dip1rona") > 0.8
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_text_pii() {
        let i = run(r#"
check "PII" {
  achados = text::detect_personal_data("CPF 529.982.247-25 e email x@y.com")
  require achados.length == 2
  require text::detect_pii().length == 0
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_text_friendly_errors() {
        let i = run(r#"check "Erro" { text::extract_from_page(99) }"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("page 99 does not exist"));

        let i = run(r#"check "Erro" { nuvem::consultar() }"#);
        assert!(i.diagnostics[0].message.contains("unknown namespace: nuvem"));
    }

    #[test]
    fn namespace_struct_metadata() {
        let i = run(r#"
check "Metadata" {
  require struct::get_title() == "Document Title"
  require struct::get_producer() == "TestPDF 1.0"
  require struct::get_creation_date() == "2026-08-02 17:36:22"
  require struct::get_author() == ""
  require struct::list_metadata_entries().length == 3
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_struct_objects_and_hash() {
        let i = run(r#"
check "Estrutura" {
  require struct::count_objects() == 7
  require struct::file_size() == 4096
  require struct::calculate_sha256() == "abc123"
  require !struct::detect_file_bloat()
  require struct::detect_file_bloat(1)
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_visual_basics() {
        let i = run(r#"
check "Imagens" {
  require visual::detect_images()
  require visual::count_images() == 2
  require visual::get_image_resolution(1) == 350.0
  require visual::get_image_size(2).contains(100)
  require visual::detect_image_color_space().contains("DeviceCMYK")
  require visual::detect_image_color_space(2) == "DeviceRGB"
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_visual_low_resolution() {
        let i = run(r#"
check "Resolution" {
  require visual::detect_low_resolution()
  require visual::detect_low_resolution(300)
  require !visual::detect_low_resolution(50)
  ruins = doc.images.filter { |img| img.dpi < 300 }
  ruins.each { |img|
    assert false, "Image #{img.width}x#{img.height} on page #{img.page_number}: #{img.dpi} DPI"
  }
}
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert_eq!(i.diagnostics[0].message, "Image 100x100 on page 1: 72 DPI");
    }

    #[test]
    fn namespace_visual_friendly_errors() {
        // mock_doc has no real file: the visual functions must fail with a
        // clear message, not a panic
        let i = run(r#"check "Erro" { visual::measure_ssim(1) }"#);
        assert!(i.diagnostics[0].message.contains("path of the other PDF"), "{:?}", i.diagnostics);

        let i = run(r#"check "Erro" { visual::calculate_perceptual_hash(99) }"#);
        assert!(i.diagnostics[0].message.contains("page 99 does not exist"), "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_visual_errors() {
        let i = run(r#"check "Erro" { visual::get_image_size(9) }"#);
        assert!(i.diagnostics[0].message.contains("image 9 does not exist"));
    }

    #[test]
    fn namespace_prepress_tac() {
        let i = run(r#"
check "TAC" {
  require prepress::calculate_tac() == 320.0
  require prepress::calculate_tac(1) == 280.0
  require prepress::calculate_ink_coverage(1) == 42.0
  require !prepress::validate_tac_limits(300)
  require prepress::validate_tac_limits(350)
  doc.pages.each { |page|
    assert prepress::calculate_tac(page) <= 320.0, "high TAC on page #{page.number}"
  }
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_prepress_lines_and_fonts() {
        let i = run(r#"
check "Linhas" {
  require prepress::detect_hairlines()
  require prepress::detect_hairlines(0.25)
  require !prepress::detect_hairlines(0.05)
  require !prepress::validate_minimum_stroke_width(0.5)
  require prepress::list_fonts().contains("Arial")
  require !prepress::validate_font_embedding()
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_prepress_boxes() {
        let i = run(r#"
check "Caixas" {
  require prepress::validate_media_box()
  require !prepress::validate_trim_box()
  require prepress::get_page_size(1).contains(595.0)
  require prepress::get_page_boxes(1).length == 3
  require doc.pages.filter { |p| p.has_trim_box }.length == 1
  page1 = doc.pages.filter { |p| p.index == 0 }
  page1.each { |p| require p.tac == 280.0 }
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_prepress_delta_e() {
        // delta_e does not need the file: it compares colors given in the script
        let i = run(r#"
check "Cores" {
  require prepress::compare_colors_delta_e([0, 0, 0, 1], [0, 0, 0, 1]) == 0.0
  require prepress::compare_colors_delta_e([0, 0, 0, 1], [0, 0, 0, 0]) > 90.0
  require prepress::compare_colors_delta_e([0.5], [0.52]) < 3.0
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_prepress_errors() {
        let i = run(r#"check "E" { prepress::compare_colors_delta_e([1, 2], [1, 2]) }"#);
        assert!(i.diagnostics[0].message.contains("1 (gray), 3 (RGB) or 4 (CMYK)"), "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_prepress_geometry() {
        // page 1 has a trim of 8.5pt (~3mm) inside the bleed; page 2 has no boxes
        let i = run(r#"
check "Geometria" {
  require !prepress::check_page_geometry()
  require !prepress::check_page_geometry(3)
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_codes_detection() {
        let i = run(r#"
check "Codes" {
  require codes::detect_barcodes()
  require codes::detect_qrcodes()
  require codes::count_barcodes() == 2
  require codes::get_barcode_type(1) == "EAN_13"
  require codes::decode_barcode(2) == "https://exemplo.com"
  require codes::get_barcode_location(1).contains(150.0)
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_codes_validation() {
        let i = run(r#"
check "GTIN" {
  require codes::validate_barcode_checksum(1)
  require codes::validate_gtin("7891234567895")
  require !codes::validate_gtin("7891234567890")
  require codes::validate_barcode_format("^(\d{13}|https://.*)$")
  require codes::validate_barcode_position(0, 0, 595, 842)
  require !codes::validate_barcode_position(0, 0, 100, 100)
  require !codes::validate_code128()
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_fix_queues_validated() {
        use crate::fixns::FixOp;
        let prog = parse(
            r#"
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::rotate_page(1, 90)
fix::reorder_pages([2, 1])
fix::add_watermark("RASCUNHO")
fix::add_page_numbers()
"#,
        )
        .unwrap();
        let mut i = Interpreter::new();
        i.allow_fixes = true;
        i.run(&prog, mock_doc()).unwrap();
        assert_eq!(i.fix_ops.len(), 5);
        assert_eq!(i.fix_ops[1], FixOp::RotatePage { page: 1, degrees: 90 });
        assert_eq!(i.fix_ops[2], FixOp::ReorderPages { order: vec![2, 1] });
    }

    #[test]
    fn namespace_fix_queues_advanced() {
        use crate::fixns::FixOp;
        let prog = parse(
            r#"
fix::split_document(1, 2, "/tmp/parte.pdf")
fix::add_stamps("APROVADO")
fix::flatten_layers()
fix::remove_annotations()
fix::remove_attachments()
fix::remove_unused_resources()
"#,
        )
        .unwrap();
        let mut i = Interpreter::new();
        i.allow_fixes = true;
        i.run(&prog, mock_doc()).unwrap();
        assert_eq!(i.fix_ops.len(), 6);
        assert_eq!(
            i.fix_ops[0],
            FixOp::SplitDocument { from: 1, to: 2, output: "/tmp/parte.pdf".into() }
        );
        assert_eq!(i.fix_ops[1], FixOp::AddStamp { text: "APROVADO".into() });
    }

    #[test]
    fn namespace_fix_validates_arguments() {
        let casos = [
            ("fix::rotate_page(1, 45)", "90, 180 or 270"),
            ("fix::delete_page(9)", "page 9 does not exist"),
            ("fix::reorder_pages([1, 1])", "exactly once"),
            ("fix::add_watermark(\"\")", "the watermark text"),
            ("fix::split_document(2, 1, \"x.pdf\")", "invalid range"),
            ("fix::split_document(1, 2)", "output file"),
            ("fix::merge_documents(\"nao_existe.pdf\")", "not found"),
            ("fix::add_stamps(\"\")", "the stamp text"),
        ];
        for (src, esperado) in casos {
            let prog = parse(&format!("check \"c\" {{ {src} }}")).unwrap();
            let mut i = Interpreter::new();
            i.allow_fixes = true;
            i.run(&prog, mock_doc()).unwrap();
            assert!(i.diagnostics[0].message.contains(esperado), "{src}: {:?}", i.diagnostics);
            assert!(i.fix_ops.is_empty());
        }
    }

    #[test]
    fn fix_blocked_in_run_mode() {
        let i = run(r#"check "Fix" { fix::rotate_page(90) }"#);
        assert!(i.diagnostics[0].message.contains("pdfl fix"));
    }

    #[test]
    fn namespace_data_glossary_and_dataset() {
        let i = run(r#"
check "Data" {
  terms = data::load_glossary("tests/fixtures/glossary.txt")
  require terms.length == 3
  require terms.first() == "Hello"

  rows = data::load_dataset("tests/fixtures/data.csv")
  require rows.length == 4
  require rows.get(2).get(2) == "Dipirona 500mg"
  require rows.last().first() == "with, comma"

  require data::lookup_value("tests/fixtures/data.csv", "L2026-08") == "August batch"
  require !data::lookup_value("tests/fixtures/data.csv", "does-not-exist")
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_data_reference() {
        // mock: the doc's text is "Hello world" — "Total Warranty" is missing
        let i = run(r#"
check "Glossary" {
  missing = data::validate_against_reference("tests/fixtures/glossary.txt")
  require missing.length == 1
  require missing.first() == "Total Warranty"
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_data_lookups() {
        // the bases live in tests/fixtures/dados/ (PDFL_DATA_DIR points there)
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("PDFL_DATA_DIR", "tests/fixtures/dados");
        let i = run(r#"
check "Consultas" {
  produto = data::query_gtin("7891234567895")
  require produto.get(2) == "Dipirona Sodica 500mg 20cp"
  require !data::query_gtin("0000000000000")

  med = data::query_medicamento("1.0298.0456")
  require med.get(2) == "Amoxicilina"
  require data::query_medicamento("amoxicilina").get(4) == "vermelha"

  endereco = data::query_postal_code("01310-100")
  require endereco.get(2) == "Avenida Paulista"
  require data::validate_address("01310100", "Avenida Paulista")
  require !data::validate_address("01310100", "Rua Inexistente")
}
"#);
        std::env::remove_var("PDFL_DATA_DIR");
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_data_errors() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("PDFL_DATA_DIR", "tests/fixtures/dados");
        let i = run(r#"check "E" { data::query_postal_code("123") }"#);
        assert!(i.diagnostics[0].message.contains("8 digits"), "{:?}", i.diagnostics);
        std::env::remove_var("PDFL_DATA_DIR");

        // missing base: the message explains where to put the file
        std::env::set_var("PDFL_DATA_DIR", "/tmp/pdfl-sem-base");
        let i = run(r#"check "E" { data::query_gtin("7891234567895") }"#);
        assert!(i.diagnostics[0].message.contains("not found"), "{:?}", i.diagnostics);
        std::env::remove_var("PDFL_DATA_DIR");
    }

    #[test]
    fn namespace_fix_images() {
        use crate::fixns::FixOp;
        let prog = parse("fix::downsample_images(150)\nfix::compress_images(70)").unwrap();
        let mut i = Interpreter::new();
        i.allow_fixes = true;
        i.run(&prog, mock_doc()).unwrap();
        assert_eq!(i.fix_ops[0], FixOp::DownsampleImages { dpi: 150.0 });
        assert_eq!(i.fix_ops[1], FixOp::CompressImages { quality: 70 });

        // argument validation
        let prog = parse("fix::compress_images(150)").unwrap();
        let mut i = Interpreter::new();
        i.allow_fixes = true;
        i.run(&prog, mock_doc()).unwrap_err();
    }

    #[test]
    fn list_get_first_last() {
        let i = run(r#"
check "Listas" {
  l = [10, 20, 30]
  require l.get(1) == 10
  require l.first() == 10
  require l.last() == 30
}
check "Erro" { [1].get(5) }
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("item 5 does not exist"));
    }

    #[test]
    fn units_become_points() {
        let i = run(r#"
check "Unidades" {
  require 1in == 72.0
  require abs(10mm - 28.35) < 0.01
  require 1cm == 10mm
  require 2pt == 2.0
  require 300% == 300
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn unknown_unit_fails_in_lexer() {
        assert!(parse("require 3kg > 1").is_err());
    }

    #[test]
    fn user_function() {
        let i = run(r#"
function dobro(x) {
  x * 2
}
function eh_a4(page) {
  abs(page.width - 595.0) < 5.0
}
check "Functions" {
  require dobro(21) == 42
  require doc.pages.all { |p| eh_a4(p) }
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn function_recursion_is_limited() {
        let i = run(r#"
function infinite_loop(x) { infinite_loop(x) }
check "Recursion" { infinite_loop(1) }
"#);
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("recursion"), "{:?}", i.diagnostics);
    }

    #[test]
    fn import_loads_functions_and_consts() {
        let i = run(r#"
import "tests/fixtures/lib.pdfl"
check "Import" {
  require VERSAO_LIB == "1.0"
  require doc.pages.all { |p| pagina_a4(p) }
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn missing_import_is_friendly() {
        let prog = parse("import \"nao_existe.pdfl\"").unwrap();
        let mut interp = Interpreter::new();
        let err = interp.run(&prog, mock_doc()).unwrap_err();
        assert!(err.message.contains("nao_existe.pdfl"), "{err}");
    }

    #[test]
    fn namespace_text_validations() {
        let i = run(r#"
check "Brazilian validations" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")
  require text::validate_cnpj("11.222.333/0001-81")
  require !text::validate_cnpj("11.222.333/0001-82")
  require text::validate_date_format("29/02/2024")
  require !text::validate_date_format("31/04/2026")
  require text::validate_date_format("02/08/2026", "dd/mm/aaaa")
  require text::validate_phone_format("(11) 98765-4321")
  require !text::validate_phone_format("12345")
  require text::validate_format("L2026-08", "L\d{4}-\d{2}")
  require !text::validate_format("X99", "L\d{4}-\d{2}")
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn namespace_text_diff_and_pii() {
        let i = run(r#"
check "Diff and PII" {
  changes = text::diff("line one\nline two", "line one\nline TWO")
  require changes.length == 2
  require changes.first() == "-line two"
  require changes.last() == "+line TWO"

  // An invalid CPF (wrong check digit) is NO LONGER reported as personal data
  require text::detect_personal_data("CPF falso: 529.982.247-26").length == 0
  require text::detect_personal_data("CPF real: 529.982.247-25").length == 1
  require !text::detect_rasterized_text()
  require text::extract_with_normalization() == "hello world"
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn region_type() {
        let i = run(r#"
check "Regions" {
  header = region(0, 742, 595, 100, "header")
  require header.name == "header"
  require header.width == 595.0
  require header.top == 842.0
  require header.right == 595.0
  require header.area == 59500.0
  require header.contains_point(300, 800)
  require !header.contains_point(300, 100)
  require header.export_coordinates().length == 4

  footer = region(0, 0, 595, 60)
  require !header.intersects(footer)
  require header.expand(10).height == 120.0
  require header.inset(10).height == 80.0
  require region(0, 0, 100, 100).intersects(region(50, 50, 100, 100))
}
"#);
        assert_eq!(i.diagnostics.len(), 0, "{:?}", i.diagnostics);
    }

    #[test]
    fn region_validates_arguments() {
        let i = run(r#"check "R" { region(0, 0, -5, 10) }"#);
        assert!(i.diagnostics[0].message.contains("positive"), "{:?}", i.diagnostics);
        let i = run(r#"check "R" { region(0, 0, 10, 10).inset(20) }"#);
        assert!(i.diagnostics[0].message.contains("no area"), "{:?}", i.diagnostics);
    }

    #[test]
    fn rule_block_all_pages() {
        // the mock has 2 pages of 595x842; the rule runs on each one
        let i = run(r#"
rule "Formato A4" {
  assert page.width == 595.0, "page #{page.number} is not A4"
  assert page.height == 100.0, "wrong height on page #{page.number}"
}
"#);
        // the second assertion fails on both pages
        assert_eq!(i.diagnostics.len(), 2);
        assert_eq!(i.diagnostics[0].check_name, "Formato A4");
        assert!(i.diagnostics[0].message.contains("page 1"));
        assert!(i.diagnostics[1].message.contains("page 2"));
    }

    #[test]
    fn rule_block_with_selection() {
        // a filter with a block works directly: the first `{` belongs to filter,
        // the second is the rule's body
        let i = run(r#"
rule "Only with text" on doc.pages.filter { |p| p.extract_text() != "" } {
  assert page.number == 99, "ran on page #{page.number}"
}
"#);
        // only page 1 of the mock has text
        assert_eq!(i.diagnostics.len(), 1);
        assert!(i.diagnostics[0].message.contains("ran on page 1"));

        // parentheses are accepted too (the explicit form)
        let i = run(r#"
rule "Idem" on (doc.pages.filter { |p| p.extract_text() != "" }) {
  assert false, "page #{page.number}"
}
"#);
        assert_eq!(i.diagnostics.len(), 1);
    }

    #[test]
    fn rule_block_friendly_error() {
        // a selection that is not a list of pages
        let i = run(r#"rule "Ruim" on 42 { require page.width > 0 }"#);
        assert!(i.diagnostics[0].message.contains("expects a list of pages"), "{:?}", i.diagnostics);

        // a selection ending in member access swallows the body: the error guides
        let err = parse("rule \"x\" on doc.title {\n  require page.width > 0\n}").unwrap_err();
        assert!(err.message.contains("wrap it in parentheses"), "{err}");
    }

    #[test]
    fn short_circuit() {
        let i = run("check \"cc\" { require doc.page_count > 0 || variavel_inexistente }");
        assert!(i.diagnostics.is_empty());
    }
}
