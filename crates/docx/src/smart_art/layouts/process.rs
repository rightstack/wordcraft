//! Process layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Paint, ROUND_INSET, Sh, best_cols, card, graded, graded_light, inset, label, lines_of, plain, tag, tint, tops};

/// Chevrons in a row, each point in the next one's notch, each item's text inside.
pub(crate) fn chevrons(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let span = w * 0.98;
    // Height 45% of the width while that fits; the point is half the height deep.
    let mut cw = span / (0.82 * (n - 1.0) + 1.0);
    let mut ch = cw * 0.45;
    if ch > h * 0.6 {
        ch = h * 0.6;
        cw = (span + 0.4 * ch * (n - 1.0)) / n;
    }
    let p = ch.min(cw) * 0.5;
    let step = cw - p * 0.8;
    let (x0, y) = ((w - (step * (n - 1.0) + cw)) / 2.0, (h - ch) / 2.0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let b = [x0 + step * k as f32, y, cw, ch];
        let mut s = card("chevron", b, graded(spec, k, n as usize), spec, i);
        s.adj = vec![("adj", 50_000)];
        s.tb = [b[0] + p, y + ch * 0.1, (cw - 2.0 * p).max(cw * 0.3), ch * 0.8];
        if s.tb[0] + s.tb[2] > b[0] + cw {
            s.tb[0] = b[0] + (cw - s.tb[2]) / 2.0;
        }
        out.push(s);
    }
    out
}

/// Chevrons pointing down the left side, each holding its item's own line, the lines under it
/// in a tinted box beside it.
pub(crate) fn chevron_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    // Chevron `len` along the flow (down), `thick` across; neighbours overlap by 15% of a row.
    let rh = h / (n + 0.15);
    let len = (rh * 1.15).min(w * 0.4);
    let thick = (rh * 0.85).min(w * 0.28).min(len);
    let cx = len.max(thick) / 2.0;
    let bx = cx + thick / 2.0 + w * 0.03;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cy = rh * (k as f32 + 0.575);
        let p = graded(spec, k, n as usize);
        // Drawn across (unturned box `len` wide), then turned a quarter clockwise to point down.
        let b = [cx - len / 2.0, cy - thick / 2.0, len, thick];
        let mut c = tag("chevron", b, p, spec, i).turned(90.0);
        c.adj = vec![("adj", 35_000)];
        let tl = thick * 0.8;
        c.tb = [cx - tl / 2.0, cy - len * 0.16, tl, len * 0.38];
        out.push(c);
        let body = [bx, cy - rh * 0.42, (w * 0.99 - bx).max(0.0), rh * 0.84];
        let mut s = plain("rect", body, graded_light(spec, k, n as usize, 18_000, 10_000)).of_item(i);
        s.text = lines_of(spec, i, false, true);
        s.tb = [body[0] + body[2] * 0.04, body[1], body[2] * 0.92, body[3]];
        s.algn = Some("l");
        out.push(s);
    }
    out
}

/// Blocks rising left to right like stairs, each joined to the next by a step-shaped line.
pub(crate) fn step_up(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bw = cw * 0.84;
    let bh = (h * 0.42).min(bw * 0.7).min(h / (1.0 + 0.5 * (n - 1) as f32));
    let rise = if n > 1 { (h * 0.98 - bh) / (n - 1) as f32 } else { 0.0 };
    let y = |k: usize| h * 0.99 - bh - rise * k as f32;
    let mut out = Vec::new();
    for k in 1..tops.len() {
        let (x0, y0) = (cw * (k - 1) as f32 + (cw + bw) / 2.0, y(k - 1) + bh / 2.0);
        let (x1, y1) = (cw * k as f32 + cw / 2.0, y(k) + bh);
        let a = spec.colors.accent(k - 1);
        out.push(Sh::line(x0, y0, x1, y0, a));
        out.push(Sh::line(x1, y0, x1, y1, a));
    }
    for (k, &i) in tops.iter().enumerate() {
        let mut s = card("roundRect", [cw * k as f32 + (cw - bw) / 2.0, y(k).max(0.0), bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 12_000)];
        out.push(s);
    }
    out
}

/// Blocks from top to bottom with an arrow down between each.
pub(crate) fn process_vertical(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    const GAP: f32 = 0.45;
    let bh = h / (n as f32 + (n - 1) as f32 * GAP);
    let bw = (w * 0.9).min(bh * 5.0).max(w * 0.4);
    let x = (w - bw) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = bh * (1.0 + GAP) * k as f32;
        if k > 0 {
            let gy = y - bh * GAP;
            let (aw, ah) = ((bh * 0.5).min(bw * 0.3), bh * GAP * 0.7);
            out.push(plain("downArrow", [w / 2.0 - aw / 2.0, gy + bh * GAP * 0.15, aw, ah], tint(spec, k - 1, 60_000)));
        }
        let mut s = card("roundRect", [x, y, bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 14_000)];
        out.push(s);
    }
    out
}

/// Blocks in rows, every row left to right with arrows between; a line bends from the end of
/// each row down to the start of the next.
pub(crate) fn process_bending(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    const GX: f32 = 0.4;
    const GY: f32 = 0.5;
    // Columns that make the blocks (3:2, with gaps) largest.
    let cols = best_cols(n, w / (1.0 + GX), h / (1.0 + GY), 1.5 * (1.0 + GY) / (1.0 + GX));
    let rows = n.div_ceil(cols);
    let bw = (w / (cols as f32 + (cols - 1) as f32 * GX)).min(h / (rows as f32 + (rows - 1) as f32 * GY) * 1.5);
    let bh = bw / 1.5;
    let (gx, gy) = (bw * GX, bh * GY);
    let x0 = (w - (cols as f32 * bw + (cols - 1) as f32 * gx)) / 2.0;
    let y0 = (h - (rows as f32 * bh + (rows - 1) as f32 * gy)) / 2.0;
    let at = |k: usize| (x0 + (k % cols) as f32 * (bw + gx), y0 + (k / cols) as f32 * (bh + gy));
    let mut out = Vec::new();
    for k in 1..tops.len() {
        let (px, py) = at(k - 1);
        let (x, y) = at(k);
        let a = spec.colors.accent(k - 1);
        if k % cols == 0 {
            // Down from the row's last block, back across, down to the next row's first.
            let mid = py + bh + gy / 2.0;
            out.push(Sh::line(px + bw / 2.0, py + bh, px + bw / 2.0, mid, a));
            out.push(Sh::line(x + bw / 2.0, mid, px + bw / 2.0, mid, a));
            out.push(Sh::line(x + bw / 2.0, mid, x + bw / 2.0, y, a));
        } else {
            let (aw, ah) = (gx * 0.6, (bh * 0.3).min(gx * 0.6));
            out.push(plain("rightArrow", [px + bw + (gx - aw) / 2.0, py + bh / 2.0 - ah / 2.0, aw, ah], tint(spec, k - 1, 60_000)));
        }
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = at(k);
        let mut s = card("roundRect", [x, y, bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 12_000)];
        out.push(s);
    }
    out
}

/// Blocks in a row on a wide light arrow that runs across the graphic.
pub(crate) fn process_continuous(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let ah = (h * 0.62).min(w * 0.4);
    let cy = h / 2.0;
    let mut arrow = plain("rightArrow", [0.0, cy - ah / 2.0, w, ah], tint(spec, 0, 45_000));
    arrow.adj = vec![("adj1", 70_000), ("adj2", 40_000)];
    let head = ah * 0.4;
    let lane = (w - head).max(0.0);
    let cw = lane / n as f32;
    let bw = cw * 0.84;
    let bh = (ah * 0.46).min(bw * 0.9);
    let mut out = vec![arrow];
    for (k, &i) in tops.iter().enumerate() {
        let mut s = card("roundRect", [cw * k as f32 + (cw - bw) / 2.0, cy - bh / 2.0, bw, bh], graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 12_000)];
        s.line = Paint::Light;
        s.line_w = 1.5;
        out.push(s);
    }
    out
}

/// A light arrow rising from bottom left to top right, a dot per item along it and the item's
/// text above its dot.
pub(crate) fn arrow_upward(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    // The arrow along the diagonal of the lower right part of the frame.
    let (run, climb) = (w * 0.9, h * 0.62);
    let deg = (climb / run.max(0.01)).atan().to_degrees().min(35.0);
    let len = (run / deg.to_radians().cos()).min(w * 0.96);
    let th = (h * 0.14).min(len * 0.12);
    let (cx, cy) = (w / 2.0, h * 0.58);
    let mut arrow = plain("rightArrow", [cx - len / 2.0, cy - th / 2.0, len, th], tint(spec, 0, 35_000)).turned(-deg);
    arrow.adj = vec![("adj1", 60_000), ("adj2", 120_000)];
    let mut out = vec![arrow];
    let (dx, dy) = (deg.to_radians().cos(), -deg.to_radians().sin());
    let d = (th * 0.75).max(1.0);
    let cw = w / n as f32;
    for (k, &i) in tops.iter().enumerate() {
        // Along the arrow's shaft (short of its head).
        let t = -0.42 + 0.78 * (k as f32 + 0.5) / n as f32;
        let (x, y) = (cx + dx * len * t, cy + dy * len * t);
        let mut dot = plain("ellipse", [x - d / 2.0, y - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 1.5;
        out.push(dot);
        let lw = cw * 0.96;
        let bottom = y - d * 0.9;
        let top = (bottom - h * 0.36).max(h * 0.01);
        let b = [(x - lw / 2.0).clamp(0.0, (w - lw).max(0.0)), top, lw, (bottom - top).max(0.0)];
        out.push(label(spec, i, b, "ctr", "b"));
    }
    out
}

/// Columns: the item's own line in a circle over a light arrow down that holds the lines under it.
pub(crate) fn process_arrows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.6).min(h * 0.36);
    let aw = cw * 0.84;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32;
        let ay = h * 0.01 + d * 0.62;
        let b = [x + (cw - aw) / 2.0, ay, aw, (h * 0.99 - ay).max(0.0)];
        let mut a = plain("downArrow", b, graded_light(spec, k, n, 26_000, 14_000)).of_item(i);
        a.adj = vec![("adj1", 74_000), ("adj2", 40_000)];
        a.text = lines_of(spec, i, false, true);
        let head = (aw.min(b[3]) * 0.4).min(b[3] * 0.5);
        let ty = ay + d * 0.45;
        a.tb = [b[0] + aw * 0.18, ty, aw * 0.64, (b[1] + b[3] - head - ty).max(0.0)];
        a.anchor = "t";
        out.push(a);
        let cb = [x + (cw - d) / 2.0, h * 0.01, d, d];
        let mut c = tag("ellipse", cb, graded(spec, k, n), spec, i);
        c.tb = inset(cb, ROUND_INSET);
        c.line = Paint::Light;
        c.line_w = 2.0;
        out.push(c);
    }
    out
}
