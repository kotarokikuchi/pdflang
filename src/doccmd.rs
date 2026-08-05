//! Comando `pdfl doc` — documentação de um script .pdfl.
//! Gera Markdown (padrão) ou HTML a partir da AST: perfil, constantes,
//! checks com tags e o que cada um valida.
//! ponytail: sem docstrings ainda — a descrição vem das mensagens de assert
//! e das expressões de require; docstrings via comentários ficam para depois.

use crate::ast::{Expr, Stmt, StrPart};

struct CheckDoc {
    name: String,
    tags: Vec<String>,
    validations: Vec<String>,
}

struct ScriptDoc {
    profile: Option<String>,
    consts: Vec<(String, String)>,
    functions: Vec<String>,
    imports: Vec<String>,
    checks: Vec<CheckDoc>,
    uses_fix: bool,
}

pub fn markdown(script_name: &str, program: &[Stmt]) -> String {
    let doc = collect(program);
    let mut out = format!("# Documentation: {script_name}\n\n");
    if let Some(p) = &doc.profile {
        out.push_str(&format!("**Profile:** {p}\n\n"));
    }
    if doc.uses_fix {
        out.push_str("> This script contains `fix::` operations — use it with the `pdfl fix` command.\n\n");
    }
    if !doc.imports.is_empty() {
        out.push_str("## Imports\n\n");
        for i in &doc.imports {
            out.push_str(&format!("- `{i}`\n"));
        }
        out.push('\n');
    }
    if !doc.consts.is_empty() {
        out.push_str("## Constants\n\n| Name | Value |\n|---|---|\n");
        for (name, value) in &doc.consts {
            out.push_str(&format!("| `{name}` | `{value}` |\n"));
        }
        out.push('\n');
    }
    if !doc.functions.is_empty() {
        out.push_str("## Functions\n\n");
        for f in &doc.functions {
            out.push_str(&format!("- `{f}`\n"));
        }
        out.push('\n');
    }
    out.push_str(&format!("## Checks ({})\n\n", doc.checks.len()));
    for c in &doc.checks {
        let tags = if c.tags.is_empty() {
            String::new()
        } else {
            format!(" — tags: {}", c.tags.iter().map(|t| format!("`{t}`")).collect::<Vec<_>>().join(", "))
        };
        out.push_str(&format!("### {}{tags}\n\n", c.name));
        if c.validations.is_empty() {
            out.push_str("(no direct validations)\n\n");
        } else {
            for v in &c.validations {
                out.push_str(&format!("- {v}\n"));
            }
            out.push('\n');
        }
    }
    out
}

pub fn html(script_name: &str, program: &[Stmt]) -> String {
    // ponytail: conversor mínimo do próprio Markdown gerado acima —
    // cobre só o subconjunto que o gerador emite (títulos, tabela, listas).
    let md = markdown(script_name, program);
    let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
    let inline = |s: &str| {
        // `código` -> <code>
        let mut out = String::new();
        for (i, part) in esc(s).split('`').enumerate() {
            out.push_str(if i % 2 == 1 { "<code>" } else { "" });
            out.push_str(part);
            out.push_str(if i % 2 == 1 { "</code>" } else { "" });
        }
        out
    };
    let mut body = String::new();
    let mut in_list = false;
    let mut in_table = false;
    let close = |body: &mut String, in_list: &mut bool, in_table: &mut bool| {
        if *in_list {
            body.push_str("</ul>\n");
            *in_list = false;
        }
        if *in_table {
            body.push_str("</table>\n");
            *in_table = false;
        }
    };
    for line in md.lines() {
        if let Some(t) = line.strip_prefix("### ") {
            close(&mut body, &mut in_list, &mut in_table);
            body.push_str(&format!("<h3>{}</h3>\n", inline(t)));
        } else if let Some(t) = line.strip_prefix("## ") {
            close(&mut body, &mut in_list, &mut in_table);
            body.push_str(&format!("<h2>{}</h2>\n", inline(t)));
        } else if let Some(t) = line.strip_prefix("# ") {
            body.push_str(&format!("<h1>{}</h1>\n", inline(t)));
        } else if let Some(t) = line.strip_prefix("- ") {
            if !in_list {
                body.push_str("<ul>\n");
                in_list = true;
            }
            body.push_str(&format!("<li>{}</li>\n", inline(t)));
        } else if line.starts_with('|') {
            if line.contains("---") {
                continue;
            }
            if !in_table {
                body.push_str("<table>\n");
                in_table = true;
            }
            let cells: Vec<String> =
                line.trim_matches('|').split('|').map(|c| inline(c.trim())).collect();
            body.push_str(&format!("<tr><td>{}</td></tr>\n", cells.join("</td><td>")));
        } else if let Some(t) = line.strip_prefix("> ") {
            close(&mut body, &mut in_list, &mut in_table);
            body.push_str(&format!("<blockquote>{}</blockquote>\n", inline(t)));
        } else if line.is_empty() {
            close(&mut body, &mut in_list, &mut in_table);
        } else {
            body.push_str(&format!("<p>{}</p>\n", inline(line)));
        }
    }
    close(&mut body, &mut in_list, &mut in_table);
    format!(
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <title>Documentation — {name}</title>\n<style>\n\
         body {{ font-family: system-ui, sans-serif; margin: 2rem auto; max-width: 55rem; color: #222; }}\n\
         table {{ border-collapse: collapse; }} td {{ border: 1px solid #ddd; padding: 0.4rem 0.8rem; }}\n\
         code {{ background: #f3f3f3; padding: 0.1rem 0.3rem; border-radius: 3px; }}\n\
         blockquote {{ border-left: 4px solid #b7791f; margin: 0; padding: 0.3rem 1rem; background: #fdf6e3; }}\n\
         </style>\n</head>\n<body>\n{body}</body>\n</html>\n",
        name = script_name
    )
}

fn collect(program: &[Stmt]) -> ScriptDoc {
    let mut doc = ScriptDoc {
        profile: None,
        consts: Vec::new(),
        functions: Vec::new(),
        imports: Vec::new(),
        checks: Vec::new(),
        uses_fix: false,
    };
    walk(program, &mut doc);
    doc
}

fn walk(stmts: &[Stmt], doc: &mut ScriptDoc) {
    for stmt in stmts {
        match stmt {
            Stmt::Profile { name, body } => {
                doc.profile = Some(name.clone());
                walk(body, doc);
            }
            Stmt::Check { name, tags, body } => {
                let mut validations = Vec::new();
                collect_validations(body, &mut validations, doc);
                doc.checks.push(CheckDoc { name: name.clone(), tags: tags.clone(), validations });
            }
            Stmt::Const { name, value } => {
                doc.consts.push((name.clone(), value.to_string()));
                if expr_uses_fix(value) {
                    doc.uses_fix = true;
                }
            }
            Stmt::Function { name, params, body } => {
                doc.functions.push(format!("{name}({})", params.join(", ")));
                collect_validations(body, &mut Vec::new(), doc); // só para uses_fix
            }
            Stmt::Import { path } => doc.imports.push(path.clone()),
            Stmt::Rule { name, pages, body } => {
                let mut validations = Vec::new();
                collect_validations(body, &mut validations, doc);
                let escopo = if pages.is_some() { " (selected pages)" } else { " (all pages)" };
                doc.checks.push(CheckDoc {
                    name: format!("{name} — rule{escopo}"),
                    tags: vec![],
                    validations,
                });
            }
            Stmt::Expr(e) if expr_uses_fix(e) => doc.uses_fix = true,
            _ => {}
        }
    }
}

/// Descrição de cada assert/require do corpo (recursivo em blocos).
fn collect_validations(stmts: &[Stmt], out: &mut Vec<String>, doc: &mut ScriptDoc) {
    for stmt in stmts {
        match stmt {
            Stmt::Assert { message, source, .. } => {
                let desc = match message {
                    Some(Expr::Str(parts)) => {
                        let text: String = parts
                            .iter()
                            .map(|p| match p {
                                StrPart::Lit(l) => l.clone(),
                                StrPart::Interp(e) => format!("#{{{e}}}"),
                            })
                            .collect();
                        format!("{text} *(condition: `{source}`)*")
                    }
                    _ => format!("`{source}`"),
                };
                out.push(desc);
            }
            Stmt::Expr(e) => {
                if expr_uses_fix(e) {
                    doc.uses_fix = true;
                }
                if let Expr::Call { block: Some(b), .. } = e {
                    collect_validations(&b.body, out, doc);
                }
            }
            Stmt::Assign { value, .. } | Stmt::Const { value, .. } => {
                if expr_uses_fix(value) {
                    doc.uses_fix = true;
                }
            }
            _ => {}
        }
    }
}

fn expr_uses_fix(expr: &Expr) -> bool {
    match expr {
        Expr::Call { name, args, block, recv } => {
            name.starts_with("fix::")
                || args.iter().any(|a| expr_uses_fix(&a.value))
                || block.as_ref().is_some_and(|b| {
                    b.body.iter().any(|s| matches!(s, Stmt::Expr(e) if expr_uses_fix(e)))
                })
                || recv.as_ref().is_some_and(|r| expr_uses_fix(r))
        }
        Expr::Binary { left, right, .. } => expr_uses_fix(left) || expr_uses_fix(right),
        Expr::Unary { expr, .. } => expr_uses_fix(expr),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    const SRC: &str = r#"
profile "teste" {
  const MAX = 300
  check "TAC" tags: ["prepress"] {
    doc.pages.each { |page|
      assert page.tac <= MAX, "Página #{page.number} com TAC alto"
    }
    require !prepress::detect_hairlines(0.25)
  }
  check "Vazio" { }
}
"#;

    #[test]
    fn markdown_estrutura() {
        let md = markdown("teste.pdfl", &parse(SRC).unwrap());
        assert!(md.contains("# Documentation: teste.pdfl"));
        assert!(md.contains("**Profile:** teste"));
        assert!(md.contains("| `MAX` | `300` |"));
        assert!(md.contains("### TAC — tags: `prepress`"));
        assert!(md.contains("Página #{page.number} com TAC alto"), "{md}");
        assert!(md.contains("`!prepress::detect_hairlines(0.25)`"));
        assert!(md.contains("### Vazio"));
        assert!(md.contains("(no direct validations)"));
        assert!(!md.contains("pdfl fix"));
    }

    #[test]
    fn html_estrutura() {
        let h = html("teste.pdfl", &parse(SRC).unwrap());
        assert!(h.contains("<h1>Documentation: teste.pdfl</h1>"));
        assert!(h.contains("<h3>TAC — tags: <code>prepress</code></h3>"));
        assert!(h.contains("<td><code>MAX</code></td>"));
    }

    #[test]
    fn aviso_de_fix() {
        let md = markdown("n.pdfl", &parse("fix::add_page_numbers()").unwrap());
        assert!(md.contains("pdfl fix"), "{md}");
    }
}
