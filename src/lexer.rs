//! Lexer for the PDFLang language (.pdfl).

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum StrSeg {
    /// Literal chunk of the string.
    Lit(String),
    /// Raw source of a `#{...}` interpolation (parsed later).
    Interp(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    // Literais
    Int(i64),
    Float(f64),
    /// Number with a unit (3mm, 300%): the value already converted to points
    /// (for lengths) + the original text preserved for the formatter.
    UnitNum(f64, String),
    Str(Vec<StrSeg>),
    True,
    False,
    // Palavras-chave
    Profile,
    Check,
    Const,
    Assert,
    Require,
    Function,
    Import,
    Rule,
    On,
    Ident(String),
    // Symbols
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    ColonColon,
    Dot,
    Pipe,
    Eq,       // =
    EqEq,     // ==
    NotEq,    // !=
    Lt,
    LtEq,
    Gt,
    GtEq,
    Plus,
    Minus,
    Star,
    Slash,
    Bang,
    AndAnd,
    OrOr,
    Newline,
    /// A `// ...` comment — only emitted by `tokenize_with_comments` (fmt).
    Comment(String),
    Eof,
}

impl fmt::Display for Tok {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Tok::Ident(s) => write!(f, "{s}"),
            Tok::Str(_) => write!(f, "string"),
            Tok::Newline => write!(f, "line break"),
            Tok::Eof => write!(f, "end of file"),
            other => write!(f, "{other:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub col: usize,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "lexical error at line {}, column {}: {}", self.line, self.col, self.message)
    }
}

impl std::error::Error for LexError {}

pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
    tokenize_impl(source, false)
}

/// Variant for the formatter: keeps comments as tokens.
pub fn tokenize_with_comments(source: &str) -> Result<Vec<Token>, LexError> {
    tokenize_impl(source, true)
}

fn tokenize_impl(source: &str, keep_comments: bool) -> Result<Vec<Token>, LexError> {
    let chars: Vec<char> = source.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut col = 1;

    macro_rules! err {
        ($($arg:tt)*) => {
            return Err(LexError { message: format!($($arg)*), line, col })
        };
    }

    while i < chars.len() {
        let c = chars[i];
        let (tline, tcol) = (line, col);
        let mut push = |tok: Tok| toks.push(Token { tok, line: tline, col: tcol });

        match c {
            ' ' | '\t' | '\r' => {
                i += 1;
                col += 1;
            }
            '\n' => {
                push(Tok::Newline);
                i += 1;
                line += 1;
                col = 1;
            }
            '/' if chars.get(i + 1) == Some(&'/') => {
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                if keep_comments {
                    let text: String = chars[start + 2..i].iter().collect();
                    push(Tok::Comment(text.trim().to_string()));
                }
                col += i - start;
            }
            '"' => {
                i += 1;
                col += 1;
                let mut segs = Vec::new();
                let mut lit = String::new();
                loop {
                    match chars.get(i) {
                        None => err!("unterminated string"),
                        Some('"') => {
                            i += 1;
                            col += 1;
                            break;
                        }
                        Some('\n') => err!("unterminated string (line break inside string)"),
                        Some('\\') => {
                            match chars.get(i + 1) {
                                Some('n') => lit.push('\n'),
                                Some('t') => lit.push('\t'),
                                Some('"') => lit.push('"'),
                                Some('\\') => lit.push('\\'),
                                Some('#') => lit.push('#'),
                                None => err!("unterminated string"),
                                // Escapes desconhecidos (\w, \d, ...) passam
                                // literalmente — strings carregam regex.
                                Some(&other) => {
                                    lit.push('\\');
                                    lit.push(other);
                                }
                            }
                            i += 2;
                            col += 2;
                        }
                        Some('#') if chars.get(i + 1) == Some(&'{') => {
                            if !lit.is_empty() {
                                segs.push(StrSeg::Lit(std::mem::take(&mut lit)));
                            }
                            i += 2;
                            col += 2;
                            let mut depth = 1;
                            let mut src = String::new();
                            loop {
                                match chars.get(i) {
                                    None | Some('\n') => err!("unterminated #{{...}} interpolation"),
                                    Some('{') => {
                                        depth += 1;
                                        src.push('{');
                                    }
                                    Some('}') => {
                                        depth -= 1;
                                        if depth == 0 {
                                            i += 1;
                                            col += 1;
                                            break;
                                        }
                                        src.push('}');
                                    }
                                    Some(&ch) => src.push(ch),
                                }
                                i += 1;
                                col += 1;
                            }
                            segs.push(StrSeg::Interp(src));
                        }
                        Some(&ch) => {
                            lit.push(ch);
                            i += 1;
                            col += 1;
                        }
                    }
                }
                if !lit.is_empty() || segs.is_empty() {
                    segs.push(StrSeg::Lit(lit));
                }
                push(Tok::Str(segs));
            }
            '0'..='9' => {
                let start = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                let mut is_float = false;
                if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(|c| c.is_ascii_digit()) {
                    is_float = true;
                    i += 1;
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        i += 1;
                    }
                }
                let text: String = chars[start..i].iter().collect();
                col += i - start;
                // Unit suffixes: 3mm/2cm/1in/10pt become POINTS; 300% keeps its value.
                if chars.get(i) == Some(&'%') {
                    i += 1;
                    col += 1;
                    push(Tok::UnitNum(text.parse().unwrap(), format!("{text}%")));
                } else if chars.get(i).is_some_and(|c| c.is_alphabetic()) {
                    let ustart = i;
                    while i < chars.len() && chars[i].is_alphanumeric() {
                        i += 1;
                    }
                    let unit: String = chars[ustart..i].iter().collect();
                    col += i - ustart;
                    let factor = match unit.as_str() {
                        "pt" => 1.0,
                        "mm" => 72.0 / 25.4,
                        "cm" => 720.0 / 25.4,
                        "in" => 72.0,
                        _ => err!("unknown unit: '{unit}' (use pt, mm, cm, in or %)"),
                    };
                    push(Tok::UnitNum(text.parse::<f64>().unwrap() * factor, format!("{text}{unit}")));
                } else if is_float {
                    push(Tok::Float(text.parse().unwrap()));
                } else {
                    push(Tok::Int(text.parse().unwrap()));
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                col += i - start;
                push(match word.as_str() {
                    "profile" => Tok::Profile,
                    "check" => Tok::Check,
                    "const" => Tok::Const,
                    "assert" => Tok::Assert,
                    "require" => Tok::Require,
                    "function" => Tok::Function,
                    "import" => Tok::Import,
                    "rule" => Tok::Rule,
                    "on" => Tok::On,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    _ => Tok::Ident(word),
                });
            }
            _ => {
                let two: String = chars[i..(i + 2).min(chars.len())].iter().collect();
                let (tok, len) = match two.as_str() {
                    "==" => (Tok::EqEq, 2),
                    "!=" => (Tok::NotEq, 2),
                    "<=" => (Tok::LtEq, 2),
                    ">=" => (Tok::GtEq, 2),
                    "&&" => (Tok::AndAnd, 2),
                    "||" => (Tok::OrOr, 2),
                    "::" => (Tok::ColonColon, 2),
                    _ => match c {
                        '{' => (Tok::LBrace, 1),
                        '}' => (Tok::RBrace, 1),
                        '(' => (Tok::LParen, 1),
                        ')' => (Tok::RParen, 1),
                        '[' => (Tok::LBracket, 1),
                        ']' => (Tok::RBracket, 1),
                        ',' => (Tok::Comma, 1),
                        ':' => (Tok::Colon, 1),
                        '.' => (Tok::Dot, 1),
                        '|' => (Tok::Pipe, 1),
                        '=' => (Tok::Eq, 1),
                        '<' => (Tok::Lt, 1),
                        '>' => (Tok::Gt, 1),
                        '+' => (Tok::Plus, 1),
                        '-' => (Tok::Minus, 1),
                        '*' => (Tok::Star, 1),
                        '/' => (Tok::Slash, 1),
                        '!' => (Tok::Bang, 1),
                        _ => err!("unexpected character: {c:?}"),
                    },
                };
                push(tok);
                i += len;
                col += len;
            }
        }
    }

    toks.push(Token { tok: Tok::Eof, line, col });
    Ok(toks)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        tokenize(src).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn basics() {
        assert_eq!(
            toks("const X = 42"),
            vec![Tok::Const, Tok::Ident("X".into()), Tok::Eq, Tok::Int(42), Tok::Eof]
        );
    }

    #[test]
    fn operators_and_float() {
        assert_eq!(
            toks("1.5 >= 2 && !ok"),
            vec![
                Tok::Float(1.5),
                Tok::GtEq,
                Tok::Int(2),
                Tok::AndAnd,
                Tok::Bang,
                Tok::Ident("ok".into()),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn comment_and_newline() {
        assert_eq!(
            toks("a // comentário\nb"),
            vec![Tok::Ident("a".into()), Tok::Newline, Tok::Ident("b".into()), Tok::Eof]
        );
    }

    #[test]
    fn simple_string() {
        assert_eq!(toks(r#""oi""#), vec![Tok::Str(vec![StrSeg::Lit("oi".into())]), Tok::Eof]);
    }

    #[test]
    fn interpolated_string() {
        assert_eq!(
            toks(r#""Fonte #{font.name} ausente""#),
            vec![
                Tok::Str(vec![
                    StrSeg::Lit("Fonte ".into()),
                    StrSeg::Interp("font.name".into()),
                    StrSeg::Lit(" ausente".into()),
                ]),
                Tok::Eof
            ]
        );
    }

    #[test]
    fn keywords_and_block() {
        assert_eq!(
            toks("check \"x\" { |p| }"),
            vec![
                Tok::Check,
                Tok::Str(vec![StrSeg::Lit("x".into())]),
                Tok::LBrace,
                Tok::Pipe,
                Tok::Ident("p".into()),
                Tok::Pipe,
                Tok::RBrace,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn unterminated_string() {
        assert!(tokenize("\"abc").is_err());
    }
}
