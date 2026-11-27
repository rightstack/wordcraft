//! List layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Paint, Sh, Tree, card, graded, graded_light, inset, lines_of, plain, tag, tint, tops};

/// Hexagons fitted together like a honeycomb: in rows, every other column half a hexagon lower.
pub(crate) fn list_hexagon(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    // A hexagon `hw` wide is 0.87 of that high; columns step three quarters of a width.
    let fit = |c: usize| {
        let rows = n.div_ceil(c) as f32;
        let across = 0.75 * (c - 1) as f32 + 1.0;
        let down = rows + if c > 1 { 0.5 } else { 0.0 };
        (w * 0.98 / across).min(h * 0.98 / (down * 0.87))
    };
    // One zigzag row when it is nearly as large: no hexagon left alone on a second row.
    let score = |c: usize| fit(c) * if c == n { 1.1 } else { 1.0 };
    let cols = (1..=n).max_by(|a, b| score(*a).total_cmp(&score(*b))).unwrap_or(1);
    let hw = fit(cols);
    let hh = hw * 0.87;
    let rows = n.div_ceil(cols);
    let across = (0.75 * (cols - 1) as f32 + 1.0) * hw;
    let down = (rows as f32 + if cols > 1 { 0.5 } else { 0.0 }) * hh;
    let (x0, y0) = ((w - across) / 2.0, (h - down) / 2.0);
    let gap = hw * 0.03;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let x = x0 + 0.75 * hw * c as f32;
        let y = y0 + hh * r as f32 + if c % 2 == 1 { hh / 2.0 } else { 0.0 };
        let b = [x + gap, y + gap, hw - 2.0 * gap, hh - 2.0 * gap];
        let mut s = card("hexagon", b, graded(spec, k, n), spec, i);
        let ss = b[2].min(b[3]).max(0.01);
        s.adj = vec![("adj", (b[2] * 0.25 / ss * 100_000.0).round().clamp(0.0, 100_000.0) as i64)];
        s.tb = [b[0] + b[2] * 0.2, b[1] + b[3] * 0.12, b[2] * 0.6, b[3] * 0.76];
        out.push(s);
    }
    out
}

/// A bar in the item's colour per top item, stacked, the lines under it in a light band below.
pub(crate) fn list_vertical_bullet(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let size = |i: usize| 1.0 + 0.8 * spec.descendants(i).len() as f32;
    let total: f32 = tops.iter().map(|&i| size(i)).sum::<f32>().max(1.0);
    let unit = h / (total + 0.15 * (n - 1) as f32);
    let mut y = 0.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let bar = [0.0, y, w, unit * 0.9];
        let p = graded(spec, k, n);
        let mut s = tag("roundRect", bar, p, spec, i);
        s.adj = vec![("adj", 14_000)];
        s.tb = [w * 0.03, y, w * 0.94, unit * 0.9];
        s.algn = Some("l");
        out.push(s);
        let body = lines_of(spec, i, false, true);
        let bh = unit * 0.8 * body.len() as f32;
        if !body.is_empty() {
            let b = [w * 0.02, y + unit * 0.9, w * 0.96, bh];
            let mut t = plain("rect", b, graded_light(spec, k, n, 14_000, 8_000)).of_item(i);
            t.text = body;
            t.tb = [w * 0.05, b[1], w * 0.9, bh];
            t.algn = Some("l");
            out.push(t);
        }
        y += size(i) * unit + unit * 0.15 - unit * 0.1;
    }
    out
}

/// A tinted column per top item headed by its text, the items under it as boxes inside.
pub(crate) fn list_grouped(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let c = tree.roots.len().max(1);
    let cw = w / c as f32;
    let colw = cw * 0.92;
    let (y0, ch) = (h * 0.01, h * 0.98);
    let head = ch * 0.2;
    let mut out = Vec::new();
    for (k, &r) in tree.roots.iter().enumerate() {
        let x = cw * k as f32 + (cw - colw) / 2.0;
        let list: Vec<usize> = spec.descendants(r).collect();
        let col = [x, y0, colw, ch];
        let hb = if list.is_empty() { inset(col, 0.06) } else { [x + colw * 0.06, y0, colw * 0.88, head] };
        let mut g = plain("roundRect", col, graded_light(spec, k, c, 24_000, 14_000)).of_item(r).picked_by(hb);
        g.adj = vec![("adj", 6_000)];
        g.text = lines_of(spec, r, true, false);
        g.tb = hb;
        out.push(g);
        let m = list.len().max(1) as f32;
        let room = ch - head - ch * 0.04;
        let bh = room / m;
        for (j, &i) in list.iter().enumerate() {
            let deeper = spec.items.get(i).map_or(0.0, |it| f32::from(it.level.saturating_sub(1).min(3)));
            let bx = x + colw * (0.08 + 0.06 * deeper);
            let b = [bx, y0 + head + bh * j as f32 + bh * 0.08, (x + colw * 0.92 - bx).max(0.0), bh * 0.84];
            let mut s = tag("roundRect", b, Paint::Light, spec, i);
            s.adj = vec![("adj", 12_000)];
            s.line = Paint::Accent(spec.colors.accent(k), None);
            s.line_w = 1.0;
            out.push(s);
        }
    }
    out
}

/// Columns of trapezoids, the wide side down, holding each item's text.
pub(crate) fn list_trapezoid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let tw = cw * 0.96;
    let th = h * 0.96;
    let inset_x = tw * 0.12;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let b = [cw * k as f32 + (cw - tw) / 2.0, h * 0.02, tw, th];
        let mut s = card("trapezoid", b, graded_light(spec, k, n, 100_000, 55_000), spec, i);
        let ss = tw.min(th).max(0.01);
        s.adj = vec![("adj", (inset_x / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64)];
        s.tb = [b[0] + inset_x + tw * 0.04, b[1] + th * 0.08, (tw - 2.0 * inset_x - tw * 0.08).max(0.0), th * 0.84];
        s.anchor = "t";
        out.push(s);
    }
    out
}

/// Columns that get shorter from left to right (standing on one line), each item's text at the top.
pub(crate) fn list_descending(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let f = if n > 1 { 1.0 - 0.5 * k as f32 / (n - 1) as f32 } else { 1.0 };
        let bh = h * 0.98 * f;
        let b = [cw * k as f32 + cw * 0.04, h * 0.99 - bh, cw * 0.92, bh];
        let mut s = card("rect", b, graded(spec, k, n), spec, i);
        s.tb = [b[0] + b[2] * 0.08, b[1] + b[2] * 0.08, b[2] * 0.84, (bh - b[2] * 0.16).max(0.0)];
        s.anchor = "t";
        out.push(s);
    }
    out
}

/// The first top item as a header across the top, the others as columns under it, on a thin bar.
pub(crate) fn list_table(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let Some((&first, rest)) = tops.split_first() else { return Vec::new() };
    let m = rest.len();
    let head = if m == 0 { h * 0.9 } else { h * 0.3 };
    let mut out = Vec::new();
    let hb = [0.0, 0.0, w, head];
    let mut header = card("rect", hb, graded(spec, 0, 1), spec, first);
    header.tb = inset(hb, 0.06);
    out.push(header);
    let gap = h * 0.03;
    let base = h * 0.04;
    if m > 0 {
        let cw = w / m as f32;
        let top = head + gap;
        for (k, &i) in rest.iter().enumerate() {
            let b = [cw * k as f32 + cw * 0.02, top, cw * 0.96, (h - base - gap - top).max(0.0)];
            let mut s = card("rect", b, graded_light(spec, k + 1, m + 1, 30_000, 16_000), spec, i);
            s.tb = [b[0] + b[2] * 0.08, b[1] + b[3] * 0.06, b[2] * 0.84, b[3] * 0.88];
            s.anchor = "t";
            out.push(s);
        }
    }
    out.push(plain("rect", [0.0, h - base, w, base], tint(spec, 0, 70_000)));
    out
}
