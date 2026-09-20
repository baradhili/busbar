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

    for part in [&boards, &edges, &nodes, &labels] {
        svg.push_str(part);
    }
    svg.push_str("</svg>\n");
    svg
}

fn draw_board(out: &mut String, tag: &str, p: &Place) {
    let _ = writeln!(
        out,
        r##"<rect x="{x}" y="{y}" width="{w}" height="{h}" fill="#f8f8f8" stroke="#444444" stroke-width="1.2"/>"##,
        x = f2(p.x),
        y = f2(p.y),
        w = f2(p.w),
        h = f2(p.h)
    );
    // Board label sits inside the rectangle (label strip reserved by the
    // layout) so it never collides with routes above the frame.
    let _ = writeln!(
        out,
        r##"<text x="{x}" y="{y}" fill="#222222" font-weight="bold">{t}</text>"##,
        x = f2(p.x + 6.0),
        y = f2(p.y + 15.0),
        t = esc(tag)
    );
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
