//! Post-routing wire declutter (guidance §3): channels are computed
//! blind by the shaping passes (midpoints, side-form bonds, gutters),
//! so a wire can slice a glyph core or share a lane with an unrelated
//! wire. This pass gives finished routes obstacle awareness:
//!
//! - **Clearing** — any segment intersecting a glyph cell's exclusion
//!   zone (boards, bars and junction dots excepted; the route's own
//!   endpoints excepted) shifts its lane clear: corner shifts for
//!   mid-route segments, jog elbows when the segment is pinned to a
//!   route endpoint, and a rise/dip rewrite for side-form bonds whose
//!   straight run crosses the band it borders.
//! - **Separation** — parallel segments of unrelated routes (no shared
//!   endpoint tag) that overlap within `SEP` px get distinct lanes.
//!   Overlaps between routes that DO share an endpoint are left alone:
//!   those are tapped trunks (guidance §2.2), not collisions.
//!
//! Everything iterates a bounded number of times in declaration order —
//! deterministic, and idempotent once clean.

use std::collections::BTreeMap;

use crate::{Glyph, Layout, Place, Route};

/// Exclusion margin around a glyph cell.
const M: f64 = 6.0;
/// Approach length of a jog elbow at a pinned route endpoint.
const JOG: f64 = 10.0;
/// Vertical lane search step when threading between cells.
const LANE: f64 = 14.0;
/// Minimum separation between parallel lanes of unrelated routes.
const SEP: f64 = 8.0;
/// Bounded fixpoint rounds.
const ROUNDS: usize = 3;

pub(crate) fn declutter(layout: &mut Layout) {
    for _ in 0..ROUNDS {
        let crossed = clear_crossings(layout);
        let squeezed = separate(layout);
        if !crossed && !squeezed {
            break;
        }
    }
}

/// A placed glyph a wire must not run through.
fn is_blocker(p: &Place) -> bool {
    p.w > 0.0 && !matches!(p.glyph, Glyph::Board | Glyph::Section | Glyph::Junction)
}

/// Cells intersecting an axis-aligned segment (with margin), excluding
/// the route's own endpoints.
fn blockers<'a>(
    places: &'a BTreeMap<String, Place>,
    horizontal: bool,
    lane: f64,
    lo: f64,
    hi: f64,
    skip: &[&str],
) -> Vec<&'a Place> {
    places
        .iter()
        .filter(|(tag, p)| !skip.contains(&tag.as_str()) && is_blocker(p))
        .filter(|(_, p)| {
            let (c_lo, c_hi) = if horizontal {
                (p.x - M, p.x + p.w + M)
            } else {
                (p.y - M, p.y + p.h + M)
            };
            let (l_lo, l_hi) = if horizontal {
                (p.y - M, p.y + p.h + M)
            } else {
                (p.x - M, p.x + p.w + M)
            };
            // Span overlap beyond grazing, lane strictly inside the
            // cell's cross-zone.
            lo < c_hi - 1.0 && hi > c_lo + 1.0 && l_lo + 1.0 < lane && lane < l_hi - 1.0
        })
        .map(|(_, p)| p)
        .collect()
}

/// First clear horizontal lane for a span, or `None`.
fn clear_h(
    places: &BTreeMap<String, Place>,
    lo: f64,
    hi: f64,
    mut y: f64,
    skip: &[&str],
) -> Option<f64> {
    for _ in 0..3 {
        let blocked = blockers(places, true, y, lo, hi, skip);
        if blocked.is_empty() {
            return Some(y);
        }
        // Under a labelled row the channel must clear the label strip
        // (label + note lines), not just the cell box.
        let below = blocked
            .iter()
            .map(|p| {
                p.y + p.h
                    + if p.label.is_empty() && p.note.is_none() {
                        M + 6.0
                    } else {
                        crate::LABEL_CLEAR
                    }
            })
            .fold(f64::MIN, f64::max);
        let above = blocked.iter().map(|p| p.y).fold(f64::MAX, f64::min) - M - 6.0;
        let next = if (below - y).abs() <= (above - y).abs() {
            below
        } else {
            above
        };
        if (next - y).abs() < 0.5 {
            return None;
        }
        y = next;
    }
    None
}

/// First clear vertical lane for a span, searching outward from `x`.
fn clear_v(
    places: &BTreeMap<String, Place>,
    lo: f64,
    hi: f64,
    x: f64,
    skip: &[&str],
    bound: f64,
) -> Option<f64> {
    for k in 1..=8 {
        for sign in [1.0, -1.0] {
            let cand = x + sign * k as f64 * LANE;
            if cand <= 4.0 || cand >= bound {
                continue;
            }
            if blockers(places, false, cand, lo, hi, skip).is_empty() {
                return Some(cand);
            }
        }
    }
    None
}

fn clear_crossings(layout: &mut Layout) -> bool {
    let mut changed = false;
    let bound = layout
        .places
        .values()
        .map(|p| p.x + p.w)
        .fold(0.0, f64::max)
        + 8.0;
    // Split borrows: places stay immutable while routes mutate.
    for route in layout.routes.iter_mut() {
        // Owned copies: the mutable route borrows must not be held by
        // the skip list.
        let skip_owned = [route.from_tag.clone(), route.to_tag.clone()];
        let mut si = 0;
        while si + 1 < route.points.len() {
            let (a, b) = (route.points[si], route.points[si + 1]);
            let horizontal = (a.1 - b.1).abs() < 0.5 && (a.0 - b.0).abs() > 0.5;
            let vertical = (a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() > 0.5;
            if !horizontal && !vertical {
                si += 1;
                continue;
            }
            let last = route.points.len() - 1;
            let pinned = si == 0 || si + 1 == last;
            if horizontal {
                let (lo, hi) = (a.0.min(b.0), a.0.max(b.0));
                let skip_tags: [&str; 2] = [skip_owned[0].as_str(), skip_owned[1].as_str()];
                if blockers(&layout.places, true, a.1, lo, hi, &skip_tags).is_empty() {
                    si += 1;
                    continue;
                }
                // A route endpoint ON the horizontal pins it: only the
                // side-form bond enters its terminals laterally. Elbow
                // horizontals (VHV midpoints, jog approaches) shift
                // freely — the adjacent verticals just stretch.
                let on_endpoint = (si == 0 && route.points[0].1 == a.1)
                    || (si + 1 == last && route.points[last].1 == a.1);
                if let Some(y2) = clear_h(&layout.places, lo, hi, a.1, &skip_tags) {
                    if on_endpoint && last == 3 {
                        // Side-form bond whose straight run crosses the
                        // band it borders: rise/dip around the band.
                        side_form_jog(route, y2);
                        changed = true;
                    } else if !on_endpoint {
                        route.points[si].1 = y2;
                        route.points[si + 1].1 = y2;
                        changed = true;
                    }
                }
            } else {
                let (lo, hi) = (a.1.min(b.1), a.1.max(b.1));
                if blockers(
                    &layout.places,
                    false,
                    a.0,
                    lo,
                    hi,
                    &[&skip_owned[0], &skip_owned[1]],
                )
                .is_empty()
                {
                    si += 1;
                    continue;
                }
                if let Some(x2) = clear_v(
                    &layout.places,
                    lo,
                    hi,
                    a.0,
                    &[&skip_owned[0], &skip_owned[1]],
                    bound,
                ) {
                    if !pinned {
                        route.points[si].0 = x2;
                        route.points[si + 1].0 = x2;
                        changed = true;
                    } else {
                        vertical_jog(route, si, x2);
                        changed = true;
                        si += 2; // skip the inserted elbow
                    }
                }
            }
            si += 1;
        }
    }
    changed
}

/// Side-form bond (4 points, straight out of both terminals' sides)
/// whose run is blocked: out 10px, rise/dip to the clear lane, across,
/// back in. Endpoint heights may differ (board bonds exit at the
/// frame's centre line) — each end keeps its own approach height.
fn side_form_jog(route: &mut Route, y2: f64) {
    let pts = &route.points;
    if pts.len() != 4 {
        return;
    }
    let dir = (pts[3].0 - pts[0].0).signum();
    if dir == 0.0 {
        return;
    }
    let (x0, y) = pts[0];
    let (x3, y3) = pts[3];
    route.points = vec![
        (x0, y),
        (x0 + dir * JOG, y),
        (x0 + dir * JOG, y2),
        (x3 - dir * JOG, y2),
        (x3 - dir * JOG, y3),
        (x3, y3),
    ];
}

/// Pinned vertical at segment `si` (0 = departure, 2 = arrival) of a
/// plain 4-point route: rebuild as terminal → JOG out → clear lane →
/// JOG back → terminal, keeping both terminal approaches vertical.
/// Richer shapes (gutters, dips) never carry blocked endpoint
/// verticals — their long runs are mid-route segments.
fn vertical_jog(route: &mut Route, si: usize, x2: f64) {
    let pts = &mut route.points;
    if pts.len() != 4 {
        return;
    }
    let (x0, y0) = pts[0];
    let (x3, y3) = pts[3];
    let dir_out = (pts[1].1 - pts[0].1).signum();
    let dir_in = (pts[3].1 - pts[2].1).signum();
    if dir_out == 0.0 || dir_in == 0.0 {
        return;
    }
    let e1 = y0 + dir_out * JOG;
    let e2 = y3 - dir_in * JOG;
    if (e1 - e2).abs() < 1.0 {
        return; // no room for a lane between the elbows
    }
    let _ = si; // either pinned end gets the same shape
    *pts = vec![(x0, y0), (x0, e1), (x2, e1), (x2, e2), (x3, e2), (x3, y3)];
}

fn separate(layout: &mut Layout) -> bool {
    #[allow(clippy::type_complexity)]
    let mut lanes: Vec<(bool, f64, f64, f64, usize, usize)> = Vec::new(); // (horiz, lane, lo, hi, route, seg)
    for (ri, route) in layout.routes.iter().enumerate() {
        for si in 0..route.points.len() - 1 {
            let (a, b) = (route.points[si], route.points[si + 1]);
            if (a.1 - b.1).abs() < 0.5 && (a.0 - b.0).abs() > 0.5 {
                lanes.push((true, a.1, a.0.min(b.0), a.0.max(b.0), ri, si));
            } else if (a.0 - b.0).abs() < 0.5 && (a.1 - b.1).abs() > 0.5 {
                lanes.push((false, a.0, a.1.min(b.1), a.1.max(b.1), ri, si));
            }
        }
    }
    let mut changed = false;
    for i in 0..lanes.len() {
        for j in (i + 1)..lanes.len() {
            let (h1, l1, lo1, hi1, r1, _s1) = lanes[i];
            let (h2, l2, lo2, hi2, r2, s2) = lanes[j];
            if h1 != h2 || r1 == r2 {
                continue;
            }
            if (l1 - l2).abs() >= SEP || lo1 >= hi2 - 6.0 || lo2 >= hi1 - 6.0 {
                continue;
            }
            // Shared-endpoint overlaps are tapped trunks, not collisions.
            let (a, b) = (&layout.routes[r1], &layout.routes[r2]);
            if a.from_tag == b.from_tag
                || a.from_tag == b.to_tag
                || a.to_tag == b.from_tag
                || a.to_tag == b.to_tag
            {
                continue;
            }
            // Shift the later segment's lane away from the earlier one —
            // never INTO a glyph (clearing and separation would otherwise
            // fight: clear lifts a lane, separation pushes it back down).
            let route = &mut layout.routes[r2];
            let pinned = s2 == 0 || s2 + 1 == route.points.len() - 1;
            if pinned {
                continue;
            }
            let skip: Vec<String> = vec![route.from_tag.clone(), route.to_tag.clone()];
            let free = |cand: f64| -> bool {
                let sk = [skip[0].as_str(), skip[1].as_str()];
                blockers(&layout.places, h2, cand, lo2, hi2, &sk).is_empty()
            };
            let away = if l2 >= l1 { 1.0 } else { -1.0 };
            let target = l1 + away * (SEP + 2.0);
            let target = if free(target) {
                target
            } else if free(l1 - away * (SEP + 2.0)) {
                l1 - away * (SEP + 2.0)
            } else {
                continue;
            };
            if h2 {
                route.points[s2].1 = target;
                route.points[s2 + 1].1 = target;
            } else {
                route.points[s2].0 = target;
                route.points[s2 + 1].0 = target;
            }
            changed = true;
            // The moved lane is the comparison basis for later pairs.
            lanes[j].1 = target;
        }
    }
    changed
}
