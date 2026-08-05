//! `pdfl lint` command — static analysis of .pdfl scripts.
//! Out of scope here: circular imports and type checking (there is no static
//! type system).

use crate::ast::{Block, Expr, Stmt, StrPart};
use std::collections::{HashMap, HashSet};

const NAMESPACES: &[&str] = &["text", "struct", "visual", "prepress", "codes", "fix", "data"];

pub fn lint(program: &[Stmt]) -> Vec<String> {
    let mut w = Walker::default();
    w.stmts(program, false);

    let mut out = Vec::new();
    // Declared and never read (an _ prefix silences it).
    for (name, _) in &w.declared {
        if w.used.contains(name) {
            continue;
        }
        if let Some(f) = name.strip_prefix("function ") {
            if !f.starts_with("'_") {
                out.push(format!("function {f} declared and never used"));
            }
        } else if !name.starts_with('_') {
            out.push(format!("variable '{name}' declared and never used"));
        }
    }
    for (name, count) in &w.check_names {
        if *count > 1 {
            out.push(format!("check \"{name}\" declared {count} times"));
        }
    }
    out.extend(w.findings);
    out.sort();
    out
}

#[derive(Default)]
struct Walker {
    /// name -> how many times it was declared
    declared: HashMap<String, usize>,
    used: HashSet<String>,
    check_names: HashMap<String, usize>,
    findings: Vec<String>,
    has_fix: bool,
}

impl Walker {
    fn stmts(&mut self, stmts: &[Stmt], in_check: bool) {
        for stmt in stmts {
            self.stmt(stmt, in_check);
        }
    }

    fn stmt(&mut self, stmt: &Stmt, in_check: bool) {
        match stmt {
            Stmt::Profile { body, .. } => self.stmts(body, in_check),
            Stmt::Import { .. } => {}
            Stmt::Rule { name, pages, body } => {
                *self.check_names.entry(name.clone()).or_insert(0) += 1;
                if body.is_empty() {
                    self.findings.push(format!("rule \"{name}\" is empty"));
                }
                if let Some(e) = pages {
                    self.expr(e);
                }
                // `page` is provided by the runtime inside the rule
                self.declared.entry("page".into()).or_insert(0);
                self.used.insert("page".into());
                self.stmts(body, true);
            }
            Stmt::Function { name, params, body } => {
                *self.declared.entry(format!("function '{name}'")).or_insert(0) += 1;
                // the body has its own scope for the parameters (like blocks)
                let mut inner = Walker::default();
                inner.stmts(body, in_check);
                for param in params {
                    if !param.starts_with('_') && !inner.used.contains(param) {
                        self.findings.push(format!("parameter '{param}' of function '{name}' never used"));
                    }
                    inner.used.remove(param);
                }
                for (n, c) in inner.declared {
                    *self.declared.entry(n).or_insert(0) += c;
                }
                self.used.extend(inner.used);
                self.findings.extend(inner.findings);
                for (n, c) in inner.check_names {
                    *self.check_names.entry(n).or_insert(0) += c;
                }
            }
            Stmt::Check { name, body, .. } => {
                *self.check_names.entry(name.clone()).or_insert(0) += 1;
                if body.is_empty() {
                    self.findings.push(format!("check \"{name}\" is empty"));
                }
                self.stmts(body, true);
            }
            Stmt::Const { name, value } | Stmt::Assign { name, value } => {
                *self.declared.entry(name.clone()).or_insert(0) += 1;
                self.expr(value);
            }
            Stmt::Assert { cond, message, .. } => {
                if !in_check {
                    self.findings.push(format!(
                        "assert/require outside any check (line {}): the diagnostic has no check name",
                        match stmt {
                            Stmt::Assert { line, .. } => *line,
                            _ => 0,
                        }
                    ));
                }
                self.expr(cond);
                if let Some(m) = message {
                    self.expr(m);
                }
            }
            Stmt::Expr(e) => self.expr(e),
        }
    }

    fn expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Ident(name) => {
                self.used.insert(name.clone());
            }
            Expr::Member { recv, .. } => self.expr(recv),
            Expr::Call { recv, name, args, block } => {
                if recv.is_none() && !name.contains("::") {
                    // calling a user function counts as a use
                    self.used.insert(format!("function '{name}'"));
                }
                if let Some((ns, _)) = name.split_once("::") {
                    if !NAMESPACES.contains(&ns) {
                        self.findings.push(format!("unknown namespace: {ns}::"));
                    } else if ns == "fix" && !self.has_fix {
                        self.has_fix = true;
                        self.findings
                            .push("script uses fix:: — only works in the 'pdfl fix' command".into());
                    }
                }
                if let Some(r) = recv {
                    self.expr(r);
                }
                for a in args {
                    self.expr(&a.value);
                }
                if let Some(b) = block {
                    self.block(b);
                }
            }
            Expr::Unary { expr, .. } => self.expr(expr),
            Expr::Binary { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            Expr::List(items) => {
                for e in items {
                    self.expr(e);
                }
            }
            Expr::Str(parts) => {
                for p in parts {
                    if let StrPart::Interp(e) = p {
                        self.expr(e);
                    }
                }
            }
            Expr::Int(_) | Expr::Float(_) | Expr::Bool(_) => {}
        }
    }

    fn block(&mut self, block: &Block) {
        // Block parameters have their own scope: check their use inside the body.
        let mut inner = Walker::default();
        inner.stmts(&block.body, true);
        for param in &block.params {
            if !param.starts_with('_') && !inner.used.contains(param) {
                self.findings.push(format!("block parameter '{param}' never used"));
            }
        }
        // Propagate what the body declared/used/found to the outer scope.
        for (name, n) in inner.declared {
            *self.declared.entry(name).or_insert(0) += n;
        }
        for param in &block.params {
            inner.used.remove(param);
        }
        self.used.extend(inner.used);
        for (name, n) in inner.check_names {
            *self.check_names.entry(name).or_insert(0) += n;
        }
        self.findings.extend(inner.findings);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    fn lint_src(src: &str) -> Vec<String> {
        lint(&parse(src).unwrap())
    }

    #[test]
    fn unused_variable() {
        let w = lint_src("const LIMITE = 300\ncheck \"a\" { require doc.page_count > 0 }");
        assert!(w.iter().any(|m| m.contains("'LIMITE' declared and never used")), "{w:?}");
    }

    #[test]
    fn used_variable_does_not_warn() {
        let w = lint_src("const L = 300\ncheck \"a\" { require doc.page_count < L }");
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn duplicate_and_empty_check() {
        let w = lint_src("check \"x\" { }\ncheck \"x\" { require doc.page_count > 0 }");
        assert!(w.iter().any(|m| m.contains("\"x\" declared 2 times")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("\"x\" is empty")), "{w:?}");
    }

    #[test]
    fn unknown_namespace_and_fix() {
        let w = lint_src("check \"a\" {\n dados::consulta()\n fix::rotate_page(90)\n}");
        assert!(w.iter().any(|m| m.contains("unknown namespace: dados::")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("pdfl fix")), "{w:?}");
    }

    #[test]
    fn assert_outside_check() {
        let w = lint_src("require doc.page_count > 0");
        assert!(w.iter().any(|m| m.contains("outside any check")), "{w:?}");
    }

    #[test]
    fn unused_block_parameter() {
        let w = lint_src("check \"a\" { doc.pages.each { |page| require doc.title != \"\" } }");
        assert!(w.iter().any(|m| m.contains("'page' never used")), "{w:?}");
        // with an underscore it does not warn
        let w2 = lint_src("check \"a\" { doc.pages.each { |_page| require doc.title != \"\" } }");
        assert!(w2.is_empty(), "{w2:?}");
    }
}
