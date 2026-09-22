//! Deterministic drafting-style layout (spec §16.1; implementation plan §5.5).
//!
//! v2 — vertical, single-line-diagram conventions: sources at the top,
//! boards stacked by rank below them; inside a board, each bus section is
//! a horizontal bar with its circuits hanging as vertical feeder columns
//! (protection → controller → loads stacked downward). Every ordering key
//! ends in the unique tag, so output is bit-stable for identical input.
//! `tests/invariants.rs` machine-checks the drafting contract: no glyph
//! overlaps, members contained in their boards, everything on the canvas.

use std::collections::BTreeMap;

use busbar_ir::{Ir, NodeKind};

pub const CELL: f64 = 40.0;
/// Width allotted to each circuit feeder column inside a board.
pub const FEEDER_W: f64 = 96.0;
pub const MARGIN: f64 = 48.0;
pub const ROW_GAP: f64 = 72.0;
pub const COL_GAP: f64 = 64.0;
pub const BOARD_PAD: f64 = 28.0;
pub const SECTION_BAR_H: f64 = 8.0;
/// Bar to the feeder band beneath it, and band to the next bar.
pub const SECTION_GAP: f64 = 56.0;
/// Gap between stacked cells in one feeder — room for the label and the
/// rating note without crowding the glyph below.
pub const STACK_GAP: f64 = 34.0;
/// Height of the board label strip: name plus voltage-system note
/// (guidance §5.1).
pub const LABEL_STRIP: f64 = 36.0;
/// Cells per feeder sub-column before long load chains wrap sideways
/// (guidance §5.4 — the ragged-cascade fix).
pub const MAX_COL_CELLS: usize = 4;
pub const BAR_W_MIN: f64 = 200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Board,
    Section,
    Source,
    Transformer,
    /// Plain switch blade (also relay / control_relay contacts).
    Switch,
    /// Breaker blade: diagonal with an x at the moving contact (IEC 60617).
    Protective,
    /// Disconnector: blade with a short bar across the fixed contact.
    Disconnector,
    /// Switch-disconnector (main switch): bar + x.
    MainSwitch,
    /// Contactor: blade with a perpendicular tick at its tip.
    Contactor,
    /// Residual-current device: blade with residual ellipse
    /// (reference sheet `rcd1`).
    Rcd,
    /// RCBO — breaker blade plus residual-current block ("IΔ"),
    /// reference sheet `gfci-breaker1`.
    Rcbo,
    /// Current transformer (reference sheet `CT1`).
    Ct,
    /// Wind turbine (reference sheet `WT1`).
    WindTurbine,
    /// Generator: "G" box (reference sheet `G1`) — distinct from the
    /// grid supply's "~" circle.
    Generator,
    /// Programmable logic controller: box + diagonal + "PLC".
    Plc,
    /// DC combiner box (reference sheet `dc-combiner1`).
    DcCombiner,
    /// MPPT charge controller (reference sheet `mppt1`).
    Mppt,
    /// AC/DC power supply (reference sheet `power-supply1`).
    PowerSupply,
    /// UPS box (reference sheet `ups1`).
    Ups,
    /// DC circuit breaker: breaker blade + "=" mark.
    DcBreaker,
    /// DC disconnector (reference sheet `dc-disconnector1`).
    DcDisconnector,
    /// Earth switch: blade terminating in earth bars.
    EarthSwitch,
    /// Shunt capacitor: two plates.
    Capacitor,
    /// Reactor: series coil.
    Reactor,
    /// Fixed resistor (NGR).
    Resistor,
    /// DC fuse: fuse with the `=` DC mark.
    DcFuse,
    Fuse,
    Load,
    Lamp,
    Motor,
    Socket,
    Meter,
    Battery,
    Inverter,
    Pv,
    Earth,
    Junction,
    Ats,
    Spd,
    /// EV supply equipment (no IEC 60617 extract: box + EV text).
    Evse,
    /// Relay / PLC: labeled box (per the corpus/render/house.svg reference).
    Relay,
    /// Heating element: box with a zigzag.
    Heating,
    Generic,
}

#[derive(Debug, Clone)]
pub struct Place {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub label: String,
    pub glyph: Glyph,
    /// Secondary annotation (rating text) drawn under the label.
    pub note: Option<String>,
}

impl Place {
    pub fn center(&self) -> (f64, f64) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    pub points: Vec<(f64, f64)>,
    pub dashed: bool,
    /// Entity tags the route connects (for layout invariants).
    pub from_tag: String,
    pub to_tag: String,
}

#[derive(Debug, Default)]
pub struct Layout {
    pub width: f64,
    pub height: f64,
    pub places: BTreeMap<String, Place>,
    pub routes: Vec<Route>,
    /// Board group sizes (width, height) from the internals pass.
    pub boards: BTreeMap<String, (f64, f64)>,
}

pub fn glyph_for(type_name: &str, kind: Option<NodeKind>) -> Glyph {
    match type_name {
        "transformer" => Glyph::Transformer,
        "fuse" => Glyph::Fuse,
        "battery" => Glyph::Battery,
        "inverter" | "rectifier" => Glyph::Inverter,
        "pv_array" | "pv_string" => Glyph::Pv,
        "wind_turbine" => Glyph::WindTurbine,
        "earth" => Glyph::Earth,
        "junction" => Glyph::Junction,
        "ats" | "changeover" => Glyph::Ats,
        "spd" => Glyph::Spd,
        "ct" => Glyph::Ct,
        "sync_check" | "meter" => Glyph::Meter,
        "motor" => Glyph::Motor,
        "lighting" => Glyph::Lamp,
        "socket" => Glyph::Socket,
        "disconnector" | "isolator" | "load_break_switch" => Glyph::Disconnector,
        "main_switch" => Glyph::MainSwitch,
        "contactor" => Glyph::Contactor,
        "rcd" | "rccb" => Glyph::Rcd,
        "rcbo" | "elcb" | "rcmcd" => Glyph::Rcbo,
        "control_relay" => Glyph::Relay,
        "heating" => Glyph::Heating,
        "evse" => Glyph::Evse,
        "bus" | "busbar" => Glyph::Section,
        "generator" => Glyph::Generator,
        "plc" => Glyph::Plc,
        "dc_combiner" => Glyph::DcCombiner,
        "mppt" => Glyph::Mppt,
        "power_supply" => Glyph::PowerSupply,
        "ups" => Glyph::Ups,
        "dc_breaker" => Glyph::DcBreaker,
        "dc_fuse" => Glyph::DcFuse,
        "dc_disconnector" => Glyph::DcDisconnector,
        "earth_switch" => Glyph::EarthSwitch,
        "capacitor_bank" => Glyph::Capacitor,
        "reactor" => Glyph::Reactor,
        "ngr" => Glyph::Resistor,
        // Weak-symbol fixes: a protection relay is a box (was the
        // breaker blade), a VT is a transformer, HVAC/pool pumps are
        // motor loads, cooking/HW loads are heating elements.
        "relay" => Glyph::Relay,
        "vt" => Glyph::Transformer,
        "hvac" | "pool_pump" => Glyph::Motor,
        "oven" | "cooktop" | "hws" => Glyph::Heating,
        "board" => Glyph::Board,
        _ => match kind {
            Some(NodeKind::Source) => Glyph::Source,
            Some(NodeKind::Switch) => Glyph::Switch,
            Some(NodeKind::Protective) => Glyph::Protective,
            Some(NodeKind::Load) => Glyph::Load,
            Some(NodeKind::Measurement) => Glyph::Meter,
            _ => Glyph::Generic,
        },
    }
}

pub fn build(ir: &Ir) -> Layout {
    let mut layout = Layout::default();

    // -- Sub-main devices (todo: incomer breakers above a bus). ---------------
    // A board-declared device whose directed feed leaves for a different
    // board is THAT board's incomer breaker: it renders in the fed
    // board's incomer column, above the fed bus — never in the feeding
    // board's below-bar strip.
    let mut submain_of: BTreeMap<String, String> = BTreeMap::new(); // device -> fed board
    let mut foreign_incomers: BTreeMap<String, Vec<String>> = BTreeMap::new(); // fed board -> devices
    for node in ir.nodes.values() {
        let Some(parent) = &node.parent else { continue };
        if !ir.boards.contains_key(parent) {
            continue;
        }
        let mut fed: Vec<String> = Vec::new();
        for e in &ir.edges {
            let Some((from_e, _, _)) = ir.resolve_endpoint(&e.from) else {
                continue;
            };
            if from_e != node.tag {
                continue;
            }
            // The feed may land on a device inside the fed board (owner
            // form) or on the fed board itself (`-> HOUSE.in` resolves to
            // the container, which owns nothing).
            let Some((to_e, _, owner)) = ir.resolve_endpoint(&e.to) else {
                continue;
            };
            let fed_board = if ir
                .nodes
                .get(&to_e)
                .is_some_and(|n| n.kind == Some(NodeKind::Container))
            {
                to_e.as_str()
            } else {
                match owner.as_deref() {
                    Some(o) => o,
                    None => continue,
                }
            };
            if fed_board != parent.as_str()
                && ir.boards.contains_key(fed_board)
                && !fed.iter().any(|f| f == fed_board)
            {
                fed.push(fed_board.to_owned());
            }
        }
        if let Some(target) = fed.into_iter().min() {
            submain_of.insert(node.tag.clone(), target.to_owned());
            foreign_incomers
                .entry(target.to_owned())
                .or_default()
                .push(node.tag.clone());
        }
    }

    // Boards whose incomer column exists (own chain or foreign sub-mains)
    // — their feed lands on the column, so they hang column-aligned.
    let mut has_incomer_column: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    // Board devices placed with NO resolvable edges (an SPD that
    // implicitly attaches to the bus) still need their wire drawn.
    let mut implicit_taps: Vec<(String, String)> = Vec::new(); // device, section

    // -- Board internals: bars + feeder bands. --------------------------------
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for board in ir.boards.values() {
        let member_list = members.entry(board.tag.clone()).or_default();

        // Circuits grouped under their (or the first) section, board order.
        let mut feeders: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for ctag in &board.circuits {
            let section = ir
                .circuits
                .get(ctag)
                .and_then(|c| c.section.clone())
                .or_else(|| board.sections.first().cloned());
            if let Some(section) = section {
                feeders.entry(section).or_default().push(ctag);
            }
        }

        // Board-declared devices (and loads declared inside boards) need a
        // Place or every edge touching them silently drops from the drawing.
        // Devices feeding a section sit above the bars (incomer style); the
        // rest sit below the feeder bands. Sections and the protection/
        // controller cells circuits own are placed by their own passes.
        let mut upstream_devs: Vec<&busbar_ir::IrNode> = Vec::new();
        let mut downstream_devs: Vec<&busbar_ir::IrNode> = Vec::new();
        // (device, section it feeds) — placed between the bars.
        let mut tie_devs: Vec<(&busbar_ir::IrNode, String)> = Vec::new();
        for node in ir.nodes.values() {
            if node.parent.as_deref() != Some(board.tag.as_str())
                || board.sections.contains(&node.tag)
                || node.tag.ends_with(".protection")
                || node.tag.ends_with(".controller")
                || submain_of.contains_key(&node.tag)
            {
                continue;
            }
            // The whole incomer chain stacks above the bar (guidance
            // §2.4): a device is upstream when a directed power path
            // from it reaches a section — grid -> fuse -> meter -> main
            // switch all promote, while an outgoing way fed from the bus
            // never does.
            let feeds_section = directed_hops_to_section(ir, &node.tag, board).is_some();
            // A device fed FROM one section that reaches a DIFFERENT
            // section is a bus tie: it renders BETWEEN the two bars it
            // joins (guidance §5.2), never in the incomer column.
            let fed_from: Vec<&String> = board
                .sections
                .iter()
                .filter(|s| {
                    ir.edges.iter().any(|e| {
                        let (Some((f, _, _)), Some((t, _, _))) =
                            (ir.resolve_endpoint(&e.from), ir.resolve_endpoint(&e.to))
                        else {
                            return false;
                        };
                        f == **s && t == node.tag
                    })
                })
                .collect();
            let reaches: Vec<&String> = board
                .sections
                .iter()
                .filter(|s| **s != node.tag)
                .filter(|s| {
                    // directed, board-local reach from node to s
                    let mut dist: BTreeMap<String, u32> = BTreeMap::from([(node.tag.clone(), 0)]);
                    let mut queue: std::collections::VecDeque<String> =
                        std::collections::VecDeque::from([node.tag.clone()]);
                    while let Some(t) = queue.pop_front() {
                        if &t == *s {
                            return true;
                        }
                        let d = dist[&t];
                        for e in &ir.edges {
                            let (Some((f, _, _)), Some((tt, _, owner))) =
                                (ir.resolve_endpoint(&e.from), ir.resolve_endpoint(&e.to))
                            else {
                                continue;
                            };
                            if f != t {
                                continue;
                            }
                            if owner.as_deref().is_some_and(|o| o != board.tag) {
                                continue;
                            }
                            if tt == *board.tag {
                                continue;
                            }
                            if !dist.contains_key(&tt) {
                                dist.insert(tt.clone(), d + 1);
                                queue.push_back(tt);
                            }
                        }
                    }
                    false
                })
                .collect();
            let is_tie = fed_from
                .iter()
                .any(|s1| reaches.iter().any(|s2| *s2 != *s1));
            if is_tie {
                // Anchor above the LATER of the joined sections in board
                // order — the tie sits between the bars whichever way
                // its edges were written.
                let pos = |s: &String| board.sections.iter().position(|x| *x == *s).unwrap_or(0);
                let s1 = fed_from[0];
                let s2 = reaches
                    .iter()
                    .find(|s| ***s != **s1)
                    .expect("tie reaches another section");
                let anchor = if pos(s1) > pos(s2) { s1 } else { s2 };
                tie_devs.push((node, anchor.clone()));
            } else if feeds_section {
                upstream_devs.push(node);
            } else {
                downstream_devs.push(node);
            }
        }

        // Sub-main breakers declared in another board feed THIS bus:
        // they join the incomer column above it, ordered by the same
        // directed-hop rule (so house QF1 sits above the house QS1).
        if let Some(foreign) = foreign_incomers.get(board.tag.as_str()) {
            for tag in foreign {
                if let Some(node) = ir.nodes.get(tag.as_str()) {
                    upstream_devs.push(node);
                }
            }
        }

        // Feeder band widths per section: each circuit spans
        // ceil(cells / MAX_COL_CELLS) sub-columns (guidance §5.4).
        let mut section_widths: BTreeMap<&String, f64> = BTreeMap::new();
        for (section, ctags) in &feeders {
            let mut w = 0.0;
            for ctag in ctags {
                if let Some(c) = ir.circuits.get(*ctag) {
                    let total = c.protection.is_some() as usize
                        + c.controller.is_some() as usize
                        + c.loads.len();
                    let sub_cols = total.div_ceil(MAX_COL_CELLS).max(1);
                    w += sub_cols as f64 * FEEDER_W;
                }
            }
            section_widths.insert(section, w);
        }
        let inner_w = section_widths.values().copied().fold(0.0f64, f64::max);
        let strip_w = |n: usize| {
            if n == 0 {
                0.0
            } else {
                n as f64 * (CELL + STACK_GAP) - STACK_GAP
            }
        };
        let bar_w = inner_w.max(strip_w(downstream_devs.len())).max(BAR_W_MIN);

        // Incomer column: upstream devices stack vertically above the bar
        // in power order — the device adjacent to the section lands nearest
        // the bar (guidance §2.4). Hop distance from the sections orders
        // the chain; the tag breaks ties.
        let mut y = BOARD_PAD + LABEL_STRIP;
        if !upstream_devs.is_empty() {
            has_incomer_column.insert(board.tag.clone());
            let mut ordered = upstream_devs;
            ordered.sort_by_key(|n| {
                let hops = directed_hops_to_section(ir, &n.tag, board).unwrap_or(u32::MAX / 2);
                (u32::MAX - hops, n.tag.clone())
            });
            for node in ordered {
                member_list.push(node.tag.clone());
                layout.places.insert(
                    node.tag.clone(),
                    Place {
                        x: BOARD_PAD,
                        y,
                        w: CELL,
                        h: CELL,
                        label: display_label(&node.tag, &node.props),
                        glyph: glyph_for(&node.type_name, node.kind),
                        note: rating_note(&node.props),
                    },
                );
                y += CELL + STACK_GAP;
            }
        }
        for section in &board.sections {
            // Bus ties land between the bars: any tie feeding THIS
            // section sits directly above its bar (guidance §5.2).
            let mut ties_here: Vec<&busbar_ir::IrNode> = tie_devs
                .iter()
                .filter(|(_, target)| target == section.as_str())
                .map(|(n, _)| *n)
                .collect();
            ties_here.sort_by(|a, b| a.tag.cmp(&b.tag));
            for node in ties_here {
                member_list.push(node.tag.clone());
                layout.places.insert(
                    node.tag.clone(),
                    Place {
                        x: BOARD_PAD,
                        y,
                        w: CELL,
                        h: CELL,
                        label: display_label(&node.tag, &node.props),
                        glyph: glyph_for(&node.type_name, node.kind),
                        note: rating_note(&node.props),
                    },
                );
                y += CELL + STACK_GAP;
            }
            // Section bar.
            member_list.push(section.clone());
            layout.places.insert(
                section.clone(),
                Place {
                    x: BOARD_PAD,
                    y,
                    w: bar_w,
                    h: SECTION_BAR_H,
                    label: section.rsplit('.').next().unwrap_or(section).to_owned(),
                    glyph: Glyph::Section,
                    note: None,
                },
            );
            y += SECTION_BAR_H + 26.0; // bar + section label
            // Feeder band under this bar.
            if let Some(ctags) = feeders.get(section) {
                let mut depth = 0.0f64;
                let mut fx = BOARD_PAD;
                for ctag in ctags {
                    let Some(circuit) = ir.circuits.get(*ctag) else {
                        continue;
                    };
                    // Circuit anchor: a junction dot at the feeder's bar
                    // tap. Ports on the circuit itself (`connect X ->
                    // CIRCUIT.out`, PV-backfeed style) land here instead of
                    // dropping off the drawing.
                    member_list.push((*ctag).clone());
                    layout.places.insert(
                        (*ctag).clone(),
                        Place {
                            x: fx + FEEDER_W / 2.0,
                            y,
                            w: 0.0,
                            h: 0.0,
                            label: String::new(),
                            glyph: Glyph::Junction,
                            note: None,
                        },
                    );
                    // Column balancing (guidance §5.4): protection and
                    // controller open the first sub-column; loads fill
                    // column-major, wrapping at MAX_COL_CELLS.
                    let total = circuit.protection.is_some() as usize
                        + circuit.controller.is_some() as usize
                        + circuit.loads.len();
                    let sub_cols = total.div_ceil(MAX_COL_CELLS).max(1);
                    let mut place_cell = |tag: &str,
                                          label: &str,
                                          glyph: Glyph,
                                          note: Option<String>,
                                          slot: usize| {
                        let col = slot / MAX_COL_CELLS;
                        let row = slot % MAX_COL_CELLS;
                        member_list.push(tag.to_owned());
                        layout.places.insert(
                            tag.to_owned(),
                            Place {
                                x: fx + col as f64 * FEEDER_W + (FEEDER_W - CELL) / 2.0,
                                y: y + row as f64 * (CELL + STACK_GAP),
                                w: CELL,
                                h: CELL,
                                label: label.to_owned(),
                                glyph,
                                note,
                            },
                        );
                        (row + 1) as f64 * (CELL + STACK_GAP)
                    };
                    let mut slot = 0usize;
                    if let Some(prot) = &circuit.protection {
                        let d = place_cell(
                            &prot.tag,
                            &short_tag(ctag),
                            glyph_for(&prot.type_name, prot.kind),
                            rating_note(&prot.props),
                            slot,
                        );
                        depth = depth.max(d);
                        slot += 1;
                    }
                    if let Some(ctl) = &circuit.controller {
                        let d = place_cell(
                            &ctl.tag,
                            &short_tag(ctag),
                            glyph_for(&ctl.type_name, ctl.kind),
                            None,
                            slot,
                        );
                        depth = depth.max(d);
                        slot += 1;
                    }
                    for (load, ..) in &circuit.loads {
                        let node = ir.nodes.get(load);
                        let d = place_cell(
                            load,
                            &node
                                .map(|n| display_label(load, &n.props))
                                .unwrap_or_else(|| short_tag(load)),
                            node.map(|n| glyph_for(&n.type_name, n.kind))
                                .unwrap_or(Glyph::Generic),
                            node.and_then(|n| rating_note(&n.props)),
                            slot,
                        );
                        depth = depth.max(d);
                        slot += 1;
                    }
                    fx += sub_cols as f64 * FEEDER_W;
                }
                y += depth;
            }
            y += SECTION_GAP;
        }

        // Devices that hang off the bus rather than feed it sit in a strip
        // below the feeder bands (contactor / relay / board-local loads).
        let mut y_end = y - SECTION_GAP;
        if !downstream_devs.is_empty() {
            y_end += 20.0;
            let mut dx = BOARD_PAD;
            for node in &downstream_devs {
                member_list.push(node.tag.clone());
                let wired = ir.edges.iter().any(|e| {
                    [&e.from, &e.to].iter().any(|t| {
                        ir.resolve_endpoint(t)
                            .is_some_and(|(entity, _, _)| entity == node.tag)
                    })
                });
                if !wired {
                    if let Some(section) = board.sections.first() {
                        implicit_taps.push((node.tag.clone(), section.clone()));
                    }
                }
                layout.places.insert(
                    node.tag.clone(),
                    Place {
                        x: dx,
                        y: y_end,
                        w: CELL,
                        h: CELL,
                        label: display_label(&node.tag, &node.props),
                        glyph: glyph_for(&node.type_name, node.kind),
                        note: rating_note(&node.props),
                    },
                );
                dx += CELL + STACK_GAP;
            }
            y_end += CELL;
        }

        let board_w = bar_w + BOARD_PAD * 2.0;
        let board_h = y_end + BOARD_PAD;
        layout
            .boards
            .insert(board.tag.clone(), (board_w, board_h.max(BOARD_PAD * 2.0)));
    }

    // -- Global: rank rows, entities side by side. ----------------------------
    let mut ranks = rank_from_sources(ir);

    // Circuits rank with their head device so feeds leaving a circuit
    // (`BOARD.CIRCUIT.out`) inherit a real BFS distance.
    for circuit in ir.circuits.values() {
        let head = circuit
            .controller
            .as_ref()
            .map(|c| c.tag.clone())
            .or_else(|| circuit.protection.as_ref().map(|p| p.tag.clone()));
        if let Some(head) = head {
            let hr = ranks.get(&head).copied();
            if let Some(r) = hr {
                let entry = ranks.entry(circuit.tag.clone()).or_insert(u32::MAX);
                *entry = (*entry).min(r);
            }
        }
    }

    // A board's row is where power first reaches it (guidance §2.1): rank
    // with its entry member — the minimum BFS rank across its members —
    // not where an edge happens to name the container itself. The deepest
    // member would be wrong: it puts the frame below unrelated entities
    // fed at intermediate ranks (e.g. the earth electrode); §2.5's
    // parent-child floor below is what orders nested boards.
    for board in ir.boards.keys() {
        let mut r = ranks.get(board).copied().unwrap_or(u32::MAX);
        let fold = |tag: &str, ranks: &BTreeMap<String, u32>, r: &mut u32| {
            if let Some(m) = ranks.get(tag) {
                *r = (*r).min(*m);
            }
        };
        for node in ir.nodes.values() {
            if node.parent.as_deref() == Some(board.as_str()) {
                fold(&node.tag, &ranks, &mut r);
            }
        }
        for section in ir.sections.values() {
            if section.board == *board {
                fold(&section.tag, &ranks, &mut r);
            }
        }
        for circuit in ir.circuits.values() {
            if circuit.board == *board {
                fold(&circuit.tag, &ranks, &mut r);
            }
        }
        ranks.insert(board.clone(), r);
    }

    // Guidance §2.5: a board fed from inside another board hangs below
    // the feeder that feeds it. Record owner and feeding entity
    // (deterministic pick: smallest owner, then feeder tag) and raise the
    // child's rank so its row is strictly below the parent's; the
    // anchor's x is read from its placed glyph when the child's row is
    // packed.
    // board -> (owner, feeder, self_placed): a self-placed feeder (a
    // sub-main breaker rendered inside the FED board's column) has no
    // global position until the fed board lands, so it cannot anchor.
    let mut feeder_of: BTreeMap<String, (String, String, bool)> = BTreeMap::new();
    for board in ir.boards.keys() {
        let mut candidates: Vec<(String, String)> = Vec::new();
        for edge in &ir.edges {
            let Some((to_entity, _, _)) = ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            // The feed may land on the board port, a section, a circuit
            // or a member device (`-> HW_SUB.QS2.in`) — all feed the
            // board equally.
            let to_owner = if &to_entity == board {
                Some(board.clone())
            } else {
                ir.nodes
                    .get(&to_entity)
                    .and_then(|n| n.parent.clone())
                    .or_else(|| ir.sections.get(&to_entity).map(|s| s.board.clone()))
                    .or_else(|| ir.circuits.get(&to_entity).map(|c| c.board.clone()))
            };
            if to_owner.as_deref() != Some(board.as_str()) {
                continue;
            }
            let Some((from_entity, _, Some(owner))) = ir.resolve_endpoint(&edge.from) else {
                continue;
            };
            if owner != *board && ir.boards.contains_key(&owner) {
                candidates.push((owner, from_entity));
            }
        }
        if let Some((owner, feeder)) = candidates.into_iter().min() {
            let floor = ranks
                .get(&owner)
                .copied()
                .unwrap_or(u32::MAX)
                .saturating_add(1);
            let rank = ranks.get_mut(board.as_str()).expect("board ranked above");
            *rank = (*rank).max(floor);
            let self_placed = foreign_incomers
                .get(board.as_str())
                .is_some_and(|tags| tags.contains(&feeder));
            feeder_of.insert(board.clone(), (owner, feeder, self_placed));
        }
    }

    // §2.5 must hold even when the whole cluster is unreachable from any
    // source (orphan sub-boards, R-109 territory): u32::MAX cannot be
    // raised, so unreachable boards are rebased by feeder-nesting depth —
    // deeper boards occupy lower rows within the trailing MAX band. The
    // relaxation is bounded by the board count and monotone, so cycles
    // (R-108 feeds) cannot loop it.
    let unranked: Vec<&String> = ir
        .boards
        .keys()
        .filter(|b| ranks.get(*b).copied() == Some(u32::MAX))
        .collect();
    if !unranked.is_empty() {
        let mut depth: BTreeMap<&str, u32> = unranked.iter().map(|b| (b.as_str(), 0)).collect();
        for _ in 0..unranked.len() {
            for b in &unranked {
                let Some((owner, _, _)) = feeder_of.get(*b) else {
                    continue;
                };
                let parent = depth.get(owner.as_str()).copied().unwrap_or(0);
                let child = depth
                    .get_mut(b.as_str())
                    .expect("unranked board has a depth entry");
                *child = (*child).max(parent + 1);
            }
        }
        let max_depth = depth.values().copied().max().unwrap_or(0);
        for b in &unranked {
            let d = depth[b.as_str()];
            ranks.insert((*b).clone(), u32::MAX - (max_depth - d));
        }
    }

    let mut peer_partner: BTreeMap<String, String> = BTreeMap::new();
    // Peer bonds (earth/neutral ties) belong BESIDE the thing they bond:
    // a node whose every edge is a peer edge joins its partner's row
    // instead of a distant rank row, so the dashed bond stays short and
    // never crosses the drawing (todo: feeds must not pass through
    // boards).
    for node in ir.nodes.values() {
        if node.parent.is_some() || ir.boards.contains_key(&node.tag) {
            continue; // boards rank by their members, never by bonds
        }
        let mut peer_rank: Option<u32> = Some(u32::MAX);
        let mut powered = false;
        for e in &ir.edges {
            if powered {
                break;
            }
            for (mine, other) in [
                (e.from.as_str(), e.to.as_str()),
                (e.to.as_str(), e.from.as_str()),
            ] {
                let Some((entity, _, _)) = ir.resolve_endpoint(mine) else {
                    continue;
                };
                if entity != node.tag {
                    continue;
                }
                if e.arrow != busbar_syntax::ast::Arrow::Peer {
                    powered = true; // a powered edge — rank rules apply
                    peer_rank = None;
                    break;
                }
                let Some((other_e, _, _)) = ir.resolve_endpoint(other) else {
                    continue;
                };
                if let Some(r) = ranks.get(&other_e) {
                    peer_rank = Some(peer_rank.unwrap_or(u32::MAX).min(*r));
                }
            }
        }
        if let Some(r) = peer_rank.filter(|r| *r != u32::MAX) {
            ranks.insert(node.tag.clone(), r);
            // Remember the partner so the row places the peer directly
            // beside it — a bond must not run across other entities.
            let mut best: Option<(u32, String)> = None;
            for e in &ir.edges {
                if e.arrow != busbar_syntax::ast::Arrow::Peer {
                    continue;
                }
                for (mine, other) in [
                    (e.from.as_str(), e.to.as_str()),
                    (e.to.as_str(), e.from.as_str()),
                ] {
                    let (Some((entity, _, _)), Some((other_e, _, _))) =
                        (ir.resolve_endpoint(mine), ir.resolve_endpoint(other))
                    else {
                        continue;
                    };
                    if entity == node.tag {
                        if let Some(r2) = ranks.get(&other_e) {
                            let name = other_e.as_str().to_owned();
                            if best.as_ref().is_none_or(|(br0, _)| *r2 < *br0) {
                                best = Some((*r2, name));
                            }
                        }
                    }
                }
            }
            if let Some((_, partner)) = best {
                peer_partner.insert(node.tag.clone(), partner);
            }
        }
    }

    let in_circuit: std::collections::BTreeSet<String> = ir
        .circuits
        .values()
        .flat_map(|c| c.loads.iter().map(|(l, ..)| l.clone()))
        .collect();

    let mut rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    // Hangers: top-level nodes feeding a board member from OUTSIDE hang
    // ABOVE that board, column-aligned to the member they feed (the PV
    // chain above its backfeed way) — they leave the rank-row flow.
    let mut hangs_above: BTreeMap<String, String> = BTreeMap::new(); // node -> fed member tag
    for node in ir.nodes.values() {
        if node.parent.is_some() || ir.boards.contains_key(&node.tag) {
            continue;
        }
        for e in &ir.edges {
            if e.arrow == busbar_syntax::ast::Arrow::Peer {
                continue;
            }
            let Some((from_e, _, _)) = ir.resolve_endpoint(&e.from) else {
                continue;
            };
            if from_e != node.tag {
                continue;
            }
            let Some((to_e, _, owner)) = ir.resolve_endpoint(&e.to) else {
                continue;
            };
            // Only CIRCUIT-PORT backfeeds hang above the board (the PV
            // inverter onto its way) — a plain source feeding a board
            // device keeps its rank row.
            if owner.is_some_and(|o| ir.boards.contains_key(&o)) && ir.circuits.contains_key(&to_e)
            {
                hangs_above.entry(node.tag.clone()).or_insert(to_e.clone());
            }
        }
    }
    // Chains extend upward: anything feeding a hanger hangs above it
    // (the array above the inverter above its way).
    loop {
        let mut grew = false;
        for node in ir.nodes.values() {
            if node.parent.is_some() || hangs_above.contains_key(&node.tag) {
                continue;
            }
            for e in &ir.edges {
                if e.arrow == busbar_syntax::ast::Arrow::Peer {
                    continue;
                }
                let Some((from_e, _, _)) = ir.resolve_endpoint(&e.from) else {
                    continue;
                };
                if from_e != node.tag {
                    continue;
                }
                let Some((to_e, _, _)) = ir.resolve_endpoint(&e.to) else {
                    continue;
                };
                if hangs_above.contains_key(&to_e) {
                    hangs_above.insert(node.tag.clone(), to_e.clone());
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    for (tag, node) in &ir.nodes {
        if node.parent.is_some() || tag.ends_with(".protection") || tag.ends_with(".controller") {
            continue;
        }
        if in_circuit.contains(tag) || hangs_above.contains_key(tag) {
            continue;
        }
        let rank = ranks.get(tag).copied().unwrap_or(u32::MAX);
        rows.entry(rank).or_default().push(tag.clone());
    }

    // Chain headroom per board (finding: a board near the top with a
    // two-deep hanger chain overflowed the canvas) — reserve the
    // chain's height above the row before placing it.
    let chain_height: BTreeMap<String, f64> = {
        let mut m: BTreeMap<String, f64> = BTreeMap::new();
        for (node, member) in &hangs_above {
            let mut board = ir.circuits.get(member).map(|c| c.board.clone());
            let mut hop = member.clone();
            while board.is_none() {
                let Some(next) = hangs_above.get(&hop) else {
                    break;
                };
                board = ir.circuits.get(next).map(|c| c.board.clone());
                if *next == hop {
                    break;
                }
                hop = next.clone();
            }
            if let Some(b) = board.filter(|b| ir.boards.contains_key(b)) {
                // Same math as the stacking: 2*extent + 12px wire gap.
                let g = ir
                    .nodes
                    .get(node.as_str())
                    .map(|n| glyph_for(&n.type_name, n.kind))
                    .unwrap_or(Glyph::Generic);
                *m.entry(b).or_insert(0.0) += 2.0 * extent(g) + 12.0;
            }
        }
        m
    };
    let mut max_right = MARGIN;
    let mut cur_y = MARGIN;
    for (_rank, mut tags) in rows {
        // `layout { TAG { column = N; } }` hints fix left-to-right order
        // (spec §16.1 advisory); §2.5 feeder alignment is the natural
        // secondary order; the tag is the deterministic tiebreak.
        let desired_x = |tag: &String| -> f64 {
            feeder_of
                .get(tag)
                .filter(|(_, _, self_placed)| !self_placed)
                .and_then(|(_, feeder, _)| layout.places.get(feeder))
                .map(|p| p.center().0)
                .unwrap_or(f64::MAX)
        };
        tags.sort_by(|a, b| {
            let ha = ir.layout_columns.get(a).copied().unwrap_or(u64::MAX);
            let hb = ir.layout_columns.get(b).copied().unwrap_or(u64::MAX);
            ha.cmp(&hb)
                .then_with(|| desired_x(a).total_cmp(&desired_x(b)))
                .then_with(|| a.cmp(b))
        });
        // Adopted peers sit immediately after their partner — moved
        // there, not merely appended (a peer sorting before its partner
        // must relocate).
        let peers: Vec<String> = peer_partner.keys().cloned().collect();
        let mut beside: Vec<String> = Vec::new();
        for t in tags.iter().filter(|t| !peers.contains(t)) {
            beside.push(t.clone());
            for (peer, partner) in &peer_partner {
                if partner == t {
                    beside.push(peer.clone());
                }
            }
        }
        // Orphaned peers (partner absent from this row) still place.
        for t in &tags {
            if peers.contains(t) && !beside.contains(t) {
                beside.push(t.clone());
            }
        }
        tags = beside;
        let headroom = tags
            .iter()
            .filter_map(|t| chain_height.get(t))
            .fold(0.0f64, |a: f64, b: &f64| a.max(*b));
        cur_y += headroom;
        // Anchored-or-jostled placement (guidance §2.5, per review):
        // a fed board sits as close as it can BELOW its feeder column —
        // at the anchor when the row is free there, jostled right (never
        // overlapping) when an earlier board occupies the spot. Flow
        // entities follow in the same row.
        let mut cur_x = MARGIN;
        for tag in &tags {
            let (w, h) = layout.boards.get(tag).copied().unwrap_or((CELL, CELL));
            let mut x = cur_x;
            if let Some((_, feeder, false)) = feeder_of.get(tag) {
                if let Some(p) = layout.places.get(feeder) {
                    // Column-aligned when the board has an incomer
                    // column (the feed drops onto the incomer device),
                    // else frame-centred under the tap.
                    let anchor = if has_incomer_column.contains(tag.as_str()) {
                        p.center().0 - BOARD_PAD - CELL / 2.0
                    } else {
                        p.center().0 - w / 2.0
                    };
                    x = x.max(anchor.max(MARGIN));
                }
            }
            let node = ir.nodes.get(tag);
            let member_list = members.get(tag).cloned().unwrap_or_default();
            offset_group(&mut layout, &member_list, x, cur_y);
            layout.places.insert(
                tag.clone(),
                Place {
                    x,
                    y: cur_y,
                    w,
                    h,
                    label: tag.clone(),
                    glyph: node
                        .map(|n| glyph_for(&n.type_name, n.kind))
                        .unwrap_or(Glyph::Generic),
                    // Boards carry their voltage system in the label
                    // strip (guidance §4.5): "230V 1ph 50Hz".
                    note: if layout.boards.contains_key(tag) {
                        voltsys_note(ir, tag)
                    } else {
                        node.and_then(|n| rating_note(&n.props))
                    },
                },
            );
            cur_x = x + w + COL_GAP;
            max_right = max_right.max(cur_x - COL_GAP);
        }
        let row_h = tags
            .iter()
            .filter_map(|t| layout.boards.get(t).map(|(_, h)| *h))
            .chain(std::iter::once(CELL))
            .fold(0.0f64, |a: f64, b: f64| a.max(b));
        cur_y += row_h + ROW_GAP;
    }

    // Hangers (the DC chain above its backfeed way): column-aligned to
    // the member they feed, stacked upward from the fed board's top —
    // the inverter sits directly above its way, the array above the
    // inverter (todo: PV inverter directly above its bus connection).
    {
        // Group by fed board; stack order = BFS rank (highest first,
        // closest to the board).
        let mut by_board: BTreeMap<String, Vec<(String, String, u32)>> = BTreeMap::new();
        for (node, member) in &hangs_above {
            // Follow the chain: a circuit anchor names its board
            // directly; a node anchor hangs above something that
            // eventually does.
            let mut board = ir.circuits.get(member).map(|c| c.board.clone());
            let mut hop = member.clone();
            while board.is_none() {
                let Some(next) = hangs_above.get(&hop) else {
                    break;
                };
                board = ir.circuits.get(next).map(|c| c.board.clone());
                if *next == hop {
                    break;
                }
                hop = next.clone();
            }
            if let Some(board) = board.filter(|b| ir.boards.contains_key(b)) {
                let rank = ranks.get(node).copied().unwrap_or(0);
                by_board.entry(board.to_owned()).or_default().push((
                    node.to_owned(),
                    member.clone(),
                    rank,
                ));
            }
        }
        for (board, hangers) in &by_board {
            let bp_xy = layout
                .places
                .get(board.as_str())
                .map(|p| (p.x, p.y, p.center().0));
            let Some((_, bp_y, bp_cx)) = bp_xy else {
                continue;
            };
            let mut hangers = hangers.clone();
            hangers.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
            let mut y = bp_y;
            for (node, member, _) in &hangers {
                let anchor_x = layout
                    .places
                    .get(member.as_str())
                    .map(|p| p.center().0)
                    .unwrap_or(bp_cx);
                let Some(n) = ir.nodes.get(node.as_str()) else {
                    continue;
                };
                let g = glyph_for(&n.type_name, n.kind);
                // Stack by real glyph extents with a 12px wire gap —
                // a 60px PV panel over a 40px inverter must show the
                // wire between them (todo: PV/INV1 overlap).
                y -= 12.0 + extent(g);
                let centre = y;
                y -= extent(g);
                layout.places.insert(
                    node.clone(),
                    Place {
                        x: anchor_x - CELL / 2.0,
                        y: centre - CELL / 2.0,
                        w: CELL,
                        h: CELL,
                        label: display_label(node, &n.props),
                        glyph: glyph_for(&n.type_name, n.kind),
                        note: rating_note(&n.props),
                    },
                );
                max_right = max_right.max(anchor_x + CELL / 2.0);
            }
        }
    }

    layout.width = max_right + MARGIN;
    layout.height = cur_y - ROW_GAP + MARGIN;

    // -- Routes: deterministic edge order. --------------------------------------
    // Reverse member map for channel routing: every placed tag -> its
    // board (boards own their frames).
    let mut members_of: BTreeMap<&str, &str> = BTreeMap::new();
    for (board_tag, member_list) in &members {
        members_of.insert(board_tag.as_str(), board_tag.as_str());
        for t in member_list {
            members_of.insert(t.as_str(), board_tag.as_str());
        }
    }
    for edge in &ir.edges {
        let (Some((a, _, _)), Some((b, _, _))) = (
            ir.resolve_endpoint(&edge.from),
            ir.resolve_endpoint(&edge.to),
        ) else {
            continue;
        };
        let (Some(pa), Some(pb)) = (layout.places.get(&a), layout.places.get(&b)) else {
            continue;
        };
        // Wires land on the glyph's own terminal — the end of its lead,
        // on the vertical centreline (guidance §3; lead lengths per the
        // symbol sheet) — never on its centre, so a device's in and out
        // wires no longer share a point.
        let down = pb.center().1 >= pa.center().1;
        // The departure uses the side facing the target, the arrival the
        // side facing the source — a downward wire leaves the source's
        // bottom and lands on the target's top.
        let (x1, y1) = terminal(pa, down);
        let (mut x2, mut y2) = terminal(pb, !down);
        // A backfeed from outside the board (the PV inverter onto its
        // way) enters the BUSBAR from above at the way's column (todo:
        // inverter incomer above the busbar) — the bar's top edge, not
        // the below-bar tap.
        let from_board = members_of.get(a.as_str()).copied();
        if ir.circuits.contains_key(&b)
            && from_board != ir.circuits.get(&b).map(|c| c.board.as_str())
        {
            if let Some(bar) = ir
                .circuits
                .get(&b)
                .and_then(|c| ir.boards.get(&c.board))
                .and_then(|bd| bd.sections.first())
                .and_then(|s| layout.places.get(s))
            {
                x2 = x2.clamp(bar.x, bar.x + bar.w);
                y2 = bar.y;
            }
        }
        // A busbar tap is perpendicular and ON the bar (guidance
        // §2.2/§3.1): the wire meets it vertically, from directly above
        // or below the thing it feeds when that column crosses the bar,
        // else above the nearest point of the bar — never running along
        // it, never off its end.
        let clamp_to = |p: &Place, x: f64| x.clamp(p.x, p.x + p.w);
        // Same-band peers (an electrode beside its board) connect via
        // their SIDE terminals with a horizontal elbow — the bond exits
        // the frame edge instead of diving through the drawing. Only
        // when their vertical extents overlap: a wide busbar's centre
        // sits far from its tap, which must not read as "horizontal".
        let horizontal = pa.glyph != Glyph::Section && pb.glyph != Glyph::Section && {
            let (acx, acy) = pa.center();
            let (bcx, bcy) = pb.center();
            (bcy - acy).abs() < (pa.h + pb.h) / 2.0 + 1.0 && (bcx - acx).abs() > (bcy - acy).abs()
        };
        let (x1, x2) = match (pa.glyph, pb.glyph) {
            (Glyph::Section, _) => {
                let t = clamp_to(pa, x2);
                (t, x2)
            }
            (_, Glyph::Section) => {
                let t = clamp_to(pb, x1);
                (x1, t)
            }
            _ => (x1, x2),
        };
        // Vertical—horizontal—vertical: both ends are approached along
        // the terminals' axis, so bus taps and cell chains read as
        // straight drops wherever the columns align. A feed between two
        // boards runs its horizontal in the clear channel BETWEEN the
        // frames — never through a board.
        let board_of = |t: &str| -> Option<&str> { members_of.get(t).copied() };
        if horizontal {
            let right = pb.center().0 >= pa.center().0;
            let side = |p: &Place, r: bool| {
                let cy = p.center().1;
                if r { (p.x + p.w, cy) } else { (p.x, cy) }
            };
            let (x1, y1) = side(pa, right);
            let (x2, y2) = side(pb, !right);
            let xm = (x1 + x2) / 2.0;
            layout.routes.push(Route {
                points: vec![(x1, y1), (xm, y1), (xm, y2), (x2, y2)],
                dashed: edge.arrow == busbar_syntax::ast::Arrow::Peer,
                from_tag: a,
                to_tag: b,
            });
            continue;
        }
        let ym = match (board_of(&a), board_of(&b)) {
            (Some(ba), Some(bb)) if ba != bb => {
                let frame = |t: &str| layout.places.get(t).map(|p| (p.y, p.y + p.h));
                let (upper, lower) = if y1 < y2 { (ba, bb) } else { (bb, ba) };
                let channel = match (frame(upper), frame(lower)) {
                    (Some((_, ub)), Some((lt, _))) => (ub + lt) / 2.0,
                    _ => (y1 + y2) / 2.0,
                };
                channel.clamp(y1.min(y2), y1.max(y2))
            }
            _ => (y1 + y2) / 2.0,
        };
        layout.routes.push(Route {
            points: vec![(x1, y1), (x1, ym), (x2, ym), (x2, y2)],
            dashed: edge.arrow == busbar_syntax::ast::Arrow::Peer,
            from_tag: a,
            to_tag: b,
        });
    }
    // Same-side relief, terminal-strip style: when a departure and an
    // arrival share a terminal point (a tie breaker joining two bars
    // that both sit above it, say), seat the departure 10px right of
    // centre and the arrival 10px left — distinct connection points on
    // the same lead. Only endpoint x moves, so elbows stay orthogonal.
    {
        let mut endpoints: Vec<(usize, bool, (f64, f64), String)> = Vec::new();
        for (i, route) in layout.routes.iter().enumerate() {
            if let Some(&p) = route.points.first() {
                endpoints.push((i, true, p, route.from_tag.clone()));
            }
            if let Some(&p) = route.points.last() {
                endpoints.push((i, false, p, route.to_tag.clone()));
            }
        }
        // Shared endpoints branch from one trunk rail (guidance §2.2):
        // every route leaving (or arriving at) the same terminal shares
        // the first member's mid-y, so the group draws as a single
        // horizontal rail with perpendicular drops instead of crossing
        // fans. Deterministic: groups keyed by (side, tag, point),
        // members in route order.
        fn key(is_start: bool, tag: &str, p: (f64, f64)) -> (bool, String, (u64, u64)) {
            (is_start, tag.to_owned(), (p.0.to_bits(), p.1.to_bits()))
        }
        // The rail hugs the shared terminal (8px out along the wires'
        // direction): every branch leaves perpendicular from one line
        // right at the source (or arrives into one line right at the
        // target), so the group's own drops never cross each other or
        // neighbouring columns.
        // Only terminals shared by two or more wires are junctions worth
        // a rail; a singleton keeps its channel/midpoint shape (hugging
        // it would drag inter-board feeds through foreign frames).
        let mut rail_y: BTreeMap<(bool, String, (u64, u64)), f64> = BTreeMap::new();
        let mut rail_n: BTreeMap<(bool, String, (u64, u64)), usize> = BTreeMap::new();
        for route in &layout.routes {
            if route.points[0].0 != route.points[1].0 {
                continue; // side-form route — no vertical rail
            }
            let going_down = route.points[3].1 >= route.points[0].1;
            let start_rail = if going_down {
                route.points[0].1 + 8.0
            } else {
                route.points[0].1 - 8.0
            };
            let end_rail = if going_down {
                route.points[3].1 - 8.0
            } else {
                route.points[3].1 + 8.0
            };
            let ks = key(true, &route.from_tag, route.points[0]);
            let ke = key(false, &route.to_tag, route.points[3]);
            rail_y.entry(ks.clone()).or_insert(start_rail);
            rail_n.entry(ks).and_modify(|n| *n += 1).or_insert(1);
            rail_y.entry(ke.clone()).or_insert(end_rail);
            rail_n.entry(ke).and_modify(|n| *n += 1).or_insert(1);
        }
        for route in &mut layout.routes {
            if route.points[0].0 != route.points[1].0 {
                continue; // side-form route — no vertical rail
            }
            // One rail per route: the departure group wins when both
            // terminals are shared (its members counted first).
            for (is_start, tag, end, elbow) in [
                (true, route.from_tag.clone(), 0, 1),
                (false, route.to_tag.clone(), 3, 2),
            ] {
                let k = key(is_start, &tag, route.points[end]);
                if rail_n.get(&k).is_some_and(|n| *n >= 2) {
                    if let Some(&yr) = rail_y.get(&k) {
                        route.points[1].1 = yr;
                        route.points[2].1 = yr;
                        route.points[elbow].0 = route.points[end].0;
                    }
                    break;
                }
            }
        }

        // Implicit bus attachments (SPD-style: no edges, attached by
        // declaration) get their wire drawn — a stub from the bar to
        // the device terminal (todo: SHED.SPD1 connects properly).
        for (device, section) in &implicit_taps {
            let already = layout
                .routes
                .iter()
                .any(|r| r.from_tag == *device || r.to_tag == *device);
            if already {
                continue;
            }
            let (Some(dp), Some(sp)) = (layout.places.get(device), layout.places.get(section))
            else {
                continue;
            };
            let down = dp.center().1 >= sp.center().1;
            let (_, dy1) = terminal(sp, down);
            let (dx2, dy2) = terminal(dp, !down);
            let x = dx2.clamp(sp.x, sp.x + sp.w);
            let ym = (dy1 + dy2) / 2.0;
            layout.routes.push(Route {
                points: vec![(x, dy1), (x, ym), (x, ym), (x, dy2)],
                dashed: false,
                from_tag: section.clone(),
                to_tag: device.clone(),
            });
        }

        // Bus-tap legibility (review round): a wire arriving on a bar
        // from ABOVE must not align with one leaving BELOW — the pair
        // reads as a single wire bypassing the bar. Later taps nudge
        // 12px sideways off opposite-side taps (deterministic order).
        {
            let mut taps: BTreeMap<String, (Vec<f64>, Vec<f64>)> = BTreeMap::new();
            for route in &mut layout.routes {
                let mut ends: Vec<(usize, &str)> = Vec::new();
                if let Some(p) = layout.places.get(&route.from_tag) {
                    if p.glyph == Glyph::Section {
                        ends.push((0, &route.from_tag));
                    }
                }
                if let Some(p) = layout.places.get(&route.to_tag) {
                    if p.glyph == Glyph::Section {
                        ends.push((3, &route.to_tag));
                    }
                }
                for (idx, tag) in ends {
                    let p = layout.places.get(tag).expect("section place");
                    let above = route.points[idx].1 < p.center().1;
                    let entry = taps.entry(tag.to_string()).or_default();
                    let (mine, other) = if above {
                        (&mut entry.0, &entry.1)
                    } else {
                        (&mut entry.1, &entry.0)
                    };
                    let mut x = route.points[idx].0;
                    if other.iter().any(|ox| (x - ox).abs() < 8.0) {
                        let shifted = x + 12.0;
                        if shifted <= p.x + p.w - 2.0 {
                            x = shifted;
                        } else {
                            x = (x - 12.0).max(p.x + 2.0);
                        }
                    }
                    route.points[idx].0 = x;
                    let elbow = if idx == 0 { 1 } else { 2 };
                    route.points[elbow].0 = x;
                    mine.push(x);
                }
            }
        }

        let mut handled = std::collections::BTreeSet::new();
        for i in 0..endpoints.len() {
            if handled.contains(&i) {
                continue;
            }
            for j in 0..endpoints.len() {
                if handled.contains(&j) {
                    continue;
                }
                let (ia, sa, pa, ta) = &endpoints[i];
                let (ib, sb, pb, tb) = &endpoints[j];
                if ia == ib || ta != tb || !(*sa && !*sb) || pa != pb {
                    continue;
                }
                let vertical_a = layout.routes[*ia].points[0].0 == layout.routes[*ia].points[1].0;
                layout.routes[*ia].points[0].0 += 10.0;
                if vertical_a {
                    layout.routes[*ia].points[1].0 += 10.0;
                }
                let vertical_b = layout.routes[*ib].points[3].0 == layout.routes[*ib].points[2].0;
                layout.routes[*ib].points[3].0 -= 10.0;
                if vertical_b {
                    layout.routes[*ib].points[2].0 -= 10.0;
                }
                handled.insert(i);
                handled.insert(j);
                break;
            }
        }
    }

    layout
}

/// Wire terminal on a place: the end of the glyph's lead (or the frame
/// edge for containers), on the vertical centreline. Extents follow the
/// reference sheet's lead lengths (corpus/render/symbols.svg): blades
/// and boxes terminate at ±20, the fuse at ±30, the bar at its own
/// half-height, junctions at their dot.
/// Vertical distance from a place's centre to where a wire meets its
/// glyph — the glyph's own geometry, so wires neither float short nor
/// overlap marks. Shared by terminal() and the hanger-chain stacking.
pub fn extent(g: Glyph) -> f64 {
    match g {
        Glyph::Junction => 0.0,
        Glyph::Section | Glyph::Board => 20.0, // callers use p.h/2 for these
        Glyph::Fuse | Glyph::Meter | Glyph::Ct | Glyph::Pv => 30.0,
        Glyph::Earth => 15.0,
        Glyph::Lamp => 10.0,
        Glyph::Motor => 15.0,
        Glyph::Socket => 15.0,
        Glyph::Heating => 15.0,
        Glyph::Load => 22.0,
        Glyph::Battery => 6.0,
        // Box glyphs: their rectangles end well inside the cell — a 20px
        // default left wires overlapping the marks (todo item).
        Glyph::Relay => 11.0,
        Glyph::Spd => 12.0,
        Glyph::Evse => 21.0,
        Glyph::Generator | Glyph::WindTurbine | Glyph::Inverter => 20.0,
        _ => 14.0, // generic box family
    }
}

pub fn terminal(p: &Place, lower: bool) -> (f64, f64) {
    let (cx, cy) = p.center();
    let d = match p.glyph {
        Glyph::Junction => 0.0,
        Glyph::Section | Glyph::Board => p.h / 2.0,
        _ => extent(p.glyph),
    };
    (cx, if lower { cy + d } else { cy - d })
}

/// Shifts a board's placed members (relative to the board's local origin)
/// by the board's global origin. The member list is explicit because
/// loads carry plain tags that prefix matching cannot find.
fn offset_group(layout: &mut Layout, members: &[String], dx: f64, dy: f64) {
    for tag in members {
        if let Some(place) = layout.places.get_mut(tag) {
            place.x += dx;
            place.y += dy;
        }
    }
}

fn short_tag(tag: &str) -> String {
    // rsplit yields segments right-to-left; next() is the final segment
    // (the circuit tag), next_back() was the board tag.
    tag.rsplit('.').next().unwrap_or(tag).to_owned()
}

/// Label for a placed glyph: the user-facing `name` property when the
/// document provides one ("Bath 1"), else the short tag.
fn display_label(tag: &str, props: &[busbar_syntax::ast::Property]) -> String {
    use busbar_syntax::ast::Value;
    if let Some(p) = props.iter().find(|p| p.name == "name") {
        if let Value::Str(s) = &p.value.value {
            if !s.is_empty() {
                return s.clone();
            }
        }
    }
    short_tag(tag)
}

/// Directed hop distance from a board device along its power path to
/// the nearest section of `board`; `None` when no directed path exists
/// (the device does not feed the bus — an outgoing way, a bus-hung
/// relay). Board-local only: paths crossing out of the board end.
fn directed_hops_to_section(ir: &Ir, tag: &str, board: &busbar_ir::Board) -> Option<u32> {
    use std::collections::VecDeque;
    let mut dist: BTreeMap<String, u32> = BTreeMap::from([(tag.to_owned(), 0)]);
    let mut queue: VecDeque<String> = VecDeque::from([tag.to_owned()]);
    while let Some(t) = queue.pop_front() {
        let d = dist[&t];
        if board.sections.iter().any(|s| s == &t) {
            return Some(d);
        }
        for e in &ir.edges {
            let Some((from_e, _, _)) = ir.resolve_endpoint(&e.from) else {
                continue;
            };
            if from_e != t {
                continue;
            }
            let Some((to_e, _, owner)) = ir.resolve_endpoint(&e.to) else {
                continue;
            };
            if owner.as_deref().is_some_and(|o| o != board.tag) {
                continue; // leaves the board — not this bar's incomer chain
            }
            // Feeding the board's own `bus`/`in` port is feeding its
            // section (spec §9.2) — descend through the container.
            if to_e == board.tag {
                for s in &board.sections {
                    if !dist.contains_key(s) {
                        dist.insert(s.clone(), d + 1);
                        queue.push_back(s.clone());
                    }
                }
                continue;
            }
            if !dist.contains_key(&to_e) {
                dist.insert(to_e.clone(), d + 1);
                queue.push_back(to_e);
            }
        }
    }
    None
}

/// Voltage-system summary for a board's label strip (guidance §4.5).
fn voltsys_note(ir: &Ir, board_tag: &str) -> Option<String> {
    let name = ir.board_vs(board_tag)?;
    let vs = ir.voltsys.get(&name)?;
    let phases = match vs.phases {
        busbar_ir::PhaseStyle::Dc => "dc".to_owned(),
        busbar_ir::PhaseStyle::Single => "1ph".to_owned(),
        busbar_ir::PhaseStyle::Split => "2ph".to_owned(),
        busbar_ir::PhaseStyle::Three => "3ph".to_owned(),
        busbar_ir::PhaseStyle::Multi(n) => format!("{n}ph"),
    };
    let v = if (vs.nominal_v - vs.nominal_v.round()).abs() < 1e-9 {
        format!("{}", vs.nominal_v.round() as i64)
    } else {
        format!("{}", vs.nominal_v)
    };
    let mut s = format!("{v}V {phases}");
    if let Some(f) = vs.frequency_hz {
        s.push_str(&format!(" {f:.0}Hz"));
    }
    Some(s)
}

fn rating_note(props: &[busbar_syntax::ast::Property]) -> Option<String> {
    use busbar_syntax::ast::Value;
    let mut parts = Vec::new();
    for name in ["rating_a", "kw", "kvar", "rcd_ma"] {
        if let Some(p) = props.iter().find(|p| p.name == name) {
            let text = match &p.value.value {
                Value::Quantity { number, unit } => format!("{number}{unit}"),
                Value::Number(n) => n.clone(),
                Value::Ident(s) => s.clone(),
                _ => continue,
            };
            parts.push(text);
        }
    }
    for name in ["manufacturer", "model"] {
        if let Some(p) = props.iter().find(|p| p.name == name) {
            if let Value::Str(s) = &p.value.value {
                if !s.is_empty() {
                    parts.push(s.clone());
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" "))
    }
}

fn rank_from_sources(ir: &Ir) -> BTreeMap<String, u32> {
    use std::collections::VecDeque;
    let mut ranks: BTreeMap<String, u32> = BTreeMap::new();
    let mut queue = VecDeque::new();
    for node in ir.nodes.values() {
        if node.kind == Some(NodeKind::Source) {
            ranks.insert(node.tag.clone(), 0);
            queue.push_back(node.tag.clone());
        }
    }
    while let Some(tag) = queue.pop_front() {
        let d = ranks[&tag];
        for edge in &ir.edges {
            let (Some((a, _, _)), Some((b, _, _))) = (
                ir.resolve_endpoint(&edge.from),
                ir.resolve_endpoint(&edge.to),
            ) else {
                continue;
            };
            let other = if a == tag {
                b
            } else if b == tag {
                a
            } else {
                continue;
            };
            if !ranks.contains_key(&other) {
                ranks.insert(other.clone(), d + 1);
                queue.push_back(other);
            }
        }
    }
    ranks
}
