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
    /// Residual-current device: rectangle with a diagonal.
    Rcd,
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
        "inverter" | "rectifier" | "ups" => Glyph::Inverter,
        "pv_array" | "pv_string" | "wind_turbine" => Glyph::Pv,
        "earth" => Glyph::Earth,
        "junction" => Glyph::Junction,
        "ats" | "changeover" => Glyph::Ats,
        "spd" => Glyph::Spd,
        "ct" | "vt" | "sync_check" | "meter" => Glyph::Meter,
        "motor" => Glyph::Motor,
        "lighting" => Glyph::Lamp,
        "socket" => Glyph::Socket,
        "disconnector" | "isolator" | "load_break_switch" => Glyph::Disconnector,
        "main_switch" => Glyph::MainSwitch,
        "contactor" => Glyph::Contactor,
        "rcd" | "rcbo" | "elcb" | "rccb" | "rcmcd" => Glyph::Rcd,
        "control_relay" => Glyph::Relay,
        "heating" => Glyph::Heating,
        "evse" => Glyph::Evse,
        "bus" | "busbar" => Glyph::Section,
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
        for node in ir.nodes.values() {
            if node.parent.as_deref() != Some(board.tag.as_str())
                || board.sections.contains(&node.tag)
                || node.tag.ends_with(".protection")
                || node.tag.ends_with(".controller")
            {
                continue;
            }
            let feeds_section = ir.edges.iter().any(|e| {
                let from = ir.resolve_endpoint(&e.from).map(|(t, _, _)| t);
                let to = ir.resolve_endpoint(&e.to).map(|(t, _, _)| t);
                from.as_deref() == Some(node.tag.as_str())
                    && to.is_some_and(|t| board.sections.iter().any(|s| s == &t))
            });
            if feeds_section {
                upstream_devs.push(node);
            } else {
                downstream_devs.push(node);
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
            let mut ordered = upstream_devs;
            ordered.sort_by_key(|n| (u32::MAX - hops_to_sections(ir, n, board), n.tag.clone()));
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
    let mut feeder_of: BTreeMap<String, (String, String)> = BTreeMap::new();
    for board in ir.boards.keys() {
        let mut candidates: Vec<(String, String)> = Vec::new();
        for edge in &ir.edges {
            let Some((to_entity, _, _)) = ir.resolve_endpoint(&edge.to) else {
                continue;
            };
            if to_entity != *board {
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
            feeder_of.insert(board.clone(), (owner, feeder));
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
                let Some((owner, _)) = feeder_of.get(*b) else {
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

    let in_circuit: std::collections::BTreeSet<String> = ir
        .circuits
        .values()
        .flat_map(|c| c.loads.iter().map(|(l, ..)| l.clone()))
        .collect();

    let mut rows: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (tag, node) in &ir.nodes {
        if node.parent.is_some() || tag.ends_with(".protection") || tag.ends_with(".controller") {
            continue;
        }
        if in_circuit.contains(tag) {
            continue;
        }
        let rank = ranks.get(tag).copied().unwrap_or(u32::MAX);
        rows.entry(rank).or_default().push(tag.clone());
    }

    let mut max_right = MARGIN;
    let mut cur_y = MARGIN;
    for (_rank, mut tags) in rows {
        // `layout { TAG { column = N; } }` hints fix left-to-right order
        // (spec §16.1 advisory); §2.5 feeder alignment is the natural
        // secondary order; the tag is the deterministic tiebreak.
        let desired_x = |tag: &String| -> f64 {
            feeder_of
                .get(tag)
                .and_then(|(_, feeder)| layout.places.get(feeder))
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
        // Feeder-centred lanes (guidance §2.5): an anchored board sits
        // centred under its feeder in the first lane with room; when
        // feeders crowd (two wide sub-boards under adjacent feeder
        // columns), the later board opens a lower lane of the same rank
        // row instead of drifting sideways off its feeder. Only the
        // canvas margin can still clamp the centre — geometry, not
        // policy. Flow entities (no feeder) always use the first lane.
        struct LaneSlot {
            tag: String,
            x: f64,
            w: f64,
            h: f64,
        }
        let mut lanes: Vec<(f64, Vec<LaneSlot>)> = vec![(MARGIN, Vec::new())];
        for tag in &tags {
            let (w, h) = layout.boards.get(tag).copied().unwrap_or((CELL, CELL));
            let desired = feeder_of
                .get(tag)
                .and_then(|(_, feeder)| layout.places.get(feeder))
                .map(|p| (p.center().0 - w / 2.0).max(MARGIN));
            match desired {
                Some(x) => {
                    if let Some((cur_x, slots)) = lanes.iter_mut().find(|(cx, _)| *cx <= x) {
                        slots.push(LaneSlot {
                            tag: tag.clone(),
                            x,
                            w,
                            h,
                        });
                        *cur_x = x + w + COL_GAP;
                    } else {
                        lanes.push((
                            x + w + COL_GAP,
                            vec![LaneSlot {
                                tag: tag.clone(),
                                x,
                                w,
                                h,
                            }],
                        ));
                    }
                }
                None => {
                    let (cur_x, slots) = &mut lanes[0];
                    let x = *cur_x;
                    slots.push(LaneSlot {
                        tag: tag.clone(),
                        x,
                        w,
                        h,
                    });
                    *cur_x = x + w + COL_GAP;
                }
            }
        }
        for (_, slots) in &lanes {
            let lane_h = slots
                .iter()
                .map(|s| s.h)
                .chain(std::iter::once(CELL))
                .fold(0.0f64, f64::max);
            for slot in slots {
                let node = ir.nodes.get(&slot.tag);
                let member_list = members.get(&slot.tag).cloned().unwrap_or_default();
                offset_group(&mut layout, &member_list, slot.x, cur_y);
                layout.places.insert(
                    slot.tag.clone(),
                    Place {
                        x: slot.x,
                        y: cur_y,
                        w: slot.w,
                        h: slot.h,
                        label: slot.tag.clone(),
                        glyph: node
                            .map(|n| glyph_for(&n.type_name, n.kind))
                            .unwrap_or(Glyph::Generic),
                        // Boards carry their voltage system in the label
                        // strip (guidance §4.5): "230V 1ph 50Hz".
                        note: if layout.boards.contains_key(&slot.tag) {
                            voltsys_note(ir, &slot.tag)
                        } else {
                            node.and_then(|n| rating_note(&n.props))
                        },
                    },
                );
                max_right = max_right.max(slot.x + slot.w);
            }
            cur_y += lane_h + ROW_GAP;
        }
    }

    layout.width = max_right + MARGIN;
    layout.height = cur_y - ROW_GAP + MARGIN;

    // -- Routes: center-to-center elbows, deterministic edge order. ------------
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
        let (x1, y1) = pa.center();
        let (x2, y2) = pb.center();
        let mx = (x1 + x2) / 2.0;
        layout.routes.push(Route {
            points: vec![(x1, y1), (mx, y1), (mx, y2), (x2, y2)],
            dashed: edge.arrow == busbar_syntax::ast::Arrow::Peer,
            from_tag: a,
            to_tag: b,
        });
    }
    layout
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

/// BFS hop distance from a board device to the nearest section across
/// resolved edges — orders the incomer column in power sequence.
fn hops_to_sections(ir: &Ir, node: &busbar_ir::IrNode, board: &busbar_ir::Board) -> u32 {
    use std::collections::VecDeque;
    let mut dist: BTreeMap<String, u32> = BTreeMap::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    for s in &board.sections {
        dist.insert(s.clone(), 0);
        queue.push_back(s.clone());
    }
    while let Some(t) = queue.pop_front() {
        let d = dist[&t];
        for e in &ir.edges {
            let (Some((a, _, _)), Some((b, _, _))) =
                (ir.resolve_endpoint(&e.from), ir.resolve_endpoint(&e.to))
            else {
                continue;
            };
            let other = if a == t {
                b
            } else if b == t {
                a
            } else {
                continue;
            };
            if !dist.contains_key(&other) {
                dist.insert(other.clone(), d + 1);
                queue.push_back(other);
            }
        }
    }
    dist.get(&node.tag).copied().unwrap_or(u32::MAX / 2)
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
