//! Comando `pdfl lint` — análise estática de scripts .pdfl (seção 8.6).
//! Fora desta fatia: imports circulares (não há imports ainda) e checagem
//! de tipos (não há sistema de tipos estático).

use crate::ast::{Block, Expr, Stmt, StrPart};
use std::collections::{HashMap, HashSet};

const NAMESPACES: &[&str] = &["text", "struct", "visual", "prepress", "codes", "fix", "data"];

pub fn lint(program: &[Stmt]) -> Vec<String> {
    let mut w = Walker::default();
    w.stmts(program, false);

    let mut out = Vec::new();
    // Declarada e nunca lida (prefixo _ silencia).
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
    /// nome -> quantas vezes declarada
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
                // `page` é fornecido pelo runtime dentro da regra
                self.declared.entry("page".into()).or_insert(0);
                self.used.insert("page".into());
                self.stmts(body, true);
            }
            Stmt::Function { name, params, body } => {
                *self.declared.entry(format!("function '{name}'")).or_insert(0) += 1;
                // corpo tem escopo próprio para os parâmetros (como blocos)
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
                    // chamada de função do usuário conta como uso
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
        // Parâmetros do bloco têm escopo próprio: checa uso dentro do corpo.
        let mut inner = Walker::default();
        inner.stmts(&block.body, true);
        for param in &block.params {
            if !param.starts_with('_') && !inner.used.contains(param) {
                self.findings.push(format!("block parameter '{param}' never used"));
            }
        }
        // Propaga o que o corpo declarou/usou/achou para o escopo externo.
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
    fn variavel_nao_usada() {
        let w = lint_src("const LIMITE = 300\ncheck \"a\" { require doc.page_count > 0 }");
        assert!(w.iter().any(|m| m.contains("'LIMITE' declared and never used")), "{w:?}");
    }

    #[test]
    fn variavel_usada_nao_avisa() {
        let w = lint_src("const L = 300\ncheck \"a\" { require doc.page_count < L }");
        assert!(w.is_empty(), "{w:?}");
    }

    #[test]
    fn check_duplicado_e_vazio() {
        let w = lint_src("check \"x\" { }\ncheck \"x\" { require doc.page_count > 0 }");
        assert!(w.iter().any(|m| m.contains("\"x\" declared 2 times")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("\"x\" is empty")), "{w:?}");
    }

    #[test]
    fn namespace_desconhecido_e_fix() {
        let w = lint_src("check \"a\" {\n dados::consulta()\n fix::rotate_page(90)\n}");
        assert!(w.iter().any(|m| m.contains("unknown namespace: dados::")), "{w:?}");
        assert!(w.iter().any(|m| m.contains("pdfl fix")), "{w:?}");
    }

    #[test]
    fn assert_fora_de_check() {
        let w = lint_src("require doc.page_count > 0");
        assert!(w.iter().any(|m| m.contains("outside any check")), "{w:?}");
    }

    #[test]
    fn parametro_de_bloco_nao_usado() {
        let w = lint_src("check \"a\" { doc.pages.each { |page| require doc.title != \"\" } }");
        assert!(w.iter().any(|m| m.contains("'page' never used")), "{w:?}");
        // com underscore não avisa
        let w2 = lint_src("check \"a\" { doc.pages.each { |_page| require doc.title != \"\" } }");
        assert!(w2.is_empty(), "{w2:?}");
    }
}
