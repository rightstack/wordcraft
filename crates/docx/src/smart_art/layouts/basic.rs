//! The first eight layouts: list, process, cycle, hierarchy, pyramid, radial, matrix, Venn.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Ink, Paint, ROUND_INSET, Sh, angle, inset, polar, tops};

/// Blocks of 3:2 in rows, as large as fit; the last row centred.
pub(crate) fn basic_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let size = |c: usize| {
        let r = n.div_ceil(c);
        let bw = (w / c as f32 * 0.9).min(h / r as f32 * 0.9 * 1.5);
        (bw, bw / 1.5)
    };
    let cols = (1..=n).max_by(|a, b| (size(*a).0).total_cmp(&size(*b).0)).unwrap_or(1);
    let rows = n.div_ceil(cols);
    let (bw, bh) = size(cols);
    let (gx, gy) = (bw * 0.1, bh * 0.1);
    let y0 = (h - (rows as f32 * bh + (rows - 1) as f32 * gy)) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let in_row = if r + 1 == rows { n - r * cols } else { cols };
        let x0 = (w - (in_row as f32 * bw + (in_row - 1) as f32 * gx)) / 2.0;
        let b = [x0 + c as f32 * (bw + gx), y0 + r as f32 * (bh + gy), bw, bh];
        out.push(Sh::node("rect", b, spec.colors.accent(k)).with_item(spec, i, true));
    }
    out
}

/// Steps left to right with an arrow in each gap.
pub(crate) fn process(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    const GAP: f32 = 0.45;
    let nw = w / (n + (n - 1.0) * GAP);
    let nh = (nw * 0.6).min(h * 0.9);
    let y = (h - nh) / 2.0;
    let gap = nw * GAP;
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate().skip(1) {
        let gx = k as f32 * (nw + gap) - gap;
        let (aw, ah) = (gap * 0.62, (nh * 0.35).min(gap * 0.7));
        let mut a = Sh::node("rightArrow", [gx + (gap - aw) / 2.0, h / 2.0 - ah / 2.0, aw, ah], spec.colors.accent(k - 1));
        a.fill = Paint::Accent(spec.colors.accent(k - 1), Some(60_000));
        a.line = Paint::None;
        out.push(a);
    }
    for (k, &i) in tops.iter().enumerate() {
        let b = [k as f32 * (nw + gap), y, nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_item(spec, i, true));
    }
    out
}

/// Circles round a circle, clockwise from the top, with a curved-path arrow between each.
pub(crate) fn cycle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let step = (std::f32::consts::PI / n as f32).sin();
    let mut d = m * 0.32;
    for _ in 0..4 {
        let r = (m - d) / 2.0;
        d = if n == 1 { m * 0.5 } else { (m * 0.32).min(2.0 * r * step * 0.62) };
    }
    let r = (m - d) / 2.0;
    let mut out = Vec::new();
    if n > 1 {
        let chord = 2.0 * r * step;
        let len = (chord - d) * 0.6;
        if len > 2.0 {
            let thick = (len * 0.6).min(d * 0.25);
            for k in 0..n {
                let mid = angle(k, n) + 180.0 / n as f32;
                let (x, y) = polar(cx, cy, r, mid);
                let mut a = Sh::node("rightArrow", [x - len / 2.0, y - thick / 2.0, len, thick], spec.colors.accent(k));
                a.rot = (mid + 90.0).rem_euclid(360.0);
                a.fill = Paint::Accent(spec.colors.accent(k), Some(60_000));
                a.line = Paint::None;
                out.push(a);
            }
        }
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 { (cx, cy) } else { polar(cx, cy, r, angle(k, n)) };
        let b = [x - d / 2.0, y - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k)).with_item(spec, i, true).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// A tree, a row per level: each item centred over the items under it, joined by elbow lines.
pub(crate) fn hierarchy(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n = spec.items.len();
    let parents = spec.parents();
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (i, p) in parents.iter().enumerate() {
        match p.and_then(|p| kids.get_mut(p)) {
            Some(k) => k.push(i),
            None => roots.push(i),
        }
    }
    // Leaves under each item (children come after their parent, so from the end).
    let mut leaves = vec![1usize; n];
    for i in (0..n).rev() {
        let sum: usize = kids.get(i).map_or(0, |k| k.iter().map(|&c| leaves.get(c).copied().unwrap_or(1)).sum());
        if sum > 0
            && let Some(l) = leaves.get_mut(i)
        {
            *l = sum;
        }
    }
    let depth = spec.items.iter().map(|it| usize::from(it.level)).max().unwrap_or(0) + 1;
    let total: usize = roots.iter().map(|&r| leaves.get(r).copied().unwrap_or(1)).sum::<usize>().max(1);
    let unit = w / total as f32;
    let rh = h / depth as f32;
    let nw = (unit * 0.88).min(rh * 0.62 * 1.8);
    let nh = (rh * 0.62).min(nw * 0.8);
    let mut start = vec![0.0f32; n];
    let mut at = 0.0;
    for &r in &roots {
        if let Some(s) = start.get_mut(r) {
            *s = at;
        }
        at += leaves.get(r).copied().unwrap_or(1) as f32 * unit;
    }
    for i in 0..n {
        let mut at = start.get(i).copied().unwrap_or(0.0);
        for &k in kids.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(s) = start.get_mut(k) {
                *s = at;
            }
            at += leaves.get(k).copied().unwrap_or(1) as f32 * unit;
        }
    }
    let cx = |i: usize| start.get(i).copied().unwrap_or(0.0) + leaves.get(i).copied().unwrap_or(1) as f32 * unit / 2.0;
    let top = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * rh + (rh - nh) / 2.0);
    let mut out = Vec::new();
    for i in 0..n {
        let Some(ks) = kids.get(i).filter(|k| !k.is_empty()) else { continue };
        let accent = spec.colors.accent(i);
        let (px, pb) = (cx(i), top(i) + nh);
        let ct = ks.first().map_or(pb, |&k| top(k));
        let mid = (pb + ct) / 2.0;
        out.push(Sh::line(px, pb, px, mid, accent));
        let xs: Vec<f32> = ks.iter().map(|&k| cx(k)).collect();
        let (lo, hi) = xs.iter().fold((px, px), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        if hi - lo > 0.5 {
            out.push(Sh::line(lo, mid, hi, mid, accent));
        }
        for x in xs {
            out.push(Sh::line(x, mid, x, ct, accent));
        }
    }
    for i in 0..n {
        let b = [cx(i) - nw / 2.0, top(i), nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(i)).with_item(spec, i, false));
    }
    out
}

/// Bands stacked into a triangle: the first item at the narrow top.
pub(crate) fn pyramid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let tw = w.min(h * 1.3);
    let x0 = (w - tw) / 2.0;
    let bh = h / n;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (top_w, bot_w) = (tw * k as f32 / n, tw * (k as f32 + 1.0) / n);
        let b = [x0 + (tw - bot_w) / 2.0, k as f32 * bh, bot_w, bh];
        let accent = spec.colors.accent(k);
        let sh = if k == 0 {
            let tb = [b[0] + bot_w * 0.22, b[1] + bh * 0.4, bot_w * 0.56, bh * 0.56];
            Sh::node("triangle", b, accent).text_box(tb)
        } else {
            let inset_x = (bot_w - top_w) / 2.0;
            let ss = bot_w.min(bh).max(0.01);
            let mut s = Sh::node("trapezoid", b, accent);
            s.adj = vec![("adj", (inset_x / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64)];
            let mid_w = (top_w + bot_w) / 2.0 * 0.8;
            s.text_box([w / 2.0 - mid_w / 2.0, b[1] + bh * 0.08, mid_w, bh * 0.84])
        };
        out.push(sh.with_item(spec, i, true));
    }
    out
}

/// The first item in the middle; the items under it (and any later top-level items) around it,
/// each joined to the middle by a line.
pub(crate) fn radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n_items = spec.items.len();
    let parents = spec.parents();
    let spokes: Vec<usize> = (1..n_items).filter(|&i| matches!(parents.get(i), Some(Some(0)) | Some(None))).collect();
    let n = spokes.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut out = Vec::new();
    if n == 0 {
        let d = m * 0.5;
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(0)).with_item(spec, 0, false).text_box(inset(b, ROUND_INSET)));
        return out;
    }
    let step = if n == 1 { 1.0 } else { (std::f32::consts::PI / n as f32).sin() };
    let mut ds = m * 0.24;
    for _ in 0..4 {
        let r = (m - ds) / 2.0;
        ds = (m * 0.24).min(2.0 * r * step * 0.85);
    }
    let r = (m - ds) / 2.0;
    let dc = (m * 0.36).min(2.0 * (r - ds / 2.0 - m * 0.03)).max(m * 0.05);
    let at = |k: usize| polar(cx, cy, r, angle(k, n));
    for k in 0..n {
        let (x, y) = at(k);
        out.push(Sh::line(cx, cy, x, y, spec.colors.accent(0)));
    }
    let bc = [cx - dc / 2.0, cy - dc / 2.0, dc, dc];
    out.push(Sh::node("ellipse", bc, spec.colors.accent(0)).with_item(spec, 0, false).text_box(inset(bc, ROUND_INSET)));
    for (k, &i) in spokes.iter().enumerate() {
        let (x, y) = at(k);
        let b = [x - ds / 2.0, y - ds / 2.0, ds, ds];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k + 1)).with_item(spec, i, true).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// Quadrants of a square (more items: a larger grid).
pub(crate) fn matrix(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cols = (1..=n).find(|c| c * c >= n).unwrap_or(n).max(1);
    let rows = n.div_ceil(cols);
    let cell = (w / cols as f32).min(h / rows as f32);
    let gap = cell * 0.04;
    let (x0, y0) = ((w - cols as f32 * cell) / 2.0, (h - rows as f32 * cell) / 2.0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let b = [x0 + c as f32 * cell + gap / 2.0, y0 + r as f32 * cell + gap / 2.0, cell - gap, cell - gap];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_item(spec, i, true).text_box(inset(b, 0.1)));
    }
    out
}

/// Overlapping circles round the middle; each one's text on its outer side.
pub(crate) fn venn(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Ring radius against circle radius: neighbours' centres 1.5 radii apart.
    let k = if n == 1 { 0.0 } else { (0.75 / (std::f32::consts::PI / n as f32).sin()).max(0.6) };
    let r = m / (2.0 * (k + 1.0));
    let start = if n == 2 { 180.0 } else { -90.0 };
    let place = |j: usize| -> (f32, f32, f32) {
        let deg = start + 360.0 * j as f32 / n as f32;
        let (x, y) = polar(cx, cy, r * k, deg);
        (x, y, deg)
    };
    let mut out = Vec::new();
    for (j, &i) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j)).of_item(i);
        s.fill = Paint::Accent(spec.colors.accent(j), Some(40_000));
        s.alpha = Some(60_000);
        s.line = Paint::None;
        out.push(s);
    }
    // Every outline over every fill, so each circle shows whole.
    for (j, _) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j));
        s.fill = Paint::None;
        s.line = Paint::Accent(spec.colors.accent(j), None);
        s.line_w = 1.5;
        out.push(s);
    }
    for (j, &i) in tops.iter().enumerate() {
        let (x, y, deg) = place(j);
        let (tx, ty) = if n == 1 { (x, y) } else { polar(x, y, r * 0.35, deg) };
        let (bw, bh) = (r * 1.1, r * 0.8);
        let b = [tx - bw / 2.0, ty - bh / 2.0, bw, bh];
        let mut s = Sh::node("rect", b, spec.colors.accent(j)).with_item(spec, i, true).text_box(b);
        s.fill = Paint::None;
        s.line = Paint::None;
        s.ink = Ink::Dark;
        out.push(s);
    }
    out
}
