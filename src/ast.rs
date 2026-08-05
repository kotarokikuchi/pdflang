//! PDFLang's AST.

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Profile { name: String, body: Vec<Stmt> },
    Check { name: String, tags: Vec<String>, body: Vec<Stmt> },
    /// `function name(a, b) { ... }` — the value is that of the last expression.
    Function { name: String, params: Vec<String>, body: Vec<Stmt> },
    /// `import "other.pdfl"` — path relative to the importing script.
    Import { path: String },
    /// `rule "name" on <page expr> { body }` — applies the body to each
    /// selected page, with `page` available inside the block.
    Rule { name: String, pages: Option<Expr>, body: Vec<Stmt> },
    Const { name: String, value: Expr },
    Assign { name: String, value: Expr },
    /// `assert expr [, "message"]` — `require` becomes an Assert with message None
    /// and `source` holding the expression text for the automatic message.
    Assert { cond: Expr, message: Option<Expr>, source: String, line: usize },
    Expr(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(Vec<StrPart>),
    Bool(bool),
    List(Vec<Expr>),
    Ident(String),
    /// `recv.name` (without parentheses)
    Member { recv: Box<Expr>, name: String },
    /// `name(args)`, `recv.name(args)`, `recv.name { |x| ... }`
    Call { recv: Option<Box<Expr>>, name: String, args: Vec<Arg>, block: Option<Block> },
    Unary { op: UnOp, expr: Box<Expr> },
    Binary { op: BinOp, left: Box<Expr>, right: Box<Expr> },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrPart {
    Lit(String),
    Interp(Expr),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnOp {
    Not,
    Neg,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,
    And,
    Or,
}
