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
        Glyph::Switch | Glyph::Ats => {
            // IEC breaker: square with the switching diagonal.
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
                r##"<line x1="{a}" y1="{b}" x2="{b}" y2="{a}" {stroke}/>"##,
                a = f2(cx - s),
                b = f2(cx + s),
                stroke = stroke
            );
            if p.glyph == Glyph::Ats {
                glyph_text(out, cx, cy - s - 6.0, "ATS");
            }
        }
        Glyph::Protective => {
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
                r##"<line x1="{a}" y1="{b}" x2="{b}" y2="{a}" {stroke}/>"##,
                a = f2(cx - s),
                b = f2(cx + s),
                stroke = stroke
            );
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
            let _ = writeln!(
                out,
                r##"<circle cx="{cx}" cy="{cy}" r="{r}" {stroke}/>"##,
                cx = f2(cx),
                cy = f2(cy),
                r = f2(s),
                stroke = stroke
            );
        }
        Glyph::Battery | Glyph::Inverter | Glyph::Pv | Glyph::Spd => {
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
            let text = match p.glyph {
                Glyph::Battery => "BAT",
                Glyph::Inverter => "INV",
                Glyph::Pv => "PV",
                _ => "SPD",
            };
            glyph_text(out, cx, cy, text);
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
            // IEC 60617 socket outlet: semicircle on a base line.
            let _ = writeln!(
                out,
                r##"<path d="M {a} {y} A {r} {r} 0 0 1 {b} {y} Z" {stroke}/>"##,
                a = f2(cx - 8.0),
                b = f2(cx + 8.0),
                y = f2(cy + 4.0),
                r = f2(8.0),
                stroke = stroke
            );
            let _ = writeln!(
                out,
                r##"<line x1="{a}" y1="{y}" x2="{b}" y2="{y}" {stroke}/>"##,
                a = f2(cx - 11.0),
                b = f2(cx + 11.0),
                y = f2(cy + 4.0),
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
