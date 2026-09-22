//! SVG renderer (spec §16.2; implementation plan §5.6).
//!
//! Symbols are drawn from vector primitives (IEC 60617 conventions) — no
//! imported artwork, per the licensing/determinism stance. Output is
//! byte-deterministic: BTreeMap iteration, tag-sorted order, one fixed
//! float formatter, and escaped text.

use std::fmt::Write as _;

use busbar_ir::Ir;
use busbar_layout::{Glyph, Place};

mod symbols;

/// Renders an ESLD document's source text. `symbols` selects the registry;
/// only "iec" exists (ANSI lands with M8).
pub fn render_str(source: &str, symbols: &str) -> Result<String, String> {
    if symbols != "iec" {
        return Err(format!(
            "symbol registry {symbols:?} not available (ANSI lands with M8)"
        ));
    }
    let doc = busbar_syntax::parse_or_string(source)?;
    let ir = Ir::build(&doc)
        .map_err(|e| format!("line {}:{}: {} [E-IR-1]", e.line, e.col, e.message))?;

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
            (Some(route.points[1]), Some(route.points[n - 2]))
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
        // The load glyph itself is the sheet's arrow symbol; lamps,
        // motors and sockets get an open V seated at the route end —
        // routes already terminate at the glyph's lead end.
        Glyph::Lamp | Glyph::Motor | Glyph::Socket => {
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
            let (tx, ty) = (x, y);
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

fn draw_node(out: &mut String, labels: &mut String, _tag: &str, p: &Place) {
    let (cx, cy) = p.center();
    let s = 14.0; // glyph half-size
    let stroke = r##"stroke="#222222" stroke-width="1.4" fill="none""##;
    // Sheet symbols (corpus/render/symbols.svg) render as transcribed
    // fragments centred on the place; glyphs without a sheet symbol keep
    // their hand-drawn primitive below.
    if let Some(fragment) = symbols::fragment(p.glyph) {
        let _ = writeln!(
            out,
            r##"<g transform="translate({x} {y})" fill="none" stroke="#222222" stroke-width="1">{f}</g>"##,
            x = f2(cx),
            y = f2(cy),
            f = fragment
        );
    } else {
        match p.glyph {
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
            Glyph::Ats => {
                // Change-over, break-before-make: common terminal below, two
                // fixed contacts at the sides — one blade closed onto its
                // contact, the other drawn OPEN (stopping short), so the
                // symbol never depicts both sources bridged at once.
                let _ = writeln!(
                    out,
                    r##"<line x1="{cx}" y1="{a}" x2="{cx}" y2="{b}" {stroke}/>"##,
                    cx = f2(cx),
                    a = f2(cy + 14.0),
                    b = f2(cy + 4.0),
                    stroke = stroke
                );
                for (dir, closed) in [(-1.0, true), (1.0, false)] {
                    let fx = cx + dir * 10.0;
                    let fy = cy - 8.0;
                    let (bx, by) = if closed {
                        (fx, fy)
                    } else {
                        (cx + dir * 6.5, cy - 4.5) // blade stops short: open
                    };
                    let _ = writeln!(
                        out,
                        r##"<line x1="{a}" y1="{b}" x2="{c}" y2="{d}" {stroke}/>"##,
                        a = f2(cx),
                        b = f2(cy + 4.0),
                        c = f2(bx),
                        d = f2(by),
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
                    r##"<line x1="{a}" y1="{e}" x2="{b}" y2="{f}" {stroke}/><line x1="{a}" y1="{f}" x2="{b}" y2="{e}" {stroke}/>"##,
                    a = f2(cx - d),
                    b = f2(cx + d),
                    e = f2(cy - d),
                    f = f2(cy + d),
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
