//! Hand-written lexer for ESLD (spec §4).
//!
//! Produces a flat token stream with line positions. Comments are tokens:
//! the formatter (§ `fmt`) re-emits them, so round-trip preserves them.
//!
//! Quantities are lexed as a single `Number` token covering the digits and
//! any immediately-attached unit (`63A`, `2.5mm2`, `1ph`, `1000W/m2`,
//! `6%`). Whitespace-separated units (`230 V`) are accepted by the spec;
//! the parser merges a `Number` followed by an `Ident` in value position.

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tok {
    /// Identifier or keyword (contextual; the parser matches text).
    Ident(String),
    /// Number with attached unit text, if any (`63A` -> number "63", unit "A").
    Number { number: String, unit: String },
    /// Double-quoted string, decoded.
    Str(String),
    /// Line (`//`) or block (`/* */`) comment; `text` is the raw source.
    Comment(String),
    /// Punctuation / operators.
    Sym(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
}

impl Token {
    /// The canonical text of this token as the formatter emits it.
    pub fn text(&self) -> String {
        match &self.tok {
            Tok::Ident(s) => s.clone(),
            Tok::Number { number, unit } => format!("{number}{unit}"),
            Tok::Str(s) => format!("\"{s}\""),
            Tok::Comment(s) => s.clone(),
            Tok::Sym(s) => (*s).to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub line: u32,
}

const SYMBOLS: &[&str] = &[
    "->", "--", "<-", "==", "!=", "<=", ">=", "&&", "||", "..", ".", "{", "}", "[", "]", "(", ")",
    ",", ";", ":", "=", "!", "*", "?", "-",
];

fn is_unit_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '%' || c == '^' || c == '/' || c == 'µ'
}

pub fn lex(src: &str) -> Result<Vec<Token>, LexError> {
    let src = src.strip_prefix('\u{feff}').unwrap_or(src);
    let bytes = src.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0usize;
    let mut line: u32 = 1;

    macro_rules! err {
        ($msg:expr) => {
            return Err(LexError {
                message: $msg.to_owned(),
                line,
            })
        };
    }

    while i < bytes.len() {
        let c = src[i..].chars().next().unwrap();
        match c {
            '\n' => {
                i += 1;
                line += 1;
            }
            ' ' | '\t' | '\r' => {
                i += 1;
            }
            '/' if src[i..].starts_with("//") => {
                let start = i;
                i += 2;
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                let text = src[start..i].trim_end().to_string();
                tokens.push(Token {
                    tok: Tok::Comment(text),
                    line,
                });
            }
            '/' if src[i..].starts_with("/*") => {
                let start = i;
                let start_line = line;
                i += 2;
                loop {
                    if i >= bytes.len() {
                        err!("unterminated block comment");
                    }
                    if src[i..].starts_with("*/") {
                        i += 2;
                        break;
                    }
                    if bytes[i] == b'\n' {
                        line += 1;
                    }
                    i += 1;
                }
                tokens.push(Token {
                    tok: Tok::Comment(src[start..i].to_string()),
                    line: start_line,
                });
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                loop {
                    let Some(ch) = src[i..].chars().next() else {
                        err!("unterminated string");
                    };
                    i += ch.len_utf8();
                    match ch {
                        '"' => break,
                        '\n' => err!("newline in string"),
                        '\\' => {
                            let Some(esc) = src[i..].chars().next() else {
                                err!("unterminated escape");
                            };
                            i += esc.len_utf8();
                            match esc {
                                'n' => s.push('\n'),
                                't' => s.push('\t'),
                                'u' => {
                                    let hex: String = src[i..].chars().take(4).collect();
                                    if hex.chars().count() < 4 {
                                        err!("bad \\u escape");
                                    }
                                    i += 4;
                                    let cp =
                                        u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
                                    match cp {
                                        Some(ch) => s.push(ch),
                                        None => err!("bad \\u escape"),
                                    }
                                }
                                other => s.push(other),
                            }
                        }
                        other => s.push(other),
                    }
                }
                tokens.push(Token {
                    tok: Tok::Str(s),
                    line,
                });
            }
            c if c.is_ascii_digit() => {
                let start = i;
                while i < bytes.len()
                    && (bytes[i].is_ascii_digit()
                        || (bytes[i] == b'.' && bytes.get(i + 1) != Some(&b'.')))
                {
                    i += 1;
                }
                let number = &src[start..i];
                let unit_start = i;
                while i < bytes.len() {
                    let ch = src[i..].chars().next().unwrap();
                    if is_unit_char(ch) {
                        i += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    tok: Tok::Number {
                        number: number.to_string(),
                        unit: src[unit_start..i].to_string(),
                    },
                    line,
                });
            }
            c if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < bytes.len() {
                    let ch = src[i..].chars().next().unwrap();
                    // `-` continues hyphenated identifiers, but never when it
                    // begins an arrow (`A->B` lexes as `A`, `->`, `B`).
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        i += ch.len_utf8();
                    } else if ch == '-'
                        && !src[i + 1..].starts_with('>')
                        && !src[i + 1..].starts_with('-')
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    tok: Tok::Ident(src[start..i].to_string()),
                    line,
                });
            }
            _ => {
                let mut sym = None;
                for s in SYMBOLS {
                    if src[i..].starts_with(s) {
                        sym = Some(*s);
                        break;
                    }
                }
                let Some(sym) = sym else {
                    err!(format!("unexpected character {c:?}"));
                };
                i += sym.len();
                tokens.push(Token {
                    tok: Tok::Sym(sym),
                    line,
                });
            }
        }
    }
    Ok(tokens)
}
