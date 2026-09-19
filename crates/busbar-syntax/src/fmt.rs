//! Canonical formatter for ESLD (`busbar fmt`, implementation plan §5.1).
//!
//! A token-stream renderer: it re-emits the lexed tokens (comments
//! included) with canonical spacing, 2-space indentation, blocks expanded
//! to multiple lines, and one statement per line. Because it works from
//! tokens, unknown properties, comments, and ordering survive round-trip
//! by construction, and it is trivially idempotent.

use crate::LexError;
use crate::lexer::{Tok, Token, lex};

/// Formats `src` into the canonical style. Errors if `src` does not lex.
pub fn format(src: &str) -> Result<String, LexError> {
    let tokens = lex(src)?;
    Ok(render(&tokens))
}

/// Significant tokens (no comments) as canonical text — the AST-equality
/// substrate for round-trip checks.
pub fn significant_tokens(src: &str) -> Result<Vec<String>, LexError> {
    Ok(lex(src)?
        .into_iter()
        .filter(|t| !matches!(t.tok, Tok::Comment(_)))
        .map(|t| t.text())
        .collect())
}

fn render(tokens: &[Token]) -> String {
    let mut f = Fmt::default();
    for token in tokens {
        match &token.tok {
            Tok::Comment(text) => f.comment(text, token.line),
            Tok::Sym("{") => {
                f.push("{", token.line);
                f.depth += 1;
                f.pending_nl = true;
            }
            Tok::Sym("}") => {
                f.depth = f.depth.saturating_sub(1);
                f.flush_nl();
                f.indent();
                f.out.push('}');
                f.last = Some("}".to_string());
                f.last_line = token.line;
            }
            Tok::Sym(";") => {
                f.push(";", token.line);
                f.pending_nl = true;
            }
            _ => f.push(&token.text(), token.line),
        }
    }
    f.out.push('\n');
    f.out
}

#[derive(Default)]
struct Fmt {
    out: String,
    depth: usize,
    /// Last emitted code-token text (for spacing decisions).
    last: Option<String>,
    /// Line of the last emitted code token.
    last_line: u32,
    /// A newline is pending before the next token.
    pending_nl: bool,
}

impl Fmt {
    fn flush_nl(&mut self) {
        if self.pending_nl {
            self.out.push('\n');
            self.pending_nl = false;
            self.last = None;
        }
    }

    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    fn push(&mut self, text: &str, line: u32) {
        self.flush_nl();
        if self.last.is_none() {
            self.indent();
        }
        let space = match (self.last.as_deref(), text) {
            (None, _) => false,
            (Some(_), _) if self.out.ends_with(' ') => false,
            (Some(l), t) => space_between(l, t),
        };
        if space {
            self.out.push(' ');
        }
        self.out.push_str(text);
        self.last = Some(text.to_string());
        self.last_line = line;
    }

    fn comment(&mut self, text: &str, line: u32) {
        if line == self.last_line && self.last.is_some() && !self.pending_nl {
            // Trailing comment: stays on the statement's line.
            self.out.push(' ');
            self.out.push_str(text);
            self.pending_nl = true;
            return;
        }
        self.flush_nl();
        if self.last.is_some() {
            self.out.push('\n');
            self.last = None;
        }
        self.indent();
        self.out.push_str(text);
        self.out.push('\n');
        self.last = None;
        // The line number bookkeeping stays with the previous code token;
        // a following token on the same source line still starts a new
        // output line, which is canonical.
    }
}

fn space_between(prev: &str, next: &str) -> bool {
    const OPEN: &[&str] = &["(", "[", ".", "!"];
    const CLOSE: &[&str] = &[";", ",", ")", "]", "."];
    if OPEN.contains(&prev) {
        return false;
    }
    if CLOSE.contains(&next) {
        return false;
    }
    true
}
