//! Recursive-descent parser for the ESLD grammar (spec §6).

use crate::LexError;
use crate::ast::*;
use crate::lexer::{Tok, Token, lex};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: u32,
}

pub fn parse(src: &str) -> Result<Document, ParseError> {
    let tokens = lex(src).map_err(|LexError { message, line }| ParseError { message, line })?;
    let mut p = Parser { tokens, pos: 0 };
    p.document()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn line(&self) -> u32 {
        self.peek().map(|t| t.line).unwrap_or(0)
    }

    fn next(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        self.pos += 1;
        t
    }

    fn at_sym(&self, s: &str) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Sym(x)) if *x == s)
    }

    fn at_ident(&self, s: &str) -> bool {
        matches!(self.peek().map(|t| &t.tok), Some(Tok::Ident(x)) if x == s)
    }

    fn eat_sym(&mut self, s: &str) -> Result<(), ParseError> {
        if self.at_sym(s) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.error(format!("expected `{s}`")))
        }
    }

    fn eat_ident(&mut self) -> Result<String, ParseError> {
        match self.peek().map(|t| t.tok.clone()) {
            Some(Tok::Ident(s)) => {
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.error("expected identifier")),
        }
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            line: self.line(),
        }
    }

    fn eat_opt_semi(&mut self) {
        if self.at_sym(";") {
            self.pos += 1;
        }
    }

    fn skip_trivia(&mut self) {
        while matches!(self.peek().map(|t| &t.tok), Some(Tok::Comment(_))) {
            self.pos += 1;
        }
    }

    fn document(&mut self) -> Result<Document, ParseError> {
        let mut statements = Vec::new();
        loop {
            self.skip_trivia();
            if self.peek().is_none() {
                break;
            }
            statements.push(self.statement()?);
        }
        Ok(Document { statements })
    }

    fn statement(&mut self) -> Result<Statement, ParseError> {
        let line = self.line();
        let keyword = self.eat_ident()?;
        let stmt = match keyword.as_str() {
            "profile" => {
                let value = self.string()?;
                self.eat_sym(";")?;
                Statement::Profile {
                    value,
                    span: Span { line },
                }
            }
            "include" => {
                let path = self.string()?;
                self.eat_sym(";")?;
                Statement::Include {
                    path,
                    span: Span { line },
                }
            }
            "note" => {
                let text = self.string()?;
                self.eat_sym(";")?;
                Statement::Note(NoteStmt {
                    text,
                    span: Span { line },
                })
            }
            "voltsys" => self.voltsys(line)?,
            "code" => {
                let name = self.string()?;
                let props = self.property_block()?;
                self.eat_opt_semi();
                Statement::Code {
                    name,
                    props,
                    span: Span { line },
                }
            }
            "type" => {
                let name = self.eat_ident()?;
                if self.at_sym(":") {
                    self.pos += 1;
                    let _ = self.type_ref()?;
                }
                self.skip_braced()?;
                self.eat_opt_semi();
                Statement::TypeDecl {
                    name,
                    span: Span { line },
                }
            }
            "board" => {
                let tag = self.eat_ident()?;
                let type_ref = self.opt_type_ref()?;
                let items = self.board_items()?;
                self.eat_opt_semi();
                Statement::Board {
                    tag,
                    type_ref,
                    items,
                    span: Span { line },
                }
            }
            "group" => {
                let name = self.eat_ident()?;
                self.eat_sym("=")?;
                let list = self.value()?;
                self.eat_sym(";")?;
                Statement::Group {
                    name,
                    list,
                    span: Span { line },
                }
            }
            "connect" => Statement::Connect(self.connect(line)?),
            "vendor" => {
                self.skip_braced_after_string()?;
                self.eat_opt_semi();
                Statement::Vendor {
                    span: Span { line },
                }
            }
            "scenario" => {
                let stmt = self.scenario(line)?;
                self.eat_opt_semi();
                stmt
            }
            "state" => {
                let stmt = self.state(line)?;
                self.eat_opt_semi();
                stmt
            }
            "interlock" => {
                let stmt = self.interlock(line)?;
                self.eat_opt_semi();
                stmt
            }
            "zone" => {
                let name = self.string()?;
                let props = self.property_block()?;
                self.eat_opt_semi();
                Statement::Zone {
                    name,
                    props,
                    span: Span { line },
                }
            }
            "layout" => {
                let items = self.layout_items()?;
                self.eat_opt_semi();
                Statement::Layout {
                    items,
                    span: Span { line },
                }
            }
            other => {
                // Node declaration: `TAG : type { … } ;?` or with a
                // redundant leading type introducer `type TAG : type`
                // (the style used throughout the spec examples).
                let mut tag = other.to_string();
                if !self.at_sym(":") && matches!(self.peek().map(|t| &t.tok), Some(Tok::Ident(_))) {
                    tag = self.eat_ident()?;
                }
                let other = tag;
                if self.at_sym(":") {
                    self.pos += 1;
                    let type_ref = self.type_ref()?;
                    let props = if self.at_sym("{") {
                        self.property_block()?
                    } else {
                        Vec::<Property>::new()
                    };
                    if self.at_sym(";") {
                        self.pos += 1;
                    }
                    Statement::Node(NodeDecl {
                        tag: other.to_string(),
                        type_ref,
                        props,
                        span: Span { line },
                    })
                } else {
                    return Err(self.error(format!(
                        "expected `:` after tag `{other}` (node declaration)"
                    )));
                }
            }
        };
        Ok(stmt)
    }

    fn string(&mut self) -> Result<String, ParseError> {
        match self.peek().map(|t| t.tok.clone()) {
            Some(Tok::Str(s)) => {
                self.pos += 1;
                Ok(s)
            }
            _ => Err(self.error("expected string")),
        }
    }

    fn type_ref(&mut self) -> Result<String, ParseError> {
        let mut parts = vec![self.eat_ident()?];
        while self.at_sym(".") {
            self.pos += 1;
            parts.push(self.eat_ident()?);
        }
        Ok(parts.join("."))
    }

    fn opt_type_ref(&mut self) -> Result<Option<String>, ParseError> {
        if self.at_sym(":") {
            self.pos += 1;
            Ok(Some(self.type_ref()?))
        } else {
            Ok(None)
        }
    }

    fn voltsys(&mut self, line: u32) -> Result<Statement, ParseError> {
        let name = self.eat_ident()?;
        self.eat_sym("=")?;
        let body = if self.at_sym("{") {
            let props = self.property_block()?;
            self.eat_opt_semi();
            VoltsysBody::Block(props)
        } else {
            let a = self.value()?;
            self.eat_sym(",")?;
            let b = self.value()?;
            self.eat_sym(",")?;
            let c = self.value()?;
            self.eat_sym(";")?;
            VoltsysBody::Positional(a, b, c)
        };
        Ok(Statement::Voltsys {
            name,
            body,
            span: Span { line },
        })
    }

    fn property_block(&mut self) -> Result<Vec<Property>, ParseError> {
        self.eat_sym("{")?;
        let mut props = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            if self.at_ident("note") {
                // Note statements inside blocks are consumed; the IR does
                // not need them and the formatter re-renders from tokens.
                self.pos += 1;
                let _ = self.string()?;
                self.eat_sym(";")?;
                continue;
            }
            props.push(self.property()?);
        }
        Ok(props)
    }

    fn property(&mut self) -> Result<Property, ParseError> {
        let line = self.line();
        let name = self.eat_ident()?;
        self.eat_sym("=")?;
        let value = self.value()?;
        self.eat_sym(";")?;
        Ok(Property {
            name,
            value,
            span: Span { line },
        })
    }

    /// Parses a value; merges `Number` + following `Ident` into a quantity
    /// when whitespace-separated (`230 V`, spec §4.6).
    fn value(&mut self) -> Result<ValueNode, ParseError> {
        if self.peek().is_none() {
            return Err(self.error("expected value, found end of input"));
        }
        let line = self.line();
        // Unary minus on numeric values (`>= -5kW`).
        if self.at_sym("-") {
            self.pos += 1;
            let inner = self.value()?;
            let negated = match inner.value {
                Value::Quantity { number, unit } => Value::Quantity {
                    number: format!("-{number}"),
                    unit,
                },
                Value::Number(number) => Value::Number(format!("-{number}")),
                other => other,
            };
            return Ok(ValueNode {
                value: negated,
                span: Span { line },
            });
        }
        let value = match self.next().tok {
            Tok::Number { number, unit } => {
                if unit.is_empty() {
                    // Possibly `230 V` — merge an immediately-following ident.
                    if let Some(Tok::Ident(u)) = self.peek().map(|t| t.tok.clone()) {
                        if is_unit_like(&u) {
                            self.pos += 1;
                            Value::Quantity { number, unit: u }
                        } else {
                            Value::Number(number)
                        }
                    } else {
                        Value::Number(number)
                    }
                } else {
                    Value::Quantity { number, unit }
                }
            }
            Tok::Str(s) => Value::Str(s),
            Tok::Ident(s) => match s.as_str() {
                "yes" | "true" => Value::Bool(true),
                "no" | "false" => Value::Bool(false),
                _ => {
                    let mut text = s;
                    while self.at_sym(".") {
                        // Only join when the dot is followed by an ident and
                        // the whole is a dotted reference, not a range.
                        let save = self.pos;
                        self.pos += 1;
                        match self.peek().map(|t| t.tok.clone()) {
                            Some(Tok::Ident(part)) => {
                                self.pos += 1;
                                text.push('.');
                                text.push_str(&part);
                            }
                            Some(Tok::Number { number, unit }) if unit == "ph" => {
                                text.push('.');
                                text.push_str(&number);
                                text.push_str("ph");
                                self.pos += 1;
                            }
                            _ => {
                                self.pos = save;
                                break;
                            }
                        }
                    }
                    Value::Ident(text)
                }
            },
            Tok::Sym("[") => {
                let mut items = Vec::new();
                if !self.at_sym("]") {
                    loop {
                        items.push(self.value()?);
                        if self.at_sym(",") {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                }
                self.eat_sym("]")?;
                Value::List(items)
            }
            Tok::Sym("{") => {
                let mut props = Vec::new();
                loop {
                    self.skip_trivia();
                    if self.at_sym("}") {
                        self.pos += 1;
                        break;
                    }
                    props.push(self.property()?);
                }
                Value::Block(props)
            }
            other => {
                return Err(ParseError {
                    message: format!("expected value, found {other:?}"),
                    line,
                });
            }
        };
        // Range: `a .. b`
        if self.at_sym("..") {
            self.pos += 1;
            let hi = self.value()?;
            return Ok(ValueNode {
                value: Value::Range(
                    Box::new(ValueNode {
                        value,
                        span: Span { line },
                    }),
                    Box::new(hi),
                ),
                span: Span { line },
            });
        }
        Ok(ValueNode {
            value,
            span: Span { line },
        })
    }

    fn board_items(&mut self) -> Result<Vec<BoardItem>, ParseError> {
        self.eat_sym("{")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            let line = self.line();
            if self.at_ident("circuit") {
                self.pos += 1;
                let tag = self.eat_ident()?;
                let type_ref = self.opt_type_ref()?;
                let items_inner = self.circuit_items()?;
                self.eat_opt_semi();
                items.push(BoardItem::Circuit(CircuitDecl {
                    tag,
                    type_ref,
                    items: items_inner,
                    span: Span { line },
                }));
            } else if self.at_ident("bus") {
                self.pos += 1;
                let tag = self.eat_ident()?;
                let type_ref = self.opt_type_ref()?;
                let mut props = Vec::new();
                self.eat_sym("{")?;
                loop {
                    self.skip_trivia();
                    if self.at_sym("}") {
                        self.pos += 1;
                        break;
                    }
                    props.push(self.property()?);
                }
                self.eat_opt_semi();
                items.push(BoardItem::Bus(BusDecl {
                    tag,
                    type_ref,
                    props,
                    span: Span { line },
                }));
            } else if self.at_ident("connect") {
                self.pos += 1; // past the keyword; connect() starts at the first endpoint
                items.push(BoardItem::Connect(self.connect(line)?));
            } else if self.at_ident("note") {
                self.pos += 1;
                let text = self.string()?;
                self.eat_sym(";")?;
                items.push(BoardItem::Note(NoteStmt {
                    text,
                    span: Span { line },
                }));
            } else {
                let mut tag = self.eat_ident()?;
                // Redundant leading type introducer (`main_switch MSB : …`).
                if !self.at_sym(":")
                    && !self.at_sym("=")
                    && matches!(self.peek().map(|t| &t.tok), Some(Tok::Ident(_)))
                {
                    tag = self.eat_ident()?;
                }
                if self.at_sym(":") {
                    self.pos += 1;
                    let type_ref = self.type_ref()?;
                    let props = if self.at_sym("{") {
                        self.property_block()?
                    } else {
                        Vec::<Property>::new()
                    };
                    if self.at_sym(";") {
                        self.pos += 1;
                    }
                    items.push(BoardItem::Node(NodeDecl {
                        tag,
                        type_ref,
                        props,
                        span: Span { line },
                    }));
                } else if self.at_sym("=") {
                    self.pos += 1;
                    let value = self.value()?;
                    self.eat_sym(";")?;
                    items.push(BoardItem::Property(Property {
                        name: tag,
                        value,
                        span: Span { line },
                    }));
                } else {
                    return Err(self.error(format!("expected `:` or `=` after `{tag}` in board")));
                }
            }
        }
        Ok(items)
    }

    fn circuit_items(&mut self) -> Result<Vec<CircuitItem>, ParseError> {
        self.eat_sym("{")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            let line = self.line();
            if self.at_ident("protection") || self.at_ident("controller") {
                let which = if self.at_ident("protection") {
                    "protection"
                } else {
                    "controller"
                };
                self.pos += 1;
                self.eat_sym(":")?;
                let type_ref = self.type_ref()?;
                let props = if self.at_sym("{") {
                    self.property_block()?
                } else {
                    Vec::<Property>::new()
                };
                self.eat_sym(";")?;
                let device = InlineDevice {
                    type_ref,
                    props,
                    span: Span { line },
                };
                items.push(if which == "protection" {
                    CircuitItem::Protection(device)
                } else {
                    CircuitItem::Controller(device)
                });
            } else if self.at_ident("connect") {
                self.pos += 1; // past the keyword; connect() starts at the first endpoint
                items.push(CircuitItem::Connect(self.connect(line)?));
            } else if self.at_ident("note") {
                self.pos += 1;
                let text = self.string()?;
                self.eat_sym(";")?;
                items.push(CircuitItem::Note(NoteStmt {
                    text,
                    span: Span { line },
                }));
            } else {
                let mut tag = self.eat_ident()?;
                // Redundant leading type introducer.
                if !self.at_sym(":")
                    && !self.at_sym("=")
                    && matches!(self.peek().map(|t| &t.tok), Some(Tok::Ident(_)))
                {
                    tag = self.eat_ident()?;
                }
                if self.at_sym(":") {
                    self.pos += 1;
                    let type_ref = self.type_ref()?;
                    let props = if self.at_sym("{") {
                        self.property_block()?
                    } else {
                        Vec::<Property>::new()
                    };
                    if self.at_sym(";") {
                        self.pos += 1;
                    }
                    items.push(CircuitItem::Node(NodeDecl {
                        tag,
                        type_ref,
                        props,
                        span: Span { line },
                    }));
                } else if self.at_sym("=") {
                    self.pos += 1;
                    let value = self.value()?;
                    self.eat_sym(";")?;
                    items.push(CircuitItem::Property(Property {
                        name: tag,
                        value,
                        span: Span { line },
                    }));
                } else {
                    return Err(self.error(format!("expected `:` or `=` after `{tag}` in circuit")));
                }
            }
        }
        Ok(items)
    }

    fn connect(&mut self, line: u32) -> Result<ConnectStmt, ParseError> {
        let mut endpoints = vec![self.endpoint_value()?];
        let mut arrows = Vec::new();
        loop {
            let arrow = if self.at_sym("->") {
                self.pos += 1;
                Arrow::Fwd
            } else if self.at_sym("<-") {
                self.pos += 1;
                Arrow::Rev
            } else if self.at_sym("--") {
                self.pos += 1;
                Arrow::Peer
            } else {
                break;
            };
            arrows.push(arrow);
            endpoints.push(self.endpoint_value()?);
        }
        if arrows.is_empty() {
            return Err(self.error("connect needs at least one arrow"));
        }
        let attrs = if self.at_sym("{") {
            self.property_block()?
        } else {
            Vec::<Property>::new()
        };
        self.eat_sym(";")?;
        Ok(ConnectStmt {
            endpoints,
            arrows,
            attrs,
            span: Span { line },
        })
    }

    /// An endpoint is a dotted reference with optional `[phases]`.
    fn endpoint_value(&mut self) -> Result<ValueNode, ParseError> {
        let line = self.line();
        let mut text = self.eat_ident()?;
        while self.at_sym(".") {
            self.pos += 1;
            let part = self.eat_ident()?;
            text.push('.');
            text.push_str(&part);
        }
        if self.at_sym("[") {
            self.pos += 1;
            let mut phases = Vec::new();
            if !self.at_sym("]") {
                loop {
                    phases.push(self.eat_ident()?);
                    if self.at_sym(",") {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
            }
            self.eat_sym("]")?;
            text.push('[');
            text.push_str(&phases.join(","));
            text.push(']');
        }
        Ok(ValueNode {
            value: Value::Ident(text),
            span: Span { line },
        })
    }

    fn scenario(&mut self, line: u32) -> Result<Statement, ParseError> {
        let name = self.string()?;
        self.eat_sym("{")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            let iline = self.line();
            let kind = self.eat_ident()?;
            let mut text_parts = Vec::new();
            // Collect tokens until `;` on the same statement.
            while !self.at_sym(";") {
                let Some(tok) = self.peek() else {
                    return Err(self.error("expected `;`"));
                };
                if matches!(tok.tok, Tok::Sym("}")) {
                    return Err(self.error("expected `;`"));
                }
                text_parts.push(self.next().text());
            }
            self.eat_sym(";")?;
            items.push(ScenarioItemRaw {
                kind,
                text: text_parts.join(" "),
                span: Span { line: iline },
            });
        }
        Ok(Statement::Scenario {
            name,
            items,
            span: Span { line },
        })
    }

    fn state(&mut self, line: u32) -> Result<Statement, ParseError> {
        let name = self.string()?;
        self.eat_sym("{")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            let iline = self.line();
            if self.at_ident("use") {
                self.pos += 1;
                let target = self.string()?;
                self.eat_sym(";")?;
                items.push(StateItem::Use(target, Span { line: iline }));
            } else if self.at_ident("note") {
                self.pos += 1;
                let text = self.string()?;
                self.eat_sym(";")?;
                items.push(StateItem::Note(NoteStmt {
                    text,
                    span: Span { line: iline },
                }));
            } else {
                // Position: `ref = ident ;`
                let mut target = self.eat_ident()?;
                while self.at_sym(".") {
                    self.pos += 1;
                    target.push('.');
                    target.push_str(&self.eat_ident()?);
                }
                self.eat_sym("=")?;
                let position = self.eat_ident()?;
                self.eat_sym(";")?;
                items.push(StateItem::Position {
                    target: ValueNode {
                        value: Value::Ident(target),
                        span: Span { line: iline },
                    },
                    position,
                    span: Span { line: iline },
                });
            }
        }
        Ok(Statement::State {
            name,
            items,
            span: Span { line },
        })
    }

    fn interlock(&mut self, line: u32) -> Result<Statement, ParseError> {
        let name = self.string()?;
        self.eat_sym("{")?;
        let mut requires = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            if self.at_ident("require") {
                self.pos += 1;
                let mut parts = Vec::new();
                while !self.at_sym(";") {
                    let Some(tok) = self.peek() else {
                        return Err(self.error("expected `;`"));
                    };
                    if matches!(tok.tok, Tok::Sym("}")) {
                        return Err(self.error("expected `;`"));
                    }
                    parts.push(self.next().text());
                }
                self.eat_sym(";")?;
                requires.push(parts.join(" "));
            } else if self.at_ident("note") {
                self.pos += 1;
                let _ = self.string()?;
                self.eat_sym(";")?;
            } else {
                return Err(self.error("expected `require` or `note` in interlock"));
            }
        }
        Ok(Statement::Interlock {
            name,
            requires,
            span: Span { line },
        })
    }

    fn layout_items(&mut self) -> Result<Vec<LayoutItem>, ParseError> {
        self.eat_sym("{")?;
        let mut items = Vec::new();
        loop {
            self.skip_trivia();
            if self.at_sym("}") {
                self.pos += 1;
                break;
            }
            let line = self.line();
            if self.at_ident("note") {
                self.pos += 1;
                let text = self.string()?;
                self.eat_sym(";")?;
                items.push(LayoutItem::Note(NoteStmt {
                    text,
                    span: Span { line },
                }));
            } else {
                let target = self.eat_ident()?;
                if self.at_sym("=") {
                    self.pos += 1;
                    let value = self.value()?;
                    self.eat_sym(";")?;
                    items.push(LayoutItem::Property(Property {
                        name: target,
                        value,
                        span: Span { line },
                    }));
                } else if self.at_sym("{") {
                    let props = self.property_block()?;
                    items.push(LayoutItem::Target {
                        target,
                        props,
                        span: Span { line },
                    });
                } else {
                    return Err(
                        self.error(format!("expected `=` or `{{` after `{target}` in layout"))
                    );
                }
            }
        }
        Ok(items)
    }

    /// Skips a `{ … }` block with nested braces.
    fn skip_braced(&mut self) -> Result<(), ParseError> {
        self.eat_sym("{")?;
        let mut depth = 1;
        while depth > 0 {
            let Some(tok) = self.peek() else {
                return Err(self.error("unterminated block"));
            };
            match &tok.tok {
                Tok::Sym("{") => depth += 1,
                Tok::Sym("}") => depth -= 1,
                _ => {}
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn skip_braced_after_string(&mut self) -> Result<(), ParseError> {
        let _ = self.string()?;
        self.skip_braced()
    }
}

/// Heuristic for whitespace-separated units: short alphabetic tokens that
/// look like units, not enum keywords or references.
fn is_unit_like(s: &str) -> bool {
    matches!(
        s,
        "V" | "A" | "W" | "Hz" | "kW" | "kV" | "kA" | "mA" | "mV" | "MVA" | "kVA" | "var"
    )
}
