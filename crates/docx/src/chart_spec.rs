//! Charts WordCraft makes ([`ChartSpec`]) as chart parts (ECMA-376 Part 1 §21.2) and back.
//!
//! [`chart_xml`] writes a `c:chartSpace` whose data names the cells of an embedded workbook
//! (`c:strRef`, `c:numRef`, with caches of their values) and [`chart_workbook`] writes that
//! workbook (`c:externalData`, the chart part's package relationship), so another program can
//! edit the chart's data as a sheet. Reading takes the data from the caches, never the workbook.
//! [`chart_items`] draws a chart through the same code that draws chart parts read from files.
//! [`spec_of`] turns a chart part back into the model when it is one chart type the model has,
//! with its values in the part; what the model has no place for is left out (and not written again).

use wordcraft_doc::chart::{
    CatLabelPos, ChartSeries, ChartSpec, ChartType, Gridlines, LegendPos, LegendShape, MAX_COLORS, MAX_POINTS, MAX_SERIES, SeriesKind, TextStyle,
};
use wordcraft_doc::graphic::GraphicItem;
use wordcraft_doc::props::Rgb;

use crate::xml::{self, El, W};

const NS_C: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// Content type of a chart part.
pub(crate) const CHART_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.chart+xml";
/// `a:graphicData` URI of a chart.
pub(crate) const CHART_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/chart";
/// Axis ids (any pair of distinct numbers will do; Word keeps them).
const CAT_AX: &str = "500000001";
const VAL_AX: &str = "500000002";
/// `c:ext` URI of WordCraft's chart extension: the palette a chart was given (as `a:srgbClr`s)
/// and its legend key shape (`wc:legendKey shape="circle"`).
pub(crate) const PALETTE_EXT: &str = "{5C0F2A47-9B1E-4D63-A8C2-7E4B91D30F65}";
/// Namespace of WordCraft's own elements in that extension.
const WORDCRAFT_NS: &str = "urn:wordcraft:chart";

/// Whether a chart space's WordCraft extension asks for round legend keys. Elements in a
/// namespace the reader doesn't know keep their local name behind `?:`.
pub(crate) fn round_legend_keys(space: &El) -> bool {
    space
        .child("c:extLst")
        .and_then(|l| l.children("c:ext").find(|e| e.attr("uri") == Some(PALETTE_EXT)))
        .and_then(|e| e.child("?:legendKey"))
        .and_then(|k| k.attr("shape"))
        == Some("circle")
}
/// Colour of the axis lines (and the value axis's tick marks): near black.
const AXIS_LINE: &str = "333333";
/// Colour of the chart's text: title, axis labels, legend.
const CHART_TEXT: &str = "1F1F1F";

/// `c:txPr` giving the element's text its look (its default run properties): `style`'s size,
/// weight, colour (else the chart text colour) and font; turned 45° up (`rotate`, the body's
/// `rot`); with a white outline (`outline`, a halo around data labels).
fn text_pr(w: &mut W, style: &TextStyle, rotate: bool, outline: bool) {
    w.open("c:txPr", &[]);
    if rotate {
        w.empty("a:bodyPr", &[("rot", ROTATED), ("vert", "horz")]);
    } else {
        w.empty("a:bodyPr", &[]);
    }
    w.empty("a:lstStyle", &[]);
    w.open("a:p", &[]);
    w.open("a:pPr", &[]);
    run_props(w, "a:defRPr", &[], style, outline);
    w.close("a:pPr");
    w.empty("a:endParaRPr", &[("lang", "en-US")]);
    w.close("a:p");
    w.close("c:txPr");
}

/// Run properties `name` (`a:defRPr`, `a:rPr`) with `attrs` and `style` (ECMA-376 element order:
/// line, fill, then fonts).
fn run_props(w: &mut W, name: &str, attrs: &[(&str, &str)], style: &TextStyle, outline: bool) {
    let sz = style.size.map(|s| ((s * 100.0).round() as i64).to_string());
    let mut a: Vec<(&str, &str)> = attrs.iter().filter(|(k, _)| !(*k == "b" && style.bold.is_some())).copied().collect();
    if let Some(sz) = &sz {
        a.push(("sz", sz));
    }
    if let Some(b) = style.bold {
        a.push(("b", if b { "1" } else { "0" }));
    }
    w.open(name, &a);
    if outline {
        w.open("a:ln", &[("w", HALO_W)]);
        solid(w, "FFFFFF");
        w.empty("a:round", &[]);
        w.close("a:ln");
    }
    solid(w, style.color.as_deref().unwrap_or(CHART_TEXT));
    if let Some(f) = &style.font {
        w.empty("a:latin", &[("typeface", f)]);
        w.empty("a:ea", &[("typeface", f)]);
    }
    w.close(name);
}

/// `a:bodyPr/@rot` of category labels turned 45° up (60,000ths of a degree).
const ROTATED: &str = "-2700000";
/// Width of the data labels' white outline, EMU (2 pt).
const HALO_W: &str = "25400";
/// Colour of the major gridlines.
const GRID_LINE: &str = "C8C8C8";
/// Size of the circle markers of a line chart, points (ECMA-376 `c:size`, 2–72).
const MARKER_SIZE: &str = "5";

/// The chart part for `spec`: a complete XML document, written as [`ChartSpec::sanitize`] leaves
/// it (a value that isn't finite is a gap).
pub fn chart_xml(spec: &ChartSpec) -> String {
    chart_part(spec).0
}

/// [`chart_xml`], and whether the chart limits left series or points of `spec` out of it.
pub(crate) fn chart_part(spec: &ChartSpec) -> (String, bool) {
    let kept = bounded(spec);
    let points = spec.series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(spec.categories.len());
    let cut = kept.series.len() < spec.series.len() || kept.points() < points;
    (write_chart(&kept), cut)
}

fn write_chart(spec: &ChartSpec) -> String {
    let mut w = W::new();
    w.open("c:chartSpace", &[("xmlns:c", NS_C), ("xmlns:a", NS_A), ("xmlns:r", NS_R)]);
    w.empty("c:roundedCorners", &[("val", "0")]);
    w.open("c:chart", &[]);
    // A hidden title is written as none (Word shows none); its text goes in our extension.
    let shown_title = spec.title.as_ref().filter(|_| spec.title_visible);
    let hidden_title = spec.title.as_ref().filter(|_| !spec.title_visible);
    if let Some(t) = shown_title {
        w.open("c:title", &[]);
        w.open("c:tx", &[]);
        w.open("c:rich", &[]);
        w.empty("a:bodyPr", &[]);
        w.empty("a:lstStyle", &[]);
        w.open("a:p", &[]);
        w.open("a:r", &[]);
        run_props(&mut w, "a:rPr", &[("lang", "en-US"), ("b", "0")], &spec.title_style, false);
        w.leaf("a:t", &[], t);
        w.close("a:r");
        w.close("a:p");
        w.close("c:rich");
        w.close("c:tx");
        w.empty("c:overlay", &[("val", "0")]);
        w.close("c:title");
    }
    w.empty("c:autoTitleDeleted", &[("val", if shown_title.is_some() { "0" } else { "1" })]);
    w.open("c:plotArea", &[]);
    w.empty("c:layout", &[]);
    let wr = written(spec);
    plot(&mut w, &wr);
    // The plot area's background: a faint translucent grey (3.5% black), else none.
    w.open("c:spPr", &[]);
    if spec.background {
        // A colour chosen for it is laid on as it is; no colour (or black) is the faint tint.
        w.open("a:solidFill", &[]);
        match spec.background_color.as_deref().filter(|c| *c != "000000") {
            Some(c) => w.empty("a:srgbClr", &[("val", c)]),
            None => {
                w.open("a:srgbClr", &[("val", "000000")]);
                w.empty("a:alpha", &[("val", "3500")]);
                w.close("a:srgbClr");
            }
        }
        w.close("a:solidFill");
    } else {
        w.empty("a:noFill", &[]);
    }
    w.close("c:spPr");
    w.close("c:plotArea");
    if let Some(pos) = spec.legend {
        w.open("c:legend", &[]);
        let p = match pos {
            LegendPos::Right => "r",
            LegendPos::Left => "l",
            LegendPos::Top => "t",
            LegendPos::Bottom => "b",
        };
        w.empty("c:legendPos", &[("val", p)]);
        // Helper series (a waterfall's or funnel's) have no legend entry.
        for (j, _) in wr.hidden.iter().enumerate().filter(|(_, h)| **h) {
            w.open("c:legendEntry", &[]);
            w.empty("c:idx", &[("val", &wr.idx.get(j).copied().unwrap_or(j).to_string())]);
            w.empty("c:delete", &[("val", "1")]);
            w.close("c:legendEntry");
        }
        w.empty("c:overlay", &[("val", "0")]);
        text_pr(&mut w, &spec.legend_style, false, false);
        w.close("c:legend");
    }
    w.empty("c:plotVisOnly", &[("val", "1")]);
    w.empty("c:dispBlanksAs", &[("val", "gap")]);
    w.close("c:chart");
    // The chart area: transparent, no border (a background shades the plot area only).
    w.open("c:spPr", &[]);
    w.empty("a:noFill", &[]);
    w.open("a:ln", &[]);
    w.empty("a:noFill", &[]);
    w.close("a:ln");
    w.close("c:spPr");
    // The workbook the data's cells are in (the part's package relationship), so the chart's data
    // can be edited as a sheet; not refreshed from it on open.
    w.open("c:externalData", &[("r:id", WORKBOOK_ID)]);
    w.empty("c:autoUpdate", &[("val", "0")]);
    w.close("c:externalData");
    // The palette as chosen (series and slices only show as many colours as they need, and use
    // them in turn), so reopening the document gives the same palette back, and round legend
    // keys (the format has no legend key shape). An extension of our own: other consumers skip
    // it (ECMA-376 Part 1 §18.2.7 / Part 3 extensibility); `c:ext` takes any content.
    let round = spec.legend_shape == LegendShape::Circle;
    let look = ext_look(spec);
    if !spec.colors.is_empty() || round || hidden_title.is_some() || !look.is_empty() {
        w.open("c:extLst", &[]);
        w.open("c:ext", &[("uri", PALETTE_EXT)]);
        for c in &spec.colors {
            w.empty("a:srgbClr", &[("val", c)]);
        }
        if round {
            w.empty("wc:legendKey", &[("xmlns:wc", WORDCRAFT_NS), ("shape", "circle")]);
        }
        if let Some(t) = hidden_title {
            w.leaf("wc:hiddenTitle", &[("xmlns:wc", WORDCRAFT_NS)], t);
        }
        if !look.is_empty() {
            let mut a: Vec<(&str, &str)> = vec![("xmlns:wc", WORDCRAFT_NS)];
            a.extend(look.iter().map(|(k, v)| (*k, v.as_str())));
            w.empty("wc:look", &a);
        }
        w.close("c:ext");
        w.close("c:extLst");
    }
    w.close("c:chartSpace");
    w.s
}

/// The look the format has no place for, as `wc:look` attributes: an area chart's smooth top edge
/// and dots (the format has them for lines only) and the whole chart's opacity. Empty when all
/// are the defaults.
fn ext_look(spec: &ChartSpec) -> Vec<(&'static str, String)> {
    let area = spec.kind.is_area();
    let mut a = Vec::new();
    // A waterfall or funnel is written as a stacked chart: which one it is (and a total bar).
    if matches!(spec.kind, ChartType::Waterfall | ChartType::Funnel) || radar_written_filled(spec) {
        a.push(("kind", spec.kind.id().to_string()));
    }
    if spec.waterfall_total {
        a.push(("total", "1".to_string()));
    }
    if area && spec.smooth {
        a.push(("smooth", "1".to_string()));
    }
    if area && spec.markers {
        a.push(("markers", "1".to_string()));
    }
    if spec.opacity < 1.0 {
        a.push(("opacity", format!("{}", spec.opacity)));
    }
    a
}

/// Whether `spec` is a line radar (with or without markers) written as a filled one (for its
/// gradient): our extension and the document record keep its kind.
pub(crate) fn radar_written_filled(spec: &ChartSpec) -> bool {
    spec.radar_gradient && matches!(spec.kind, ChartType::Radar | ChartType::RadarMarkers)
}

/// The `wc:look` element of a chart space's WordCraft extension, when it has one.
pub(crate) fn ext_look_el(space: &El) -> Option<&El> {
    space.child("c:extLst").and_then(|l| l.children("c:ext").find(|e| e.attr("uri") == Some(PALETTE_EXT))).and_then(|e| e.child("?:look"))
}

/// Whether a chart part carries WordCraft's chart extension (its palette, legend key shape or
/// hidden title).
pub(crate) fn has_extension(space: &El) -> bool {
    space.child("c:extLst").is_some_and(|l| l.children("c:ext").any(|e| e.attr("uri") == Some(PALETTE_EXT)))
}

/// The series colour `i` of `spec`'s palette (in turn), when it has one.
fn palette(spec: &ChartSpec, i: usize) -> Option<&str> {
    spec.colors.get(i.checked_rem(spec.colors.len())?).map(String::as_str)
}

/// `<c:spPr>` with a solid fill (and an outline of the same colour when `outline`).
fn solid_sp(w: &mut W, color: &str, outline: bool) {
    w.open("c:spPr", &[]);
    solid(w, color);
    if outline {
        w.open("a:ln", &[("w", "9525")]);
        solid(w, color);
        w.close("a:ln");
    }
    w.close("c:spPr");
}

/// A pie's gap between slices: a white `a:ln` `gap` points wide.
fn slice_gap_ln(w: &mut W, gap: f32) {
    w.open("a:ln", &[("w", &emu(gap))]);
    solid(w, "FFFFFF");
    w.close("a:ln");
}

/// The gap between pie slices a series' outline gives: a white line's width, points.
fn slice_gap_of(ser: Option<&El>) -> f32 {
    let white_w = |sp: Option<&El>| {
        let ln = sp?.child("a:ln")?;
        let white = ln.find("a:srgbClr").and_then(|c| c.attr("val")).is_some_and(|v| v.eq_ignore_ascii_case("FFFFFF"));
        let w = ln.attr("w")?.parse::<f64>().ok().filter(|v| v.is_finite())?;
        white.then_some((w / 12_700.0) as f32)
    };
    ser.and_then(|s| white_w(s.child("c:spPr")).or_else(|| s.child("c:dPt").and_then(|d| white_w(d.child("c:spPr"))))).unwrap_or(0.0)
}

/// A bubble's `c:spPr`: a solid fill at `opacity` (its alpha) and, when see-through, a 0.75 pt
/// outline of the colour, so overlapping bubbles show through each other.
/// `color` is a palette colour, else (`None`) the theme accent series `idx` takes in turn.
fn bubble_sp(w: &mut W, color: Option<&str>, idx: usize, opacity: f32) {
    let accent = format!("accent{}", idx % 6 + 1);
    let (tag, val) = match color {
        Some(c) => ("a:srgbClr", c),
        None => ("a:schemeClr", accent.as_str()),
    };
    w.open("c:spPr", &[]);
    w.open("a:solidFill", &[]);
    if opacity < 1.0 {
        w.open(tag, &[("val", val)]);
        w.empty("a:alpha", &[("val", &((opacity.clamp(0.0, 1.0) * 100_000.0).round() as i64).to_string())]);
        w.close(tag);
    } else {
        w.empty(tag, &[("val", val)]);
    }
    w.close("a:solidFill");
    if opacity < 1.0 {
        w.open("a:ln", &[("w", "9525")]);
        w.open("a:solidFill", &[]);
        w.empty(tag, &[("val", val)]);
        w.close("a:solidFill");
        w.close("a:ln");
    }
    w.close("c:spPr");
}

/// An area series' `c:spPr`: a fill fading down from 80% to 15% opacity of the series colour (a
/// top-to-bottom linear gradient) and a 1.5 pt line of that colour. The colour is the palette's
/// (`color`), else the theme accent the series takes in turn.
fn area_sp(w: &mut W, color: Option<&str>, idx: usize, spec: &ChartSpec) {
    let accent = format!("accent{}", idx % 6 + 1);
    let clr = |w: &mut W, alpha: Option<&str>| {
        let (name, val) = match color {
            Some(c) => ("a:srgbClr", c),
            None => ("a:schemeClr", accent.as_str()),
        };
        match alpha {
            Some(a) => {
                w.open(name, &[("val", val)]);
                w.empty("a:alpha", &[("val", a)]);
                w.close(name);
            }
            None => w.empty(name, &[("val", val)]),
        }
    };
    w.open("c:spPr", &[]);
    w.open("a:gradFill", &[("rotWithShape", "1")]);
    w.open("a:gsLst", &[]);
    // The top at the chart's area opacity, the bottom at the same share of it as 15% is of 80%.
    let (top, bottom) = area_alphas(spec.area_opacity);
    for (pos, alpha) in [("0", top.to_string()), ("100000", bottom.to_string())] {
        w.open("a:gs", &[("pos", pos)]);
        clr(w, Some(&alpha));
        w.close("a:gs");
    }
    w.close("a:gsLst");
    w.empty("a:lin", &[("ang", "5400000"), ("scaled", "0")]);
    w.close("a:gradFill");
    let width = emu(spec.line_width.unwrap_or(AREA_EDGE_W));
    w.open("a:ln", &[("w", &width)]);
    w.open("a:solidFill", &[]);
    clr(w, None);
    w.close("a:solidFill");
    w.close("a:ln");
    w.close("c:spPr");
}

/// Default width of an area's top edge and of a line series, points.
const AREA_EDGE_W: f32 = 1.5;
const LINE_SERIES_W: f32 = 2.25;

/// A width in points as EMU text.
fn emu(pt: f32) -> String {
    ((pt * 12_700.0).round() as i64).to_string()
}

/// The gradient's top and bottom alpha (1,000ths of a percent) for an area opacity.
fn area_alphas(opacity: f32) -> (i64, i64) {
    let top = (opacity.clamp(0.0, 1.0) * 100_000.0).round() as i64;
    (top, (top as f64 * 15.0 / 80.0).round() as i64)
}

fn solid(w: &mut W, color: &str) {
    w.open("a:solidFill", &[]);
    w.empty("a:srgbClr", &[("val", color)]);
    w.close("a:solidFill");
}

/// `spec` held to the chart limits. A model deserialised as is (a `.json` document) never went
/// through [`ChartSpec::sanitize`], and every series repeats every category, so its size would
/// multiply (#3648). Only what the limits keep is copied before sanitizing.
fn bounded(spec: &ChartSpec) -> ChartSpec {
    ChartSpec {
        kind: spec.kind,
        categories: spec.categories.iter().take(MAX_POINTS).cloned().collect(),
        series: spec
            .series
            .iter()
            .take(MAX_SERIES)
            .map(|s| ChartSeries {
                name: s.name.clone(),
                values: s.values.iter().take(MAX_POINTS).copied().collect(),
                sizes: s.sizes.iter().take(MAX_POINTS).copied().collect(),
                kind: s.kind,
                secondary: s.secondary,
            })
            .collect(),
        title: spec.title.clone(),
        legend: spec.legend,
        legend_shape: spec.legend_shape,
        data_labels: spec.data_labels,
        colors: spec.colors.iter().take(wordcraft_doc::chart::MAX_COLORS * 4).cloned().collect(),
        smooth: spec.smooth,
        markers: spec.markers,
        rounded_bars: spec.rounded_bars,
        series_gap: spec.series_gap,
        gridlines: spec.gridlines,
        axis_min: spec.axis_min,
        axis_max: spec.axis_max,
        axis_step: spec.axis_step,
        title_visible: spec.title_visible,
        background: spec.background,
        cat_labels: spec.cat_labels,
        cat_axis: spec.cat_axis,
        val_labels: spec.val_labels,
        val_axis: spec.val_axis,
        legend_style: spec.legend_style.clone(),
        data_labels_style: spec.data_labels_style.clone(),
        cat_labels_style: spec.cat_labels_style.clone(),
        val_labels_style: spec.val_labels_style.clone(),
        title_style: spec.title_style.clone(),
        data_labels_outline: spec.data_labels_outline,
        cat_label_pos: spec.cat_label_pos,
        cat_axis_color: spec.cat_axis_color.clone(),
        val_axis_color: spec.val_axis_color.clone(),
        cat_ticks: spec.cat_ticks,
        val_ticks: spec.val_ticks,
        background_color: spec.background_color.clone(),
        line_width: spec.line_width,
        area_opacity: spec.area_opacity,
        opacity: spec.opacity,
        bubble_opacity: spec.bubble_opacity,
        hole_size: spec.hole_size,
        slice_gap: spec.slice_gap,
        waterfall_total: spec.waterfall_total,
        split_pos: spec.split_pos,
        radar_gradient: spec.radar_gradient,
    }
    .sanitized()
}

/// What `spec` draws as, `w` × `h` points, in the colours of `theme` (series take the accents in
/// turn). Empty when the chart has no series.
pub fn chart_items(spec: &ChartSpec, theme: &[Rgb], w: f32, h: f32) -> Vec<GraphicItem> {
    match xml::parse(chart_xml(spec).as_bytes()) {
        Ok(space) => crate::read::chart::chart_items(&space, theme, w, h),
        Err(_) => Vec::new(),
    }
}

/// The plot area of `spec` drawn `w` × `h` points (as [`chart_items`] draws it, by the same
/// layout): `[x, y, w, h]` in points from the graphic's top-left. Bar, column, line, area and
/// scatter charts: the rectangle between the axes (from the value axis line to the plot's far
/// edge, from the top of the value scale to the category axis line); pie and doughnut charts:
/// the circle's bounding square. `None` when nothing is drawn.
pub fn chart_plot_rect(spec: &ChartSpec, theme: &[Rgb], w: f32, h: f32) -> Option<[f32; 4]> {
    let space = xml::parse(chart_xml(spec).as_bytes()).ok()?;
    crate::read::chart::chart_plot_rect(&space, theme, w, h)
}

/// What a chart is written as: the model itself, or for a waterfall or funnel (types the chart
/// part format has no element for) a stacked column or bar chart that draws the same, with
/// invisible helper series (the classic spreadsheet technique). `idx` is each series' `c:idx`
/// (which theme accent it takes), `hidden` the helper series (no fill, no legend entry, no labels).
pub(crate) struct Written {
    pub(crate) spec: ChartSpec,
    idx: Vec<usize>,
    hidden: Vec<bool>,
    /// The bar groups' `c:gapWidth` (percent of a bar's width): 150, or a funnel's narrow 10.
    gap: u32,
    /// A funnel: each stage of the shown series takes the palette in turn (`c:dPt`), the first
    /// stage the first colour, as pie slices do.
    stages: bool,
}

/// Names of a written waterfall's series after the first (its steps keep the model's name).
const DECREASE: &str = "Decrease";
const TOTAL: &str = "Total";
const BASE: &str = "Base";

/// [`Written`] for `spec` (as [`bounded`] leaves it).
pub(crate) fn written(spec: &ChartSpec) -> Written {
    let n = spec.series.len();
    let plain = |spec: ChartSpec| Written { spec, idx: (0..n).collect(), hidden: vec![false; n], gap: 150, stages: false };
    match spec.kind {
        ChartType::Waterfall => {
            // Square ends: the floating steps read as one run, and a rounded top would point the
            // wrong way on a decrease.
            let (series, idx, hidden) = waterfall_series(spec);
            Written {
                spec: ChartSpec { kind: ChartType::ColumnStacked, series, rounded_bars: false, ..spec.clone() },
                idx,
                hidden,
                gap: 150,
                stages: false,
            }
        }
        ChartType::Funnel => {
            let first = spec.series.first().cloned().unwrap_or_default();
            let top = first.values.iter().flatten().fold(0.0f64, |a, v| a.max(*v));
            let rev = |v: &[Option<f64>]| v.iter().rev().copied().collect::<Vec<_>>();
            let offset = first.values.iter().rev().map(|v| Some((top - v.unwrap_or(0.0).max(0.0)) / 2.0)).collect();
            let name = if first.name.is_empty() { "Value".to_string() } else { first.name.clone() };
            let series = vec![ChartSeries::of("Offset", offset), ChartSeries::of(&name, rev(&first.values))];
            let categories = spec.categories.iter().rev().cloned().collect();
            // Drawn as a funnel is: stages nearly touching, filling the plot's width, square ends, no
            // value scale (no value labels, axis line, ticks or gridlines).
            let top = if top > 0.0 { Some(top) } else { None };
            Written {
                spec: ChartSpec {
                    kind: ChartType::BarStacked,
                    categories,
                    series,
                    gridlines: Gridlines::None,
                    val_labels: false,
                    val_axis: false,
                    val_ticks: false,
                    rounded_bars: false,
                    axis_min: top.map(|_| 0.0),
                    axis_max: top,
                    axis_step: None,
                    ..spec.clone()
                },
                idx: vec![1, 0],
                hidden: vec![true, false],
                gap: 10,
                stages: true,
            }
        }
        _ => plain(spec.clone()),
    }
}

/// A waterfall's first series as the stacked series that draw it: an invisible base under each
/// floating bar, the increases, the decreases and (with `waterfall_total`) the total bar. A step
/// that crosses zero is drawn from zero to where it ends.
fn waterfall_series(spec: &ChartSpec) -> (Vec<ChartSeries>, Vec<usize>, Vec<bool>) {
    let first = spec.series.first().cloned().unwrap_or_default();
    let n = spec.points();
    let (mut base, mut inc, mut dec, mut total) = (vec![None; n], vec![None; n], vec![None; n], vec![None; n]);
    let mut cum = 0.0f64;
    for i in 0..n {
        let Some(v) = first.values.get(i).copied().flatten() else { continue };
        if spec.waterfall_total && i + 1 == n {
            if let Some(t) = total.get_mut(i) {
                *t = Some(v);
            }
            continue;
        }
        let (a, b) = (cum, cum + v);
        cum = b;
        let (lo, hi) = (a.min(b), a.max(b));
        let (bottom, bar) = if lo >= 0.0 {
            (lo, hi - lo)
        } else if hi <= 0.0 {
            (hi, lo - hi)
        } else {
            (0.0, b)
        };
        if let Some(x) = base.get_mut(i) {
            *x = Some(bottom);
        }
        let target = if v >= 0.0 { inc.get_mut(i) } else { dec.get_mut(i) };
        if let Some(x) = target {
            *x = Some(bar);
        }
    }
    let name = if first.name.is_empty() { "Increase".to_string() } else { first.name.clone() };
    let mut series = vec![ChartSeries::of(BASE, base), ChartSeries::of(&name, inc), ChartSeries::of(DECREASE, dec)];
    let mut idx = vec![2, 0, 1];
    if spec.waterfall_total {
        series.push(ChartSeries::of(TOTAL, total));
        idx = vec![3, 0, 1, 2];
    }
    let hidden = (0..series.len()).map(|j| j == 0).collect();
    (series, idx, hidden)
}

/// The model of a waterfall or funnel written by [`written`] and read back as the stacked chart
/// `base` (what our extension or document record says it is): `None` when `base` isn't one.
pub(crate) fn rebuilt(base: &ChartSpec, kind: ChartType, total: bool) -> Option<ChartSpec> {
    let val = |s: Option<&ChartSeries>, i: usize| s.and_then(|s| s.values.get(i).copied().flatten());
    match kind {
        ChartType::Waterfall if base.kind == ChartType::ColumnStacked && (3..=4).contains(&base.series.len()) => {
            let (inc, dec, tot) = (base.series.get(1), base.series.get(2), base.series.get(3));
            let n = base.points();
            let values = (0..n)
                .map(|i| {
                    if total && i + 1 == n {
                        return val(tot, i);
                    }
                    match (val(inc, i), val(dec, i)) {
                        (None, None) => None,
                        (a, b) => Some(a.unwrap_or(0.0).abs() - b.unwrap_or(0.0).abs()),
                    }
                })
                .collect();
            let name = inc.map(|s| s.name.clone()).unwrap_or_default();
            let series = vec![ChartSeries::of(&name, values)];
            // The square ends [`written`] gives it aren't the model's: back to the default.
            Some(ChartSpec { kind, series, waterfall_total: total, rounded_bars: ChartSpec::default().rounded_bars, ..base.clone() }.sanitized())
        }
        ChartType::Funnel if base.kind == ChartType::BarStacked && base.series.len() == 2 => {
            let v = base.series.get(1)?;
            let series = vec![ChartSeries::of(&v.name, v.values.iter().rev().copied().collect())];
            let categories = base.categories.iter().rev().cloned().collect();
            // What [`written`] turns off to draw a funnel (no value scale, square ends) isn't the
            // model's: back to the defaults.
            let d = ChartSpec::default();
            Some(
                ChartSpec {
                    kind,
                    categories,
                    series,
                    axis_min: None,
                    axis_max: None,
                    axis_step: None,
                    gridlines: d.gridlines,
                    val_labels: d.val_labels,
                    val_axis: d.val_axis,
                    val_ticks: d.val_ticks,
                    rounded_bars: d.rounded_bars,
                    ..base.clone()
                }
                .sanitized(),
            )
        }
        _ => None,
    }
}

/// The chart part element of a chart type (combo charts have one per kind and axis).
fn type_element(k: ChartType) -> &'static str {
    match k {
        k if k.is_bar() => "c:barChart",
        k if k.is_line() => "c:lineChart",
        k if k.is_area() => "c:areaChart",
        k if k.is_radar() => "c:radarChart",
        k if k.is_of_pie() => "c:ofPieChart",
        ChartType::Pie => "c:pieChart",
        ChartType::Doughnut => "c:doughnutChart",
        ChartType::Bubble => "c:bubbleChart",
        ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth => "c:scatterChart",
        // Written as their stacked or grouped forms; never asked for.
        _ => "c:barChart",
    }
}

/// `c:grouping` of a stacking type.
fn grouping(k: ChartType) -> &'static str {
    match k {
        k if k.is_percent() => "percentStacked",
        ChartType::ColumnStacked | ChartType::BarStacked | ChartType::AreaStacked | ChartType::LineStacked => "stacked",
        k if k.is_bar() => "clustered",
        _ => "standard",
    }
}

/// Secondary axis ids (combo charts).
const CAT_AX2: &str = "500000003";
const VAL_AX2: &str = "500000004";

/// The chart type element and the series in it, then the axes.
fn plot(w: &mut W, wr: &Written) {
    let spec = &wr.spec;
    let k = spec.kind;
    if k == ChartType::Combo {
        plot_combo(w, wr);
        return;
    }
    let all: Vec<usize> = (0..spec.series.len()).collect();
    type_group(w, wr, k, &all, (CAT_AX, VAL_AX));
    match k {
        k if k.is_round() => {}
        k if k.is_xy() => {
            value_axis(w, CAT_AX, VAL_AX, "b", None, "midCat", cat_look(spec), false);
            value_axis(w, VAL_AX, CAT_AX, "l", Some(spec), "midCat", val_look(spec), false);
        }
        _ => {
            let horizontal = matches!(k, ChartType::Bar | ChartType::BarStacked | ChartType::BarPercent);
            category_axis(w, (CAT_AX, VAL_AX), if horizontal { "l" } else { "b" }, false, cat_look(spec));
            // Area charts run from the value axis to the far edge: categories on the tick marks.
            let between = if k.is_area() { "midCat" } else { "between" };
            value_axis(w, VAL_AX, CAT_AX, if horizontal { "b" } else { "l" }, Some(spec), between, val_look(spec), false);
        }
    }
}

/// A combo chart: a stacked area group, a clustered column group and a line group on the
/// primary axes (in that order, areas at the back), and the same on the secondary axes (a value
/// axis at the right, its category axis deleted) for series marked `secondary`. When every
/// series is secondary they all go on the primary axes.
fn plot_combo(w: &mut W, wr: &Written) {
    let spec = &wr.spec;
    let any_primary = spec.series.iter().any(|s| !s.secondary);
    let on2 = |s: &ChartSeries| s.secondary && any_primary;
    let pick = |want: SeriesKind, second: bool| -> Vec<usize> {
        spec.series.iter().enumerate().filter(|(_, s)| s.kind.unwrap_or_default() == want && on2(s) == second).map(|(j, _)| j).collect()
    };
    let secondary = spec.series.iter().any(on2);
    for second in [false, true] {
        for want in [SeriesKind::Area, SeriesKind::Column, SeriesKind::Line] {
            let members = pick(want, second);
            if members.is_empty() {
                continue;
            }
            let k = match want {
                SeriesKind::Area => ChartType::AreaStacked,
                SeriesKind::Column => ChartType::Column,
                SeriesKind::Line => ChartType::Line,
            };
            type_group(w, wr, k, &members, if second { (CAT_AX2, VAL_AX2) } else { (CAT_AX, VAL_AX) });
        }
    }
    category_axis(w, (CAT_AX, VAL_AX), "b", false, cat_look(spec));
    value_axis(w, VAL_AX, CAT_AX, "l", Some(spec), "between", val_look(spec), false);
    if secondary {
        category_axis(w, (CAT_AX2, VAL_AX2), "b", true, cat_look(spec));
        value_axis(w, VAL_AX2, CAT_AX2, "r", None, "between", val_look(spec), true);
    }
}

/// One chart type element of kind `k` holding the series `members` (indices into the written
/// chart's series) on the axes `ax` (category or x, value or y).
fn type_group(w: &mut W, wr: &Written, k: ChartType, members: &[usize], ax: (&str, &str)) {
    let spec = &wr.spec;
    let el = type_element(k);
    w.open(el, &[]);
    match k {
        k if k.is_bar() => {
            let horizontal = matches!(k, ChartType::Bar | ChartType::BarStacked | ChartType::BarPercent);
            w.empty("c:barDir", &[("val", if horizontal { "bar" } else { "col" })]);
            w.empty("c:grouping", &[("val", grouping(k))]);
        }
        k if k.is_line() || k.is_area() => w.empty("c:grouping", &[("val", grouping(k))]),
        ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth => {
            let style = match (k, spec.markers) {
                (ChartType::Scatter, _) => "marker",
                (ChartType::ScatterLines, true) => "lineMarker",
                (ChartType::ScatterLines, false) => "line",
                (_, true) => "smoothMarker",
                (_, false) => "smooth",
            };
            w.empty("c:scatterStyle", &[("val", style)]);
        }
        k if k.is_radar() => {
            // A gradient is a fill, which only filled radars have: written filled (our extension
            // keeps the kind).
            let style = match k {
                _ if spec.radar_gradient => "filled",
                ChartType::RadarMarkers => "marker",
                ChartType::RadarFilled => "filled",
                _ => "standard",
            };
            w.empty("c:radarStyle", &[("val", style)]);
        }
        k if k.is_of_pie() => w.empty("c:ofPieType", &[("val", if k == ChartType::BarOfPie { "bar" } else { "pie" })]),
        _ => {}
    }
    w.empty("c:varyColors", &[("val", if k.is_round() { "1" } else { "0" })]);
    // Pie charts show the first series; the others are kept for other types.
    for j in members {
        if let Some(s) = spec.series.get(*j) {
            series(w, wr, k, *j, s);
        }
    }
    if spec.data_labels {
        data_labels(w, spec, k);
    }
    match k {
        k if k.is_bar() => {
            w.empty("c:gapWidth", &[("val", &wr.gap.to_string())]);
            if grouping(k) != "clustered" {
                w.empty("c:overlap", &[("val", "100")]);
            } else if spec.series_gap > 0.0 {
                // A negative overlap is a gap between the bars of a category (ECMA-376 ST_Overlap).
                w.empty("c:overlap", &[("val", &format!("-{}", spec.series_gap))]);
            }
        }
        k if k.is_line() => w.empty("c:marker", &[("val", "1")]),
        ChartType::Pie => w.empty("c:firstSliceAng", &[("val", "0")]),
        ChartType::Doughnut => {
            w.empty("c:firstSliceAng", &[("val", "0")]);
            w.empty("c:holeSize", &[("val", &spec.hole_size.clamp(10, 90).to_string())]);
        }
        k if k.is_of_pie() => {
            // The last `split_pos` points go to the second plot, three quarters the main pie's size.
            w.empty("c:gapWidth", &[("val", "100")]);
            w.empty("c:splitType", &[("val", "pos")]);
            w.empty("c:splitPos", &[("val", &spec.split_pos.to_string())]);
            w.empty("c:secondPieSize", &[("val", "75")]);
            w.open("c:serLines", &[]);
            w.open("c:spPr", &[]);
            w.open("a:ln", &[("w", "9525")]);
            solid(w, AXIS_LINE);
            w.close("a:ln");
            w.close("c:spPr");
            w.close("c:serLines");
        }
        ChartType::Bubble => {
            // Bubbles sized by area, the biggest at the default scale.
            w.empty("c:bubble3D", &[("val", "0")]);
            w.empty("c:bubbleScale", &[("val", "100")]);
            w.empty("c:showNegBubbles", &[("val", "0")]);
            w.empty("c:sizeRepresents", &[("val", "area")]);
        }
        _ => {}
    }
    if !k.is_round() {
        w.empty("c:axId", &[("val", ax.0)]);
        w.empty("c:axId", &[("val", ax.1)]);
    }
    w.close(el);
}

/// A chart type element's `c:dLbls`: values shown, in their look, where the type allows a place.
fn data_labels(w: &mut W, spec: &ChartSpec, k: ChartType) {
    w.open("c:dLbls", &[]);
    // Their text's look and white outline, when set (else each label takes the colour that
    // reads on what it sits on).
    if !spec.data_labels_style.is_default() || spec.data_labels_outline {
        text_pr(w, &spec.data_labels_style, false, spec.data_labels_outline);
    }
    // Where the labels go, where the chart type allows one (area, doughnut and radar charts
    // don't): inside the end of a bar, the middle of a stacked segment or a bubble, above a
    // point, best fit on a pie.
    let pos = match k {
        ChartType::Column | ChartType::Bar => Some("inEnd"),
        k if k.is_bar() => Some("ctr"),
        k if k.is_line() => Some("t"),
        ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth => Some("t"),
        ChartType::Bubble => Some("ctr"),
        ChartType::Pie | ChartType::PieOfPie | ChartType::BarOfPie => Some("bestFit"),
        _ => None,
    };
    if let Some(p) = pos {
        w.empty("c:dLblPos", &[("val", p)]);
    }
    for (name, on) in [
        ("c:showLegendKey", false),
        ("c:showVal", true),
        ("c:showCatName", false),
        ("c:showSerName", false),
        ("c:showPercent", false),
        ("c:showBubbleSize", false),
    ] {
        w.empty(name, &[("val", if on { "1" } else { "0" })]);
    }
    w.close("c:dLbls");
}

/// The workbook column of series `j`'s values (and of its bubble sizes): the column after the
/// categories, each series in turn; a bubble chart's sizes in the column after each series'.
fn value_col(spec: &ChartSpec, j: usize) -> usize {
    if spec.kind == ChartType::Bubble { j.saturating_mul(2).saturating_add(1) } else { j.saturating_add(1) }
}

fn size_col(j: usize) -> usize {
    j.saturating_mul(2).saturating_add(2)
}

/// A line's `c:spPr`: its colour (else the theme's), its width (else the default), round caps.
fn line_sp(w: &mut W, spec: &ChartSpec, color: Option<&str>) {
    if color.is_some() || spec.line_width.is_some() {
        let width = emu(spec.line_width.unwrap_or(LINE_SERIES_W));
        w.open("c:spPr", &[]);
        w.open("a:ln", &[("w", &width), ("cap", "rnd")]);
        if let Some(c) = color {
            solid(w, c);
        }
        w.empty("a:round", &[]);
        w.close("a:ln");
        w.close("c:spPr");
    }
}

/// Opacity of a radar gradient at the centre and at the edge, 1,000ths of a percent.
const RADAR_CENTRE: &str = "8000";
const RADAR_EDGE: &str = "35000";

/// A filled radar series' `c:spPr`: a solid fill of its colour, or with `radar_gradient` a
/// circular gradient from the middle of its shape (`a:path path="circle"` with the focus at the
/// centre: light there, stronger at the edge), and its outline at the chart's line width. The
/// colour is the palette's (`color`), else the theme accent the series takes (`idx`).
fn radar_fill_sp(w: &mut W, spec: &ChartSpec, color: Option<&str>, idx: usize) {
    let accent = format!("accent{}", idx % 6 + 1);
    let clr = |w: &mut W, alpha: Option<&str>| {
        let (name, val) = match color {
            Some(c) => ("a:srgbClr", c),
            None => ("a:schemeClr", accent.as_str()),
        };
        match alpha {
            Some(a) => {
                w.open(name, &[("val", val)]);
                w.empty("a:alpha", &[("val", a)]);
                w.close(name);
            }
            None => w.empty(name, &[("val", val)]),
        }
    };
    w.open("c:spPr", &[]);
    if spec.radar_gradient {
        w.open("a:gradFill", &[("rotWithShape", "1")]);
        w.open("a:gsLst", &[]);
        for (pos, alpha) in [("0", RADAR_CENTRE), ("100000", RADAR_EDGE)] {
            w.open("a:gs", &[("pos", pos)]);
            clr(w, Some(alpha));
            w.close("a:gs");
        }
        w.close("a:gsLst");
        w.open("a:path", &[("path", "circle")]);
        w.empty("a:fillToRect", &[("l", "50000"), ("t", "50000"), ("r", "50000"), ("b", "50000")]);
        w.close("a:path");
        w.close("a:gradFill");
    } else {
        w.open("a:solidFill", &[]);
        clr(w, None);
        w.close("a:solidFill");
    }
    w.open("a:ln", &[("w", &emu(spec.line_width.unwrap_or(LINE_SERIES_W))), ("cap", "rnd")]);
    w.open("a:solidFill", &[]);
    clr(w, None);
    w.close("a:solidFill");
    w.empty("a:round", &[]);
    w.close("a:ln");
    w.close("c:spPr");
}

/// A series' `c:marker`: circles in its colour when `on`, else none.
fn marker(w: &mut W, on: bool, color: Option<&str>) {
    w.open("c:marker", &[]);
    if on {
        w.empty("c:symbol", &[("val", "circle")]);
        w.empty("c:size", &[("val", MARKER_SIZE)]);
        if let Some(c) = color {
            solid_sp(w, c, true);
        }
    } else {
        w.empty("c:symbol", &[("val", "none")]);
    }
    w.close("c:marker");
}

/// One `c:ser` of the written chart's series `j`, drawn as type `k` (ECMA-376 element order:
/// idx, order, tx, spPr, marker / invertIfNegative, dPt, dLbls, cat/xVal, val/yVal, bubbleSize,
/// smooth). A palette colour is the series' fill (a line series: its line; a scatter series: its
/// markers); pie slices of the first series take the palette in turn (`c:dPt`). A hidden helper
/// series has no fill and no labels.
fn series(w: &mut W, wr: &Written, k: ChartType, j: usize, s: &ChartSeries) {
    let spec = &wr.spec;
    let idx = wr.idx.get(j).copied().unwrap_or(j);
    let hidden = wr.hidden.get(j).copied().unwrap_or(false);
    let color = if hidden { None } else { palette(spec, idx) };
    w.open("c:ser", &[]);
    w.empty("c:idx", &[("val", &idx.to_string())]);
    w.empty("c:order", &[("val", &j.to_string())]);
    let vcol = value_col(spec, j);
    if !s.name.is_empty() {
        w.open("c:tx", &[]);
        let name_ref = format!("{SHEET}!${}$1", column(vcol));
        // The header cell's text: the name, made unique among the table's columns.
        let name = header_names(spec).get(vcol).cloned().unwrap_or_else(|| s.name.clone());
        str_ref(w, &name_ref, std::slice::from_ref(&name));
        w.close("c:tx");
    }
    match k {
        // Markers only: the series has no line.
        ChartType::Scatter => {
            w.open("c:spPr", &[]);
            w.open("a:ln", &[("w", "19050")]);
            w.empty("a:noFill", &[]);
            w.close("a:ln");
            w.close("c:spPr");
            if let Some(c) = color {
                w.open("c:marker", &[]);
                w.empty("c:symbol", &[("val", "circle")]);
                w.empty("c:size", &[("val", MARKER_SIZE)]);
                solid_sp(w, c, true);
                w.close("c:marker");
            }
        }
        k if k.is_radar() && (spec.radar_gradient || k == ChartType::RadarFilled) => {
            radar_fill_sp(w, spec, color, idx);
            if k == ChartType::RadarMarkers {
                marker(w, true, color);
            }
        }
        ChartType::ScatterLines | ChartType::ScatterSmooth | ChartType::Radar | ChartType::RadarMarkers => {
            line_sp(w, spec, color);
            let dots = match k {
                ChartType::Radar => false,
                ChartType::RadarMarkers => true,
                _ => spec.markers,
            };
            marker(w, dots, color);
        }
        ChartType::Bubble => {
            bubble_sp(w, color, idx, spec.bubble_opacity);
            w.empty("c:invertIfNegative", &[("val", "0")]);
            // One series: each bubble takes the palette in turn (as pie slices do).
            if spec.series.len() == 1 && !spec.colors.is_empty() {
                for p in 0..s.values.len() {
                    let Some(c) = palette(spec, p) else { break };
                    w.open("c:dPt", &[]);
                    w.empty("c:idx", &[("val", &p.to_string())]);
                    w.empty("c:invertIfNegative", &[("val", "0")]);
                    w.empty("c:bubble3D", &[("val", "0")]);
                    bubble_sp(w, Some(c), p, spec.bubble_opacity);
                    w.close("c:dPt");
                }
            }
        }
        k if k.is_bar() => {
            // Rounded bars: the series' geometry (a:prstGeom comes before the fill, ECMA-376
            // CT_ShapeProperties). Word draws chart series with its own geometry and ignores it;
            // WordCraft draws the bars with rounded corners.
            if hidden {
                w.open("c:spPr", &[]);
                w.empty("a:noFill", &[]);
                w.open("a:ln", &[]);
                w.empty("a:noFill", &[]);
                w.close("a:ln");
                w.close("c:spPr");
            } else if color.is_some() || spec.rounded_bars {
                w.open("c:spPr", &[]);
                if spec.rounded_bars {
                    w.open("a:prstGeom", &[("prst", "roundRect")]);
                    w.empty("a:avLst", &[]);
                    w.close("a:prstGeom");
                }
                if let Some(c) = color {
                    solid(w, c);
                }
                w.close("c:spPr");
            }
            w.empty("c:invertIfNegative", &[("val", "0")]);
            // A funnel's stages in the palette's colours (written bottom-up: point p is stage n-1-p).
            if wr.stages && !hidden {
                let n = s.values.len();
                for p in 0..n {
                    let Some(c) = palette(spec, n - 1 - p) else { break };
                    w.open("c:dPt", &[]);
                    w.empty("c:idx", &[("val", &p.to_string())]);
                    w.empty("c:invertIfNegative", &[("val", "0")]);
                    w.empty("c:bubble3D", &[("val", "0")]);
                    solid_sp(w, c, false);
                    w.close("c:dPt");
                }
            }
            if hidden {
                w.open("c:dLbls", &[]);
                w.empty("c:delete", &[("val", "1")]);
                w.close("c:dLbls");
            }
        }
        k if k.is_line() => {
            line_sp(w, spec, color);
            marker(w, spec.markers, color);
        }
        k if k.is_area() => area_sp(w, color, idx, spec),
        _ => {
            // Pie charts: only the first series is shown, its slices take the palette. A gap
            // between slices is a white outline (the series' and each slice's).
            let gap = (k.is_round() && spec.slice_gap > 0.0).then_some(spec.slice_gap);
            if let Some(g) = gap {
                w.open("c:spPr", &[]);
                slice_gap_ln(w, g);
                w.close("c:spPr");
            }
            if j == 0 && !spec.colors.is_empty() {
                for p in 0..spec.points() {
                    let Some(c) = palette(spec, p) else { break };
                    w.open("c:dPt", &[]);
                    w.empty("c:idx", &[("val", &p.to_string())]);
                    w.open("c:spPr", &[]);
                    solid(w, c);
                    if let Some(g) = gap {
                        slice_gap_ln(w, g);
                    }
                    w.close("c:spPr");
                    w.close("c:dPt");
                }
            }
        }
    }
    // The data, from the embedded workbook's cells (categories in column A, this series in its
    // column), with a copy of them (the cache) for whoever reads the chart without it.
    let n = spec.points();
    let cats_ref = sheet_range(0, n);
    let xy = k.is_xy();
    let (cat_el, val_el) = if xy { ("c:xVal", "c:yVal") } else { ("c:cat", "c:val") };
    w.open(cat_el, &[]);
    match scatter_xs(spec).filter(|_| xy) {
        Some(xs) => num_ref(w, &cats_ref, &xs),
        None => str_ref(w, &cats_ref, &spec.categories),
    }
    w.close(cat_el);
    w.open(val_el, &[]);
    num_ref(w, &sheet_range(vcol, n), &s.values);
    w.close(val_el);
    if k == ChartType::Bubble {
        w.open("c:bubbleSize", &[]);
        num_ref(w, &sheet_range(size_col(j), n), &s.sizes);
        w.close("c:bubbleSize");
        w.empty("c:bubble3D", &[("val", "0")]);
    }
    if k.is_line() || matches!(k, ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth) {
        let smooth = (k.is_line() && spec.smooth) || k == ChartType::ScatterSmooth;
        w.empty("c:smooth", &[("val", if smooth { "1" } else { "0" })]);
    }
    w.close("c:ser");
}

/// The embedded workbook's sheet the chart's data is in.
const SHEET: &str = "Sheet1";

/// A spreadsheet column's letters (0 = `A`, 25 = `Z`, 26 = `AA`…).
fn column(i: usize) -> String {
    let mut n = i.saturating_add(1);
    let mut out = Vec::new();
    while n > 0 {
        let r = (n - 1) % 26;
        out.push(char::from(b'A' + r as u8));
        n = (n - 1) / 26;
    }
    out.iter().rev().collect()
}

/// The cells of column `col` holding `n` values, under the header row: `Sheet1!$B$2:$B$5`
/// (a single cell when there is at most one).
fn sheet_range(col: usize, n: usize) -> String {
    let c = column(col);
    if n <= 1 { format!("{SHEET}!${c}$2") } else { format!("{SHEET}!${c}$2:${c}${}", n.saturating_add(1)) }
}

/// The categories of a scatter chart as x values, when every one is a number (or empty).
fn scatter_xs(spec: &ChartSpec) -> Option<Vec<Option<f64>>> {
    spec.categories.iter().map(|c| if c.is_empty() { Some(None) } else { c.parse::<f64>().ok().filter(|v| v.is_finite()).map(Some) }).collect()
}

/// `c:strRef`: the cells `f` and a cache of their texts.
fn str_ref(w: &mut W, f: &str, items: &[String]) {
    w.open("c:strRef", &[]);
    w.leaf("c:f", &[], f);
    w.open("c:strCache", &[]);
    w.empty("c:ptCount", &[("val", &items.len().to_string())]);
    for (i, s) in items.iter().enumerate().filter(|(_, s)| !s.is_empty()) {
        w.open("c:pt", &[("idx", &i.to_string())]);
        w.leaf("c:v", &[], s);
        w.close("c:pt");
    }
    w.close("c:strCache");
    w.close("c:strRef");
}

/// `c:numRef`: the cells `f` and a cache of their values (a gap has no point).
fn num_ref(w: &mut W, f: &str, values: &[Option<f64>]) {
    w.open("c:numRef", &[]);
    w.leaf("c:f", &[], f);
    w.open("c:numCache", &[]);
    w.leaf("c:formatCode", &[], "General");
    w.empty("c:ptCount", &[("val", &values.len().to_string())]);
    for (i, v) in values.iter().enumerate() {
        if let Some(v) = v.filter(|v| v.is_finite()) {
            w.open("c:pt", &[("idx", &i.to_string())]);
            w.leaf("c:v", &[], &num(v));
            w.close("c:pt");
        }
    }
    w.close("c:numCache");
    w.close("c:numRef");
}

/// Content type of a workbook (`.xlsx`) part.
pub(crate) const XLSX_CT: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
/// Relationship type of an embedded package (the chart's workbook).
pub(crate) const PACKAGE_REL: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package";
/// The chart part's relationship id of its workbook (its only relationship).
const WORKBOOK_ID: &str = "rId1";

/// The workbook a chart part's data comes from (`.xlsx` bytes): one sheet, `Sheet1`, with the
/// categories in column A from row 2, each series in the next column with its name in row 1
/// (A1 empty), numbers as numbers, texts as inline strings, gaps empty — the cells the chart's
/// `c:f` formulas name, as [`chart_xml`] writes them for `spec`.
pub fn chart_workbook(spec: &ChartSpec) -> Result<Vec<u8>, crate::DocxError> {
    let spec = written(&bounded(spec)).spec;
    let xs = if spec.kind.is_xy() { scatter_xs(&spec) } else { None };
    let mut s = W::new();
    s.open("worksheet", &[("xmlns", NS_SHEET), ("xmlns:r", NS_R)]);
    s.open("sheetData", &[]);
    let text_cell = |s: &mut W, at: &str, t: &str| {
        if !t.is_empty() {
            s.open("c", &[("r", at), ("t", "inlineStr")]);
            s.open("is", &[]);
            s.leaf("t", &[("xml:space", "preserve")], t);
            s.close("is");
            s.close("c");
        }
    };
    let num_cell = |s: &mut W, at: &str, v: Option<f64>| {
        if let Some(v) = v.filter(|v| v.is_finite()) {
            s.open("c", &[("r", at)]);
            s.leaf("v", &[], &num(v));
            s.close("c");
        }
    };
    // The header row: the table's column names (A1 a single space, as a table needs one).
    let headers = header_names(&spec);
    s.open("row", &[("r", "1")]);
    for (j, h) in headers.iter().enumerate() {
        text_cell(&mut s, &format!("{}1", column(j)), h);
    }
    s.close("row");
    for (i, cat) in spec.categories.iter().enumerate() {
        let r = i.saturating_add(2).to_string();
        s.open("row", &[("r", &r)]);
        match xs.as_ref().and_then(|xs| xs.get(i)) {
            Some(x) => num_cell(&mut s, &format!("A{r}"), *x),
            None => text_cell(&mut s, &format!("A{r}"), cat),
        }
        for (j, ser) in spec.series.iter().enumerate() {
            num_cell(&mut s, &format!("{}{r}", column(value_col(&spec, j))), ser.values.get(i).copied().flatten());
            if spec.kind == ChartType::Bubble {
                num_cell(&mut s, &format!("{}{r}", column(size_col(j))), ser.sizes.get(i).copied().flatten());
            }
        }
        s.close("row");
    }
    s.close("sheetData");
    // The data as a table, so rows and columns added next to it join it and a deleted column takes
    // its series with it, rather than leaving a formula pointing at nothing.
    s.open("tableParts", &[("count", "1")]);
    s.empty("tablePart", &[("r:id", "rId1")]);
    s.close("tableParts");
    s.close("worksheet");
    let part = |xml: &str| format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n{xml}").into_bytes();
    let entries = vec![
        (
            "[Content_Types].xml".to_string(),
            part(concat!(
                r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#,
                r#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#,
                r#"<Default Extension="xml" ContentType="application/xml"/>"#,
                r#"<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>"#,
                r#"<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>"#,
                r#"<Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>"#,
                r#"<Override PartName="/xl/tables/table1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml"/>"#,
                r#"</Types>"#
            )),
        ),
        (
            "_rels/.rels".to_string(),
            part(concat!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>"#,
                r#"</Relationships>"#
            )),
        ),
        (
            "xl/workbook.xml".to_string(),
            part(concat!(
                r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
                r#"<sheets><sheet name="Sheet1" sheetId="1" r:id="rId1"/></sheets></workbook>"#
            )),
        ),
        (
            "xl/_rels/workbook.xml.rels".to_string(),
            part(concat!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>"#,
                r#"<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>"#,
                r#"</Relationships>"#
            )),
        ),
        ("xl/worksheets/sheet1.xml".to_string(), s.into_bytes()),
        (
            "xl/worksheets/_rels/sheet1.xml.rels".to_string(),
            part(concat!(
                r#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#,
                r#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/>"#,
                r#"</Relationships>"#
            )),
        ),
        ("xl/tables/table1.xml".to_string(), table_xml(&headers, spec.points())),
        (
            "xl/styles.xml".to_string(),
            part(concat!(
                r#"<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">"#,
                r#"<fonts count="1"><font><sz val="11"/></font></fonts>"#,
                r#"<fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>"#,
                r#"<borders count="1"><border><left/><right/><top/><bottom/><diagonal/></border></borders>"#,
                r#"<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>"#,
                r#"<cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs>"#,
                r#"<cellStyles count="1"><cellStyle name="Normal" xfId="0" builtinId="0"/></cellStyles>"#,
                r#"</styleSheet>"#
            )),
        ),
    ];
    crate::package::zip_entries(&entries)
}

/// The workbook's header row, which is also its table's column names: a single space over the
/// categories (a table column needs a name), then each series' name, or `Series N` for one
/// without; a name another column already has (table column names are unique, in any case)
/// gets the smallest number after it that makes it unique.
fn header_names(spec: &ChartSpec) -> Vec<String> {
    let mut taken: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut out = Vec::with_capacity(spec.series.len().saturating_add(1));
    taken.insert(" ".to_string());
    out.push(" ".to_string());
    for (j, s) in spec.series.iter().enumerate() {
        let base = if s.name.is_empty() { format!("Series {}", j.saturating_add(1)) } else { s.name.clone() };
        // A bubble chart's sizes follow each series in a column of their own.
        let names = if spec.kind == ChartType::Bubble { vec![base.clone(), format!("{base} size")] } else { vec![base] };
        for base in names {
            let mut name = base.clone();
            let mut k = 2u32;
            while !taken.insert(name.to_lowercase()) {
                name = format!("{base}{k}");
                k = k.saturating_add(1);
            }
            out.push(name);
        }
    }
    out
}

/// The workbook's table over its data: the header row and the rows under it (at least one), a
/// column a header; no filter buttons and no table style.
fn table_xml(headers: &[String], points: usize) -> Vec<u8> {
    let last = format!("{}{}", column(headers.len().saturating_sub(1)), points.max(1).saturating_add(1));
    let mut t = W::new();
    t.open(
        "table",
        &[("xmlns", NS_SHEET), ("id", "1"), ("name", "Table1"), ("displayName", "Table1"), ("ref", &format!("A1:{last}")), ("totalsRowShown", "0")],
    );
    t.open("tableColumns", &[("count", &headers.len().to_string())]);
    for (i, h) in headers.iter().enumerate() {
        t.empty("tableColumn", &[("id", &i.saturating_add(1).to_string()), ("name", h)]);
    }
    t.close("tableColumns");
    t.close("table");
    t.into_bytes()
}

/// A chart part as [`chart_xml`] writes it, without its `c:externalData` (for a chart written
/// without its workbook).
pub(crate) fn without_workbook(xml: &str) -> String {
    match (xml.find("<c:externalData"), xml.find("</c:externalData>")) {
        (Some(a), Some(b)) if b > a => {
            let end = b + "</c:externalData>".len();
            format!("{}{}", xml.get(..a).unwrap_or_default(), xml.get(end..).unwrap_or_default())
        }
        _ => xml.to_string(),
    }
}

/// SpreadsheetML's main namespace.
const NS_SHEET: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A value as the shortest text that reads back as the same number.
fn num(v: f64) -> String {
    if v == 0.0 { "0".to_string() } else { format!("{v}") }
}

/// How an axis looks: its labels shown (and their text, turned 45° up when `rotate`), its line
/// shown in its colour (`None`: near black), its major tick marks shown.
#[derive(Clone, Copy)]
struct AxisLook<'a> {
    labels: bool,
    line: bool,
    ticks: bool,
    color: Option<&'a str>,
    style: &'a TextStyle,
    rotate: bool,
}

/// The category axis's look (a scatter chart's x axis).
fn cat_look(spec: &ChartSpec) -> AxisLook<'_> {
    AxisLook {
        labels: spec.cat_labels,
        line: spec.cat_axis,
        ticks: spec.cat_ticks && spec.cat_axis,
        color: spec.cat_axis_color.as_deref(),
        style: &spec.cat_labels_style,
        rotate: spec.cat_label_pos == CatLabelPos::Rotate,
    }
}

/// The value axis's look (a scatter chart's y axis).
fn val_look(spec: &ChartSpec) -> AxisLook<'_> {
    AxisLook {
        labels: spec.val_labels,
        line: spec.val_axis,
        ticks: spec.val_ticks && spec.val_axis,
        color: spec.val_axis_color.as_deref(),
        style: &spec.val_labels_style,
        rotate: false,
    }
}

fn category_axis(w: &mut W, (id, cross): (&str, &str), pos: &str, deleted: bool, look: AxisLook<'_>) {
    w.open("c:catAx", &[]);
    w.empty("c:axId", &[("val", id)]);
    w.open("c:scaling", &[]);
    w.empty("c:orientation", &[("val", "minMax")]);
    w.close("c:scaling");
    w.empty("c:delete", &[("val", if deleted { "1" } else { "0" })]);
    w.empty("c:axPos", &[("val", pos)]);
    w.empty("c:majorTickMark", &[("val", if look.ticks { "out" } else { "none" })]);
    w.empty("c:minorTickMark", &[("val", "none")]);
    w.empty("c:tickLblPos", &[("val", if look.labels { "nextTo" } else { "none" })]);
    axis_line(w, look.line, look.color);
    text_pr(w, look.style, look.rotate, false);
    w.empty("c:crossAx", &[("val", cross)]);
    w.empty("c:crosses", &[("val", "autoZero")]);
    w.empty("c:auto", &[("val", "1")]);
    w.empty("c:lblAlgn", &[("val", "ctr")]);
    w.empty("c:lblOffset", &[("val", "100")]);
    w.empty("c:noMultiLvlLbl", &[("val", "0")]);
    w.close("c:catAx");
}

/// An axis line (`c:spPr/a:ln`): thin, in `color` (else near black) when `shown`, else none
/// (`a:noFill`).
fn axis_line(w: &mut W, shown: bool, color: Option<&str>) {
    w.open("c:spPr", &[]);
    if shown {
        w.open("a:ln", &[("w", "9525")]);
        solid(w, color.unwrap_or(AXIS_LINE));
    } else {
        w.open("a:ln", &[]);
        w.empty("a:noFill", &[]);
    }
    w.close("a:ln");
    w.close("c:spPr");
}

/// A value axis, with its line ([`axis_line`]). The chart's value axis (`main`) has outward
/// major tick marks, its gridlines as the chart says (light grey, solid or dashed) and its
/// scaling (`c:min`, `c:max`, `c:majorUnit`); the other (scatter x) has no ticks or gridlines.
#[allow(clippy::too_many_arguments)]
fn value_axis(w: &mut W, id: &str, cross: &str, pos: &str, main: Option<&ChartSpec>, between: &str, look: AxisLook<'_>, at_max: bool) {
    w.open("c:valAx", &[]);
    w.empty("c:axId", &[("val", id)]);
    w.open("c:scaling", &[]);
    w.empty("c:orientation", &[("val", "minMax")]);
    if let Some(v) = main.and_then(|s| s.axis_max) {
        w.empty("c:max", &[("val", &num(v))]);
    }
    if let Some(v) = main.and_then(|s| s.axis_min) {
        w.empty("c:min", &[("val", &num(v))]);
    }
    w.close("c:scaling");
    w.empty("c:delete", &[("val", "0")]);
    w.empty("c:axPos", &[("val", pos)]);
    let grid = main.map(|s| s.gridlines).filter(|g| *g != Gridlines::None);
    if let Some(g) = grid {
        w.open("c:majorGridlines", &[]);
        w.open("c:spPr", &[]);
        w.open("a:ln", &[("w", "9525")]);
        solid(w, GRID_LINE);
        if g == Gridlines::Dashed {
            w.empty("a:prstDash", &[("val", "dash")]);
        }
        w.close("a:ln");
        w.close("c:spPr");
        w.close("c:majorGridlines");
    }
    // 100% stacked charts: the shares as percentages.
    if main.is_some_and(|s| s.kind.is_percent()) {
        w.empty("c:numFmt", &[("formatCode", "0%"), ("sourceLinked", "0")]);
    }
    w.empty("c:majorTickMark", &[("val", if look.ticks { "out" } else { "none" })]);
    w.empty("c:minorTickMark", &[("val", "none")]);
    w.empty("c:tickLblPos", &[("val", if look.labels { "nextTo" } else { "none" })]);
    axis_line(w, look.line, look.color);
    text_pr(w, look.style, look.rotate, false);
    w.empty("c:crossAx", &[("val", cross)]);
    // A secondary value axis sits at the far end of its (hidden) category axis.
    w.empty("c:crosses", &[("val", if at_max { "max" } else { "autoZero" })]);
    w.empty("c:crossBetween", &[("val", between)]);
    if let Some(v) = main.and_then(|s| s.axis_step) {
        w.empty("c:majorUnit", &[("val", &num(v))]);
    }
    w.close("c:valAx");
}

/// The chart a chart part holds, as the model: one chart type the model has (no combination, no
/// 100% stacking or stacked lines) whose every series has its values in the part. Markup the model has no place for (another program's styles, extensions, number formats) is left out: saving writes the chart as WordCraft does. `None` for any other chart.
pub(crate) fn spec_of(space: &El) -> Option<ChartSpec> {
    adopt(space).ok()
}

/// [`spec_of`], or why the chart part isn't one the model takes.
pub(crate) fn adopt(space: &El) -> Result<ChartSpec, &'static str> {
    let chart = space.child("c:chart").ok_or("no chart")?;
    let plot = chart.child("c:plotArea").ok_or("no plot area")?;
    let val = |e: &El, n: &str| e.child(n).and_then(|c| c.attr("val")).map(str::to_string);
    let types: Vec<&El> = plot.els().filter(|e| e.name.ends_with("Chart")).take(8).collect();
    let ty = *types.first().ok_or("no chart type")?;
    // The series with what they are drawn as (a line, combo charts) and on which axis.
    let (kind, members) = if types.len() > 1 { (ChartType::Combo, combo_members(plot, &types)?) } else { (kind_of(ty)?, all_members(ty)) };
    let xy = kind.is_xy();
    let sers: Vec<&El> = members.iter().map(|m| m.0).collect();
    // Every series has its values in the part (a literal, or the cache of its cells).
    let has_values =
        |s: &El| s.child(if xy { "c:yVal" } else { "c:val" }).is_some_and(|v| v.find("c:numCache").or_else(|| v.find("c:numLit")).is_some());
    if sers.is_empty() {
        return Err("no series");
    }
    if !sers.iter().all(|s| has_values(s)) {
        return Err("a series without its values in the part");
    }
    // More than the model holds can't be the same chart: don't build it to find out.
    let count = |s: &&El| s.find("c:ptCount").and_then(|c| c.attr("val")).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
    let longest = sers.iter().map(count).max().unwrap_or(0);
    if sers.len() > wordcraft_doc::chart::MAX_SERIES
        || longest > wordcraft_doc::chart::MAX_POINTS
        || sers.len().saturating_mul(longest) > wordcraft_doc::chart::MAX_CELLS
    {
        return Err("more series or points than the model holds");
    }
    let categories = sers.first().and_then(|s| s.child(if xy { "c:xVal" } else { "c:cat" })).map(literal).unwrap_or_default();
    let numbers = |e: Option<&El>| e.map(literal).unwrap_or_default().iter().map(|v| v.parse::<f64>().ok().filter(|v| v.is_finite())).collect();
    let combo = kind == ChartType::Combo;
    let series = members
        .iter()
        .map(|(s, line, secondary)| ChartSeries {
            name: s.child("c:tx").and_then(|t| t.find("c:v")).map(|v| v.text()).unwrap_or_default(),
            values: numbers(s.child(if xy { "c:yVal" } else { "c:val" })),
            sizes: if kind == ChartType::Bubble { numbers(s.child("c:bubbleSize")) } else { Vec::new() },
            kind: (combo && *line != SeriesKind::Column).then_some(*line),
            secondary: combo && *secondary,
        })
        .collect();
    let shown_title = chart.child("c:title").and_then(|t| t.find("a:t")).map(|t| t.text());
    // A hidden title's text, from our extension (only when no title is shown).
    let hidden_title = space
        .child("c:extLst")
        .and_then(|l| l.children("c:ext").find(|e| e.attr("uri") == Some(PALETTE_EXT)))
        .and_then(|e| e.child("?:hiddenTitle"))
        .map(|t| t.text());
    let title_visible = shown_title.is_some() || hidden_title.is_none();
    let title = shown_title.or(hidden_title);
    let background = plot.child("c:spPr").is_some_and(|p| p.child("a:solidFill").is_some());
    // Labels and lines of an axis (the category axis, or a scatter chart's x axis by its id):
    // shown unless `c:tickLblPos` is `none`, or its line has no fill.
    let shows = |ax: Option<&El>| match ax {
        Some(a) => (
            val(a, "c:tickLblPos").as_deref() != Some("none"),
            a.child("c:spPr").and_then(|p| p.child("a:ln")).is_none_or(|l| l.child("a:noFill").is_none()),
        ),
        None => (true, true),
    };
    // The axes: the category axis (a scatter chart's horizontal value axis) and the value axis (a
    // scatter chart's vertical one), whatever ids the file gives them.
    let across = |a: &&El| matches!(val(a, "c:axPos").as_deref(), Some("b" | "t"));
    let (cat_el, val_ax) = if xy {
        (plot.children("c:valAx").find(across), plot.children("c:valAx").find(|a| !across(a)))
    } else {
        (plot.els().find(|e| matches!(e.name.as_str(), "c:catAx" | "c:dateAx")), plot.children("c:valAx").next())
    };
    let (cat_labels, cat_axis) = shows(cat_el);
    let (val_labels, val_axis) = shows(val_ax);
    let legend = chart.child("c:legend").map(|l| match val(l, "c:legendPos").as_deref() {
        Some("l") => LegendPos::Left,
        Some("t") => LegendPos::Top,
        Some("b") => LegendPos::Bottom,
        _ => LegendPos::Right,
    });
    // Values shown on the chart type, or on every series (another program may set them there only).
    let shows_values = |d: Option<&El>| d.and_then(|d| d.child("c:showVal")).and_then(|v| v.attr("val")).is_some_and(|v| v == "1" || v == "true");
    let data_labels = shows_values(ty.child("c:dLbls")) || sers.iter().all(|s| shows_values(s.child("c:dLbls")));
    let first = sers.first();
    // Smooth lines and dots: of the (first line) series of a line, scatter-with-lines or combo chart.
    let first_line = members.iter().find(|m| m.1 == SeriesKind::Line).map(|m| m.0).or(first.copied());
    let lined = kind.is_line() || (combo && members.iter().any(|m| m.1 == SeriesKind::Line));
    let smooth = lined && first_line.and_then(|s| val(s, "c:smooth")).as_deref() == Some("1");
    let dotted = lined || matches!(kind, ChartType::ScatterLines | ChartType::ScatterSmooth);
    let markers = dotted && first_line.and_then(|s| s.child("c:marker")).and_then(|m| val(m, "c:symbol")).is_some_and(|v| v != "none");
    // Bars: of a column or bar chart, or a combo chart's columns.
    let first_bar = members.iter().find(|m| m.1 == SeriesKind::Column).map(|m| m.0);
    let bars = kind.is_bar() || (combo && first_bar.is_some());
    let bar_el = types.iter().find(|t| t.name == "c:barChart").copied().unwrap_or(ty);
    // Rounded when a column series has the rounded geometry (a waterfall's invisible base has none).
    let rounded_bars = bars
        && members
            .iter()
            .filter(|m| m.1 == SeriesKind::Column)
            .any(|m| m.0.child("c:spPr").and_then(|p| p.child("a:prstGeom")).and_then(|g| g.attr("prst")) == Some("roundRect"));
    // Clustered bars: a negative overlap is the gap between series (anything else is checked by
    // writing again).
    let series_gap = match (bars, val(bar_el, "c:overlap").and_then(|v| v.parse::<f64>().ok())) {
        (true, Some(o)) if o < 0.0 && matches!(kind, ChartType::Column | ChartType::Bar | ChartType::Combo) => -o,
        _ => 0.0,
    };
    // The palette from our own extension; the series' own colours are checked by writing again.
    let colors: Vec<String> = space
        .child("c:extLst")
        .and_then(|l| l.children("c:ext").find(|e| e.attr("uri") == Some(PALETTE_EXT)))
        .map(|e| e.children("a:srgbClr").take(MAX_COLORS + 1).filter_map(|c| c.attr("val")).map(str::to_string).collect())
        .unwrap_or_default();
    // Pie and doughnut charts have no axes: their gridlines stay the default.
    let gridlines = match val_ax.map(|a| a.child("c:majorGridlines")) {
        Some(None) => Gridlines::None,
        Some(Some(g)) if g.find("a:prstDash").is_some() => Gridlines::Dashed,
        Some(Some(_)) | None => Gridlines::Solid,
    };
    let scale = |n: &str| val_ax.and_then(|a| a.child("c:scaling")).and_then(|s| val(s, n)).and_then(|v| v.parse::<f64>().ok());
    let axis_step = val_ax.and_then(|a| val(a, "c:majorUnit")).and_then(|v| v.parse::<f64>().ok());
    // Our own palette, or else the series' own colours when every one has one (a chart saved by
    // another program, which drops our extension).
    let colors = if colors.is_empty() { series_colours(kind, &sers) } else { colors };
    // Text looks: each element's default run properties (the title: its run's).
    let def_rpr = def_rpr_of;
    let data_rpr = def_rpr(ty.child("c:dLbls")).or_else(|| first.and_then(|s| def_rpr(s.child("c:dLbls"))));
    let title_rpr = chart.child("c:title").and_then(|t| t.find("a:rPr").or_else(|| def_rpr(Some(t))));
    let data_labels_outline =
        data_rpr.and_then(|r| r.child("a:ln")).is_some_and(|l| l.child("a:noFill").is_none() && l.child("a:solidFill").is_some());
    let rotated = cat_el
        .and_then(|a| a.child("c:txPr"))
        .and_then(|t| t.child("a:bodyPr"))
        .and_then(|b| b.attr("rot"))
        .and_then(|r| r.parse::<i64>().ok())
        .is_some_and(|r| r != 0 && r.unsigned_abs() <= 5_400_000);
    // An axis line's colour, when it isn't the default.
    let axis_color = |ax: Option<&El>| {
        ax.and_then(|a| a.child("c:spPr"))
            .and_then(|p| p.child("a:ln"))
            .and_then(|l| l.child("a:solidFill"))
            .and_then(|f| f.child("a:srgbClr"))
            .and_then(|c| c.attr("val"))
            .and_then(wordcraft_doc::chart::hex_color)
            .filter(|c| c != AXIS_LINE)
    };
    // Tick marks: as written, unless the axis line is hidden (then none is written whatever was
    // asked: keep the default).
    let ticks = |ax: Option<&El>, shown: bool, default: bool| match ax.and_then(|a| val(a, "c:majorTickMark")) {
        Some(v) if shown => v != "none",
        _ => default,
    };
    let background_color = plot
        .child("c:spPr")
        .and_then(|p| p.child("a:solidFill"))
        .and_then(|f| f.child("a:srgbClr"))
        .and_then(|c| c.attr("val"))
        .and_then(wordcraft_doc::chart::hex_color)
        .filter(|c| c != "000000");
    let area = kind.is_area();
    // The series' line width, when it isn't the default (a line series' line, an area's top edge).
    let line_width = (if combo { members.iter().find(|m| m.1 == SeriesKind::Line).map(|m| m.0) } else { first.copied() })
        .and_then(|s| s.child("c:spPr"))
        .and_then(|p| p.child("a:ln"))
        .and_then(|l| l.attr("w"))
        .and_then(|w| w.parse::<f64>().ok())
        .filter(|w| w.is_finite())
        .map(|w| (w / 12_700.0) as f32)
        .filter(|w| match kind {
            k if k.is_line()
                || matches!(
                    k,
                    ChartType::Combo
                        | ChartType::ScatterLines
                        | ChartType::ScatterSmooth
                        | ChartType::Radar
                        | ChartType::RadarMarkers
                        | ChartType::RadarFilled
                ) =>
            {
                (w - LINE_SERIES_W).abs() > 0.01
            }
            _ if area => (w - AREA_EDGE_W).abs() > 0.01,
            _ => false,
        });
    // An area's opacity: its gradient's top alpha.
    let area_opacity = first
        .filter(|_| area)
        .and_then(|s| s.child("c:spPr"))
        .and_then(|p| p.child("a:gradFill"))
        .and_then(|g| g.find("a:gs"))
        .and_then(|g| g.find("a:alpha"))
        .and_then(|a| a.attr("val"))
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .map_or(wordcraft_doc::chart::DEFAULT_AREA_OPACITY, |v| (v / 100_000.0) as f32);
    // A bubble's fill opacity: its solid fill's alpha (the series', else its first point's).
    let alpha_of = |sp: Option<&El>| {
        sp.and_then(|p| p.child("a:solidFill"))
            .and_then(|f| f.find("a:alpha"))
            .and_then(|a| a.attr("val"))
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| v.is_finite())
            .map(|v| (v / 100_000.0) as f32)
    };
    let bubble_opacity = first
        .filter(|_| kind == ChartType::Bubble)
        .and_then(|s| alpha_of(s.child("c:spPr")).or_else(|| s.child("c:dPt").and_then(|d| alpha_of(d.child("c:spPr")))))
        .unwrap_or(1.0);
    // Our extension's look: an area's smooth edge and dots, the chart's opacity.
    let look = ext_look_el(space);
    let look_on = |n: &str| look.and_then(|l| l.attr(n)) == Some("1");
    let opacity = look.and_then(|l| l.attr("opacity")).and_then(|v| v.parse::<f32>().ok()).unwrap_or(1.0);
    let spec = ChartSpec {
        kind,
        categories,
        series,
        title,
        legend,
        // Not in the format: our extension's, else square (another program drops it).
        legend_shape: if round_legend_keys(space) { LegendShape::Circle } else { LegendShape::Square },
        data_labels,
        colors,
        smooth: smooth || (area && look_on("smooth")),
        markers: markers || (area && look_on("markers")),
        rounded_bars,
        series_gap,
        gridlines,
        axis_min: scale("c:min"),
        axis_max: scale("c:max"),
        axis_step,
        title_visible,
        background,
        cat_labels,
        cat_axis,
        val_labels,
        val_axis,
        legend_style: text_style(chart.child("c:legend").and_then(|l| def_rpr(Some(l)))),
        data_labels_style: text_style(data_rpr),
        cat_labels_style: text_style(def_rpr(cat_el)),
        val_labels_style: text_style(def_rpr(val_ax)),
        title_style: text_style(title_rpr),
        data_labels_outline,
        cat_label_pos: if rotated { CatLabelPos::Rotate } else { CatLabelPos::Auto },
        cat_axis_color: axis_color(cat_el),
        val_axis_color: axis_color(val_ax),
        cat_ticks: ticks(cat_el, cat_axis, false),
        val_ticks: ticks(val_ax, val_axis, true),
        background_color,
        line_width,
        area_opacity,
        opacity,
        bubble_opacity: if kind == ChartType::Bubble { bubble_opacity } else { wordcraft_doc::chart::DEFAULT_BUBBLE_OPACITY },
        hole_size: if kind == ChartType::Doughnut {
            val(ty, "c:holeSize").and_then(|v| v.parse::<u32>().ok()).unwrap_or(wordcraft_doc::chart::DEFAULT_HOLE_SIZE)
        } else {
            wordcraft_doc::chart::DEFAULT_HOLE_SIZE
        },
        slice_gap: if kind.is_round() { slice_gap_of(first.copied()) } else { 0.0 },
        waterfall_total: false,
        // A radar's series filled with a circular gradient.
        radar_gradient: kind.is_radar()
            && first.and_then(|s| s.child("c:spPr")).and_then(|p| p.child("a:gradFill")).and_then(|g| g.child("a:path")).is_some(),
        // Pie of pie: the points the second plot takes (by position; another split is read as
        // the default).
        split_pos: if kind.is_of_pie() && val(ty, "c:splitType").as_deref().is_none_or(|t| t == "pos") {
            val(ty, "c:splitPos").and_then(|v| v.parse::<u32>().ok()).unwrap_or(wordcraft_doc::chart::DEFAULT_SPLIT)
        } else {
            wordcraft_doc::chart::DEFAULT_SPLIT
        },
    }
    .sanitized();
    // A waterfall or funnel written as a stacked chart (our extension says which); a line radar
    // written filled for its gradient.
    let drawn_as = look.and_then(|l| l.attr("kind")).and_then(ChartType::from_id);
    let spec = match drawn_as {
        Some(k @ (ChartType::Radar | ChartType::RadarMarkers)) if spec.kind == ChartType::RadarFilled => ChartSpec { kind: k, ..spec },
        _ => spec,
    };
    if let Some(k) = drawn_as.filter(|k| matches!(k, ChartType::Waterfall | ChartType::Funnel)) {
        return rebuilt(&spec, k, look_on("total")).ok_or("a waterfall or funnel that isn't as WordCraft writes it");
    }
    Ok(spec)
}

/// A series element, what it is drawn as (combo charts; columns elsewhere) and whether it is on
/// the secondary axis.
type Member<'a> = (&'a El, SeriesKind, bool);

/// The chart type a single chart type element is, as the model has it.
fn kind_of(ty: &El) -> Result<ChartType, &'static str> {
    let val = |e: &El, n: &str| e.child(n).and_then(|c| c.attr("val")).map(str::to_string);
    let grouping = val(ty, "c:grouping");
    let stack = |percent, stacked, plain| match grouping.as_deref() {
        Some("percentStacked") => percent,
        Some("stacked") => stacked,
        _ => plain,
    };
    let first = ty.child("c:ser");
    Ok(match ty.name.as_str() {
        "c:barChart" if val(ty, "c:barDir").as_deref() == Some("bar") => stack(ChartType::BarPercent, ChartType::BarStacked, ChartType::Bar),
        "c:barChart" => stack(ChartType::ColumnPercent, ChartType::ColumnStacked, ChartType::Column),
        "c:lineChart" => stack(ChartType::LinePercent, ChartType::LineStacked, ChartType::Line),
        "c:areaChart" => stack(ChartType::AreaPercent, ChartType::AreaStacked, ChartType::Area),
        "c:pieChart" => ChartType::Pie,
        "c:doughnutChart" => ChartType::Doughnut,
        "c:bubbleChart" => ChartType::Bubble,
        "c:ofPieChart" if val(ty, "c:ofPieType").as_deref() == Some("bar") => ChartType::BarOfPie,
        "c:ofPieChart" => ChartType::PieOfPie,
        // A scatter series without a line is markers only; else straight or smooth lines.
        "c:scatterChart" => {
            let no_line = first.and_then(|s| s.child("c:spPr")).and_then(|p| p.child("a:ln")).is_some_and(|l| l.child("a:noFill").is_some());
            match (no_line, first.and_then(|s| val(s, "c:smooth")).as_deref()) {
                (true, _) => ChartType::Scatter,
                (false, Some("1" | "true")) => ChartType::ScatterSmooth,
                _ => ChartType::ScatterLines,
            }
        }
        // Filled, else with markers when the series have a marker symbol other than none.
        "c:radarChart" => match val(ty, "c:radarStyle").as_deref() {
            Some("filled") => ChartType::RadarFilled,
            Some("marker") if first.and_then(|s| s.child("c:marker")).and_then(|m| val(m, "c:symbol")).is_none_or(|v| v != "none") => {
                ChartType::RadarMarkers
            }
            _ => ChartType::Radar,
        },
        _ => return Err("a chart type the model doesn't have"),
    })
}

/// The series of one chart type element.
fn all_members(ty: &El) -> Vec<Member<'_>> {
    ty.children("c:ser").take(wordcraft_doc::chart::MAX_SERIES + 1).map(|s| (s, SeriesKind::Column, false)).collect()
}

/// The series of a combo chart: clustered column, standard line and standard or stacked area
/// groups only (at most one of each on each value axis), in their `c:order`; a group whose value
/// axis isn't the plot's first is on the secondary axis.
fn combo_members<'a>(plot: &'a El, types: &[&'a El]) -> Result<Vec<Member<'a>>, &'static str> {
    const NOT: &str = "a combination the model doesn't have (only clustered columns, lines and areas)";
    let val = |e: &El, n: &str| e.child(n).and_then(|c| c.attr("val")).map(str::to_string);
    let primary = plot.children("c:valAx").next().and_then(|a| val(a, "c:axId"));
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<(i64, Member<'a>)> = Vec::new();
    for t in types {
        let grouping = val(t, "c:grouping");
        let line = match t.name.as_str() {
            "c:barChart" if val(t, "c:barDir").as_deref() != Some("bar") && grouping.as_deref().is_none_or(|g| g == "clustered") => {
                SeriesKind::Column
            }
            "c:lineChart" if grouping.as_deref().is_none_or(|g| g == "standard") => SeriesKind::Line,
            "c:areaChart" if grouping.as_deref().is_none_or(|g| g == "standard" || g == "stacked") => SeriesKind::Area,
            _ => return Err(NOT),
        };
        let val_id = t.children("c:axId").nth(1).and_then(|a| a.attr("val")).map(str::to_string);
        let secondary = val_id.is_some() && val_id != primary;
        if !seen.insert((line, secondary)) {
            return Err(NOT);
        }
        for s in t.children("c:ser").take(wordcraft_doc::chart::MAX_SERIES + 1) {
            let order = val(s, "c:order").and_then(|v| v.parse::<i64>().ok()).unwrap_or(i64::MAX);
            out.push((order, (s, line, secondary)));
        }
    }
    out.sort_by_key(|m| m.0);
    Ok(out.into_iter().map(|m| m.1).take(wordcraft_doc::chart::MAX_SERIES + 1).collect())
}

/// An element's default run properties (`c:txPr`'s `a:defRPr`).
pub(crate) fn def_rpr_of(e: Option<&El>) -> Option<&El> {
    e.and_then(|e| e.child("c:txPr")).and_then(|t| t.find("a:defRPr"))
}

/// A text look from run properties (`a:defRPr`, `a:rPr`): size, bold, an `rrggbb` colour other
/// than the chart text's, the Latin font. What isn't given (or is the default) stays unset.
fn text_style(rpr: Option<&El>) -> TextStyle {
    let Some(r) = rpr else { return TextStyle::default() };
    TextStyle {
        font: r.child("a:latin").and_then(|f| f.attr("typeface")).filter(|f| !f.is_empty() && !f.starts_with('+')).map(str::to_string),
        size: r.attr("sz").and_then(|s| s.parse::<f32>().ok()).filter(|s| s.is_finite()).map(|s| s / 100.0),
        color: r
            .child("a:solidFill")
            .and_then(|f| f.child("a:srgbClr"))
            .and_then(|c| c.attr("val"))
            .and_then(wordcraft_doc::chart::hex_color)
            .filter(|c| !c.eq_ignore_ascii_case(CHART_TEXT)),
        bold: matches!(r.attr("b"), Some("1" | "true")).then_some(true),
    }
}

/// The colours of a chart's series as a palette, when every series (pie and doughnut: every
/// slice of the first series) has its own `rrggbb` colour; empty (the theme's accents) else.
fn series_colours(kind: ChartType, sers: &[&El]) -> Vec<String> {
    let srgb = |fill: Option<&El>| fill.and_then(|f| f.find("a:srgbClr")).and_then(|c| c.attr("val")).and_then(wordcraft_doc::chart::hex_color);
    // The fill of a shape's properties: solid, or a gradient's first stop.
    let fill_of = |sp: Option<&El>| {
        srgb(sp.and_then(|p| p.child("a:solidFill").or_else(|| p.child("a:gradFill").and_then(|g| g.child("a:gsLst")).and_then(|l| l.child("a:gs")))))
    };
    let colour = |s: &El| -> Option<String> {
        let line = || srgb(s.child("c:spPr").and_then(|p| p.child("a:ln")).and_then(|l| l.child("a:solidFill")));
        match kind {
            k if k.is_line() || matches!(k, ChartType::ScatterLines | ChartType::ScatterSmooth | ChartType::Radar | ChartType::RadarMarkers) => {
                line()
            }
            ChartType::Scatter => fill_of(s.child("c:marker").and_then(|m| m.child("c:spPr"))),
            // A combo chart's columns and areas by their fill, its lines by their line.
            _ => fill_of(s.child("c:spPr")).or_else(line),
        }
    };
    let all = |v: Vec<Option<String>>| -> Vec<String> {
        if !v.is_empty() && v.iter().all(Option::is_some) { v.into_iter().flatten().take(MAX_COLORS).collect() } else { Vec::new() }
    };
    if kind.is_round() || (kind == ChartType::Bubble && sers.len() == 1 && sers.first().is_some_and(|s| s.child("c:dPt").is_some())) {
        let Some(first) = sers.first() else { return Vec::new() };
        let mut pts: Vec<(usize, Option<String>)> = first
            .children("c:dPt")
            .take(MAX_COLORS)
            .filter_map(|d| Some((d.child("c:idx")?.attr("val")?.parse::<usize>().ok()?, fill_of(d.child("c:spPr")))))
            .collect();
        pts.sort_by_key(|p| p.0);
        // Slices in order from the first, with nothing missing.
        if pts.iter().enumerate().any(|(i, p)| p.0 != i) {
            return Vec::new();
        }
        return all(pts.into_iter().map(|p| p.1).collect());
    }
    all(sers.iter().take(MAX_COLORS).map(|s| colour(s)).collect())
}

/// The points of a `c:strLit` / `c:numLit` (or a cache) as texts by index, `ptCount` long.
fn literal(e: &El) -> Vec<String> {
    // A literal, or the cache of the cells a reference names (the caches are what the chart shows;
    // the workbook isn't read).
    let lit = e.els().find(|c| matches!(c.name.as_str(), "c:strLit" | "c:numLit")).or_else(|| {
        e.els()
            .filter(|c| matches!(c.name.as_str(), "c:strRef" | "c:numRef"))
            .find_map(|r| r.els().find(|c| matches!(c.name.as_str(), "c:strCache" | "c:numCache")))
    });
    let Some(lit) = lit else { return Vec::new() };
    let n = lit
        .child("c:ptCount")
        .and_then(|c| c.attr("val"))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
        .min(wordcraft_doc::chart::MAX_POINTS + 1);
    let mut out = vec![String::new(); n];
    for p in lit.children("c:pt") {
        if let Some(slot) = p.attr("idx").and_then(|i| i.parse::<usize>().ok()).and_then(|i| out.get_mut(i)) {
            *slot = p.child("c:v").map(|v| v.text()).unwrap_or_default();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::THEME_COLORS;
    use wordcraft_doc::graphic::PathSeg;

    fn parse(spec: &ChartSpec) -> El {
        xml::parse(chart_xml(spec).as_bytes()).unwrap()
    }

    /// Children of `e` by name, in order.
    fn names(e: &El) -> Vec<&str> {
        e.els().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn a_chart_past_the_limits_is_written_within_them() {
        // A model deserialised as is (a `.json` document) never went through `sanitize`, and every
        // series repeats every category: 300 series of 4,500 points wrote 2.7 million points (#3648).
        use wordcraft_doc::chart::MAX_CELLS;
        let spec = ChartSpec {
            kind: ChartType::Column,
            categories: (0..4500).map(|i| i.to_string()).collect(),
            series: (0..300).map(|i| ChartSeries { name: format!("S{i}"), values: vec![Some(1.0); 4500], ..Default::default() }).collect(),
            title: None,
            legend: None,
            data_labels: false,
            ..Default::default()
        };
        let x = chart_xml(&spec);
        let series = x.matches("<c:ser>").count();
        let points = x.matches("<c:pt ").count();
        assert!((1..=MAX_SERIES).contains(&series), "{series} series");
        // Categories and values, and a name each series.
        assert!(points <= 2 * MAX_CELLS + MAX_SERIES, "{points} points");
        assert!(!x.contains(&format!("<c:pt idx=\"{MAX_POINTS}\"")), "past the point limit");
    }

    #[test]
    fn every_type_reads_back_as_itself() {
        for kind in ChartType::ALL {
            let mut spec = ChartSpec::sample(kind);
            spec.title = Some("Sales & <costs>".into());
            spec.data_labels = true;
            spec.legend = Some(LegendPos::Right);
            spec.series[0].values[1] = None;
            spec.categories[2] = String::new();
            let spec = spec.sanitized();
            assert_eq!(spec_of(&parse(&spec)).as_ref(), Some(&spec), "{kind:?}");
            assert!(!chart_items(&spec, &THEME_COLORS, 300.0, 200.0).is_empty(), "{kind:?} draws");
        }
        let plain = ChartSpec { legend: None, ..ChartSpec::sample(ChartType::Line) };
        assert_eq!(spec_of(&parse(&plain)), Some(plain));
    }

    /// A chart with every style field set.
    fn styled(kind: ChartType) -> ChartSpec {
        ChartSpec {
            colors: vec!["111111".into(), "777777".into()],
            smooth: true,
            markers: true,
            rounded_bars: true,
            series_gap: 25.0,
            gridlines: Gridlines::Dashed,
            axis_min: Some(-1.5),
            axis_max: Some(12.0),
            axis_step: Some(2.5),
            ..ChartSpec::sample(kind)
        }
        .sanitized()
    }

    fn plot_of(space: &El) -> &El {
        space.child("c:chart").unwrap().child("c:plotArea").unwrap()
    }

    fn srgb(e: &El) -> Vec<&str> {
        let mut out = Vec::new();
        fn walk<'a>(e: &'a El, out: &mut Vec<&'a str>) {
            for c in e.els() {
                if c.name == "a:srgbClr" {
                    out.extend(c.attr("val"));
                }
                walk(c, out);
            }
        }
        walk(e, &mut out);
        out
    }

    #[test]
    fn style_fields_are_written_as_the_spec_says() {
        let line = parse(&styled(ChartType::Line));
        let plot = plot_of(&line);
        let sers: Vec<&El> = plot.child("c:lineChart").unwrap().children("c:ser").collect();
        // Colours in turn: the third series takes the first colour again.
        let lines: Vec<&str> = sers.iter().map(|s| s.child("c:spPr").unwrap().find("a:srgbClr").unwrap().attr("val").unwrap()).collect();
        assert_eq!(lines, ["111111", "777777", "111111"]);
        let s0 = sers[0];
        assert_eq!(names(s0), ["c:idx", "c:order", "c:tx", "c:spPr", "c:marker", "c:cat", "c:val", "c:smooth"]);
        assert!(s0.child("c:spPr").unwrap().child("a:ln").unwrap().child("a:solidFill").is_some());
        assert_eq!(s0.child("c:smooth").unwrap().attr("val"), Some("1"));
        let marker = s0.child("c:marker").unwrap();
        assert_eq!(names(marker), ["c:symbol", "c:size", "c:spPr"]);
        assert_eq!(marker.child("c:symbol").unwrap().attr("val"), Some("circle"));
        let val_ax = plot.child("c:valAx").unwrap();
        let scaling = val_ax.child("c:scaling").unwrap();
        assert_eq!(names(scaling), ["c:orientation", "c:max", "c:min"]);
        assert_eq!((scaling.child("c:min").unwrap().attr("val"), scaling.child("c:max").unwrap().attr("val")), (Some("-1.5"), Some("12")));
        assert_eq!(names(val_ax).last(), Some(&"c:majorUnit"));
        assert_eq!(val_ax.child("c:majorUnit").unwrap().attr("val"), Some("2.5"));
        let dash = val_ax.child("c:majorGridlines").unwrap().find("a:prstDash").unwrap();
        assert_eq!(dash.attr("val"), Some("dash"));
        // The palette is kept as given, after the chart area's shape properties.
        assert_eq!(names(&line).last(), Some(&"c:extLst"));
        assert_eq!(srgb(line.child("c:extLst").unwrap()), ["111111", "777777"]);

        // No gridlines; no markers: symbol none; not smooth.
        let plain = parse(&ChartSpec { gridlines: Gridlines::None, ..ChartSpec::sample(ChartType::Line) });
        let plot = plot_of(&plain);
        assert!(plot.child("c:valAx").unwrap().child("c:majorGridlines").is_none());
        let s0 = plot.child("c:lineChart").unwrap().child("c:ser").unwrap();
        assert_eq!(s0.find("c:symbol").unwrap().attr("val"), Some("none"));
        assert_eq!(s0.child("c:smooth").unwrap().attr("val"), Some("0"));
        let series_colours = srgb(plot.child("c:lineChart").unwrap());
        assert!(plain.child("c:extLst").is_none() && series_colours.is_empty(), "no palette: theme colours");

        // Stacked area.
        let area = parse(&styled(ChartType::AreaStacked));
        let ac = plot_of(&area).child("c:areaChart").unwrap();
        assert_eq!(ac.child("c:grouping").unwrap().attr("val"), Some("stacked"));
        assert_eq!(names(ac.child("c:ser").unwrap()), ["c:idx", "c:order", "c:tx", "c:spPr", "c:cat", "c:val"]);
        assert!(ac.child("c:ser").unwrap().child("c:spPr").unwrap().child("a:gradFill").is_some());

        // Pie slices take the palette in turn.
        let pie = parse(&styled(ChartType::Pie));
        let p0 = plot_of(&pie).child("c:pieChart").unwrap().child("c:ser").unwrap();
        assert_eq!(names(p0), ["c:idx", "c:order", "c:tx", "c:dPt", "c:dPt", "c:dPt", "c:dPt", "c:cat", "c:val"]);
        assert_eq!(srgb(p0), ["111111", "777777", "111111", "777777"]);

        // Scatter markers take the palette.
        let sc = parse(&styled(ChartType::Scatter));
        let s0 = plot_of(&sc).child("c:scatterChart").unwrap().child("c:ser").unwrap();
        assert_eq!(names(s0), ["c:idx", "c:order", "c:tx", "c:spPr", "c:marker", "c:xVal", "c:yVal", "c:smooth"]);
    }

    #[test]
    fn bar_style_fields_are_written_as_the_spec_says() {
        // Bars fill with the palette.
        let bar = parse(&styled(ChartType::Column));
        let b0 = plot_of(&bar).child("c:barChart").unwrap().child("c:ser").unwrap();
        assert_eq!(names(b0), ["c:idx", "c:order", "c:tx", "c:spPr", "c:invertIfNegative", "c:cat", "c:val"]);
        let sp = b0.child("c:spPr").unwrap();
        assert_eq!(names(sp), ["a:prstGeom", "a:solidFill"], "geometry before the fill");
        assert_eq!(sp.child("a:prstGeom").unwrap().attr("prst"), Some("roundRect"));
        assert_eq!(names(sp.child("a:prstGeom").unwrap()), ["a:avLst"]);
        let plain = parse(&ChartSpec { rounded_bars: true, ..ChartSpec::sample(ChartType::BarStacked) });
        let sp = plot_of(&plain).child("c:barChart").unwrap().child("c:ser").unwrap().child("c:spPr").unwrap();
        assert_eq!(names(sp), ["a:prstGeom"], "rounded without a palette: geometry only");
        assert!(!chart_xml(&styled(ChartType::Line)).contains("prstGeom"), "only bar charts");
        // The series gap: a negative overlap after the gap width, clustered bars only.
        let bc = plot_of(&bar).child("c:barChart").unwrap();
        let tail: Vec<&str> = names(bc).into_iter().skip_while(|n| *n != "c:gapWidth").collect();
        assert_eq!(tail, ["c:gapWidth", "c:overlap", "c:axId", "c:axId"]);
        assert_eq!(bc.child("c:overlap").unwrap().attr("val"), Some("-25"));
        let st = parse(&styled(ChartType::ColumnStacked));
        assert_eq!(plot_of(&st).child("c:barChart").unwrap().child("c:overlap").unwrap().attr("val"), Some("100"));
        assert!(!chart_xml(&ChartSpec::sample(ChartType::Column)).contains("c:overlap"), "no gap: no overlap");
    }

    #[test]
    fn a_combo_tells_line_labels_from_column_labels() {
        // Columns near-black, a line in 444444 running under their tops, labels on.
        let mut spec =
            ChartSpec { data_labels: true, colors: vec!["111111".into(), "7d7d7d".into(), "444444".into()], ..ChartSpec::sample(ChartType::Combo) };
        if let Some(line) = spec.series.get_mut(2) {
            line.secondary = false;
            line.values = vec![Some(0.5), Some(0.6), Some(0.8), Some(0.7)];
        }
        let it = chart_items(&spec, &THEME_COLORS, 400.0, 300.0);
        let text_of = |t: &str| {
            it.iter().find_map(|i| match i {
                GraphicItem::Text { text, rect, color, .. } if text == t => Some((*rect, *color)),
                _ => None,
            })
        };
        // A column's label: white, inside its bar.
        let (_, c) = text_of("4.1").unwrap();
        assert_eq!(c, Rgb::WHITE);
        // The line's labels: dark text on a white chip outlined in the line's colour, just above
        // their points (not lifted over the columns).
        let chips: Vec<[f32; 4]> = it
            .iter()
            .filter_map(|i| match i {
                GraphicItem::Path { segs, fill: Some(Rgb::WHITE), stroke: Some(Rgb(0x44, 0x44, 0x44)), .. } => {
                    let ys: Vec<f32> =
                        segs.iter().filter_map(|s| if let PathSeg::Line(_, y) | PathSeg::Move(_, y) = *s { Some(y) } else { None }).collect();
                    let xs: Vec<f32> =
                        segs.iter().filter_map(|s| if let PathSeg::Line(x, _) | PathSeg::Move(x, _) = *s { Some(x) } else { None }).collect();
                    Some([
                        xs.iter().copied().fold(f32::MAX, f32::min),
                        ys.iter().copied().fold(f32::MAX, f32::min),
                        xs.iter().copied().fold(f32::MIN, f32::max),
                        ys.iter().copied().fold(f32::MIN, f32::max),
                    ])
                }
                _ => None,
            })
            .collect();
        assert_eq!(chips.len(), 4, "a chip each");
        for v in ["0.5", "0.6", "0.8", "0.7"] {
            let (r, c) = text_of(v).unwrap();
            assert_eq!(c, Rgb(0x1F, 0x1F, 0x1F), "{v}: dark");
            let (cx, cy) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
            assert!(chips.iter().any(|b| cx > b[0] && cx < b[2] && cy > b[1] && cy < b[3]), "{v} on its chip");
            assert!(r[1] > 150.0, "{v} by its point, not over the columns: {r:?}");
        }
        // An area's top edge is drawn after the columns (its fill before them).
        let mut area = ChartSpec { colors: spec.colors.clone(), ..ChartSpec::sample(ChartType::Combo) };
        area.series.truncate(2);
        if let Some(a) = area.series.get_mut(1) {
            a.kind = Some(wordcraft_doc::chart::SeriesKind::Area);
        }
        let it = chart_items(&area, &THEME_COLORS, 400.0, 300.0);
        let last_bar = it.iter().rposition(|i| matches!(i, GraphicItem::Path { fill: Some(Rgb(0x11, 0x11, 0x11)), .. })).unwrap();
        let edge = it.iter().rposition(|i| matches!(i, GraphicItem::Path { fill: None, stroke: Some(Rgb(0x7d, 0x7d, 0x7d)), .. })).unwrap();
        assert!(edge > last_bar, "the area's edge in front of the columns");
    }

    #[test]
    fn a_doughnut_hole_and_a_gap_between_slices_round_trip() {
        let spec =
            ChartSpec { hole_size: 70, slice_gap: 2.0, colors: vec!["111111".into(), "222222".into()], ..ChartSpec::sample(ChartType::Doughnut) };
        let space = parse(&spec);
        let dn = plot_of(&space).child("c:doughnutChart").unwrap();
        assert_eq!(dn.child("c:holeSize").unwrap().attr("val"), Some("70"));
        let ser = dn.child("c:ser").unwrap();
        assert_eq!(ser.child("c:spPr").unwrap().child("a:ln").unwrap().attr("w"), Some("25400"));
        assert!(ser.children("c:dPt").all(|d| d.child("c:spPr").unwrap().child("a:ln").is_some()), "each slice too");
        let back = spec_of(&space).unwrap();
        assert_eq!((back.hole_size, back.slice_gap), (70, 2.0));
        // The gap is drawn: white outlines over the slices.
        let it = chart_items(&spec, &THEME_COLORS, 300.0, 200.0);
        assert!(it.iter().any(|i| matches!(i, GraphicItem::Path { stroke: Some(Rgb(255, 255, 255)), fill: None, .. })));
        // No gap: no outline.
        let pie = parse(&ChartSpec::sample(ChartType::Pie));
        assert!(plot_of(&pie).child("c:pieChart").unwrap().child("c:ser").unwrap().child("c:spPr").is_none());
    }

    #[test]
    fn bubbles_are_see_through_and_one_series_takes_the_palette_per_bubble() {
        // The fill's alpha and a solid outline; read back as the bubble opacity.
        let spec = ChartSpec { bubble_opacity: 0.45, ..ChartSpec::sample(ChartType::Bubble) };
        let space = parse(&spec);
        let ser = plot_of(&space).child("c:bubbleChart").unwrap().child("c:ser").unwrap();
        let sp = ser.child("c:spPr").unwrap();
        assert_eq!(sp.find("a:alpha").unwrap().attr("val"), Some("45000"));
        assert!(sp.child("a:ln").is_some(), "a solid outline");
        assert_eq!(spec_of(&space).map(|s| s.bubble_opacity), Some(0.45));
        // Opaque: no alpha, no outline.
        let opaque = parse(&ChartSpec { bubble_opacity: 1.0, ..ChartSpec::sample(ChartType::Bubble) });
        let sp = plot_of(&opaque).child("c:bubbleChart").unwrap().child("c:ser").unwrap().child("c:spPr").unwrap();
        assert!(sp.find("a:alpha").is_none() && sp.child("a:ln").is_none());
        // One series with a palette: each bubble its colour, the palette reads back.
        let mut one = ChartSpec { colors: vec!["111111".into(), "222222".into(), "333333".into()], ..ChartSpec::sample(ChartType::Bubble) };
        one.series.truncate(1);
        let space = parse(&one);
        let pts: Vec<_> = plot_of(&space).child("c:bubbleChart").unwrap().child("c:ser").unwrap().children("c:dPt").collect();
        assert_eq!(pts.len(), one.points());
        assert_eq!(pts[1].find("a:srgbClr").unwrap().attr("val"), Some("222222"));
        assert_eq!(pts[3].find("a:srgbClr").unwrap().attr("val"), Some("111111"), "in turn");
        assert_eq!(spec_of(&space).map(|s| s.colors), Some(one.colors.clone()));
        // Two series: a colour each, no points.
        let two = ChartSpec { colors: one.colors.clone(), ..ChartSpec::sample(ChartType::Bubble) };
        assert!(!chart_xml(&two).contains("c:dPt"));
    }

    #[test]
    fn a_funnel_is_drawn_without_a_value_scale() {
        // As a funnel looks: stages nearly touching, the widest filling the plot, no value labels,
        // axis line, ticks or gridlines, square ends, whatever the model's axis settings.
        let spec = ChartSpec { rounded_bars: true, gridlines: Gridlines::Dashed, axis_max: Some(5000.0), ..ChartSpec::sample(ChartType::Funnel) };
        let space = parse(&spec);
        let plot = plot_of(&space);
        let bc = plot.child("c:barChart").unwrap();
        assert_eq!(bc.child("c:gapWidth").unwrap().attr("val"), Some("10"));
        let va = plot.child("c:valAx").unwrap();
        assert!(va.child("c:majorGridlines").is_none());
        assert_eq!(va.child("c:tickLblPos").unwrap().attr("val"), Some("none"));
        assert_eq!(va.child("c:majorTickMark").unwrap().attr("val"), Some("none"));
        let top = spec.series[0].values.iter().flatten().fold(0.0f64, |a, v| a.max(*v));
        assert_eq!(va.child("c:scaling").unwrap().child("c:max").unwrap().attr("val"), Some(num(top).as_str()));
        assert!(!chart_xml(&spec).contains("roundRect"));
        // Each stage takes the palette in turn, the first (top) stage the first colour: written
        // bottom-up, so the last point has it.
        let coloured = ChartSpec { colors: vec!["111111".into(), "222222".into(), "333333".into()], ..ChartSpec::sample(ChartType::Funnel) };
        let cs = parse(&coloured);
        let sers: Vec<_> = plot_of(&cs).child("c:barChart").unwrap().children("c:ser").collect();
        assert!(sers[0].child("c:dPt").is_none(), "the hidden offset has no points");
        let pts: Vec<_> = sers[1].children("c:dPt").collect();
        let n = coloured.series[0].values.len();
        assert_eq!(pts.len(), n);
        let fill = |d: &El| d.child("c:spPr").unwrap().child("a:solidFill").unwrap().child("a:srgbClr").unwrap().attr("val").map(str::to_string);
        assert_eq!(fill(pts[n - 1]).as_deref(), Some("111111"));
        assert_eq!(fill(pts[n - 2]).as_deref(), Some("222222"));
        assert_eq!(spec_of(&cs).map(|s| s.colors), Some(coloured.colors.clone()), "the palette reads back");
        // Other bars keep the usual gap.
        assert_eq!(
            plot_of(&parse(&ChartSpec::sample(ChartType::BarStacked))).child("c:barChart").unwrap().child("c:gapWidth").unwrap().attr("val"),
            Some("150")
        );
    }

    #[test]
    fn style_fields_read_back_for_every_type() {
        for kind in ChartType::ALL {
            let spec = styled(kind);
            let back = spec_of(&parse(&spec)).unwrap_or_else(|| panic!("{kind:?} reads back"));
            // Smooth lines and markers belong to line and area charts; axes and gridlines to
            // charts with axes.
            let mut want = spec.clone();
            if !(kind.is_line() || kind.is_area() || kind == ChartType::Combo) {
                want.smooth = false;
            }
            if !(kind.is_line() || kind.is_area() || matches!(kind, ChartType::Combo | ChartType::ScatterLines | ChartType::ScatterSmooth)) {
                want.markers = false;
            }
            if !(kind.is_bar() || matches!(kind, ChartType::Combo | ChartType::Waterfall | ChartType::Funnel)) {
                want.rounded_bars = false;
            }
            if !matches!(kind, ChartType::Column | ChartType::Bar | ChartType::Combo) {
                want.series_gap = 0.0;
            }
            if kind.is_round() {
                (want.gridlines, want.axis_min, want.axis_max, want.axis_step) = (Gridlines::Solid, None, None, None);
            }
            if kind == ChartType::Waterfall {
                // Square ends: read back as the default.
                want.rounded_bars = ChartSpec::default().rounded_bars;
            }
            if kind == ChartType::Funnel {
                // A funnel has no value scale and square ends: those fields read back as the defaults.
                let d = ChartSpec::default();
                (want.gridlines, want.axis_min, want.axis_max, want.axis_step) = (d.gridlines, None, None, None);
                (want.val_labels, want.val_axis, want.val_ticks, want.rounded_bars) = (d.val_labels, d.val_axis, d.val_ticks, d.rounded_bars);
            }
            assert_eq!(back, want, "{kind:?}");
        }
        assert_eq!(spec_of(&parse(&styled(ChartType::Bar))).map(|s| s.series_gap), Some(25.0));
        // A positive overlap from another program isn't a series gap: no gap.
        let x = chart_xml(&styled(ChartType::Column)).replace("<c:overlap val=\"-25\"/>", "<c:overlap val=\"30\"/>");
        assert_eq!(spec_of(&xml::parse(x.as_bytes()).unwrap()).map(|s| s.series_gap), Some(0.0));
        for gridlines in Gridlines::ALL {
            let spec = ChartSpec { gridlines, colors: vec!["abcdef".into(); 5], ..ChartSpec::sample(ChartType::Bar) };
            assert_eq!(spec_of(&parse(&spec)), Some(spec));
        }
        // A series colour another program changed: the palette in our extension is what counts.
        let x = chart_xml(&styled(ChartType::Column)).replacen("val=\"111111\"", "val=\"FF0000\"", 1);
        assert_eq!(spec_of(&xml::parse(x.as_bytes()).unwrap()).map(|s| s.colors), Some(vec!["111111".to_string(), "777777".to_string()]));
    }

    /// Fill colours of the drawn paths and shapes.
    fn fills(items: &[GraphicItem]) -> Vec<Rgb> {
        items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Path { fill: Some(c), .. } | GraphicItem::Shape { fill: Some(c), .. } => Some(*c),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn style_fields_reach_the_drawing() {
        let (dark, mid) = (Rgb(0x11, 0x11, 0x11), Rgb(0x77, 0x77, 0x77));
        for kind in [ChartType::Column, ChartType::AreaStacked, ChartType::Pie, ChartType::Doughnut, ChartType::Scatter] {
            let f = fills(&chart_items(&styled(kind), &THEME_COLORS, 300.0, 200.0));
            assert!(f.contains(&dark) && f.contains(&mid), "{kind:?}: {f:?}");
            assert!(!f.contains(&THEME_COLORS[4]), "{kind:?}: no theme accent");
        }
        let line = chart_items(&styled(ChartType::Line), &THEME_COLORS, 300.0, 200.0);
        let strokes: Vec<Rgb> =
            line.iter().filter_map(|it| if let GraphicItem::Path { stroke: Some(c), .. } = it { Some(*c) } else { None }).collect();
        assert!(strokes.contains(&dark) && strokes.contains(&mid));
        let cubic = line.iter().any(|it| matches!(it, GraphicItem::Path { segs, .. } if segs.iter().any(|s| matches!(s, PathSeg::Cubic(..)))));
        assert!(cubic, "smooth lines are curves");
        // Rounded bars: each bar a path with four curved corners; square bars have none.
        let corners = |spec: &ChartSpec| -> Vec<usize> {
            chart_items(spec, &THEME_COLORS, 300.0, 200.0)
                .iter()
                .filter_map(|it| match it {
                    GraphicItem::Path { segs, fill: Some(_), .. } => Some(segs.iter().filter(|s| matches!(s, PathSeg::Cubic(..))).count()),
                    _ => None,
                })
                .collect()
        };
        // Clustered: every bar's value end (two corners); stacked: only the top segment of each stack.
        let round = corners(&ChartSpec { rounded_bars: true, ..ChartSpec::sample(ChartType::Column) });
        assert!(round.len() == 12 && round.iter().all(|n| *n == 2), "{round:?}");
        let round = corners(&ChartSpec { rounded_bars: true, ..ChartSpec::sample(ChartType::BarStacked) });
        assert_eq!(round, [0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 2]);
        assert!(corners(&ChartSpec::sample(ChartType::Column)).iter().all(|n| *n == 0));
        let circles = line.iter().filter(|it| matches!(it, GraphicItem::Shape { kind: wordcraft_doc::para::ShapeKind::Ellipse, .. })).count();
        assert_eq!(circles, 12, "a marker on each of 3 × 4 points");
        let texts: Vec<&str> =
            line.iter().filter_map(|it| if let GraphicItem::Text { text, .. } = it { Some(text.as_str()) } else { None }).collect();
        for t in ["-1.5", "1.0", "3.5", "6.0", "8.5", "11.0"] {
            assert!(texts.contains(&t), "tick {t} in {texts:?}");
        }
        // Dashed gridlines: each one many pieces.
        let dashed = line.iter().any(
            |it| matches!(it, GraphicItem::Path { segs, fill: None, .. } if segs.iter().filter(|s| matches!(s, PathSeg::Move(..))).count() > 10),
        );
        assert!(dashed);
    }

    #[test]
    fn axes_have_near_black_lines_outward_value_ticks_and_grey_gridlines() {
        let ln = |e: &El| -> (Option<String>, Option<String>) {
            let l = e.child("c:spPr").and_then(|p| p.child("a:ln"));
            (l.and_then(|l| l.attr("w")).map(str::to_string), l.and_then(|l| l.find("a:srgbClr")).and_then(|c| c.attr("val")).map(str::to_string))
        };
        let want = (Some("9525".to_string()), Some("333333".to_string()));
        for kind in [ChartType::Column, ChartType::BarStacked, ChartType::Line, ChartType::AreaStacked, ChartType::Scatter] {
            for gridlines in Gridlines::ALL {
                let spec = ChartSpec { gridlines, ..ChartSpec::sample(kind) };
                let space = parse(&spec);
                let plot = plot_of(&space);
                let val_ax = plot.children("c:valAx").find(|a| a.child("c:axId").and_then(|i| i.attr("val")) == Some(VAL_AX)).unwrap();
                assert_eq!(val_ax.child("c:majorTickMark").unwrap().attr("val"), Some("out"), "{kind:?}");
                assert_eq!(val_ax.child("c:delete").unwrap().attr("val"), Some("0"));
                assert_eq!(ln(val_ax), want, "{kind:?} value axis line");
                let other = plot.child("c:catAx").or_else(|| plot.children("c:valAx").find(|a| !std::ptr::eq(*a, val_ax))).unwrap();
                assert_eq!(other.child("c:majorTickMark").unwrap().attr("val"), Some("none"), "{kind:?}: ticks only on the value axis");
                assert_eq!(ln(other), want, "{kind:?} category axis line");
                match val_ax.child("c:majorGridlines") {
                    Some(g) => {
                        assert_ne!(gridlines, Gridlines::None);
                        assert_eq!(ln(g), (Some("9525".to_string()), Some("C8C8C8".to_string())), "{kind:?} {gridlines:?}");
                        assert_eq!(g.find("a:prstDash").is_some(), gridlines == Gridlines::Dashed);
                    }
                    None => assert_eq!(gridlines, Gridlines::None),
                }
                assert_eq!(spec_of(&space), Some(spec.sanitized()), "{kind:?} {gridlines:?} reads back");
            }
        }
        assert!(!chart_xml(&ChartSpec::sample(ChartType::Pie)).contains("c:majorTickMark"), "no axes on a pie");
    }

    /// The stroked paths of a drawn chart in `color`, as their (Move, Line) pieces.
    fn strokes_in(items: &[GraphicItem], color: Rgb) -> Vec<Vec<((f32, f32), (f32, f32))>> {
        items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Path { segs, stroke: Some(c), .. } if *c == color => Some(
                    segs.chunks(2)
                        .filter_map(|p| match p {
                            [PathSeg::Move(a, b), PathSeg::Line(c, d)] => Some(((*a, *b), (*c, *d))),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn area_series_fade_down_with_a_line_along_the_top() {
        for kind in [ChartType::Area, ChartType::AreaStacked] {
            for colors in [vec![], vec!["112233".to_string(), "445566".to_string()]] {
                let spec = ChartSpec { colors: colors.clone(), legend: None, ..ChartSpec::sample(kind) };
                let space = parse(&spec);
                let sers: Vec<&El> = plot_of(&space).child("c:areaChart").unwrap().children("c:ser").collect();
                assert_eq!(sers.len(), 3);
                for (j, s) in sers.iter().enumerate() {
                    let sp = s.child("c:spPr").unwrap();
                    assert_eq!(names(sp), ["a:gradFill", "a:ln"], "{kind:?}");
                    let gf = sp.child("a:gradFill").unwrap();
                    assert_eq!(gf.attr("rotWithShape"), Some("1"));
                    assert_eq!(names(gf), ["a:gsLst", "a:lin"]);
                    let lin = gf.child("a:lin").unwrap();
                    assert_eq!((lin.attr("ang"), lin.attr("scaled")), (Some("5400000"), Some("0")));
                    let stops: Vec<(&str, &str, &str, &str)> = gf
                        .child("a:gsLst")
                        .unwrap()
                        .children("a:gs")
                        .map(|g| {
                            let c = g.els().next().unwrap();
                            (g.attr("pos").unwrap(), c.name.as_str(), c.attr("val").unwrap(), c.child("a:alpha").unwrap().attr("val").unwrap())
                        })
                        .collect();
                    let (el, val) = match colors.get(j % 2) {
                        Some(c) if !colors.is_empty() => ("a:srgbClr", c.clone()),
                        _ => ("a:schemeClr", format!("accent{}", j + 1)),
                    };
                    assert_eq!(stops, [("0", el, val.as_str(), "80000"), ("100000", el, val.as_str(), "15000")], "{kind:?} series {j}");
                    let ln = sp.child("a:ln").unwrap();
                    assert_eq!(ln.attr("w"), Some("19050"));
                    assert_eq!(ln.child("a:solidFill").unwrap().els().next().unwrap().attr("val"), Some(val.as_str()));
                }
                assert_eq!(spec_of(&space), Some(spec.clone().sanitized()), "{kind:?} reads back");

                // Drawn: per series a gradient fill (80% at the plot's top to 15% at its bottom,
                // in the series colour) and a 1.5 pt line of that colour along its top.
                let items = chart_items(&spec, &THEME_COLORS, 300.0, 200.0);
                let plot = chart_plot_rect(&spec, &THEME_COLORS, 300.0, 200.0).unwrap();
                let grads: Vec<&wordcraft_doc::graphic::LinearGradient> =
                    items.iter().filter_map(|it| if let GraphicItem::Path { gradient: Some(g), .. } = it { Some(g) } else { None }).collect();
                assert_eq!(grads.len(), 3, "{kind:?}");
                for g in &grads {
                    assert!((g.from[1] - plot[1]).abs() < 1e-3 && (g.to[1] - plot[1] - plot[3]).abs() < 1e-3, "top to bottom of the plot: {g:?}");
                    assert_eq!(g.from[0], g.to[0]);
                    let alphas: Vec<f32> = g.stops.iter().map(|s| s.alpha).collect();
                    assert!((alphas[0] - 0.8).abs() < 1e-3 && (alphas[1] - 0.15).abs() < 1e-3, "{alphas:?}");
                    assert_eq!(g.stops[0].color, g.stops[1].color);
                }
                let edges: Vec<(Rgb, f32)> = items
                    .iter()
                    .filter_map(|it| match it {
                        GraphicItem::Path { stroke: Some(c), stroke_width, fill: None, segs, .. }
                            if segs.len() == 4 && !segs.iter().any(|s| matches!(s, PathSeg::Close)) =>
                        {
                            Some((*c, *stroke_width))
                        }
                        _ => None,
                    })
                    .filter(|(c, _)| grads.iter().any(|g| g.stops[0].color == *c))
                    .collect();
                assert_eq!(edges.len(), 3, "{kind:?}: {edges:?}");
                assert!(edges.iter().all(|(_, w)| (*w - 1.5).abs() < 1e-3));
                if !colors.is_empty() {
                    assert_eq!(grads[0].stops[0].color, Rgb(0x11, 0x22, 0x33));
                    assert_eq!(grads[2].stops[0].color, Rgb(0x11, 0x22, 0x33), "the palette in turn");
                } else {
                    assert_eq!(grads[1].stops[0].color, THEME_COLORS[5], "theme accent 2");
                }
                // The edges come after every fill.
                let last_fill = items.iter().rposition(|it| matches!(it, GraphicItem::Path { gradient: Some(_), .. })).unwrap();
                let first_edge =
                    items.iter().position(|it| matches!(it, GraphicItem::Path { stroke: Some(_), fill: None, segs, .. } if segs.len() == 4)).unwrap();
                assert!(first_edge > last_fill);
            }
        }
    }

    /// The category axis (or a scatter chart's x axis) and the value axis of a written chart.
    fn axes_of(space: &El) -> (&El, &El) {
        let plot = plot_of(space);
        let by_id = |id: &str| plot.children("c:valAx").find(|a| a.child("c:axId").and_then(|i| i.attr("val")) == Some(id));
        (plot.child("c:catAx").or_else(|| by_id(CAT_AX)).unwrap(), by_id(VAL_AX).unwrap())
    }

    /// The `c:f` formula and cached point texts of a `c:strRef`/`c:numRef` under `e`.
    fn reference(e: &El) -> (String, Vec<String>) {
        let r = e.els().find(|c| matches!(c.name.as_str(), "c:strRef" | "c:numRef")).unwrap();
        let cache = r.els().find(|c| c.name.ends_with("Cache")).unwrap();
        (r.child("c:f").unwrap().text(), cache.children("c:pt").map(|p| p.child("c:v").unwrap().text()).collect())
    }

    #[test]
    fn series_name_the_workbook_cells_with_caches_of_them() {
        let space = parse(&ChartSpec::sample(ChartType::Column));
        let bar = plot_of(&space).child("c:barChart").unwrap();
        let sers: Vec<&El> = bar.children("c:ser").collect();
        assert_eq!(reference(sers[0].child("c:tx").unwrap()), ("Sheet1!$B$1".into(), vec!["Series 1".into()]));
        assert_eq!(reference(sers[0].child("c:cat").unwrap()), ("Sheet1!$A$2:$A$5".into(), vec!["Q1".into(), "Q2".into(), "Q3".into(), "Q4".into()]));
        assert_eq!(
            reference(sers[0].child("c:val").unwrap()),
            ("Sheet1!$B$2:$B$5".into(), vec!["3.2".into(), "4.1".into(), "3.6".into(), "5".into()])
        );
        assert_eq!(reference(sers[2].child("c:tx").unwrap()).0, "Sheet1!$D$1");
        assert_eq!(reference(sers[2].child("c:val").unwrap()).0, "Sheet1!$D$2:$D$5");
        let cat = sers[0].child("c:cat").unwrap();
        assert_eq!(names(cat), ["c:strRef"]);
        assert_eq!(names(sers[0].child("c:val").unwrap().child("c:numRef").unwrap().child("c:numCache").unwrap())[0], "c:formatCode");
        assert!(!chart_xml(&ChartSpec::sample(ChartType::Column)).contains("Lit>"), "no literals");
        let ext = space.child("c:externalData").unwrap();
        assert_eq!((ext.attr("r:id"), ext.child("c:autoUpdate").and_then(|a| a.attr("val"))), (Some("rId1"), Some("0")));
        // Scatter: numeric x values as a number reference; text ones as a string reference.
        let sc = parse(&ChartSpec::sample(ChartType::Scatter));
        let s0 = plot_of(&sc).child("c:scatterChart").unwrap().child("c:ser").unwrap();
        assert_eq!(names(s0.child("c:xVal").unwrap()), ["c:numRef"]);
        assert_eq!(reference(s0.child("c:xVal").unwrap()).0, "Sheet1!$A$2:$A$5");
        assert_eq!(reference(s0.child("c:yVal").unwrap()).0, "Sheet1!$B$2:$B$5");
        let words = parse(&ChartSpec { categories: vec!["x".into(), "y".into(), "z".into(), "w".into()], ..ChartSpec::sample(ChartType::Scatter) });
        let s0 = plot_of(&words).child("c:scatterChart").unwrap().child("c:ser").unwrap();
        assert_eq!(names(s0.child("c:xVal").unwrap()), ["c:strRef"]);
        // Columns past Z.
        assert_eq!((column(0), column(25), column(26), column(255)), ("A".into(), "Z".into(), "AA".into(), "IV".into()));
        // Every type still reads back as itself.
        for kind in ChartType::ALL {
            let spec = ChartSpec::sample(kind);
            assert_eq!(spec_of(&parse(&spec)), Some(spec), "{kind:?}");
        }
    }

    /// The parts of a zip, by name.
    fn unzip(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        use std::io::Read as _;
        let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
        let mut out = std::collections::BTreeMap::new();
        for i in 0..z.len() {
            let mut f = z.by_index(i).unwrap();
            let mut b = Vec::new();
            f.read_to_end(&mut b).unwrap();
            out.insert(f.name().to_string(), b);
        }
        out
    }

    /// The cells of a workbook's first sheet: reference → text (inline string or number).
    fn cells(xlsx: &[u8]) -> std::collections::BTreeMap<String, String> {
        let parts = unzip(xlsx);
        for p in ["[Content_Types].xml", "_rels/.rels", "xl/workbook.xml", "xl/_rels/workbook.xml.rels", "xl/styles.xml"] {
            assert!(xml::parse(parts.get(p).unwrap_or_else(|| panic!("{p}"))).is_ok(), "{p} parses");
        }
        let sheet = xml::parse(&parts["xl/worksheets/sheet1.xml"]).unwrap();
        let mut out = std::collections::BTreeMap::new();
        for c in sheet.find("?:sheetData").or_else(|| sheet.els().find(|e| e.name.ends_with("sheetData"))).unwrap().els().flat_map(|r| r.els()) {
            let text = c
                .els()
                .find(|e| e.name.ends_with(":is"))
                .and_then(|is| is.els().next())
                .map(|t| t.text())
                .or_else(|| c.els().find(|e| e.name.ends_with(":v")).map(|v| v.text()));
            out.insert(c.attr("r").unwrap().to_string(), text.unwrap_or_default());
        }
        out
    }

    /// A document holding one chart of `spec`.
    fn doc_with(spec: &ChartSpec) -> wordcraft_doc::Document {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let mut doc = wordcraft_doc::Document::new();
        let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec.clone())), ..Default::default() };
        let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float: Default::default(), graphic: Arc::new(graphic) };
        doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
        doc
    }

    /// The chart model of the first object in a document's first paragraph.
    fn chart_of(doc: &wordcraft_doc::Document) -> Option<ChartSpec> {
        let p = doc.para_at(&wordcraft_doc::Pos::body(0, 0))?;
        p.object_offsets().into_iter().find_map(|off| match p.object_at(off) {
            Some(wordcraft_doc::InlineObject::Graphic { graphic, .. }) => graphic.chart.as_deref().cloned(),
            _ => None,
        })
    }

    #[test]
    fn a_saved_chart_carries_its_workbook() {
        let mut spec = ChartSpec { categories: vec!["Jan".into(), "Feb".into(), "Mar".into()], ..ChartSpec::sample(ChartType::Line) };
        spec.series.iter_mut().for_each(|s| s.values.truncate(3));
        let spec = spec.sanitized();
        let bytes = crate::write(&doc_with(&spec)).unwrap();
        let parts = unzip(&bytes);
        let book = parts.get("word/embeddings/Workbook1.xlsx").expect("the workbook part");
        let rels = String::from_utf8_lossy(&parts["word/charts/_rels/chart1.xml.rels"]).into_owned();
        assert!(rels.contains(PACKAGE_REL) && rels.contains(r#"Target="../embeddings/Workbook1.xlsx""#) && rels.contains(r#"Id="rId1""#), "{rels}");
        let types = String::from_utf8_lossy(&parts["[Content_Types].xml"]).into_owned();
        assert!(
            types.contains(
                r#"PartName="/word/embeddings/Workbook1.xlsx" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet""#
            ),
            "{types}"
        );
        // The sheet: A1 empty, series names in row 1, categories in column A, values beside them.
        let c = cells(book);
        assert_eq!(c["A1"], " ", "a single space over the categories, the table's first column name");
        assert_eq!((c["B1"].as_str(), c["C1"].as_str(), c["D1"].as_str()), ("Series 1", "Series 2", "Series 3"));
        assert_eq!((c["A2"].as_str(), c["A3"].as_str(), c["A4"].as_str()), ("Jan", "Feb", "Mar"));
        assert_eq!((c["B2"].as_str(), c["D3"].as_str()), ("3.2", "2.2"));
        assert!(!c.contains_key("A5") && !c.contains_key("B5"), "only the chart's rows");
        // Read back: the same chart, and saving again writes the workbook again.
        let back = crate::read(&bytes).unwrap();
        assert_eq!(chart_of(&back), Some(spec.clone()));
        assert!(unzip(&crate::write(&back).unwrap()).contains_key("word/embeddings/Workbook1.xlsx"));
        // A gap is an empty cell.
        let mut gap = spec.clone();
        gap.series[0].values[1] = None;
        let c = cells(&chart_workbook(&gap).unwrap());
        assert!(!c.contains_key("B3") && c.contains_key("C3"));
    }

    #[test]
    fn the_workbook_data_is_a_table_with_unique_column_names() {
        let spec = ChartSpec::sample(ChartType::Column);
        let book = chart_workbook(&spec).unwrap();
        let parts = unzip(&book);
        let types = String::from_utf8_lossy(&parts["[Content_Types].xml"]).into_owned();
        assert!(
            types.contains(r#"PartName="/xl/tables/table1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.table+xml""#)
        );
        let rels = String::from_utf8_lossy(&parts["xl/worksheets/_rels/sheet1.xml.rels"]).into_owned();
        assert!(rels.contains(r#"Id="rId1""#) && rels.contains("relationships/table") && rels.contains(r#"Target="../tables/table1.xml""#), "{rels}");
        let sheet = xml::parse(&parts["xl/worksheets/sheet1.xml"]).unwrap();
        let tp = sheet.els().last().unwrap();
        assert!(tp.name.ends_with("tableParts") && tp.attr("count") == Some("1"));
        assert_eq!(tp.els().next().and_then(|p| p.attr("r:id")), Some("rId1"));
        // The table: over A1 to the last series column and category row, a header row, no filter.
        let table = xml::parse(&parts["xl/tables/table1.xml"]).unwrap();
        for a in ["id", "name", "displayName", "ref"] {
            assert!(table.attr(a).is_some(), "{a}");
        }
        assert_eq!(table.attr("ref"), Some("A1:D5"));
        assert!(!table.els().any(|e| e.name.ends_with("autoFilter")));
        let cols: Vec<String> =
            table.els().find(|e| e.name.ends_with("tableColumns")).unwrap().els().map(|c| c.attr("name").unwrap().to_string()).collect();
        let c = cells(&book);
        let headers: Vec<String> = ["A1", "B1", "C1", "D1"].iter().map(|r| c[*r].clone()).collect();
        assert_eq!(cols, headers, "column names are the header cells");
        assert_eq!(cols, [" ", "Series 1", "Series 2", "Series 3"]);
        // Repeated or missing names become unique, the same in the cells, the table and the chart.
        let dup = ChartSpec {
            series: vec![
                ChartSeries { name: "Sales".into(), values: vec![Some(1.0)], ..Default::default() },
                ChartSeries { name: "sales".into(), values: vec![Some(2.0)], ..Default::default() },
                ChartSeries { name: String::new(), values: vec![Some(3.0)], ..Default::default() },
                ChartSeries { name: "Sales".into(), values: vec![Some(4.0)], ..Default::default() },
            ],
            categories: vec!["a".into()],
            ..Default::default()
        }
        .sanitized();
        let book = chart_workbook(&dup).unwrap();
        let table = xml::parse(&unzip(&book)["xl/tables/table1.xml"]).unwrap();
        let cols: Vec<String> =
            table.els().find(|e| e.name.ends_with("tableColumns")).unwrap().els().map(|c| c.attr("name").unwrap().to_string()).collect();
        assert_eq!(cols, [" ", "Sales", "sales2", "Series 3", "Sales3"]);
        assert_eq!(table.attr("ref"), Some("A1:E2"));
        let c = cells(&book);
        assert_eq!((c["C1"].as_str(), c["D1"].as_str(), c["E1"].as_str()), ("sales2", "Series 3", "Sales3"));
        let space = parse(&dup);
        let sers: Vec<&El> = plot_of(&space).child("c:barChart").unwrap().children("c:ser").collect();
        assert_eq!(reference(sers[1].child("c:tx").unwrap()).1, ["sales2"]);
        assert!(sers[2].child("c:tx").is_none(), "a series without a name has none in the chart");
        assert_eq!(reference(sers[3].child("c:tx").unwrap()).1, ["Sales3"]);
        // The formulas stay plain cell references.
        assert_eq!(reference(sers[1].child("c:val").unwrap()).0, "Sheet1!$C$2");
    }

    #[test]
    fn a_chart_edited_elsewhere_reads_from_its_caches_and_writes_a_fresh_workbook() {
        let spec = ChartSpec {
            categories: vec!["a".into(), "b".into()],
            series: vec![ChartSeries { name: "S1".into(), values: vec![Some(1.0), Some(2.0)], ..Default::default() }],
            ..ChartSpec::sample(ChartType::Column)
        }
        .sanitized();
        let bytes = crate::write(&doc_with(&spec)).unwrap();
        // Another program edits the data: a category and a series more, in the chart's caches and
        // formulas; its workbook here is left as it was (stale), to show the caches are what counts.
        let edited = ChartSpec {
            categories: vec!["a".into(), "b".into(), "c".into()],
            series: vec![
                ChartSeries { name: "S1".into(), values: vec![Some(1.0), Some(2.0), Some(3.0)], ..Default::default() },
                ChartSeries { name: "S2".into(), values: vec![Some(4.0), Some(5.0), Some(6.0)], ..Default::default() },
            ],
            ..spec.clone()
        };
        let mut parts = unzip(&bytes);
        let old_book = parts["word/embeddings/Workbook1.xlsx"].clone();
        let old_xml = String::from_utf8(parts["word/charts/chart1.xml"].clone()).unwrap();
        let new_xml = old_xml
            .replace("Sheet1!$A$2:$A$3", "Sheet1!$A$2:$A$4")
            .replace("Sheet1!$B$2:$B$3", "Sheet1!$B$2:$B$4")
            .replace(r#"<c:ptCount val="2"/>"#, r#"<c:ptCount val="3"/>"#)
            .replace(r#"<c:pt idx="1"><c:v>b</c:v></c:pt>"#, r#"<c:pt idx="1"><c:v>b</c:v></c:pt><c:pt idx="2"><c:v>c</c:v></c:pt>"#)
            .replace(r#"<c:pt idx="1"><c:v>2</c:v></c:pt>"#, r#"<c:pt idx="1"><c:v>2</c:v></c:pt><c:pt idx="2"><c:v>3</c:v></c:pt>"#);
        // The second series, as the program writes it: its own cells, names and caches.
        let s2 = chart_xml(&edited);
        let start = s2.match_indices("<c:ser>").nth(1).unwrap().0;
        let end = s2.get(start..).unwrap().find("</c:ser>").unwrap() + start + "</c:ser>".len();
        let new_xml = new_xml.replacen("<c:dLbls>", &format!("{}<c:dLbls>", s2.get(start..end).unwrap()), 1);
        let new_xml = if new_xml.contains("<c:dLbls>") {
            new_xml
        } else {
            new_xml.replacen("<c:gapWidth", &format!("{}<c:gapWidth", s2.get(start..end).unwrap()), 1)
        };
        assert_eq!(new_xml, chart_xml(&edited), "the edit is what the other program would write");
        parts.insert("word/charts/chart1.xml".into(), new_xml.into_bytes());
        let edited_bytes = crate::package::zip_entries(&parts.into_iter().collect::<Vec<_>>()).unwrap();
        let back = crate::read(&edited_bytes).unwrap();
        assert_eq!(chart_of(&back), Some(edited.clone()), "the model follows the caches");
        // Saved without an edit: the chart's own parts, as they were read.
        let kept = unzip(&crate::write(&back).unwrap());
        assert!(kept.values().any(|b| *b == old_book), "the parts as read");
        // Once edited (which drops the chart's source), the save writes the chart and its workbook
        // from the model, with the new row and column.
        let mut back = back;
        let para = back.para_mut(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Pos::body(0, 0).path).unwrap();
        let off = para.object_offsets()[0];
        if let Some(wordcraft_doc::InlineObject::Graphic { graphic, .. }) = para.object_at_mut(off) {
            let mut g = (**graphic).clone();
            g.source = None;
            *graphic = std::sync::Arc::new(g);
        }
        let again = unzip(&crate::write(&back).unwrap());
        let book = &again["word/embeddings/Workbook1.xlsx"];
        assert_ne!(*book, old_book);
        let c = cells(book);
        assert_eq!((c["A4"].as_str(), c["C1"].as_str(), c["C4"].as_str()), ("c", "S2", "6"));
    }

    /// A document with two charts: a column chart with round legend keys and a hidden title, and
    /// a plain line chart.
    fn two_chart_doc() -> (wordcraft_doc::Document, ChartSpec, ChartSpec) {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let special = ChartSpec {
            legend_shape: LegendShape::Circle,
            title: Some("Secret".into()),
            title_visible: false,
            ..ChartSpec::sample(ChartType::Column)
        }
        .sanitized();
        let plain = ChartSpec::sample(ChartType::Line);
        let mut doc = wordcraft_doc::Document::new();
        for spec in [&plain, &special] {
            let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec.clone())), ..Default::default() };
            let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float: Default::default(), graphic: Arc::new(graphic) };
            doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
        }
        (doc, special, plain)
    }

    /// `bytes` as another program might save them: each chart part without our extension, and
    /// every drawing id renumbered (names kept); the custom XML parts as they were.
    fn as_saved_elsewhere(bytes: &[u8]) -> std::collections::BTreeMap<String, Vec<u8>> {
        let mut parts = unzip(bytes);
        for (name, b) in parts.iter_mut() {
            if name.starts_with("word/charts/chart") && name.ends_with(".xml") {
                *b = without_ext(&String::from_utf8_lossy(b)).into_bytes();
            }
        }
        let doc = String::from_utf8(parts["word/document.xml"].clone()).unwrap();
        let mut out = String::new();
        let mut rest = doc.as_str();
        let mut n = 900;
        while let Some(i) = rest.find("<wp:docPr id=\"") {
            out.push_str(&rest[..i]);
            let after = &rest[i + "<wp:docPr id=\"".len()..];
            let end = after.find('"').unwrap();
            out.push_str(&format!("<wp:docPr id=\"{n}\""));
            n += 7;
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        parts.insert("word/document.xml".into(), out.into_bytes());
        parts
    }

    /// All the chart models in a document's first paragraph, in order.
    fn charts_of(doc: &wordcraft_doc::Document) -> Vec<ChartSpec> {
        let p = doc.para_at(&wordcraft_doc::Pos::body(0, 0)).unwrap();
        p.object_offsets()
            .into_iter()
            .filter_map(|off| match p.object_at(off) {
                Some(wordcraft_doc::InlineObject::Graphic { graphic, .. }) => graphic.chart.as_deref().cloned(),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn settings_only_wordcraft_keeps_come_back_from_the_document_records() {
        let (doc, special, plain) = two_chart_doc();
        let bytes = crate::write(&doc).unwrap();
        let parts = unzip(&bytes);
        let records = parts
            .iter()
            .find(|(n, b)| n.starts_with("customXml/item") && String::from_utf8_lossy(b).contains("wordcraftCharts"))
            .expect("a records part");
        let text = String::from_utf8_lossy(records.1).into_owned();
        assert!(text.contains(r#"legendShape="circle""#) && text.contains("<wc:hiddenTitle>Secret</wc:hiddenTitle>"), "{text}");
        assert_eq!(text.matches("<wc:chart ").count(), 1, "only the chart with such settings: {text}");
        // Saved elsewhere: the chart parts lose our extension and the drawings their ids.
        let elsewhere = crate::package::zip_entries(&as_saved_elsewhere(&bytes).into_iter().collect::<Vec<_>>()).unwrap();
        let back = crate::read(&elsewhere).unwrap();
        let charts = charts_of(&back);
        let col = charts.iter().find(|c| c.kind == ChartType::Column).unwrap();
        assert_eq!(col.legend_shape, LegendShape::Circle, "from the records");
        assert_eq!((col.title.as_deref(), col.title_visible), (Some("Secret"), false));
        assert_eq!(*col, special);
        assert_eq!(*charts.iter().find(|c| c.kind == ChartType::Line).unwrap(), plain, "the other chart untouched");
        // Saved again, the record is written again (not passed through twice).
        let again = unzip(&crate::write(&back).unwrap());
        let recs: Vec<&Vec<u8>> = again
            .iter()
            .filter(|(n, b)| n.starts_with("customXml/item") && String::from_utf8_lossy(b).contains("wordcraftCharts"))
            .map(|(_, b)| b)
            .collect();
        assert_eq!(recs.len(), 1);
    }

    #[test]
    fn records_of_deleted_charts_go_and_mismatched_records_are_ignored() {
        let (doc, _, _) = two_chart_doc();
        let bytes = crate::write(&doc).unwrap();
        // The chart with the record deleted: the next save has no records part.
        let mut back = crate::read(&bytes).unwrap();
        let para = back.para_mut(wordcraft_doc::StoryRef::Body, &wordcraft_doc::Pos::body(0, 0).path).unwrap();
        let offs = para.object_offsets();
        let special = offs
            .iter()
            .copied()
            .find(|o| matches!(para.object_at(*o), Some(wordcraft_doc::InlineObject::Graphic { graphic, .. }) if graphic.chart.as_deref().is_some_and(|c| c.kind == ChartType::Column)))
            .unwrap();
        let obj = wordcraft_doc::para::OBJ.len_utf8();
        let text = para.text.clone();
        let mut d = back.clone();
        d.delete_range(&wordcraft_doc::Pos::body(0, special), &wordcraft_doc::Pos::body(0, special + obj)).unwrap();
        assert_eq!(charts_of(&d).len(), 1);
        let pruned = unzip(&crate::write(&d).unwrap());
        assert!(!pruned.values().any(|b| String::from_utf8_lossy(b).contains("wordcraftCharts")), "no record for a deleted chart");
        assert!(!text.is_empty());
        // A record that names no chart and matches none by signature: ignored.
        let mut parts = as_saved_elsewhere(&bytes);
        let key = parts
            .keys()
            .find(|n| n.starts_with("customXml/item") && String::from_utf8_lossy(&parts[*n]).contains("wordcraftCharts"))
            .unwrap()
            .clone();
        let rec = String::from_utf8(parts[&key].clone()).unwrap();
        let name = rec.split("name=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
        let mismatched = rec.replace(&format!("name=\"{name}\""), "name=\"Chart 999\"").replace("signature=\"column/3/4\"", "signature=\"pie/1/4\"");
        assert_ne!(mismatched, rec);
        parts.insert(key, mismatched.into_bytes());
        back = crate::read(&crate::package::zip_entries(&parts.into_iter().collect::<Vec<_>>()).unwrap()).unwrap();
        let col = charts_of(&back).into_iter().find(|c| c.kind == ChartType::Column).unwrap();
        assert_eq!((col.legend_shape, col.title.as_deref()), (LegendShape::Square, None), "a record matching no chart changes nothing");
    }

    /// Writes a document with a chart of each type to a temporary file, to open in a word
    /// processor by hand: `cargo test -p wordcraft-docx --lib chart_sample_docx -- --ignored`.
    #[test]
    #[ignore = "writes a file for a person to open"]
    fn chart_sample_docx() {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let mut doc = wordcraft_doc::Document::new();
        for kind in ChartType::ALL.iter().rev() {
            let mut spec = ChartSpec { title: Some(kind.label().into()), data_labels: true, ..ChartSpec::sample(*kind) };
            // One chart with settings only WordCraft draws (kept in the chart records part too).
            if *kind == ChartType::Column {
                (spec.legend_shape, spec.colors) = (LegendShape::Circle, vec!["1f1f1f".into(), "6b6b6b".into(), "b0b0b0".into(), "d9d9d9".into()]);
            }
            let graphic = Graphic { kind: GraphicKind::Chart, w: 360.0, h: 220.0, chart: Some(Arc::new(spec)), ..Default::default() };
            let obj = InlineObject::Graphic { w: 360.0, h: 220.0, alt: kind.label().into(), float: Default::default(), graphic: Arc::new(graphic) };
            doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
        }
        let path = std::env::temp_dir().join("wordcraft-chart-sample.docx");
        std::fs::write(&path, crate::write(&doc).unwrap()).unwrap();
        println!("{}", path.display());
    }

    #[test]
    fn data_labels_say_where_they_go_where_the_chart_type_allows() {
        for (kind, want) in [
            (ChartType::Column, Some("inEnd")),
            (ChartType::Bar, Some("inEnd")),
            (ChartType::ColumnStacked, Some("ctr")),
            (ChartType::BarStacked, Some("ctr")),
            (ChartType::Line, Some("t")),
            (ChartType::Scatter, Some("t")),
            (ChartType::Pie, Some("bestFit")),
            (ChartType::Area, None),
            (ChartType::AreaStacked, None),
            (ChartType::Doughnut, None),
        ] {
            let spec = ChartSpec { data_labels: true, ..ChartSpec::sample(kind) };
            let space = parse(&spec);
            let ty = plot_of(&space).els().find(|e| e.name.ends_with("Chart")).unwrap();
            let d = ty.child("c:dLbls").unwrap();
            assert_eq!(d.child("c:dLblPos").and_then(|p| p.attr("val")), want, "{kind:?}");
            if want.is_some() {
                assert_eq!(names(d).first(), Some(&"c:dLblPos"), "{kind:?}: before showLegendKey");
            }
            assert_eq!(spec_of(&space), Some(spec.clone()), "{kind:?} reads back");
        }
    }

    #[test]
    fn chart_elements_are_written_as_standard_markup_and_read_back() {
        let attr = |e: &El, child: &str| e.child(child).and_then(|c| c.attr("val")).map(str::to_string);
        let line_off = |e: &El| e.child("c:spPr").and_then(|p| p.child("a:ln")).is_some_and(|l| l.child("a:noFill").is_some());
        for kind in [ChartType::Column, ChartType::Bar, ChartType::Line, ChartType::AreaStacked, ChartType::Scatter] {
            let on = ChartSpec { title: Some("Sales".into()), ..ChartSpec::sample(kind) };
            let space = parse(&on);
            let (cat, val) = axes_of(&space);
            assert_eq!((attr(cat, "c:tickLblPos"), attr(val, "c:tickLblPos")), (Some("nextTo".into()), Some("nextTo".into())));
            assert!(!line_off(cat) && !line_off(val));
            assert_eq!(attr(val, "c:majorTickMark").as_deref(), Some("out"));
            assert!(space.child("c:spPr").unwrap().child("a:noFill").is_some(), "the chart area is transparent");
            assert!(plot_of(&space).child("c:spPr").unwrap().child("a:noFill").is_some(), "no plot background by default");

            let off = ChartSpec {
                title_visible: false,
                background: true,
                cat_labels: false,
                cat_axis: false,
                val_labels: false,
                val_axis: false,
                ..on.clone()
            };
            let space = parse(&off);
            let (cat, val) = axes_of(&space);
            assert_eq!((attr(cat, "c:tickLblPos"), attr(val, "c:tickLblPos")), (Some("none".into()), Some("none".into())), "{kind:?}");
            assert!(line_off(cat) && line_off(val), "{kind:?}");
            assert_eq!(attr(val, "c:majorTickMark").as_deref(), Some("none"), "no ticks without the axis line");
            let chart = space.child("c:chart").unwrap();
            assert!(chart.child("c:title").is_none());
            assert_eq!(attr(chart, "c:autoTitleDeleted").as_deref(), Some("1"));
            // The background shades the plot area (3.5% black); the chart area stays transparent.
            assert!(space.child("c:spPr").unwrap().child("a:noFill").is_some());
            let clr = plot_of(&space).child("c:spPr").unwrap().child("a:solidFill").unwrap().child("a:srgbClr").unwrap();
            assert_eq!((clr.attr("val"), clr.child("a:alpha").and_then(|a| a.attr("val"))), (Some("000000"), Some("3500")));
            let ext = space.child("c:extLst").unwrap().child("c:ext").unwrap();
            assert_eq!(ext.child("?:hiddenTitle").map(|t| t.text()).as_deref(), Some("Sales"), "the hidden title's text is kept");
            // Each switch reads back on its own and all together.
            assert_eq!(spec_of(&space), Some(off.clone()), "{kind:?}");
            assert_eq!(spec_of(&parse(&on)), Some(on.clone()), "{kind:?}");
            for one in [
                ChartSpec { title_visible: false, ..on.clone() },
                ChartSpec { background: true, ..on.clone() },
                ChartSpec { cat_labels: false, ..on.clone() },
                ChartSpec { cat_axis: false, ..on.clone() },
                ChartSpec { val_labels: false, ..on.clone() },
                ChartSpec { val_axis: false, ..on.clone() },
            ] {
                assert_eq!(spec_of(&parse(&one)), Some(one.clone()), "{kind:?}");
            }
        }
        // Scatter: cat* is the x axis (at the bottom), val* the y axis.
        let sc = parse(&ChartSpec { cat_labels: false, ..ChartSpec::sample(ChartType::Scatter) });
        let (x, y) = axes_of(&sc);
        assert_eq!((attr(x, "c:axPos").as_deref(), attr(x, "c:tickLblPos").as_deref()), (Some("b"), Some("none")));
        assert_eq!(attr(y, "c:tickLblPos").as_deref(), Some("nextTo"));
        // A hidden title from another program's file (no extension) is just no title.
        let plain = chart_xml(&ChartSpec::sample(ChartType::Column));
        assert_eq!(spec_of(&xml::parse(plain.as_bytes()).unwrap()).map(|s| (s.title, s.title_visible)), Some((None, true)));
    }

    #[test]
    fn chart_elements_hide_what_they_say_and_the_plot_takes_the_room() {
        let (w, h) = (320.0, 220.0);
        let draw = |spec: &ChartSpec| chart_items(spec, &THEME_COLORS, w, h);
        let texts = |items: &[GraphicItem]| -> Vec<String> {
            items.iter().filter_map(|it| if let GraphicItem::Text { text, .. } = it { Some(text.clone()) } else { None }).collect()
        };
        let rect = |spec: &ChartSpec| chart_plot_rect(spec, &THEME_COLORS, w, h).unwrap();
        let axis = Rgb(0x33, 0x33, 0x33);
        let on = ChartSpec {
            title: Some("Sales".into()),
            legend: None,
            axis_min: Some(0.0),
            axis_max: Some(6.0),
            axis_step: Some(2.0),
            ..ChartSpec::sample(ChartType::Column)
        };
        let base = draw(&on);
        let r0 = rect(&on);
        assert!(texts(&base).iter().any(|t| t == "Sales") && texts(&base).iter().any(|t| t == "Q1") && texts(&base).iter().any(|t| t == "6"));
        assert!(!base.iter().any(|it| matches!(it, GraphicItem::Shape { rect, fill: Some(_), .. } if rect[2] == w)), "no background");

        // Title hidden: not drawn, and the plot grows upwards.
        let s = ChartSpec { title_visible: false, ..on.clone() };
        assert!(!texts(&draw(&s)).iter().any(|t| t == "Sales"));
        assert!(rect(&s)[1] < r0[1] && rect(&s)[3] > r0[3]);
        // Background: a translucent fill exactly at the plot rect, under the gridlines and bars.
        let s = ChartSpec { background: true, ..on.clone() };
        let items = draw(&s);
        let r = rect(&s);
        let shade = items.iter().position(|it| matches!(it, GraphicItem::Path { gradient: Some(_), .. })).expect("a plot background");
        let GraphicItem::Path { segs, gradient: Some(g), fill, .. } = &items[shade] else { panic!() };
        assert_eq!(segs.first(), Some(&PathSeg::Move(r[0], r[1])));
        assert!(segs.contains(&PathSeg::Line(r[0] + r[2], r[1] + r[3])));
        assert!(g.stops.len() == 2 && g.stops.iter().all(|st| st.color == Rgb::BLACK && (st.alpha - 0.035).abs() < 1e-4), "{g:?}");
        assert_eq!(*fill, Some(Rgb(0xF6, 0xF6, 0xF6)), "3.5% black over white where opacity can't be drawn");
        let grid = items.iter().position(|it| matches!(it, GraphicItem::Path { stroke: Some(c), .. } if *c == Rgb(0xC8, 0xC8, 0xC8))).unwrap();
        let bar = items.iter().position(|it| matches!(it, GraphicItem::Path { fill: Some(c), gradient: None, .. } if *c == THEME_COLORS[4])).unwrap();
        assert!(shade < grid && shade < bar, "behind the gridlines and series");
        assert_eq!(r, rect(&on), "the plot rect doesn't change");
        assert!(!items.iter().any(|it| matches!(it, GraphicItem::Shape { rect, fill: Some(_), .. } if rect[2] == w)), "the chart area isn't filled");
        // Category labels hidden: no "Q1", and the plot reaches further down.
        let s = ChartSpec { cat_labels: false, ..on.clone() };
        assert!(!texts(&draw(&s)).iter().any(|t| t.starts_with('Q')));
        assert!(rect(&s)[1] + rect(&s)[3] > r0[1] + r0[3]);
        // Value labels hidden: no tick labels, and the plot starts further left.
        let s = ChartSpec { val_labels: false, ..on.clone() };
        assert!(!texts(&draw(&s)).iter().any(|t| t == "6" || t == "2"));
        assert!(rect(&s)[0] < r0[0] && rect(&s)[2] > r0[2]);
        // Axis lines hidden: no horizontal category line along the bottom, no vertical value line or ticks.
        let lines = |items: &[GraphicItem]| -> (bool, bool, bool) {
            let segs: Vec<(PathSeg, PathSeg)> = items
                .iter()
                .filter_map(|it| if let GraphicItem::Path { segs, stroke: Some(c), .. } = it { (*c == axis).then(|| segs.clone()) } else { None })
                .flat_map(|s| {
                    s.chunks(2)
                        .filter_map(|p| match p {
                            [a, b] => Some((*a, *b)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                })
                .collect();
            let horizontal = segs.iter().any(|p| matches!(p, (PathSeg::Move(x0, y0), PathSeg::Line(x1, y1)) if y0 == y1 && (x1 - x0).abs() > 50.0));
            let vertical = segs.iter().any(|p| matches!(p, (PathSeg::Move(x0, y0), PathSeg::Line(x1, y1)) if x0 == x1 && (y1 - y0).abs() > 50.0));
            let ticks = segs.iter().any(|p| matches!(p, (PathSeg::Move(x0, y0), PathSeg::Line(x1, y1)) if y0 == y1 && (x1 - x0 - 4.0).abs() < 1e-3));
            (horizontal, vertical, ticks)
        };
        assert_eq!(lines(&base), (true, true, true));
        assert_eq!(lines(&draw(&ChartSpec { cat_axis: false, ..on.clone() })), (false, true, true));
        assert_eq!(lines(&draw(&ChartSpec { val_axis: false, ..on.clone() })), (true, false, false));
        // Everything off, and the plot rect still matches the drawing's layout.
        let bare = ChartSpec { title_visible: false, cat_labels: false, cat_axis: false, val_labels: false, val_axis: false, ..on.clone() };
        let items = draw(&bare);
        assert_eq!(texts(&items), Vec::<String>::new());
        assert_eq!(lines(&items), (false, false, false));
        let r = rect(&bare);
        assert!(r[2] > r0[2] && r[3] > r0[3]);
        // Pie and doughnut charts ignore the axis switches.
        for kind in [ChartType::Pie, ChartType::Doughnut] {
            let pie = ChartSpec::sample(kind);
            let off = ChartSpec { cat_labels: false, cat_axis: false, val_labels: false, val_axis: false, ..pie.clone() };
            assert_eq!(draw(&off), draw(&pie), "{kind:?}");
        }
    }

    #[test]
    fn area_charts_run_from_the_value_axis_to_the_far_edge() {
        let cross = |spec: &ChartSpec| {
            let space = parse(spec);
            plot_of(&space).children("c:valAx").last().and_then(|a| a.child("c:crossBetween")).and_then(|c| c.attr("val")).map(str::to_string)
        };
        for kind in [ChartType::Area, ChartType::AreaStacked] {
            let spec = ChartSpec { legend: None, ..ChartSpec::sample(kind) };
            assert_eq!(cross(&spec).as_deref(), Some("midCat"), "{kind:?}");
            assert_eq!(spec_of(&parse(&spec)), Some(spec.clone()), "{kind:?} reads back");
            let r = chart_plot_rect(&spec, &THEME_COLORS, 300.0, 200.0).unwrap();
            // The same plot rectangle as columns of the same data on the same scale (categories between ticks).
            let cols = ChartSpec { kind: if kind == ChartType::Area { ChartType::Column } else { ChartType::ColumnStacked }, ..spec.clone() };
            assert_eq!(Some(r), chart_plot_rect(&cols, &THEME_COLORS, 300.0, 200.0), "{kind:?}");
            // Every area's top edge starts on the value axis and ends on the plot's right edge.
            let items = chart_items(&spec, &THEME_COLORS, 300.0, 200.0);
            let fills: Vec<&Vec<PathSeg>> =
                items.iter().filter_map(|it| if let GraphicItem::Path { segs, gradient: Some(_), .. } = it { Some(segs) } else { None }).collect();
            assert_eq!(fills.len(), 3);
            for segs in fills {
                let xs: Vec<f32> = segs
                    .iter()
                    .take(4)
                    .filter_map(|s| match s {
                        PathSeg::Move(x, _) | PathSeg::Line(x, _) => Some(*x),
                        _ => None,
                    })
                    .collect();
                assert!((xs[0] - r[0]).abs() < 1e-3 && (xs[3] - r[0] - r[2]).abs() < 1e-3, "{kind:?}: {xs:?} in {r:?}");
            }
        }
        for kind in [ChartType::Line, ChartType::Column, ChartType::Bar, ChartType::BarStacked] {
            assert_eq!(cross(&ChartSpec::sample(kind)).as_deref(), Some("between"), "{kind:?} keeps its categories between ticks");
        }
    }

    #[test]
    fn the_plot_rect_is_the_drawn_plot_area() {
        let (w, h) = (360.0, 220.0);
        let inside = |r: [f32; 4]| r[0] >= 0.0 && r[1] >= 0.0 && r[2] > 0.0 && r[3] > 0.0 && r[0] + r[2] <= w && r[1] + r[3] <= h;
        let axis = Rgb(0x33, 0x33, 0x33);
        for kind in [ChartType::Column, ChartType::Line, ChartType::AreaStacked, ChartType::Scatter] {
            let spec = ChartSpec { title: Some("T".into()), legend: Some(LegendPos::Right), ..ChartSpec::sample(kind) };
            let r = chart_plot_rect(&spec, &THEME_COLORS, w, h).unwrap();
            assert!(inside(r), "{kind:?}: {r:?}");
            // The value axis line is the rect's left edge; the category axis line its bottom.
            let lines = strokes_in(&chart_items(&spec, &THEME_COLORS, w, h), axis);
            let vertical = lines.iter().flatten().find(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() > 20.0).unwrap();
            let horizontal = lines.iter().flatten().find(|(a, b)| a.1 == b.1 && (a.0 - b.0).abs() > 20.0).unwrap();
            assert_eq!(vertical.0.0, r[0], "{kind:?} left edge");
            assert_eq!(horizontal.0.1, r[1] + r[3], "{kind:?} bottom edge");
            assert_eq!((vertical.0.1.min(vertical.1.1), horizontal.1.0.max(horizontal.0.0)), (r[1], r[0] + r[2]), "{kind:?} top, right");
        }
        // Horizontal bars: category axis on the left, value axis along the bottom.
        let bar = ChartSpec::sample(ChartType::Bar);
        let r = chart_plot_rect(&bar, &THEME_COLORS, w, h).unwrap();
        let lines = strokes_in(&chart_items(&bar, &THEME_COLORS, w, h), axis);
        assert!(lines.iter().flatten().any(|(a, b)| a.0 == r[0] && b.0 == r[0]), "category axis at the left edge");
        assert!(
            lines.iter().flatten().any(|(a, b)| a.1 == r[1] + r[3] && b.1 == r[1] + r[3] && (a.0 - b.0).abs() > 20.0),
            "value axis at the bottom"
        );
        // Pie and doughnut: the circle's bounding square, every slice inside it and touching it.
        for kind in [ChartType::Pie, ChartType::Doughnut] {
            let spec = ChartSpec::sample(kind);
            let r = chart_plot_rect(&spec, &THEME_COLORS, w, h).unwrap();
            assert!(inside(r) && (r[2] - r[3]).abs() < 1e-3, "{kind:?}: {r:?}");
            let pts: Vec<(f32, f32)> = chart_items(&spec, &THEME_COLORS, w, h)
                .iter()
                .filter_map(|it| if let GraphicItem::Path { segs, fill: Some(_), .. } = it { Some(segs.clone()) } else { None })
                .flatten()
                .filter_map(|s| match s {
                    PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(.., x, y) => Some((x, y)),
                    PathSeg::Close => None,
                })
                .collect();
            let (x0, x1) = pts.iter().fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), p| (lo.min(p.0), hi.max(p.0)));
            assert!(x0 >= r[0] - 0.01 && x1 <= r[0] + r[2] + 0.01, "{kind:?}: slices {x0}..{x1} in {r:?}");
            let (cx, cy, rad) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, r[2] / 2.0);
            let far = pts.iter().map(|p| (p.0 - cx).hypot(p.1 - cy)).fold(0.0f32, f32::max);
            assert!((far - rad).abs() < 0.01, "{kind:?}: the circle is the square's inscribed circle ({far} vs {rad})");
        }
        // Nothing to draw: no rect.
        assert_eq!(chart_plot_rect(&ChartSpec::sample(ChartType::Column), &THEME_COLORS, 0.0, 100.0), None);
        assert_eq!(chart_plot_rect(&ChartSpec { categories: vec![], series: vec![], ..Default::default() }, &THEME_COLORS, w, h), None);
    }

    #[test]
    fn chart_text_is_near_black_in_the_file_and_in_the_drawing() {
        // The default run colour of a `c:txPr`.
        let txpr = |e: &El| {
            e.child("c:txPr").and_then(|t| t.find("a:defRPr")).and_then(|d| d.find("a:srgbClr")).and_then(|c| c.attr("val")).map(str::to_string)
        };
        let black = Some("1F1F1F".to_string());
        for kind in [ChartType::Column, ChartType::Bar, ChartType::Line, ChartType::Scatter, ChartType::Pie] {
            let spec = ChartSpec { title: Some("Sales".into()), legend: Some(LegendPos::Right), ..ChartSpec::sample(kind) };
            let space = parse(&spec);
            let chart = space.child("c:chart").unwrap();
            let legend = chart.child("c:legend").unwrap();
            assert_eq!(names(legend), ["c:legendPos", "c:overlay", "c:txPr"], "{kind:?}");
            assert_eq!(txpr(legend), black);
            let run = chart.child("c:title").unwrap().find("a:r").unwrap();
            assert_eq!(names(run), ["a:rPr", "a:t"]);
            assert_eq!(run.child("a:rPr").unwrap().find("a:srgbClr").unwrap().attr("val"), Some("1F1F1F"));
            assert_eq!(run.child("a:rPr").unwrap().attr("b"), Some("0"), "not bold (Word bolds titles by default)");
            let plot = plot_of(&space);
            for ax in plot.els().filter(|e| matches!(e.name.as_str(), "c:catAx" | "c:valAx")) {
                assert_eq!(txpr(ax), black, "{kind:?} {}", ax.name);
                let order = names(ax);
                let sp = order.iter().position(|n| *n == "c:spPr").unwrap();
                assert_eq!(order.get(sp + 1), Some(&"c:txPr"), "txPr right after spPr");
            }
            assert_eq!(spec_of(&space), Some(spec.clone().sanitized()), "{kind:?} reads back");
            // Drawn: title, legend and axis labels in the file's colour.
            let items = chart_items(&spec, &THEME_COLORS, 300.0, 200.0);
            let colours: Vec<(&str, Rgb)> = items
                .iter()
                .filter_map(|it| if let GraphicItem::Text { text, color, .. } = it { Some((text.as_str(), *color)) } else { None })
                .collect();
            assert!(!colours.is_empty() && colours.iter().all(|(_, c)| *c == Rgb(0x1F, 0x1F, 0x1F)), "{kind:?}: {colours:?}");
            assert!(colours.iter().any(|(t, _)| *t == "Sales"));
            let bold =
                items.iter().find_map(|it| if let GraphicItem::Text { text, bold, .. } = it { (text == "Sales").then_some(*bold) } else { None });
            assert_eq!(bold, Some(false), "{kind:?}: our title is regular weight");
        }
    }

    #[test]
    fn value_ticks_point_out_of_the_plot_and_gridlines_take_their_colour() {
        let (axis, grid) = (Rgb(0x33, 0x33, 0x33), Rgb(0xC8, 0xC8, 0xC8));
        let spec = ChartSpec { axis_min: Some(0.0), axis_max: Some(6.0), axis_step: Some(2.0), legend: None, ..ChartSpec::sample(ChartType::Column) };
        let items = chart_items(&spec, &THEME_COLORS, 300.0, 200.0);
        let gridlines = strokes_in(&items, grid);
        assert_eq!(gridlines.len(), 4, "a gridline at 0, 2, 4, 6 in the gridline colour");
        let lines = strokes_in(&items, axis);
        // The axis lines, then one path of tick marks: 4 short horizontal pieces left of the
        // value axis, each at a gridline's height.
        let ticks = lines.iter().find(|p| p.len() == 4).unwrap();
        let axis_x = lines.iter().flatten().find(|(a, b)| a.0 == b.0 && (a.1 - b.1).abs() > 50.0).unwrap().0.0;
        for (k, ((x0, y0), (x1, y1))) in ticks.iter().enumerate() {
            assert_eq!((*y0, *x1), (*y1, axis_x), "tick {k} ends on the axis");
            assert!((axis_x - x0 - 4.0).abs() < 1e-3, "tick {k} reaches 4 pt out to the left");
            let grid_y = gridlines[k][0].0.1;
            assert!((y0 - grid_y).abs() < 1e-3, "tick {k} at its gridline");
        }
        // Horizontal bars: the value axis is at the bottom and its ticks point down.
        let bar = ChartSpec { kind: ChartType::Bar, ..spec };
        let items = chart_items(&bar, &THEME_COLORS, 300.0, 200.0);
        let lines = strokes_in(&items, axis);
        let ticks = lines.iter().find(|p| p.len() == 4).unwrap();
        let base_y = lines.iter().flatten().find(|(a, b)| a.1 == b.1 && (a.0 - b.0).abs() > 50.0).unwrap().0.1;
        for ((x0, y0), (x1, y1)) in ticks {
            assert_eq!(x0, x1);
            assert!((y0 - base_y).abs() < 1e-3 && (y1 - base_y - 4.0).abs() < 1e-3, "down from the axis");
        }
    }

    #[test]
    fn round_legend_keys_are_kept_in_the_extension_and_drawn_as_circles() {
        // Legend keys: (is a circle, width, height) of the SWATCH-sized shapes.
        let keys = |spec: &ChartSpec| -> Vec<(bool, f32, f32)> {
            chart_items(spec, &THEME_COLORS, 300.0, 200.0)
                .iter()
                .filter_map(|it| match it {
                    GraphicItem::Shape { kind, rect, .. } if rect[2] == 7.0 && rect[3] == 7.0 => {
                        Some((*kind == wordcraft_doc::para::ShapeKind::Ellipse, rect[2], rect[3]))
                    }
                    _ => None,
                })
                .collect()
        };
        for kind in [ChartType::Column, ChartType::Line, ChartType::Pie, ChartType::Doughnut, ChartType::AreaStacked] {
            let square = ChartSpec::sample(kind);
            assert!(!chart_xml(&square).contains("c:extLst"), "{kind:?}: square keys need no extension");
            let k = keys(&square);
            assert!(!k.is_empty() && k.iter().all(|(round, ..)| !round), "{kind:?}: squares by default");
            let round = ChartSpec { legend_shape: LegendShape::Circle, ..square };
            let k = keys(&round);
            assert!(!k.is_empty() && k.iter().all(|(round, ..)| *round), "{kind:?}: circles");
            // Kept in the extension (no palette needed) and read back.
            let x = chart_xml(&round);
            assert!(x.contains(r#"<wc:legendKey xmlns:wc="urn:wordcraft:chart" shape="circle"/>"#), "{x}");
            assert!(srgb(parse(&round).child("c:extLst").unwrap()).is_empty(), "no palette in the extension");
            assert_eq!(spec_of(&parse(&round)), Some(round.clone()), "{kind:?}");
        }
        // With a palette too: both in the one extension.
        let both = ChartSpec { legend_shape: LegendShape::Circle, colors: vec!["123456".into()], ..ChartSpec::sample(ChartType::Bar) };
        let space = parse(&both);
        let ext = space.child("c:extLst").unwrap().child("c:ext").unwrap();
        assert_eq!(names(ext), ["a:srgbClr", "?:legendKey"]);
        assert_eq!(spec_of(&space), Some(both));
        // A chart from another program has square keys.
        let foreign = chart_xml(&ChartSpec::sample(ChartType::Column))
            .replace("<c:roundedCorners val=\"0\"/>", "<c:roundedCorners val=\"0\"/><c:style val=\"2\"/>");
        let foreign = xml::parse(foreign.as_bytes()).unwrap();
        assert!(!round_legend_keys(&foreign));
    }

    #[test]
    fn a_series_gap_separates_the_bars_of_a_category() {
        // x range of each filled bar, in drawing order (series by series, category by category).
        let bars = |gap: f64| -> Vec<(f32, f32)> {
            let spec = ChartSpec { series_gap: gap, legend: None, colors: vec!["111111".into()], ..ChartSpec::sample(ChartType::Column) };
            chart_items(&spec, &THEME_COLORS, 400.0, 200.0)
                .iter()
                .filter_map(|it| match it {
                    GraphicItem::Path { segs, fill: Some(Rgb(0x11, 0x11, 0x11)), .. } => {
                        let xs = segs.iter().filter_map(|s| if let PathSeg::Line(x, _) | PathSeg::Move(x, _) = s { Some(*x) } else { None });
                        let (lo, hi) = xs.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), x| (lo.min(x), hi.max(x)));
                        Some((lo, hi))
                    }
                    _ => None,
                })
                .collect()
        };
        let (touch, apart) = (bars(0.0), bars(50.0));
        assert_eq!((touch.len(), apart.len()), (12, 12));
        // Series 1 and 2 of the first category: touching without a gap, half a bar apart with 50%.
        let (a, b) = (touch[0], touch[4]);
        assert!((b.0 - a.1).abs() < 0.01, "{a:?} {b:?}");
        let (a, b) = (apart[0], apart[4]);
        let thick = a.1 - a.0;
        assert!(((b.0 - a.1) - thick * 0.5).abs() < 0.01, "{a:?} {b:?}");
        // The whole cluster and its gaps stay inside the first category's band.
        let c = apart[8];
        let band = apart[1].0 - apart[0].0;
        assert!(c.1 - a.0 < band, "cluster {} wider than the band {band}", c.1 - a.0);
        assert!(apart.iter().all(|(lo, hi)| lo < hi));
    }

    #[test]
    fn charts_the_model_has_no_place_for_stay_read_only() {
        let spec = ChartSpec::sample(ChartType::Column);
        let adopt_of = |x: &str| adopt(&xml::parse(x.as_bytes()).unwrap());
        // Markup the model has no place for is left out, not refused; numbers read as numbers.
        let x = chart_xml(&spec).replace("<c:roundedCorners val=\"0\"/>", "<c:roundedCorners val=\"0\"/><c:style val=\"2\"/>");
        assert_eq!(adopt_of(&x), Ok(spec.clone()));
        let x = chart_xml(&spec).replace("<c:v>3.2</c:v>", "<c:v>3.20</c:v>");
        assert_eq!(adopt_of(&x).map(|s| s.series[0].values[0]), Ok(Some(3.2)));
        // A combination of other kinds (stacked columns with lines), values not in the part.
        let x = chart_xml(&ChartSpec::sample(ChartType::ColumnStacked))
            .replace("</c:barChart>", "</c:barChart><c:lineChart><c:grouping val=\"standard\"/></c:lineChart>");
        assert!(adopt_of(&x).is_err());
        let x = chart_xml(&ChartSpec::sample(ChartType::ColumnStacked)).replace("val=\"stacked\"", "val=\"percentStacked\"");
        assert_eq!(adopt_of(&x).map(|s| s.kind), Ok(ChartType::ColumnPercent));
        let x = chart_xml(&ChartSpec::sample(ChartType::Line)).replace("val=\"standard\"", "val=\"stacked\"");
        assert_eq!(adopt_of(&x).map(|s| s.kind), Ok(ChartType::LineStacked));
        let x = chart_xml(&spec);
        let a = x.find("<c:numCache>").unwrap();
        let b = x.find("</c:numCache>").unwrap() + "</c:numCache>".len();
        assert!(adopt_of(&format!("{}{}", &x[..a], &x[b..])).is_err(), "a series whose cells aren't cached");
        // A chart type the model doesn't have.
        let x = chart_xml(&spec).replace("c:barChart", "c:bar3DChart");
        assert!(adopt_of(&x).is_err());
    }

    /// A chart part as another program saves one of ours: the same chart with its own extras (a
    /// language, a style, a date system, per-series ids, label and axis details), our extension
    /// dropped and the series colours kept on the series.
    fn word_style(spec: &ChartSpec) -> String {
        let ser_ext = r#"<c:extLst><c:ext uri="{C3380CC4-5D6E-409C-BE32-E72D297353CC}" xmlns:c16="http://schemas.microsoft.com/office/drawing/2014/chart"><c16:uniqueId val="{00000000-0000-0000-0000-000000000001}"/></c:ext></c:extLst></c:ser>"#;
        let dlbls = r#"<c:dLbls><c:spPr><a:noFill/><a:ln><a:noFill/></a:ln><a:effectLst/></c:spPr><c:showLegendKey val="0"/><c:showVal val="1"/><c:showCatName val="0"/><c:showSerName val="0"/><c:showPercent val="0"/><c:showBubbleSize val="0"/><c:showLeaderLines val="0"/><c:extLst><c:ext uri="{CE6537A1-D6FC-4f65-9D91-7224C49458BB}" xmlns:c15="http://schemas.microsoft.com/office/drawing/2012/chart"><c15:showLeaderLines val="0"/></c:ext></c:extLst></c:dLbls>"#;
        let mut x = without_ext(&chart_xml(spec))
            .replace(r#"<c:roundedCorners val="0"/>"#, r#"<c:date1904 val="0"/><c:lang val="ko-KR"/><c:roundedCorners val="0"/><c:style val="2"/>"#)
            .replace("</c:ser>", ser_ext)
            .replace(r#"<c:dispBlanksAs val="gap"/>"#, r#"<c:dispBlanksAs val="gap"/><c:showDLblsOverMax val="1"/>"#)
            .replace(r#"<c:majorTickMark val="none"/>"#, r#"<c:numFmt formatCode="General" sourceLinked="0"/><c:majorTickMark val="none"/>"#);
        if spec.data_labels {
            // Labels on every series too.
            x = x.replace("<c:cat>", &format!("{dlbls}<c:cat>"));
        }
        x
    }

    /// `xml` without our chart extension.
    fn without_ext(xml: &str) -> String {
        match (xml.find("<c:extLst><c:ext uri=\"{5C0F2A47"), xml.rfind("</c:extLst>")) {
            (Some(a), Some(b)) if b > a => format!("{}{}", &xml[..a], &xml[b + "</c:extLst>".len()..]),
            _ => xml.to_string(),
        }
    }

    #[test]
    fn charts_saved_by_another_program_are_adopted_with_what_the_model_has() {
        for kind in [ChartType::Column, ChartType::Line, ChartType::Pie, ChartType::AreaStacked] {
            let spec = ChartSpec {
                title: Some("Sales".into()),
                data_labels: true,
                colors: vec!["112233".into(), "445566".into(), "778899".into()],
                legend_shape: LegendShape::Circle,
                gridlines: Gridlines::Dashed,
                axis_min: Some(0.0),
                axis_max: Some(8.0),
                axis_step: Some(2.0),
                val_labels: false,
                ..ChartSpec::sample(kind)
            }
            .sanitized();
            let x = word_style(&spec);
            assert!(x.contains("c:date1904") && x.contains("c16:uniqueId") && !x.contains("5C0F2A47"), "{kind:?}");
            let got = adopt(&xml::parse(x.as_bytes()).unwrap()).unwrap_or_else(|e| panic!("{kind:?}: {e}"));
            // What our extension held is gone: the legend keys are square again, and the palette is
            // the series' own colours (a pie: its slices', one per category).
            let mut want = ChartSpec { legend_shape: LegendShape::Square, ..spec.clone() };
            if kind.is_round() {
                want.colors = (0..spec.points()).map(|i| spec.colors[i % spec.colors.len()].clone()).collect();
                (want.gridlines, want.axis_min, want.axis_max, want.axis_step, want.val_labels) = (Gridlines::Solid, None, None, None, true);
            } else {
                want.colors = (0..spec.series.len()).map(|j| spec.colors[j % spec.colors.len()].clone()).collect();
            }
            assert_eq!(got, want, "{kind:?}");
            // Saved by us: our own markup, which reads back as the same model, with a fresh workbook.
            assert_eq!(spec_of(&parse(&got)), Some(got.clone()), "{kind:?} round trip");
            let book = cells(&chart_workbook(&got).unwrap());
            assert_eq!(book.get("B1").map(String::as_str), Some(got.series[0].name.as_str()));
        }
    }

    /// Lists which chart parts of the `.docx` at `WORDCRAFT_CHART_SURVEY` the model adopts, and why
    /// not: `WORDCRAFT_CHART_SURVEY=path cargo test -p wordcraft-docx --lib chart_survey -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads a file named by the environment"]
    fn chart_survey() {
        let Ok(path) = std::env::var("WORDCRAFT_CHART_SURVEY") else { return };
        let parts = unzip(&std::fs::read(path).unwrap());
        for (name, bytes) in parts.iter().filter(|(n, _)| n.starts_with("word/charts/chart") && n.ends_with(".xml")) {
            match xml::parse(bytes).map_err(|e| e.to_string()).and_then(|s| adopt(&s).map_err(str::to_string)) {
                Ok(s) => println!(
                    "{name}: adopted {:?}, {} series × {} points, colours {:?}, labels {}",
                    s.kind,
                    s.series.len(),
                    s.points(),
                    s.colors,
                    s.data_labels
                ),
                Err(e) => println!("{name}: read-only ({e})"),
            }
        }
    }

    #[test]
    fn elements_come_in_schema_order() {
        let mut spec = ChartSpec::sample(ChartType::ColumnStacked);
        spec.title = Some("T".into());
        spec.data_labels = true;
        let space = parse(&spec);
        assert_eq!(names(&space), ["c:roundedCorners", "c:chart", "c:spPr", "c:externalData"]);
        let chart = space.child("c:chart").unwrap();
        assert_eq!(names(chart), ["c:title", "c:autoTitleDeleted", "c:plotArea", "c:legend", "c:plotVisOnly", "c:dispBlanksAs"]);
        let plot = chart.child("c:plotArea").unwrap();
        assert_eq!(names(plot), ["c:layout", "c:barChart", "c:catAx", "c:valAx", "c:spPr"]);
        let bar = plot.child("c:barChart").unwrap();
        assert_eq!(
            names(bar),
            ["c:barDir", "c:grouping", "c:varyColors", "c:ser", "c:ser", "c:ser", "c:dLbls", "c:gapWidth", "c:overlap", "c:axId", "c:axId"]
        );
        assert_eq!(names(bar.child("c:ser").unwrap()), ["c:idx", "c:order", "c:tx", "c:invertIfNegative", "c:cat", "c:val"]);
        let val_ax = plot.child("c:valAx").unwrap();
        assert_eq!(
            names(val_ax),
            [
                "c:axId",
                "c:scaling",
                "c:delete",
                "c:axPos",
                "c:majorGridlines",
                "c:majorTickMark",
                "c:minorTickMark",
                "c:tickLblPos",
                "c:spPr",
                "c:txPr",
                "c:crossAx",
                "c:crosses",
                "c:crossBetween"
            ]
        );
        let line = parse(&ChartSpec { data_labels: true, ..ChartSpec::sample(ChartType::Line) });
        let lc = line.child("c:chart").unwrap().child("c:plotArea").unwrap().child("c:lineChart").unwrap();
        assert_eq!(names(lc), ["c:grouping", "c:varyColors", "c:ser", "c:ser", "c:ser", "c:dLbls", "c:marker", "c:axId", "c:axId"]);
        assert_eq!(names(lc.child("c:ser").unwrap()), ["c:idx", "c:order", "c:tx", "c:marker", "c:cat", "c:val", "c:smooth"]);
        let sc = parse(&ChartSpec::sample(ChartType::Scatter));
        let sc = sc.child("c:chart").unwrap().child("c:plotArea").unwrap().child("c:scatterChart").unwrap();
        assert_eq!(names(sc.child("c:ser").unwrap()), ["c:idx", "c:order", "c:tx", "c:spPr", "c:xVal", "c:yVal", "c:smooth"]);
        let dn = parse(&ChartSpec::sample(ChartType::Doughnut));
        let dn = dn.child("c:chart").unwrap().child("c:plotArea").unwrap();
        assert_eq!(names(dn), ["c:layout", "c:doughnutChart", "c:spPr"]);
        assert_eq!(names(dn.child("c:doughnutChart").unwrap()), ["c:varyColors", "c:ser", "c:firstSliceAng", "c:holeSize"]);
    }

    /// A chart with every property-panel setting changed.
    fn looked(kind: ChartType) -> ChartSpec {
        let text = |f: &str, size: f32, color: &str, bold: bool| TextStyle {
            font: Some(f.into()),
            size: Some(size),
            color: Some(color.into()),
            bold: Some(bold),
        };
        ChartSpec {
            title: Some("Sales".into()),
            legend: Some(LegendPos::Top),
            data_labels: true,
            legend_style: text("Noto Sans", 12.0, "ff0000", true),
            data_labels_style: text("Noto Serif", 10.0, "00ff00", true),
            cat_labels_style: text("Noto Sans KR", 11.0, "0000ff", false),
            val_labels_style: text("Noto Sans JP", 8.5, "123456", true),
            title_style: text("Noto Sans", 20.0, "654321", true),
            data_labels_outline: true,
            cat_label_pos: CatLabelPos::Rotate,
            cat_axis_color: Some("aa0000".into()),
            val_axis_color: Some("00aa00".into()),
            cat_ticks: true,
            val_ticks: false,
            background: true,
            background_color: Some("fafad2".into()),
            line_width: Some(4.5),
            area_opacity: 0.4,
            opacity: 0.6,
            smooth: true,
            markers: true,
            ..ChartSpec::sample(kind)
        }
        .sanitized()
    }

    /// The `a:defRPr` of an element's `c:txPr`.
    fn def_rpr(e: &El) -> &El {
        e.child("c:txPr").and_then(|t| t.find("a:defRPr")).unwrap_or_else(|| panic!("{} has no default run properties", e.name))
    }

    fn texts(items: &[GraphicItem]) -> Vec<(&str, f32, bool, Rgb, Option<&str>, f32)> {
        items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Text { text, size, bold, color, font, rotation, .. } => {
                    Some((text.as_str(), *size, *bold, *color, font.as_deref(), *rotation))
                }
                _ => None,
            })
            .collect()
    }

    #[test]
    fn property_panel_settings_are_written_as_standard_markup() {
        let space = parse(&looked(ChartType::Line));
        let chart = space.child("c:chart").unwrap();
        let plot = plot_of(&space);
        // Text looks: size in 1/100 pt, weight, colour, Latin and East Asian font.
        let legend = def_rpr(chart.child("c:legend").unwrap());
        assert_eq!((legend.attr("sz"), legend.attr("b")), (Some("1200"), Some("1")));
        assert_eq!(srgb(legend), ["ff0000"]);
        assert_eq!(legend.child("a:latin").and_then(|f| f.attr("typeface")), Some("Noto Sans"));
        assert_eq!(legend.child("a:ea").and_then(|f| f.attr("typeface")), Some("Noto Sans"));
        let title = chart.child("c:title").unwrap().find("a:rPr").unwrap();
        assert_eq!((title.attr("sz"), title.attr("b")), (Some("2000"), Some("1")));
        let cat = plot.child("c:catAx").unwrap();
        let val = plot.child("c:valAx").unwrap();
        assert_eq!(def_rpr(val).attr("sz"), Some("850"));
        // Slanted category labels, tick marks, axis line colours.
        assert_eq!(cat.child("c:txPr").unwrap().child("a:bodyPr").unwrap().attr("rot"), Some("-2700000"));
        assert_eq!(val.child("c:txPr").unwrap().child("a:bodyPr").unwrap().attr("rot"), None);
        assert_eq!(cat.child("c:majorTickMark").unwrap().attr("val"), Some("out"));
        assert_eq!(val.child("c:majorTickMark").unwrap().attr("val"), Some("none"));
        assert_eq!(srgb(cat.child("c:spPr").unwrap()), ["aa0000"]);
        assert_eq!(srgb(val.child("c:spPr").unwrap()), ["00aa00"]);
        // Data labels: their look with a white 2 pt outline.
        let dl = def_rpr(plot.child("c:lineChart").unwrap().child("c:dLbls").unwrap());
        let ln = dl.child("a:ln").expect("an outline");
        assert_eq!((ln.attr("w"), srgb(ln)), (Some("25400"), vec!["FFFFFF"]));
        assert!(ln.child("a:round").is_some());
        assert_eq!(names(dl), ["a:ln", "a:solidFill", "a:latin", "a:ea"], "ECMA-376 order");
        // The plot area in the colour chosen, opaque; the series line 4.5 pt.
        let bg = plot.child("c:spPr").unwrap().child("a:solidFill").unwrap().child("a:srgbClr").unwrap();
        assert_eq!((bg.attr("val"), bg.child("a:alpha")), (Some("fafad2"), None));
        let ser = plot.child("c:lineChart").unwrap().child("c:ser").unwrap();
        assert_eq!(ser.child("c:spPr").unwrap().child("a:ln").unwrap().attr("w"), Some("57150"));
        // The opacity in our extension (the format has none for a whole chart).
        let look = ext_look_el(&space).expect("wc:look");
        assert_eq!((look.attr("opacity"), look.attr("smooth")), (Some("0.6"), None), "smooth lines are in the format");
        // An area: its opacity in the gradient, its edge width, its smoothing and dots in our extension.
        let area = parse(&looked(ChartType::Area));
        let ser = plot_of(&area).child("c:areaChart").unwrap().child("c:ser").unwrap();
        let alphas: Vec<&str> = ser.find("a:gsLst").unwrap().els().filter_map(|g| g.find("a:alpha")).filter_map(|a| a.attr("val")).collect();
        assert_eq!(alphas, ["40000", "7500"]);
        assert_eq!(ser.child("c:spPr").unwrap().child("a:ln").unwrap().attr("w"), Some("57150"));
        let look = ext_look_el(&area).unwrap();
        assert_eq!((look.attr("smooth"), look.attr("markers")), (Some("1"), Some("1")));
        // Defaults write what they did before: no extension look, the faint tint, the text colour.
        let plain = ChartSpec { background: true, ..ChartSpec::sample(ChartType::Line) };
        let p = parse(&plain);
        assert!(ext_look_el(&p).is_none());
        let bg = plot_of(&p).child("c:spPr").unwrap().child("a:solidFill").unwrap().child("a:srgbClr").unwrap();
        assert_eq!((bg.attr("val"), bg.child("a:alpha").and_then(|a| a.attr("val"))), (Some("000000"), Some("3500")));
        let legend = def_rpr(p.child("c:chart").unwrap().child("c:legend").unwrap());
        assert_eq!((legend.attr("sz"), legend.child("a:latin"), srgb(legend)), (None, None, vec!["1F1F1F"]));
        let cat = plot_of(&p).child("c:catAx").unwrap();
        assert_eq!(cat.child("c:txPr").unwrap().child("a:bodyPr").unwrap().attr("rot"), None);
        let a = parse(&ChartSpec::sample(ChartType::Area));
        let alphas: Vec<&str> = a.find("a:gsLst").unwrap().els().filter_map(|g| g.find("a:alpha")).filter_map(|a| a.attr("val")).collect();
        assert_eq!(alphas, ["80000", "15000"]);
    }

    #[test]
    fn property_panel_settings_read_back() {
        for kind in [ChartType::Line, ChartType::Area, ChartType::Column, ChartType::Scatter, ChartType::Pie] {
            let spec = looked(kind);
            let back = spec_of(&parse(&spec)).unwrap_or_else(|| panic!("{kind:?} reads back"));
            let mut want = spec.clone();
            if !matches!(kind, ChartType::Line | ChartType::Area) {
                (want.smooth, want.markers, want.line_width) = (false, false, None);
            }
            if kind != ChartType::Area {
                want.area_opacity = wordcraft_doc::chart::DEFAULT_AREA_OPACITY;
            }
            if kind.is_round() {
                // No axes: nothing of theirs is written.
                (want.cat_labels_style, want.val_labels_style) = (TextStyle::default(), TextStyle::default());
                (want.cat_label_pos, want.cat_axis_color, want.val_axis_color, want.cat_ticks, want.val_ticks) =
                    (CatLabelPos::Auto, None, None, false, true);
            }
            assert_eq!(back, want, "{kind:?}");
            // And written again the same.
            assert_eq!(chart_xml(&back), chart_xml(&want), "{kind:?}");
        }
        // Word's "automatic" turn (-60000000) leaves the labels level.
        let x = chart_xml(&looked(ChartType::Column)).replace("rot=\"-2700000\"", "rot=\"-60000000\"");
        assert_eq!(spec_of(&xml::parse(x.as_bytes()).unwrap()).map(|s| s.cat_label_pos), Some(CatLabelPos::Auto));
    }

    #[test]
    fn property_panel_settings_change_the_drawing() {
        let at_full = |kind| ChartSpec { opacity: 1.0, ..looked(kind) };
        let items = chart_items(&at_full(ChartType::Line), &THEME_COLORS, 400.0, 300.0);
        let t = texts(&items);
        let red = Rgb(0xff, 0, 0);
        // Legend: its size, weight, colour and font.
        let legend: Vec<_> = t.iter().filter(|x| x.0.starts_with("Series")).collect();
        assert!(!legend.is_empty());
        assert!(legend.iter().all(|x| x.1 == 12.0 && x.2 && x.3 == red && x.4 == Some("Noto Sans")), "{legend:?}");
        let title = t.iter().find(|x| x.0 == "Sales").unwrap();
        assert_eq!((title.1, title.2, title.3, title.4), (20.0, true, Rgb(0x65, 0x43, 0x21), Some("Noto Sans")));
        // Category labels slanted up, in their look.
        let q1 = t.iter().find(|x| x.0 == "Q1").unwrap();
        assert_eq!((q1.1, q1.3, q1.4, q1.5), (11.0, Rgb(0, 0, 0xff), Some("Noto Sans KR"), -45.0));
        // Data labels: their look over a white outline (white copies of each label under it).
        let green = Rgb(0, 0xff, 0);
        let label = t.iter().find(|x| x.3 == green).expect("a data label");
        assert_eq!((label.1, label.2, label.4), (10.0, true, Some("Noto Serif")));
        let halos = t.iter().filter(|x| x.0 == label.0 && x.3 == Rgb::WHITE).count();
        assert!(halos >= 8, "{halos} white copies of {:?}", label.0);
        // Category tick marks in the category axis colour (5 between 4 categories), the series
        // line 4.5 pt, the plot area's colour.
        let dark_red = Rgb(0xaa, 0, 0);
        let ticks = items.iter().any(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == dark_red && segs.len() == 10));
        assert!(ticks, "category tick marks");
        assert!(items.iter().any(|it| matches!(it, GraphicItem::Path { stroke: Some(_), stroke_width, .. } if *stroke_width == 4.5)));
        assert!(items.iter().any(|it| matches!(it, GraphicItem::Path { fill: Some(c), gradient: None, .. } if *c == Rgb(0xfa, 0xfa, 0xd2))));
        // Bigger and slanted text takes more room: the plot is smaller.
        let plain = ChartSpec { title: Some("Sales".into()), legend: Some(LegendPos::Top), ..ChartSpec::sample(ChartType::Line) };
        let small = chart_plot_rect(&plain, &THEME_COLORS, 400.0, 300.0).unwrap();
        let big = chart_plot_rect(&at_full(ChartType::Line), &THEME_COLORS, 400.0, 300.0).unwrap();
        assert!(big[1] > small[1] + 10.0 && big[3] < small[3] - 20.0, "{small:?} {big:?}");
        // An area: a smooth top edge, dots on its points, its gradient at its opacity.
        let area = chart_items(&at_full(ChartType::Area), &THEME_COLORS, 400.0, 300.0);
        let curved =
            area.iter().filter(|it| matches!(it, GraphicItem::Path { segs, .. } if segs.iter().any(|s| matches!(s, PathSeg::Cubic(..))))).count();
        assert!(curved >= 3, "smooth fills and edges: {curved}");
        let dots = area
            .iter()
            .filter(|it| matches!(it, GraphicItem::Shape { kind: wordcraft_doc::para::ShapeKind::Ellipse, rect, .. } if rect[2] == 5.0))
            .count();
        assert_eq!(dots, 3 * 4);
        let top_alpha = area.iter().find_map(|it| match it {
            GraphicItem::Path { gradient: Some(g), .. } if g.stops.len() == 2 && g.stops[0].alpha != g.stops[1].alpha => Some(g.stops[0].alpha),
            _ => None,
        });
        assert!(top_alpha.is_some_and(|a| (a - 0.4).abs() < 1e-4), "{top_alpha:?}");
        // The whole chart at 60%: colours toward white, gradients more see-through.
        let faded = chart_items(&looked(ChartType::Area), &THEME_COLORS, 400.0, 300.0);
        assert_eq!(faded.len(), area.len());
        let mix = |v: u8| (f32::from(v) * 0.6 + 255.0 * 0.4).round() as u8;
        for (a, b) in area.iter().zip(&faded) {
            match (a, b) {
                (GraphicItem::Text { color: x, .. }, GraphicItem::Text { color: y, .. }) => assert_eq!(*y, Rgb(mix(x.0), mix(x.1), mix(x.2))),
                (GraphicItem::Path { gradient: Some(g), .. }, GraphicItem::Path { gradient: Some(h), .. }) => {
                    for (s, t) in g.stops.iter().zip(&h.stops) {
                        assert!((t.alpha - s.alpha * 0.6).abs() < 1e-4);
                    }
                }
                _ => {}
            }
        }
        // Without the settings, nothing of theirs: no slant, no outline copies, no dots.
        let plain = chart_items(&ChartSpec { data_labels: true, ..ChartSpec::sample(ChartType::Area) }, &THEME_COLORS, 400.0, 300.0);
        assert!(texts(&plain).iter().all(|x| x.5 == 0.0 && x.3 != Rgb::WHITE));
        assert!(!plain.iter().any(|it| matches!(it, GraphicItem::Shape { kind: wordcraft_doc::para::ShapeKind::Ellipse, .. })));
    }

    #[test]
    fn area_smoothing_dots_and_opacity_come_back_from_the_document_records() {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let spec = ChartSpec { smooth: true, markers: true, opacity: 0.5, ..ChartSpec::sample(ChartType::Area) }.sanitized();
        let mut doc = wordcraft_doc::Document::new();
        let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec.clone())), ..Default::default() };
        let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float: Default::default(), graphic: Arc::new(graphic) };
        doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
        let bytes = crate::write(&doc).unwrap();
        let records = unzip(&bytes)
            .into_iter()
            .find(|(n, b)| n.starts_with("customXml/item") && String::from_utf8_lossy(b).contains("wordcraftCharts"))
            .unwrap();
        let text = String::from_utf8_lossy(&records.1).into_owned();
        assert!(text.contains(r#"smooth="1""#) && text.contains(r#"markers="1""#) && text.contains(r#"opacity="0.5""#), "{text}");
        let elsewhere = crate::package::zip_entries(&as_saved_elsewhere(&bytes).into_iter().collect::<Vec<_>>()).unwrap();
        let back = crate::read(&elsewhere).unwrap();
        assert_eq!(charts_of(&back), std::slice::from_ref(&spec));
        // Read straight back (our extension there) too.
        assert_eq!(charts_of(&crate::read(&bytes).unwrap()), [spec]);
    }

    #[test]
    fn a_locked_chart_is_written_with_frame_locks_and_read_back_locked() {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let spec = ChartSpec::sample(ChartType::Column);
        let mut doc = wordcraft_doc::Document::new();
        let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec)), ..Default::default() };
        let float = wordcraft_doc::para::Float { locked: true, ..Default::default() };
        let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float, graphic: Arc::new(graphic) };
        doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
        let bytes = crate::write(&doc).unwrap();
        let xml = String::from_utf8(unzip(&bytes)["word/document.xml"].clone()).unwrap();
        assert!(xml.contains(r#"<wp:cNvGraphicFramePr><a:graphicFrameLocks noMove="1" noResize="1"/></wp:cNvGraphicFramePr>"#), "{xml}");
        let locked = |d: &wordcraft_doc::Document| {
            let p = d.para_at(&Pos::body(0, 0)).unwrap();
            p.object_offsets().into_iter().find_map(|o| p.object_at(o).and_then(|o| o.frame()).map(|(_, _, f)| f.locked))
        };
        assert_eq!(locked(&crate::read(&bytes).unwrap()), Some(true));
        // Saved elsewhere (our chart extension gone), the lock stays: it is standard markup.
        let elsewhere = crate::package::zip_entries(&as_saved_elsewhere(&bytes).into_iter().collect::<Vec<_>>()).unwrap();
        assert_eq!(locked(&crate::read(&elsewhere).unwrap()), Some(true));
        // Only noResize (as another program may write it) is locked too; none is not.
        let one = xml.replace(r#"noMove="1" noResize="1""#, r#"noResize="1""#);
        let none = xml.replace(r#"noMove="1" noResize="1""#, r#"noMove="0""#);
        for (text, want) in [(one, true), (none, false)] {
            let mut parts = unzip(&bytes);
            parts.insert("word/document.xml".into(), text.into_bytes());
            let pkg = crate::package::zip_entries(&parts.into_iter().collect::<Vec<_>>()).unwrap();
            assert_eq!(locked(&crate::read(&pkg).unwrap()), Some(want));
        }
        // An unlocked chart has an empty frame properties element, as before.
        let mut plain = doc.clone();
        let para = plain.para_mut(wordcraft_doc::StoryRef::Body, &Pos::body(0, 0).path).unwrap();
        let off = para.object_offsets()[0];
        if let Some(o) = para.object_at_mut(off).and_then(|o| o.float_mut()) {
            o.locked = false;
        }
        let xml = String::from_utf8(unzip(&crate::write(&plain).unwrap())["word/document.xml"].clone()).unwrap();
        assert!(xml.contains("<wp:cNvGraphicFramePr/>") && !xml.contains("Locks noMove"), "{xml}");
    }

    /// Paths filled with a plain colour (no gradient).
    fn filled_paths(items: &[GraphicItem]) -> Vec<(&Vec<PathSeg>, Rgb)> {
        items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Path { segs, fill: Some(c), stroke: None, gradient: None, .. } => Some((segs, *c)),
                _ => None,
            })
            .collect()
    }

    /// The bounding box of path segments: (x0, y0, x1, y1).
    fn bbox(segs: &[PathSeg]) -> (f32, f32, f32, f32) {
        let pts = segs.iter().filter_map(|s| match *s {
            PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(_, _, _, _, x, y) => Some((x, y)),
            _ => None,
        });
        pts.fold((f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY), |(a, b, c, d), (x, y)| {
            (a.min(x), b.min(y), c.max(x), d.max(y))
        })
    }

    /// The chart type elements of `k`'s sample, in order.
    fn plot_types(k: ChartType) -> String {
        let space = parse(&ChartSpec::sample(k));
        let p = plot_of(&space);
        names(p).into_iter().filter(|n| n.ends_with("Chart")).map(str::to_string).collect::<Vec<_>>().join(",")
    }

    /// `val` of `child` in the first `el` of `k`'s sample.
    fn plot_attr(k: ChartType, el: &str, child: &str) -> Option<String> {
        let space = parse(&ChartSpec::sample(k));
        plot_of(&space).find(el).and_then(|e| e.child(child)).and_then(|c| c.attr("val")).map(str::to_string)
    }

    /// A series' cached values as text.
    fn ser_vals(s: &El) -> Vec<String> {
        literal(s.child("c:val").unwrap())
    }

    #[test]
    fn stacked_and_percent_types_are_written_with_their_grouping() {
        let (plot, attr) = (plot_types, plot_attr);
        for (k, el, grouping) in [
            (ChartType::ColumnPercent, "c:barChart", "percentStacked"),
            (ChartType::BarPercent, "c:barChart", "percentStacked"),
            (ChartType::AreaPercent, "c:areaChart", "percentStacked"),
            (ChartType::LinePercent, "c:lineChart", "percentStacked"),
            (ChartType::LineStacked, "c:lineChart", "stacked"),
        ] {
            assert_eq!(plot(k), el, "{k:?}");
            assert_eq!(attr(k, el, "c:grouping").as_deref(), Some(grouping), "{k:?}");
        }
        // 100% stacked value axes show percentages.
        let space = parse(&ChartSpec::sample(ChartType::ColumnPercent));
        assert_eq!(plot_of(&space).child("c:valAx").unwrap().child("c:numFmt").unwrap().attr("formatCode"), Some("0%"));
    }

    #[test]
    fn scatter_and_radar_styles_are_written_as_their_types_say() {
        let (plot, attr) = (plot_types, plot_attr);
        // Scatter styles follow the type and the markers setting.
        let style = |k, markers| {
            let space = parse(&ChartSpec { markers, ..ChartSpec::sample(k) });
            let sc = plot_of(&space).child("c:scatterChart").unwrap().child("c:scatterStyle").unwrap().attr("val").unwrap().to_string();
            let smooth = plot_of(&space).find("c:smooth").and_then(|s| s.attr("val")).unwrap().to_string();
            (sc, smooth)
        };
        assert_eq!(style(ChartType::Scatter, false), ("marker".into(), "0".into()));
        assert_eq!(style(ChartType::ScatterLines, false), ("line".into(), "0".into()));
        assert_eq!(style(ChartType::ScatterLines, true), ("lineMarker".into(), "0".into()));
        assert_eq!(style(ChartType::ScatterSmooth, false), ("smooth".into(), "1".into()));
        assert_eq!(style(ChartType::ScatterSmooth, true), ("smoothMarker".into(), "1".into()));
        for (k, st) in [(ChartType::Radar, "standard"), (ChartType::RadarMarkers, "marker"), (ChartType::RadarFilled, "filled")] {
            assert_eq!(plot(k), "c:radarChart");
            assert_eq!(attr(k, "c:radarChart", "c:radarStyle").as_deref(), Some(st));
        }
    }

    #[test]
    fn bubble_and_of_pie_types_are_written_with_their_settings() {
        let (plot, attr) = (plot_types, plot_attr);
        // Bubbles: sizes from their own workbook column, by area at the default scale.
        let space = parse(&ChartSpec::sample(ChartType::Bubble));
        let bc = plot_of(&space).child("c:bubbleChart").unwrap();
        assert_eq!(bc.child("c:sizeRepresents").unwrap().attr("val"), Some("area"));
        assert_eq!(bc.child("c:bubbleScale").unwrap().attr("val"), Some("100"));
        let ser = bc.child("c:ser").unwrap();
        assert_eq!(names(ser).iter().filter(|n| matches!(**n, "c:xVal" | "c:yVal" | "c:bubbleSize")).count(), 3);
        assert_eq!(ser.child("c:bubbleSize").unwrap().find("c:f").unwrap().text(), "Sheet1!$C$2:$C$5");
        assert_eq!(bc.children("c:ser").nth(1).unwrap().child("c:yVal").unwrap().find("c:f").unwrap().text(), "Sheet1!$D$2:$D$5");
        // Pie of pie and bar of pie: the second plot takes the last points by position.
        for (k, t) in [(ChartType::PieOfPie, "pie"), (ChartType::BarOfPie, "bar")] {
            assert_eq!(plot(k), "c:ofPieChart");
            assert_eq!(attr(k, "c:ofPieChart", "c:ofPieType").as_deref(), Some(t));
            assert_eq!(attr(k, "c:ofPieChart", "c:splitType").as_deref(), Some("pos"));
            assert_eq!(attr(k, "c:ofPieChart", "c:splitPos").as_deref(), Some("2"));
        }
        let space = parse(&ChartSpec { split_pos: 3, ..ChartSpec::sample(ChartType::PieOfPie) }.sanitized());
        assert_eq!(plot_of(&space).find("c:splitPos").unwrap().attr("val"), Some("3"));
    }

    #[test]
    fn waterfalls_and_funnels_are_written_as_stacked_bars() {
        let plot = plot_types;
        let vals = ser_vals;
        // A waterfall: stacked columns over an invisible base, increases, decreases and a total,
        // the base out of the legend, our extension saying what it is.
        let space = parse(&ChartSpec::sample(ChartType::Waterfall));
        assert_eq!(plot(ChartType::Waterfall), "c:barChart");
        let bar = plot_of(&space).child("c:barChart").unwrap();
        assert_eq!(bar.child("c:grouping").unwrap().attr("val"), Some("stacked"));
        let sers: Vec<&El> = bar.children("c:ser").collect();
        assert_eq!(sers.len(), 4);
        let sp = sers[0].child("c:spPr").unwrap();
        assert!(sp.child("a:noFill").is_some() && sp.child("a:ln").unwrap().child("a:noFill").is_some(), "the base is invisible");
        assert_eq!(sers[0].child("c:idx").unwrap().attr("val"), Some("3"));
        let entry = space.child("c:chart").unwrap().child("c:legend");
        assert!(entry.is_none(), "the sample has no legend");
        let with_legend = parse(&ChartSpec { legend: Some(LegendPos::Top), ..ChartSpec::sample(ChartType::Waterfall) });
        let le = with_legend.child("c:chart").unwrap().child("c:legend").unwrap().child("c:legendEntry").unwrap();
        assert_eq!((le.child("c:idx").unwrap().attr("val"), le.child("c:delete").unwrap().attr("val")), (Some("3"), Some("1")));
        let look = ext_look_el(&space).unwrap();
        assert_eq!((look.attr("kind"), look.attr("total")), (Some("waterfall"), Some("1")));
        // 10, +6, -2, +3, -4 then a total of 13.
        assert_eq!(vals(sers[0]), ["0", "10", "14", "14", "13", ""]);
        assert_eq!(vals(sers[1]), ["10", "6", "", "3", "", ""]);
        assert_eq!(vals(sers[2]), ["", "", "2", "", "4", ""]);
        assert_eq!(vals(sers[3]), ["", "", "", "", "", "13"]);
        // A funnel: stacked bars, an invisible offset centring them, the first stage on top.
        let space = parse(&ChartSpec::sample(ChartType::Funnel));
        let bar = plot_of(&space).child("c:barChart").unwrap();
        assert_eq!(bar.child("c:barDir").unwrap().attr("val"), Some("bar"));
        let sers: Vec<&El> = bar.children("c:ser").collect();
        assert_eq!(vals(sers[1]), ["120", "310", "620", "1000"]);
        assert_eq!(vals(sers[0]), ["440", "345", "190", "0"]);
        assert_eq!(ext_look_el(&space).unwrap().attr("kind"), Some("funnel"));
    }

    #[test]
    fn combo_charts_are_written_as_groups_on_their_axes() {
        let plot = plot_types;
        // A combo chart: columns, then a line on a secondary axis at the right.
        let space = parse(&ChartSpec::sample(ChartType::Combo));
        assert_eq!(plot(ChartType::Combo), "c:barChart,c:lineChart");
        let p = plot_of(&space);
        let axes: Vec<(&str, Option<&str>)> =
            p.els().filter(|e| e.name.ends_with("Ax")).map(|a| (a.name.as_str(), a.child("c:axPos").and_then(|x| x.attr("val")))).collect();
        assert_eq!(axes, [("c:catAx", Some("b")), ("c:valAx", Some("l")), ("c:catAx", Some("b")), ("c:valAx", Some("r"))]);
        let second_cat = p.children("c:catAx").nth(1).unwrap();
        assert_eq!(second_cat.child("c:delete").unwrap().attr("val"), Some("1"));
        assert_eq!(p.children("c:valAx").nth(1).unwrap().child("c:crosses").unwrap().attr("val"), Some("max"));
        let line = p.child("c:lineChart").unwrap();
        assert_eq!(line.children("c:axId").nth(1).unwrap().attr("val"), Some(VAL_AX2));
        // Stacked areas with clustered columns: the areas first (at the back).
        let area_combo = ChartSpec {
            series: vec![
                ChartSeries { kind: Some(SeriesKind::Area), ..ChartSeries::of("A", vec![Some(1.0); 4]) },
                ChartSeries { kind: Some(SeriesKind::Area), ..ChartSeries::of("B", vec![Some(2.0); 4]) },
                ChartSeries::of("C", vec![Some(3.0); 4]),
            ],
            ..ChartSpec::sample(ChartType::Combo)
        }
        .sanitized();
        let space = parse(&area_combo);
        assert_eq!(names(plot_of(&space)).into_iter().filter(|n| n.ends_with("Chart")).collect::<Vec<_>>(), ["c:areaChart", "c:barChart"]);
        assert_eq!(plot_of(&space).child("c:areaChart").unwrap().child("c:grouping").unwrap().attr("val"), Some("stacked"));
        assert_eq!(spec_of(&space), Some(area_combo));
    }

    #[test]
    fn new_types_round_trip_through_a_document_and_from_word_style_files() {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        let new = ChartType::ALL.iter().copied().skip(10).collect::<Vec<_>>();
        for k in &new {
            let spec = ChartSpec { title: Some("T".into()), ..ChartSpec::sample(*k) }.sanitized();
            let back = spec_of(&parse(&spec)).unwrap_or_else(|| panic!("{k:?} reads back"));
            assert_eq!(back, spec, "{k:?}");
            // Through a saved document, and saved elsewhere (our extension gone: the records).
            let mut doc = wordcraft_doc::Document::new();
            let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec.clone())), ..Default::default() };
            let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float: Default::default(), graphic: Arc::new(graphic) };
            doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
            let bytes = crate::write(&doc).unwrap();
            assert_eq!(charts_of(&crate::read(&bytes).unwrap()), std::slice::from_ref(&spec), "{k:?} through a document");
            let elsewhere = crate::package::zip_entries(&as_saved_elsewhere(&bytes).into_iter().collect::<Vec<_>>()).unwrap();
            assert_eq!(charts_of(&crate::read(&elsewhere).unwrap()), std::slice::from_ref(&spec), "{k:?} saved elsewhere");
            // And its workbook has the sheet the formulas name.
            let wb = chart_workbook(&spec).unwrap();
            assert!(!cells(&wb).is_empty(), "{k:?}");
        }
        // A combo as Word writes it: a line group first, then the columns, its own axis ids.
        let word = format!(
            r#"<c:chartSpace {ns}><c:chart><c:plotArea>
            <c:lineChart><c:grouping val="standard"/><c:ser><c:idx val="1"/><c:order val="1"/><c:tx><c:v>L</c:v></c:tx><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>5</c:v></c:pt><c:pt idx="1"><c:v>6</c:v></c:pt></c:numLit></c:val></c:ser><c:axId val="7"/><c:axId val="8"/></c:lineChart>
            <c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>B</c:v></c:tx><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:val></c:ser><c:axId val="3"/><c:axId val="4"/></c:barChart>
            <c:catAx><c:axId val="3"/><c:axPos val="b"/></c:catAx><c:valAx><c:axId val="4"/><c:axPos val="l"/></c:valAx>
            <c:catAx><c:axId val="7"/><c:delete val="1"/><c:axPos val="b"/></c:catAx><c:valAx><c:axId val="8"/><c:axPos val="r"/></c:valAx>
            </c:plotArea></c:chart></c:chartSpace>"#,
            ns =
                r#"xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#
        );
        let c = adopt(&xml::parse(word.as_bytes()).unwrap()).unwrap();
        assert_eq!(c.kind, ChartType::Combo);
        assert_eq!(
            c.series.iter().map(|s| (s.name.as_str(), s.kind, s.secondary)).collect::<Vec<_>>(),
            [("B", None, false), ("L", Some(SeriesKind::Line), true)]
        );
        // Word's plain radar: radarStyle marker with no marker symbol.
        let radar = chart_xml(&ChartSpec::sample(ChartType::Radar)).replace("val=\"standard\"", "val=\"marker\"");
        assert_eq!(spec_of(&xml::parse(radar.as_bytes()).unwrap()).map(|s| s.kind), Some(ChartType::Radar));
        // A scatter with lines from elsewhere: no marker-only line, so straight lines.
        let x = chart_xml(&ChartSpec::sample(ChartType::Scatter)).replace(r#"<a:ln w="19050"><a:noFill/></a:ln>"#, "");
        assert_eq!(spec_of(&xml::parse(x.as_bytes()).unwrap()).map(|s| s.kind), Some(ChartType::ScatterLines));
    }

    #[test]
    fn new_types_draw_what_they_are() {
        let items = |spec: &ChartSpec| chart_items(spec, &THEME_COLORS, 400.0, 300.0);
        let text_list = |it: &[GraphicItem]| -> Vec<String> {
            it.iter().filter_map(|i| if let GraphicItem::Text { text, .. } = i { Some(text.clone()) } else { None }).collect()
        };
        // 100% stacked: every category's stack reaches the same height; the axis says %.
        let pc = items(&ChartSpec { legend: None, ..ChartSpec::sample(ChartType::ColumnPercent) });
        assert!(text_list(&pc).iter().any(|t| t == "100%"), "{:?}", text_list(&pc));
        let tops: Vec<f32> = filled_paths(&pc).iter().map(|(s, _)| bbox(s).1).collect();
        let top = tops.iter().copied().fold(f32::INFINITY, f32::min);
        assert_eq!(tops.iter().filter(|t| (**t - top).abs() < 0.01).count(), 4, "one full stack per category");
        // Stacked lines: the second line above the first everywhere.
        let ls = items(&ChartSpec::sample(ChartType::LineStacked));
        let lines: Vec<(f32, f32, f32, f32)> = ls
            .iter()
            .filter_map(|i| {
                if let GraphicItem::Path { segs, stroke: Some(_), stroke_width, .. } = i { (*stroke_width > 2.0).then(|| bbox(segs)) } else { None }
            })
            .collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[1].1 < lines[0].1 && lines[2].1 < lines[1].1);
        // Smooth scatter lines are curves; straight ones aren't.
        let curved = |k| {
            items(&ChartSpec::sample(k))
                .iter()
                .any(|i| matches!(i, GraphicItem::Path { segs, stroke: Some(_), .. } if segs.iter().any(|s| matches!(s, PathSeg::Cubic(..)))))
        };
        assert!(curved(ChartType::ScatterSmooth) && !curved(ChartType::ScatterLines));
        // Radar: a polygon per series; filled radar fills them.
        let filled = filled_paths(&items(&ChartSpec::sample(ChartType::RadarFilled))).len();
        assert!(filled >= 2, "{filled}");
        // Bubbles: one circle a point, the biggest a quarter of the plot's shorter side across.
        // See-through by default: the fill at 60% (an even gradient) over a solid outline.
        let see = items(&ChartSpec::sample(ChartType::Bubble));
        let translucent: Vec<_> = see
            .iter()
            .filter_map(|i| match i {
                GraphicItem::Path { stroke: Some(_), gradient: Some(g), .. } => g.stops.first().map(|s| s.alpha),
                _ => None,
            })
            .collect();
        assert!(translucent.len() >= 8 && translucent.iter().all(|a| (a - 0.6).abs() < 1e-3), "{translucent:?}");
        // Opaque: plain circles.
        let spec = ChartSpec { bubble_opacity: 1.0, ..ChartSpec::sample(ChartType::Bubble) };
        let b = items(&spec);
        let circles: Vec<f32> = b
            .iter()
            .filter_map(|i| if let GraphicItem::Shape { kind: wordcraft_doc::para::ShapeKind::Ellipse, rect, .. } = i { Some(rect[2]) } else { None })
            .collect();
        let plot = chart_plot_rect(&spec, &THEME_COLORS, 400.0, 300.0).unwrap();
        let big = circles.iter().copied().fold(0.0, f32::max);
        assert!(circles.len() >= 8 && (big - 0.25 * plot[2].min(plot[3])).abs() < 0.5, "{circles:?} {plot:?}");
        // A waterfall: no base drawn, each bar starting where the one before ended.
        let wf = items(&ChartSpec::sample(ChartType::Waterfall));
        let bars: Vec<(f32, f32, f32, f32)> = filled_paths(&wf).iter().map(|(s, _)| bbox(s)).filter(|b| b.2 - b.0 > 5.0).collect();
        let plot_area = chart_plot_rect(&ChartSpec::sample(ChartType::Waterfall), &THEME_COLORS, 400.0, 300.0).unwrap();
        let bars: Vec<_> = bars.into_iter().filter(|b| b.3 <= plot_area[1] + plot_area[3] + 0.5 && b.1 >= plot_area[1] - 0.5).collect();
        assert_eq!(bars.len(), 6, "{bars:?}");
        assert!((bars[1].3 - bars[0].1).abs() < 0.5, "the second step starts at the first's top");
        // A funnel: centred bars, the widest on top.
        let fu = items(&ChartSpec::sample(ChartType::Funnel));
        let fb: Vec<(f32, f32, f32, f32)> = filled_paths(&fu).iter().map(|(s, _)| bbox(s)).collect();
        assert_eq!(fb.len(), 4);
        let centres: Vec<f32> = fb.iter().map(|b| (b.0 + b.2) / 2.0).collect();
        assert!(centres.iter().all(|c| (c - centres[0]).abs() < 0.5), "{centres:?}");
        let top = fb.iter().min_by(|a, b| a.1.total_cmp(&b.1)).unwrap();
        assert!(fb.iter().all(|b| b.2 - b.0 <= top.2 - top.0 + 0.01));
        // A combo: columns, a line, and labels at the right for the secondary axis.
        let spec = ChartSpec::sample(ChartType::Combo);
        let co = items(&spec);
        let plot = chart_plot_rect(&spec, &THEME_COLORS, 400.0, 300.0).unwrap();
        assert_eq!(filled_paths(&co).len(), 8, "two column series");
        assert!(co.iter().any(|i| matches!(i, GraphicItem::Path { stroke: Some(_), stroke_width, .. } if *stroke_width > 2.0)), "a line");
        let right = co.iter().filter(|i| matches!(i, GraphicItem::Text { rect, .. } if rect[0] > plot[0] + plot[2])).count();
        assert!(right >= 3, "secondary axis labels: {right}");
        // Pie of pie and bar of pie: main slices, the joined one in its points' colours (a part
        // each), the second plot, two lines.
        for (k, second) in [(ChartType::PieOfPie, 2), (ChartType::BarOfPie, 2)] {
            let it = items(&ChartSpec { legend: None, ..ChartSpec::sample(k) });
            let fills = filled_paths(&it).len();
            assert_eq!(fills, 4 + second + second, "{k:?}");
            let joins = it.iter().filter(|i| matches!(i, GraphicItem::Path { stroke: Some(_), fill: None, segs, .. } if segs.len() == 2)).count();
            assert_eq!(joins, 2, "{k:?}");
        }
    }

    #[test]
    fn radar_gradients_are_written_filled_and_read_back_as_their_type() {
        use std::sync::Arc;
        use wordcraft_doc::graphic::{Graphic, GraphicKind};
        use wordcraft_doc::{CharProps, InlineObject, Pos};
        for k in [ChartType::Radar, ChartType::RadarMarkers, ChartType::RadarFilled] {
            let spec = ChartSpec { radar_gradient: true, colors: vec!["336699".into()], ..ChartSpec::sample(k) }.sanitized();
            assert_eq!(spec.line_width, Some(0.75), "radar outlines a hairline by default");
            let space = parse(&spec);
            let rc = plot_of(&space).child("c:radarChart").unwrap();
            assert_eq!(rc.child("c:radarStyle").unwrap().attr("val"), Some("filled"), "{k:?}: a fill needs a filled radar");
            let sp = rc.child("c:ser").unwrap().child("c:spPr").unwrap();
            let grad = sp.child("a:gradFill").expect("a gradient fill");
            assert_eq!(grad.child("a:path").unwrap().attr("path"), Some("circle"));
            let alphas: Vec<&str> = grad.find("a:gsLst").unwrap().els().filter_map(|g| g.find("a:alpha")).filter_map(|a| a.attr("val")).collect();
            assert_eq!(alphas, ["8000", "35000"], "light at the centre, stronger at the edge");
            let ln = sp.child("a:ln").unwrap();
            assert_eq!((ln.attr("w"), srgb(ln)), (Some("9525"), vec!["336699"]), "the outline at the line width");
            let marker = rc.child("c:ser").unwrap().child("c:marker").and_then(|m| m.child("c:symbol")).and_then(|s| s.attr("val"));
            assert_eq!(marker, if k == ChartType::RadarMarkers { Some("circle") } else { None }, "{k:?}");
            // The kind in our extension for the line radars written filled.
            let kind = ext_look_el(&space).and_then(|l| l.attr("kind"));
            assert_eq!(kind, (k != ChartType::RadarFilled).then_some(k.id()), "{k:?}");
            assert_eq!(spec_of(&space), Some(spec.clone()), "{k:?} reads back");
            // Saved elsewhere (our extension gone): the document record keeps the kind.
            let mut doc = wordcraft_doc::Document::new();
            let graphic = Graphic { kind: GraphicKind::Chart, w: 300.0, h: 200.0, chart: Some(Arc::new(spec.clone())), ..Default::default() };
            let obj = InlineObject::Graphic { w: 300.0, h: 200.0, alt: String::new(), float: Default::default(), graphic: Arc::new(graphic) };
            doc.insert_object(&Pos::body(0, 0), obj, &CharProps::default()).unwrap();
            let bytes = crate::write(&doc).unwrap();
            let elsewhere = crate::package::zip_entries(&as_saved_elsewhere(&bytes).into_iter().collect::<Vec<_>>()).unwrap();
            assert_eq!(charts_of(&crate::read(&elsewhere).unwrap()), std::slice::from_ref(&spec), "{k:?} saved elsewhere");
            // Drawn: a circular gradient from the centre in the series colour, its outline, and
            // dots for the radar with markers.
            let items = chart_items(&spec, &THEME_COLORS, 400.0, 300.0);
            let grads: Vec<&wordcraft_doc::graphic::LinearGradient> =
                items.iter().filter_map(|i| if let GraphicItem::Path { gradient: Some(g), .. } = i { Some(g) } else { None }).collect();
            assert_eq!(grads.len(), 2, "{k:?}: one a series");
            assert!(grads.iter().all(|g| g.radial && (g.stops[0].alpha - 0.08).abs() < 1e-3 && (g.stops[1].alpha - 0.35).abs() < 1e-3));
            assert!(grads.iter().all(|g| g.stops[0].color == Rgb(0x33, 0x66, 0x99)));
            let outlines = items.iter().filter(|i| matches!(i, GraphicItem::Path { stroke: Some(c), stroke_width, .. } if *c == Rgb(0x33, 0x66, 0x99) && (*stroke_width - 0.75).abs() < 1e-3)).count();
            assert!(outlines >= 1, "{k:?}");
            let dots = items.iter().filter(|i| matches!(i, GraphicItem::Shape { kind: wordcraft_doc::para::ShapeKind::Ellipse, .. })).count();
            assert_eq!(dots > 0, k == ChartType::RadarMarkers, "{k:?}");
        }
        // Without it: as before (no gradient; a filled radar's plain fill).
        let plain = chart_items(&ChartSpec::sample(ChartType::RadarFilled), &THEME_COLORS, 400.0, 300.0);
        assert!(!plain.iter().any(|i| matches!(i, GraphicItem::Path { gradient: Some(_), .. })));
        assert_eq!(plot_attr(ChartType::Radar, "c:radarChart", "c:radarStyle").as_deref(), Some("standard"));
        // Only radars keep it.
        assert!(!ChartSpec { radar_gradient: true, ..ChartSpec::sample(ChartType::Line) }.sanitized().radar_gradient);
    }

    #[test]
    fn radar_labels_sit_over_the_series_with_ring_values_outlined() {
        let spec = ChartSpec { colors: vec!["202020".into(), "303030".into()], legend: None, ..ChartSpec::sample(ChartType::RadarFilled) };
        let items = chart_items(&spec, &THEME_COLORS, 400.0, 300.0);
        let last_shape = items
            .iter()
            .rposition(
                |i| matches!(i, GraphicItem::Path { fill: Some(c), .. } | GraphicItem::Path { stroke: Some(c), .. } if *c == Rgb(0x30, 0x30, 0x30)),
            )
            .expect("the series shapes");
        // The web (light grey) comes before the shapes.
        let web = items.iter().position(
            |i| matches!(i, GraphicItem::Path { stroke: Some(c), fill: None, .. } if *c != Rgb(0x20, 0x20, 0x20) && *c != Rgb(0x30, 0x30, 0x30)),
        );
        assert!(web.is_some_and(|w| w < last_shape));
        let texts: Vec<(usize, &str, Rgb)> = items
            .iter()
            .enumerate()
            .filter_map(|(k, i)| if let GraphicItem::Text { text, color, .. } = i { Some((k, text.as_str(), *color)) } else { None })
            .collect();
        assert!(!texts.is_empty() && texts.iter().all(|t| t.0 > last_shape), "every label after the shapes");
        // Each ring value over eight white copies of itself; category names without.
        for v in ["0", "1", "2", "3", "4", "5"] {
            let white = texts.iter().filter(|t| t.1 == v && t.2 == Rgb::WHITE).count();
            let dark = texts.iter().filter(|t| t.1 == v && t.2 != Rgb::WHITE).count();
            assert_eq!((white, dark), (8, 1), "ring value {v}");
        }
        assert!(!texts.iter().any(|t| t.1 == "Speed" && t.2 == Rgb::WHITE));
    }

    #[test]
    fn radar_ring_values_centre_on_the_spoke_and_the_top_name_clears_them() {
        for (w, h) in [(400.0, 300.0), (240.0, 160.0)] {
            let spec = ChartSpec { legend: None, ..ChartSpec::sample(ChartType::RadarFilled) };
            let items = chart_items(&spec, &THEME_COLORS, w, h);
            let texts: Vec<(&str, [f32; 4], Rgb)> = items
                .iter()
                .filter_map(|i| if let GraphicItem::Text { text, rect, color, .. } = i { Some((text.as_str(), *rect, *color)) } else { None })
                .collect();
            // The ring values (not their white outline copies): centred on one vertical line.
            let rings: Vec<[f32; 4]> = texts.iter().filter(|t| t.2 != Rgb::WHITE && t.0.parse::<f64>().is_ok()).map(|t| t.1).collect();
            assert!(rings.len() >= 3, "{rings:?}");
            let centre = |r: &[f32; 4]| r[0] + r[2] / 2.0;
            assert!(rings.iter().all(|r| (centre(r) - centre(&rings[0])).abs() < 0.01), "{rings:?}");
            assert!((centre(&rings[0]) - w / 2.0).abs() < 0.5, "on the spoke through the centre");
            // The top category name at least 4 pt above the outermost ring value.
            let outer = rings.iter().min_by(|a, b| a[1].total_cmp(&b[1])).unwrap();
            let top = texts.iter().find(|t| t.0 == "Speed").unwrap().1;
            assert!(outer[1] - (top[1] + top[3]) >= 4.0, "{w}x{h}: name {top:?}, ring {outer:?}");
            // Nothing outside the chart.
            for (t, r, _) in &texts {
                assert!(r[0] >= -0.5 && r[1] >= -0.5 && r[0] + r[2] <= w + 0.5 && r[1] + r[3] <= h + 0.5, "{w}x{h}: {t} at {r:?}");
            }
        }
    }
}
