//! Process layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{
    Paint, ROUND_INSET, Sh, arrow, best_cols, card, graded, graded_light, inset, kids_label, label, lines_of, plain, tag, tint, tops,
};

/// Chevrons in a row, each point in the next one's notch, each item's text inside.
pub(crate) fn chevrons(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    chevron_row(spec, w, h, false)
}

/// The chevron row, the first a flat-backed arrow when `closed`.
fn chevron_row(spec: &SmartArtSpec, w: f32, h: f32, closed: bool) -> Vec<Sh> {
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
        let flat = closed && k == 0;
        let mut s = card(if flat { "homePlate" } else { "chevron" }, b, graded(spec, k, n as usize), spec, i);
        s.adj = vec![("adj", 50_000)];
        s.tb = if flat {
            [b[0] + cw * 0.06, y + ch * 0.1, (cw - p - cw * 0.06).max(cw * 0.3), ch * 0.8]
        } else {
            [b[0] + p, y + ch * 0.1, (cw - 2.0 * p).max(cw * 0.3), ch * 0.8]
        };
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

// ---- fourth batch ----

/// An arrow pointing right, centred in the gap from `x0` to `x1` at height `cy`.
fn gap_arrow(x0: f32, x1: f32, cy: f32, th: f32, paint: Paint) -> Sh {
    let len = ((x1 - x0) * 0.64).max(0.0);
    let th = th.min(len * 1.2);
    arrow("rightArrow", [x0 + (x1 - x0 - len) / 2.0, cy - th / 2.0, len, th], paint, 50_000, 60_000)
}

/// Columns: each top item's own line in a filled block over a light box with the lines under it;
/// arrows between the blocks.
pub(crate) fn process_accent(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bw = cw * if n > 1 { 0.76 } else { 0.9 };
    let head = h * 0.3;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + if n > 1 { 0.0 } else { (cw - bw) / 2.0 };
        let body = [x, h * 0.01 + head, bw, h * 0.98 - head];
        let mut s = plain("rect", body, graded_light(spec, k, n, 24_000, 14_000)).of_item(i);
        s.text = lines_of(spec, i, false, true);
        s.tb = [body[0] + bw * 0.08, body[1] + body[3] * 0.06, bw * 0.84, body[3] * 0.88];
        s.algn = Some("l");
        s.anchor = "t";
        out.push(s);
        out.push(tag("rect", [x, h * 0.01, bw, head], graded(spec, k, n), spec, i));
        if k + 1 < tops.len() {
            out.push(gap_arrow(x + bw, cw * (k + 1) as f32, h * 0.01 + head / 2.0, head * 0.4, tint(spec, k, 60_000)));
        }
    }
    out
}

/// Filled blocks in a row with arrows between; each top item's lines in a light box above and
/// below in turn.
pub(crate) fn process_alternating_flow(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bw = cw * if n > 1 { 0.74 } else { 0.9 };
    let (by, bh) = (h * 0.37, h * 0.26);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + if n > 1 { 0.0 } else { (cw - bw) / 2.0 };
        let db = if k % 2 == 0 { [x, h * 0.01, bw, h * 0.34] } else { [x, h * 0.65, bw, h * 0.34] };
        let mut d = plain("rect", db, graded_light(spec, k, n, 22_000, 12_000)).of_item(i);
        d.text = lines_of(spec, i, false, true);
        d.tb = inset(db, 0.06);
        out.push(d);
        out.push(tag("roundRect", [x, by, bw, bh], graded(spec, k, n), spec, i));
        if k + 1 < tops.len() {
            out.push(gap_arrow(x + bw, cw * (k + 1) as f32, by + bh / 2.0, bh * 0.45, tint(spec, k, 60_000)));
        }
    }
    out
}

/// A flat-backed arrow then chevrons nested point to notch, each item's text inside.
pub(crate) fn chevrons_closed(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    chevron_row(spec, w, h, true)
}

/// Light chevrons in a row with gaps, each top item's own line inside and the lines under it below.
pub(crate) fn chevrons_detailed(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bw = cw * 0.96;
    let ch = (bw * 0.42).min(h * 0.34);
    let y = h * 0.03;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + (cw - bw) / 2.0;
        let b = [x, y, bw, ch];
        let p = ch.min(bw) * 0.5;
        let mut s = tag("chevron", b, graded_light(spec, k, n, 50_000, 30_000), spec, i);
        s.adj = vec![("adj", 50_000)];
        s.tb = [x + p, y + ch * 0.1, (bw - 2.0 * p).max(bw * 0.3), ch * 0.8];
        if s.tb[0] + s.tb[2] > x + bw {
            s.tb[0] = x + (bw - s.tb[2]) / 2.0;
        }
        out.push(s);
        let top = y + ch + h * 0.04;
        out.push(kids_label(spec, i, [x + p * 0.5, top, (bw - p).max(0.0), (h * 0.99 - top).max(0.0)], "l", "t"));
    }
    out
}

/// Filled circles in a row holding each top item's own line, arrows between, the lines under it
/// below its circle.
pub(crate) fn process_circle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.66).min(h * 0.52);
    let y = h * 0.02;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32;
        let b = [x + (cw - d) / 2.0, y, d, d];
        let mut c = tag("ellipse", b, graded(spec, k, n), spec, i);
        c.tb = inset(b, ROUND_INSET);
        out.push(c);
        let top = y + d + h * 0.03;
        out.push(kids_label(spec, i, [x + cw * 0.04, top, cw * 0.92, (h * 0.99 - top).max(0.0)], "ctr", "t"));
        if k + 1 < tops.len() {
            out.push(gap_arrow(b[0] + d, x + cw + (cw - d) / 2.0, y + d / 2.0, d * 0.22, tint(spec, k, 60_000)));
        }
    }
    out
}

/// Rows: each top item's bar across its row, the items under it as boxes side by side below the
/// bar; small arrows down between the rows.
pub(crate) fn process_segmented(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    const GAP: f32 = 0.25;
    let rh = h / (n as f32 + GAP * (n - 1) as f32);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * (1.0 + GAP) * k as f32;
        let kids: Vec<usize> = spec.descendants(i).collect();
        let bar_h = if kids.is_empty() { rh } else { rh * 0.46 };
        let mut bar = tag("roundRect", [0.0, y, w, bar_h], graded(spec, k, n), spec, i);
        bar.adj = vec![("adj", 10_000)];
        out.push(bar);
        let m = kids.len().max(1) as f32;
        let bw = w / m;
        for (j, &c) in kids.iter().enumerate() {
            let b = [bw * j as f32 + bw * 0.02, y + rh * 0.52, bw * 0.96, rh * 0.48];
            let mut s = tag("roundRect", b, graded_light(spec, k, n, 30_000, 18_000), spec, c);
            s.adj = vec![("adj", 10_000)];
            out.push(s);
        }
        if k + 1 < tops.len() {
            let gh = rh * GAP;
            let aw = (gh * 1.2).min(w * 0.1);
            out.push(arrow("downArrow", [w / 2.0 - aw / 2.0, y + rh + gh * 0.1, aw, gh * 0.8], tint(spec, k, 60_000), 50_000, 60_000));
        }
    }
    out
}

/// Grid places for blocks of 3:2 in lines (rows when `across`, else columns), each line turning
/// back from where the one before ended: the box of each place.
fn winding(n: usize, w: f32, h: f32, across: bool) -> Vec<[f32; 4]> {
    const GX: f32 = 0.4;
    const GY: f32 = 0.5;
    let n = n.max(1);
    let size = |per: usize| {
        let lines = n.div_ceil(per);
        let (cols, rows) = if across { (per, lines) } else { (lines, per) };
        (w / (cols as f32 + (cols - 1) as f32 * GX)).min(h / (rows as f32 + (rows - 1) as f32 * GY) * 1.5)
    };
    let per = (1..=n).max_by(|a, b| size(*a).total_cmp(&size(*b))).unwrap_or(1);
    let lines = n.div_ceil(per);
    let (cols, rows) = if across { (per, lines) } else { (lines, per) };
    let bw = size(per);
    let bh = bw / 1.5;
    let (gx, gy) = (bw * GX, bh * GY);
    let x0 = (w - (cols as f32 * bw + (cols - 1) as f32 * gx)) / 2.0;
    let y0 = (h - (rows as f32 * bh + (rows - 1) as f32 * gy)) / 2.0;
    (0..n)
        .map(|k| {
            let (line, mut at) = (k / per, k % per);
            if line % 2 == 1 {
                at = per - 1 - at;
            }
            let (c, r) = if across { (at, line) } else { (line, at) };
            [x0 + c as f32 * (bw + gx), y0 + r as f32 * (bh + gy), bw, bh]
        })
        .collect()
}

/// Blocks winding through rows (`across`) or columns, an arrow from each to the next.
fn winding_process(spec: &SmartArtSpec, w: f32, h: f32, across: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let boxes = winding(n, w, h, across);
    let mut out = Vec::new();
    for k in 1..tops.len() {
        let (Some(a), Some(b)) = (boxes.get(k - 1), boxes.get(k)) else { continue };
        let p = tint(spec, k - 1, 60_000);
        let (acx, acy, bcx, bcy) = (a[0] + a[2] / 2.0, a[1] + a[3] / 2.0, b[0] + b[2] / 2.0, b[1] + b[3] / 2.0);
        let s = if (acy - bcy).abs() < 0.5 {
            // Across a row gap.
            let (x0, x1) = if bcx > acx { (a[0] + a[2], b[0]) } else { (b[0] + b[2], a[0]) };
            let len = (x1 - x0) * 0.64;
            let th = (a[3] * 0.3).min(len * 1.2);
            arrow(if bcx > acx { "rightArrow" } else { "leftArrow" }, [x0 + (x1 - x0 - len) / 2.0, acy - th / 2.0, len, th], p, 50_000, 60_000)
        } else {
            let (y0, y1) = if bcy > acy { (a[1] + a[3], b[1]) } else { (b[1] + b[3], a[1]) };
            let len = (y1 - y0) * 0.64;
            let th = (a[2] * 0.2).min(len * 1.2);
            arrow(if bcy > acy { "downArrow" } else { "upArrow" }, [acx - th / 2.0, y0 + (y1 - y0 - len) / 2.0, th, len], p, 50_000, 60_000)
        };
        out.push(s);
    }
    for (k, &i) in tops.iter().enumerate() {
        let Some(&b) = boxes.get(k) else { continue };
        let mut s = card("roundRect", b, graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 12_000)];
        out.push(s);
    }
    out
}

pub(crate) fn process_snake(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    winding_process(spec, w, h, true)
}

pub(crate) fn process_bending_vertical(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    winding_process(spec, w, h, false)
}

/// Filled circles stepping down to the right, an elbow line from each to the next; the lines
/// under each top item beside its circle.
pub(crate) fn step_down(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = (h * 0.98 / n as f32).min(w * 0.16) * 0.9;
    let dy = if n > 1 { (h * 0.98 - d) / (n - 1) as f32 } else { 0.0 };
    let dx = d * 0.9;
    let at = |k: usize| (w * 0.01 + dx * k as f32, h * 0.01 + dy * k as f32);
    let mut out = Vec::new();
    for k in 1..tops.len() {
        let ((x0, y0), (x1, y1)) = (at(k - 1), at(k));
        let a = spec.colors.accent(k - 1);
        out.push(Sh::line(x0 + d / 2.0, y0 + d, x0 + d / 2.0, y1 + d / 2.0, a));
        out.push(Sh::line(x0 + d / 2.0, y1 + d / 2.0, x1, y1 + d / 2.0, a));
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = at(k);
        let b = [x, y, d, d];
        let mut c = tag("ellipse", b, graded(spec, k, n), spec, i);
        c.tb = inset(b, ROUND_INSET);
        out.push(c);
        let tx = x + d * 1.15;
        out.push(kids_label(spec, i, [tx, y, (w * 0.99 - tx).max(0.0), d], "l", "ctr"));
    }
    out
}

/// Arrows to the right edge stacked, each starting a column further right with its item's own
/// line; the lines under each item in its column below the arrows.
pub(crate) fn process_increasing_arrows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let area = h * 0.5;
    let ah = area / n as f32 * 0.9;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32;
        let y = area / n as f32 * k as f32;
        let b = [x, y, (w - x).max(0.0), ah];
        let mut a = card("rightArrow", b, graded(spec, k, n), spec, i);
        a.text = lines_of(spec, i, true, false);
        a.adj = vec![("adj1", 80_000), ("adj2", 50_000)];
        let head = ah.min(b[2]) * 0.5;
        a.tb = [x + cw * 0.05, y + ah * 0.12, (b[2] - head - cw * 0.08).max(0.0), ah * 0.76];
        a.algn = Some("l");
        out.push(a);
        let top = area + h * 0.04;
        out.push(kids_label(spec, i, [x + cw * 0.05, top, cw * 0.9, (h * 0.99 - top).max(0.0)], "l", "t"));
    }
    out
}

/// One wide arrow across the graphic, each top item's text in a section of it, light bars between.
pub(crate) fn process_continuous_arrow(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let ah = (h * 0.7).min(w * 0.4);
    let cy = h / 2.0;
    let head = ah * 0.5;
    let mut out = vec![arrow("rightArrow", [0.0, cy - ah / 2.0, w, ah], graded(spec, 0, 1), 72_000, 50_000)];
    let lane = (w - head * 0.6).max(0.0);
    let seg = lane / n as f32;
    let th = ah * 0.72;
    for (k, &i) in tops.iter().enumerate() {
        if k > 0 {
            let t = (w * 0.006).max(1.0);
            out.push(plain("rect", [seg * k as f32 - t / 2.0, cy - th / 2.0, t, th], Paint::Light));
        }
        out.push(label(spec, i, [seg * k as f32 + seg * 0.06, cy - th / 2.0 + th * 0.08, seg * 0.88, th * 0.84], "ctr", "ctr"));
    }
    out
}

/// Columns: each top item in a filled header with arrows between, the items under it as light
/// boxes below.
pub(crate) fn process_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bw = cw * if n > 1 { 0.76 } else { 0.9 };
    let head = h * 0.24;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + if n > 1 { 0.0 } else { (cw - bw) / 2.0 };
        let mut hd = tag("roundRect", [x, h * 0.01, bw, head], graded(spec, k, n), spec, i);
        hd.adj = vec![("adj", 12_000)];
        out.push(hd);
        if k + 1 < tops.len() {
            out.push(gap_arrow(x + bw, cw * (k + 1) as f32, h * 0.01 + head / 2.0, head * 0.4, tint(spec, k, 60_000)));
        }
        let list: Vec<usize> = spec.descendants(i).collect();
        let top = h * 0.04 + head;
        let bh = ((h * 0.99 - top) / list.len().max(1) as f32).min(head * 1.1);
        for (j, &c) in list.iter().enumerate() {
            let deeper = spec.items.get(c).map_or(0.0, |it| f32::from(it.level.saturating_sub(1).min(3)));
            let bx = x + bw * 0.08 * deeper;
            let mut s = tag(
                "roundRect",
                [bx, top + bh * j as f32 + bh * 0.06, (x + bw - bx).max(0.0), bh * 0.88],
                graded_light(spec, k, n, 26_000, 16_000),
                spec,
                c,
            );
            s.adj = vec![("adj", 12_000)];
            out.push(s);
        }
    }
    out
}

/// A light arrow falling from top left to bottom right, a dot per item along it and the item's
/// text below its dot.
pub(crate) fn arrow_downward(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let (run, fall) = (w * 0.9, h * 0.62);
    let deg = (fall / run.max(0.01)).atan().to_degrees().min(35.0);
    let len = (run / deg.to_radians().cos()).min(w * 0.96);
    let th = (h * 0.14).min(len * 0.12);
    let (cx, cy) = (w / 2.0, h * 0.42);
    let mut out = vec![arrow("rightArrow", [cx - len / 2.0, cy - th / 2.0, len, th], tint(spec, 0, 35_000), 60_000, 120_000).turned_by(deg)];
    let (dx, dy) = (deg.to_radians().cos(), deg.to_radians().sin());
    let d = (th * 0.75).max(1.0);
    let cw = w / n as f32;
    for (k, &i) in tops.iter().enumerate() {
        let t = -0.42 + 0.78 * (k as f32 + 0.5) / n as f32;
        let (x, y) = (cx + dx * len * t, cy + dy * len * t);
        let mut dot = plain("ellipse", [x - d / 2.0, y - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 1.5;
        out.push(dot);
        let lw = cw * 0.96;
        let top = y + d * 0.9;
        let bottom = (top + h * 0.36).min(h * 0.99);
        out.push(label(spec, i, [(x - lw / 2.0).clamp(0.0, (w - lw).max(0.0)), top.min(h), lw, (bottom - top).max(0.0)], "ctr", "t"));
    }
    out
}

/// Rows: a light ring on the left filled further each row (a wedge of `k + 1` of `n`, the last
/// whole), each top item's text beside it.
pub(crate) fn process_circles_increasing(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let d = (rh * 0.84).min(w * 0.2);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32 + (rh - d) / 2.0;
        let b = [w * 0.01, y, d, d];
        let mut bg = plain("ellipse", b, tint(spec, k, 20_000)).of_item(i);
        bg.line = Paint::Accent(spec.colors.accent(k), Some(50_000));
        bg.line_w = 1.0;
        out.push(bg);
        let p = graded(spec, k, n);
        if k + 1 >= n {
            out.push(plain("ellipse", b, p));
        } else {
            let mut wedge = plain("pie", b, p);
            wedge.adj = vec![("adj1", 16_200_000), ("adj2", super::matrix::wedge_edge(16_200_000, k + 1, n))];
            out.push(wedge);
        }
        let tx = w * 0.01 + d * 1.25;
        out.push(label(spec, i, [tx, rh * k as f32, (w * 0.99 - tx).max(0.0), rh], "l", "ctr"));
    }
    out
}
