//! Recursive-descent parser for the PDFLang language.

use crate::ast::*;
use crate::lexer::{tokenize, StrSeg, Tok, Token};
use std::fmt;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "syntax error at line {}, column {}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(source: &str) -> Result<Vec<Stmt>, ParseError> {
    let toks = tokenize(source).map_err(|e| ParseError { message: e.message, line: e.line, col: e.col })?;
    let mut p = Parser { toks, pos: 0 };
    p.parse_program()
}

/// Parses a standalone expression (used by `#{...}` interpolations).
fn parse_expr_source(source: &str, line: usize, col: usize) -> Result<Expr, ParseError> {
    let toks = tokenize(source).map_err(|e| ParseError {
        message: format!("in interpolation: {}", e.message),
        line,
        col,
    })?;
    let mut p = Parser { toks, pos: 0 };
    let expr = p.expr()?;
    p.skip_newlines();
    if !matches!(p.peek(), Tok::Eof) {
        return Err(p.error(format!("unexpected token in interpolation: {}", p.peek())));
    }
    Ok(expr)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].tok
    }

    fn advance(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn error(&self, message: String) -> ParseError {
        let t = &self.toks[self.pos];
        ParseError { message, line: t.line, col: t.col }
    }

    fn expect(&mut self, tok: Tok, what: &str) -> Result<(), ParseError> {
        if *self.peek() == tok {
            self.advance();
            Ok(())
        } else {
            Err(self.error(format!("expected {what}, found {}", self.peek())))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Tok::Newline) {
            self.advance();
        }
    }

    fn parse_program(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let body = self.stmts_until(Tok::Eof)?;
        Ok(body)
    }

    /// A sequence of statements up to `end` (the caller consumes `end`).
    fn stmts_until(&mut self, end: Tok) -> Result<Vec<Stmt>, ParseError> {
        let mut out = Vec::new();
        loop {
            self.skip_newlines();
            if *self.peek() == end || matches!(self.peek(), Tok::Eof) {
                return Ok(out);
            }
            out.push(self.stmt()?);
            match self.peek() {
                Tok::Newline => {
                    self.advance();
                }
                t if *t == end => {}
                Tok::Eof => {}
                other => return Err(self.error(format!("expected end of line after statement, found {other}"))),
            }
        }
    }

    fn braced_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        self.skip_newlines();
        self.expect(Tok::LBrace, "'{'")?;
        let body = self.stmts_until(Tok::RBrace)?;
        self.expect(Tok::RBrace, "'}'")?;
        Ok(body)
    }

    fn string_literal(&mut self, what: &str) -> Result<String, ParseError> {
        match self.peek().clone() {
            Tok::Str(segs) => {
                if segs.iter().any(|s| matches!(s, StrSeg::Interp(_))) {
                    return Err(self.error(format!("{what} cannot contain interpolation")));
                }
                self.advance();
                Ok(segs
                    .iter()
                    .map(|s| match s {
                        StrSeg::Lit(l) => l.as_str(),
                        StrSeg::Interp(_) => unreachable!(),
                    })
                    .collect())
            }
            other => Err(self.error(format!("expected {what} in quotes, found {other}"))),
        }
    }

    fn stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek().clone() {
            Tok::Profile => {
                self.advance();
                let name = self.string_literal("profile name")?;
                let body = self.braced_body()?;
                Ok(Stmt::Profile { name, body })
            }
            Tok::Check => {
                self.advance();
                let name = self.string_literal("check name")?;
                let mut tags = Vec::new();
                if matches!(self.peek(), Tok::Ident(w) if w == "tags") {
                    self.advance();
                    self.expect(Tok::Colon, "':' after tags")?;
                    self.expect(Tok::LBracket, "'[' with the tag list")?;
                    loop {
                        self.skip_newlines();
                        if matches!(self.peek(), Tok::RBracket) {
                            break;
                        }
                        tags.push(self.string_literal("tag")?);
                        self.skip_newlines();
                        if matches!(self.peek(), Tok::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    self.expect(Tok::RBracket, "']'")?;
                }
                let body = self.braced_body()?;
                Ok(Stmt::Check { name, tags, body })
            }
            Tok::Function => {
                self.advance();
                let name = match self.advance() {
                    Tok::Ident(n) => n,
                    other => return Err(self.error(format!("expected function name, found {other}"))),
                };
                self.expect(Tok::LParen, "'(' with the parameters")?;
                let mut params = Vec::new();
                loop {
                    self.skip_newlines();
                    match self.peek().clone() {
                        Tok::RParen => break,
                        Tok::Ident(p) => {
                            self.advance();
                            params.push(p);
                            if matches!(self.peek(), Tok::Comma) {
                                self.advance();
                            }
                        }
                        other => return Err(self.error(format!("expected parameter, found {other}"))),
                    }
                }
                self.expect(Tok::RParen, "')'")?;
                let body = self.braced_body()?;
                Ok(Stmt::Function { name, params, body })
            }
            Tok::Import => {
                self.advance();
                let path = self.string_literal("import path")?;
                Ok(Stmt::Import { path })
            }
            Tok::Rule => {
                // rule "name" [on <page list>] { body }
                self.advance();
                let name = self.string_literal("rule name")?;
                let pages = if matches!(self.peek(), Tok::On) {
                    self.advance();
                    Some(self.expr()?)
                } else {
                    None
                };
                // If the selection ends in member access, the body's `{` is
                // absorbed as a method block (as in Ruby) — explain how to fix
                // it instead of giving a generic error.
                self.skip_newlines();
                if pages.is_some() && !matches!(self.peek(), Tok::LBrace) {
                    return Err(self.error(
                        "expected '{' with the rule body — if the 'on' selection uses a block, \
                         wrap it in parentheses: rule \"x\" on (doc.pages.filter { ... }) { ... }"
                            .to_string(),
                    ));
                }
                let body = self.braced_body()?;
                Ok(Stmt::Rule { name, pages, body })
            }
            Tok::Const => {
                self.advance();
                let name = match self.advance() {
                    Tok::Ident(n) => n,
                    other => return Err(self.error(format!("expected constant name, found {other}"))),
                };
                self.expect(Tok::Eq, "'='")?;
                let value = self.expr()?;
                Ok(Stmt::Const { name, value })
            }
            Tok::Assert | Tok::Require => {
                let is_require = matches!(self.peek(), Tok::Require);
                let line = self.toks[self.pos].line;
                self.advance();
                let cond = self.expr()?;
                let source = cond.to_string();
                let message = if !is_require && matches!(self.peek(), Tok::Comma) {
                    self.advance();
                    self.skip_newlines();
                    Some(self.expr()?)
                } else {
                    None
                };
                Ok(Stmt::Assert { cond, message, source, line })
            }
            Tok::Ident(name) if matches!(self.peek2(), Tok::Eq) => {
                self.advance();
                self.advance();
                let value = self.expr()?;
                Ok(Stmt::Assign { name, value })
            }
            _ => Ok(Stmt::Expr(self.expr()?)),
        }
    }

    // ---- expressions (increasing precedence) ----

    fn expr(&mut self) -> Result<Expr, ParseError> {
        self.or_expr()
    }

    fn binary_level(
        &mut self,
        next: fn(&mut Self) -> Result<Expr, ParseError>,
        ops: &[(Tok, BinOp)],
    ) -> Result<Expr, ParseError> {
        let mut left = next(self)?;
        loop {
            let Some((_, op)) = ops.iter().find(|(t, _)| t == self.peek()) else {
                return Ok(left);
            };
            let op = *op;
            self.advance();
            self.skip_newlines();
            let right = next(self)?;
            left = Expr::Binary { op, left: Box::new(left), right: Box::new(right) };
        }
    }

    fn or_expr(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(Self::and_expr, &[(Tok::OrOr, BinOp::Or)])
    }

    fn and_expr(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(Self::equality, &[(Tok::AndAnd, BinOp::And)])
    }

    fn equality(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(Self::comparison, &[(Tok::EqEq, BinOp::Eq), (Tok::NotEq, BinOp::NotEq)])
    }

    fn comparison(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(
            Self::term,
            &[
                (Tok::LtEq, BinOp::LtEq),
                (Tok::GtEq, BinOp::GtEq),
                (Tok::Lt, BinOp::Lt),
                (Tok::Gt, BinOp::Gt),
            ],
        )
    }

    fn term(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(Self::factor, &[(Tok::Plus, BinOp::Add), (Tok::Minus, BinOp::Sub)])
    }

    fn factor(&mut self) -> Result<Expr, ParseError> {
        self.binary_level(Self::unary, &[(Tok::Star, BinOp::Mul), (Tok::Slash, BinOp::Div)])
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Tok::Bang => {
                self.advance();
                Ok(Expr::Unary { op: UnOp::Not, expr: Box::new(self.unary()?) })
            }
            Tok::Minus => {
                self.advance();
                Ok(Expr::Unary { op: UnOp::Neg, expr: Box::new(self.unary()?) })
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.primary()?;
        loop {
            match self.peek() {
                Tok::Dot => {
                    self.advance();
                    let name = match self.advance() {
                        Tok::Ident(n) => n,
                        other => return Err(self.error(format!("expected name after '.', found {other}"))),
                    };
                    if matches!(self.peek(), Tok::LParen) {
                        let args = self.call_args()?;
                        let block = self.maybe_block()?;
                        expr = Expr::Call { recv: Some(Box::new(expr)), name, args, block };
                    } else if matches!(self.peek(), Tok::LBrace) {
                        let block = self.maybe_block()?;
                        expr = Expr::Call { recv: Some(Box::new(expr)), name, args: Vec::new(), block };
                    } else {
                        expr = Expr::Member { recv: Box::new(expr), name };
                    }
                }
                _ => return Ok(expr),
            }
        }
    }

    fn call_args(&mut self) -> Result<Vec<Arg>, ParseError> {
        self.expect(Tok::LParen, "'('")?;
        let mut args = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Tok::RParen) {
                break;
            }
            let name = if matches!(self.peek(), Tok::Ident(_)) && matches!(self.peek2(), Tok::Colon) {
                let Tok::Ident(n) = self.advance() else { unreachable!() };
                self.advance(); // ':'
                self.skip_newlines();
                Some(n)
            } else {
                None
            };
            let value = self.expr()?;
            args.push(Arg { name, value });
            self.skip_newlines();
            if matches!(self.peek(), Tok::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        self.skip_newlines();
        self.expect(Tok::RParen, "')'")?;
        Ok(args)
    }

    /// A `{ |a, b| body }` block — the `|params|` part is optional.
    fn maybe_block(&mut self) -> Result<Option<Block>, ParseError> {
        if !matches!(self.peek(), Tok::LBrace) {
            return Ok(None);
        }
        self.advance();
        self.skip_newlines();
        let mut params = Vec::new();
        if matches!(self.peek(), Tok::Pipe) {
            self.advance();
            loop {
                match self.advance() {
                    Tok::Ident(n) => params.push(n),
                    other => return Err(self.error(format!("expected block parameter, found {other}"))),
                }
                match self.advance() {
                    Tok::Comma => continue,
                    Tok::Pipe => break,
                    other => return Err(self.error(format!("expected ',' or '|', found {other}"))),
                }
            }
        }
        let body = self.stmts_until(Tok::RBrace)?;
        self.expect(Tok::RBrace, "'}'")?;
        Ok(Some(Block { params, body }))
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        let (line, col) = (self.toks[self.pos].line, self.toks[self.pos].col);
        match self.advance() {
            Tok::Int(n) => Ok(Expr::Int(n)),
            Tok::Float(n) => Ok(Expr::Float(n)),
            Tok::UnitNum(n, _) => Ok(Expr::Float(n)),
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
            Tok::Str(segs) => {
                let mut parts = Vec::new();
                for seg in segs {
                    match seg {
                        StrSeg::Lit(l) => parts.push(StrPart::Lit(l)),
                        StrSeg::Interp(src) => parts.push(StrPart::Interp(parse_expr_source(&src, line, col)?)),
                    }
                }
                Ok(Expr::Str(parts))
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                loop {
                    self.skip_newlines();
                    if matches!(self.peek(), Tok::RBracket) {
                        break;
                    }
                    items.push(self.expr()?);
                    self.skip_newlines();
                    if matches!(self.peek(), Tok::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
                self.expect(Tok::RBracket, "']'")?;
                Ok(Expr::List(items))
            }
            Tok::LParen => {
                self.skip_newlines();
                let e = self.expr()?;
                self.skip_newlines();
                self.expect(Tok::RParen, "')'")?;
                Ok(e)
            }
            Tok::Ident(name) => {
                if matches!(self.peek(), Tok::ColonColon) {
                    // Namespace call: text::count_words(...)
                    self.advance();
                    let func = match self.advance() {
                        Tok::Ident(n) => n,
                        other => {
                            return Err(self.error(format!("expected function name after '::', found {other}")))
                        }
                    };
                    let full = format!("{name}::{func}");
                    let (args, block) = if matches!(self.peek(), Tok::LParen) {
                        let args = self.call_args()?;
                        (args, self.maybe_block()?)
                    } else {
                        (Vec::new(), None)
                    };
                    Ok(Expr::Call { recv: None, name: full, args, block })
                } else if matches!(self.peek(), Tok::LParen) {
                    let args = self.call_args()?;
                    let block = self.maybe_block()?;
                    Ok(Expr::Call { recv: None, name, args, block })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => {
                self.pos = self.pos.saturating_sub(1);
                Err(ParseError { message: format!("unexpected expression: {other}"), line, col })
            }
        }
    }
}

// ---- source reconstruction (automatic `require` messages) ----

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Int(n) => write!(f, "{n}"),
            Expr::Float(n) => write!(f, "{n}"),
            Expr::Bool(b) => write!(f, "{b}"),
            Expr::Str(parts) => {
                write!(f, "\"")?;
                for p in parts {
                    match p {
                        StrPart::Lit(l) => write!(f, "{l}")?,
                        StrPart::Interp(e) => write!(f, "#{{{e}}}")?,
                    }
                }
                write!(f, "\"")
            }
            Expr::List(items) => {
                write!(f, "[")?;
                for (i, e) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{e}")?;
                }
                write!(f, "]")
            }
            Expr::Ident(n) => write!(f, "{n}"),
            Expr::Member { recv, name } => write!(f, "{recv}.{name}"),
            Expr::Call { recv, name, args, block } => {
                if let Some(r) = recv {
                    write!(f, "{r}.")?;
                }
                write!(f, "{name}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    if let Some(n) = &a.name {
                        write!(f, "{n}: ")?;
                    }
                    write!(f, "{}", a.value)?;
                }
                write!(f, ")")?;
                if block.is_some() {
                    write!(f, " {{ ... }}")?;
                }
                Ok(())
            }
            Expr::Unary { op, expr } => match op {
                UnOp::Not => write!(f, "!{expr}"),
                UnOp::Neg => write!(f, "-{expr}"),
            },
            Expr::Binary { op, left, right } => {
                let sym = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Eq => "==",
                    BinOp::NotEq => "!=",
                    BinOp::Lt => "<",
                    BinOp::LtEq => "<=",
                    BinOp::Gt => ">",
                    BinOp::GtEq => ">=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };
                write!(f, "{left} {sym} {right}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_and_assign() {
        let prog = parse("const X = 1 + 2\ny = X * 3").unwrap();
        assert_eq!(prog.len(), 2);
        assert!(matches!(&prog[0], Stmt::Const { name, .. } if name == "X"));
        assert!(matches!(&prog[1], Stmt::Assign { name, .. } if name == "y"));
    }

    #[test]
    fn check_with_tags_and_require() {
        let src = r#"
check "Fontes" tags: ["fonts", "basico"] {
  require doc.page_count > 0
}
"#;
        let prog = parse(src).unwrap();
        let Stmt::Check { name, tags, body } = &prog[0] else { panic!("esperava check") };
        assert_eq!(name, "Fontes");
        assert_eq!(tags, &["fonts".to_string(), "basico".to_string()]);
        let Stmt::Assert { message, source, .. } = &body[0] else { panic!("esperava assert") };
        assert!(message.is_none());
        assert_eq!(source, "doc.page_count > 0");
    }

    #[test]
    fn assert_with_interpolated_message() {
        let src = r#"assert font.is_embedded, "Font #{font.name} is not embedded""#;
        let prog = parse(src).unwrap();
        let Stmt::Assert { message: Some(Expr::Str(parts)), .. } = &prog[0] else {
            panic!("esperava assert com mensagem string")
        };
        assert_eq!(parts.len(), 3);
        assert!(matches!(&parts[1], StrPart::Interp(Expr::Member { .. })));
    }

    #[test]
    fn block_with_parameters() {
        let src = "doc.pages.each { |page|\n  require page.width > 0\n}";
        let prog = parse(src).unwrap();
        let Stmt::Expr(Expr::Call { name, block: Some(b), .. }) = &prog[0] else {
            panic!("esperava chamada com bloco")
        };
        assert_eq!(name, "each");
        assert_eq!(b.params, vec!["page".to_string()]);
        assert_eq!(b.body.len(), 1);
    }

    #[test]
    fn named_args() {
        let prog = parse("visual.detect_low_resolution(dpi: 300)").unwrap();
        let Stmt::Expr(Expr::Call { args, .. }) = &prog[0] else { panic!() };
        assert_eq!(args[0].name.as_deref(), Some("dpi"));
    }

    #[test]
    fn full_profile() {
        let src = r#"
profile "offset" {
  const MIN = 1
  check "Estrutura" {
    require doc.page_count >= MIN
    assert doc.title != "", "PDF has no title"
  }
}
"#;
        let prog = parse(src).unwrap();
        let Stmt::Profile { name, body } = &prog[0] else { panic!() };
        assert_eq!(name, "offset");
        assert_eq!(body.len(), 2);
    }

    #[test]
    fn assert_message_on_new_line() {
        let src = "assert x >= 1,\n  \"mensagem\"";
        assert!(parse(src).is_ok());
    }

    #[test]
    fn syntax_error_has_line() {
        let err = parse("check {\n}").unwrap_err();
        assert_eq!(err.line, 1);
    }

    #[test]
    fn namespace_call() {
        let prog = parse("require text::count_words() > 10").unwrap();
        let Stmt::Assert { source, .. } = &prog[0] else { panic!() };
        assert_eq!(source, "text::count_words() > 10");

        // it works without parentheses too
        let prog = parse("text::extract_all").unwrap();
        let Stmt::Expr(Expr::Call { name, args, .. }) = &prog[0] else { panic!() };
        assert_eq!(name, "text::extract_all");
        assert!(args.is_empty());
    }

    #[test]
    fn precedence() {
        let prog = parse("assert 1 + 2 * 3 == 7").unwrap();
        let Stmt::Assert { source, .. } = &prog[0] else { panic!() };
        assert_eq!(source, "1 + 2 * 3 == 7");
    }
}
