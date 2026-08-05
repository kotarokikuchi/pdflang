//! `pdfl fmt` command — formatter for .pdfl scripts.
//! Formats from the tokens (preserving comments and the author's line
//! breaks): 2-space indentation per brace level, one space around
//! operators, blank lines collapsed to a single one.

use crate::lexer::{tokenize_with_comments, LexError, StrSeg, Tok};

pub fn format_source(source: &str) -> Result<String, LexError> {
    let toks: Vec<Tok> = tokenize_with_comments(source)?.into_iter().map(|t| t.tok).collect();

    let mut out = String::new();
    let mut depth: usize = 0;
    let mut line: Vec<&Tok> = Vec::new();
    let mut blank_pending = false;

    for tok in toks.iter().chain(std::iter::once(&Tok::Eof)) {
        if !matches!(tok, Tok::Newline | Tok::Eof) {
            line.push(tok);
            continue;
        }
        if line.is_empty() {
            // blank line: collapses runs and never opens the file
            blank_pending = !out.is_empty();
            continue;
        }
        // dedent first if the line starts by closing blocks
        let leading_closers = line.iter().take_while(|t| matches!(t, Tok::RBrace)).count();
        let indent = depth.saturating_sub(leading_closers);
        if blank_pending {
            out.push('\n');
            blank_pending = false;
        }
        out.push_str(&"  ".repeat(indent));
        out.push_str(&render_line(&line));
        out.push('\n');
        for t in &line {
            match t {
                Tok::LBrace => depth += 1,
                Tok::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        line.clear();
    }
    Ok(out)
}

fn render_line(tokens: &[&Tok]) -> String {
    let mut out = String::new();
    let mut in_block_params = false; // estado ANTES do token atual
    for (i, tok) in tokens.iter().enumerate() {
        if i > 0 && needs_space(tokens[i - 1], tok, in_block_params) {
            out.push(' ');
        }
        if matches!(tok, Tok::Pipe) {
            in_block_params = !in_block_params;
        }
        out.push_str(&render_token(tok));
    }
    out
}

/// Should there be a space between `prev` and `next`?
/// `in_params` = we are inside a block's `|...|` (before `next`).
fn needs_space(prev: &Tok, next: &Tok, in_params: bool) -> bool {
    use Tok::*;
    // left-hugging: nothing comes after these
    if matches!(prev, LParen | LBracket | Dot | ColonColon | Bang) {
        return false;
    }
    // the opening pipe hugs the first parameter: "{ |page"
    if matches!(prev, Pipe) && in_params {
        return false;
    }
    // right-hugging: nothing comes before these
    if matches!(next, RParen | RBracket | Comma | Dot | ColonColon | Colon) {
        return false;
    }
    if matches!(next, Pipe) {
        // inside params it is the closing pipe ("page|"); outside, the opening one (" |")
        return !in_params;
    }
    // call: ident( hugs; grouping after an operator gets a space
    if matches!(next, LParen) {
        return !matches!(prev, Ident(_) | RParen | RBracket);
    }
    true
}

fn render_token(tok: &Tok) -> String {
    use Tok::*;
    match tok {
        Int(n) => n.to_string(),
        Float(n) => format!("{n:?}"),
        UnitNum(_, raw) => raw.clone(),
        Str(segs) => render_string(segs),
        True => "true".into(),
        False => "false".into(),
        Profile => "profile".into(),
        Check => "check".into(),
        Const => "const".into(),
        Assert => "assert".into(),
        Require => "require".into(),
        Function => "function".into(),
        Import => "import".into(),
        Rule => "rule".into(),
        On => "on".into(),
        Ident(s) => s.clone(),
        LBrace => "{".into(),
        RBrace => "}".into(),
        LParen => "(".into(),
        RParen => ")".into(),
        LBracket => "[".into(),
        RBracket => "]".into(),
        Comma => ",".into(),
        Colon => ":".into(),
        ColonColon => "::".into(),
        Dot => ".".into(),
        Pipe => "|".into(),
        Eq => "=".into(),
        EqEq => "==".into(),
        NotEq => "!=".into(),
        Lt => "<".into(),
        LtEq => "<=".into(),
        Gt => ">".into(),
        GtEq => ">=".into(),
        Plus => "+".into(),
        Minus => "-".into(),
        Star => "*".into(),
        Slash => "/".into(),
        Bang => "!".into(),
        AndAnd => "&&".into(),
        OrOr => "||".into(),
        Comment(text) => format!("// {text}"),
        Newline | Eof => String::new(),
    }
}

fn render_string(segs: &[StrSeg]) -> String {
    let mut out = String::from("\"");
    for seg in segs {
        match seg {
            StrSeg::Lit(l) => {
                for c in l.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\t' => out.push_str("\\t"),
                        _ => out.push(c),
                    }
                }
            }
            StrSeg::Interp(src) => {
                out.push_str("#{");
                out.push_str(src.trim());
                out.push('}');
            }
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn indents_and_spaces() {
        let src = "profile \"x\"{\ncheck \"a\"   {\nrequire doc.page_count>0\n}\n}\n";
        let want = "profile \"x\" {\n  check \"a\" {\n    require doc.page_count > 0\n  }\n}\n";
        assert_eq!(format_source(src).unwrap(), want);
    }

    #[test]
    fn preserves_comments() {
        let src = "// cabeçalho\ncheck \"a\" {\n  require doc.title != \"\" // trailing\n}\n";
        let got = format_source(src).unwrap();
        assert!(got.starts_with("// cabeçalho\n"), "{got}");
        assert!(got.contains("!= \"\" // trailing"), "{got}");
    }

    #[test]
    fn blocks_and_interpolation() {
        let src = "doc.fonts.each{|font|\nassert font.is_embedded,\"Fonte #{ font.name } ruim\"\n}\n";
        let got = format_source(src).unwrap();
        assert_eq!(
            got,
            "doc.fonts.each { |font|\n  assert font.is_embedded, \"Fonte #{font.name} ruim\"\n}\n"
        );
    }

    #[test]
    fn collapses_blank_lines() {
        let src = "const A = 1\n\n\n\nconst B = A\n";
        assert_eq!(format_source(src).unwrap(), "const A = 1\n\nconst B = A\n");
    }

    #[test]
    fn preserves_units() {
        let src = "const SANGRIA = 3mm\nconst TAC = 300%\nrequire x > 2.5cm\n";
        assert_eq!(format_source(src).unwrap(), src);
    }

    #[test]
    fn idempotent() {
        let src = r#"
// exemplo
profile "p" {
  const MAX = 300
  check "TAC" tags: ["prepress"] {
    doc.pages.each { |page|
      assert prepress::calculate_tac(page) <= MAX, "TAC #{page.tac} alto"
    }
    require !prepress::detect_hairlines(0.25)
    x = [1, 2.5, "três"]
    require x.length == 3 && (1 + 2) * 3 == 9
  }
}
"#;
        let once = format_source(src).unwrap();
        let twice = format_source(&once).unwrap();
        assert_eq!(once, twice, "formatador não é idempotente:\n{once}");
    }
}
