use super::canvas::{
    End, Rect, circle_segs, clip_text, curves, dashes, fill, rect_segs, round_end_segs, runs, seg, stroke, text_at, text_in, text_w, widest, wrap,
};
use super::format::format_value;
use super::labels::{BarSpot, bar_label, draw_data_labels, label_size, on_fill};
use super::model::{Axis, DataLabel, Grouping, Kind, LabelPos, Marker, Series};
use super::parse::category_text;
use super::polar::label_of;
use super::{
    AXIS, Chart, El, GAP, GRID, GraphicItem, LINE, MAX_LABEL, MAX_TICKS, PathSeg, Rgb, ShapeKind, TEXT, TextAlign, fill_colour, halo, measure,
    restyle,
};

pub(super) fn label(s: &str) -> String {
    s.chars().take(MAX_LABEL).collect()
}

/// Chart area: the chart space's fill and border (white and borderless by default).
pub(super) fn backdrop(out: &mut Vec<GraphicItem>, space: &El, theme: &[Rgb], w: f32, h: f32) {
    let sp = space.child("c:spPr");
    let fill = match sp {
        Some(p) if p.child("a:noFill").is_some() => None,
        Some(p) => fill_colour(p, theme).or(Some(Rgb::WHITE)),
        None => Some(Rgb::WHITE),
    };
    let ln = sp.and_then(|p| p.child("a:ln"));
    let stroke = ln.and_then(|l| fill_colour(l, theme));
    let stroke_width = ln.and_then(|l| l.attr("w")).and_then(|v| measure(v, 12_700.0)).unwrap_or(0.75).clamp(0.0, 20.0);
    if fill.is_some() || stroke.is_some() {
        out.push(GraphicItem::Shape { rect: [0.0, 0.0, w, h], kind: ShapeKind::Rectangle, fill, stroke, stroke_width });
    }
}

/// An axis scale: `lo` to `hi` in steps of `step`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Scale {
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) step: f64,
}

impl Scale {
    pub(super) fn ticks(&self) -> Vec<f64> {
        let n = (((self.hi - self.lo) / self.step + 1e-9).floor().max(0.0) as usize).min(MAX_TICKS);
        (0..=n).map(|k| self.lo + k as f64 * self.step).filter(|t| *t <= self.hi + 1e-9).collect()
    }
}

/// [`scale`] with an automatic step.
#[cfg(test)]
pub(super) fn nice_scale(dlo: f64, dhi: f64, zero: bool, min: Option<f64>, max: Option<f64>) -> Scale {
    scale(dlo, dhi, zero, min, max, None)
}

/// A scale for data `dlo..dhi` (zero included when `zero`), as Excel and Word pick it: an end
/// away from zero sits 5% of the data span beyond the data (an end at zero stays there), rounded
/// out to a step of 1, 2, 2.5, 5 or 10 times a power of ten (about seven steps). `min`/`max`
/// from the file replace the ends (one set on the wrong side of the other end is met by the
/// data span), and its major unit (`unit`, `c:majorUnit`) replaces the step, made a whole
/// multiple larger when it would give more than [`MAX_TICKS`] ticks.
pub(super) fn scale(dlo: f64, dhi: f64, zero: bool, min: Option<f64>, max: Option<f64>, unit: Option<f64>) -> Scale {
    let (dlo, dhi) = if dhi > dlo { (dlo, dhi) } else { (dlo, dlo + 1.0) };
    let span = dhi - dlo;
    // With zero, the ends reach it: all-negative data still starts its bars at zero.
    let (ends_lo, ends_hi) = if zero { (dlo.min(0.0), dhi.max(0.0)) } else { (dlo, dhi) };
    let mut lo = min.unwrap_or(if ends_lo < 0.0 { ends_lo - 0.05 * span } else { ends_lo });
    let mut hi = max.unwrap_or(if ends_hi > 0.0 { ends_hi + 0.05 * span } else { ends_hi });
    let unit = unit.filter(|u| u.is_finite() && *u > 0.0);
    if hi <= lo {
        if max.is_none() {
            hi = lo + unit.unwrap_or(span).max(span);
        } else if min.is_none() {
            lo = hi - unit.unwrap_or(span).max(span);
        }
    }
    if !(hi > lo && hi.is_finite() && lo.is_finite()) {
        return Scale { lo: 0.0, hi: 1.0, step: 0.2 };
    }
    let raw = (hi - lo) / 7.5;
    let mag = 10f64.powf(raw.log10().floor());
    let auto = [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * mag).find(|s| *s >= raw).unwrap_or(10.0 * mag);
    // Rounding the ends out can add a step at each end: leave room for both.
    let most = MAX_TICKS.saturating_sub(2).max(1) as f64;
    let step = match unit {
        Some(u) if (hi - lo) / u > most => Some(u * ((hi - lo) / u / most).ceil()),
        other => other,
    }
    .filter(|s| s.is_finite() && *s > 0.0)
    .unwrap_or(auto);
    let lo_r = if min.is_some() { lo } else { ((lo / step) + 1e-9).floor() * step };
    let hi_r = if max.is_some() { hi } else { ((hi / step) - 1e-9).ceil() * step };
    if !(step.is_finite() && step > 0.0 && lo_r.is_finite() && hi_r.is_finite() && hi_r > lo_r) {
        return Scale { lo: 0.0, hi: 1.0, step: 0.2 };
    }
    Scale { lo: lo_r, hi: hi_r, step }
}

/// The plot rectangle and the scales that map values and categories onto it.
struct Frame {
    plot: Rect,
    horiz: bool,
    n: usize,
    vs: Scale,
    xs: Option<Scale>,
    mid: bool,
}

impl Frame {
    /// Width of one category band.
    fn band(&self) -> f32 {
        let len = if self.horiz { self.plot.h } else { self.plot.w };
        len / self.n.max(1) as f32
    }

    /// Start of category `i`'s band: its left edge, or for bars its top edge (category 0 at the
    /// bottom).
    fn band_start(&self, i: usize) -> f32 {
        let k = i as f32;
        if self.horiz { self.plot.bottom() - (k + 1.0) * self.band() } else { self.plot.x + k * self.band() }
    }

    /// A value's position along the value direction: y for columns, x for bars.
    fn val(&self, v: f64) -> f32 {
        let t = ((v - self.vs.lo) / (self.vs.hi - self.vs.lo)).clamp(0.0, 1.0) as f32;
        if self.horiz { self.plot.x + t * self.plot.w } else { self.plot.bottom() - t * self.plot.h }
    }

    /// x of category `i` for line and area points.
    fn cat_x(&self, i: usize) -> f32 {
        if !self.mid {
            return self.plot.x + (i as f32 + 0.5) * self.band();
        }
        if self.n <= 1 { self.plot.x + self.plot.w / 2.0 } else { self.plot.x + self.plot.w * i as f32 / (self.n - 1) as f32 }
    }

    /// x of a scatter point.
    fn scat_x(&self, x: f64) -> f32 {
        match self.xs {
            Some(s) => self.plot.x + ((x - s.lo) / (s.hi - s.lo)).clamp(0.0, 1.0) as f32 * self.plot.w,
            None => self.plot.x,
        }
    }
}

/// The value scale and, for scatter charts, the x scale, with their ticks and tick texts.
pub(super) struct Axes {
    pub(super) vs: Scale,
    vticks: Vec<(f64, String)>,
    xs: Option<Scale>,
    xticks: Vec<(f64, String)>,
}

impl Axes {
    pub(super) fn of(c: &Chart) -> Axes {
        let (dlo, dhi) = extent(c);
        let (vmin, vmax) = if c.grouping == Grouping::Percent { (Some(0.0), Some(1.0)) } else { (c.val.min, c.val.max) };
        // Lines and scatters don't need zero for a narrow range (1000..1050 stays on its own scale).
        let zero = !matches!(c.kind, Kind::Line | Kind::Scatter | Kind::Bubble) || !narrow(dlo, dhi);
        let vs = scale(dlo, dhi, zero, vmin, vmax, c.val.step);
        let vticks = vs.ticks().into_iter().map(|t| (t, format_value(t, &c.val_fmt, vs.step))).collect();
        let xs = xy(c).then(|| {
            let (xl, xh) = x_extent(c);
            // Room left of the first points, so they don't sit on the value axis: the low end
            // goes 5% of the span below the data (not below zero) before it's rounded down.
            let xl = if c.cat.min.is_none() && xl > 0.0 && xh > xl { (xl - 0.05 * (xh - xl)).max(0.0) } else { xl };
            scale(xl, xh, false, c.cat.min, c.cat.max, c.cat.step)
        });
        // Scatter x values show in the axis's own format, else their cache's (dates included).
        let xfmt = c.cat.fmt.clone().or_else(|| c.series.iter().find_map(|s| s.x_fmt.clone())).unwrap_or_else(|| "General".to_string());
        let xticks = xs.map(|s| s.ticks().into_iter().map(|t| (t, category_text(t, &xfmt, s.step, c.date1904))).collect()).unwrap_or_default();
        Axes { vs, vticks, xs, xticks }
    }
}

/// The room around the plot for axis labels, and the category labels as they'll be drawn.
struct Margins {
    left: f32,
    top: f32,
    bottom: f32,
    /// Category labels in lines (none for scatter charts or a deleted category axis).
    wrapped: Vec<Vec<String>>,
    /// Every `skip`-th category label is shown under a column chart.
    skip: usize,
}

/// The margins the axis labels need around the plot in `area`. Category labels under a column
/// chart wrap to the band; labels that still don't fit show every k-th category.
fn axis_margins(c: &Chart, ax: &Axes, area: Rect) -> Margins {
    let (fs, vfs) = (c.cat_fs(), c.val_fs());
    let (lh, vlh) = (fs * LINE, vfs * LINE);
    let scatter = xy(c);
    let (cat_on, val_on) = (c.cat.labelled(), c.val.labelled());
    // Value tick marks reaching out of the plot push the value labels out by as much.
    let tick_out = val_tick_out(c);
    let left = if c.horizontal {
        if cat_on { widest(&c.cats, fs).min(area.w * 0.4) + GAP } else { 0.0 }
    } else if val_on {
        ax.vticks.iter().map(|(_, s)| text_w(s, vfs)).fold(0.0, f32::max) + GAP + tick_out
    } else {
        // No labels: only the tick marks, if any, need room.
        tick_out
    };
    let band_w = (area.w - left).max(1.0) / c.n as f32;
    let slanted = slants(c);
    let wrapped: Vec<Vec<String>> = if cat_on && !scatter && slanted {
        // Slanted labels stay on one line, cut to what the room under the plot can take.
        let longest = area.h * SLANT_SHARE / slant_sin(c).max(0.1);
        c.cats.iter().map(|s| vec![clip_text(s, longest, fs)]).collect()
    } else if cat_on && !scatter {
        let width = if c.horizontal { left - GAP } else { band_w };
        c.cats.iter().map(|s| wrap(s, width, fs)).collect()
    } else {
        Vec::new()
    };
    let widest_line = wrapped.iter().flatten().map(|s| text_w(s, fs)).fold(0.0, f32::max);
    // Slanted labels need a line's height across the slant between them; level ones their width.
    let pitch = if slanted { lh / slant_sin(c).max(0.1) } else { widest_line };
    let skip = ((pitch / band_w).ceil() as usize).max(1);
    let label_lines = wrapped.iter().map(Vec::len).max().unwrap_or(1) as f32;
    let bottom = if cat_on && !c.horizontal && slanted {
        let (sin, cos) = (slant_sin(c), slant_cos(c));
        (widest_line + 4.0) * sin + lh * cos + cat_gap(c) + GAP / 2.0
    } else if cat_on && !c.horizontal {
        // Down from the axis line by the category gap, a little room under them.
        let rows = if scatter { 1.0 } else { label_lines };
        rows * lh + cat_gap(c) + GAP / 2.0
    } else if c.horizontal && val_on {
        vlh + GAP + tick_out
    } else if c.horizontal {
        tick_out
    } else {
        cat_tick_out(c)
    };
    let top = if !c.horizontal && val_on { vlh / 2.0 } else { 0.0 };
    Margins { left, top, bottom, wrapped, skip }
}

/// The layout of a bar, column, line, area or scatter chart in `area`: its scales, the room
/// for axis labels and the plot rectangle between the axes (the value axis at its left or
/// bottom edge, the category axis at its bottom or left edge).
pub(super) struct PlotFrame {
    ax: Axes,
    m: Margins,
    pub(super) plot: Rect,
}

/// [`PlotFrame`] of `c` in `area`; `None` when there's no category or no room for a plot.
pub(super) fn plot_frame(c: &Chart, area: Rect) -> Option<PlotFrame> {
    if c.n == 0 {
        return None;
    }
    let mut ax = Axes::of(c);
    let mut area = area;
    // A combo chart: the primary value axis spans its own series' groups, and its secondary
    // axis's labels take room at the right.
    if c.combo {
        (ax.vs, ax.vticks) = combo_scale(&combo_views(c, false), &c.val);
        let second = combo_views(c, true);
        if let Some(a2) = c.val2.as_ref().filter(|a| a.labelled() && !second.is_empty()) {
            let (_, ticks) = combo_scale(&second, a2);
            let right = ticks.iter().map(|(_, s)| text_w(s, c.val_fs())).fold(0.0, f32::max) + GAP + 2.0;
            area.w = (area.w - right).max(1.0);
        }
    }
    let m = axis_margins(c, &ax, area);
    let plot = Rect { x: area.x + m.left, y: area.y + m.top, w: area.w - m.left, h: area.h - m.top - m.bottom };
    (plot.w >= 1.0 && plot.h >= 1.0).then_some(PlotFrame { ax, m, plot })
}

/// A bar, column, line, area or scatter chart in `area`: gridlines, series, axes, then labels.
pub(super) fn cartesian(out: &mut Vec<GraphicItem>, c: &Chart, area: Rect) {
    let Some(PlotFrame { ax, m, plot }) = plot_frame(c, area) else { return };
    let g = Frame { plot, horiz: c.horizontal, n: c.n, vs: ax.vs, xs: ax.xs, mid: c.val.mid };
    if let Some((color, alpha)) = c.plot_fill {
        shade(out, plot, color, alpha);
    }
    if c.val.grid {
        draw_grid(out, &g, &ax.vticks, &c.val);
    }
    let mut labels = Vec::new();
    // Where in `out` the bars went (their shapes are surfaces for the point labels).
    let mut bar_runs: Vec<(usize, usize)> = Vec::new();
    // A combo chart's secondary scale and ticks, for its right axis.
    let mut right_axis: Option<(Frame, Vec<(f64, String)>)> = None;
    if c.combo {
        for second in [false, true] {
            let views = combo_views(c, second);
            if views.is_empty() {
                continue;
            }
            let (vs, ticks) = if second { combo_scale(&views, c.val2.as_ref().unwrap_or(&c.val)) } else { (ax.vs, ax.vticks.clone()) };
            let gv = Frame { plot, horiz: false, n: c.n, vs, xs: None, mid: c.val.mid };
            let mut area_runs: Vec<(usize, usize)> = Vec::new();
            let mut has_bars = false;
            for v in &views {
                match v.kind {
                    Kind::Area => {
                        let from = out.len();
                        areas(out, v, &gv, &mut labels);
                        area_runs.push((from, out.len()));
                    }
                    Kind::Bar => {
                        let from = out.len();
                        bars(out, v, &gv, &mut labels);
                        bar_runs.push((from, out.len()));
                        has_bars = true;
                    }
                    _ => lines(out, v, &gv, &mut labels),
                }
            }
            // An area's top edge in front of the columns (its fill stays behind them), so the line
            // runs unbroken across them. The edge is moved: hidden where it was, drawn again last.
            if has_bars {
                let mut edges = Vec::new();
                for &(a, b) in &area_runs {
                    for it in out.get_mut(a..b).unwrap_or_default() {
                        if matches!(it, GraphicItem::Path { fill: None, stroke: Some(_), .. }) {
                            edges.push(it.clone());
                            if let GraphicItem::Path { stroke, .. } = it {
                                *stroke = None;
                            }
                        }
                    }
                }
                out.extend(edges);
            }
            if second {
                right_axis = Some((gv, ticks));
            }
        }
    } else {
        match c.kind {
            Kind::Bar => {
                let from = out.len();
                bars(out, c, &g, &mut labels);
                bar_runs.push((from, out.len()));
            }
            Kind::Line => lines(out, c, &g, &mut labels),
            Kind::Area => areas(out, c, &g, &mut labels),
            Kind::Scatter => draw_scatter(out, c, &g, &mut labels),
            Kind::Bubble => bubbles(out, c, &g, &mut labels),
            Kind::Pie | Kind::Doughnut | Kind::Radar | Kind::OfPie => {}
        }
    }
    draw_axes(out, c, &g, &ax.vticks, &ax.xticks);
    if let (Some((gv, ticks)), Some(a2)) = (&right_axis, c.val2.as_ref()) {
        right_value_axis(out, c, gv, ticks, a2, area);
    }
    let axis_labels_from = out.len();
    if c.val.labelled() {
        draw_value_labels(out, c, &g, &ax.vticks, Rect { w: m.left, ..area });
        restyle(out.get_mut(axis_labels_from..).unwrap_or_default(), &c.val.look);
    }
    let cat_labels_from = out.len();
    if c.cat.labelled() {
        draw_category_labels(out, c, &g, &ax.xticks, &m, area);
        restyle(out.get_mut(cat_labels_from..).unwrap_or_default(), &c.cat.look);
    }
    if !g.horiz {
        clear_category_labels(out, axis_labels_from, cat_labels_from);
    }
    // Data labels last: across the plot between the axes (from the value axis to the far edge),
    // down to the category axis and up into the room above the plot, clear of the axis labels.
    let obstacles: Vec<Rect> = out
        .get(axis_labels_from..)
        .unwrap_or_default()
        .iter()
        .filter_map(|it| if let GraphicItem::Text { rect, .. } = it { Some(Rect { x: rect[0], y: rect[1], w: rect[2], h: rect[3] }) } else { None })
        .collect();
    let (top, bottom) = if g.horiz { (plot.y, plot.bottom()) } else { (area.y, plot.bottom()) };
    let bounds = Rect { x: plot.x, y: top, w: plot.w, h: (bottom - top).max(0.0) };
    // The bars, for a point label on one (a combo's line or area label) to read on it.
    let surfaces: Vec<(Rect, Rgb)> = bar_runs
        .iter()
        .flat_map(|&(a, b)| out.get(a..b).unwrap_or_default().iter())
        .filter_map(|it| match it {
            GraphicItem::Path { segs, fill: Some(f), gradient: None, .. } => seg_box(segs).map(|r| (r, *f)),
            _ => None,
        })
        .collect();
    let data_from = out.len();
    draw_data_labels(out, labels, c.data_fs(), c.label_dark(), bounds, &obstacles, &surfaces);
    restyle(out.get_mut(data_from..).unwrap_or_default(), &c.data_look);
    if c.halo {
        halo(out, data_from);
    }
}

/// Width of a gridline, points.
const GRID_W: f32 = 0.75;

/// Major gridlines at the value ticks, solid or dashed (dashes 4 and gaps 3 line widths long,
/// the `dash` preset of ECMA-376 §20.1.10.48, drawn as separate pieces of the path).
fn draw_grid(out: &mut Vec<GraphicItem>, g: &Frame, ticks: &[(f64, String)], ax: &Axis) {
    let plot = g.plot;
    let (color, w) = (ax.grid_color.unwrap_or(GRID), ax.grid_w.unwrap_or(GRID_W));
    // Dash lengths follow the line width; a hairline still gets visible dashes.
    let dw = w.max(GRID_W);
    for (t, _) in ticks {
        let p = g.val(*t);
        let (x0, y0, x1, y1) = if g.horiz { (p, plot.y, p, plot.bottom()) } else { (plot.x, p, plot.right(), p) };
        let segs = if ax.grid_dash { dashes(x0, y0, x1, y1, 4.0 * dw, 3.0 * dw) } else { seg(x0, y0, x1, y1) };
        stroke(out, segs, color, w);
    }
}

/// Length of a major tick mark, points.
const TICK: f32 = 4.0;

/// How far the value axis's tick marks reach out of the plot, points (0 when the axis isn't shown).
/// The category labels (items from `cats` on) moved down together, by as little as needed, off
/// any value tick label (items `vals..cats`) one of them would still cover.
fn clear_category_labels(out: &mut [GraphicItem], vals: usize, cats: usize) {
    let boxes = |items: &[GraphicItem]| -> Vec<[f32; 4]> {
        items.iter().filter_map(|it| if let GraphicItem::Text { rect, .. } = it { Some(*rect) } else { None }).collect()
    };
    let (Some(v), Some(k)) = (out.get(vals..cats), out.get(cats..)) else { return };
    let (ticks, labels) = (boxes(v), boxes(k));
    let hit = |a: &[f32; 4], b: &[f32; 4]| a[0] < b[0] + b[2] && b[0] < a[0] + a[2] && a[1] < b[1] + b[3] && b[1] < a[1] + a[3];
    let dy = labels.iter().flat_map(|l| ticks.iter().filter(move |t| hit(l, t)).map(move |t| t[1] + t[3] - l[1])).fold(0.0f32, f32::max);
    if dy > 0.0 && dy.is_finite() {
        for it in out.iter_mut().skip(cats) {
            if let GraphicItem::Text { rect, .. } = it {
                rect[1] += dy;
            }
        }
    }
}

/// How far below the category axis line the category labels start: clear of the value axis's
/// lowest tick label, which is centred on that line (half its height and 3 pt), and at least 6 pt.
fn cat_gap(c: &Chart) -> f32 {
    (c.val_fs() * LINE / 2.0 + 3.0).max(6.0).max(cat_tick_out(c) + GAP / 2.0)
}

/// How far the category axis's tick marks reach out of the plot, points.
fn cat_tick_out(c: &Chart) -> f32 {
    if c.cat.deleted { 0.0 } else { c.cat.ticks.reach(TICK).0 }
}

/// Most of the chart's height slanted category labels take.
const SLANT_SHARE: f32 = 0.4;

/// Whether the category labels are drawn slanted: a column, line or area chart (labels under the
/// plot, one per category) whose category axis turns its text.
fn slants(c: &Chart) -> bool {
    c.cat.slant > 0.0 && !c.horizontal && matches!(c.kind, Kind::Bar | Kind::Line | Kind::Area)
}

fn slant_sin(c: &Chart) -> f32 {
    c.cat.slant.to_radians().sin()
}

fn slant_cos(c: &Chart) -> f32 {
    c.cat.slant.to_radians().cos()
}

fn val_tick_out(c: &Chart) -> f32 {
    if c.val.deleted { 0.0 } else { c.val.ticks.reach(TICK).0 }
}

/// The category axis line, the plot area's outline and the value axis line with its major tick
/// marks at `ticks`, in the colours and widths the file gives them.
fn draw_axes(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, ticks: &[(f64, String)], xticks: &[(f64, String)]) {
    let plot = g.plot;
    let pen = |a: &Axis| (a.line_color.unwrap_or(AXIS), a.line_w.unwrap_or(0.75));
    if !c.cat.deleted && c.cat.line {
        let segs = if g.horiz { seg(plot.x, plot.y, plot.x, plot.bottom()) } else { seg(plot.x, plot.bottom(), plot.right(), plot.bottom()) };
        let (color, w) = pen(&c.cat);
        stroke(out, segs, color, w);
    }
    // Category tick marks: between the bands, or on the points (`midCat`); a scatter chart's x
    // axis at its ticks. Out of the plot is below a horizontal axis, left of a vertical one.
    let (reach_out, reach_in) = if c.cat.deleted { (0.0, 0.0) } else { c.cat.ticks.reach(TICK) };
    if reach_out + reach_in > 0.0 {
        let at: Vec<f32> = if xy(c) {
            xticks.iter().map(|(t, _)| g.scat_x(*t)).collect()
        } else if g.mid && !g.horiz {
            (0..g.n).map(|i| g.cat_x(i)).collect()
        } else {
            (0..=g.n).map(|i| if g.horiz { plot.bottom() - i as f32 * g.band() } else { plot.x + i as f32 * g.band() }).collect()
        };
        let mut segs = Vec::with_capacity(2 * at.len());
        for p in at {
            let (x0, y0, x1, y1) =
                if g.horiz { (plot.x - reach_out, p, plot.x + reach_in, p) } else { (p, plot.bottom() - reach_in, p, plot.bottom() + reach_out) };
            segs.push(PathSeg::Move(x0, y0));
            segs.push(PathSeg::Line(x1, y1));
        }
        let (color, w) = pen(&c.cat);
        stroke(out, segs, color, w);
    }
    if let Some(color) = c.plot_border {
        stroke(out, rect_segs(plot.x, plot.y, plot.right(), plot.bottom()), color, 0.75);
    }
    if c.val.deleted {
        return;
    }
    let (color, w) = pen(&c.val);
    if c.val.line {
        let segs = if g.horiz { seg(plot.x, plot.bottom(), plot.right(), plot.bottom()) } else { seg(plot.x, plot.y, plot.x, plot.bottom()) };
        stroke(out, segs, color, w);
    }
    // Tick marks across the value axis: out of the plot is left of a vertical axis, below a
    // horizontal one. Drawn even when the axis line isn't, as the file asks for them.
    let (reach_out, reach_in) = c.val.ticks.reach(TICK);
    if reach_out + reach_in > 0.0 {
        let mut segs = Vec::with_capacity(2 * ticks.len());
        for (t, _) in ticks {
            let p = g.val(*t);
            let (x0, y0, x1, y1) =
                if g.horiz { (p, plot.bottom() - reach_in, p, plot.bottom() + reach_out) } else { (plot.x - reach_out, p, plot.x + reach_in, p) };
            segs.push(PathSeg::Move(x0, y0));
            segs.push(PathSeg::Line(x1, y1));
        }
        stroke(out, segs, color, w);
    }
}

/// Value tick labels: under the plot for bars, else right-aligned in `margin` (the area's left
/// margin), beyond the tick marks.
fn draw_value_labels(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, ticks: &[(f64, String)], margin: Rect) {
    let (fs, lh) = (c.val_fs(), c.val_fs() * LINE);
    let tick_out = val_tick_out(c);
    let color = c.val.text_color.unwrap_or(TEXT);
    for (t, s) in ticks {
        let p = g.val(*t);
        if g.horiz {
            text_at(out, p, g.plot.bottom() + GAP / 2.0 + tick_out, s, fs, color);
        } else {
            text_in(
                out,
                Rect { x: margin.x, y: p - lh / 2.0, w: (margin.w - GAP - tick_out).max(0.0), h: lh },
                s,
                fs,
                TextAlign::Right,
                false,
                color,
            );
        }
    }
}

/// Category labels: scatter x ticks under the plot, bar labels in the left margin, column
/// labels under their bands (every `m.skip`-th).
fn draw_category_labels(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, xticks: &[(f64, String)], m: &Margins, area: Rect) {
    let (fs, lh) = (c.cat_fs(), c.cat_fs() * LINE);
    let color = c.cat.text_color.unwrap_or(TEXT);
    let plot = g.plot;
    if slants(c) {
        // Each label slanted up to the right, its end under its band's centre (its point with
        // `midCat`), its top at the category gap.
        let band = g.band();
        let (sin, cos) = (slant_sin(c), slant_cos(c));
        for (i, lines) in m.wrapped.iter().enumerate().filter(|(i, _)| i % m.skip == 0) {
            let Some(s) = lines.first() else { continue };
            let cx = if c.kind == Kind::Bar { g.band_start(i) + band / 2.0 } else { g.cat_x(i) };
            let w = text_w(s, fs) + 4.0;
            // The centre of the turned box: its right end at `cx`, its highest corner at the gap.
            let (mx, my) = (cx - w * cos / 2.0, plot.bottom() + cat_gap(c) + (w * sin + lh * cos) / 2.0);
            out.push(GraphicItem::Text {
                rect: [mx - w / 2.0, my - lh / 2.0, w, lh],
                text: s.clone(),
                size: fs,
                color,
                bold: false,
                align: TextAlign::Right,
                font: None,
                rotation: -c.cat.slant,
            });
        }
    } else if xy(c) {
        for (t, s) in xticks {
            text_at(out, g.scat_x(*t), plot.bottom() + cat_gap(c), s, fs, color);
        }
    } else if g.horiz {
        for (i, lines) in m.wrapped.iter().enumerate() {
            let top = g.band_start(i) + g.band() / 2.0 - lines.len() as f32 * lh / 2.0;
            for (k, s) in lines.iter().enumerate() {
                text_in(out, Rect { x: area.x, y: top + k as f32 * lh, w: m.left - GAP, h: lh }, s, fs, TextAlign::Right, false, color);
            }
        }
    } else {
        let band = g.band();
        for (i, lines) in m.wrapped.iter().enumerate().filter(|(i, _)| i % m.skip == 0) {
            // Under its point (on the tick mark with `midCat`); a column always sits in its band.
            let cx = if c.kind == Kind::Bar { g.band_start(i) + band / 2.0 } else { g.cat_x(i) };
            for (k, s) in lines.iter().enumerate() {
                let y = plot.bottom() + cat_gap(c) + k as f32 * lh;
                text_in(out, Rect { x: cx - band / 2.0, y, w: band, h: lh }, s, fs, TextAlign::Center, false, color);
            }
        }
    }
}

/// Whether data `lo..hi` is a narrow band away from zero: its span is under a sixth of the end
/// farther from zero (the mirror image for negative data).
pub(super) fn narrow(lo: f64, hi: f64) -> bool {
    if lo > 0.0 {
        (hi - lo) / hi < 1.0 / 6.0
    } else if hi < 0.0 {
        (hi - lo) / lo.abs() < 1.0 / 6.0
    } else {
        false
    }
}

/// Lowest and highest value of the plot; stacked series add up per category.
fn extent(c: &Chart) -> (f64, f64) {
    let mut pts: Vec<f64> = Vec::new();
    if c.stacked() {
        for i in 0..c.n {
            let (mut p, mut q) = (0.0, 0.0);
            for s in &c.series {
                match s.values.get(i).copied().flatten() {
                    Some(v) if v >= 0.0 => p += v,
                    Some(v) => q += v,
                    None => {}
                }
            }
            pts.push(p);
            pts.push(q);
        }
    } else {
        pts.extend(c.series.iter().flat_map(|s| s.values.iter().flatten().copied()));
    }
    let lo = pts.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = pts.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo.is_finite() && hi.is_finite() { (lo, hi) } else { (0.0, 0.0) }
}

/// Range of scatter x values (index + 1 where missing).
fn x_extent(c: &Chart) -> (f64, f64) {
    let xs: Vec<f64> = c.series.iter().flat_map(|s| s.xs.iter().flatten().copied()).collect();
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if lo.is_finite() && hi.is_finite() { (lo, hi) } else { (0.0, 1.0) }
}

fn bars(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, labels: &mut Vec<DataLabel>) {
    let stacked = c.stacked();
    let helper = c.series.iter().any(|s| s.hidden);
    let slots = if stacked { 1 } else { c.series.len().max(1) };
    let ov = if stacked { 0.0 } else { c.overlap };
    let ns = slots as f32;
    let band = g.band();
    let thick = band / (ns - (ns - 1.0) * ov + c.gap);
    let group = thick * (ns - (ns - 1.0) * ov);
    let mut pos = vec![0.0f64; g.n];
    let mut neg = vec![0.0f64; g.n];
    // Stacked bars round only the outer end of each stack: the last series with a positive value
    // (the top of the positive stack) and the last with a negative one, per category.
    let outer = |i: usize, positive: bool| {
        c.series.iter().enumerate().filter(|(_, s)| has_values(s)).rev().find_map(|(j, s)| match s.values.get(i).copied().flatten() {
            Some(v) if (positive && v > 0.0) || (!positive && v < 0.0) => Some(j),
            _ => None,
        })
    };
    let (outer_pos, outer_neg): (Vec<Option<usize>>, Vec<Option<usize>>) = if stacked && c.series.iter().any(|s| s.rounded) {
        (0..g.n).map(|i| (outer(i, true), outer(i, false))).unzip()
    } else {
        (Vec::new(), Vec::new())
    };
    for (j, s) in c.series.iter().enumerate().filter(|(_, s)| has_values(s)) {
        let slot = if stacked { 0 } else { j };
        let slot = (if g.horiz { slots.saturating_sub(1 + slot) } else { slot }) as f32;
        for i in 0..g.n {
            let Some(v) = s.values.get(i).copied().flatten() else { continue };
            let (v0, v1) = if !stacked {
                (0.0, v)
            } else if v >= 0.0 {
                let Some(p) = pos.get_mut(i) else { continue };
                let b = *p;
                *p += v;
                (b, *p)
            } else {
                let Some(q) = neg.get_mut(i) else { continue };
                let b = *q;
                *q += v;
                (b, *q)
            };
            let a = g.band_start(i) + (band - group) / 2.0 + slot * thick * (1.0 - ov);
            let (p0, p1) = (g.val(v0), g.val(v1));
            let (p0, p1) = (p0.min(p1), p0.max(p1));
            let (x0, y0, x1, y1) = if g.horiz { (p0, a, p1, a + thick) } else { (a, p0, a + thick, p1) };
            // Rounded bars: the value end's corners, a quarter of the thickness and at most 4 pt;
            // the axis side stays square. In a stack only the outermost segment is rounded.
            let round_here = s.rounded
                && (!stacked
                    || if v >= 0.0 { outer_pos.get(i).copied().flatten() == Some(j) } else { outer_neg.get(i).copied().flatten() == Some(j) });
            let segs = if round_here {
                let end = match (g.horiz, v >= 0.0) {
                    (false, true) => End::Top,
                    (false, false) => End::Bottom,
                    (true, true) => End::Right,
                    (true, false) => End::Left,
                };
                round_end_segs(x0, y0, x1, y1, (thick * 0.25).min(4.0), end)
            } else {
                rect_segs(x0, y0, x1, y1)
            };
            // A helper series (a waterfall's base) only lifts the bars on it.
            if s.hidden {
                continue;
            }
            fill(out, segs, c.point_colour(j, i));
            if let Some(text) = label_of(s, i) {
                // Bars floating on a helper series (a waterfall's steps, a funnel's stages) are
                // one bar each: centred when the label fits, else past the bar's end (a stacked
                // segment's label is left out instead).
                let floating = stacked && helper;
                let spot = BarSpot {
                    bar: Rect { x: x0, y: y0, w: x1 - x0, h: y1 - y0 },
                    horiz: g.horiz,
                    positive: v >= 0.0,
                    stacked: stacked && !floating,
                    fill: c.point_colour(j, i),
                };
                let pos = if floating { s.label_pos.or(Some(LabelPos::Centre)) } else { s.label_pos };
                labels.extend(bar_label(text, &spot, c.label_dark(), pos, c.data_fs()));
            }
        }
    }
}

/// Whether a series has any point to draw.
pub(super) fn has_values(s: &Series) -> bool {
    s.values.iter().any(Option::is_some)
}

/// Line series; stacked ones are drawn at the running total of the series up to them.
fn lines(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, labels: &mut Vec<DataLabel>) {
    let mut total = vec![0.0f64; g.n];
    for (j, s) in c.series.iter().enumerate().filter(|(_, s)| has_values(s)) {
        let pts: Vec<Option<(f32, f32)>> = (0..g.n)
            .map(|i| {
                let v = s.values.get(i).copied().flatten()?;
                let y = match total.get_mut(i) {
                    Some(t) if c.stacked() => {
                        *t += v;
                        *t
                    }
                    _ => v,
                };
                Some((g.cat_x(i), g.val(y)))
            })
            .collect();
        for (i, p) in pts.iter().enumerate() {
            if let (Some((x, y)), Some(text)) = (p, label_of(s, i)) {
                labels.push(DataLabel::Point { x: *x, y: *y, text, cat: i, pos: s.label_pos, series: Some(c.series_colour(j)) });
            }
        }
        stroke(out, path_of(s, &pts), c.series_colour(j), s.line_w);
        if let Some(m) = s.marker {
            markers(out, &pts, m, c.series_colour(j));
        }
    }
}

/// A series' line through `pts`: straight pieces, or a smooth curve when the series asks.
fn path_of(s: &Series, pts: &[Option<(f32, f32)>]) -> Vec<PathSeg> {
    if s.smooth { curves(pts.iter().copied()) } else { runs(pts.iter().copied()) }
}

/// A marker on each point: the marker's own fill, else `color`.
fn markers(out: &mut Vec<GraphicItem>, pts: &[Option<(f32, f32)>], m: Marker, color: Rgb) {
    let kind = if m.square { ShapeKind::Rectangle } else { ShapeKind::Ellipse };
    let d = m.size;
    for (x, y) in pts.iter().flatten() {
        let rect = [x - d / 2.0, y - d / 2.0, d, d];
        out.push(GraphicItem::Shape { rect, kind, fill: Some(m.fill.unwrap_or(color)), stroke: None, stroke_width: 0.0 });
    }
}

/// Area series, lower ones first: each filled (with its gradient across the plot when it has
/// one), then the top edges of those with an outline, over all the fills.
fn areas(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, labels: &mut Vec<DataLabel>) {
    let mut base = vec![0.0f64; g.n];
    let mut edges = Vec::new();
    let mut dots = Vec::new();
    for (j, s) in c.series.iter().enumerate().filter(|(_, s)| has_values(s)) {
        let vals = (0..g.n).map(|i| s.values.get(i).copied().flatten().unwrap_or(0.0));
        let top: Vec<f64> = base.iter().zip(vals).map(|(b, v)| b + v).collect();
        let top_pts: Vec<Option<(f32, f32)>> = top.iter().enumerate().map(|(i, t)| Some((g.cat_x(i), g.val(*t)))).collect();
        // The top edge (a smooth curve when the series asks), then back along the base: the
        // series below's top edge, drawn the same way, reversed.
        let line = path_of(s, &top_pts);
        let mut segs = line.clone();
        let base_pts = base.iter().enumerate().rev().map(|(i, b)| Some((g.cat_x(i), g.val(*b))));
        let back = if s.smooth && c.stacked() { curves(base_pts) } else { runs(base_pts) };
        segs.extend(back.into_iter().map(|p| match p {
            PathSeg::Move(x, y) => PathSeg::Line(x, y),
            other => other,
        }));
        if let Some((color, w)) = s.edge {
            edges.push((line, color, w));
        }
        if let Some(m) = s.marker {
            dots.push((top_pts, m, c.series_colour(j)));
        }
        segs.push(PathSeg::Close);
        let color = c.series_colour(j);
        match s.gradient.as_deref().and_then(|gr| plot_gradient(gr, g.plot)) {
            // Where a gradient can't be drawn, the plain fill is the colour half way to white.
            Some(gradient) => {
                out.push(GraphicItem::Path { segs, fill: Some(half_tint(color)), stroke: None, stroke_width: 0.0, gradient: Some(gradient) })
            }
            None => fill(out, segs, color),
        }
        for (i, t) in top.iter().enumerate() {
            if let Some(text) = label_of(s, i) {
                labels.push(DataLabel::Point { x: g.cat_x(i), y: g.val(*t), text, cat: i, pos: s.label_pos, series: Some(color) });
            }
        }
        if c.stacked() {
            base = top;
        }
    }
    for (line, color, w) in edges {
        stroke(out, line, color, w);
    }
    for (pts, m, color) in dots {
        markers(out, &pts, m, color);
    }
}

/// `c` blended half way to white: a gradient's stand-in where only a plain fill is drawn.
fn half_tint(c: Rgb) -> Rgb {
    tint(c, 0.5)
}

/// `c` at opacity `a` over white.
pub(super) fn tint(c: Rgb, a: f32) -> Rgb {
    let a = if a.is_finite() { a.clamp(0.0, 1.0) } else { 1.0 };
    let mix = |v: u8| (f32::from(v) * a + 255.0 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
    Rgb(mix(c.0), mix(c.1), mix(c.2))
}

/// The plot area's background: `rect` filled with `color` at opacity `alpha`. A translucent fill
/// is an even gradient (one colour, two stops at that opacity), the item model's way to carry
/// opacity; its plain fill is the colour at that opacity over white.
fn shade(out: &mut Vec<GraphicItem>, rect: Rect, color: Rgb, alpha: f32) {
    use wordcraft_doc::graphic::{GradientStop, LinearGradient};
    let segs = rect_segs(rect.x, rect.y, rect.right(), rect.bottom());
    if alpha >= 1.0 || alpha.is_nan() {
        fill(out, segs, color);
        return;
    }
    let stop = |offset| GradientStop { offset, color, alpha: alpha.max(0.0) };
    let gradient = LinearGradient { from: [rect.x, rect.y], to: [rect.x, rect.bottom()], stops: vec![stop(0.0), stop(1.0)], radial: false };
    out.push(GraphicItem::Path { segs, fill: Some(tint(color, alpha)), stroke: None, stroke_width: 0.0, gradient: Some(gradient) });
}

/// A series' linear gradient laid across the plot rectangle `plot` (in the graphic's points):
/// along its angle (0 = left to right, 90 = top to bottom; `scaled`, the angle of the plot
/// squashed to a square), through the plot's centre, spanning the plot; stops at their places
/// (0..1) with their opacities. So every area fades the same way down to the plot's bottom.
fn plot_gradient(gr: &wordcraft_doc::fill::Gradient, plot: Rect) -> Option<wordcraft_doc::graphic::LinearGradient> {
    use wordcraft_doc::fill::GradientKind;
    use wordcraft_doc::graphic::{GradientStop, LinearGradient};
    let GradientKind::Linear { angle, scaled } = gr.kind else { return None };
    let (s, c) = angle.to_radians().sin_cos();
    let (dx, dy) = if scaled && plot.w > 0.0 && plot.h > 0.0 { (c / plot.w, s / plot.h) } else { (c, s) };
    let len = dx.hypot(dy);
    let (dx, dy) = if len > 0.0 && len.is_finite() { (dx / len, dy / len) } else { (1.0, 0.0) };
    let half = (plot.w * dx.abs() + plot.h * dy.abs()) / 2.0;
    let (cx, cy) = (plot.x + plot.w / 2.0, plot.y + plot.h / 2.0);
    let stops: Vec<GradientStop> = gr
        .stops
        .iter()
        .map(|st| GradientStop { offset: (st.pos / 100.0).clamp(0.0, 1.0), color: st.color, alpha: (1.0 - st.transparency / 100.0).clamp(0.0, 1.0) })
        .collect();
    (half > 0.0 && half.is_finite() && !stops.is_empty()).then_some(LinearGradient {
        from: [cx - dx * half, cy - dy * half],
        to: [cx + dx * half, cy + dy * half],
        stops,
        radial: false,
    })
}

/// Scatter series as lines, or as small markers when the series has no line.
fn draw_scatter(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, labels: &mut Vec<DataLabel>) {
    const MARKER: f32 = 6.0;
    for (j, s) in c.series.iter().enumerate() {
        let pts: Vec<Option<(f32, f32)>> =
            s.xs.iter()
                .zip(&s.values)
                .map(|(x, y)| match (x, y) {
                    (Some(x), Some(y)) => Some((g.scat_x(*x), g.val(*y))),
                    _ => None,
                })
                .collect();
        for (i, p) in pts.iter().enumerate() {
            if let (Some((x, y)), Some(text)) = (p, label_of(s, i)) {
                labels.push(DataLabel::Point { x: *x, y: *y - MARKER / 2.0, text, cat: i, pos: s.label_pos, series: None });
            }
        }
        if s.line {
            stroke(out, path_of(s, &pts), c.series_colour(j), s.line_w);
            if let Some(m) = s.marker {
                markers(out, &pts, m, c.series_colour(j));
            }
            continue;
        }
        let m = s.marker.unwrap_or(Marker { size: MARKER, square: false, fill: None });
        markers(out, &pts, m, c.series_colour(j));
    }
}

/// Whether the chart's horizontal axis is a value axis (scatter and bubble charts).
fn xy(c: &Chart) -> bool {
    matches!(c.kind, Kind::Scatter | Kind::Bubble)
}

/// A combo chart's groups on one value axis (`secondary`), each as a chart of its own series:
/// its areas (stacked on each other), its columns (clustered) and its lines.
fn combo_views(c: &Chart, secondary: bool) -> Vec<Chart> {
    [(Kind::Area, Grouping::Stacked), (Kind::Bar, Grouping::Clustered), (Kind::Line, Grouping::Clustered)]
        .into_iter()
        .filter_map(|(kind, grouping)| {
            let series: Vec<Series> = c.series.iter().filter(|s| s.group == kind && s.secondary == secondary).cloned().collect();
            (!series.is_empty()).then(|| Chart { kind, grouping, series, combo: false, ..c.clone() })
        })
        .collect()
}

/// The value scale and tick texts of a combo chart's axis `ax` over its groups `views`.
fn combo_scale(views: &[Chart], ax: &Axis) -> (Scale, Vec<(f64, String)>) {
    let (lo, hi) = views.iter().map(extent).fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), (l, h)| (a.min(l), b.max(h)));
    let (lo, hi) = if lo.is_finite() && hi.is_finite() { (lo, hi) } else { (0.0, 0.0) };
    let vs = scale(lo, hi, true, ax.min, ax.max, ax.step);
    let fmt =
        ax.fmt.clone().or_else(|| views.iter().flat_map(|v| v.series.iter()).find_map(|s| s.fmt.clone())).unwrap_or_else(|| "General".to_string());
    let ticks = vs.ticks().into_iter().map(|t| (t, format_value(t, &fmt, vs.step))).collect();
    (vs, ticks)
}

/// A combo chart's secondary value axis at the right of the plot: its line (when shown) and its
/// tick labels, left-aligned beyond it.
fn right_value_axis(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, ticks: &[(f64, String)], ax: &Axis, area: Rect) {
    let plot = g.plot;
    if ax.deleted {
        return;
    }
    if ax.line {
        stroke(out, seg(plot.right(), plot.y, plot.right(), plot.bottom()), ax.line_color.unwrap_or(AXIS), ax.line_w.unwrap_or(0.75));
    }
    if !ax.labels {
        return;
    }
    let (fs, lh) = (c.val_fs(), c.val_fs() * LINE);
    let w = (area.right() - plot.right() - GAP).max(0.0);
    let from = out.len();
    for (t, s) in ticks {
        let p = g.val(*t);
        text_in(out, Rect { x: plot.right() + GAP, y: p - lh / 2.0, w, h: lh }, s, fs, TextAlign::Left, false, ax.text_color.unwrap_or(TEXT));
    }
    restyle(out.get_mut(from..).unwrap_or_default(), &c.val.look);
}

/// The bounding box of a path's points (control points included); `None` for no points.
fn seg_box(segs: &[PathSeg]) -> Option<Rect> {
    let mut pts = segs.iter().flat_map(|s| match *s {
        PathSeg::Move(x, y) | PathSeg::Line(x, y) => vec![(x, y)],
        PathSeg::Cubic(a, b, c, d, e, f) => vec![(a, b), (c, d), (e, f)],
        PathSeg::Close => vec![],
    });
    let (x0, y0) = pts.next()?;
    let (mut lo, mut hi) = ((x0, y0), (x0, y0));
    for (x, y) in pts {
        lo = (lo.0.min(x), lo.1.min(y));
        hi = (hi.0.max(x), hi.1.max(y));
    }
    Some(Rect { x: lo.0, y: lo.1, w: hi.0 - lo.0, h: hi.1 - lo.1 })
}

/// A see-through bubble's outline, points (1 px at 96 dpi).
const BUBBLE_EDGE: f32 = 0.75;

/// Bubble series: a circle at each point, its area in proportion to the point's size; the
/// biggest bubble's diameter a quarter of the plot's shorter side (the default scale).
fn bubbles(out: &mut Vec<GraphicItem>, c: &Chart, g: &Frame, labels: &mut Vec<DataLabel>) {
    let biggest = c.series.iter().flat_map(|s| s.sizes.iter().flatten()).copied().filter(|v| v.is_finite() && *v > 0.0).fold(0.0f64, f64::max);
    if biggest <= 0.0 {
        return;
    }
    let rmax = 0.125 * g.plot.w.min(g.plot.h);
    for (j, s) in c.series.iter().enumerate().filter(|(_, s)| !s.hidden) {
        for (i, (x, y)) in s.xs.iter().zip(&s.values).enumerate() {
            let (Some(x), Some(y)) = (x, y) else { continue };
            let Some(size) = s.sizes.get(i).copied().flatten().filter(|v| v.is_finite() && *v > 0.0) else { continue };
            let r = rmax * (size / biggest).sqrt() as f32;
            let (px, py) = (g.scat_x(*x), g.val(*y));
            let colour = c.point_colour(j, i);
            if s.fill_alpha < 1.0 {
                // See-through: the fill at its opacity (an even gradient carries it, the plain
                // fill is the colour over white), its outline solid, so overlaps show.
                use wordcraft_doc::graphic::{GradientStop, LinearGradient};
                let stop = |offset| GradientStop { offset, color: colour, alpha: s.fill_alpha.max(0.0) };
                let gradient = LinearGradient { from: [px, py - r], to: [px, py + r], stops: vec![stop(0.0), stop(1.0)], radial: false };
                out.push(GraphicItem::Path {
                    segs: circle_segs(px, py, r),
                    fill: Some(tint(colour, s.fill_alpha)),
                    stroke: Some(colour),
                    stroke_width: BUBBLE_EDGE,
                    gradient: Some(gradient),
                });
            } else {
                out.push(GraphicItem::Shape {
                    rect: [px - r, py - r, 2.0 * r, 2.0 * r],
                    kind: ShapeKind::Ellipse,
                    fill: Some(colour),
                    stroke: None,
                    stroke_width: 0.0,
                });
            }
            if let Some(text) = label_of(s, i) {
                // Inside the bubble when it fits, white or dark by what shows there (the fill at its
                // opacity over white); else above it.
                let (w, h) = label_size(&text, c.data_fs());
                if w <= 2.0 * r && h <= 2.0 * r {
                    let shown = if s.fill_alpha < 1.0 { tint(colour, s.fill_alpha) } else { colour };
                    let rect = Rect { x: px - w / 2.0, y: py - h / 2.0, w, h };
                    labels.push(DataLabel::Placed { rect, text, color: on_fill(shown, c.label_dark()), align: TextAlign::Center });
                } else {
                    labels.push(DataLabel::Point { x: px, y: py, text, cat: i, pos: s.label_pos, series: None });
                }
            }
        }
    }
}
