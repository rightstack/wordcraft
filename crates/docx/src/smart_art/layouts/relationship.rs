//! Relationship layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use super::matrix::spokes;
use crate::smart_art::shapes::{
    Ink, Paint, ROUND_INSET, Sh, angle, arrow, card, graded, graded_light, inset, label, lines_of, plain, polar, sign, tag, tint, tops,
};

/// Two sides on a beam over a stand: the first top item (and every other one after) on the left,
/// the second on the right, stacked as boxes above the beam's ends; the beam tips towards the side
/// with more lines.
pub(crate) fn balance(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let weight = |i: usize| 1 + spec.descendants(i).len();
    let left: Vec<usize> = tops.iter().copied().step_by(2).collect();
    let right: Vec<usize> = tops.iter().copied().skip(1).step_by(2).collect();
    let (wl, wr): (usize, usize) = (left.iter().map(|&i| weight(i)).sum(), right.iter().map(|&i| weight(i)).sum());
    let tilt: f32 = match wl.cmp(&wr) {
        std::cmp::Ordering::Greater => -4.0,
        std::cmp::Ordering::Less => 4.0,
        std::cmp::Ordering::Equal => 0.0,
    };
    let cx = w / 2.0;
    let beam_y = h * 0.66;
    let beam_l = w * 0.86;
    let t = (h * 0.035).max(1.0);
    let a = spec.colors.accent(0);
    let mut out = Vec::new();
    // The stand, then the beam.
    let fw = (w * 0.16).min(h * 0.3);
    out.push(plain("triangle", [cx - fw / 2.0, beam_y + t / 2.0, fw, (h * 0.98 - beam_y - t / 2.0).max(0.0)], tint(spec, 0, 60_000)));
    out.push(plain("rect", [cx - beam_l / 2.0, beam_y - t / 2.0, beam_l, t], Paint::Accent(a, Some(80_000))).turned(tilt));
    // Each side's boxes, bottom up from where the beam is under the box's middle.
    let arm = w * 0.27;
    let bw = w * 0.4;
    for (group, sign) in [(&left, -1.0f32), (&right, 1.0f32)] {
        if group.is_empty() {
            continue;
        }
        let x = cx + sign * arm - bw / 2.0;
        // The beam's top under this side (turned by `tilt` clockwise: a positive tilt lowers the right).
        let end = beam_y + sign * arm * tilt.to_radians().tan() - t / 2.0 - h * 0.01 - bw / 2.0 * tilt.to_radians().tan().abs();
        let m = group.len() as f32;
        let gap = h * 0.02;
        let bh = ((end - h * 0.02) / m - gap).clamp(0.0, h * 0.34);
        for (j, &i) in group.iter().enumerate() {
            let y = end - (j as f32 + 1.0) * (bh + gap) + gap;
            let k = tops.iter().position(|&t| t == i).unwrap_or(0);
            let mut s = card("roundRect", [x, y.max(0.0), bw, bh], graded(spec, k, tops.len()), spec, i);
            s.adj = vec![("adj", 10_000)];
            out.push(s);
        }
    }
    out
}

/// The items but the last in circles above a funnel; an arrow down from the funnel to the last
/// item in a box below.
pub(crate) fn funnel(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let (inputs, result) = tops.split_at(n.saturating_sub(1).min(tops.len()));
    let cx = w / 2.0;
    let fw = (w * 0.8).min(h * 1.3);
    let m = inputs.len().max(1) as f32;
    let d = (fw / (m * 1.04)).min(h * 0.32);
    let mut out = Vec::new();
    let row = d * 1.04;
    for (k, &i) in inputs.iter().enumerate() {
        let x = cx - row * m / 2.0 + row * k as f32 + (row - d) / 2.0;
        let b = [x, h * 0.01, d, d];
        let mut c = card("ellipse", b, graded(spec, k, n), spec, i);
        c.text = lines_of(spec, i, true, false);
        c.tb = inset(b, ROUND_INSET);
        out.push(c);
    }
    let fy = h * 0.03 + d;
    let fh = h * 0.24;
    let neck = fw * 0.22;
    let mut f = plain("trapezoid", [cx - fw / 2.0, fy, fw, fh], tint(spec, 0, 35_000)).turned(180.0);
    let ss = fw.min(fh).max(0.01);
    f.adj = vec![("adj", ((fw - neck) / 2.0 / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64)];
    out.push(f);
    let ay = fy + fh;
    let ah = h * 0.1;
    let mut arrow = plain("downArrow", [cx - neck * 0.35, ay, neck * 0.7, ah], tint(spec, 0, 60_000));
    arrow.adj = vec![("adj1", 50_000), ("adj2", 60_000)];
    out.push(arrow);
    if let Some(&i) = result.first() {
        let ry = ay + ah + h * 0.01;
        let rw = (fw * 0.62).min(w * 0.96);
        let b = [cx - rw / 2.0, ry, rw, (h * 0.99 - ry).max(0.0)];
        let mut r = card("roundRect", b, graded(spec, n - 1, n), spec, i);
        r.adj = vec![("adj", 12_000)];
        out.push(r);
    }
    out
}

/// Interlocking gears along a zigzag, large and small in turn, each item's text in its gear.
pub(crate) fn gears(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    // Gear `k`'s radius and centre at unit size (the first gear's radius 1).
    let r = |k: usize| if k.is_multiple_of(2) { 1.0f32 } else { 0.72 };
    let mut centres = vec![(0.0f32, 0.0f32)];
    for k in 1..n {
        let (px, py) = centres.last().copied().unwrap_or((0.0, 0.0));
        let dist = 0.94 * (r(k - 1) + r(k));
        // Down and up in turn, 32 degrees off the horizontal.
        let dy = dist * 0.53 * if k % 2 == 1 { -1.0 } else { 1.0 };
        centres.push((px + dist * 0.848, py + dy));
    }
    let ext = 1.05;
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for (k, &(x, y)) in centres.iter().enumerate() {
        let e = r(k) * ext;
        (x0, y0, x1, y1) = (x0.min(x - e), y0.min(y - e), x1.max(x + e), y1.max(y + e));
    }
    let scale = ((w * 0.98) / (x1 - x0).max(0.01)).min((h * 0.98) / (y1 - y0).max(0.01));
    let (ox, oy) = ((w - (x1 - x0) * scale) / 2.0 - x0 * scale, (h - (y1 - y0) * scale) / 2.0 - y0 * scale);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let Some(&(ux, uy)) = centres.get(k) else { continue };
        let (cx, cy, rad) = (ox + ux * scale, oy + uy * scale, r(k) * scale);
        let p = graded(spec, k, n);
        let teeth = if k.is_multiple_of(2) { 10 } else { 8 };
        let offset = if k.is_multiple_of(2) { 0.0 } else { 180.0 / teeth as f32 };
        let (tw, th) = (rad * 0.3, rad * 0.32);
        for t in 0..teeth {
            let ang = offset + 360.0 * t as f32 / teeth as f32;
            let (tx, ty) = polar(cx, cy, rad * 0.84, ang);
            out.push(plain("rect", [tx - tw / 2.0, ty - th / 2.0, tw, th], p).turned(ang - 90.0));
        }
        let body = rad * 0.82;
        let b = [cx - body, cy - body, body * 2.0, body * 2.0];
        let mut g = card("ellipse", b, p, spec, i);
        g.tb = inset(b, ROUND_INSET);
        out.push(g);
    }
    out
}

/// Arrows up and down in turn, each item's text beside its arrow.
pub(crate) fn arrows_opposing(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let aw = (cw * 0.32).min(h * 0.4);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + cw * 0.04;
        let up = k.is_multiple_of(2);
        let mut a = plain(if up { "upArrow" } else { "downArrow" }, [x, h * 0.04, aw, h * 0.92], graded(spec, k, n)).of_item(i);
        a.adj = vec![("adj1", 55_000), ("adj2", 70_000)];
        out.push(a);
        let tx = x + aw + cw * 0.04;
        out.push(label(spec, i, [tx, h * 0.04, (cw * 0.96 - aw - cw * 0.08).max(0.0), h * 0.92], "l", if up { "t" } else { "b" }));
    }
    out
}

/// The items in circles joined by plus signs, the last after an equals sign.
pub(crate) fn equation(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    const OP: f32 = 0.42;
    const GAP: f32 = 0.1;
    let d = (w * 0.98 / (n as f32 + (n - 1) as f32 * (OP + 2.0 * GAP))).min(h * 0.9);
    let total = n as f32 * d + (n - 1) as f32 * d * (OP + 2.0 * GAP);
    let cy = h / 2.0;
    let mut x = (w - total) / 2.0;
    let op_paint = tint(spec, 0, 60_000);
    let (o, t) = (d * OP, d * OP * 0.22);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        if k > 0 {
            let ox = x + d * GAP;
            if k + 1 == tops.len() {
                out.push(plain("rect", [ox, cy - o * 0.25 - t / 2.0, o, t], op_paint));
                out.push(plain("rect", [ox, cy + o * 0.25 - t / 2.0, o, t], op_paint));
            } else {
                out.push(plain("rect", [ox, cy - t / 2.0, o, t], op_paint));
                out.push(plain("rect", [ox + o / 2.0 - t / 2.0, cy - o / 2.0, t, o], op_paint));
            }
            x += d * (OP + 2.0 * GAP);
        }
        let b = [x, cy - d / 2.0, d, d];
        let mut c = card("ellipse", b, graded(spec, k, n), spec, i);
        c.tb = inset(b, ROUND_INSET);
        out.push(c);
        x += d;
    }
    out
}

/// Circles overlapping in a row; each item's text in the part of its circle no other covers.
pub(crate) fn venn_linear(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = (w * 0.98 / (1.0 + 0.75 * (n - 1) as f32)).min(h * 0.96);
    let x0 = (w - d * (1.0 + 0.75 * (n - 1) as f32)) / 2.0;
    let y = (h - d) / 2.0;
    let at = |k: usize| x0 + d * 0.75 * k as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let mut s = plain("ellipse", [at(k), y, d, d], Paint::Accent(spec.colors.accent(k), Some(40_000))).of_item(i);
        s.alpha = Some(60_000);
        out.push(s);
    }
    for k in 0..tops.len() {
        let mut s = plain("ellipse", [at(k), y, d, d], Paint::None);
        s.line = Paint::Accent(spec.colors.accent(k), None);
        s.line_w = 1.5;
        out.push(s);
    }
    for (k, &i) in tops.iter().enumerate() {
        let tw = if n == 1 { d * 0.7 } else { d * 0.44 };
        let b = [at(k) + (d - tw) / 2.0, y + d * 0.2, tw, d * 0.6];
        let mut s = label(spec, i, b, "ctr", "ctr");
        s.ink = Ink::Dark;
        out.push(s);
    }
    out
}

/// Rings inside each other, the first item outermost; each item's text to the right of the
/// target, joined to its ring by a leader line.
pub(crate) fn target(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let big = (h * 0.96).min(w * 0.56);
    let r0 = big / 2.0;
    let (cx, cy) = (w * 0.01 + r0, h / 2.0);
    let band = r0 / n as f32;
    let radius = |k: usize| r0 - band * k as f32;
    let one = spec.colors != wordcraft_doc::smart_art::SmartArtColors::Colorful;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let r = radius(k);
        // Light rings, stronger towards the middle (leader lines and text stay on light ground).
        let keep = if n > 1 { 22_000 + 43_000 * k as u32 / (n - 1) as u32 } else { 45_000 };
        let p = Paint::Accent(spec.colors.accent(if one { 0 } else { k }), Some(keep));
        let pick = [cx - band / 2.0, cy - r, band, band];
        let mut s = plain("ellipse", [cx - r, cy - r, 2.0 * r, 2.0 * r], p).of_item(i).picked_by(pick);
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
    }
    let lx = cx + r0 + w * 0.06;
    let rh = h / n as f32;
    for (k, &i) in tops.iter().enumerate() {
        let ly = rh * (k as f32 + 0.5);
        let rm = radius(k) - band / 2.0;
        let s = ((ly - cy) / rm.max(0.01)).clamp(-0.9, 0.9);
        let (px, py) = (cx + rm * (1.0 - s * s).sqrt(), cy + rm * s);
        let mut lead = Sh::line(px, py, lx - w * 0.01, ly, spec.colors.accent(k));
        lead.line_w = 1.0;
        out.push(lead);
        let d = band.min(rh) * 0.22;
        out.push(plain("ellipse", [px - d / 2.0, py - d / 2.0, d, d], Paint::Accent(spec.colors.accent(k), None)));
        out.push(label(spec, i, [lx, rh * k as f32, (w * 0.99 - lx).max(0.0), rh], "l", "ctr"));
    }
    out
}

/// The first item in a circle in the middle; arrows out to the items under it in blocks round it.
pub(crate) fn radial_diverging(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let (cx, cy) = (w / 2.0, h / 2.0);
    let m = w.min(h);
    let mut out = Vec::new();
    if n == 0 {
        let d = m * 0.5;
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        let mut c = card("ellipse", b, graded(spec, 0, 1), spec, 0);
        c.text = lines_of(spec, 0, true, false);
        c.tb = inset(b, ROUND_INSET);
        out.push(c);
        return out;
    }
    let bw = (w * 0.24).min(m * 0.5);
    let bh = (h * 0.2).min(bw * 0.62);
    let (rx, ry) = (((w - bw) / 2.0).max(0.0), ((h - bh) / 2.0).max(0.0));
    let dc = (m * 0.32).min(rx.min(ry) * 1.1).max(m * 0.1);
    let angle = |k: usize| -90.0 + 360.0 * k as f32 / n as f32;
    let at = |k: usize| polar(cx, cy, 1.0, angle(k));
    for k in 0..n {
        let (ux, uy) = at(k);
        let (ex, ey) = (cx + (ux - cx) * rx, cy + (uy - cy) * ry);
        // Halfway between the circle's edge and the block, pointing out.
        let dist = ((ex - cx).powi(2) + (ey - cy).powi(2)).sqrt();
        let len = ((dist - dc / 2.0) * 0.32).max(1.0).min(dist);
        let th = (len * 0.7).min(dc * 0.3);
        let mid = dc / 2.0 + (dist - dc / 2.0) * 0.45;
        let (ax, ay) = (cx + (ex - cx) / dist.max(0.01) * mid, cy + (ey - cy) / dist.max(0.01) * mid);
        let deg = (ey - cy).atan2(ex - cx).to_degrees();
        let mut a = plain("rightArrow", [ax - len / 2.0, ay - th / 2.0, len, th], tint(spec, k + 1, 60_000)).turned(deg);
        a.adj = vec![("adj1", 55_000), ("adj2", 60_000)];
        out.push(a);
    }
    let bc = [cx - dc / 2.0, cy - dc / 2.0, dc, dc];
    let mut c = card("ellipse", bc, graded(spec, 0, 1), spec, 0);
    c.text = lines_of(spec, 0, true, false);
    c.tb = inset(bc, ROUND_INSET);
    out.push(c);
    for (k, &i) in around.iter().enumerate() {
        let (ux, uy) = at(k);
        let (ex, ey) = (cx + (ux - cx) * rx, cy + (uy - cy) * ry);
        let mut s = card("roundRect", [ex - bw / 2.0, ey - bh / 2.0, bw, bh], graded(spec, k + 1, n + 1), spec, i);
        s.adj = vec![("adj", 12_000)];
        out.push(s);
    }
    out
}

// ---- fourth batch ----

/// The text box of an arrow pointing `right` (or left) in `b`: its shaft, short of its head.
fn shaft_box(b: [f32; 4], right: bool) -> [f32; 4] {
    let head = b[2].min(b[3]) * 0.5;
    let pad = b[2] * 0.05;
    let x = if right { b[0] + pad } else { b[0] + head };
    [x, b[1] + b[3] * 0.14, (b[2] - head - pad).max(0.0), b[3] * 0.72]
}

/// An arrow (`rightArrow` or `leftArrow`) in `b` holding item `i`'s text.
fn text_arrow(spec: &SmartArtSpec, i: usize, b: [f32; 4], right: bool, fill: Paint) -> Sh {
    let mut a = card(if right { "rightArrow" } else { "leftArrow" }, b, fill, spec, i);
    a.adj = vec![("adj1", 76_000), ("adj2", 50_000)];
    a.tb = shaft_box(b, right);
    a
}

/// The first half of the top items on the left, the rest on the right, each in an arrow pointing
/// in to the middle (`inward`) or out from it; more than one a side are stacked.
fn arrows_sides(spec: &SmartArtSpec, w: f32, h: f32, inward: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let left = n.div_ceil(2);
    let rows = left.max(1);
    let rh = h / rows as f32;
    let aw = if n == 1 { w * 0.9 } else { w * 0.47 };
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let on_left = k < left;
        let row = if on_left { k } else { k - left };
        let x = if n == 1 {
            w * 0.05
        } else if on_left {
            w * 0.02
        } else {
            w * 0.51
        };
        let ah = (rh * 0.84).min(aw * 0.6);
        let b = [x, rh * row as f32 + (rh - ah) / 2.0, aw, ah];
        out.push(text_arrow(spec, i, b, on_left == inward, graded(spec, k, n)));
    }
    out
}

pub(crate) fn arrows_converging(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    arrows_sides(spec, w, h, true)
}

pub(crate) fn arrows_diverging(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    arrows_sides(spec, w, h, false)
}

/// Wide arrows stacked, pointing right and left in turn (offset), each item's text inside.
pub(crate) fn arrows_counterbalance(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let right = k % 2 == 0;
        let ah = (rh * 0.84).min(w * 0.4);
        let b = [if right { w * 0.01 } else { w * 0.13 }, rh * k as f32 + (rh - ah) / 2.0, w * 0.86, ah];
        out.push(text_arrow(spec, i, b, right, graded(spec, k, n)));
    }
    out
}

/// The first item in a circle in the middle and the items under it in see-through circles over
/// its edge; each item's text in its own circle, away from the others.
pub(crate) fn venn_radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let dc = if n == 0 { m * 0.7 } else { m * 0.56 };
    let rc = dc / 2.0;
    let chord = if n <= 1 { m } else { 2.0 * rc * (std::f32::consts::PI / n as f32).sin() };
    let ds = (m * 0.4).min(m - dc - m * 0.02).min(chord * 1.9).max(m * 0.1);
    let one = spec.colors != wordcraft_doc::smart_art::SmartArtColors::Colorful;
    let mut centre =
        plain("ellipse", [cx - rc, cy - rc, dc, dc], Paint::Accent(spec.colors.accent(0), Some(if one { 30_000 } else { 40_000 }))).of_item(0);
    centre.line = Paint::Accent(spec.colors.accent(0), None);
    centre.line_w = 1.5;
    let mut out = vec![centre];
    let mut labels = Vec::new();
    let tc = dc * if n == 0 { 0.6 } else { 0.44 };
    let mut centre_text = label(spec, 0, [cx - tc / 2.0, cy - tc / 2.0, tc, tc], "ctr", "ctr");
    centre_text.text = lines_of(spec, 0, true, n == 0);
    labels.push(centre_text);
    for (k, &i) in around.iter().enumerate() {
        let a = angle(k, n);
        let (x, y) = polar(cx, cy, rc, a);
        let mut s = plain("ellipse", [x - ds / 2.0, y - ds / 2.0, ds, ds], Paint::Accent(spec.colors.accent(k + 1), Some(45_000))).of_item(i);
        s.alpha = Some(60_000);
        s.line = Paint::Accent(spec.colors.accent(k + 1), None);
        s.line_w = 1.0;
        out.push(s);
        let (tx, ty) = polar(cx, cy, rc + ds * 0.2, a);
        let t = ds * 0.5;
        labels.push(label(spec, i, [tx - t / 2.0, ty - t * 0.35, t, t * 0.7], "ctr", "ctr"));
    }
    out.extend(labels);
    for s in &mut out {
        if !s.text.is_empty() {
            s.ink = Ink::Dark;
        }
    }
    out
}

/// See-through circles inside each other, all standing on one point at the bottom, the first the
/// largest; each item's text at the top of its own band.
pub(crate) fn venn_stacked(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let big = (h * 0.98).min(w * 0.98);
    let d = |k: usize| if n > 1 { big * (1.0 - 0.62 * k as f32 / (n - 1) as f32) } else { big };
    let (cx, bottom) = (w / 2.0, h * 0.99);
    let mut out = Vec::new();
    let mut labels = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let dk = d(k);
        let top = bottom - dk;
        let band = if k + 1 < tops.len() { dk - d(k + 1) } else { dk };
        let tb = if k + 1 < tops.len() {
            // The chord half way down the band.
            let r = dk / 2.0;
            let depth = band * 0.5;
            let chord = 2.0 * (r * r - (r - depth).powi(2)).max(0.0).sqrt();
            let tw = (chord * 0.86).min(dk * 0.6);
            [cx - tw / 2.0, top + band * 0.1, tw, band * 0.8]
        } else {
            inset([cx - dk / 2.0, top, dk, dk], ROUND_INSET)
        };
        let mut s = plain(
            "ellipse",
            [cx - dk / 2.0, top, dk, dk],
            Paint::Accent(spec.colors.accent(k), Some(if n > 1 { 30_000 + 25_000 * k as u32 / (n - 1) as u32 } else { 40_000 })),
        )
        .of_item(i)
        .picked_by(tb);
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
        labels.push(label(spec, i, tb, "ctr", "ctr"));
    }
    out.extend(labels);
    out
}

/// Rounded rectangles inside each other, the first outermost; each item's text at the top of its
/// own band (the innermost holds its text in the middle).
pub(crate) fn target_nested(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let (x0, y0, ww, hh) = (w * 0.01, h * 0.01, w * 0.98, h * 0.98);
    let t = hh / (1.25 * (n - 1) as f32 + 1.4);
    let sx = (ww * 0.04).min(ww / (2.0 * n as f32 + 1.0));
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let kf = k as f32;
        let b = [x0 + sx * kf, y0 + t * kf, (ww - 2.0 * sx * kf).max(0.0), (hh - 1.25 * t * kf).max(0.0)];
        let last = k + 1 == tops.len();
        let keep = if n > 1 { 18_000 + 52_000 * k as u32 / (n - 1) as u32 } else { 40_000 };
        let tb = if last { inset(b, 0.08) } else { [b[0] + sx * 0.5, b[1], (b[2] - sx).max(0.0), t] };
        let mut s =
            card("roundRect", b, tint(spec, if spec.colors == wordcraft_doc::smart_art::SmartArtColors::Colorful { k } else { 0 }, keep), spec, i)
                .picked_by(tb);
        s.adj = vec![("adj", 6_000)];
        s.tb = tb;
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
    }
    out
}

/// The first item in a large light circle, the items under it in small filled circles round its edge.
pub(crate) fn circle_relationship(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let dc = m * if n == 0 { 0.8 } else { 0.66 };
    let rc = dc / 2.0;
    let chord = if n <= 1 { m } else { 2.0 * rc * (std::f32::consts::PI / n as f32).sin() };
    let ds = (m * 0.28).min(m * 0.98 - dc).min(chord * 0.92).max(m * 0.06);
    let bc = [cx - rc, cy - rc, dc, dc];
    let mut c = card("ellipse", bc, tint(spec, 0, 25_000), spec, 0);
    c.text = lines_of(spec, 0, true, false);
    c.tb = inset(bc, if n == 0 { ROUND_INSET } else { 0.3 });
    c.line = Paint::Accent(spec.colors.accent(0), Some(60_000));
    c.line_w = 1.5;
    let mut out = vec![c];
    for (k, &i) in around.iter().enumerate() {
        let (x, y) = polar(cx, cy, rc, angle(k, n) + 45.0);
        let b = [x - ds / 2.0, y - ds / 2.0, ds, ds];
        let mut s = card("ellipse", b, graded(spec, k + 1, n + 1), spec, i);
        s.tb = inset(b, ROUND_INSET);
        s.line = Paint::Light;
        s.line_w = 2.0;
        out.push(s);
    }
    out
}

/// Columns under a plus sign and a minus sign in turn, each item's text in a light box under its sign.
pub(crate) fn plus_minus(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let s = (cw * 0.3).min(h * 0.22);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32;
        let top = h * 0.04 + s;
        let b = [x + cw * 0.05, top, cw * 0.9, (h * 0.99 - top).max(0.0)];
        let mut c = card("roundRect", b, graded_light(spec, k, n, 22_000, 14_000), spec, i);
        c.adj = vec![("adj", 6_000)];
        c.tb = [b[0] + b[2] * 0.08, b[1] + b[3] * 0.06, b[2] * 0.84, b[3] * 0.88];
        c.anchor = "t";
        out.push(c);
        out.extend(sign(x + cw / 2.0, h * 0.02 + s / 2.0, s, k % 2 == 0, graded(spec, k, n)));
    }
    out
}

/// Rows: the items under each top item in light boxes on the left, arrows pointing from each box
/// into a circle holding the top item.
pub(crate) fn text_converging(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let size = |i: usize| spec.descendants(i).len().max(1) as f32;
    let total: f32 = tops.iter().map(|&i| size(i)).sum::<f32>().max(1.0);
    let unit = h / total;
    let mut out = Vec::new();
    let mut boxes = Vec::new();
    let mut y0 = 0.0;
    for (k, &i) in tops.iter().enumerate() {
        let rh = size(i) * unit;
        let kids: Vec<usize> = spec.descendants(i).collect();
        let d = (rh * 0.9).min(w * 0.3);
        let (ccx, ccy) = (if kids.is_empty() { w / 2.0 } else { w * 0.99 - d / 2.0 }, y0 + rh / 2.0);
        let bw = w * 0.36;
        for (j, &c) in kids.iter().enumerate() {
            let bh = unit * 0.8;
            let b = [w * 0.01, y0 + unit * j as f32 + unit * 0.1, bw, bh];
            let mut s = tag("roundRect", b, graded_light(spec, k, tops.len(), 26_000, 16_000), spec, c);
            s.adj = vec![("adj", 12_000)];
            boxes.push(s);
            // An arrow from the box's right end towards the circle, halfway.
            let (x0, y) = (b[0] + bw, b[1] + bh / 2.0);
            let (x1, y1) = (ccx - d / 2.0, ccy);
            let dist = (x1 - x0).hypot(y1 - y);
            let len = (dist * 0.6).max(0.0);
            let (mx, my) = ((x0 + x1) / 2.0, (y + y1) / 2.0);
            let room = 2.0 * mx.min(w - mx).min(my).min(h - my);
            let len = len.min(room);
            if len > 2.0 {
                let th = (bh * 0.36).min(len * 0.5);
                let deg = (y1 - y).atan2(x1 - x0).to_degrees();
                out.push(arrow("rightArrow", [mx - len / 2.0, my - th / 2.0, len, th], tint(spec, k, 55_000), 50_000, 60_000).turned_by(deg));
            }
        }
        let cb = [ccx - d / 2.0, ccy - d / 2.0, d, d];
        let mut c = tag("ellipse", cb, graded(spec, k, tops.len()), spec, i);
        c.tb = inset(cb, ROUND_INSET);
        boxes.push(c);
        y0 += rh;
    }
    out.extend(boxes);
    out
}

/// The first item in a large circle on the left, the items under it in small circles on an arc
/// beside it, each one's text to its right.
pub(crate) fn radial_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let dc = (h * 0.7).min(w * 0.34);
    let (cx, cy) = (w * 0.01 + dc / 2.0, h / 2.0);
    let mut out = Vec::new();
    let ds = (h * 0.96 / n.max(1) as f32 * 0.7).min(w * 0.1).min(dc * 0.5);
    let r = (dc / 2.0 + ds * 0.9).max((h / 2.0 - ds / 2.0) * 0.98);
    let y_at = |k: usize| ds / 2.0 + h * 0.01 + (h * 0.98 - ds) * (k as f32 + 0.5) / n.max(1) as f32;
    let x_at = |k: usize| {
        let dy = (y_at(k) - cy).abs().min(r);
        (cx + (r * r - dy * dy).sqrt()).min(w * 0.6)
    };
    for k in 0..n {
        let mut l = Sh::line(cx, cy, x_at(k), y_at(k), spec.colors.accent(k + 1));
        l.line_w = 1.0;
        out.push(l);
    }
    let bc = [cx - dc / 2.0, cy - dc / 2.0, dc, dc];
    let mut c = card("ellipse", bc, graded(spec, 0, 1), spec, 0);
    c.text = lines_of(spec, 0, true, false);
    c.tb = inset(bc, ROUND_INSET);
    c.line = Paint::Light;
    c.line_w = 2.0;
    out.push(c);
    for (k, &i) in around.iter().enumerate() {
        let (x, y) = (x_at(k), y_at(k));
        let mut s = plain("ellipse", [x - ds / 2.0, y - ds / 2.0, ds, ds], graded(spec, k + 1, n + 1)).of_item(i);
        s.line = Paint::Light;
        s.line_w = 2.0;
        out.push(s);
        let tx = x + ds * 0.7;
        let lh = (h / n.max(1) as f32).max(ds);
        out.push(label(spec, i, [tx, (y - lh / 2.0).clamp(0.0, (h - lh).max(0.0)), (w * 0.99 - tx).max(0.0), lh.min(h)], "l", "ctr"));
    }
    out
}

/// The items but the last in circles from top to bottom joined by plus signs; an arrow from them
/// to the last item in a larger circle on the right.
pub(crate) fn equation_vertical(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let mut out = Vec::new();
    let Some((&last, parts)) = tops.split_last() else { return out };
    let circle = |b: [f32; 4], k: usize, i: usize| {
        let mut c = card("ellipse", b, graded(spec, k, n), spec, i);
        c.tb = inset(b, ROUND_INSET);
        c
    };
    if parts.is_empty() {
        let d = w.min(h) * 0.7;
        out.push(circle([(w - d) / 2.0, (h - d) / 2.0, d, d], 0, last));
        return out;
    }
    let m = parts.len() as f32;
    const OP: f32 = 0.4;
    let d = (h * 0.98 / (m + (m - 1.0) * OP)).min(w * 0.3);
    let total = m * d + (m - 1.0) * d * OP;
    let mut y = (h - total) / 2.0;
    let x = w * 0.04;
    for (k, &i) in parts.iter().enumerate() {
        if k > 0 {
            out.extend(sign(x + d / 2.0, y + d * OP / 2.0, d * OP * 0.8, true, tint(spec, 0, 60_000)));
            y += d * OP;
        }
        out.push(circle([x, y, d, d], k, i));
        y += d;
    }
    let big = (h * 0.7).min(w * 0.36);
    let bx = w * 0.98 - big;
    let ax0 = x + d + w * 0.04;
    let len = (bx - w * 0.04 - ax0).max(0.0);
    if len > 2.0 {
        let th = (big * 0.3).min(len * 0.6);
        out.push(arrow("rightArrow", [ax0, h / 2.0 - th / 2.0, len, th], tint(spec, 0, 60_000), 50_000, 60_000));
    }
    out.push(circle([bx, (h - big) / 2.0, big, big], n - 1, last));
    out
}

/// Thick rings linked in a row, each item's text inside its ring.
pub(crate) fn rings_interconnected(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = (w * 0.96 / (1.0 + 0.8 * (n - 1) as f32)).min(h * 0.96);
    let x0 = (w - d * (1.0 + 0.8 * (n - 1) as f32)) / 2.0;
    let y = (h - d) / 2.0;
    let t = (d * 0.08).clamp(1.0, 14.0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = x0 + d * 0.8 * k as f32;
        let mut r = plain("ellipse", [x + t / 2.0, y + t / 2.0, d - t, d - t], Paint::None).of_item(i);
        r.line = Paint::Accent(
            spec.colors.accent(k),
            if spec.colors == wordcraft_doc::smart_art::SmartArtColors::Colorful { None } else { graded_keep(k, n) },
        );
        r.line_w = t;
        out.push(r);
    }
    for (k, &i) in tops.iter().enumerate() {
        let x = x0 + d * 0.8 * k as f32;
        let tw = d * if n == 1 { 0.6 } else { 0.5 };
        out.push(label(spec, i, [x + (d - tw) / 2.0, y + d * 0.25, tw, d * 0.5], "ctr", "ctr"));
    }
    out
}

/// The tint kept for item `k` of `n` in a one-accent graphic (full to half).
fn graded_keep(k: usize, n: usize) -> Option<u32> {
    if n <= 1 {
        return None;
    }
    let t = 100_000 - 50_000 * k.min(n - 1) as u32 / (n - 1) as u32;
    Some(t).filter(|t| *t < 100_000)
}

/// The first item in a block in the middle, the items under it in smaller blocks round it, joined
/// by lines.
pub(crate) fn radial_cluster(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let (cx, cy) = (w / 2.0, h / 2.0);
    let m = w.min(h);
    let (cw, ch) = ((w * 0.3).min(m * 0.6), (h * 0.26).min(m * 0.36));
    let bw = (w * 0.22).min(m * 0.46);
    let bh = (h * 0.18).min(bw * 0.6);
    let (rx, ry) = (((w - bw) / 2.0).max(0.0), ((h - bh) / 2.0).max(0.0));
    let at = |k: usize| {
        let a = angle(k, n.max(1)).to_radians();
        (cx + rx * a.cos(), cy + ry * a.sin())
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (x, y) = at(k);
        out.push(Sh::line(cx, cy, x, y, spec.colors.accent(k + 1)));
    }
    let b = [cx - cw / 2.0, cy - ch / 2.0, cw, ch];
    let mut c = card("roundRect", b, graded(spec, 0, 1), spec, 0);
    c.text = lines_of(spec, 0, true, n == 0);
    c.adj = vec![("adj", 12_000)];
    out.push(c);
    for (k, &i) in around.iter().enumerate() {
        let (x, y) = at(k);
        let mut s = card("roundRect", [x - bw / 2.0, y - bh / 2.0, bw, bh], graded(spec, k + 1, n + 1), spec, i);
        s.adj = vec![("adj", 12_000)];
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
    }
    out
}

/// The items under the first on an arc above, arrows pointing down to the first item in a circle
/// at the bottom middle.
pub(crate) fn radial_converging(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let n = around.len();
    let dc = (h * 0.4).min(w * 0.3);
    let (cx, cyc) = (w / 2.0, h * 0.99 - dc / 2.0);
    let bw = (w * 0.24).min(w * 0.94 / (n.max(1) as f32 * 0.7 + 0.3));
    let bh = (h * 0.2).min(bw * 0.6);
    let rx = ((w - bw) / 2.0 * 0.98).max(0.0);
    let ry = (cyc - bh / 2.0 - h * 0.01).max(0.0);
    let theta = |k: usize| if n <= 1 { 270.0f32 } else { 200.0 + 140.0 * k as f32 / (n - 1) as f32 };
    let at = |k: usize| {
        let t = theta(k).to_radians();
        (cx + rx * t.cos(), cyc + ry * t.sin())
    };
    let mut out = Vec::new();
    for k in 0..n {
        let (bx, by) = at(k);
        let (dx, dy) = (cx - bx, cyc - by);
        let dist = dx.hypot(dy).max(0.01);
        // How far the block reaches from its middle towards the circle.
        let (ux, uy) = ((dx / dist).abs().max(1.0e-3), (dy / dist).abs().max(1.0e-3));
        let reach = (bw / 2.0 / ux).min(bh / 2.0 / uy);
        let free = (dist - dc / 2.0 - reach).max(0.0);
        let len = (free * 0.6).max(0.0);
        let mid = reach + free * 0.5;
        let (mx, my) = (bx + dx / dist * mid, by + dy / dist * mid);
        let room = 2.0 * mx.min(w - mx).min(my).min(h - my);
        let len = len.min(room);
        if len > 2.0 {
            let th = (len * 0.5).min(bh * 0.5);
            out.push(
                arrow("rightArrow", [mx - len / 2.0, my - th / 2.0, len, th], tint(spec, k + 1, 60_000), 50_000, 60_000)
                    .turned_by(dy.atan2(dx).to_degrees()),
            );
        }
    }
    let bc = [cx - dc / 2.0, cyc - dc / 2.0, dc, dc];
    let mut c = card("ellipse", bc, graded(spec, 0, 1), spec, 0);
    c.text = lines_of(spec, 0, true, n == 0);
    c.tb = inset(bc, ROUND_INSET);
    out.push(c);
    for (k, &i) in around.iter().enumerate() {
        let (x, y) = at(k);
        let mut s = card("roundRect", [x - bw / 2.0, y - bh / 2.0, bw, bh], graded(spec, k + 1, n + 1), spec, i);
        s.adj = vec![("adj", 12_000)];
        out.push(s);
    }
    out
}
