//! List layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{
    Ink, Paint, ROUND_INSET, Sh, Tree, bullets, card, graded, graded_light, hex_adj, inset, kids_label, label, line_box, lines_of, own, plain, tag,
    tint, tops, with_text,
};

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

// ---- fourth batch ----

/// A zigzag of hexagons: each top item's own line in a filled hexagon on the upper row, the lines
/// under it in a light hexagon fitted beside it on the lower row.
pub(crate) fn list_hexagon_alternating(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = 2 * n;
    let hw = (w * 0.98 / (0.75 * (m - 1) as f32 + 1.0)).min(h * 0.98 / (1.5 * 0.87));
    let hh = hw * 0.87;
    let (x0, y0) = ((w - (0.75 * (m - 1) as f32 + 1.0) * hw) / 2.0, (h - 1.5 * hh) / 2.0);
    let gap = hw * 0.03;
    let cell = |c: usize| {
        let (x, y) = (x0 + 0.75 * hw * c as f32, y0 + if c % 2 == 1 { hh / 2.0 } else { 0.0 });
        [x + gap, y + gap, hw - 2.0 * gap, hh - 2.0 * gap]
    };
    let text_box = |b: [f32; 4]| [b[0] + b[2] * 0.2, b[1] + b[3] * 0.12, b[2] * 0.6, b[3] * 0.76];
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let b = cell(2 * k);
        let mut s = tag("hexagon", b, graded(spec, k, n), spec, i);
        s.adj = vec![("adj", hex_adj(b))];
        s.tb = text_box(b);
        out.push(s);
        let d = cell(2 * k + 1);
        let mut t = plain("hexagon", d, graded_light(spec, k, n, 26_000, 16_000)).of_item(i);
        t.adj = vec![("adj", hex_adj(d))];
        t.text = lines_of(spec, i, false, true);
        t.tb = text_box(d);
        out.push(t);
    }
    out
}

/// Rows: a light box with a coloured outline holding the lines under each top item, its own line
/// on a filled tab over the box's top-left corner.
pub(crate) fn list_vertical_box(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        let a = spec.colors.accent(k);
        let tab_h = rh * 0.36;
        let bx = [w * 0.03, y + tab_h * 0.5, w * 0.96, rh * 0.92 - tab_h * 0.5];
        let mut b = plain("rect", bx, Paint::Light).of_item(i);
        b.line = Paint::Accent(a, None);
        b.line_w = 1.0;
        b.text = lines_of(spec, i, false, true);
        let top = y + tab_h * 1.05;
        b.tb = [bx[0] + bx[2] * 0.04, top, bx[2] * 0.92, (bx[1] + bx[3] - top).max(0.0)];
        b.algn = Some("l");
        out.push(b);
        let tb = [0.0, y, w * 0.42, tab_h];
        let mut t = tag("rect", tb, graded(spec, k, n), spec, i);
        t.tb = [tb[0] + tb[2] * 0.06, y, tb[2] * 0.88, tab_h];
        t.algn = Some("l");
        out.push(t);
    }
    out
}

/// Columns: each top item's own line in a filled header, the lines under it as bullets in a light
/// box below.
pub(crate) fn list_horizontal_bullet(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let head = h * 0.26;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + cw * 0.03;
        let body = [x, h * 0.01 + head, cw * 0.94, h * 0.98 - head];
        let mut s = plain("rect", body, graded_light(spec, k, n, 22_000, 12_000)).of_item(i);
        s.text = bullets(spec, i);
        s.tb = [body[0] + body[2] * 0.08, body[1] + body[3] * 0.06, body[2] * 0.84, body[3] * 0.88];
        s.algn = Some("l");
        s.anchor = "t";
        out.push(s);
        let hb = [x, h * 0.01, cw * 0.94, head];
        out.push(tag("rect", hb, graded(spec, k, n), spec, i));
    }
    out
}

/// Columns: a small filled square and each top item's own line over a thick rule in its colour,
/// the lines under it below, each after a small square.
pub(crate) fn list_square_accent(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let th = h * 0.2;
    let s = (th * 0.45).min(cw * 0.14);
    let rule_y = th + h * 0.02;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + cw * 0.04;
        let colw = cw * 0.92;
        let a = spec.colors.accent(k);
        out.push(line_box([x + s * 1.5, 0.0, (colw - s * 1.5).max(0.0), th], own(spec, i), false, "l", i));
        out.push(plain("rect", [x, (th - s) / 2.0, s, s], graded(spec, k, n)).of_item(i));
        let mut rule = Sh::line(x, rule_y, x + colw, rule_y, a);
        rule.line = Paint::Accent(a, None);
        rule.line_w = (h * 0.012).clamp(1.0, 4.0);
        out.push(rule);
        let kids: Vec<usize> = spec.descendants(i).collect();
        let top = rule_y + h * 0.04;
        let rh = ((h - top) / kids.len().max(1) as f32).min(h * 0.18);
        let q = s * 0.5;
        for (j, &c) in kids.iter().enumerate() {
            let y = top + rh * j as f32;
            out.push(plain("rect", [x, y + (rh - q) / 2.0, q, q], tint(spec, k, 60_000)));
            out.push(line_box([x + q * 2.0, y, (colw - q * 2.0).max(0.0), rh], own(spec, c), true, "l", i));
        }
    }
    out
}

/// Columns: a tall light card with the lines under each top item, a filled circle with its own
/// line over the card's top.
pub(crate) fn list_stacked(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.56).min(h * 0.38);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32;
        let cb = [x + cw * 0.06, h * 0.01 + d * 0.5, cw * 0.88, (h * 0.98 - d * 0.5).max(0.0)];
        let mut c = plain("roundRect", cb, graded_light(spec, k, n, 24_000, 14_000)).of_item(i);
        c.adj = vec![("adj", 8_000)];
        c.text = lines_of(spec, i, false, true);
        let top = h * 0.01 + d * 1.08;
        c.tb = [cb[0] + cb[2] * 0.08, top, cb[2] * 0.84, (cb[1] + cb[3] - top - cb[3] * 0.04).max(0.0)];
        c.anchor = "t";
        out.push(c);
        let b = [x + (cw - d) / 2.0, h * 0.01, d, d];
        let mut s = tag("ellipse", b, graded(spec, k, n), spec, i);
        s.tb = inset(b, ROUND_INSET);
        s.line = Paint::Light;
        s.line_w = 2.0;
        out.push(s);
    }
    out
}

/// Rows: each top item's own line in a filled block on the left, the lines under it in a light
/// block beside it (`point`: the left block is an arrow pointing into the box, which is outlined).
fn rows_with_details(spec: &SmartArtSpec, w: f32, h: f32, point: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let lw = w * if point { 0.36 } else { 0.3 };
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32 + rh * 0.06;
        let bh = rh * 0.88;
        let lb = [0.0, y, lw, bh];
        let mut l = tag(if point { "homePlate" } else { "rect" }, lb, graded(spec, k, n), spec, i);
        if point {
            let tip = (bh * 0.5).min(lw * 0.4);
            l.adj = vec![("adj", (tip / lw.min(bh).max(0.01) * 100_000.0).round().clamp(0.0, 100_000.0) as i64)];
            l.tb = [lw * 0.06, y + bh * 0.08, (lw - tip - lw * 0.08).max(0.0), bh * 0.84];
        }
        out.push(l);
        let x = lw + w * if point { 0.0 } else { 0.01 };
        let rb = [x, y, (w - x).max(0.0), bh];
        let mut r = plain("rect", rb, if point { Paint::Light } else { graded_light(spec, k, n, 22_000, 12_000) }).of_item(i);
        if point {
            r.line = Paint::Accent(spec.colors.accent(k), None);
            r.line_w = 1.0;
        }
        r.text = lines_of(spec, i, false, true);
        r.tb = [rb[0] + rb[2] * 0.05, rb[1], rb[2] * 0.9, rb[3]];
        r.algn = Some("l");
        out.push(r);
    }
    out
}

pub(crate) fn list_vertical_block(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    rows_with_details(spec, w, h, false)
}

pub(crate) fn list_vertical_arrow(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    rows_with_details(spec, w, h, true)
}

/// Squares nested in the lower left corner, the first outermost; each top item's text in a row to
/// the right, under a line that runs from the top of its square.
pub(crate) fn list_target(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let big = (h * 0.98).min(w * 0.38);
    let step = big / n as f32;
    let side = |k: usize| big - step * k as f32;
    let one = spec.colors != wordcraft_doc::smart_art::SmartArtColors::Colorful;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let s = side(k);
        let keep = if n > 1 { 22_000 + 50_000 * k as u32 / (n - 1) as u32 } else { 50_000 };
        let p = Paint::Accent(spec.colors.accent(if one { 0 } else { k }), Some(keep));
        let band = [w * 0.01, h - s, s, if k + 1 == tops.len() { s } else { step }];
        let mut q = plain("rect", [w * 0.01, h - s, s, s], p).of_item(i).picked_by(band);
        q.line = Paint::Light;
        q.line_w = 1.0;
        out.push(q);
    }
    let tx = w * 0.01 + big + w * 0.03;
    for (k, &i) in tops.iter().enumerate() {
        let y = h - side(k);
        let a = spec.colors.accent(k);
        let mut l = Sh::line(w * 0.01 + side(k), y, w * 0.99, y, a);
        l.line = Paint::Accent(a, None);
        l.line_w = 1.0;
        out.push(l);
        out.push(label(spec, i, [tx, y, (w * 0.99 - tx).max(0.0), step.min(h - y)], "l", "ctr"));
    }
    out
}

/// A line down the left with a filled circle per top item; the item's text beside its circle,
/// a thin rule under each row.
pub(crate) fn list_vertical_circle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let d = (rh * 0.5).min(w * 0.1);
    let lx = w * 0.02 + d / 2.0;
    let a = spec.colors.accent(0);
    let mut out = Vec::new();
    if n > 1 {
        out.push(Sh::line(lx, rh / 2.0, lx, rh * (n as f32 - 0.5), a));
    }
    let tx = lx + d;
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        let mut c = plain("ellipse", [lx - d / 2.0, y + (rh - d) / 2.0, d, d], graded(spec, k, n)).of_item(i);
        c.line = Paint::Light;
        c.line_w = 1.5;
        out.push(c);
        out.push(label(spec, i, [tx, y, (w - tx).max(0.0), rh * 0.96], "l", "ctr"));
        if k + 1 < tops.len() {
            let mut r = Sh::line(tx, y + rh, w * 0.99, y + rh, spec.colors.accent(k));
            r.line_w = 0.75;
            out.push(r);
        }
    }
    out
}

/// Centred bars stacked, each as wide as its text needs (by its length against the longest).
pub(crate) fn list_varying_width(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let len = |i: usize| lines_of(spec, i, true, true).iter().map(|(t, _)| t.chars().count()).max().unwrap_or(0).max(1) as f32;
    let most = tops.iter().map(|&i| len(i)).fold(1.0f32, f32::max);
    let rh = h / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let bw = w * (0.35 + 0.63 * len(i) / most);
        let b = [(w - bw) / 2.0, rh * k as f32 + rh * 0.07, bw, rh * 0.86];
        let mut s = card("roundRect", b, graded(spec, k, n), spec, i);
        s.adj = vec![("adj", 30_000)];
        s.tb = [b[0] + b[2] * 0.06, b[1] + b[3] * 0.06, b[2] * 0.88, b[3] * 0.88];
        out.push(s);
    }
    out
}

/// Rows: each top item's own line on the left in its colour, the lines under it on the right, a
/// rule under each row with a small filled tab at its left end.
pub(crate) fn list_tab(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        let a = spec.colors.accent(k);
        let tab = (rh * 0.1).max(1.0);
        let tb = [w * 0.01, y, w * 0.3, (rh - tab * 1.6).max(0.0)];
        let mut t = with_text(plain("rect", tb, Paint::None), own(spec, i), "l", Ink::Accent(a), 1.1).of_item(i).text_box(tb);
        t.anchor = "ctr";
        out.push(t);
        let lb = [w * 0.34, y, w * 0.65, (rh - tab * 1.6).max(0.0)];
        out.push(kids_label(spec, i, lb, "l", "ctr"));
        let ry = y + rh - tab / 2.0 - h * 0.005;
        let mut rule = Sh::line(w * 0.01, ry, w * 0.99, ry, a);
        rule.line_w = 1.0;
        out.push(rule);
        out.push(plain("rect", [w * 0.01, ry - tab / 2.0, w * 0.18, tab], graded(spec, k, n)));
    }
    out
}

/// A curve down the left with a filled circle on it per top item and a light bar beside each
/// circle holding the item's text.
pub(crate) fn list_curved(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let d = (h / n as f32 * 0.8).min(w * 0.12);
    let rx = (w * 0.16).max(d);
    let ry = ((h - d) / 2.0).max(1.0);
    let cy = h / 2.0;
    let y_at = |k: usize| d / 2.0 + (h - d) * (k as f32 + 0.5) / n as f32;
    let theta = |k: usize| ((y_at(k) - cy) / ry).clamp(-1.0, 1.0).asin().to_degrees();
    let x_at = |k: usize| rx + rx * theta(k).to_radians().cos();
    let band = (d * 0.12).max(1.0);
    let mut out = Vec::new();
    let (t0, t1) = if n > 1 { (theta(0) - 10.0, theta(n - 1) + 10.0) } else { (-40.0, 40.0) };
    let mut arc = plain("blockArc", [0.0, cy - ry, 2.0 * rx, 2.0 * ry], tint(spec, 0, 60_000));
    let ss = (2.0 * rx).min(2.0 * ry).max(0.01);
    arc.adj =
        vec![("adj1", super::cycle::ang(t0)), ("adj2", super::cycle::ang(t1)), ("adj3", (band / ss * 100_000.0).round().clamp(0.0, 50_000.0) as i64)];
    out.push(arc);
    let bh = d * 0.74;
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = (x_at(k), y_at(k));
        let b = [x, y - bh / 2.0, (w * 0.99 - x).max(0.0), bh];
        let mut s = card("rect", b, graded_light(spec, k, n, 26_000, 16_000), spec, i);
        s.tb = [x + d * 0.7, b[1], (b[2] - d * 0.8).max(0.0), bh];
        s.algn = Some("l");
        out.push(s);
        let mut c = plain("ellipse", [x - d / 2.0, y - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        c.line = Paint::Light;
        c.line_w = 2.0;
        out.push(c);
    }
    out
}
