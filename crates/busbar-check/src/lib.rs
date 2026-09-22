//! Validation rule engine: R-1xx..R-2xx for Phase 1 (spec §15).
//!
//! Diagnostics are values — code, severity, line — never panics.
//! R-3xx/R-4xx land with M4; R-5xx/R-6xx with M6.

use std::collections::BTreeMap;
use std::path::Path;

use busbar_ir::Ir;
use busbar_ir::types::{NodeKind, PortDir};
use busbar_syntax::ast::{self, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub message: String,
    pub line: u32,
    /// 1-based column of the construct the diagnostic points at.
    pub col: u32,
    /// Supplementary detail lines rendered under the diagnostic
    /// (implementation plan §4.4 `notes`).
    pub notes: Vec<String>,
}

/// Checks an ESLD document. `base_dir` (the document's own directory) is
/// used for include resolution (R-112).
pub fn check(source: &str, base_dir: Option<&Path>) -> Result<Vec<Diagnostic>, String> {
    // Syntax failures are diagnostics like any other (E-LEX-1/E-PARSE-1,
    // implementation plan §4.4) so one bad file doesn't abort a run.
    if let Err(busbar_syntax::LexError { message, line, col }) = busbar_syntax::lex(source) {
        return Ok(vec![Diagnostic {
            code: "E-LEX-1".to_owned(),
            severity: Severity::Error,
            message,
            line,
            col,
            notes: Vec::new(),
        }]);
    }
    let doc = match busbar_syntax::parse(source) {
        Ok(doc) => doc,
        Err(busbar_syntax::ParseError { message, line, col }) => {
            return Ok(vec![Diagnostic {
                code: "E-PARSE-1".to_owned(),
                severity: Severity::Error,
                message,
                line,
                col,
                notes: Vec::new(),
            }]);
        }
    };
    let ir = match Ir::build(&doc) {
        Ok(ir) => ir,
        Err(busbar_ir::IrBuildError { message, line, col }) => {
            return Ok(vec![Diagnostic {
                code: "E-IR-1".to_owned(),
                severity: Severity::Error,
                message,
                line,
                col,
                notes: Vec::new(),
            }]);
        }
    };
    let mut ctx = Ctx {
        ir: &ir,
        doc: &doc,
        base_dir,
        diags: Vec::new(),
    };
    ctx.r101_unknown_type();
    ctx.r102_duplicate_tags();
    ctx.r112_includes();
    ctx.r111_nonexistent_ports();
    ctx.r103_required_ports();
    ctx.r104_port_arity();
    ctx.r106_r109_board_feeds();
    ctx.r107_no_circuits();
    ctx.r108_circular_supply();
    ctx.r110_unlisted_feeds();
    ctx.r113_multi_section();
    ctx.r114_state_targets();
    ctx.r105_unreachable();
    ctx.r201_r202_voltage_frequency();
    ctx.r203_phase_missing();
    ctx.r204_phase_imbalance();
    ctx.r205_multiphase_load();
    ctx.r206_neutral();
    ctx.r207_earthing_tie();
    ctx.r208_parallel_transformers();
    ctx.r209_r210_load_profiles();
    ctx.diags
        .sort_by(|a, b| (&a.code, a.line, a.col).cmp(&(&b.code, b.line, b.col)));
    Ok(ctx.diags)
}

struct Ctx<'a> {
    ir: &'a Ir,
    doc: &'a ast::Document,
    #[allow(dead_code)]
    base_dir: Option<&'a Path>,
    diags: Vec<Diagnostic>,
}

impl Ctx<'_> {
    fn error(&mut self, code: &str, line: u32, col: u32, message: String) {
        self.diags.push(Diagnostic {
            code: code.to_owned(),
            severity: Severity::Error,
            message,
            line,
            col,
            notes: Vec::new(),
        });
    }

    fn warn(&mut self, code: &str, line: u32, col: u32, message: String) {
        self.diags.push(Diagnostic {
            code: code.to_owned(),
            severity: Severity::Warning,
            message,
            line,
            col,
            notes: Vec::new(),
        });
    }

    /// Every declared tag in document order: (tag, line).
    fn declared_tags(&self) -> Vec<(String, u32, u32)> {
        let mut tags = Vec::new();
        for stmt in &self.doc.statements {
            match stmt {
                ast::Statement::Node(n) => tags.push((n.tag.clone(), n.span.line, n.span.col)),
                ast::Statement::Board {
                    tag, items, span, ..
                } => {
                    tags.push((tag.clone(), span.line, span.col));
                    for item in items {
                        match item {
                            ast::BoardItem::Node(n) => {
                                tags.push((n.tag.clone(), n.span.line, n.span.col))
                            }
                            ast::BoardItem::Circuit(c) => {
                                tags.push((format!("{tag}.{}", c.tag), c.span.line, c.span.col))
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        tags
    }

    fn r101_unknown_type(&mut self) {
        for node in self.ir.nodes.values() {
            let known = busbar_ir::types::lookup(&node.type_name).is_some()
                || self.ir.user_types.contains(&node.type_name)
                || node.type_name.contains('.')
                || node.type_name == "board";
            if !known {
                self.error(
                    "R-101",
                    node.line,
                    node.col,
                    format!(
                        "unknown type `{}` referenced by `{}`",
                        node.type_name, node.tag
                    ),
                );
            }
        }
    }

    fn r102_duplicate_tags(&mut self) {
        let mut seen: BTreeMap<String, u32> = BTreeMap::new();
        for (tag, line, col) in self.declared_tags() {
            if let Some(first) = seen.get(&tag) {
                self.error(
                    "R-102",
                    line,
                    col,
                    format!("duplicate tag `{tag}` (first declared at line {first})"),
                );
            } else {
                seen.insert(tag, line);
            }
        }
    }

    fn r112_includes(&mut self) {
        for stmt in &self.doc.statements {
            if let ast::Statement::Include { path, span } = stmt {
                let ok = self
                    .base_dir
                    .map(|d| d.join(path).is_file())
                    .unwrap_or(false);
                if !ok {
                    self.error(
                        "R-112",
                        span.line,
                        span.col,
                        format!(
                            "include `{path}` not found (loading and cycle detection land with M2)"
                        ),
                    );
                }
            }
        }
    }

    /// Edge lines whose endpoint ports failed existence — R-110 skips
    /// these so the two rules never double-report.
    fn invalid_port_edges(&self) -> Vec<(u32, u32)> {
        let mut bad = Vec::new();
        for edge in &self.ir.edges {
            for text in [&edge.from, &edge.to] {
                if let Some((entity, Some(port), _)) = self.ir.resolve_endpoint(text) {
                    if !self.port_exists(&entity, &port) {
                        bad.push((edge.line, edge.col));
                    }
                }
            }
        }
        bad
    }

    fn port_exists(&self, entity: &str, port: &str) -> bool {
        if let Some(board) = self.ir.boards.get(entity) {
            return matches!(port, "in" | "out" | "n" | "pe" | "bus")
                || board
                    .sections
                    .iter()
                    .any(|s| s.ends_with(&format!(".{port}")));
        }
        if self.ir.sections.contains_key(entity) {
            return true; // bus attachment point
        }
        if self.ir.circuits.contains_key(entity) {
            return matches!(port, "in" | "out");
        }
        if let Some(node) = self.ir.nodes.get(entity) {
            if let Some(def) = busbar_ir::types::lookup(&node.type_name) {
                return def.port(port).is_some();
            }
            return true; // unknown type: R-101 already fired
        }
        false
    }

    fn r111_nonexistent_ports(&mut self) {
        for edge in &self.ir.edges {
            for text in [&edge.from, &edge.to] {
                if let Some((entity, Some(port), _)) = self.ir.resolve_endpoint(text) {
                    if !self.port_exists(&entity, &port) {
                        self.error(
                            "R-111",
                            edge.line,
                            edge.col,
                            format!("`{text}`: port `{port}` does not exist on `{entity}`"),
                        );
                    }
                }
            }
        }
    }

    fn r103_required_ports(&mut self) {
        // Connected (entity, port) pairs from edges; a bare endpoint
        // satisfies every port of its entity (tie devices, section refs).
        let mut connected: BTreeMap<(String, String), ()> = BTreeMap::new();
        let mut wholly: BTreeMap<String, ()> = BTreeMap::new();
        for edge in &self.ir.edges {
            for text in [&edge.from, &edge.to] {
                let Some((entity, port, _)) = self.ir.resolve_endpoint(text) else {
                    continue;
                };
                match port {
                    Some(p) => {
                        connected.insert((entity, p), ());
                    }
                    None => {
                        wholly.insert(entity, ());
                    }
                }
            }
        }
        // Implicit busbar attach (spec §9.2): a switching/protective/
        // measurement device declared in a single-section board with no
        // explicit edges attaches at its `in` port.
        for node in self.ir.nodes.values() {
            let attachable = matches!(
                node.kind,
                Some(NodeKind::Switch) | Some(NodeKind::Protective) | Some(NodeKind::Measurement)
            );
            let Some(parent) = &node.parent else { continue };
            let Some(board) = self.ir.boards.get(parent) else {
                continue;
            };
            if attachable && board.sections.len() == 1 && !wholly.contains_key(&node.tag) {
                connected.insert((node.tag.clone(), "in".to_owned()), ());
            }
        }

        for node in self.ir.nodes.values() {
            let Some(def) = busbar_ir::types::lookup(&node.type_name) else {
                continue;
            };
            for pd in def.ports {
                if pd.optional || pd.dir == PortDir::None {
                    continue;
                }
                // Inline circuit devices: `out` is the circuit's supply
                // point and may legitimately dangle (feeds, R-109).
                let inline = node.tag.ends_with(".protection") || node.tag.ends_with(".controller");
                if inline && pd.dir == PortDir::Out {
                    continue;
                }
                if wholly.contains_key(&node.tag)
                    || connected.contains_key(&(node.tag.clone(), pd.name.to_owned()))
                {
                    continue;
                }
                self.error(
                    "R-103",
                    node.line,
                    node.col,
                    format!(
                        "required port `{}` of `{}` ({}) is unconnected",
                        pd.name, node.tag, node.type_name
                    ),
                );
                break;
            }
        }
    }

    fn r104_port_arity(&mut self) {
        let mut incoming: BTreeMap<(String, String), Vec<(u32, u32)>> = BTreeMap::new();
        for edge in &self.ir.edges {
            let Some((entity, Some(port), _)) = self.ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            if self.ir.sections.contains_key(&entity) {
                continue; // buses legitimately accept multiple feeds
            }
            let Some(node) = self.ir.nodes.get(&entity) else {
                continue;
            };
            if node.kind == Some(NodeKind::Container) {
                continue;
            }
            let Some(def) = busbar_ir::types::lookup(&node.type_name) else {
                continue;
            };
            let Some(pd) = def.port(&port) else { continue };
            if pd.dir != PortDir::In || pd.multi_in {
                continue;
            }
            incoming
                .entry((entity, port))
                .or_default()
                .push((edge.line, edge.col));
        }
        for ((entity, port), lines) in incoming {
            if lines.len() > 1 {
                let (line, col) = *lines.iter().max().unwrap();
                self.error(
                    "R-104",
                    line,
                    col,
                    format!("port `{entity}.{port}` is fed by {} sources", lines.len()),
                );
            }
        }
    }

    fn r106_r109_board_feeds(&mut self) {
        // Incoming external feed edges per board.
        let mut fed: BTreeMap<String, u32> = BTreeMap::new();
        for edge in &self.ir.edges {
            let Some((_, _, Some(tb))) = self.ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            let sb = self.ir.resolve_endpoint(&edge.from).and_then(|(_, _, b)| b);
            if sb.as_deref() != Some(tb.as_str()) {
                fed.insert(tb, edge.line);
            }
        }
        for board in self.ir.boards.values() {
            let authorized: Vec<&String> = board
                .incomers
                .iter()
                .chain(
                    board
                        .sections
                        .iter()
                        .filter_map(|s| self.ir.sections.get(s))
                        .flat_map(|s| s.incomers.iter()),
                )
                .collect();
            let is_fed = fed.contains_key(&board.tag);
            if authorized.is_empty() && !is_fed {
                self.error(
                    "R-106",
                    board.line,
                    board.col,
                    format!("board `{}` has no incomer and no feed", board.tag),
                );
            } else if !authorized.is_empty() && !is_fed {
                self.error(
                    "R-109",
                    board.line,
                    board.col,
                    format!(
                        "board `{}` is declared with incomers but never fed",
                        board.tag
                    ),
                );
            }
        }
    }

    fn r107_no_circuits(&mut self) {
        for board in self.ir.boards.values() {
            if board.circuits.is_empty() {
                self.warn(
                    "R-107",
                    board.line,
                    board.col,
                    format!("board `{}` has no outgoing circuits", board.tag),
                );
            }
        }
    }

    fn r108_circular_supply(&mut self) {
        // Board-to-board arcs from cross-board feed edges.
        let mut arcs: Vec<(String, String, u32, u32)> = Vec::new();
        for edge in &self.ir.edges {
            let Some((_, _, Some(tb))) = self.ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            let Some(sb) = self.ir.resolve_endpoint(&edge.from).and_then(|(_, _, b)| b) else {
                continue;
            };
            if sb != tb {
                arcs.push((sb, tb, edge.line, edge.col));
            }
        }
        let cyclic: Vec<&str> = self
            .ir
            .boards
            .keys()
            .filter(|b| reaches(&arcs, b, b, &mut Vec::new()))
            .map(String::as_str)
            .collect();
        if cyclic.is_empty() {
            return;
        }
        // Fire only when nothing in the cycle is source-reachable.
        let reach = self.reachable_from_sources();
        let any_alive = cyclic.iter().any(|b| {
            reach.iter().any(|r| r == b)
                || self
                    .ir
                    .nodes
                    .values()
                    .any(|n| n.parent.as_deref() == Some(b) && reach.contains(&n.tag))
        });
        if any_alive {
            return;
        }
        if let Some((line, col)) = arcs
            .iter()
            .filter(|(_, tb, _, _)| cyclic.contains(&tb.as_str()))
            .map(|(_, _, l, c)| (*l, *c))
            .max()
        {
            self.error(
                "R-108",
                line,
                col,
                "circular supply path with no source".to_owned(),
            );
        }
    }

    fn r110_unlisted_feeds(&mut self) {
        let invalid = self.invalid_port_edges();
        for edge in &self.ir.edges {
            if invalid.contains(&(edge.line, edge.col))
                || edge.arrow == busbar_syntax::ast::Arrow::Peer
            {
                continue; // R-111 already reported / ties are self-authorized (§9.4)
            }
            let Some((to_entity, _, Some(tb))) = self.ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            let sb = self.ir.resolve_endpoint(&edge.from).and_then(|(_, _, b)| b);
            if sb.as_deref() == Some(tb.as_str()) {
                continue; // internal to the board
            }
            // A feed terminating on a circuit port enters through that
            // circuit's own protection — the circuit IS the declaration
            // (PV-backfeed style, spec §18.3).
            if self.ir.circuits.contains_key(&to_entity) {
                continue;
            }
            let Some(board) = self.ir.boards.get(tb.as_str()) else {
                continue;
            };
            let authorized = board
                .incomers
                .iter()
                .chain(
                    board
                        .sections
                        .iter()
                        .filter_map(|s| self.ir.sections.get(s))
                        .flat_map(|s| s.incomers.iter()),
                )
                .any(|a| *a == edge.from);
            if !authorized {
                self.error(
                    "R-110",
                    edge.line,
                    edge.col,
                    format!(
                        "feed `{}` into board `{}` is not listed in its incomers",
                        edge.from, tb
                    ),
                );
                if let Some(d) = self.diags.last_mut() {
                    d.notes.push(format!(
                        "add `{}` to the `incomers` of `{}`, or land the feed on a circuit port",
                        edge.from, tb
                    ));
                }
            }
        }
    }

    fn r113_multi_section(&mut self) {
        for circuit in self.ir.circuits.values() {
            let Some(board) = self.ir.boards.get(&circuit.board) else {
                continue;
            };
            if board.sections.len() > 1 && circuit.section.is_none() {
                self.error(
                    "R-113",
                    circuit.line,
                    circuit.col,
                    format!(
                        "circuit `{}` does not select a `bus` on multi-section board `{}`",
                        circuit.tag, board.tag
                    ),
                );
            }
        }
        // Spec §9.2: in a multi-section board, a device that requires
        // busbar power (a non-optional In port) and has no explicit
        // connections is an implicit-attach error. Measurement and relay
        // devices are exempt — their ports are optional because they
        // associate via signal links, not busbar power.
        for node in self.ir.nodes.values() {
            let Some(parent) = &node.parent else {
                continue;
            };
            let Some(board) = self.ir.boards.get(parent) else {
                continue;
            };
            if board.sections.len() <= 1 {
                continue;
            }
            if node.tag.ends_with(".protection") || node.tag.ends_with(".controller") {
                continue; // inline devices attach via their circuit's bus
            }
            let Some(def) = busbar_ir::types::lookup(&node.type_name) else {
                continue; // unknown types are R-101's business
            };
            let needs_power = def
                .ports
                .iter()
                .any(|p| !p.optional && p.dir == PortDir::In);
            if !needs_power {
                continue;
            }
            let wired = self.has_explicit_power_edge(&node.tag);
            if !wired {
                self.error(
                    "R-113",
                    node.line,
                    node.col,
                    format!(
                        "device `{}` ({}) requires busbar power but has no connections on multi-section board `{}` — declare a `bus` or wire it",
                        node.tag, node.type_name, parent
                    ),
                );
            }
        }
    }

    fn r114_state_targets(&mut self) {
        for (_, positions) in &self.ir.states {
            for pos in positions {
                // Resolve the full dotted target: board-qualified device
                // positions (`MSB.CB_TIE = open`) name the device, not the
                // board.
                let Some((entity, _, _)) = self.ir.resolve_endpoint(&pos.target) else {
                    continue;
                };
                let Some(node) = self.ir.nodes.get(&entity) else {
                    continue;
                };
                if node.kind != Some(NodeKind::Switch) {
                    self.error(
                        "R-114",
                        pos.line,
                        pos.col,
                        format!(
                            "state position targets `{}` ({}), which is not a switching device",
                            node.tag, node.type_name
                        ),
                    );
                }
            }
        }
    }

    fn reachable_from_sources(&self) -> Vec<String> {
        // Union-find over entities; conduction ignores switch positions in
        // Phase 1 (default positions apply, spec §13.1).
        let mut parent: BTreeMap<String, String> = BTreeMap::new();
        let vertices: Vec<String> = self
            .ir
            .nodes
            .keys()
            .chain(self.ir.sections.keys())
            .chain(self.ir.circuits.keys())
            .cloned()
            .collect();
        for v in &vertices {
            parent.insert(v.clone(), v.clone());
        }
        let find = |tag: &str, parent: &BTreeMap<String, String>| -> String {
            let mut t = tag.to_owned();
            while let Some(p) = parent.get(&t) {
                if p == &t {
                    break;
                }
                t = p.clone();
            }
            t
        };
        let union = |a: &str, b: &str, parent: &mut BTreeMap<String, String>| {
            if parent.contains_key(a) && parent.contains_key(b) {
                let ra = find(a, parent);
                let rb = find(b, parent);
                if ra != rb {
                    parent.insert(ra, rb);
                }
            }
        };
        for edge in &self.ir.edges {
            let Some((a, _, _)) = self.ir.resolve_endpoint(&edge.from) else {
                continue;
            };
            let Some((b, _, _)) = self.ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            union(&a, &b, &mut parent);
        }
        // Structural connections that explicit edges don't spell out:
        // boards to their sections, circuits to their protection chain,
        // and implicitly busbar-attached devices to their sole section
        // (spec §9.2).
        for board in self.ir.boards.values() {
            for section in &board.sections {
                union(&board.tag, section, &mut parent);
            }
        }
        for circuit in self.ir.circuits.values() {
            if let Some(prot) = &circuit.protection {
                union(&circuit.tag, &prot.tag, &mut parent);
            }
            if let Some(ctl) = &circuit.controller {
                union(&circuit.tag, &ctl.tag, &mut parent);
            }
        }
        for node in self.ir.nodes.values() {
            let attachable = matches!(
                node.kind,
                Some(NodeKind::Switch) | Some(NodeKind::Protective) | Some(NodeKind::Measurement)
            );
            let Some(parent_board) = &node.parent else {
                continue;
            };
            let Some(board) = self.ir.boards.get(parent_board) else {
                continue;
            };
            let explicitly_wired = self.has_explicit_power_edge(&node.tag);
            if attachable && board.sections.len() == 1 && !explicitly_wired {
                if let Some(section) = board.sections.first() {
                    union(&node.tag, section, &mut parent);
                }
            }
        }
        let live: Vec<String> = self
            .ir
            .nodes
            .values()
            .filter(|n| n.kind == Some(NodeKind::Source))
            .map(|n| find(&n.tag, &parent))
            .collect();
        vertices
            .into_iter()
            .filter(|v| live.contains(&find(v, &parent)))
            .collect()
    }

    /// True when any explicit edge terminates on `tag` via a power port.
    /// Earth/PE/neutral wiring is not a power-path connection (spec §6.1
    /// role inference) and does not defeat implicit busbar attachment
    /// (spec §9.2) — an SPD wired only through its PE terminal still
    /// attaches to the busbar at `in`.
    fn has_explicit_power_edge(&self, tag: &str) -> bool {
        const NON_POWER_PORTS: [&str; 3] = ["e", "pe", "n"];
        self.ir.edges.iter().any(|e| {
            [&e.from, &e.to].iter().any(|t| {
                self.ir
                    .resolve_endpoint(t)
                    .is_some_and(|(entity, port, _)| {
                        entity == tag
                            && !port
                                .as_deref()
                                .is_some_and(|p| NON_POWER_PORTS.contains(&p))
                    })
            })
        })
    }

    fn r105_unreachable(&mut self) {
        let reach = self.reachable_from_sources();
        for node in self.ir.nodes.values() {
            if node.kind == Some(NodeKind::Source) {
                continue;
            }
            // Containers (boards) are groupings, not powered equipment —
            // their members are checked individually. A board whose sole
            // incomer device transits straight through (GRID -> QF1 ->
            // sub-board) never joins the union itself, by design.
            if node.kind == Some(NodeKind::Container) {
                continue;
            }
            if !reach.contains(&node.tag) {
                self.warn(
                    "R-105",
                    node.line,
                    node.col,
                    format!("`{}` is unreachable from any source", node.tag),
                );
            }
        }
    }

    /// Duration unit -> seconds (spec §8.8 intermittent load profiles).
    fn duration_seconds(value: &busbar_syntax::ast::Value) -> Option<f64> {
        let busbar_syntax::ast::Value::Quantity { number, unit } = value else {
            return None;
        };
        let per: f64 = match unit.as_str() {
            "s" => 1.0,
            "min" => 60.0,
            "h" => 3600.0,
            "d" => 86400.0,
            _ => return None,
        };
        number.parse::<f64>().ok().map(|n| n * per)
    }

    /// `HH:MM..HH:MM` with valid clock times; start before end (no
    /// midnight crossing in v1).
    fn valid_window(text: &str) -> bool {
        let Some((start, end)) = text.split_once("..") else {
            return false;
        };
        let clock = |t: &str| -> Option<u32> {
            let (h, m) = t.split_once(':')?;
            if h.len() != 2 || m.len() != 2 {
                return None;
            }
            let h: u32 = h.parse().ok()?;
            let m: u32 = m.parse().ok()?;
            (h < 24 && m < 60).then_some(h * 60 + m)
        };
        match (clock(start), clock(end)) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        }
    }

    const SEASONS: [&'static str; 4] = ["summer", "autumn", "winter", "spring"];

    /// Source-text rendering of a value for diagnostics.
    fn value_text(value: &busbar_syntax::ast::Value) -> String {
        use busbar_syntax::ast::Value;
        match value {
            Value::Quantity { number, unit } => format!("{number}{unit}"),
            Value::Number(n) => n.clone(),
            Value::Str(s) => format!("\"{s}\""),
            Value::Ident(i) => i.clone(),
            Value::Bool(b) => b.to_string(),
            _ => "(list)".to_owned(),
        }
    }

    /// R-209/R-210: intermittent load profiles (spec §8.8).
    fn r209_r210_load_profiles(&mut self) {
        use busbar_syntax::ast::Value;
        for node in self.ir.nodes.values() {
            if node.kind != Some(NodeKind::Load) {
                continue;
            }
            let prop = |name: &str| {
                node.props
                    .iter()
                    .find(|p| p.name == name)
                    .map(|p| &p.value.value)
            };
            // R-209: durations.
            let on = prop("on_time").and_then(Self::duration_seconds);
            let period = prop("period").and_then(Self::duration_seconds);
            if let Some(p) = node.props.iter().find(|p| p.name == "on_time") {
                if on.is_none() {
                    self.error(
                        "R-209",
                        node.line,
                        node.col,
                        format!(
                            "`on_time` of `{}` must be a duration (s/min/h/d), got {}",
                            node.tag,
                            Self::value_text(&p.value.value)
                        ),
                    );
                }
            }
            if let Some(p) = node.props.iter().find(|p| p.name == "period") {
                if period.is_none() {
                    self.error(
                        "R-209",
                        node.line,
                        node.col,
                        format!(
                            "`period` of `{}` must be a duration (s/min/h/d), got {}",
                            node.tag,
                            Self::value_text(&p.value.value)
                        ),
                    );
                }
            }
            if let (Some(on), Some(period)) = (on, period) {
                if on >= period {
                    self.error(
                        "R-209",
                        node.line,
                        node.col,
                        format!("`on_time` of `{}` must be shorter than `period`", node.tag),
                    );
                    if let Some(d) = self.diags.last_mut() {
                        d.notes.push(format!("on_time = {on}s, period = {period}s"));
                    }
                }
            }
            // R-210: windows and seasons.
            if let Some(w) = prop("window") {
                let windows: Vec<&str> = match w {
                    Value::Str(s) => vec![s.as_str()],
                    Value::List(items) => items
                        .iter()
                        .map(|i| match &i.value {
                            Value::Str(s) => Some(s.as_str()),
                            _ => None,
                        })
                        .collect::<Option<_>>()
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                if windows.is_empty() || windows.iter().any(|t| !Self::valid_window(t)) {
                    self.error(
                        "R-210",
                        node.line,
                        node.col,
                        format!(
                            "`window` of `{}` must be \"HH:MM..HH:MM\" (start before end)",
                            node.tag
                        ),
                    );
                }
            }
            if let Some(s) = prop("seasons") {
                let seasons: Vec<String> = match s {
                    Value::Ident(i) => vec![i.clone()],
                    Value::List(items) => items
                        .iter()
                        .map(|i| match &i.value {
                            Value::Ident(v) => Some(v.clone()),
                            _ => None,
                        })
                        .collect::<Option<_>>()
                        .unwrap_or_default(),
                    _ => Vec::new(),
                };
                if seasons.is_empty()
                    || seasons.iter().any(|s| !Self::SEASONS.contains(&s.as_str()))
                {
                    self.error(
                        "R-210",
                        node.line,
                        node.col,
                        format!(
                            "`seasons` of `{}` must be from {}",
                            node.tag,
                            Self::SEASONS.join("/")
                        ),
                    );
                }
            }
        }
    }

    fn r201_r202_voltage_frequency(&mut self) {
        for edge in &self.ir.edges {
            let Some(va) = self.ir.endpoint_vs(&edge.from) else {
                continue;
            };
            let Some(vb) = self.ir.endpoint_vs(&edge.to) else {
                continue;
            };
            if va == vb {
                continue;
            }
            let (Some(sa), Some(sb)) = (self.ir.voltsys.get(&va), self.ir.voltsys.get(&vb)) else {
                continue;
            };
            if (sa.nominal_v - sb.nominal_v).abs() > 1e-6 || sa.phases.count() != sb.phases.count()
            {
                self.error(
                    "R-201",
                    edge.line,
                    edge.col,
                    format!(
                        "voltage-system mismatch across edge `{}` -> `{}` ({} vs {})",
                        edge.from, edge.to, va, vb
                    ),
                );
                continue;
            }
            if let (Some(fa), Some(fb)) = (sa.frequency_hz, sb.frequency_hz) {
                if (fa - fb).abs() > 1e-6 {
                    self.error(
                        "R-202",
                        edge.line,
                        edge.col,
                        format!(
                            "frequency mismatch across edge `{}` -> `{}` ({fa}Hz vs {fb}Hz)",
                            edge.from, edge.to
                        ),
                    );
                }
            }
        }
    }

    fn r203_phase_missing(&mut self) {
        for circuit in self.ir.circuits.values() {
            let Some(board) = self.ir.boards.get(&circuit.board) else {
                continue;
            };
            let Some(vs_name) = self.ir.board_vs_prop(board) else {
                continue;
            };
            let Some(vs) = self.ir.voltsys.get(&vs_name) else {
                continue;
            };
            if vs.phases.count() <= 1 || circuit.phase.is_some() {
                continue;
            }
            let Some(prot) = &circuit.protection else {
                continue;
            };
            let Some(def) = busbar_ir::types::lookup(&prot.type_name) else {
                continue;
            };
            let poles: u32 = def
                .param(&prot.props, "poles")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            if poles < vs.phases.count() {
                self.error(
                    "R-203",
                    circuit.line,
                    circuit.col,
                    format!(
                        "single-phase circuit `{}` (poles={poles}) has no `phase` on multi-phase board `{}`",
                        circuit.tag, board.tag
                    ),
                );
            }
        }
    }

    fn r204_phase_imbalance(&mut self) {
        let Some(threshold) = self.code_quantity("phase_imbalance_max_pct") else {
            return;
        };
        for board in self.ir.boards.values() {
            let Some(vs_name) = self.ir.board_vs_prop(board) else {
                continue;
            };
            let Some(vs) = self.ir.voltsys.get(&vs_name) else {
                continue;
            };
            let n = vs.phases.count();
            if n <= 1 {
                continue;
            }
            let mut per_phase = vec![0.0f64; n as usize];
            for ctag in &board.circuits {
                let Some(circuit) = self.ir.circuits.get(ctag) else {
                    continue;
                };
                let Some(phase) = &circuit.phase else {
                    continue;
                };
                let idx = match phase.as_str() {
                    "L1" => 0,
                    "L2" => 1,
                    "L3" => 2,
                    _ => continue,
                };
                if idx as u32 >= n {
                    continue;
                }
                for (load, ..) in &circuit.loads {
                    if let Some(node) = self.ir.nodes.get(load) {
                        if let Some((w, _)) = node.quantity_prop("kw") {
                            per_phase[idx] += w;
                        }
                    }
                }
            }
            let total: f64 = per_phase.iter().sum();
            if total <= 0.0 {
                continue;
            }
            let avg = total / n as f64;
            let max_dev = per_phase
                .iter()
                .map(|l| (l - avg).abs())
                .fold(0.0, f64::max);
            let pct = max_dev / avg * 100.0;
            if pct > threshold {
                self.warn(
                    "R-204",
                    board.line,
                    board.col,
                    format!(
                        "phase imbalance on board `{}` is {pct:.0}% (max {threshold:.0}%)",
                        board.tag
                    ),
                );
            }
        }
    }

    fn r205_multiphase_load(&mut self) {
        for node in self.ir.nodes.values() {
            if node.kind != Some(NodeKind::Load) {
                continue;
            }
            let Some(Value::List(phases)) = node.get("phases").map(|v| &v.value) else {
                continue;
            };
            let Some(vs) = self.ir.voltsys_of_node(&node.tag) else {
                continue;
            };
            if phases.len() as u32 > vs.phases.count() {
                self.error(
                    "R-205",
                    node.line,
                    node.col,
                    format!(
                        "load `{}` uses {} phases on a {}-phase system",
                        node.tag,
                        phases.len(),
                        vs.phases.count()
                    ),
                );
            }
        }
    }

    fn r206_neutral(&mut self) {
        for edge in &self.ir.edges {
            for text in [&edge.from, &edge.to] {
                let Some((_, Some(port), _)) = self.ir.resolve_endpoint(text) else {
                    continue;
                };
                if port != "n" {
                    continue;
                }
                let Some(vs_name) = self.ir.endpoint_vs(text) else {
                    continue;
                };
                let Some(vs) = self.ir.voltsys.get(&vs_name) else {
                    continue;
                };
                if vs.neutral == Some(false) {
                    self.error(
                        "R-206",
                        edge.line,
                        edge.col,
                        format!(
                            "neutral port `{text}` used but voltage system `{vs_name}` declares neutral = no"
                        ),
                    );
                    break;
                }
            }
        }
    }

    fn r207_earthing_tie(&mut self) {
        // Chain-level: compare the outermost endpoints of each connect.
        for stmt in &self.doc.statements {
            let ast::Statement::Connect(c) = stmt else {
                continue;
            };
            let (Value::Ident(fa), Value::Ident(la)) =
                (&c.endpoints[0].value, &c.endpoints.last().unwrap().value)
            else {
                continue;
            };
            let Some(va) = self.ir.endpoint_vs(fa) else {
                continue;
            };
            let Some(vb) = self.ir.endpoint_vs(la) else {
                continue;
            };
            let (Some(sa), Some(sb)) = (self.ir.voltsys.get(&va), self.ir.voltsys.get(&vb)) else {
                continue;
            };
            if let (Some(ea), Some(eb)) = (&sa.earthing, &sb.earthing) {
                if ea != eb {
                    self.error(
                        "R-207",
                        c.span.line,
                        c.span.col,
                        format!("connection between different earthing schemes ({ea} vs {eb})"),
                    );
                }
            }
        }
    }

    fn r208_parallel_transformers(&mut self) {
        use std::collections::BTreeMap as Map;
        // (primary board, secondary board) -> transformers
        let mut groups: Map<(String, String), Vec<&busbar_ir::IrNode>> = Map::new();
        for node in self.ir.nodes.values() {
            if node.type_name != "transformer" {
                continue;
            }
            let is_port_of = |text: &str, tag: &str, port: &str| {
                self.ir
                    .resolve_endpoint(text)
                    .is_some_and(|(entity, p, _)| entity == tag && p.as_deref() == Some(port))
            };
            let primary_board = self.ir.edges.iter().find_map(|e| {
                is_port_of(&e.to, &node.tag, "primary")
                    .then(|| self.ir.resolve_endpoint(&e.from))
                    .flatten()
                    .and_then(|(_, _, b)| b)
            });
            let secondary_board = self.ir.edges.iter().find_map(|e| {
                is_port_of(&e.from, &node.tag, "secondary")
                    .then(|| self.ir.resolve_endpoint(&e.to))
                    .flatten()
                    .and_then(|(_, _, b)| b)
            });
            if let (Some(p), Some(s)) = (primary_board, secondary_board) {
                groups.entry((p, s)).or_default().push(node);
            }
        }
        for group in groups.values() {
            if group.len() < 2 {
                continue;
            }
            let vectors: Vec<&str> = group
                .iter()
                .map(|n| n.get("vector").and_then(|v| v.value.as_str()).unwrap_or(""))
                .collect();
            let all_same = vectors.windows(2).all(|w| w[0] == w[1]);
            if !all_same {
                let (line, col, tag) = group
                    .iter()
                    .map(|n| (n.line, n.col, n.tag.clone()))
                    .max_by_key(|(l, ..)| *l)
                    .unwrap();
                self.warn(
                    "R-208",
                    line,
                    col,
                    format!(
                        "paralleled transformers have incompatible vector groups (`{tag}` differs)"
                    ),
                );
            }
        }
    }

    fn code_quantity(&self, key: &str) -> Option<f64> {
        let mut found = None;
        for profile in &self.ir.code_profiles {
            for p in &profile.props {
                if p.name == key {
                    if let Some((v, _)) = p.value.value.quantity() {
                        found = Some(v);
                    }
                }
            }
        }
        found
    }
}

fn reaches(
    arcs: &[(String, String, u32, u32)],
    from: &str,
    target: &str,
    seen: &mut Vec<String>,
) -> bool {
    for (a, b, ..) in arcs {
        if a == from {
            if b == target {
                return true;
            }
            if !seen.iter().any(|s| s == b) {
                seen.push(b.clone());
                if reaches(arcs, b, target, seen) {
                    return true;
                }
            }
        }
    }
    false
}
