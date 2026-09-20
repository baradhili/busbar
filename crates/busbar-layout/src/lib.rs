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
pub const BAR_W_MIN: f64 = 200.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Board,
    Section,
    Source,
    Transformer,
    Switch,
    Protective,
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

        let n_feeders = feeders.values().map(Vec::len).max().unwrap_or(0);
        let inner_w = FEEDER_W * n_feeders.max(1) as f64;
        let strip_w = |n: usize| {
            if n == 0 {
                0.0
            } else {
                n as f64 * (CELL + STACK_GAP) - STACK_GAP
            }
        };
        let bar_w = inner_w
            .max(strip_w(upstream_devs.len()))
            .max(strip_w(downstream_devs.len()))
            .max(BAR_W_MIN);

        let mut y = BOARD_PAD + 22.0; // label strip inside the board
        if !upstream_devs.is_empty() {
            let mut dx = BOARD_PAD;
            for node in &upstream_devs {
                member_list.push(node.tag.clone());
                layout.places.insert(
                    node.tag.clone(),
                    Place {
                        x: dx,
                        y,
                        w: CELL,
                        h: CELL,
                        label: display_label(&node.tag, &node.props),
                        glyph: glyph_for(&node.type_name, node.kind),
                        note: rating_note(&node.props),
                    },
                );
                dx += CELL + STACK_GAP;
            }
            y += CELL + STACK_GAP;
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
                for (fi, ctag) in ctags.iter().enumerate() {
                    let fx = BOARD_PAD + fi as f64 * FEEDER_W + (FEEDER_W - CELL) / 2.0;
                    // Circuit anchor: a junction dot at the feeder's bar
                    // tap. Ports on the circuit itself (`connect X ->
                    // CIRCUIT.out`, PV-backfeed style) land here instead of
                    // dropping off the drawing.
                    member_list.push((*ctag).clone());
                    layout.places.insert(
                        (*ctag).clone(),
                        Place {
                            x: fx + CELL / 2.0,
                            y,
                            w: 0.0,
                            h: 0.0,
                            label: String::new(),
                            glyph: Glyph::Junction,
                            note: None,
                        },
                    );
                    let mut cy = y;
                    let Some(circuit) = ir.circuits.get(*ctag) else {
                        continue;
                    };
                    let mut place_cell = |tag: &str,
                                          label: &str,
                                          glyph: Glyph,
                                          note: Option<String>,
                                          cy: &mut f64|
                     -> f64 {
                        member_list.push(tag.to_owned());
                        layout.places.insert(
                            tag.to_owned(),
                            Place {
                                x: fx,
                                y: *cy,
                                w: CELL,
                                h: CELL,
                                label: label.to_owned(),
                                glyph,
                                note,
                            },
                        );
                        *cy += CELL + STACK_GAP;
                        *cy - (CELL + STACK_GAP)
                    };
                    if let Some(prot) = &circuit.protection {
                        place_cell(
                            &prot.tag,
                            &short_tag(ctag),
                            glyph_for(&prot.type_name, prot.kind),
                            rating_note(&prot.props),
                            &mut cy,
                        );
                    }
                    if let Some(ctl) = &circuit.controller {
                        place_cell(
                            &ctl.tag,
                            &short_tag(ctag),
                            glyph_for(&ctl.type_name, ctl.kind),
                            None,
                            &mut cy,
                        );
                    }
                    for (load, _) in &circuit.loads {
                        let node = ir.nodes.get(load);
                        place_cell(
                            load,
                            &node
                                .map(|n| display_label(load, &n.props))
                                .unwrap_or_else(|| short_tag(load)),
                            node.map(|n| glyph_for(&n.type_name, n.kind))
                                .unwrap_or(Glyph::Generic),
                            node.and_then(|n| rating_note(&n.props)),
                            &mut cy,
                        );
                    }
                    depth = depth.max(cy - y);
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
    let ranks = rank_from_sources(ir);
    let in_circuit: std::collections::BTreeSet<String> = ir
        .circuits
        .values()
        .flat_map(|c| c.loads.iter().map(|(l, _)| l.clone()))
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
        // (spec §16.1 advisory); unadorned tags keep tag order after them.
        tags.sort_by_key(|t| {
            (
                ir.layout_columns.get(t).copied().unwrap_or(u64::MAX),
                t.clone(),
            )
        });
        let row_h = tags
            .iter()
            .filter_map(|t| layout.boards.get(t).map(|(_, h)| *h))
            .chain(std::iter::once(CELL))
            .fold(0.0f64, f64::max);
        let mut cur_x = MARGIN;
        for tag in &tags {
            let (w, h) = layout.boards.get(tag).copied().unwrap_or((CELL, CELL));
            let node = ir.nodes.get(tag);
            let member_list = members.get(tag).cloned().unwrap_or_default();
            offset_group(&mut layout, &member_list, cur_x, cur_y);
            layout.places.insert(
                tag.clone(),
                Place {
                    x: cur_x,
                    y: cur_y,
                    w,
                    h,
                    label: tag.clone(),
                    glyph: node
                        .map(|n| glyph_for(&n.type_name, n.kind))
                        .unwrap_or(Glyph::Generic),
                    note: node.and_then(|n| rating_note(&n.props)),
                },
            );
            cur_x += w + COL_GAP;
        }
        max_right = max_right.max(cur_x - COL_GAP);
        cur_y += row_h + ROW_GAP;
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
