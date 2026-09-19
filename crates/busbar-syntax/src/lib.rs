//! Lexer, parser, AST, trivia, and canonical formatter for ESLD.
//!
//! Scope: spec §4 (lexical structure) and §6 (grammar); the formatter owns
//! the round-trip contract (spec §17.1).
//!
//! Phase 1 status: full grammar parses; formatter is a token-stream
//! renderer (canonical spacing, expanded blocks, preserved comments).

pub mod ast;
pub mod fmt;
pub mod lexer;
pub mod parser;

pub use lexer::{LexError, Tok, Token, lex};
pub use parser::{ParseError, parse};

/// Convenience: parse and map the error to a `line: message` string.
pub fn parse_or_string(source: &str) -> Result<ast::Document, String> {
    parse(source).map_err(|e| format!("line {}: {}", e.line, e.message))
}
