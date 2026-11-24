//! Charts (ECMA-376 Part 1 §21.2): the cached data of a `c:chartSpace` drawn as items in points.
//!
//! Bar and column (clustered, stacked or 100% stacked), line, area, pie, doughnut and scatter
//! charts are drawn from the first supported chart type of the plot area, with their values as
//! data labels where `c:dLbls` shows them (`c:showVal`). Radar charts (standard, with markers or
//! filled) draw as spokes, concentric grid polygons and one polygon per series. A chart of only other types (stock, bubble…)
//! draws a placeholder: its frame, its title and the type it is. Text widths are estimated ([`CHAR_W`] em a character), so label places are approximate.

use std::f64::consts::{FRAC_PI_2, TAU};

use wordcraft_doc::graphic::{GraphicItem, PathSeg, TextAlign};
use wordcraft_doc::para::ShapeKind;
use wordcraft_doc::props::Rgb;

use super::drawing_color::{color_alpha, fill as dml_fill, fill_colour, slot};
use crate::units::{int, measure};
use crate::xml::El;

mod canvas;
mod format;
mod labels;
mod model;
mod parse;
mod plot;
mod polar;

#[cfg(test)]
use canvas::SWATCH;
pub(super) use canvas::text_w;
use canvas::{Rect, clip_text, legend, text, text_in};
#[cfg(test)]
use format::{civil, excel_stamp, format_value};
use model::{Axis, Grouping, Kind, Look, Pos, Series};
#[cfg(test)]
use parse::{axis, category_text, labels, numbers};
use parse::{read_series, text_size, title_bold, title_colour, title_of};
#[cfg(test)]
use plot::{Scale, narrow, nice_scale, scale};
use plot::{backdrop, cartesian, plot_frame};
use polar::{of_pie, pie, pie_square, radar};

/// Most series read from one chart.
const MAX_SERIES: usize = 256;
/// Most points read from one series.
const MAX_POINTS: usize = 10_000;
/// Most points read from one chart, all series together.
const MAX_CELLS: usize = 100_000;
/// Longest series name or category label kept.
const MAX_LABEL: usize = 200;
/// Most categories (spokes) of a radar chart.
const MAX_SPOKES: usize = 100;
/// Most series of a radar chart.
const MAX_RADAR_SERIES: usize = 50;
/// Most axis ticks drawn.
const MAX_TICKS: usize = 20;
/// Default text size, points.
const FONT: f32 = 9.0;
/// Average character width estimate, em (charts and diagrams measure text before shaping).
pub(super) const CHAR_W: f32 = 0.55;
/// Widest share of the chart a legend entry's text takes.
const LEGEND_SHARE: f32 = 0.4;
/// Most rows of a legend above or below the plot.
const MAX_LEGEND_ROWS: usize = 3;
const TITLE_SIZE: f32 = 14.0;
/// Space between the frame and the content, points.
const PAD: f32 = 6.0;
/// Line height as a multiple of the font size.
const LINE: f32 = 1.3;
/// Gap between a label and its axis, points.
const GAP: f32 = 4.0;
/// Default width of a line series, points (Word's 2.25 pt).
const LINE_W: f32 = 2.25;
const TEXT: Rgb = Rgb(0x59, 0x59, 0x59);
const GRID: Rgb = Rgb(0xD9, 0xD9, 0xD9);
const AXIS: Rgb = Rgb(0xBF, 0xBF, 0xBF);
/// Data labels off the fill: the dark text colour.
const LABEL_DARK: Rgb = Rgb(0x1F, 0x1F, 0x1F);

/// Items for a chart part, sized `w` x `h` points: a placeholder when only unsupported chart
/// types are in it, empty when there's nothing to draw.
pub(crate) fn chart_items(space: &El, theme: &[Rgb], w: f32, h: f32) -> Vec<GraphicItem> {
    let mut out = Vec::new();
    let Some((chart, area)) = frame(&mut out, space, theme, w, h) else { return out };
    match chart.kind {
        Kind::Pie | Kind::Doughnut => pie(&mut out, &chart, area),
        Kind::Radar => radar(&mut out, &chart, area),
        Kind::OfPie => of_pie(&mut out, &chart, area),
        _ => cartesian(&mut out, &chart, area),
    }
    fade(&mut out, chart.opacity);
    out
}

/// The plot area of a chart part drawn `w` x `h` points, laid out as [`chart_items`] lays it
/// out: the rectangle between the axes of a bar, column, line, area or scatter chart, the
/// bounding square of a pie or doughnut. `None` for a radar chart, a placeholder or nothing.
pub(crate) fn chart_plot_rect(space: &El, theme: &[Rgb], w: f32, h: f32) -> Option<[f32; 4]> {
    let (chart, area) = frame(&mut Vec::new(), space, theme, w, h)?;
    let r = match chart.kind {
        Kind::Pie | Kind::Doughnut => pie_square(area)?,
        Kind::Radar | Kind::OfPie => return None,
        _ => plot_frame(&chart, area)?.plot,
    };
    Some(r.arr())
}

/// The chart part read, with its backdrop, title and legend drawn into `out`; returns the chart
/// and the room left for its plot. `None` (after drawing a placeholder when it calls for one)
/// when there's no chart this module draws, or the size isn't a positive finite number.
fn frame(out: &mut Vec<GraphicItem>, space: &El, theme: &[Rgb], w: f32, h: f32) -> Option<(Chart, Rect)> {
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return None;
    }
    let Some(chart) = Chart::parse(space, theme) else {
        placeholder(out, space, theme, w, h);
        return None;
    };
    backdrop(out, space, theme, w, h);
    let mut area = Rect { x: PAD, y: PAD, w: (w - 2.0 * PAD).max(1.0), h: (h - 2.0 * PAD).max(1.0) };
    if let Some(t) = &chart.title {
        let from = out.len();
        area = title_block(out, t, area, chart.title_fs(), chart.title_color.unwrap_or(TEXT), chart.title_bold);
        restyle(out.get_mut(from..).unwrap_or_default(), &Look { color: None, bold: None, ..chart.title_look.clone() });
    }
    if let Some(pos) = chart.legend {
        // Word lists bar series in the order they are drawn: reversed for clustered horizontal bars
        // and for stacked columns with a vertical legend.
        let vertical = matches!(pos, Pos::Left | Pos::Right);
        let reverse = chart.kind == Kind::Bar && if chart.horizontal { !chart.stacked() } else { chart.stacked() && vertical };
        let mut entries = chart.legend_entries();
        if reverse {
            entries.reverse();
        }
        let from = out.len();
        area = legend(out, &entries, pos, area, chart.legend_fs(), chart.round_keys, chart.legend_color.unwrap_or(TEXT));
        restyle(out.get_mut(from..).unwrap_or_default(), &Look { color: None, ..chart.legend_look.clone() });
    }
    Some((chart, area))
}

/// The title at the top of `area`; returns the room left under it.
fn title_block(out: &mut Vec<GraphicItem>, t: &str, area: Rect, size: f32, color: Rgb, bold: bool) -> Rect {
    let lh = size * LINE;
    text_in(out, Rect { x: area.x, y: area.y, w: area.w, h: lh }, t, size, TextAlign::Center, bold, color);
    Rect { y: area.y + lh + PAD, h: (area.h - lh - PAD).max(1.0), ..area }
}

/// A chart whose plot area holds only chart types WordCraft doesn't draw (stock, bubble…): its
/// frame with a light border, its title and a line saying which type it is, so it isn't blank
/// space. Nothing when the plot area has no chart type at all, or a supported one with nothing
/// to draw.
fn placeholder(out: &mut Vec<GraphicItem>, space: &El, theme: &[Rgb], w: f32, h: f32) {
    let Some(chart) = space.child("c:chart") else { return };
    let Some(plot) = chart.child("c:plotArea") else { return };
    if plot.els().any(|e| Kind::of(&e.name).is_some()) {
        return;
    }
    let Some(ty) = plot.els().find(|e| e.name.ends_with("Chart")) else { return };
    backdrop(out, space, theme, w, h);
    out.push(GraphicItem::Shape { rect: [0.0, 0.0, w, h], kind: ShapeKind::Rectangle, fill: None, stroke: Some(AXIS), stroke_width: 0.75 });
    let mut area = Rect { x: PAD, y: PAD, w: (w - 2.0 * PAD).max(1.0), h: (h - 2.0 * PAD).max(1.0) };
    if let Some(t) = title_of(chart, &read_series(ty, false, theme)) {
        area = title_block(out, &t, area, TITLE_SIZE, title_colour(chart, theme).unwrap_or(TEXT), title_bold(chart));
    }
    let size = text_size(space);
    let lh = size * LINE;
    let msg = format!("{} (not supported yet)", unsupported_name(&ty.name));
    let line = Rect { x: area.x, y: area.y + ((area.h - lh) / 2.0).max(0.0), w: area.w, h: lh };
    text(out, line, &clip_text(&msg, area.w, size), size, TextAlign::Center, false);
}

/// What a placeholder calls a chart type element WordCraft doesn't draw.
fn unsupported_name(name: &str) -> &'static str {
    match name {
        "c:stockChart" => "Stock chart",
        "c:bubbleChart" => "Bubble chart",
        "c:surfaceChart" | "c:surface3DChart" => "Surface chart",
        "c:ofPieChart" => "Pie of pie chart",
        "c:bar3DChart" => "3-D bar chart",
        "c:line3DChart" => "3-D line chart",
        "c:area3DChart" => "3-D area chart",
        "c:pie3DChart" => "3-D pie chart",
        _ => "Chart",
    }
}

#[derive(Clone)]
struct Chart {
    kind: Kind,
    horizontal: bool,
    grouping: Grouping,
    /// Gap between groups as a fraction of a bar's width.
    gap: f32,
    /// Overlap of clustered bars, -1..1 (1 = all on top of each other).
    overlap: f32,
    /// Doughnut hole, fraction of the outer radius.
    hole: f32,
    first_angle: f64,
    /// Radar: series polygons are filled (`c:radarStyle` = `filled`).
    radar_fill: bool,
    /// Radar: points carry markers (`c:radarStyle` = `marker`).
    radar_marks: bool,
    vary: bool,
    /// Number of categories (scatter: points per series at most).
    n: usize,
    cats: Vec<String>,
    series: Vec<Series>,
    accents: Vec<Rgb>,
    title: Option<String>,
    legend: Option<Pos>,
    /// Category axis (scatter: the x axis).
    cat: Axis,
    val: Axis,
    text_size: f32,
    val_fmt: String,
    /// Serial days count from 1904 (`c:date1904`), not 1900.
    date1904: bool,
    /// Outline of the plot area, when the file draws one.
    plot_border: Option<Rgb>,
    /// Fill of the plot area and its opacity (0..1), when the file gives one.
    plot_fill: Option<(Rgb, f32)>,
    /// Legend keys are circles (WordCraft's chart extension), not squares.
    round_keys: bool,
    /// Colours of the legend text and the title, when the file sets them.
    legend_color: Option<Rgb>,
    title_color: Option<Rgb>,
    /// The title is bold (`b` of its run, else its default run properties; bold when unset).
    title_bold: bool,
    /// Size and font of the title, the legend and the data labels, where the file sets them;
    /// the data labels' colour and weight too.
    title_look: Look,
    legend_look: Look,
    data_look: Look,
    /// Data labels have a white outline (`a:ln` in their default run properties).
    halo: bool,
    /// The whole chart's opacity, 0..1 (WordCraft's chart extension; 1 when it has none).
    opacity: f32,
    /// Columns, lines and areas together, each series in its own group (`Series::group`), some
    /// on the secondary value axis (`val2`, at the right).
    combo: bool,
    val2: Option<Axis>,
    /// Legend entries the file deletes (by series `c:idx`).
    legend_deleted: Vec<usize>,
    /// Pie of pie: how many of the last points the second plot takes, and whether it is a bar.
    split: usize,
    of_bar: bool,
}

impl Chart {
    /// Size of the legend text, points.
    fn legend_fs(&self) -> f32 {
        self.legend_look.size.unwrap_or(self.text_size)
    }
    /// Size of the title, points.
    fn title_fs(&self) -> f32 {
        self.title_look.size.unwrap_or(TITLE_SIZE)
    }
    /// Size of the data labels, points.
    fn data_fs(&self) -> f32 {
        self.data_look.size.unwrap_or(self.text_size)
    }
    /// Size of the category axis labels (a scatter chart's x axis), points.
    fn cat_fs(&self) -> f32 {
        self.cat.look.size.unwrap_or(self.text_size)
    }
    /// Size of the value axis labels, points.
    fn val_fs(&self) -> f32 {
        self.val.look.size.unwrap_or(self.text_size)
    }
}

/// The text items of `out` given `look`'s weight, colour and font where it sets them.
fn restyle(out: &mut [GraphicItem], look: &Look) {
    if look.bold.is_none() && look.color.is_none() && look.font.is_none() {
        return;
    }
    for it in out {
        if let GraphicItem::Text { bold, color, font, .. } = it {
            if let Some(b) = look.bold {
                *bold = b;
            }
            if let Some(c) = look.color {
                *color = c;
            }
            if let Some(f) = &look.font {
                *font = Some(f.clone());
            }
        }
    }
}

/// Most labels given a white outline; past that they are drawn without one (each outline is
/// eight white copies under the label).
const MAX_HALOS: usize = 2000;
/// How far the outline's copies sit from the label, points (half the outline's 2 pt width).
const HALO: f32 = 1.0;

/// The text items of `out` from `from` on with a white outline: each over eight white copies of
/// itself moved [`HALO`] around it.
fn halo(out: &mut Vec<GraphicItem>, from: usize) {
    let Some(tail) = out.get(from..) else { return };
    if tail.len() > MAX_HALOS {
        return;
    }
    let tail: Vec<GraphicItem> = out.drain(from..).collect();
    let d = HALO * std::f32::consts::FRAC_1_SQRT_2;
    let offsets = [(HALO, 0.0), (-HALO, 0.0), (0.0, HALO), (0.0, -HALO), (d, d), (d, -d), (-d, d), (-d, -d)];
    for it in tail {
        if let GraphicItem::Text { rect, .. } = &it {
            let r = *rect;
            for (dx, dy) in offsets {
                let mut copy = it.clone();
                if let GraphicItem::Text { rect, color, .. } = &mut copy {
                    *rect = [r[0] + dx, r[1] + dy, r[2], r[3]];
                    *color = Rgb::WHITE;
                }
                out.push(copy);
            }
        }
        out.push(it);
    }
}

/// Every item of `out` drawn at opacity `a` over white paper: plain colours blended toward
/// white, gradient stops' opacities scaled (a fill that already lets the page through).
fn fade(out: &mut [GraphicItem], a: f32) {
    if !(a.is_finite() && a < 1.0) {
        return;
    }
    let a = a.max(0.0);
    let mix = |c: Rgb| {
        let m = |v: u8| (f32::from(v) * a + 255.0 * (1.0 - a)).round().clamp(0.0, 255.0) as u8;
        Rgb(m(c.0), m(c.1), m(c.2))
    };
    for it in out {
        match it {
            GraphicItem::Shape { fill, stroke, .. } => {
                *fill = fill.map(mix);
                *stroke = stroke.map(mix);
            }
            GraphicItem::Path { fill, stroke, gradient, .. } => {
                *fill = fill.map(mix);
                *stroke = stroke.map(mix);
                if let Some(g) = gradient {
                    for s in &mut g.stops {
                        s.alpha *= a;
                    }
                }
            }
            GraphicItem::Text { color, .. } => *color = mix(*color),
            GraphicItem::Image { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests;
