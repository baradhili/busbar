//! ESLD semantic model: linking, type instantiation, circuit expansion,
//! and reference resolution (spec §7, §9.3).
//!
//! Phase 1 status: builds the IR from a parsed document, expands circuits
//! into protection/controller/load edges, materializes implicit bus
//! sections, and offers endpoint/voltage-system resolution for the
//! validator. JSON export and include resolution arrive with M2.

pub mod types;

use std::collections::BTreeMap;

use busbar_syntax::ast::{self, Property, Value, ValueNode};
pub use types::NodeKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStyle {
    Dc,
    Single,
    Split,
    Three,
    Multi(u32),
}

impl PhaseStyle {
    pub fn count(self) -> u32 {
        match self {
            PhaseStyle::Dc | PhaseStyle::Single => 1,
            PhaseStyle::Split => 2,
            PhaseStyle::Three => 3,
            PhaseStyle::Multi(n) => n,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Voltsys {
    pub name: String,
    pub nominal_v: f64,
    pub phases: PhaseStyle,
    pub frequency_hz: Option<f64>,
    pub neutral: Option<bool>,
    pub earthing: Option<String>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct CodeProfile {
    pub name: String,
    pub props: Vec<Property>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct IrNode {
    pub tag: String,
    pub type_name: String,
    pub kind: Option<NodeKind>,
    /// Containing board, if any.
    pub parent: Option<String>,
    pub props: Vec<Property>,
    pub line: u32,
}

impl IrNode {
    pub fn get(&self, name: &str) -> Option<&ValueNode> {
        self.props.iter().find(|p| p.name == name).map(|p| &p.value)
    }

    pub fn ident_prop(&self, name: &str) -> Option<&str> {
        self.get(name).and_then(|v| v.value.as_ident())
    }

    pub fn quantity_prop(&self, name: &str) -> Option<(f64, String)> {
        self.get(name).and_then(|v| v.value.quantity())
    }
}

#[derive(Debug, Clone)]
pub struct Section {
    /// Qualified tag (`BOARD.SEC`, or `BOARD.bus` for the implicit section).
    pub tag: String,
    pub board: String,
    pub incomers: Vec<String>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct Board {
    pub tag: String,
    pub sections: Vec<String>,
    pub incomers: Vec<String>,
    pub circuits: Vec<String>,
    pub line: u32,
    pub props: Vec<Property>,
}

#[derive(Debug, Clone)]
pub struct Circuit {
    /// Qualified tag (`BOARD.CIRCUIT`).
    pub tag: String,
    pub board: String,
    pub section: Option<String>,
    pub phase: Option<String>,
    pub protection: Option<IrNode>,
    pub controller: Option<IrNode>,
    /// Load tags with the line of the `loads` property that referenced them.
    pub loads: Vec<(String, u32)>,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct Edge {
    /// Endpoint texts as written (`MAIN.MSB.in`).
    pub from: String,
    pub to: String,
    pub arrow: ast::Arrow,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct StatePosition {
    pub target: String,
    pub position: String,
    pub line: u32,
}

#[derive(Debug, Default)]
pub struct Ir {
    pub voltsys: BTreeMap<String, Voltsys>,
    pub code_profiles: Vec<CodeProfile>,
    /// User-declared type names (R-101).
    pub user_types: Vec<String>,
    pub nodes: BTreeMap<String, IrNode>,
    pub boards: BTreeMap<String, Board>,
    pub sections: BTreeMap<String, Section>,
    pub circuits: BTreeMap<String, Circuit>,
    pub edges: Vec<Edge>,
    pub states: Vec<(String, Vec<StatePosition>)>,
    /// `layout { TAG { column = N; } }` hints (spec §16.1, advisory).
    pub layout_columns: BTreeMap<String, u64>,
}

impl Ir {
    pub fn build(doc: &ast::Document) -> Result<Ir, String> {
        let mut ir = Ir::default();
        ir.collect(doc)?;
        ir.expand();
        Ok(ir)
    }

    fn collect(&mut self, doc: &ast::Document) -> Result<(), String> {
        for stmt in &doc.statements {
            match stmt {
                ast::Statement::Voltsys { name, body, span } => {
                    let vs = voltsys_from(name, body, span.line)?;
                    self.voltsys.insert(name.clone(), vs);
                }
                ast::Statement::Code { name, props, span } => {
                    self.code_profiles.push(CodeProfile {
                        name: name.clone(),
                        props: props.clone(),
                        line: span.line,
                    });
                }
                ast::Statement::TypeDecl { name, .. } => {
                    self.user_types.push(name.clone());
                }
                ast::Statement::Node(node) => {
                    self.add_node(node, None)?;
                }
                ast::Statement::Board {
                    tag, items, span, ..
                } => {
                    self.add_board(tag, items, span.line)?;
                }
                ast::Statement::Connect(c) => {
                    push_chain(&mut self.edges, &c.endpoints, &c.arrows);
                }
                ast::Statement::Layout { items, .. } => {
                    for item in items {
                        if let ast::LayoutItem::Target { target, props, .. } = item {
                            if let Some(prop) = props.iter().find(|p| p.name == "column") {
                                if let Some((v, _)) = prop.value.value.quantity() {
                                    self.layout_columns.insert(target.clone(), v as u64);
                                }
                            }
                        }
                    }
                }
                ast::Statement::State { name, items, .. } => {
                    let positions = items
                        .iter()
                        .filter_map(|item| match item {
                            ast::StateItem::Position {
                                target,
                                position,
                                span,
                            } => Some(StatePosition {
                                target: ident_of(target),
                                position: position.clone(),
                                line: span.line,
                            }),
                            _ => None,
                        })
                        .collect();
                    self.states.push((name.clone(), positions));
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn add_node(&mut self, node: &ast::NodeDecl, parent: Option<&str>) -> Result<(), String> {
        let kind = types::lookup(&node.type_ref).map(|t| t.kind);
        self.nodes.insert(
            node.tag.clone(),
            IrNode {
                tag: node.tag.clone(),
                type_name: node.type_ref.clone(),
                kind,
                parent: parent.map(str::to_owned),
                props: node.props.clone(),
                line: node.span.line,
            },
        );
        Ok(())
    }

    fn add_board(&mut self, tag: &str, items: &[ast::BoardItem], line: u32) -> Result<(), String> {
        let mut board = Board {
            tag: tag.to_owned(),
            sections: Vec::new(),
            incomers: Vec::new(),
            circuits: Vec::new(),
            line,
            props: Vec::new(),
        };

        for item in items {
            match item {
                ast::BoardItem::Property(p) => {
                    if p.name == "incomers" {
                        board.incomers = ref_list(&p.value.value);
                    }
                    board.props.push(p.clone());
                }
                ast::BoardItem::Node(n) => self.add_node(n, Some(tag))?,
                ast::BoardItem::Bus(bus) => {
                    let qualified = format!("{tag}.{}", bus.tag);
                    let mut incomers = Vec::new();
                    for p in &bus.props {
                        if p.name == "incomers" {
                            incomers = ref_list(&p.value.value);
                        }
                    }
                    self.sections.insert(
                        qualified.clone(),
                        Section {
                            tag: qualified.clone(),
                            board: tag.to_owned(),
                            incomers,
                            line: bus.span.line,
                        },
                    );
                    board.sections.push(qualified);
                }
                ast::BoardItem::Circuit(c) => {
                    let qualified = format!("{tag}.{}", c.tag);
                    let mut circuit = Circuit {
                        tag: qualified.clone(),
                        board: tag.to_owned(),
                        section: None,
                        phase: None,
                        protection: None,
                        controller: None,
                        loads: Vec::new(),
                        line: c.span.line,
                    };
                    for ci in &c.items {
                        match ci {
                            ast::CircuitItem::Property(p) => match p.name.as_str() {
                                "phase" => {
                                    circuit.phase = p.value.value.as_ident().map(str::to_owned);
                                }
                                "bus" => {
                                    let Some(parts) = p.value.value.as_ref() else {
                                        continue;
                                    };
                                    let section_tag = parts.last().unwrap().clone();
                                    circuit.section = Some(format!("{tag}.{section_tag}"));
                                }
                                "loads" => {
                                    if let Value::List(entries) = &p.value.value {
                                        for entry in entries {
                                            if let Some(r) = entry.value.as_ref() {
                                                circuit.loads.push((r.join("."), entry.span.line));
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            },
                            ast::CircuitItem::Protection(d) => {
                                circuit.protection = Some(IrNode {
                                    tag: format!("{qualified}.protection"),
                                    type_name: d.type_ref.clone(),
                                    kind: types::lookup(&d.type_ref).map(|t| t.kind),
                                    parent: Some(tag.to_owned()),
                                    props: d.props.clone(),
                                    line: d.span.line,
                                });
                            }
                            ast::CircuitItem::Controller(d) => {
                                circuit.controller = Some(IrNode {
                                    tag: format!("{qualified}.controller"),
                                    type_name: d.type_ref.clone(),
                                    kind: types::lookup(&d.type_ref).map(|t| t.kind),
                                    parent: Some(tag.to_owned()),
                                    props: d.props.clone(),
                                    line: d.span.line,
                                });
                            }
                            ast::CircuitItem::Node(n) => self.add_node(n, Some(tag))?,
                            ast::CircuitItem::Connect(c) => {
                                push_chain(&mut self.edges, &c.endpoints, &c.arrows);
                            }
                            ast::CircuitItem::Note(_) => {}
                        }
                    }
                    self.circuits.insert(qualified.clone(), circuit);
                    board.circuits.push(qualified);
                }
                ast::BoardItem::Connect(c) => {
                    push_chain(&mut self.edges, &c.endpoints, &c.arrows);
                }
                ast::BoardItem::Note(_) => {}
            }
        }

        // Materialize the implicit single section (spec §9.1).
        if board.sections.is_empty() {
            let qualified = format!("{tag}.bus");
            self.sections.insert(
                qualified.clone(),
                Section {
                    tag: qualified.clone(),
                    board: tag.to_owned(),
                    incomers: board.incomers.clone(),
                    line: board.line,
                },
            );
            board.sections.push(qualified);
        }

        // Boards are also nodes (containers), carrying the board's props
        // so `vs` inheritance and port checks see them.
        self.nodes.insert(
            tag.to_owned(),
            IrNode {
                tag: tag.to_owned(),
                type_name: "board".to_owned(),
                kind: Some(NodeKind::Container),
                parent: None,
                props: board.props.clone(),
                line,
            },
        );
        self.boards.insert(tag.to_owned(), board);
        Ok(())
    }

    /// Circuit expansion (spec §9.3): protection (and controller) attach to
    /// the section busbar and feed the loads.
    fn expand(&mut self) {
        let circuit_tags: Vec<String> = self.circuits.keys().cloned().collect();
        for tag in circuit_tags {
            let Some(circuit) = self.circuits.get(&tag) else {
                continue;
            };
            // Inline protection/controller devices are real nodes once
            // expanded — endpoints like `BOARD.CIRCUIT.protection.in`
            // resolve through them.
            if let Some(prot) = &circuit.protection {
                self.nodes.insert(prot.tag.clone(), prot.clone());
            }
            if let Some(ctl) = &circuit.controller {
                self.nodes.insert(ctl.tag.clone(), ctl.clone());
            }
            // The busbar feeds protection if present, else the controller
            // (controller-only circuits still produce connectivity);
            // circuits with neither have no expansion at all.
            let head = match (&circuit.protection, &circuit.controller) {
                (Some(p), _) => p.tag.clone(),
                (None, Some(c)) => c.tag.clone(),
                (None, None) => continue,
            };
            let section = circuit.section.clone().or_else(|| {
                self.boards
                    .get(&circuit.board)
                    .and_then(|b| b.sections.first().cloned())
            });
            let Some(section) = section else { continue };
            self.edges.push(Edge {
                from: section,
                to: format!("{head}.in"),
                arrow: ast::Arrow::Fwd,
                line: circuit.line,
            });
            let supply = circuit
                .controller
                .as_ref()
                .map(|c| c.tag.clone())
                .unwrap_or(head);
            if let (Some(prot), Some(ctl)) = (&circuit.protection, &circuit.controller) {
                self.edges.push(Edge {
                    from: format!("{}.out", prot.tag),
                    to: format!("{}.in", ctl.tag),
                    arrow: ast::Arrow::Fwd,
                    line: circuit.line,
                });
            }
            for (load, line) in &circuit.loads {
                self.edges.push(Edge {
                    from: format!("{supply}.out"),
                    to: format!("{load}.in"),
                    arrow: ast::Arrow::Fwd,
                    line: *line,
                });
            }
        }
    }

    // -- Resolution helpers -------------------------------------------------

    pub fn lookup(&self, tag: &str) -> Option<&IrNode> {
        self.nodes.get(tag)
    }

    /// Resolves an endpoint text to (owning entity tag, optional port,
    /// owning board tag of the entity). Descends qualified names greedily
    /// (`MAIN.FEED_A.protection.in` walks board -> circuit -> inline
    /// device -> port), so synthetic expansion tags resolve too.
    pub fn resolve_endpoint(&self, text: &str) -> Option<(String, Option<String>, Option<String>)> {
        let parts: Vec<&str> = text.split('.').collect();
        let mut current = parts[0].to_owned();
        if self.nodes.get(&current)?.kind != Some(NodeKind::Container) {
            // Non-container node: everything after the tag is a port. The
            // owning board is the node's parent, so board-local references
            // (`MSB.out` inside MAIN) still count as internal.
            let port = parts.get(1).map(|p| p.to_string());
            let owner = self.nodes.get(&current).and_then(|n| n.parent.clone());
            return Some((current, port, owner));
        }
        let mut idx = 1;
        while idx < parts.len() {
            let next = parts[idx];
            let qualified = format!("{current}.{next}");
            let descend = self.sections.contains_key(&qualified)
                || self.circuits.contains_key(&qualified)
                || self.nodes.contains_key(&qualified)
                || self
                    .nodes
                    .get(next)
                    .map(|n| {
                        n.parent.as_deref() == Some(current.as_str())
                            && self.boards.contains_key(&current)
                    })
                    .unwrap_or(false);
            if descend {
                current = if self.sections.contains_key(&qualified)
                    || self.circuits.contains_key(&qualified)
                    || self.nodes.contains_key(&qualified)
                {
                    qualified
                } else {
                    next.to_owned()
                };
                idx += 1;
            } else {
                break;
            }
        }
        let port = if idx < parts.len() {
            Some(parts[idx].to_owned())
        } else {
            None
        };
        let owner = if self.sections.contains_key(&current) {
            self.sections.get(&current).map(|s| s.board.clone())
        } else if self.circuits.contains_key(&current) {
            self.circuits.get(&current).map(|c| c.board.clone())
        } else {
            self.nodes
                .get(&current)
                .and_then(|n| n.parent.clone())
                .or_else(|| self.boards.contains_key(&current).then(|| current.clone()))
        };
        Some((current, port, owner))
    }

    /// Effective voltage system name for an endpoint (converter ports map
    /// to their side's system, spec §11.4).
    pub fn endpoint_vs(&self, text: &str) -> Option<String> {
        let (entity, port, _) = self.resolve_endpoint(text)?;
        // Converter side mapping (spec §11.4), then the general
        // node/section/circuit resolution.
        if let Some(node) = self.nodes.get(&entity) {
            let is_converter =
                types::lookup(&node.type_name).map(|t| t.kind) == Some(NodeKind::Converter);
            if is_converter {
                match node.type_name.as_str() {
                    "transformer" => {
                        return match port.as_deref() {
                            Some("primary") | Some("n") => {
                                node.ident_prop("vs_in").map(str::to_owned)
                            }
                            Some("secondary") => node.ident_prop("vs_out").map(str::to_owned),
                            _ => None,
                        };
                    }
                    "inverter" => {
                        return match port.as_deref() {
                            Some("ac_in") | Some("ac_out") | Some("backup_out") => {
                                node.ident_prop("vs").map(str::to_owned)
                            }
                            _ => None,
                        };
                    }
                    _ => {}
                }
            }
        }
        self.node_vs(&entity)
    }

    /// The voltage system governing a node: its `vs`, else its parent
    /// board's (spec §9.1 inheritance).
    pub fn node_vs(&self, tag: &str) -> Option<String> {
        if let Some(section) = self.sections.get(tag) {
            let board = section.board.clone();
            return self.board_vs(&board);
        }
        if let Some(circuit) = self.circuits.get(tag) {
            let board = circuit.board.clone();
            return self.board_vs(&board);
        }
        let node = self.nodes.get(tag)?;
        if let Some(vs) = node.ident_prop("vs") {
            return Some(vs.to_owned());
        }
        if let Some(parent) = &node.parent {
            return self.board_vs(parent);
        }
        // A load fed by a circuit inherits that board's system.
        for circuit in self.circuits.values() {
            if circuit.loads.iter().any(|(l, _)| l == tag) {
                return self.board_vs(&circuit.board);
            }
        }
        None
    }

    pub fn board_vs(&self, board: &str) -> Option<String> {
        self.boards.get(board).and_then(|b| {
            b.props.iter().find(|p| p.name == "vs").and_then(|p| {
                if let Value::Ident(vs) = &p.value.value {
                    Some(vs.clone())
                } else {
                    None
                }
            })
        })
    }

    /// The board's declared `vs` property, if any.
    pub fn board_vs_prop(&self, board: &Board) -> Option<String> {
        board
            .props
            .iter()
            .find(|p| p.name == "vs")
            .and_then(|p| p.value.value.as_ident().map(str::to_owned))
    }

    pub fn voltsys_of_node(&self, tag: &str) -> Option<&Voltsys> {
        self.node_vs(tag).and_then(|name| self.voltsys.get(&name))
    }
}

fn push_chain(edges: &mut Vec<Edge>, endpoints: &[ValueNode], arrows: &[ast::Arrow]) {
    for i in 0..arrows.len() {
        let (from, to, arrow) = match arrows[i] {
            ast::Arrow::Fwd => (&endpoints[i], &endpoints[i + 1], ast::Arrow::Fwd),
            ast::Arrow::Rev => (&endpoints[i + 1], &endpoints[i], ast::Arrow::Fwd),
            ast::Arrow::Peer => (&endpoints[i], &endpoints[i + 1], ast::Arrow::Peer),
        };
        edges.push(Edge {
            from: ident_of(from),
            to: ident_of(to),
            arrow,
            line: from.span.line,
        });
    }
}

fn ident_of(v: &ValueNode) -> String {
    match &v.value {
        Value::Ident(s) => s.clone(),
        _ => String::new(),
    }
}

fn ref_list(value: &Value) -> Vec<String> {
    match value {
        Value::List(items) => items
            .iter()
            .filter_map(|i| i.value.as_ident().map(str::to_owned))
            .collect(),
        Value::Ident(s) => vec![s.clone()],
        _ => Vec::new(),
    }
}

fn voltsys_from(name: &str, body: &ast::VoltsysBody, line: u32) -> Result<Voltsys, String> {
    let mut nominal_v = 0.0;
    let mut phases = PhaseStyle::Single;
    let mut frequency_hz = None;
    let mut neutral = None;
    let mut earthing = None;

    let mut set_phase = |v: &Value| -> Result<(), String> {
        phases = match v {
            Value::Ident(s) => match s.as_str() {
                "dc" => PhaseStyle::Dc,
                "split" => PhaseStyle::Split,
                "1ph" => PhaseStyle::Single,
                "3ph" => PhaseStyle::Three,
                other => {
                    let count = other.trim_end_matches("ph");
                    let n: u32 = count
                        .parse()
                        .map_err(|_| format!("voltsys {name}: bad phases {other:?}"))?;
                    PhaseStyle::Multi(n)
                }
            },
            Value::Quantity { number, unit } if unit == "ph" => {
                let n: u32 = number
                    .parse()
                    .map_err(|_| format!("voltsys {name}: bad phase count {number:?}"))?;
                PhaseStyle::Multi(n)
            }
            _ => return Err(format!("voltsys {name}: bad phases value")),
        };
        Ok(())
    };

    match body {
        ast::VoltsysBody::Positional(a, b, c) => {
            nominal_v = a
                .value
                .quantity()
                .ok_or_else(|| format!("voltsys {name}: nominal must be a quantity"))?
                .0;
            set_phase(&b.value)?;
            frequency_hz = Some(
                c.value
                    .quantity()
                    .ok_or_else(|| format!("voltsys {name}: frequency must be a quantity"))?
                    .0,
            );
        }
        ast::VoltsysBody::Block(props) => {
            for p in props {
                match p.name.as_str() {
                    "nominal" => {
                        nominal_v = p
                            .value
                            .value
                            .quantity()
                            .ok_or_else(|| format!("voltsys {name}: nominal must be a quantity"))?
                            .0
                    }
                    "phases" => set_phase(&p.value.value)?,
                    "frequency" => {
                        frequency_hz = Some(
                            p.value
                                .value
                                .quantity()
                                .ok_or_else(|| {
                                    format!("voltsys {name}: frequency must be a quantity")
                                })?
                                .0,
                        )
                    }
                    "neutral" => {
                        neutral = match &p.value.value {
                            Value::Bool(b) => Some(*b),
                            _ => None,
                        }
                    }
                    "earthing" => {
                        earthing = match &p.value.value {
                            Value::Str(s) => Some(s.clone()),
                            Value::Ident(s) => Some(s.clone()),
                            _ => None,
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(Voltsys {
        name: name.to_owned(),
        nominal_v,
        phases,
        frequency_hz,
        neutral,
        earthing,
        line,
    })
}
