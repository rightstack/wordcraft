//! Matrix layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Paint, Sh, card, graded, graded_light, grid_cell, inset, lines_of, plain, polar, tag, tops};

/// Where wedge `k` of `n` starts (a preset angle, 60000ths of a degree clockwise from three
/// o'clock) when the first starts at `first`: whole numbers, so each wedge ends exactly where the
/// next begins.
pub(crate) fn wedge_edge(first: i64, k: usize, n: usize) -> i64 {
    let (k, n) = (i64::try_from(k).unwrap_or(0), i64::try_from(n.max(1)).unwrap_or(1));
    (first + 21_600_000 * k / n).rem_euclid(21_600_000)
}

/// The items a centre item has round it: those under item 0, and any later top items.
pub(crate) fn spokes(spec: &SmartArtSpec) -> Vec<usize> {
    let parents = spec.parents();
    (1..spec.items.len()).filter(|&i| matches!(parents.get(i), Some(Some(0) | None))).collect()
}

/// The first item in a light circle over the middle of quadrants that hold the items under it
/// (more than four: a grid of them under the first item in a band).
pub(crate) fn matrix_titled(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let around = spokes(spec);
    let k = around.len();
    let mut out = Vec::new();
    let title = |b: [f32; 4], prst: &'static str| {
        let mut t = card(prst, b, Paint::Light, spec, 0);
        t.text = lines_of(spec, 0, true, false);
        t.line = Paint::Accent(spec.colors.accent(0), None);
        t.line_w = 2.0;
        t.tb = inset(b, if prst == "ellipse" { 0.15 } else { 0.06 });
        t
    };
    if k == 0 {
        let d = w.min(h) * 0.7;
        out.push(title([(w - d) / 2.0, (h - d) / 2.0, d, d], "ellipse"));
        return out;
    }
    if k <= 4 {
        let side = w.min(h) * 0.98;
        let (x0, y0) = ((w - side) / 2.0, (h - side) / 2.0);
        let cell = side / 2.0;
        let gap = side * 0.02;
        let d = side * 0.36;
        for (j, &i) in around.iter().enumerate() {
            let (r, c) = (j / 2, j % 2);
            let b = [x0 + c as f32 * cell + gap / 2.0, y0 + r as f32 * cell + gap / 2.0, cell - gap, cell - gap];
            // The text in the quadrant's outer part, clear of the circle in the middle.
            let m = d / 2.0 + side * 0.02;
            // The quadrant's whole width, above (or below) the circle.
            let th = (b[3] * 0.94 - m + gap / 2.0).max(0.0);
            let tb = [b[0] + b[2] * 0.08, if r == 0 { b[1] + b[3] * 0.06 } else { b[1] + b[3] * 0.94 - th }, b[2] * 0.84, th];
            let mut q = card("roundRect", b, graded(spec, j, k.max(4)), spec, i);
            q.adj = vec![("adj", 8_000)];
            q.tb = tb;
            q.algn = Some("ctr");
            q.anchor = if r == 0 { "t" } else { "b" };
            out.push(q);
        }
        out.push(title([w / 2.0 - d / 2.0, h / 2.0 - d / 2.0, d, d], "ellipse"));
        return out;
    }
    let band = h * 0.22;
    out.push(title([w * 0.1, h * 0.01, w * 0.8, band * 0.9], "roundRect"));
    let cols = (1..=k).find(|c| c * c >= k).unwrap_or(k);
    for (j, &i) in around.iter().enumerate() {
        let cell = grid_cell(j, k, cols, [0.0, band, w, h - band]);
        let b = inset(cell, 0.03);
        let mut q = card("roundRect", b, graded(spec, j, k), spec, i);
        q.adj = vec![("adj", 8_000)];
        out.push(q);
    }
    out
}

/// Tinted quadrants on two axes (an arrow up the left and along the bottom), lines between them.
pub(crate) fn matrix_grid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cols = (1..=n).find(|c| c * c >= n).unwrap_or(n).max(1);
    let (x0, y0, side_w, side_h) = (w * 0.01, h * 0.01, w * 0.98, h * 0.98);
    let axis = side_w.min(side_h) * 0.06;
    let area = [x0 + axis, y0, side_w - axis, side_h - axis];
    let a = spec.colors.accent(0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cell = grid_cell(k, n, cols, area);
        let b = inset(cell, 0.035);
        let mut q = card("rect", b, graded_light(spec, k, n, 40_000, 18_000), spec, i);
        q.tb = inset(b, 0.1);
        out.push(q);
    }
    // The axes: up the left side and along the bottom.
    let t = axis * 0.55;
    let mut up = plain("upArrow", [x0 + (axis - t) / 2.0, y0, t, side_h - axis * 0.6], Paint::Accent(a, Some(70_000)));
    up.adj = vec![("adj1", 40_000), ("adj2", 100_000)];
    out.push(up);
    let mut right =
        plain("rightArrow", [x0 + axis * 0.6, y0 + side_h - axis + (axis - t) / 2.0, side_w - axis * 0.6, t], Paint::Accent(a, Some(70_000)));
    right.adj = vec![("adj1", 40_000), ("adj2", 100_000)];
    out.push(right);
    out
}

/// A circle cut into a wedge per item holding its own line; the lines under each item in a box
/// at the wedge's side of the graphic.
pub(crate) fn matrix_cycle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let wide = w >= h;
    // The circle, and room on its two sides (left and right, or above and below) for the boxes.
    let d = if wide { h.min(w * 0.5) } else { w.min(h * 0.5) } * 0.96;
    let (cx, cy) = (w / 2.0, h / 2.0);
    let circle = [cx - d / 2.0, cy - d / 2.0, d, d];
    let step = 360.0 / n as f32;
    // Wedges start at nine o'clock, so four items fill the quarters (top left first).
    let start = |k: usize| 180.0 + step * k as f32;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let mid = start(k) + step / 2.0;
        let p = graded(spec, k, n);
        let (tx, ty) = polar(cx, cy, d * if n == 1 { 0.0 } else { 0.27 }, mid);
        let s = if n == 1 { d * 0.6 } else { (d * 0.3).min(d * 0.38 * (step.min(180.0) / 2.0).to_radians().sin()) };
        let tb = [tx - s / 2.0, ty - s / 2.0, s, s];
        let mut wedge = tag(if n == 1 { "ellipse" } else { "pie" }, circle, p, spec, i).picked_by(tb);
        if n > 1 {
            wedge.adj = vec![("adj1", wedge_edge(10_800_000, k, n)), ("adj2", wedge_edge(10_800_000, k + 1, n))];
        }
        wedge.tb = tb;
        wedge.line = Paint::Light;
        wedge.line_w = 2.0;
        out.push(wedge);
    }
    // The boxes: on the side the wedge faces, sharing that side top to bottom (or left to right).
    let side = |k: usize| {
        let (c, s) = (start(k) + step / 2.0).to_radians().sin_cos();
        if wide { s < -1.0e-3 } else { c < -1.0e-3 }
    };
    let first: Vec<usize> = (0..tops.len()).filter(|&k| side(k)).collect();
    let second: Vec<usize> = (0..tops.len()).filter(|&k| !side(k)).collect();
    for (group, near) in [(first, true), (second, false)] {
        // Top to bottom (left to right): order by the wedge's middle.
        let mut group = group;
        let key = |k: usize| {
            let (s, c) = (start(k) + step / 2.0).to_radians().sin_cos();
            if wide { s } else { c }
        };
        group.sort_by(|a, b| key(*a).total_cmp(&key(*b)));
        let m = group.len().max(1) as f32;
        for (j, &k) in group.iter().enumerate() {
            let Some(&i) = tops.get(k) else { continue };
            let lines = lines_of(spec, i, false, true);
            if lines.is_empty() {
                continue;
            }
            let b = if wide {
                let bw = (w - d) / 2.0 * 0.9;
                let x = if near { (w - d) / 2.0 * 0.05 } else { cx + d / 2.0 + (w - d) / 2.0 * 0.05 };
                let bh = h / m;
                [x, bh * j as f32 + bh * 0.08, bw, bh * 0.84]
            } else {
                let bh = (h - d) / 2.0 * 0.9;
                let y = if near { (h - d) / 2.0 * 0.05 } else { cy + d / 2.0 + (h - d) / 2.0 * 0.05 };
                let bw = w / m;
                [bw * j as f32 + bw * 0.04, y, bw * 0.92, bh]
            };
            let mut s = plain("roundRect", b, graded_light(spec, k, n, 22_000, 12_000)).of_item(i);
            s.adj = vec![("adj", 10_000)];
            s.text = lines;
            s.tb = inset(b, 0.08);
            s.algn = Some(if wide && near {
                "r"
            } else if wide {
                "l"
            } else {
                "ctr"
            });
            out.push(s);
        }
    }
    out
}
