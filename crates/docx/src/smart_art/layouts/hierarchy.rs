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
fn hang(spec: &SmartArtSpec, list: &[usize], (x, top): (f32, f32), (bx, bw): (f32, f32), row: f32, accent: usize, half: bool) -> Vec<Sh> {
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
        if half {
            boxes.extend(half_rings(spec, i, b));
        } else {
            let mut s = tag("roundRect", b, tint(spec, i, 30_000), spec, i);
            s.adj = vec![("adj", 10_000)];
            s.tb = inset(b, 0.06);
            boxes.push(s);
        }
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
    org(spec, w, h, false)
}

/// The organization chart with each item's text inside two half rings in its colour.
pub(crate) fn org_chart_half_circle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    org(spec, w, h, true)
}

/// Item `i`'s own line in `b` between two half rings (left and right) in its colour.
fn half_rings(spec: &SmartArtSpec, i: usize, b: [f32; 4]) -> Vec<Sh> {
    let mut t = own_label(spec, i, b, "ctr");
    t.tb = [b[0] + b[2] * 0.1, b[1] + b[3] * 0.1, b[2] * 0.8, b[3] * 0.8];
    let a = spec.colors.accent(i);
    let band = (b[2].min(b[3]) * 0.07).max(0.75);
    let ss = b[2].min(b[3]).max(0.01);
    let mut out = vec![t];
    for (from, to) in [(110.0, 250.0), (290.0, 70.0)] {
        let mut arc = plain("blockArc", b, Paint::Accent(a, None));
        arc.adj = vec![
            ("adj1", super::cycle::ang(from)),
            ("adj2", super::cycle::ang(to)),
            ("adj3", (band / ss * 100_000.0).round().clamp(0.0, 50_000.0) as i64),
        ];
        out.push(arc);
    }
    out
}

fn org(spec: &SmartArtSpec, w: f32, h: f32, half: bool) -> Vec<Sh> {
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
        if half {
            boxes.extend(half_rings(spec, r, rb));
        } else {
            let mut root = tag("roundRect", rb, Paint::Accent(spec.colors.accent(r), None), spec, r);
            root.adj = vec![("adj", 10_000)];
            boxes.push(root);
        }
        let ky = rh + (rh - nh) / 2.0;
        let xs: Vec<f32> = (0..tree.kids(r).len()).map(|j| cw * (at + j) as f32 + cw / 2.0).collect();
        if !xs.is_empty() {
            lines.extend(elbows_down((x0 + x1) / 2.0, rb[1] + nh, &xs, ky, spec.colors.accent(r)));
        }
        for (j, &k) in tree.kids(r).iter().enumerate() {
            let cx0 = cw * (at + j) as f32;
            let kb = [cx0 + (cw - nw) / 2.0, ky, nw, nh];
            if half {
                boxes.extend(half_rings(spec, k, kb));
            } else {
                let mut s = tag("roundRect", kb, Paint::Accent(spec.colors.accent(k), None), spec, k);
                s.adj = vec![("adj", 10_000)];
                boxes.push(s);
            }
            let list: Vec<usize> = spec.descendants(k).collect();
            let hx = cx0 + cw * 0.12;
            lines.extend(hang(spec, &list, (hx, ky + nh), (cx0 + cw * 0.2, cw * 0.74), rh * 0.8, spec.colors.accent(k), half));
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
        lines.extend(hang(spec, &list, (x0 + cw * 0.14, b[1] + nh), (x0 + cw * 0.24, cw * 0.71), rh * 0.85, spec.colors.accent(k), false));
    }
    let (hang_boxes, plain_lines): (Vec<Sh>, Vec<Sh>) = lines.into_iter().partition(|s| s.prst != "line");
    let mut out = plain_lines;
    out.extend(boxes);
    out.extend(hang_boxes);
    out
}

/// Blocks in a row per level, each as wide as the leaves under it: the top items across the top.
pub(crate) fn hierarchy_table(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    blocks(spec, w, h, Blocks::Down)
}

/// Blocks stacked from the bottom, each top item along the bottom as wide as the items over it.
pub(crate) fn hierarchy_architecture(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    blocks(spec, w, h, Blocks::Up)
}

/// Blocks in a column per level from the left, each as tall as the items beside it.
pub(crate) fn hierarchy_table_horizontal(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    blocks(spec, w, h, Blocks::Across)
}

/// Which way the levels of a block hierarchy go: rows down from the top, rows up from the bottom,
/// or columns from the left.
#[derive(Clone, Copy, PartialEq)]
enum Blocks {
    Down,
    Up,
    Across,
}

fn blocks(spec: &SmartArtSpec, w: f32, h: f32, way: Blocks) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let start = tree.starts();
    let (along, deep) = if way == Blocks::Across { (h, w) } else { (w, h) };
    let unit = along / tree.total() as f32;
    let rh = deep / tree.depth as f32;
    let gap = unit.min(rh) * 0.05;
    let mut out = Vec::new();
    for (i, it) in spec.items.iter().enumerate() {
        let level = u32::from(it.level);
        let a = start.get(i).copied().unwrap_or(0.0) * unit + gap / 2.0;
        let la = (tree.leaves(i) as f32 * unit - gap).max(0.0);
        let d = if way == Blocks::Up { deep - (level + 1) as f32 * rh } else { level as f32 * rh } + gap / 2.0;
        let ld = (rh - gap).max(0.0);
        let b = if way == Blocks::Across { [d, a, ld, la] } else { [a, d, la, ld] };
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

/// A tree over a tinted band per level (rows down from the top, or columns from the left when
/// `across`). The first top item and every top item with items under it make the tree; the other
/// top items name the bands, in order.
fn labeled(spec: &SmartArtSpec, w: f32, h: f32, across: bool) -> Vec<Sh> {
    let tree = Tree::of(spec);
    let first = tree.roots.first().copied();
    let (roots, names): (Vec<usize>, Vec<usize>) = tree.roots.iter().partition(|&&r| Some(r) == first || !tree.kids(r).is_empty());
    // Where each tree item's run of leaves starts.
    let mut start = vec![0.0f32; spec.items.len()];
    let mut total = 0.0f32;
    for &r in &roots {
        if let Some(s) = start.get_mut(r) {
            *s = total;
        }
        total += tree.leaves(r) as f32;
    }
    for i in 0..spec.items.len() {
        let mut at = start.get(i).copied().unwrap_or(0.0);
        for &k in tree.kids(i) {
            if let Some(s) = start.get_mut(k) {
                *s = at;
            }
            at += tree.leaves(k) as f32;
        }
    }
    let total = total.max(1.0);
    let depth = roots
        .iter()
        .flat_map(|&r| std::iter::once(r).chain(spec.descendants(r)))
        .filter_map(|i| spec.items.get(i))
        .map(|it| usize::from(it.level))
        .max()
        .unwrap_or(0)
        + 1;
    let bands = depth.max(names.len()).max(1);
    // Along = the way a level's items spread; deep = the way levels go.
    let (along, deep) = if across { (h, w) } else { (w, h) };
    let name_room = along * if across { 0.16 } else { 0.2 };
    let band = deep / bands as f32;
    let unit = (along - name_room) / total;
    let box_of = |a: f32, la: f32, d: f32, ld: f32| if across { [d, a, ld, la] } else { [a, d, la, ld] };
    let mut out = Vec::new();
    for b in 0..bands {
        let bb = box_of(0.0, along, band * b as f32, band);
        let bb = [bb[0] + bb[2] * 0.004, bb[1] + bb[3] * 0.004, bb[2] * 0.992, bb[3] * 0.992];
        let mut s = plain("rect", bb, tint(spec, 0, if b % 2 == 0 { 16_000 } else { 26_000 }));
        if let Some(&i) = names.get(b) {
            let nb = box_of(along * 0.01, name_room * 0.94, band * b as f32, band);
            s = s.of_item(i).picked_by(nb);
            s.text = lines_of(spec, i, true, true);
            s.tb = nb;
            s.algn = Some(if across { "ctr" } else { "l" });
        }
        out.push(s);
    }
    let nl = unit * 0.84;
    let nd = (band * 0.56).min(if across { nl * 2.0 } else { nl * 0.8 });
    let mid_a = |i: usize| name_room + (start.get(i).copied().unwrap_or(0.0) + tree.leaves(i) as f32 / 2.0) * unit;
    let mid_d = |i: usize| spec.items.get(i).map_or(0.0, |it| band * (f32::from(it.level) + 0.5));
    let in_tree: Vec<usize> = roots.iter().flat_map(|&r| std::iter::once(r).chain(spec.descendants(r))).collect();
    for &i in &in_tree {
        let ks = tree.kids(i);
        if ks.is_empty() {
            continue;
        }
        let a = spec.colors.accent(i);
        if across {
            let (px, py) = (mid_d(i) + nd / 2.0, mid_a(i));
            let kx = ks.first().map_or(px, |&k| mid_d(k) - nd / 2.0);
            let mid = (px + kx) / 2.0;
            out.push(Sh::line(px, py, mid, py, a));
            let ys: Vec<f32> = ks.iter().map(|&k| mid_a(k)).collect();
            let (lo, hi) = ys.iter().fold((py, py), |(lo, hi), &y| (lo.min(y), hi.max(y)));
            if hi - lo > 0.5 {
                out.push(Sh::line(mid, lo, mid, hi, a));
            }
            out.extend(ys.iter().map(|&y| Sh::line(mid, y, kx, y, a)));
        } else {
            let xs: Vec<f32> = ks.iter().map(|&k| mid_a(k)).collect();
            let ct = ks.first().map_or(mid_d(i), |&k| mid_d(k) - nd / 2.0);
            out.extend(elbows_down(mid_a(i), mid_d(i) + nd / 2.0, &xs, ct, a));
        }
    }
    for &i in &in_tree {
        let b = box_of(mid_a(i) - nl / 2.0, nl, mid_d(i) - nd / 2.0, nd);
        let mut s = tag("roundRect", b, Paint::Accent(spec.colors.accent(i), None), spec, i);
        s.adj = vec![("adj", 10_000)];
        out.push(s);
    }
    out
}

pub(crate) fn hierarchy_labeled(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    labeled(spec, w, h, false)
}

pub(crate) fn hierarchy_labeled_horizontal(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    labeled(spec, w, h, true)
}
