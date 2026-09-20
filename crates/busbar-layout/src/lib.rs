//! Deterministic layered layout (spec §16.1; implementation plan §5.5).
//!
//! Two-level layout: top-level entities (sources, converters, boards,
//! standalone loads) are ranked by BFS distance from sources and stacked
//! in columns; board internals are schedule-style — bus sections as
//! horizontal bars, circuits as rows of protection/controller/load cells
//! under their section. Every ordering key ends in the (unique) tag, so
//! output is bit-stable for identical input.

use std::collections::BTreeMap;

use busbar_ir::{Ir, NodeKind};

pub const CELL: f64 = 40.0;
pub const COL_W: f64 = 160.0;
pub const MARGIN: f64 = 40.0;
pub const V_GAP: f64 = 48.0;
pub const BOARD_PAD: f64 = 28.0;
pub const SECTION_GAP: f64 = 64.0;
pub const ROW_H: f64 = 56.0;
pub const BAR_W: f64 = 240.0;

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

    // -- Board internals first: sizes feed the global pass. ---------------
    // Board member tags (sections, protection, controller, loads) per
    // board — loads carry plain tags, so prefix matching can't find them.
    let mut members: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for board in ir.boards.values() {
        let mut y = BOARD_PAD;
        let mut max_w = BAR_W;
        let member_list = members.entry(board.tag.clone()).or_default();
        let mut section_y: BTreeMap<String, f64> = BTreeMap::new();
        for section in &board.sections {
            section_y.insert(section.clone(), y);
            member_list.push(section.clone());
            layout.places.insert(
                section.clone(),
                Place {
                    x: BOARD_PAD,
                    y,
                    w: BAR_W,
                    h: 8.0,
                    label: section.rsplit('.').next().unwrap_or(section).to_owned(),
                    glyph: Glyph::Section,
                    note: None,
                },
            );
            y += SECTION_GAP;
        }
        // Circuit rows, grouped under their (or the first) section.
        let mut row_y: BTreeMap<String, f64> = BTreeMap::new();
        for section in &board.sections {
            row_y.insert(section.clone(), y);
        }
        for ctag in &board.circuits {
            let Some(circuit) = ir.circuits.get(ctag) else {
                continue;
            };
            let section = circuit
                .section
                .clone()
                .or_else(|| board.sections.first().cloned());
            let Some(section) = section else { continue };
            let ry = *row_y.get(&section).unwrap_or(&y);
            let mut x = BOARD_PAD + 12.0;
            let board_tag = board.tag.clone();
            let mut place_cell = |tag: &str, label: &str, glyph: Glyph, note: Option<String>| {
                members
                    .entry(board_tag.clone())
                    .or_default()
                    .push(tag.to_owned());
                layout.places.insert(
                    tag.to_owned(),
                    Place {
                        x,
                        y: ry,
                        w: CELL,
                        h: CELL,
                        label: label.to_owned(),
                        glyph,
                        note,
                    },
                );
                x += CELL + 16.0;
            };
            if let Some(prot) = &circuit.protection {
                place_cell(
                    &prot.tag,
                    &short_tag(ctag),
                    glyph_for(&prot.type_name, prot.kind),
                    rating_note(&prot.props),
                );
            }
            if let Some(ctl) = &circuit.controller {
                place_cell(
                    &ctl.tag,
                    &short_tag(ctag),
                    glyph_for(&ctl.type_name, ctl.kind),
                    None,
                );
            }
            for (load, _) in &circuit.loads {
                if let Some(node) = ir.nodes.get(load) {
                    place_cell(
                        load,
                        load,
                        glyph_for(&node.type_name, node.kind),
                        rating_note(&node.props),
                    );
                } else {
                    place_cell(load, load, Glyph::Generic, None);
                }
            }
            max_w = max_w.max(x - BOARD_PAD + BOARD_PAD);
            if let Some(ry_mut) = row_y.get_mut(&section) {
                *ry_mut += ROW_H;
            }
            y = y.max(*row_y.get(&section).unwrap_or(&y));
        }
        let board_w = max_w + BOARD_PAD;
        let board_h = y + BOARD_PAD;
        layout.places.insert(
            board.tag.clone(),
            Place {
                x: 0.0,
                y: 0.0, // final origin set in the global pass
                w: board_w,
                h: board_h,
                label: board.tag.clone(),
                glyph: Glyph::Board,
                note: None,
            },
        );
        // Offset internals by the board origin placeholder for now; the
        // global pass translates the whole group.
        layout.boards.insert(board.tag.clone(), (board_w, board_h));
    }

    // -- Global pass: rank top-level entities from sources. ----------------
    let ranks = rank_from_sources(ir);
    let in_circuit: std::collections::BTreeSet<String> = ir
        .circuits
        .values()
        .flat_map(|c| c.loads.iter().map(|(l, _)| l.clone()))
        .collect();

    let mut columns: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for (tag, node) in &ir.nodes {
        if node.parent.is_some() || tag.ends_with(".protection") || tag.ends_with(".controller") {
            continue; // board members and inline devices live inside boards
        }
        if in_circuit.contains(tag) {
            continue; // loads are drawn in their circuit's row
        }
        let rank = ranks.get(tag).copied().unwrap_or(u32::MAX);
        columns.entry(rank).or_default().push(tag.clone());
    }
    // Sections/circuits never appear at top level.

    let mut cur_x = MARGIN;
    let mut max_bottom = MARGIN;
    for (_rank, mut tags) in columns {
        tags.sort(); // within-column order: tag (unique) => total order
        let mut cur_y = MARGIN;
        let mut col_w = CELL;
        for tag in &tags {
            let (w, h) = layout.boards.get(tag).copied().unwrap_or((CELL, CELL));
            let node = ir.nodes.get(tag);
            let place = Place {
                x: cur_x,
                y: cur_y,
                w,
                h,
                label: tag.clone(),
                glyph: node
                    .map(|n| glyph_for(&n.type_name, n.kind))
                    .unwrap_or(Glyph::Generic),
                note: node.and_then(|n| rating_note(&n.props)),
            };
            let member_list = members.get(tag).cloned().unwrap_or_default();
            offset_group(&mut layout, &member_list, cur_x, cur_y);
            layout.places.insert(tag.clone(), place);
            cur_y += h + V_GAP;
            col_w = col_w.max(w);
        }
        max_bottom = max_bottom.max(cur_y);
        cur_x += col_w + COL_W;
    }

    layout.width = cur_x.max(MARGIN * 2.0);
    layout.height = max_bottom.max(MARGIN * 2.0);

    // -- Routes: one elbow per edge, deterministic order. -------------------
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
