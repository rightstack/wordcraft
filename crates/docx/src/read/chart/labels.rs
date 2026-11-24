//! Data labels: where each goes and in which colour, so they stay readable, apart and inside the
//! chart.
//!
//! - Points (line, area, scatter, radar): just above the point in the dark text colour; labels
//!   of one category that would overlap go the lower one below its point, then apart by as little
//!   as needed. A label that would cross the plot's side is set flush with its point instead.
//! - Bars: inside the end of a clustered bar (outside when it doesn't fit), in the centre of a
//!   stacked segment (left out when it doesn't fit); white or dark by the fill's contrast.
//! - Pie slices: in the slice, or outside with a leader line when the slice is too thin.
//!
//! Every label is kept inside the bounds it is given; none is cut.

use std::collections::BTreeMap;

use super::canvas::{Rect, text_in, text_w};
use super::model::{DataLabel, LabelPos};
use super::{GraphicItem, LINE, Rgb, TextAlign};

/// Room between a label and the bar end, point or edge it sits by, points.
pub(super) const PAD: f32 = 3.0;
/// The most a label is moved down to clear another, in steps, before it is left where it is.
const MAX_NUDGES: usize = 64;

/// The colour that reads on `fill`: white when it has enough contrast there (WCAG, 4.5:1), else
/// `dark`.
pub(super) fn on_fill(fill: Rgb, dark: Rgb) -> Rgb {
    if contrast(Rgb::WHITE, fill) >= 4.5 { Rgb::WHITE } else { dark }
}

/// WCAG contrast ratio of two colours (1..21).
fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// WCAG relative luminance of an sRGB colour.
fn luminance(c: Rgb) -> f64 {
    let ch = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.039_28 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(c.0) + 0.7152 * ch(c.1) + 0.0722 * ch(c.2)
}

/// Width and height of a label's box at `fs` points.
pub(super) fn label_size(text: &str, fs: f32) -> (f32, f32) {
    (text_w(text, fs) + 4.0, fs * LINE)
}

/// A bar (or stacked segment) as drawn, for its label.
pub(super) struct BarSpot {
    pub(super) bar: Rect,
    /// A bar along x (else a column).
    pub(super) horiz: bool,
    /// It grows right or up from its base.
    pub(super) positive: bool,
    pub(super) stacked: bool,
    pub(super) fill: Rgb,
}

/// A bar's value label: clustered, inside its end, else outside it; stacked, in the middle, else
/// none. A file's `c:dLblPos` (`pos`) moves it where it can.
pub(super) fn bar_label(text: String, spot: &BarSpot, dark: Rgb, pos: Option<LabelPos>, fs: f32) -> Option<DataLabel> {
    let BarSpot { bar, horiz, positive, stacked, fill } = *spot;
    let (w, h) = label_size(&text, fs);
    // Room along the bar and across it.
    let (len, thick, need_len, need_thick) = if horiz { (bar.w, bar.h, w + 2.0 * PAD, h) } else { (bar.h, bar.w, h + 2.0 * PAD, w) };
    let fits = len >= need_len && (!stacked || thick >= need_thick);
    let place = match (stacked, pos) {
        (true, _) if !fits => return None,
        (true, _) => LabelPos::Centre,
        (false, Some(LabelPos::OutEnd)) => LabelPos::OutEnd,
        (false, Some(p @ (LabelPos::Centre | LabelPos::InBase))) if fits => p,
        (false, _) if fits => LabelPos::InEnd,
        (false, _) => LabelPos::OutEnd,
    };
    let (cx, cy) = (bar.x + bar.w / 2.0, bar.y + bar.h / 2.0);
    // The bar's end and base along the value direction (screen coordinates).
    let (end, base) = match (horiz, positive) {
        (true, true) => (bar.right(), bar.x),
        (true, false) => (bar.x, bar.right()),
        (false, true) => (bar.y, bar.bottom()),
        (false, false) => (bar.bottom(), bar.y),
    };
    // Which way the end points: +1 right or down, -1 left or up.
    let out = if (horiz && positive) || (!horiz && !positive) { 1.0 } else { -1.0 };
    let along = |at: f32, size: f32| match place {
        LabelPos::OutEnd => at + if out > 0.0 { PAD } else { -PAD - size },
        LabelPos::InEnd => at + if out > 0.0 { -PAD - size } else { PAD },
        LabelPos::InBase => base + if out > 0.0 { PAD } else { -PAD - size },
        _ => at,
    };
    let rect = if horiz {
        let x = if place == LabelPos::Centre { cx - w / 2.0 } else { along(end, w) };
        Rect { x, y: cy - h / 2.0, w, h }
    } else {
        let y = if place == LabelPos::Centre { cy - h / 2.0 } else { along(end, h) };
        Rect { x: cx - w / 2.0, y, w, h }
    };
    let color = if place == LabelPos::OutEnd { dark } else { on_fill(fill, dark) };
    Some(DataLabel::Placed { rect, text, color, align: TextAlign::Center })
}

/// Draw `labels` at size `fs`, point labels in `dark`, all kept inside `bounds` (a cartesian
/// chart's plot between its axes, with the room above it) and clear of `obstacles` (the axis
/// labels): one that would cover an obstacle moves up or down to the nearest free place, and is
/// left out when there is none. Point labels also keep clear of the placed ones (a combo's bar
/// labels), and one that lands on a solid shape in `surfaces` (a bar, with its fill) takes the
/// colour that reads there.
pub(super) fn draw_data_labels(
    out: &mut Vec<GraphicItem>,
    labels: Vec<DataLabel>,
    fs: f32,
    dark: Rgb,
    bounds: Rect,
    obstacles: &[Rect],
    surfaces: &[(Rect, Rgb)],
) {
    type PointLabel = (f32, f32, String, Option<LabelPos>, Option<Rgb>);
    let mut by_cat: BTreeMap<usize, Vec<PointLabel>> = BTreeMap::new();
    let mut taken: Vec<Rect> = Vec::new();
    for l in labels {
        match l {
            DataLabel::Placed { rect, text, color, align } => {
                if let Some(rect) = clear_of(clamp(rect, bounds), bounds, obstacles) {
                    text_in(out, rect, &text, fs, align, false, color);
                    taken.push(rect);
                }
            }
            DataLabel::Point { x, y, text, cat, pos, series } => by_cat.entry(cat).or_default().push((x, y, text, pos, series)),
        }
    }
    for mut group in by_cat.into_values() {
        // The highest point first: its label keeps the place above.
        group.sort_by(|a, b| a.1.total_cmp(&b.1));
        // The placed labels first: point labels move off them as off each other.
        let mut placed: Vec<Rect> = taken.clone();
        let first = placed.len();
        for (k, (x, y, text, pos, series)) in group.into_iter().enumerate() {
            let k = k + first;
            // A bar's place (a combo's line labels can carry its chart's `inEnd`) means above.
            let pos = pos.filter(|p| matches!(p, LabelPos::Top | LabelPos::Bottom | LabelPos::Left | LabelPos::Right | LabelPos::Centre));
            let (w, h) = label_size(&text, fs);
            let at = |below: bool| point_rect(x, y, w, h, if below { LabelPos::Bottom } else { pos.unwrap_or(LabelPos::Top) }, bounds);
            let mut rect = at(false);
            if placed.iter().any(|p| overlaps(*p, rect)) && k > 0 && matches!(pos, None | Some(LabelPos::Top)) {
                rect = at(true);
            }
            for _ in 0..MAX_NUDGES {
                let Some(hit) = placed.iter().find(|p| overlaps(**p, rect)) else { break };
                rect.y = hit.bottom() + 0.5;
            }
            let mut rect = clamp(rect, bounds);
            // Held inside the bounds onto its own point (a 100% stacked line along the top):
            // below the point instead.
            let covers = |r: Rect| r.y < y && y < r.bottom();
            if covers(rect) && matches!(pos, None | Some(LabelPos::Top)) {
                let below = clamp(at(true), bounds);
                if !covers(below) {
                    rect = below;
                }
            }
            placed.push(rect);
            if let Some(rect) = clear_of(rect, bounds, obstacles) {
                let on_bar = surfaces.iter().any(|(r, _)| overlaps(*r, rect));
                match series {
                    // A line's or area's label on a bar (a combo): on a white chip outlined in its
                    // series' colour, so it reads as the line's, apart from the bar's own labels.
                    Some(col) if on_bar => {
                        chip(out, rect, col);
                        text_in(out, rect, &text, fs, TextAlign::Center, false, dark);
                    }
                    _ => {
                        // On a solid shape (the topmost under the label's centre): white or dark by its fill.
                        let (cx, cy) = (rect.x + rect.w / 2.0, rect.y + rect.h / 2.0);
                        let under = surfaces.iter().rev().find(|(r, _)| cx >= r.x && cx <= r.right() && cy >= r.y && cy <= r.bottom());
                        let color = under.map_or(dark, |(_, f)| on_fill(*f, dark));
                        text_in(out, rect, &text, fs, TextAlign::Center, false, color);
                    }
                }
            }
        }
    }
}

/// A label's chip: a white rounded box a little larger than `r`, outlined 0.75 pt in `edge`.
fn chip(out: &mut Vec<GraphicItem>, r: Rect, edge: Rgb) {
    use super::PathSeg;
    let (x0, y0, x1, y1) = (r.x - 2.0, r.y - 0.5, r.right() + 2.0, r.bottom() + 0.5);
    let rad = ((y1 - y0) / 2.0).min(4.0);
    let k = 0.552_284_8 * rad;
    let segs = vec![
        PathSeg::Move(x0 + rad, y0),
        PathSeg::Line(x1 - rad, y0),
        PathSeg::Cubic(x1 - rad + k, y0, x1, y0 + rad - k, x1, y0 + rad),
        PathSeg::Line(x1, y1 - rad),
        PathSeg::Cubic(x1, y1 - rad + k, x1 - rad + k, y1, x1 - rad, y1),
        PathSeg::Line(x0 + rad, y1),
        PathSeg::Cubic(x0 + rad - k, y1, x0, y1 - rad + k, x0, y1 - rad),
        PathSeg::Line(x0, y0 + rad),
        PathSeg::Cubic(x0, y0 + rad - k, x0 + rad - k, y0, x0 + rad, y0),
        PathSeg::Close,
    ];
    out.push(GraphicItem::Path { segs, fill: Some(Rgb::WHITE), stroke: Some(edge), stroke_width: 0.75, gradient: None });
}

/// `r`, or `r` moved up or down by as little as needed (staying inside `bounds`) to cover none of
/// `obstacles`; `None` when no such place is within reach.
fn clear_of(r: Rect, bounds: Rect, obstacles: &[Rect]) -> Option<Rect> {
    let free = |c: Rect| !obstacles.iter().any(|o| overlaps(*o, c));
    if free(r) {
        return Some(r);
    }
    let reach = bounds.h.min(MAX_NUDGES as f32 * r.h);
    let step = (r.h / 4.0).max(1.0);
    let mut d = step;
    while d <= reach {
        for y in [r.y - d, r.y + d] {
            let c = Rect { y, ..r };
            if y >= bounds.y && y + r.h <= bounds.bottom() && free(c) {
                return Some(c);
            }
        }
        d += step;
    }
    None
}

/// A point label's box by the point (`x`, `y`) at `pos`; flush with the point (left or right)
/// where centring it would cross the side of `bounds`.
fn point_rect(x: f32, y: f32, w: f32, h: f32, pos: LabelPos, bounds: Rect) -> Rect {
    let (rx, ry) = match pos {
        LabelPos::Bottom => (x - w / 2.0, y + PAD),
        LabelPos::Left => (x - PAD - w, y - h / 2.0),
        LabelPos::Right => (x + PAD, y - h / 2.0),
        LabelPos::Centre => (x - w / 2.0, y - h / 2.0),
        _ => (x - w / 2.0, y - PAD - h),
    };
    // Flush with the point, a small gap inside the edge: left-justified at the value axis,
    // right-justified at the far edge.
    let rx = if rx < bounds.x + PAD {
        x.max(bounds.x) + PAD
    } else if rx + w > bounds.right() - PAD {
        x.min(bounds.right()) - PAD - w
    } else {
        rx
    };
    Rect { x: rx, y: ry, w, h }
}

/// `r` moved (not resized) to lie inside `b` as far as it fits.
fn clamp(r: Rect, b: Rect) -> Rect {
    let x = r.x.min(b.right() - r.w).max(b.x);
    let y = r.y.min(b.bottom() - r.h).max(b.y);
    Rect { x, y, ..r }
}

/// Whether two boxes overlap (touching is not overlapping).
pub(super) fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() && b.x < a.right() && a.y < b.bottom() && b.y < a.bottom()
}
