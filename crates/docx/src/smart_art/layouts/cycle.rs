//! Cycle layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use super::matrix::{spokes, wedge_edge};
use crate::smart_art::shapes::{Paint, ROUND_INSET, Sh, angle, card, graded, hex_adj, inset, label, lines_of, plain, polar, tag, tint, tops};

/// A preset angle (`adj1`, `adj2` of `pie` and `blockArc`): degrees clockwise from three o'clock
/// in 60000ths.
pub(crate) fn ang(deg: f32) -> i64 {
    (deg.rem_euclid(360.0) * 60_000.0).round() as i64
}

/// A band along an ellipse from `from` to `to` degrees (clockwise), `band` thick, centred on the
/// ellipse with radii `rx`, `ry` round (`cx`, `cy`), with an arrowhead at its end.
pub(crate) fn arc_arrow((cx, cy): (f32, f32), (rx, ry): (f32, f32), band: f32, from: f32, to: f32, paint: Paint) -> Vec<Sh> {
    let b = [cx - rx - band / 2.0, cy - ry - band / 2.0, 2.0 * rx + band, 2.0 * ry + band];
    let mut arc = plain("blockArc", b, paint);
    let ss = b[2].min(b[3]).max(0.01);
    arc.adj = vec![("adj1", ang(from)), ("adj2", ang(to)), ("adj3", (band / ss * 100_000.0).round().clamp(0.0, 50_000.0) as i64)];
    // The head: a triangle at the end, pointing along the way round (the ellipse's tangent).
    let (s_, c_) = to.to_radians().sin_cos();
    let (hx, hy) = (cx + rx * c_, cy + ry * s_);
    let dir = (ry * c_).atan2(-rx * s_).to_degrees();
    let s = band * 2.2;
    let head = plain("triangle", [hx - s / 2.0, hy - s / 2.0, s, s], paint).turned(dir + 90.0);
    vec![arc, head]
}

/// Each item's text round a circle, a curved arrow from each to the next.
pub(crate) fn cycle_text(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    ring_with_arcs(spec, w, h, false)
}

/// Filled blocks round a circle, a curved arrow from each to the next.
pub(crate) fn cycle_block(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    ring_with_arcs(spec, w, h, true)
}

/// Each item's text (in a filled block when `blocks`) round a circle, curved arrows between.
fn ring_with_arcs(spec: &SmartArtSpec, w: f32, h: f32, blocks: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let lw = if n == 1 { m * 0.6 } else { (m * if blocks { 0.36 } else { 0.3 }).min(m * 0.9 * (std::f32::consts::PI / n as f32).sin()) };
    let lh = lw * if blocks { 0.56 } else { 0.5 };
    // The labels' middles on an ellipse that keeps them in the frame.
    let (rx, ry) = (((w - lw) / 2.0 * 0.96).max(0.0), ((h - lh) / 2.0 * 0.96).max(0.0));
    let at = |deg: f32| {
        let (s, c) = deg.to_radians().sin_cos();
        (cx + rx * c, cy + ry * s)
    };
    let mut out = Vec::new();
    // Text first: nothing drawn behind it, and the arcs keep clear of it.
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 { (cx, cy) } else { at(angle(k, n)) };
        let b = [x - lw / 2.0, y - lh / 2.0, lw, lh];
        if blocks {
            let mut s = card("roundRect", b, graded(spec, k, n), spec, i);
            s.adj = vec![("adj", 14_000)];
            out.push(s);
        } else {
            out.push(label(spec, i, b, "ctr", "ctr"));
        }
    }
    if n > 1 {
        let band = (m * 0.03).max(0.5);
        // How far round (degrees) a label reaches either side of its middle: half its box
        // across the way round, over the ellipse's speed there.
        let half = |deg: f32| {
            let (s, c) = deg.to_radians().sin_cos();
            let reach = s.abs() * lw / 2.0 + c.abs() * lh / 2.0 + band;
            let speed = (rx * s).hypot(ry * c).max(0.01);
            (reach / speed).min(1.4).to_degrees()
        };
        for k in 0..n {
            let (a0, a1) = (angle(k, n), angle(k + 1, n));
            let (from, to) = (a0 + half(a0), a1 - half(a1));
            if to - from > 8.0 {
                out.extend(arc_arrow((cx, cy), (rx, ry), band, from, to - 4.0, tint(spec, k, 85_000)));
            }
        }
    }
    out
}

/// Blocks on a ring that ends in an arrowhead, the first block at the top.
pub(crate) fn cycle_continuous(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let bw = (w * 0.26).min(w * 0.9 / (n as f32 / 2.0 + 1.0));
    let bh = (h * 0.2).min(bw * 0.6);
    let band = w.min(h) * 0.07;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (rx, ry) = (((w - bw) / 2.0 * 0.92).max(band), ((h - bh) / 2.0 * 0.92).max(band));
    let mut out = arc_arrow((cx, cy), (rx, ry), band, -60.0, 225.0, tint(spec, 0, 40_000));
    for (k, &i) in tops.iter().enumerate() {
        let a = angle(k, n).to_radians();
        let (x, y) = (cx + rx * a.cos(), cy + ry * a.sin());
        let mut s = card("roundRect", [x - bw / 2.0, y - bh / 2.0, bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 14_000)];
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
    }
    out
}

/// A ring cut into a segment per item holding its text, a light circle in the middle.
pub(crate) fn cycle_segmented(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    wedges(spec, w, h, true)
}

/// A circle cut into a wedge per item holding its text.
pub(crate) fn cycle_pie(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    wedges(spec, w, h, false)
}

/// A circle cut into a wedge per item, with a light hole in the middle when `hole`.
fn wedges(spec: &SmartArtSpec, w: f32, h: f32, hole: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = w.min(h) * 0.96;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let circle = [cx - d / 2.0, cy - d / 2.0, d, d];
    let step = 360.0 / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let from = -90.0 + step * k as f32;
        let mid = from + step / 2.0;
        let r = if n == 1 && !hole {
            0.0
        } else if hole {
            d * 0.36
        } else {
            d * 0.27
        };
        let (tx, ty) = polar(cx, cy, r, if n == 1 { -90.0 } else { mid });
        let s = if n == 1 && !hole {
            d * 0.6
        } else {
            (d * if hole { 0.24 } else { 0.3 }).min(d * if hole { 0.5 } else { 0.4 } * (step.min(180.0) / 2.0).to_radians().sin())
        };
        let tb = [tx - s / 2.0, ty - s / 2.0, s, s];
        let mut seg = card(if n == 1 { "ellipse" } else { "pie" }, circle, graded(spec, k, n), spec, i).picked_by(tb);
        if n > 1 {
            seg.adj = vec![("adj1", wedge_edge(16_200_000, k, n)), ("adj2", wedge_edge(16_200_000, k + 1, n))];
        }
        seg.tb = tb;
        seg.line = Paint::Light;
        seg.line_w = 2.5;
        out.push(seg);
    }
    if hole {
        let hd = d * 0.4;
        out.push(plain("ellipse", [cx - hd / 2.0, cy - hd / 2.0, hd, hd], Paint::Light));
    }
    out
}

/// Filled circles on a ring line (no arrows), each item's text inside its circle.
pub(crate) fn cycle_nondirectional(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let d = if n == 1 { m * 0.6 } else { (m * 0.34).min(m * 0.9 * (std::f32::consts::PI / n as f32).sin()) };
    let (rx, ry) = (((w - d) / 2.0 * 0.96).max(0.0), ((h - d) / 2.0 * 0.96).max(0.0));
    let mut out = Vec::new();
    if n > 1 {
        let mut ring = plain("ellipse", [cx - rx, cy - ry, 2.0 * rx, 2.0 * ry], Paint::None);
        ring.line = Paint::Accent(spec.colors.accent(0), Some(60_000));
        ring.line_w = (m * 0.012).clamp(1.0, 4.0);
        out.push(ring);
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 {
            (cx, cy)
        } else {
            let a = angle(k, n).to_radians();
            (cx + rx * a.cos(), cy + ry * a.sin())
        };
        let b = [x - d / 2.0, y - d / 2.0, d, d];
        let mut s = card("ellipse", b, graded(spec, k, n), spec, i);
        s.tb = inset(b, ROUND_INSET);
        s.line = Paint::Light;
        s.line_w = 2.0;
        out.push(s);
    }
    out
}

/// The first item in a large hexagon in the middle, the items under it in hexagons round it.
pub(crate) fn cycle_hexagon_radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let hex = |b: [f32; 4], fill: Paint, i: usize| {
        let mut s = card("hexagon", b, fill, spec, i);
        s.text = lines_of(spec, i, true, n == 0 || i != 0);
        s.adj = vec![("adj", hex_adj(b))];
        s.tb = [b[0] + b[2] * 0.2, b[1] + b[3] * 0.12, b[2] * 0.6, b[3] * 0.76];
        s.line = Paint::Light;
        s.line_w = 2.0;
        s
    };
    let mut out = Vec::new();
    // Hexagons as wide as `s`, 0.87 of that high.
    let step = if n <= 1 { 1.0 } else { (std::f32::consts::PI / n as f32).sin() };
    let mut hs = m * 0.34;
    let (mut rx, mut ry) = (0.0, 0.0);
    for _ in 0..4 {
        rx = ((w - hs) / 2.0).max(0.0).min(m * 0.36);
        ry = ((h - hs * 0.87) / 2.0).max(0.0).min(m * 0.36);
        hs = (m * 0.34).min(2.0 * rx.min(ry) * step * 0.95).max(m * 0.08);
    }
    let hc = if n == 0 { m * 0.6 } else { (hs * 1.25).min(2.0 * (rx.min(ry) - hs * 0.45)).max(hs * 0.6) };
    out.push(hex([cx - hc / 2.0, cy - hc * 0.435, hc, hc * 0.87], graded(spec, 0, 1), 0));
    for (k, &i) in around.iter().enumerate() {
        let a = angle(k, n).to_radians();
        let (x, y) = (cx + rx * a.cos(), cy + ry * a.sin());
        out.push(hex([x - hs / 2.0, y - hs * 0.435, hs, hs * 0.87], graded(spec, k + 1, n + 1), i));
    }
    out
}

/// Blocks round a circle with a two-headed arrow between each pair of neighbours.
pub(crate) fn cycle_multidirectional(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let bw = (w * 0.3).min(w * 0.9 / (n as f32 / 2.0 + 1.0));
    let bh = (h * 0.24).min(bw * 0.6);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let (rx, ry) = (((w - bw) / 2.0 * 0.96).max(0.0), ((h - bh) / 2.0 * 0.96).max(0.0));
    let at = |k: usize| {
        let a = angle(k, n).to_radians();
        (cx + rx * a.cos(), cy + ry * a.sin())
    };
    let mut out = Vec::new();
    if n > 1 {
        for k in 0..n {
            let ((x0, y0), (x1, y1)) = (at(k), at((k + 1) % n));
            let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            let dist = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
            // Short enough that its box (before turning) stays in the frame.
            let room = 2.0 * mx.min(w - mx).min(my).min(h - my);
            let len = (dist - bw.min(bh) * 1.2).clamp(0.0, dist * 0.5).min(room);
            if len < 2.0 {
                continue;
            }
            let th = (bh * 0.4).min(len * 0.45);
            let deg = (y1 - y0).atan2(x1 - x0).to_degrees();
            let mut a = plain("leftRightArrow", [mx - len / 2.0, my - th / 2.0, len, th], tint(spec, k, 55_000)).turned(deg);
            a.adj = vec![("adj1", 50_000), ("adj2", 60_000)];
            out.push(a);
        }
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 { (cx, cy) } else { at(k) };
        let mut s = card("roundRect", [x - bw / 2.0, y - bh / 2.0, bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 14_000)];
        out.push(s);
    }
    out
}

/// Dots on a ring, each item's text just outside its dot.
pub(crate) fn cycle_dots(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let lw = (w * 0.26).min(w * 0.9 / (n as f32 / 2.0 + 1.0));
    let lh = (h * 0.2).min(lw * 0.7);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let d = (w.min(h) * 0.06).max(1.0);
    // Room for a label beside the ring (left and right) and above and below it.
    let (rx, ry) = ((w / 2.0 - lw - d).max(w * 0.1), (h / 2.0 - lh - d).max(h * 0.1));
    let mut ring = plain("ellipse", [cx - rx, cy - ry, 2.0 * rx, 2.0 * ry], Paint::None);
    ring.line = Paint::Accent(spec.colors.accent(0), Some(60_000));
    ring.line_w = 2.0;
    let mut out = vec![ring];
    for (k, &i) in tops.iter().enumerate() {
        let a = angle(k, n).to_radians();
        let (c, s) = (a.cos(), a.sin());
        let (x, y) = (cx + rx * c, cy + ry * s);
        let mut dot = plain("ellipse", [x - d / 2.0, y - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 1.5;
        out.push(dot);
        // Beside the dot (to the side it faces) or above or below it, clear of it, in the frame.
        let side = c.abs() * ry > s.abs() * rx;
        let (lx, ly) = if side { (x + c.signum() * (lw / 2.0 + d * 0.7), y) } else { (x, y + s.signum() * (lh / 2.0 + d * 0.7)) };
        let b = [(lx - lw / 2.0).clamp(0.0, (w - lw).max(0.0)), (ly - lh / 2.0).clamp(0.0, (h - lh).max(0.0)), lw, lh];
        let algn = if !side {
            "ctr"
        } else if c > 0.0 {
            "l"
        } else {
            "r"
        };
        out.push(label(spec, i, b, algn, "ctr"));
    }
    out
}

/// The first item in a circle in the middle, the items under it in circles on a ring round it.
pub(crate) fn cycle_radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut out = Vec::new();
    let centre = |d: f32| {
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        let mut c = tag("ellipse", b, graded(spec, 0, 1), spec, 0);
        c.tb = inset(b, ROUND_INSET);
        c
    };
    if n == 0 {
        out.push(centre(m * 0.5));
        return out;
    }
    let step = if n == 1 { 1.0 } else { (std::f32::consts::PI / n as f32).sin() };
    let mut ds = m * 0.26;
    for _ in 0..4 {
        let r = (m - ds) / 2.0;
        ds = (m * 0.26).min(2.0 * r * step * 0.8);
    }
    let r = (m - ds) / 2.0;
    let dc = (m * 0.42).min(2.0 * (r - ds / 2.0) - m * 0.06).max(m * 0.08);
    let mut ring = plain("ellipse", [cx - r, cy - r, 2.0 * r, 2.0 * r], Paint::None);
    ring.line = Paint::Accent(spec.colors.accent(0), Some(60_000));
    ring.line_w = 2.0;
    out.push(ring);
    out.push(centre(dc));
    for (k, &i) in around.iter().enumerate() {
        let (x, y) = polar(cx, cy, r, angle(k, n));
        let b = [x - ds / 2.0, y - ds / 2.0, ds, ds];
        let mut s = card("ellipse", b, graded(spec, k + 1, n + 1), spec, i);
        s.text = lines_of(spec, i, true, true);
        s.tb = inset(b, ROUND_INSET);
        s.line = Paint::Light;
        s.line_w = 2.0;
        out.push(s);
    }
    out
}
