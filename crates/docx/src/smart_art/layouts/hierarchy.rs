//! Hierarchy layouts: every item is a shape of its own (see `children_are_shapes`).

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Ink, Paint, ROUND_INSET, Sh, Tree, elbows_down, inset, lines_of, own, plain, tag, tint, with_text};

/// Item `i`'s own line in a box with no fill (its children have rows of their own).
fn own_label(spec: &SmartArtSpec, i: usize, b: [f32; 4], algn: &'static str) -> Sh {
    let mut s = plain("rect", b, Paint::None);
    s.text = lines_of(spec, i, true, false);
    s.item = Some(i);
    s.tb = b;
    s.algn = Some(algn);
    s
}

/// Items `list` hanging in a column from under a box whose bottom is at `top`: a line down the
/// column's left side (at `x`), a tick to each item's box, the boxes `bw` wide from `bx`, one per
/// `row` of height.
fn hang(spec: &SmartArtSpec, list: &[usize], (x, top): (f32, f32), (bx, bw): (f32, f32), row: f32, accent: usize) -> Vec<Sh> {
    let mut out = Vec::new();
    let bh = row * 0.78;
    let mut last = top;
    let mut boxes = Vec::new();
    for (j, &i) in list.iter().enumerate() {
        let y = top + row * j as f32 + (row - bh) / 2.0 + row * 0.1;
        let deeper = spec.items.get(i).map_or(0.0, |it| f32::from(it.level.saturating_sub(2).min(3)));
        let x0 = bx + bw * 0.08 * deeper;
        let b = [x0, y, (bx + bw - x0).max(0.0), bh];
        last = y + bh / 2.0;
        out.push(Sh::line(x, last, x0, last, accent));
        let mut s = tag("roundRect", b, tint(spec, i, 30_000), spec, i);
        s.adj = vec![("adj", 10_000)];
        s.tb = inset(b, 0.06);
        boxes.push(s);
    }
    if !list.is_empty() {
        out.insert(0, Sh::line(x, top, x, last, accent));
    }
    out.extend(boxes);
    out
}

/// A tree whose top two levels are rows (each top item over the items under it) and whose deeper
/// levels hang in a column under their second-level item.
pub(crate) fn org_chart(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    // Columns: each second-level item, or a top item with none.
    let mut cols: Vec<(usize, Option<usize>)> = Vec::new();
    for &r in &tree.roots {
        match tree.kids(r) {
            [] => cols.push((r, None)),
            ks => cols.extend(ks.iter().map(|&k| (r, Some(k)))),
        }
    }
    let c = cols.len().max(1);
    let hanging = |k: usize| spec.descendants(k).len();
    let most = cols.iter().filter_map(|(_, k)| k.map(hanging)).max().unwrap_or(0);
    let has_kids = cols.iter().any(|(_, k)| k.is_some());
    let units = if has_kids { 2.0 + most as f32 * 0.8 } else { 1.0 };
    let rh = h / units;
    let nh = (rh * 0.7).min(h * 0.3);
    let cw = w / c as f32;
    let nw = cw * 0.86;
    let mut lines = Vec::new();
    let mut boxes = Vec::new();
    let mut at = 0usize;
    for &r in &tree.roots {
        let span = tree.kids(r).len().max(1);
        let (x0, x1) = (cw * at as f32, cw * (at + span) as f32);
        let rw = ((x1 - x0) * 0.9).min(nw * 1.4).min(x1 - x0);
        let rb = [(x0 + x1) / 2.0 - rw / 2.0, (rh - nh) / 2.0, rw, nh];
        let mut root = tag("roundRect", rb, Paint::Accent(spec.colors.accent(r), None), spec, r);
        root.adj = vec![("adj", 10_000)];
        boxes.push(root);
        let ky = rh + (rh - nh) / 2.0;
        let xs: Vec<f32> = (0..tree.kids(r).len()).map(|j| cw * (at + j) as f32 + cw / 2.0).collect();
        if !xs.is_empty() {
            lines.extend(elbows_down((x0 + x1) / 2.0, rb[1] + nh, &xs, ky, spec.colors.accent(r)));
        }
        for (j, &k) in tree.kids(r).iter().enumerate() {
            let cx0 = cw * (at + j) as f32;
            let kb = [cx0 + (cw - nw) / 2.0, ky, nw, nh];
            let mut s = tag("roundRect", kb, Paint::Accent(spec.colors.accent(k), None), spec, k);
            s.adj = vec![("adj", 10_000)];
            boxes.push(s);
            let list: Vec<usize> = spec.descendants(k).collect();
            let hx = cx0 + cw * 0.12;
            lines.extend(hang(spec, &list, (hx, ky + nh), (cx0 + cw * 0.2, cw * 0.74), rh * 0.8, spec.colors.accent(k)));
        }
        at += span;
    }
    // Lines first so boxes sit on them; the hanging boxes came with their lines, after them.
    let (hang_boxes, plain_lines): (Vec<Sh>, Vec<Sh>) = lines.into_iter().partition(|s| s.prst != "line");
    let mut out = plain_lines;
    out.extend(boxes);
    out.extend(hang_boxes);
    out
}

/// A tree from left to right: a column per level, each item level with the middle of the items
/// under it, joined by elbow lines.
pub(crate) fn hierarchy_horizontal(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let start = tree.starts();
    let unit = h / tree.total() as f32;
    let colw = w / tree.depth as f32;
    let nw = colw * 0.74;
    let nh = (unit * 0.8).min(nw * 0.7);
    let cy = |i: usize| (start.get(i).copied().unwrap_or(0.0) + tree.leaves(i) as f32 / 2.0) * unit;
    let left = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * colw + (colw - nw) / 2.0);
    let mut out = Vec::new();
    for i in 0..spec.items.len() {
        let ks = tree.kids(i);
        if ks.is_empty() {
            continue;
        }
        let a = spec.colors.accent(i);
        let (px, py) = (left(i) + nw, cy(i));
        let kx = ks.first().map_or(px, |&k| left(k));
        let mid = (px + kx) / 2.0;
        out.push(Sh::line(px, py, mid, py, a));
        let ys: Vec<f32> = ks.iter().map(|&k| cy(k)).collect();
        let (lo, hi) = ys.iter().fold((py, py), |(lo, hi), &y| (lo.min(y), hi.max(y)));
        if hi - lo > 0.5 {
            out.push(Sh::line(mid, lo, mid, hi, a));
        }
        out.extend(ys.iter().map(|&y| Sh::line(mid, y, kx, y, a)));
    }
    for i in 0..spec.items.len() {
        let b = [left(i), cy(i) - nh / 2.0, nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(i)).with_item(spec, i, false));
    }
    out
}

/// Each top item heads a column; the items under it hang below on a line down its left side.
pub(crate) fn hierarchy_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let c = tree.roots.len().max(1);
    let most = tree.roots.iter().map(|&r| spec.descendants(r).len()).max().unwrap_or(0);
    let cw = w / c as f32;
    let units = 1.0 + most as f32 * 0.85;
    let rh = h / units;
    let nh = (rh * 0.78).min(h * 0.3);
    let mut lines = Vec::new();
    let mut boxes = Vec::new();
    for (k, &r) in tree.roots.iter().enumerate() {
        let x0 = cw * k as f32;
        let b = [x0 + cw * 0.05, (rh - nh) / 2.0, cw * 0.9, nh];
        let mut s = tag("roundRect", b, Paint::Accent(spec.colors.accent(k), None), spec, r);
        s.adj = vec![("adj", 10_000)];
        boxes.push(s);
        let list: Vec<usize> = spec.descendants(r).collect();
        lines.extend(hang(spec, &list, (x0 + cw * 0.14, b[1] + nh), (x0 + cw * 0.24, cw * 0.71), rh * 0.85, spec.colors.accent(k)));
    }
    let (hang_boxes, plain_lines): (Vec<Sh>, Vec<Sh>) = lines.into_iter().partition(|s| s.prst != "line");
    let mut out = plain_lines;
    out.extend(boxes);
    out.extend(hang_boxes);
    out
}

/// Blocks in a row per level, each as wide as the leaves under it: the top items across the top.
pub(crate) fn hierarchy_table(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let start = tree.starts();
    let unit = w / tree.total() as f32;
    let rh = h / tree.depth as f32;
    let gap = unit.min(rh) * 0.05;
    let mut out = Vec::new();
    for (i, it) in spec.items.iter().enumerate() {
        let level = u32::from(it.level);
        let b = [
            start.get(i).copied().unwrap_or(0.0) * unit + gap / 2.0,
            level as f32 * rh + gap / 2.0,
            (tree.leaves(i) as f32 * unit - gap).max(0.0),
            (rh - gap).max(0.0),
        ];
        let p = tint(spec, i, 100_000u32.saturating_sub(24_000 * level).max(28_000));
        out.push(tag("rect", b, p, spec, i));
    }
    out
}

/// A tree of circles, a row per level, each over the middle of the items under it.
pub(crate) fn hierarchy_circles(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let start = tree.starts();
    let unit = w / tree.total() as f32;
    let rh = h / tree.depth as f32;
    let d = (unit * 0.88).min(rh * 0.8);
    let cx = |i: usize| (start.get(i).copied().unwrap_or(0.0) + tree.leaves(i) as f32 / 2.0) * unit;
    let top = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * rh + (rh - d) / 2.0);
    let mut out = Vec::new();
    for i in 0..spec.items.len() {
        let ks = tree.kids(i);
        if ks.is_empty() {
            continue;
        }
        let xs: Vec<f32> = ks.iter().map(|&k| cx(k)).collect();
        let ct = ks.first().map_or(top(i) + d, |&k| top(k));
        out.extend(elbows_down(cx(i), top(i) + d, &xs, ct, spec.colors.accent(i)));
    }
    for i in 0..spec.items.len() {
        let b = [cx(i) - d / 2.0, top(i), d, d];
        let mut s = Sh::node("ellipse", b, spec.colors.accent(i)).with_item(spec, i, false).text_box(inset(b, ROUND_INSET));
        s.line_w = 2.0;
        out.push(s);
    }
    out
}

/// Each top item on the left under a rule in its colour, the items under it as rows beside it
/// with thin rules between them.
pub(crate) fn lined_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let rows = |r: usize| spec.descendants(r).len().max(1);
    let total: usize = tree.roots.iter().map(|&r| rows(r)).sum::<usize>().max(1);
    let rh = h / total as f32;
    let tx = w * 0.33;
    let mut out = Vec::new();
    let mut at = 0usize;
    for (k, &r) in tree.roots.iter().enumerate() {
        let y0 = rh * at as f32;
        let a = spec.colors.accent(k);
        let mut rule = Sh::line(0.0, y0 + 1.0f32.min(rh * 0.05), w, y0 + 1.0f32.min(rh * 0.05), a);
        rule.line = Paint::Accent(a, None);
        rule.line_w = 2.0;
        out.push(rule);
        let tb = [0.0, y0 + rh * 0.12, w * 0.3, (rows(r) as f32 * rh - rh * 0.12).max(0.0)];
        let mut title = with_text(plain("rect", tb, Paint::None), own(spec, r), "l", Ink::Accent(a), 1.15).of_item(r).text_box(tb);
        title.anchor = "t";
        out.push(title);
        for (j, i) in spec.descendants(r).enumerate() {
            let y = y0 + rh * j as f32;
            out.push(own_label(spec, i, [tx, y, (w - tx).max(0.0), rh], "l"));
            if j + 1 < rows(r) {
                let mut thin = Sh::line(tx, y + rh, w, y + rh, a);
                thin.line_w = 0.75;
                out.push(thin);
            }
        }
        at += rows(r);
    }
    out
}
