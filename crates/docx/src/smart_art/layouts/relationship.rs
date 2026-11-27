//! Relationship layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use super::matrix::spokes;
use crate::smart_art::shapes::{Ink, Paint, ROUND_INSET, Sh, card, graded, inset, label, lines_of, plain, polar, tint, tops};

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
