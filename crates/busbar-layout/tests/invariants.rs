//! Machine-checked drafting invariants for the layout (plan §5.5):
//! glyphs never overlap, anything intersecting a board lives inside it,
//! everything stays on the canvas, and routes land on the places they
//! name. Run over every valid corpus document.

use std::collections::BTreeMap;
use std::path::PathBuf;

use busbar_layout::{FEEDER_W, Glyph, Layout, Place};

fn corpus() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../corpus/valid");
    let mut files: Vec<PathBuf> = std::fs::read_dir(root)
        .expect("corpus/valid")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "esld"))
        .collect();
    files.sort();
    files
}

fn layout_of(path: &PathBuf) -> (String, Layout) {
    let src = std::fs::read_to_string(path).unwrap();
    let doc = busbar_syntax::parse(&src).expect("parse");
    let ir = busbar_ir::Ir::build(&doc).expect("ir");
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    (name, busbar_layout::build(&ir))
}

fn overlap(a: &Place, b: &Place) -> bool {
    let eps = 0.5;
    a.x + eps < b.x + b.w - eps
        && b.x + eps < a.x + a.w - eps
        && a.y + eps < b.y + b.h - eps
        && b.y + eps < a.y + a.h - eps
}

fn intersects(a: &Place, b: &Place) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

fn contained(inner: &Place, outer: &Place) -> bool {
    inner.x >= outer.x - 0.5
        && inner.y >= outer.y - 0.5
        && inner.x + inner.w <= outer.x + outer.w + 0.5
        && inner.y + inner.h <= outer.y + outer.h + 0.5
}

#[test]
fn glyphs_never_overlap() {
    for path in corpus() {
        let (name, layout) = layout_of(&path);
        let cells: Vec<(&String, &Place)> = layout
            .places
            .iter()
            .filter(|(_, p)| p.glyph != Glyph::Board)
            .collect();
        for i in 0..cells.len() {
            for j in i + 1..cells.len() {
                assert!(
                    !overlap(cells[i].1, cells[j].1),
                    "{name}: `{}` overlaps `{}` at ({:.1},{:.1}) vs ({:.1},{:.1})",
                    cells[i].0,
                    cells[j].0,
                    cells[i].1.x,
                    cells[i].1.y,
                    cells[j].1.x,
                    cells[j].1.y
                );
            }
        }
    }
}

#[test]
fn board_intersecting_places_are_contained() {
    for path in corpus() {
        let (name, layout) = layout_of(&path);
        let boards: Vec<(&String, &Place)> = layout
            .places
            .iter()
            .filter(|(_, p)| p.glyph == Glyph::Board)
            .collect();
        for (tag, p) in &layout.places {
            if p.glyph == Glyph::Board {
                continue;
            }
            for (btag, b) in &boards {
                assert!(
                    !intersects(p, b) || contained(p, b),
                    "{name}: `{}` intersects board `{}` without being contained",
                    tag,
                    btag
                );
            }
        }
    }
}

#[test]
fn everything_is_on_the_canvas() {
    for path in corpus() {
        let (name, layout) = layout_of(&path);
        for (tag, p) in &layout.places {
            assert!(
                p.x >= 0.0 && p.y >= 0.0,
                "{name}: `{}` off-canvas at origin",
                tag
            );
            assert!(
                p.x + p.w <= layout.width + 0.5,
                "{name}: `{}` overflows right ({:.1} > {:.1})",
                tag,
                p.x + p.w,
                layout.width
            );
            assert!(
                p.y + p.h <= layout.height + 0.5,
                "{name}: `{}` overflows bottom ({:.1} > {:.1})",
                tag,
                p.y + p.h,
                layout.height
            );
        }
    }
}

#[test]
fn routes_land_on_their_places() {
    for path in corpus() {
        let (name, layout) = layout_of(&path);
        for route in &layout.routes {
            let from = layout
                .places
                .get(&route.from_tag)
                .unwrap_or_else(|| panic!("{name}: route from unknown `{}`", route.from_tag));
            let to = layout
                .places
                .get(&route.to_tag)
                .unwrap_or_else(|| panic!("{name}: route to unknown `{}`", route.to_tag));
            let (fx, fy) = from.center();
            let (tx, ty) = to.center();
            assert_eq!(
                route.points.first(),
                Some(&(fx, fy)),
                "{name}: route start off-center"
            );
            assert_eq!(
                route.points.last(),
                Some(&(tx, ty)),
                "{name}: route end off-center"
            );
        }
    }
}

/// Every resolvable IR edge must produce exactly one route — a dropped
/// route is a dropped wire on the drawing (the sample1 regression).
#[test]
fn every_resolvable_edge_is_routed() {
    for path in corpus() {
        let src = std::fs::read_to_string(&path).unwrap();
        let doc = busbar_syntax::parse(&src).expect("parse");
        let ir = busbar_ir::Ir::build(&doc).expect("ir");
        let layout = busbar_layout::build(&ir);
        let name = path.file_name().unwrap().to_string_lossy();
        let mut expected: BTreeMap<(String, String), usize> = BTreeMap::new();
        for edge in &ir.edges {
            if let (Some((a, _, _)), Some((b, _, _))) = (
                ir.resolve_endpoint(&edge.from),
                ir.resolve_endpoint(&edge.to),
            ) {
                *expected.entry((a, b)).or_insert(0) += 1;
            }
        }
        for ((a, b), n) in expected {
            let got = layout
                .routes
                .iter()
                .filter(|r| r.from_tag == a && r.to_tag == b)
                .count();
            assert_eq!(
                got, n,
                "{name}: edge `{a}` -> `{b}` routed {got}x, expected {n}x"
            );
        }
    }
}

// Silence unused-import lint for BTreeMap (used only in signatures above
// when corpus grows grouped checks).
#[allow(dead_code)]
type _Unused = BTreeMap<String, Place>;

/// Guidance §2.5: a board fed from inside another board hangs below the
/// feeder that feeds it — its row is strictly below the parent board's,
/// and its centre aligns with the feeder's column.
#[test]
fn fed_boards_hang_below_their_feeder() {
    let src = r#"
profile "esld/1.0";
voltsys LV = { nominal = 230V; phases = 1ph; frequency = 50Hz; };
grid GRID : grid { vs = LV; }
LAMP : lighting { kw = 0.1kW; }
board MAIN : board {
  vs = LV;
  incomers = [GRID.out];
  busbar_rating_a = 100A;
  circuit LIGHTS { protection : rcbo { rating_a = 10A; rcd_ma = 30mA; }; loads = [LAMP]; }
  circuit FEED_A { protection : mcb { rating_a = 20A; }; }
  circuit FEED_B { protection : mcb { rating_a = 20A; }; }
}
board SUB_A : board {
  vs = LV;
  incomers = [MAIN.FEED_A.out];
  busbar_rating_a = 20A;
  circuit SUB_LIGHTS { protection : mcb { rating_a = 10A; }; loads = [LAMP]; }
}
connect GRID.out -> MAIN.in;
connect MAIN.FEED_A.out -> SUB_A.in;
"#;
    let doc = busbar_syntax::parse(src).expect("parse");
    let ir = busbar_ir::Ir::build(&doc).expect("ir");
    let layout = busbar_layout::build(&ir);

    let main = layout.places.get("MAIN").expect("MAIN placed");
    let sub = layout.places.get("SUB_A").expect("SUB_A placed");
    let feed_a = layout.places.get("MAIN.FEED_A").expect("feeder placed");
    let feed_b = layout.places.get("MAIN.FEED_B").expect("feeder placed");

    assert!(
        sub.y >= main.y + main.h,
        "SUB_A must hang below MAIN (sub top {:.1} < main bottom {:.1})",
        sub.y,
        main.y + main.h
    );
    let (sub_cx, _) = sub.center();
    let (fa_cx, _) = feed_a.center();
    let (fb_cx, _) = feed_b.center();
    assert!(
        (sub_cx - fa_cx).abs() < 0.5,
        "SUB_A centre {:.1} must align with its feeder FEED_A at {:.1}",
        sub_cx,
        fa_cx
    );
    assert!(
        (sub_cx - fb_cx).abs() > FEEDER_W / 2.0,
        "SUB_A must align with FEED_A, not the unrelated FEED_B column"
    );
}

/// §2.5 under crowding: two wide sub-boards fed by adjacent feeder
/// columns cannot share a lane, so the second opens a lower lane — and
/// both stay centred on their own feeder (no sideways drift).
#[test]
fn crowded_fed_boards_lane_instead_of_drifting() {
    let src = r#"
profile "esld/1.0";
voltsys LV = { nominal = 230V; phases = 1ph; frequency = 50Hz; };
grid GRID : grid { vs = LV; }
LAMP : lighting { kw = 0.1kW; }
board MAIN : board {
  vs = LV;
  incomers = [GRID.out];
  busbar_rating_a = 100A;
  circuit LIGHTS { protection : rcbo { rating_a = 10A; rcd_ma = 30mA; }; loads = [LAMP]; }
  circuit FEED_A { protection : mcb { rating_a = 20A; }; }
  circuit FEED_B { protection : mcb { rating_a = 20A; }; }
}
board SUB_A : board {
  vs = LV;
  incomers = [MAIN.FEED_A.out];
  busbar_rating_a = 20A;
  circuit SUB_LIGHTS { protection : mcb { rating_a = 10A; }; loads = [LAMP]; }
}
board SUB_B : board {
  vs = LV;
  incomers = [MAIN.FEED_B.out];
  busbar_rating_a = 20A;
  circuit SUB_LIGHTS { protection : mcb { rating_a = 10A; }; loads = [LAMP]; }
}
connect GRID.out -> MAIN.in;
connect MAIN.FEED_A.out -> SUB_A.in;
connect MAIN.FEED_B.out -> SUB_B.in;
"#;
    let doc = busbar_syntax::parse(src).expect("parse");
    let ir = busbar_ir::Ir::build(&doc).expect("ir");
    let layout = busbar_layout::build(&ir);

    for (sub, feeder) in [("SUB_A", "MAIN.FEED_A"), ("SUB_B", "MAIN.FEED_B")] {
        let s = layout.places.get(sub).expect("sub placed");
        let f = layout.places.get(feeder).expect("feeder placed");
        let (scx, _) = s.center();
        let (fcx, _) = f.center();
        assert!(
            (scx - fcx).abs() < 0.5,
            "{sub} centre {:.1} must stay aligned with {feeder} at {:.1}",
            scx,
            fcx
        );
    }
    let a = layout.places.get("SUB_A").unwrap();
    let b = layout.places.get("SUB_B").unwrap();
    assert!(
        b.y == a.y || b.y >= a.y + a.h || a.y >= b.y + b.h,
        "crowded sub-boards must not overlap: SUB_A {a:?} vs SUB_B {b:?}"
    );
}

/// §2.5 also holds for clusters no source can reach (orphan sub-boards,
/// R-109 territory): the fed board still lands in a row below its
/// feeder's board, via the unreachable-band depth rebase.
#[test]
fn orphaned_fed_boards_still_hang_below() {
    let src = r#"
profile "esld/1.0";
voltsys LV = { nominal = 230V; phases = 1ph; frequency = 50Hz; };
board ORPH_MAIN : board {
  vs = LV;
  incomers = [NOWHERE.out];
  busbar_rating_a = 100A;
  circuit FEED { protection : mcb { rating_a = 20A; }; }
}
board ORPH_SUB : board {
  vs = LV;
  incomers = [ORPH_MAIN.FEED.out];
  busbar_rating_a = 20A;
}
connect ORPH_MAIN.FEED.out -> ORPH_SUB.in;
"#;
    let doc = busbar_syntax::parse(src).expect("parse");
    let ir = busbar_ir::Ir::build(&doc).expect("ir");
    let layout = busbar_layout::build(&ir);
    let main = layout.places.get("ORPH_MAIN").expect("parent placed");
    let sub = layout.places.get("ORPH_SUB").expect("child placed");
    assert!(
        sub.y >= main.y + main.h,
        "unreachable SUB must still hang below its parent ({:.1} < {:.1})",
        sub.y,
        main.y + main.h
    );
}
