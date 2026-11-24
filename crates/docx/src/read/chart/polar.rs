use super::canvas::{Rect, clip_text, fill, stroke, text_in, text_w, widest};
use super::format::format_value;
use super::labels::{PAD, draw_data_labels, label_size, on_fill};
use super::model::{DataLabel, Kind, LabelPos, Series};
use super::plot::{Axes, has_values, tint};
use super::{AXIS, Chart, FRAC_PI_2, GAP, GRID, GraphicItem, LINE, PathSeg, Rgb, ShapeKind, TAU, TEXT, TextAlign, halo, restyle};

/// The bounding square of a pie or doughnut in `area`: centred, 2 pt inside its shorter side.
/// `None` when that leaves a radius under 1 pt.
pub(super) fn pie_square(area: Rect) -> Option<Rect> {
    let r = area.w.min(area.h) / 2.0 - 2.0;
    if !(r.is_finite() && r >= 1.0) {
        return None;
    }
    let (cx, cy) = (area.x + area.w / 2.0, area.y + area.h / 2.0);
    Some(Rect { x: cx - r, y: cy - r, w: 2.0 * r, h: 2.0 * r })
}

pub(super) fn pie(out: &mut Vec<GraphicItem>, c: &Chart, area: Rect) {
    let Some(s) = c.series.first() else { return };
    let vals: Vec<f64> = s.values.iter().map(|v| v.unwrap_or(0.0).max(0.0)).collect();
    let total: f64 = vals.iter().sum();
    let Some(sq) = pie_square(area) else { return };
    if !(total > 0.0 && total.is_finite()) {
        return;
    }
    let r = (sq.w / 2.0) as f64;
    let (cx, cy) = ((sq.x + sq.w / 2.0) as f64, (sq.y + sq.h / 2.0) as f64);
    let inner = if c.kind == Kind::Doughnut { r * c.hole as f64 } else { 0.0 };
    let mut a = c.first_angle.to_radians();
    let mut labels = Vec::new();
    for (i, v) in vals.iter().enumerate() {
        if *v <= 0.0 {
            continue;
        }
        let b = a + v / total * TAU;
        let mut segs = Vec::new();
        if inner > 0.0 {
            segs.push(pt(polar(cx, cy, r, a)));
            arc(&mut segs, (cx, cy, r), a, b);
            segs.push(line_to(polar(cx, cy, inner, b)));
            arc(&mut segs, (cx, cy, inner), b, a);
        } else {
            segs.push(pt((cx, cy)));
            segs.push(line_to(polar(cx, cy, r, a)));
            arc(&mut segs, (cx, cy, r), a, b);
        }
        segs.push(PathSeg::Close);
        fill(out, segs.clone(), c.point_colour(0, i));
        // A gap between slices: the series' outline (white) over the edges.
        if let Some((col, w)) = s.edge.filter(|e| e.1 > 0.0) {
            stroke(out, segs, col, w);
        }
        if let Some(text) = label_of(s, i) {
            let slice = Slice { cx, cy, r, inner, mid: (a + b) / 2.0, span: b - a, fill: c.point_colour(0, i) };
            labels.push(slice_label(out, text, &slice, c.label_dark(), s.label_pos, c.data_fs()));
        }
        a = b;
    }
    data_labels(out, c, labels, area);
}

/// The data labels in the chart's data label look (size, then weight, colour, font and outline).
fn data_labels(out: &mut Vec<GraphicItem>, c: &Chart, labels: Vec<DataLabel>, area: Rect) {
    let from = out.len();
    draw_data_labels(out, labels, c.data_fs(), c.label_dark(), area, &[], &[]);
    restyle(out.get_mut(from..).unwrap_or_default(), &c.data_look);
    if c.halo {
        halo(out, from);
    }
}

/// A radar chart in `area`: category spokes from the centre, concentric grid polygons at the value
/// ticks, one polygon per series (filled, or outlined with markers) and the category labels at the
/// spoke ends. The lowest tick is the centre.
/// Extra room between a radar's outer ring and its category names, points (beyond [`GAP`]).
const CAT_GAP: f32 = 6.0;

pub(super) fn radar(out: &mut Vec<GraphicItem>, c: &Chart, area: Rect) {
    const MARKER: f32 = 5.0;
    if c.n == 0 {
        return;
    }
    let fs = c.text_size;
    let lh = fs * LINE;
    let labelled = c.cat.labels && c.cats.iter().any(|s| !s.is_empty());
    // Category names stand off the outer ring; the top one also clears the outermost ring value,
    // which is centred on the ring at the top spoke.
    let out_gap = GAP + CAT_GAP;
    let top_gap = out_gap + if c.val.labelled() { lh / 2.0 } else { 0.0 };
    let side = if labelled { widest(&c.cats, fs).min(area.w * 0.25) + out_gap + 2.0 } else { 0.0 };
    let vert = if labelled { lh + top_gap } else { 0.0 };
    let r = ((area.w / 2.0 - side).min(area.h / 2.0 - vert) - 2.0) as f64;
    if r.is_nan() || r < 8.0 {
        return;
    }
    let (cx, cy) = ((area.x + area.w / 2.0) as f64, (area.y + area.h / 2.0) as f64);
    let vs = Axes::of(c).vs;
    let span = vs.hi - vs.lo;
    let angle = |i: usize| i as f64 / c.n as f64 * TAU;
    let at = |v: f64, i: usize| {
        let t = if span > 0.0 { ((v - vs.lo) / span).clamp(0.0, 1.0) } else { 0.0 };
        polar(cx, cy, r * t, angle(i))
    };
    let ticks = vs.ticks();
    if c.val.grid {
        for t in ticks.iter().skip(1) {
            let mut segs: Vec<PathSeg> = (0..c.n).map(|i| if i == 0 { pt(at(*t, i)) } else { line_to(at(*t, i)) }).collect();
            segs.push(PathSeg::Close);
            stroke(out, segs, GRID, 0.75);
        }
    }
    for i in 0..c.n {
        let end = polar(cx, cy, r, angle(i));
        stroke(out, vec![pt((cx, cy)), line_to(end)], GRID, 0.75);
    }
    let mut labels = Vec::new();
    for (j, s) in c.series.iter().enumerate().filter(|(_, s)| has_values(s)) {
        let col = c.series_colour(j);
        // A missing value counts as the centre, as Word draws it.
        let ring: Vec<(f64, f64)> = (0..c.n).map(|i| at(s.values.get(i).copied().flatten().unwrap_or(vs.lo), i)).collect();
        let mut segs: Vec<PathSeg> = ring.iter().enumerate().map(|(i, p)| if i == 0 { pt(*p) } else { line_to(*p) }).collect();
        segs.push(PathSeg::Close);
        // A path gradient: circular, out from the centre to the outer ring, its stops' opacities.
        let radial = s.gradient.as_deref().filter(|g| matches!(g.kind, wordcraft_doc::fill::GradientKind::Path { .. })).map(|g| {
            use wordcraft_doc::graphic::{GradientStop, LinearGradient};
            let stops = g
                .stops
                .iter()
                .map(|st| GradientStop {
                    offset: (st.pos / 100.0).clamp(0.0, 1.0),
                    color: st.color,
                    alpha: (1.0 - st.transparency / 100.0).clamp(0.0, 1.0),
                })
                .collect();
            LinearGradient { from: [cx as f32, cy as f32], to: [(cx + r) as f32, cy as f32], stops, radial: true }
        });
        if let Some(gradient) = radial {
            let plain = gradient.stops.last().map_or(col, |st| tint(st.color, st.alpha));
            out.push(GraphicItem::Path { segs: segs.clone(), fill: Some(plain), stroke: None, stroke_width: 0.0, gradient: Some(gradient) });
        } else if c.radar_fill {
            fill(out, segs.clone(), col);
        }
        stroke(out, segs, col, s.line_w);
        for (i, p) in ring.iter().enumerate() {
            if c.radar_marks && s.values.get(i).copied().flatten().is_some() {
                let rect = [p.0 as f32 - MARKER / 2.0, p.1 as f32 - MARKER / 2.0, MARKER, MARKER];
                out.push(GraphicItem::Shape { rect, kind: ShapeKind::Ellipse, fill: Some(col), stroke: None, stroke_width: 0.0 });
            }
            if let Some(text) = label_of(s, i) {
                labels.push(DataLabel::Point { x: p.0 as f32, y: p.1 as f32, text, cat: i, pos: s.label_pos, series: None });
            }
        }
    }
    // The labels over the series' shapes (the web stays under them): the ring values with a white
    // outline so they read on any fill, the category names, then the data labels on top.
    if c.val.labelled() {
        let from = out.len();
        for t in &ticks {
            let y = at(*t, 0).1 as f32;
            let s = format_value(*t, &c.val_fmt, vs.step);
            // Centred on the top spoke (the white outline keeps it readable over it).
            let w = text_w(&s, fs) + 4.0;
            text_in(
                out,
                Rect { x: cx as f32 - w / 2.0, y: y - lh / 2.0, w, h: lh },
                &s,
                fs,
                TextAlign::Center,
                false,
                c.val.text_color.unwrap_or(TEXT),
            );
        }
        restyle(out.get_mut(from..).unwrap_or_default(), &c.val.look);
        halo(out, from);
    }
    if labelled {
        let from = out.len();
        for (i, s) in c.cats.iter().enumerate().filter(|(_, s)| !s.is_empty()) {
            let a = angle(i);
            let (sx, sy) = (a.sin(), a.cos());
            let gap = if sx.abs() <= 0.2 && sy > 0.0 { top_gap } else { out_gap };
            let (x, y) = polar(cx, cy, r + f64::from(gap), a);
            let (x, y) = (x as f32, y as f32);
            let w = (side - out_gap).max(1.0);
            let txt = clip_text(s, w, fs);
            let (rect, align) = if sx > 0.2 {
                (Rect { x, y: y - lh / 2.0, w, h: lh }, TextAlign::Left)
            } else if sx < -0.2 {
                (Rect { x: x - w, y: y - lh / 2.0, w, h: lh }, TextAlign::Right)
            } else {
                let cw = (text_w(&txt, fs) + 4.0).max(1.0);
                (Rect { x: x - cw / 2.0, y: if sy > 0.0 { y - lh } else { y }, w: cw, h: lh }, TextAlign::Center)
            };
            text_in(out, rect, &txt, fs, align, false, c.cat.text_color.unwrap_or(TEXT));
        }
        restyle(out.get_mut(from..).unwrap_or_default(), &c.cat.look);
    }
    data_labels(out, c, labels, area);
}

/// The data label of point `i` of series `s`, when it shows one: the value as the file has it,
/// in the series' number format.
pub(super) fn label_of(s: &Series, i: usize) -> Option<String> {
    if s.labels != Some(true) {
        return None;
    }
    let v = s.raw.get(i).copied().flatten()?;
    let fmt = s.fmt.as_deref().filter(|f| !f.eq_ignore_ascii_case("General"));
    Some(match fmt {
        Some(f) => format_value(v, f, 1.0),
        None => general_value(v),
    })
}

/// A value in the `General` format: whole numbers without decimals, others with up to six.
fn general_value(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        return format!("{v:.0}");
    }
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".to_string() } else { s.to_string() }
}

/// A pie or doughnut slice as drawn, for its label.
struct Slice {
    /// Centre, outer and inner radius (0 for a pie).
    cx: f64,
    cy: f64,
    r: f64,
    inner: f64,
    /// The slice's middle angle and its angle, radians.
    mid: f64,
    span: f64,
    fill: Rgb,
}

/// A slice's value label: in the slice when it fits there and the file doesn't put it outside,
/// in the colour that reads on its fill; else just outside the circle with a light grey leader
/// line to it, in `dark`.
fn slice_label(out: &mut Vec<GraphicItem>, text: String, slice: &Slice, dark: Rgb, pos: Option<LabelPos>, fs: f32) -> DataLabel {
    let Slice { cx, cy, r, inner, mid, span, fill } = *slice;
    let (w, h) = label_size(&text, fs);
    // Halfway across the ring (pie: two thirds out from the centre).
    let at = if inner > 0.0 { (r + inner) / 2.0 } else { r * 0.65 };
    let room_across = (span * at) as f32;
    let room_out = (r - inner) as f32;
    let fits = room_across >= w.max(h) + 2.0 && room_out >= h;
    if fits && pos != Some(LabelPos::OutEnd) {
        let (x, y) = polar(cx, cy, at, mid);
        return DataLabel::Placed {
            rect: Rect { x: x as f32 - w / 2.0, y: y as f32 - h / 2.0, w, h },
            text,
            color: on_fill(fill, dark),
            align: TextAlign::Center,
        };
    }
    // Outside: a short leader from the rim, the label beyond it on the side it points to.
    let (x0, y0) = polar(cx, cy, r, mid);
    let (x1, y1) = polar(cx, cy, r + f64::from(LEADER), mid);
    stroke(out, vec![PathSeg::Move(x0 as f32, y0 as f32), PathSeg::Line(x1 as f32, y1 as f32)], AXIS, 0.75);
    let right = mid.sin() >= 0.0;
    let x = if right { x1 as f32 + PAD / 2.0 } else { x1 as f32 - PAD / 2.0 - w };
    DataLabel::Placed {
        rect: Rect { x, y: y1 as f32 - h / 2.0, w, h },
        text,
        color: dark,
        align: if right { TextAlign::Left } else { TextAlign::Right },
    }
}

/// Length of a pie label's leader line, points.
const LEADER: f32 = 8.0;

/// A point at clockwise angle `t` (radians from 12 o'clock) on a circle of radius `r`.
fn polar(cx: f64, cy: f64, r: f64, t: f64) -> (f64, f64) {
    (cx + r * t.sin(), cy - r * t.cos())
}

fn pt(p: (f64, f64)) -> PathSeg {
    PathSeg::Move(p.0 as f32, p.1 as f32)
}

fn line_to(p: (f64, f64)) -> PathSeg {
    PathSeg::Line(p.0 as f32, p.1 as f32)
}

/// Cubic segments along a circle from angle `a` to `b` (the current point is at `a`). Each piece
/// spans at most a quarter turn.
fn arc(segs: &mut Vec<PathSeg>, (cx, cy, r): (f64, f64, f64), a: f64, b: f64) {
    let n = (((b - a).abs() / FRAC_PI_2).ceil() as usize).clamp(1, 16);
    let d = (b - a) / n as f64;
    let k = 4.0 / 3.0 * (d / 4.0).tan();
    for j in 0..n {
        let t0 = a + d * j as f64;
        let t1 = t0 + d;
        let p0 = polar(cx, cy, r, t0);
        let p1 = polar(cx, cy, r, t1);
        let c1 = (p0.0 + k * r * t0.cos(), p0.1 + k * r * t0.sin());
        let c2 = (p1.0 - k * r * t1.cos(), p1.1 - k * r * t1.sin());
        segs.push(PathSeg::Cubic(c1.0 as f32, c1.1 as f32, c2.0 as f32, c2.1 as f32, p1.0 as f32, p1.1 as f32));
    }
}

/// A slice of a circle centred (`cx`, `cy`) with radius `r`, from angle `a` to `b`.
/// Where a line from `p` touches the circle (`cx`, `cy`, `r`): the upper tangent point when
/// `upper`, else the lower one; the circle's top or bottom when `p` is inside it.
fn tangent(p: (f64, f64), (cx, cy, r): (f64, f64, f64), upper: bool) -> (f64, f64) {
    let (dx, dy) = (p.0 - cx, p.1 - cy);
    let d = dx.hypot(dy);
    if !(d > r && d.is_finite()) {
        return (cx, if upper { cy - r } else { cy + r });
    }
    let (base, spread) = (dy.atan2(dx), (r / d).acos());
    let at = |t: f64| (cx + r * t.cos(), cy + r * t.sin());
    let (a, b) = (at(base + spread), at(base - spread));
    if (a.1 < b.1) == upper { a } else { b }
}

/// With `edge`, its outline (a gap between slices) over it.
fn wedge(out: &mut Vec<GraphicItem>, (cx, cy, r): (f64, f64, f64), a: f64, b: f64, color: Rgb, edge: Option<(Rgb, f32)>) {
    let mut segs = vec![pt((cx, cy)), line_to(polar(cx, cy, r, a))];
    arc(&mut segs, (cx, cy, r), a, b);
    segs.push(PathSeg::Close);
    fill(out, segs.clone(), color);
    if let Some((col, w)) = edge.filter(|e| e.1 > 0.0) {
        stroke(out, segs, col, w);
    }
}

/// A pie of pie or bar of pie in `area`: the main pie at the left with the last `split` points
/// joined into one slice facing right, those points again as a smaller pie or a stacked bar at
/// the right, and lines from that slice's edges to the second plot.
pub(super) fn of_pie(out: &mut Vec<GraphicItem>, c: &Chart, area: Rect) {
    let Some(s) = c.series.first() else { return };
    let vals: Vec<f64> = s.values.iter().map(|v| v.unwrap_or(0.0).max(0.0)).collect();
    let n = vals.len();
    let k = c.split.min(n.saturating_sub(1));
    let total: f64 = vals.iter().sum();
    if k == 0 || !(total > 0.0 && total.is_finite()) {
        pie(out, c, area);
        return;
    }
    let rest: f64 = vals.iter().skip(n - k).sum();
    let left = Rect { w: area.w * 0.58, ..area };
    let Some(sq) = pie_square(left) else { return };
    let r = (sq.w / 2.0) as f64;
    let (cx, cy) = ((sq.x + sq.w / 2.0) as f64, (sq.y + sq.h / 2.0) as f64);
    // The joined slice centred on 3 o'clock, the others clockwise after it. It is drawn in its
    // points' colours (as the second plot has them), so where they went reads at a glance.
    let joined = rest / total * TAU;
    let a0 = FRAC_PI_2 - joined / 2.0;
    let edge = s.edge;
    let mut aj = a0;
    for (j, v) in vals.iter().enumerate().skip(n - k) {
        let bj = aj + v / total * TAU;
        if *v > 0.0 {
            wedge(out, (cx, cy, r), aj, bj, c.point_colour(0, j), edge);
        }
        aj = bj;
    }
    let mut labels = Vec::new();
    let mut a = a0 + joined;
    for (i, v) in vals.iter().enumerate().take(n - k) {
        if *v <= 0.0 {
            continue;
        }
        let b = a + v / total * TAU;
        wedge(out, (cx, cy, r), a, b, c.point_colour(0, i), edge);
        if let Some(text) = label_of(s, i) {
            let slice = Slice { cx, cy, r, inner: 0.0, mid: (a + b) / 2.0, span: b - a, fill: c.point_colour(0, i) };
            labels.push(slice_label(out, text, &slice, c.label_dark(), s.label_pos, c.data_fs()));
        }
        a = b;
    }
    // The second plot, right of the main pie, and the lines to it from the joined slice's edges.
    let room = Rect { x: area.x + area.w * 0.64, w: area.w * 0.34, ..area };
    let (p_top, p_bottom) = (polar(cx, cy, r, a0), polar(cx, cy, r, a0 + joined));
    let (q_top, q_bottom);
    // The second plot is drawn after the lines to it, so they stop at its edge.
    let mut second = Vec::new();
    if c.of_bar {
        let h = (room.h * 0.85).min(sq.h);
        let bw = (room.w * 0.4).max(4.0);
        let (x, mut y) = (room.x + (room.w - bw) / 2.0, room.y + (room.h - h) / 2.0);
        (q_top, q_bottom) = ((x as f64, y as f64), (x as f64, (y + h) as f64));
        for (j, v) in vals.iter().enumerate().skip(n - k) {
            let sh = (v / rest) as f32 * h;
            let segs = vec![PathSeg::Move(x, y), PathSeg::Line(x + bw, y), PathSeg::Line(x + bw, y + sh), PathSeg::Line(x, y + sh), PathSeg::Close];
            fill(&mut second, segs.clone(), c.point_colour(0, j));
            if let Some((col, w)) = edge.filter(|e| e.1 > 0.0) {
                stroke(&mut second, segs, col, w);
            }
            y += sh;
        }
    } else {
        let r2 = (r * 0.75).min((room.w / 2.0) as f64).min((room.h / 2.0) as f64).max(1.0);
        let (cx2, cy2) = ((room.x + room.w / 2.0) as f64, cy);
        // The lines touch the second pie (tangents from the joined slice's edges), so none
        // crosses it.
        (q_top, q_bottom) = (tangent(p_top, (cx2, cy2, r2), true), tangent(p_bottom, (cx2, cy2, r2), false));
        // In the joined slice's order from top to bottom: the points go anticlockwise from 3
        // o'clock (over the top, down the side facing the main pie), so the first is at the top
        // as it is in the joined slice. Drawn clockwise from 3 o'clock, last point first.
        let mut b0 = FRAC_PI_2;
        for (j, v) in vals.iter().enumerate().skip(n - k).rev() {
            let b1 = b0 + v / rest * TAU;
            if *v > 0.0 {
                wedge(&mut second, (cx2, cy2, r2), b0, b1, c.point_colour(0, j), edge);
            }
            b0 = b1;
        }
    }
    for (p, q) in [(p_top, q_top), (p_bottom, q_bottom)] {
        stroke(out, vec![pt(p), line_to(q)], AXIS, 0.75);
    }
    out.extend(second);
    data_labels(out, c, labels, area);
}
