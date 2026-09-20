//! SVG renderer (spec §16.2; implementation plan §5.6).
//!
//! Symbols are drawn from vector primitives (IEC 60617 conventions) — no
//! imported artwork, per the licensing/determinism stance. Output is
//! byte-deterministic: BTreeMap iteration, tag-sorted order, one fixed
//! float formatter, and escaped text.

use std::fmt::Write as _;

use busbar_ir::Ir;
use busbar_layout::{Glyph, Place};

/// Renders an ESLD document's source text. `symbols` selects the registry;
/// only "iec" exists (ANSI lands with M8).
pub fn render_str(source: &str, symbols: &str) -> Result<String, String> {
    if symbols != "iec" {
        return Err(format!(
            "symbol registry {symbols:?} not available (ANSI lands with M8)"
        ));
    }
    let doc = busbar_syntax::parse_or_string(source)?;
    let ir = Ir::build(&doc)?;
    Ok(render_ir(&ir))
}

pub fn render_ir(ir: &Ir) -> String {
    let layout = busbar_layout::build(ir);
    let mut svg = String::new();
    let _ = writeln!(
        svg,
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" font-family="Helvetica, Arial, sans-serif" font-size="9">"##,
        w = f2(layout.width),
        h = f2(layout.height)
    );
    let _ = writeln!(
        svg,
        r##"<rect x="0" y="0" width="{w}" height="{h}" fill="#ffffff"/>"##,
        w = f2(layout.width),
        h = f2(layout.height)
    );

    // Draw order: boards (background) -> edges -> glyphs -> labels.
    let mut boards = String::new();
    let mut edges = String::new();
    let mut nodes = String::new();
    let mut labels = String::new();

    for (tag, place) in &layout.places {
        match place.glyph {
            Glyph::Board => draw_board(&mut boards, tag, place),
            Glyph::Section => draw_section(&mut boards, tag, place),
            _ => draw_node(&mut nodes, &mut labels, tag, place),
        }
    }
    for route in &layout.routes {
        draw_route(&mut edges, route);
    }
    // Drafting decorations: a junction dot where a wire taps a busbar, an
    // open arrowhead where it terminates on a load (power-flow direction).
    for route in &layout.routes {
        let n = route.points.len();
        let (prev_first, prev_last) = if n >= 2 {
            (Some(route.points[0]), Some(route.points[n - 2]))
        } else {
            (None, None)
        };
        decorate_endpoint(
            &mut edges,
            &layout,
            &route.from_tag,
            route.points.first(),
            prev_first,
        );
        decorate_endpoint(
            &mut edges,
            &layout,
            &route.to_tag,
            route.points.last(),
            prev_last,
        );
    }

    for part in [&boards, &edges, &nodes, &labels] {
        svg.push_str(part);
    }
    svg.push_str("</svg>\n");
    svg
}

fn draw_board(out: &mut String, tag: &str, p: &Place) {
    // Dashed frame: drafting convention for functional-group boundaries —
    // keeps boards visually distinct from wires.
    let _ = writeln!(
        out,
        r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#f8f8f8" stroke="#555555" stroke-width="1.1" stroke-dasharray="7 4"/>"##,
        x = f2(p.x),
        y = f2(p.y),
        w = f2(p.w),
        h = f2(p.h)
    );
    // Board label sits inside the rectangle (label strip reserved by the
    // layout) so it never collides with routes above the frame. The note
    // line carries the voltage system (guidance §4.5).
    let _ = writeln!(
        out,
        r##"<text x="{x}" y="{y}" fill="#222222" font-weight="bold">{t}</text>"##,
        x = f2(p.x + 6.0),
        y = f2(p.y + 15.0),
        t = esc(tag)
    );
    if let Some(note) = &p.note {
        let _ = writeln!(
            out,
            r##"<text x="{x}" y="{y}" fill="#666666" font-size="8">{t}</text>"##,
            x = f2(p.x + 6.0),
            y = f2(p.y + 28.0),
            t = esc(note)
        );
    }
}

fn draw_section(out: &mut String, tag: &str, p: &Place) {
    let _ = writeln!(
        out,
        r##"<rect x="{x}" y="{y}" width="{w}" height="6" fill="#222222"/>"##,
        x = f2(p.x),
        y = f2(p.y),
        w = f2(p.w)
    );
    let _ = writeln!(
        out,
        r##"<text x="{x}" y="{y}" fill="#222222">{t}</text>"##,
        x = f2(p.x + 4.0),
        y = f2(p.y + 18.0),
        t = esc(tag)
    );
}

fn draw_route(out: &mut String, route: &busbar_layout::Route) {
    let pts: Vec<String> = route
        .points
        .iter()
        .map(|(x, y)| format!("{},{}", f2(*x), f2(*y)))
        .collect();
    let dash = if route.dashed {
        r##" stroke-dasharray="4 3""##
    } else {
        ""
    };
    let _ = writeln!(
        out,
        r##"<polyline points="{p}" fill="none" stroke="#333333" stroke-width="1"{dash}/>"##,
        p = pts.join(" "),
        dash = dash
    );
}

/// Junction dot / open arrowhead at one end of a route, keyed on the glyph
/// the route lands on. `prev` is the neighbouring point, for arrow heading.
fn decorate_endpoint(
    out: &mut String,
    layout: &busbar_layout::Layout,
    tag: &str,
    point: Option<&(f64, f64)>,
    prev: Option<(f64, f64)>,
) {
    let Some(&(x, y)) = point else { return };
    let Some(place) = layout.places.get(tag) else {
        return;
    };
    match place.glyph {
        Glyph::Section => {
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="3" fill="#333333"/>"##,
                cx = f2(x),
                cy = f2(y)
            );
        }
        Glyph::Load | Glyph::Lamp | Glyph::Motor | Glyph::Socket => {
            let Some((px, py)) = prev else { return };
            // Unit vector along the final wire segment, then an open V
            // perpendicular to it at the endpoint.
            let (dx, dy) = (x - px, y - py);
            let len = (dx * dx + dy * dy).sqrt();
            if len < 0.5 {
                return;
            }
            let (ux, uy) = (dx / len, dy / len);
            let (vx, vy) = (-uy, ux); // perpendicular
            // Routes run center-to-center and glyphs draw over wires, so
            // the arrow must sit at the glyph's edge, not its center.
            let edge = 15.0;
            let (tx, ty) = (x - ux * edge, y - uy * edge);
            let back = 6.0;
            let half = 2.8;
            let _ = writeln!(
                out,
                r##"<path d="M {a} {b} L {x} {y} L {c} {d}" fill="none" stroke="#333333" stroke-width="1"/>"##,
                x = f2(tx),
                y = f2(ty),
                a = f2(tx - ux * back + vx * half),
                b = f2(ty - uy * back + vy * half),
                c = f2(tx - ux * back - vx * half),
                d = f2(ty - uy * back - vy * half),
            );
        }
        _ => {}
    }
}

/// IEC 60617-7 functional switch symbol, drawn VERTICALLY as in the
/// reference drawing (corpus/render/house.svg: feeders run top-to-bottom,
/// blade rises from the lower terminal to the upper fixed contact) with
/// optional function marks — an x (circuit-breaker release), a bar across
/// the fixed contact (disconnector), or a perpendicular tick at the blade
/// tip (contactor).
fn draw_blade(
    out: &mut String,
    cx: f64,
    cy: f64,
    cross: bool,
    bar: bool,
    tick: bool,
    stroke: &str,
) {
    let tip_x = cx - 8.0;
    let tip_y = cy - 10.0;
    // Conductor: full lead below the pivot, only a short stub above the
    // fixed contact (the reference stops the upper lead at the contact).
    let _ = writeln!(
        out,
        r##"<line x1="{cx}" y1="{a}" x2="{cx}" y2="{b}" {stroke}/>"##,
        cx = f2(cx),
        a = f2(cy - 14.0),
        b = f2(cy - 10.0),
        stroke = stroke
    );
    let _ = writeln!(
        out,
        r##"<line x1="{cx}" y1="{a}" x2="{cx}" y2="{b}" {stroke}/>"##,
        cx = f2(cx),
        a = f2(cy + 6.0),
        b = f2(cy + 14.0),
        stroke = stroke
    );
    // Blade from the lower pivot up-left to the fixed contact.
    let _ = writeln!(
        out,
        r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
        a = f2(cx),
        b = f2(cy + 6.0),
        c = f2(tip_x),
        d = f2(tip_y),
        stroke = stroke
    );
    if cross {
        let _ = writeln!(
            out,
            r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/><line x1="{e}" y1="{f}" x2="{g}" y2="{h}" {stroke}/>"##,
            a = f2(tip_x + 2.0),
            b = f2(tip_y - 2.0),
            c = f2(tip_x + 6.0),
            d = f2(tip_y + 2.0),
            e = f2(tip_x + 2.0),
            f = f2(tip_y + 2.0),
            g = f2(tip_x + 6.0),
            h = f2(tip_y - 2.0),
            stroke = stroke
        );
    }
    if bar {
        let _ = writeln!(
            out,
            r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
            a = f2(cx - 3.5),
            b = f2(cy - 10.0),
            c = f2(cx + 3.5),
            stroke = stroke
        );
    }
    if tick {
        let _ = writeln!(
            out,
            r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
            a = f2(tip_x - 2.2),
            b = f2(tip_y + 2.2),
            c = f2(tip_x + 2.2),
            d = f2(tip_y - 2.2),
            stroke = stroke
        );
    }
}

fn draw_node(out: &mut String, labels: &mut String, _tag: &str, p: &Place) {
    let (cx, cy) = p.center();
    let s = 14.0; // glyph half-size
    let stroke = r##"stroke="#222222" stroke-width="1.4" fill="none""##;
    match p.glyph {
        Glyph::Source => {
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(cy),
                r = f2(s),
                stroke = stroke
            );
            glyph_text(out, cx, cy, "G");
        }
        Glyph::Transformer => {
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{c1}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                c1 = f2(cy - 5.0),
                r = f2(9.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{c2}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                c2 = f2(cy + 5.0),
                r = f2(9.0),
                stroke = stroke
            );
        }
        Glyph::Switch => draw_blade(out, cx, cy, false, false, false, stroke),
        Glyph::Protective => draw_blade(out, cx, cy, true, false, false, stroke),
        Glyph::Disconnector => draw_blade(out, cx, cy, false, true, false, stroke),
        Glyph::MainSwitch => draw_blade(out, cx, cy, true, true, false, stroke),
        Glyph::Contactor => draw_blade(out, cx, cy, false, false, true, stroke),
        Glyph::Rcd => {
            // RCBO/RCD composite, transcribed from the corpus/render/
            // house.svg reference: breaker blade with an x, the residual
            // current-transformer mark below it (core ellipse with the
            // conductor through), and the IΔn legend.
            draw_blade(out, cx, cy - 4.0, true, false, false, stroke);
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{a}" y2="{c}" {stroke}/>"##,
                a = f2(cx),
                b = f2(cy + 4.0),
                c = f2(cy + 14.0),
                stroke = stroke
            );
            let ey = cy + 8.0;
            let _ = writeln!(
                out,
                r##"<ellipse cx="{cx}" cy="{cy}" rx="4" ry="6" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(ey),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
                a = f2(cx - 5.5),
                b = f2(ey),
                c = f2(cx + 5.5),
                stroke = stroke
            );
            glyph_text(out, cx + 9.0, ey, "IΔn");
        }
        Glyph::Ats => {
            // Change-over (break-before-make), vertical: common terminal
            // below, blades to two stacked fixed contacts at the sides.
            let _ = writeln!(
                out,
                r##"<line x1="{cx}" y1="{a}" x2="{cx}" y2="{b}" {stroke}/>"##,
                cx = f2(cx),
                a = f2(cy + 14.0),
                b = f2(cy + 4.0),
                stroke = stroke
            );
            for dir in [-1.0, 1.0] {
                let fx = cx + dir * 10.0;
                let fy = cy - 8.0;
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
                    a = f2(cx),
                    b = f2(cy + 4.0),
                    c = f2(fx),
                    d = f2(fy),
                    stroke = stroke
                );
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
                    a = f2(fx),
                    b = f2(fy),
                    c = f2(fx + dir * 6.0),
                    stroke = stroke
                );
            }
        }
        Glyph::Fuse => {
            let x = f2(cx - s);
            let y = f2(cy - 7.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="14" {stroke}/><line x1="{x}" y1="{cy}" x2="{xr}" y2="{cy}" {stroke}/>"##,
                x = x,
                y = y,
                w = f2(s * 2.0),
                cy = f2(cy),
                xr = f2(cx + s),
                stroke = stroke
            );
        }
        Glyph::Lamp => {
            let r = f2(10.0);
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(cy),
                r = r,
                stroke = stroke
            );
            let d = 7.07;
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{a}" x2="{b}" y2="{b}" {stroke}/><line x1="{a}" y1="{b}" x2="{b}" y2="{a}" {stroke}/>"##,
                a = f2(cx - d),
                b = f2(cx + d),
                stroke = stroke
            );
        }
        Glyph::Motor => {
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(cy),
                r = f2(s),
                stroke = stroke
            );
            glyph_text(out, cx, cy, "M");
        }
        Glyph::Meter => {
            // Integrating instrument (watt-hour): circle with a chord and
            // kWh legend (IEC 60617-8 / extracted p44-04).
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(cy),
                r = f2(s),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
                a = f2(cx - s * 0.7),
                b = f2(cy - 4.0),
                c = f2(cx + s * 0.7),
                stroke = stroke
            );
            glyph_text(out, cx, cy + 7.0, "kWh");
        }
        Glyph::Inverter => {
            // Converter (IEC 60617-7 / extracted p11-09): square, TL-BR
            // diagonal, DC mark upper-left (solid over dashed), AC mark
            // lower-right (tilde).
            let x = f2(cx - s);
            let y = f2(cy - s);
            let w = f2(s * 2.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{w}" {stroke}/>"##,
                x = x,
                y = y,
                w = w,
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
                a = x,
                b = y,
                c = f2(cx + s),
                d = f2(cy + s),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
                a = f2(cx - s + 2.0),
                b = f2(cy - 7.0),
                c = f2(cx - 4.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" stroke="#222222" stroke-width="1.4" stroke-dasharray="2.5 1.8" fill="none"/>"##,
                a = f2(cx - s + 2.0),
                b = f2(cy - 3.5),
                c = f2(cx - 4.0),
            );
            let _ = writeln!(
                out,
                r##"<path d="M {a} {b} Q {c} {d} {e} {b}" fill="none" stroke="#222222" stroke-width="1.4"/>"##,
                a = f2(cx + 4.0),
                b = f2(cy + 8.0),
                c = f2(cx + 7.0),
                d = f2(cy + 2.0),
                e = f2(cx + 10.0),
            );
        }
        Glyph::Pv => {
            // Photovoltaic generator (extracted p43-05): rectangle with a
            // diagonal arrow (irradiation).
            let x = f2(cx - 13.0);
            let y = f2(cy - 9.0);
            let w = f2(26.0);
            let h = f2(18.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" {stroke}/>"##,
                x = x,
                y = y,
                w = w,
                h = h,
                stroke = stroke
            );
            let ax = cx + 7.0;
            let ay = cy - 5.0;
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
                a = f2(cx - 8.0),
                b = f2(cy + 6.0),
                c = f2(ax),
                d = f2(ay),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/><line x1="{e}" y1="{f}" x2="{c}" y2="{d}" {stroke}/>"##,
                a = f2(ax - 5.0),
                b = f2(ay - 1.0),
                c = f2(ax),
                d = f2(ay),
                e = f2(ax - 1.5),
                f = f2(ay - 5.0),
                stroke = stroke
            );
        }
        Glyph::Evse => {
            let x = f2(cx - s);
            let y = f2(cy - s);
            let w = f2(s * 2.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{w}" {stroke}/>"##,
                x = x,
                y = y,
                w = w,
                stroke = stroke
            );
            glyph_text(out, cx, cy, "EV");
        }
        Glyph::Battery => {
            // Battery of cells (extracted p50-00): alternating long/short
            // plates between horizontal terminals.
            for (dx, hh) in [(-10.0, 9.0), (-3.0, 4.5), (4.0, 9.0), (11.0, 4.5)] {
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{b}" x2="{a}" y2="{d}" {stroke}/>"##,
                    a = f2(cx + dx),
                    b = f2(cy - hh),
                    d = f2(cy + hh),
                    stroke = stroke
                );
            }
            for side in [-1.0, 1.0] {
                let (xa, xb) = if side < 0.0 {
                    (cx - 14.0, cx - 10.0)
                } else {
                    (cx + 11.0, cx + 14.0)
                };
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{cy}" x2="{b}" y2="{cy}" {stroke}/>"##,
                    a = f2(xa),
                    b = f2(xb),
                    cy = f2(cy),
                    stroke = stroke
                );
            }
        }
        Glyph::Spd => {
            // Surge diverter (extracted p18-02): box with a down-arrow
            // discharging to earth.
            let x = f2(cx - 8.0);
            let y = f2(cy - 11.0);
            let w = f2(16.0);
            let h = f2(16.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" {stroke}/>"##,
                x = x,
                y = y,
                w = w,
                h = h,
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
                a = f2(cx),
                b = f2(cy - 15.0),
                c = f2(cx),
                d = f2(cy + 4.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/><line x1="{e}" y1="{f}" x2="{c}" y2="{d}" {stroke}/>"##,
                a = f2(cx - 2.5),
                b = f2(cy + 0.5),
                c = f2(cx),
                d = f2(cy + 4.0),
                e = f2(cx + 2.5),
                f = f2(cy + 0.5),
                stroke = stroke
            );
            for (dy, half) in [(7.0, 8.0), (11.0, 5.0), (15.0, 2.5)] {
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{b}" {stroke}/>"##,
                    a = f2(cx - half),
                    b = f2(cy + dy),
                    c = f2(cx + half),
                    stroke = stroke
                );
            }
        }
        Glyph::Heating => {
            // Heating element per the reference: box with three vertical
            // ticks on horizontal leads.
            let x = f2(cx - 12.0);
            let y = f2(cy - 8.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" {stroke}/>"##,
                x = x,
                y = y,
                w = f2(24.0),
                h = f2(16.0),
                stroke = stroke
            );
            for dx in [-5.0, 0.0, 5.0] {
                let _ = writeln!(
                    out,
                    r##"<line x1="{a}" y1="{b}" x2="{a}" y2="{c}" {stroke}/>"##,
                    a = f2(cx + dx),
                    b = f2(cy - 5.0),
                    c = f2(cy + 5.0),
                    stroke = stroke
                );
            }
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{cy}" x2="{b}" y2="{cy}" {stroke}/><line x1="{c}" y1="{cy}" x2="{d}" y2="{cy}" {stroke}/>"##,
                a = f2(cx - 20.0),
                b = f2(cx - 12.0),
                c = f2(cx + 12.0),
                d = f2(cx + 20.0),
                cy = f2(cy),
                stroke = stroke
            );
        }
        Glyph::Earth => {
            let _ = writeln!(
                out,
                r##"<line x1="{cx}" y1="{t0}" x2="{cx}" y2="{cy}" {stroke}/>"##,
                cx = f2(cx),
                t0 = f2(cy - 12.0),
                cy = f2(cy),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{x0}" y1="{cy}" x2="{x1}" y2="{cy}" {stroke}/>"##,
                x0 = f2(cx - 10.0),
                cy = f2(cy),
                x1 = f2(cx + 10.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{x0}" y1="{y1}" x2="{x1}" y2="{y1}" {stroke}/>"##,
                x0 = f2(cx - 7.0),
                x1 = f2(cx + 7.0),
                y1 = f2(cy + 4.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{x0}" y1="{y2}" x2="{x1}" y2="{y2}" {stroke}/>"##,
                x0 = f2(cx - 4.0),
                x1 = f2(cx + 4.0),
                y2 = f2(cy + 8.0),
                stroke = stroke
            );
        }
        Glyph::Socket => {
            // BS/IEC socket outlet, vertical per the reference: stem from
            // the conductor down to a semicircle face.
            let _ = writeln!(
                out,
                r##"<line x1="{cx}" y1="{a}" x2="{cx}" y2="{b}" {stroke}/>"##,
                cx = f2(cx),
                a = f2(cy - 14.0),
                b = f2(cy + 2.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<path d="M {a} {y} A 8 8 0 0 0 {b} {y} Z" {stroke}/>"##,
                a = f2(cx - 8.0),
                b = f2(cx + 8.0),
                y = f2(cy + 2.0),
                stroke = stroke
            );
        }
        Glyph::Relay => {
            // Relay / PLC: labeled box on vertical leads (reference draws
            // PLC1/PLC2 as a square with the device legend).
            let x = f2(cx - 12.0);
            let y = f2(cy - 12.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{w}" {stroke}/>"##,
                x = x,
                y = y,
                w = f2(24.0),
                stroke = stroke
            );
            glyph_text(out, cx, cy, "CR");
        }
        Glyph::Junction => {
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="3" fill="#222222"/>"##,
                cx = f2(cx),
                cy = f2(cy)
            );
        }
        _ => {
            let x = f2(cx - s);
            let y = f2(cy - s);
            let w = f2(s * 2.0);
            let _ = writeln!(
                out,
                r##"<rect x="{x}" y="{y}" width="{w}" height="{w}" {stroke}/>"##,
                x = x,
                y = y,
                w = w,
                stroke = stroke
            );
        }
    }
    let _ = writeln!(
        labels,
        r##"<text x="{x}" y="{y}" text-anchor="middle" fill="#222222">{t}</text>"##,
        x = f2(cx),
        y = f2(p.y + p.h + 10.0),
        t = esc(&p.label)
    );
    if let Some(note) = &p.note {
        let _ = writeln!(
            labels,
            r##"<text x="{x}" y="{y}" text-anchor="middle" fill="#666666">{t}</text>"##,
            x = f2(cx),
            y = f2(p.y + p.h + 20.0),
            t = esc(note)
        );
    }
}

fn glyph_text(out: &mut String, cx: f64, cy: f64, text: &str) {
    let _ = writeln!(
        out,
        r##"<text x="{x}" y="{y}" text-anchor="middle" dominant-baseline="middle" fill="#222222" font-size="8">{t}</text>"##,
        x = f2(cx),
        y = f2(cy + 1.0),
        t = esc(text)
    );
}

/// Fixed two-decimal float formatting — the determinism contract.
fn f2(v: f64) -> String {
    format!("{:.2}", v)
}

/// Mandatory text escaping (anti-XSS for embedded diagrams).
fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}
