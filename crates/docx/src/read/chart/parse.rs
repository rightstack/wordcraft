use super::format::{date_text, excel_stamp, format_value, is_date_code};
use super::model::{Axis, Grouping, Kind, LabelPos, Look, Marker, Pos, Series, Ticks};
use super::plot::label;
use super::{
    Chart, El, FONT, LABEL_DARK, LINE_W, MAX_CELLS, MAX_POINTS, MAX_RADAR_SERIES, MAX_SERIES, MAX_SPOKES, Rgb, color_alpha, dml_fill, fill_colour,
    int, measure, slot,
};

impl Chart {
    pub(super) fn parse(space: &El, theme: &[Rgb]) -> Option<Chart> {
        let chart = space.child("c:chart")?;
        let plot = chart.child("c:plotArea")?;
        let types: Vec<&El> = plot.els().filter(|e| Kind::of(&e.name).is_some()).take(8).collect();
        let ty = *types.first()?;
        // Columns, lines and areas together (a combo chart): every group's series, each with its
        // group and value axis, in their order.
        let combo = types.len() > 1 && types.iter().all(|t| matches!(Kind::of(&t.name), Some(Kind::Bar | Kind::Line | Kind::Area)));
        let kind = if combo && types.iter().any(|t| Kind::of(&t.name) == Some(Kind::Bar)) { Kind::Bar } else { Kind::of(&ty.name)? };
        let scatter = matches!(kind, Kind::Scatter | Kind::Bubble);
        let date1904 = space.child("c:date1904").is_some_and(|d| d.attr("val").is_none_or(truthy));
        let primary_ax = plot.children("c:valAx").next().and_then(|a| a.child("c:axId")).and_then(|a| a.attr("val"));
        let groups: &[&El] = if combo { &types } else { &types[..1] };
        let mut ordered: Vec<(i64, Series)> = Vec::new();
        for t in groups {
            let group = Kind::of(&t.name).unwrap_or(kind);
            let axis = t.children("c:axId").nth(1).and_then(|a| a.attr("val"));
            let secondary = combo && axis.is_some() && axis != primary_ax;
            let orders: Vec<i64> =
                t.children("c:ser").map(|s| s.child("c:order").and_then(|o| o.attr("val")).and_then(int).unwrap_or(i64::MAX)).collect();
            for (k, mut s) in read_series(t, scatter, theme).into_iter().enumerate() {
                (s.group, s.secondary) = (group, secondary);
                ordered.push((orders.get(k).copied().unwrap_or(i64::MAX), s));
            }
        }
        if combo {
            ordered.sort_by_key(|o| o.0);
        }
        let mut series: Vec<Series> = ordered.into_iter().map(|o| o.1).take(MAX_SERIES).collect();
        if series.is_empty() {
            return None;
        }
        let val2 = combo.then(|| plot.children("c:valAx").nth(1).map(|a| axis(Some(a), theme))).flatten();
        let legend_deleted: Vec<usize> = chart
            .child("c:legend")
            .map(|l| {
                l.children("c:legendEntry")
                    .filter(|e| e.child("c:delete").and_then(|d| d.attr("val")).is_some_and(truthy))
                    .filter_map(|e| e.child("c:idx").and_then(|i| i.attr("val")).and_then(int).and_then(|i| usize::try_from(i).ok()))
                    .take(MAX_SERIES)
                    .collect()
            })
            .unwrap_or_default();
        let split = ty.child("c:splitPos").and_then(|e| e.attr("val")).and_then(int).unwrap_or(2).clamp(1, MAX_SPOKES as i64) as usize;
        let of_bar = ty.child("c:ofPieType").and_then(|e| e.attr("val")) == Some("bar");
        let cats = ty.children("c:ser").find_map(|s| s.child("c:cat")).map(|c| labels(Some(c), MAX_POINTS, date1904)).unwrap_or_default();
        let n = if kind == Kind::Radar {
            series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(cats.len()).min(MAX_SPOKES)
        } else if matches!(kind, Kind::Pie | Kind::Doughnut | Kind::OfPie) {
            series.first().map_or(0, |s| s.values.len()).max(cats.len())
        } else {
            series.iter().map(|s| s.values.len()).max().unwrap_or(0).max(cats.len())
        };
        // Every series is padded to `n` points, so keep only as many as the cell budget holds.
        if let Some(keep) = MAX_CELLS.checked_div(n) {
            series.truncate(keep.max(1));
        }
        if kind == Kind::Radar {
            series.truncate(MAX_RADAR_SERIES);
        }
        let grouping = match ty.child("c:grouping").and_then(|g| g.attr("val")) {
            _ if combo => Grouping::Clustered,
            Some("stacked") if matches!(kind, Kind::Bar | Kind::Area | Kind::Line) => Grouping::Stacked,
            Some("percentStacked") if matches!(kind, Kind::Bar | Kind::Area | Kind::Line) => Grouping::Percent,
            _ => Grouping::Clustered,
        };
        let mut cats = cats;
        cats.resize(n, String::new());
        let radar_style = ty.child("c:radarStyle").and_then(|r| r.attr("val"));
        let type_labels = show_values(ty.child("c:dLbls"));
        let type_pos = label_pos(ty.child("c:dLbls"));
        for s in &mut series {
            s.values.resize(n, None);
            s.xs.resize(n, None);
            s.labels = Some(s.labels.or(type_labels).unwrap_or(false));
            s.label_pos = s.label_pos.or(type_pos);
            if s.labels == Some(true) {
                s.raw = s.values.clone();
            }
        }
        if grouping == Grouping::Percent {
            to_percent(&mut series, n);
        }
        // An area chart's smooth top edge and dots, from WordCraft's chart extension (the format
        // has them for lines only).
        let ext = crate::chart_spec::ext_look_el(space);
        let ext_on = |n: &str| ext.and_then(|l| l.attr(n)) == Some("1");
        if kind == Kind::Area {
            for s in &mut series {
                s.smooth |= ext_on("smooth");
                if ext_on("markers") && s.marker.is_none() {
                    s.marker = Some(Marker { size: 5.0, square: false, fill: None });
                }
            }
        }
        let opacity = ext
            .and_then(|l| l.attr("opacity"))
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|v| v.is_finite())
            .map_or(1.0, |v| v.clamp(0.0, 1.0));
        let def_rpr = crate::chart_spec::def_rpr_of;
        let data_rpr = def_rpr(ty.child("c:dLbls")).or_else(|| ty.children("c:ser").find_map(|s| def_rpr(s.child("c:dLbls"))));
        let title_el = chart.child("c:title");
        let title_rpr = title_el.and_then(|t| t.child("c:tx")).and_then(|t| t.find("a:rPr")).or_else(|| def_rpr(title_el));
        let halo = data_rpr.and_then(|r| r.child("a:ln")).is_some_and(|l| l.child("a:noFill").is_none() && fill_colour(l, theme).is_some());
        let (cat, val) = chart_axes(plot, scatter, theme);
        let val_fmt = if grouping == Grouping::Percent {
            "0%".to_string()
        } else {
            val.fmt.clone().or_else(|| series.iter().find_map(|s| s.fmt.clone())).unwrap_or_else(|| "General".to_string())
        };
        let num = |n: &str| ty.child(n).and_then(|g| g.attr("val")).and_then(int);
        Some(Chart {
            kind,
            horizontal: kind == Kind::Bar && ty.child("c:barDir").and_then(|d| d.attr("val")) == Some("bar"),
            grouping,
            gap: num("c:gapWidth").unwrap_or(150).clamp(0, 500) as f32 / 100.0,
            overlap: num("c:overlap").unwrap_or(0).clamp(-100, 100) as f32 / 100.0,
            hole: num("c:holeSize").unwrap_or(50).clamp(10, 90) as f32 / 100.0,
            first_angle: num("c:firstSliceAng").unwrap_or(0).clamp(0, 360) as f64,
            radar_fill: radar_style == Some("filled"),
            radar_marks: radar_style == Some("marker") || ext.and_then(|l| l.attr("kind")) == Some("radarMarkers"),
            vary: matches!(kind, Kind::Pie | Kind::Doughnut | Kind::OfPie) && ty.child("c:varyColors").and_then(|v| v.attr("val")).is_none_or(truthy),
            n,
            cats,
            title: title_of(chart, &series),
            series,
            accents: (4..10).map(|i| slot(theme, i)).collect(),
            legend: legend_pos(chart),
            legend_color: txpr_colour(chart.child("c:legend"), theme),
            title_color: title_colour(chart, theme),
            title_bold: title_bold(chart),
            cat,
            val,
            text_size: text_size(space),
            val_fmt,
            date1904,
            plot_border: plot.child("c:spPr").and_then(|p| p.child("a:ln")).and_then(|l| fill_colour(l, theme)),
            plot_fill: plot_fill(plot, theme),
            round_keys: crate::chart_spec::round_legend_keys(space),
            title_look: look_of(title_rpr, theme),
            legend_look: look_of(def_rpr(chart.child("c:legend")), theme),
            data_look: look_of(data_rpr, theme),
            halo,
            opacity,
            combo,
            val2,
            legend_deleted,
            split,
            of_bar,
        })
    }

    /// The dark colour of data labels: the axis text colour the file gives, else near black.
    pub(super) fn label_dark(&self) -> Rgb {
        self.val.text_color.or(self.cat.text_color).unwrap_or(LABEL_DARK)
    }

    pub(super) fn stacked(&self) -> bool {
        self.grouping != Grouping::Clustered
    }

    fn accent(&self, i: usize) -> Rgb {
        self.accents.get(i % 6).copied().unwrap_or(Rgb::BLACK)
    }

    pub(super) fn series_colour(&self, j: usize) -> Rgb {
        let s = self.series.get(j);
        s.and_then(|s| s.color).unwrap_or_else(|| self.accent(s.map_or(j, |s| s.idx)))
    }

    /// Colour of point `i` of series `j`: its `c:dPt` override, else the varied accent (pie) or
    /// the series colour.
    pub(super) fn point_colour(&self, j: usize, i: usize) -> Rgb {
        if let Some(Some(c)) = self.series.get(j).and_then(|s| s.points.get(i)) {
            return *c;
        }
        if self.vary { self.accent(i) } else { self.series_colour(j) }
    }

    pub(super) fn legend_entries(&self) -> Vec<(String, Rgb)> {
        if matches!(self.kind, Kind::Pie | Kind::Doughnut | Kind::OfPie) {
            return self
                .cats
                .iter()
                .enumerate()
                .map(|(i, c)| (if c.is_empty() { (i + 1).to_string() } else { c.clone() }, self.point_colour(0, i)))
                .collect();
        }
        // Helper series and those the legend deletes have no entry.
        self.series
            .iter()
            .enumerate()
            .filter(|(_, s)| !s.hidden && !self.legend_deleted.contains(&s.idx))
            .map(|(j, s)| (if s.name.is_empty() { format!("Series {}", j + 1) } else { s.name.clone() }, self.series_colour(j)))
            .collect()
    }
}

/// The series of a chart type element, within the chart's cell budget.
pub(super) fn read_series(ty: &El, scatter: bool, theme: &[Rgb]) -> Vec<Series> {
    let mut budget = MAX_CELLS;
    let mut series = Vec::new();
    for (k, ser) in ty.children("c:ser").take(MAX_SERIES).enumerate() {
        let mut s = Series::parse(ser, scatter, theme, budget.min(MAX_POINTS));
        if s.idx == usize::MAX {
            s.idx = k;
        }
        budget = budget.saturating_sub(s.values.len().max(s.xs.len()));
        series.push(s);
    }
    series
}

/// Values as their share of each category's total (100% stacked charts).
fn to_percent(series: &mut [Series], n: usize) {
    for i in 0..n {
        let total: f64 = series.iter().map(|s| s.values.get(i).copied().flatten().map_or(0.0, f64::abs)).sum();
        for s in series.iter_mut() {
            if let Some(Some(v)) = s.values.get_mut(i) {
                *v = if total > 0.0 { *v / total } else { 0.0 };
            }
        }
    }
}

/// The category axis (scatter: the horizontal value axis) and the value axis of a plot area.
fn chart_axes(plot: &El, scatter: bool, theme: &[Rgb]) -> (Axis, Axis) {
    let vals_ax: Vec<&El> = plot.children("c:valAx").collect();
    let horiz_pos = |a: &&El| matches!(a.child("c:axPos").and_then(|p| p.attr("val")), Some("b" | "t"));
    let (val_el, x_el) = if scatter {
        let x = vals_ax.iter().copied().find(horiz_pos).or_else(|| vals_ax.first().copied());
        let y = vals_ax.iter().copied().find(|a| !horiz_pos(a)).or_else(|| vals_ax.iter().copied().find(|a| Some(*a) != x));
        (y, x)
    } else {
        (vals_ax.first().copied(), None)
    };
    let cat_el = plot.els().find(|e| matches!(e.name.as_str(), "c:catAx" | "c:dateAx"));
    (axis(if scatter { x_el } else { cat_el }, theme), axis(val_el, theme))
}

/// The chart's title. An empty `c:title` shows the series name (Word's automatic title of a
/// one-series chart) unless the automatic title is deleted.
pub(super) fn title_of(chart: &El, series: &[Series]) -> Option<String> {
    let auto_deleted = chart.child("c:autoTitleDeleted").and_then(|a| a.attr("val")).is_some_and(truthy);
    let auto = if !auto_deleted && series.len() == 1 { series.first().map(|s| s.name.clone()).filter(|s| !s.is_empty()) } else { None };
    chart.child("c:title").and_then(|t| title_text(t).or(auto))
}

/// Whether a `c:dLbls` shows the values (`None` when there's none, or it doesn't say).
/// Where a `c:dLbls` puts its labels (`c:dLblPos`), when it says.
fn label_pos(d: Option<&El>) -> Option<LabelPos> {
    d?.child("c:dLblPos").and_then(|p| p.attr("val")).and_then(LabelPos::of)
}

fn show_values(d: Option<&El>) -> Option<bool> {
    let d = d?;
    if d.child("c:delete").and_then(|e| e.attr("val")).is_some_and(truthy) {
        return Some(false);
    }
    d.child("c:showVal").map(|v| v.attr("val").is_none_or(truthy))
}

fn legend_pos(chart: &El) -> Option<Pos> {
    chart.child("c:legend").map(|l| match l.child("c:legendPos").and_then(|p| p.attr("val")).unwrap_or("r") {
        "l" => Pos::Left,
        "t" | "tr" => Pos::Top,
        "b" => Pos::Bottom,
        _ => Pos::Right,
    })
}

/// The text colour an element's `c:txPr` gives (its default run properties' fill).
/// The plot area's fill and its opacity (0..1): a solid fill with its alpha, else the colour any
/// other fill gives, opaque. `None` for no fill.
fn plot_fill(plot: &El, theme: &[Rgb]) -> Option<(Rgb, f32)> {
    let sp = plot.child("c:spPr")?;
    match sp.child("a:solidFill") {
        Some(f) => color_alpha(f, theme, None).map(|(c, transparency)| (c, (1.0 - transparency / 100.0).clamp(0.0, 1.0))),
        None => fill_colour(sp, theme).map(|c| (c, 1.0)),
    }
}

/// Most text size honoured, points.
const MAX_TEXT_SIZE: f32 = 96.0;

/// How run properties (`a:defRPr`, `a:rPr`) make text look: size (`sz`, 1/100 pt), weight (`b`),
/// colour (their fill) and Latin font (a theme font, `+mn-lt`, is left to the chart's).
pub(super) fn look_of(rpr: Option<&El>, theme: &[Rgb]) -> Look {
    let Some(r) = rpr else { return Look::default() };
    Look {
        size: r.attr("sz").and_then(int).map(|v| (v as f32 / 100.0).clamp(4.0, MAX_TEXT_SIZE)),
        bold: r.attr("b").map(truthy),
        color: fill_colour(r, theme),
        font: r
            .child("a:latin")
            .and_then(|f| f.attr("typeface"))
            .filter(|f| !f.is_empty() && !f.starts_with('+'))
            .map(|f| f.chars().take(64).collect()),
    }
}

pub(super) fn txpr_colour(e: Option<&El>, theme: &[Rgb]) -> Option<Rgb> {
    e?.child("c:txPr")?.find("a:defRPr").and_then(|d| fill_colour(d, theme))
}

/// The colour of a chart title: its first run's own fill, else its default run properties'
/// (in the rich text or the title's `c:txPr`).
/// Whether a chart title is bold: `b` of its first run's properties, else of its default run
/// properties (in the rich text or the title's `c:txPr`); bold when neither says, as Word
/// draws titles.
pub(super) fn title_bold(chart: &El) -> bool {
    let Some(title) = chart.child("c:title") else { return true };
    let rich = title.child("c:tx").and_then(|t| t.child("c:rich"));
    let b = |e: Option<&El>| e.and_then(|e| e.attr("b")).map(truthy);
    b(rich.and_then(|r| r.find("a:rPr")))
        .or_else(|| b(rich.and_then(|r| r.find("a:defRPr"))))
        .or_else(|| b(title.child("c:txPr").and_then(|t| t.find("a:defRPr"))))
        .unwrap_or(true)
}

pub(super) fn title_colour(chart: &El, theme: &[Rgb]) -> Option<Rgb> {
    let title = chart.child("c:title")?;
    let rich = title.child("c:tx").and_then(|t| t.child("c:rich"));
    rich.and_then(|r| r.find("a:rPr"))
        .and_then(|p| fill_colour(p, theme))
        .or_else(|| rich.and_then(|r| r.find("a:defRPr")).and_then(|d| fill_colour(d, theme)))
        .or_else(|| txpr_colour(Some(title), theme))
}

/// The chart's text size (its `c:txPr` default run size), points.
pub(super) fn text_size(space: &El) -> f32 {
    space
        .child("c:txPr")
        .and_then(|t| t.find("a:defRPr"))
        .and_then(|d| d.attr("sz"))
        .and_then(int)
        .map_or(FONT, |v| (v as f32 / 100.0).clamp(6.0, 24.0))
}

impl Series {
    fn parse(ser: &El, scatter: bool, theme: &[Rgb], cap: usize) -> Series {
        let idx = ser.child("c:idx").and_then(|e| e.attr("val")).and_then(int).and_then(|i| usize::try_from(i).ok());
        let name = ser.child("c:tx").and_then(|t| t.find("c:v")).map(|v| label(v.text().trim())).unwrap_or_default();
        let sp = ser.child("c:spPr");
        let ln = sp.and_then(|p| p.child("a:ln"));
        let marker_el = ser.child("c:marker");
        let marker_fill = marker_el.and_then(|m| m.child("c:spPr")).and_then(|p| fill_colour(p, theme));
        let color = sp.and_then(|p| fill_colour(p, theme)).or_else(|| ln.and_then(|l| fill_colour(l, theme))).or(marker_fill);
        let symbol = marker_el.and_then(|m| m.child("c:symbol")).and_then(|s| s.attr("val"));
        let marker = symbol.filter(|s| *s != "none").map(|s| Marker {
            // `c:size` is 2–72 points (ECMA-376 ST_MarkerSize); 5 when the file doesn't say.
            size: marker_el.and_then(|m| m.child("c:size")).and_then(|e| e.attr("val")).and_then(int).unwrap_or(5).clamp(2, 72) as f32,
            square: s == "square",
            fill: marker_fill,
        });
        let smooth = ser.child("c:smooth").is_some_and(|e| e.attr("val").is_none_or(truthy));
        let line_w = ln.and_then(|l| l.attr("w")).and_then(|v| measure(v, 12_700.0)).map_or(LINE_W, |w| w.clamp(0.0, 20.0));
        let line = ln.is_none_or(|l| l.child("a:noFill").is_none());
        let (values, fmt) = numbers(ser.child(if scatter { "c:yVal" } else { "c:val" }), cap);
        let (mut xs, x_fmt) = if scatter { numbers(ser.child("c:xVal"), cap) } else { (Vec::new(), None) };
        if scatter && xs.is_empty() {
            xs = (1..=values.len()).map(|i| Some(i as f64)).collect();
        }
        // A solid fill's alpha (the series', else its first point's), as an opacity.
        let alpha_of = |p: Option<&El>| {
            p.and_then(|p| p.child("a:solidFill")).and_then(|f| color_alpha(f, theme, None)).map(|(_, t)| (1.0 - t / 100.0).clamp(0.0, 1.0))
        };
        let fill_alpha = alpha_of(sp).or_else(|| ser.child("c:dPt").and_then(|d| alpha_of(d.child("c:spPr")))).unwrap_or(1.0);
        let mut points = vec![None; values.len()];
        for dpt in ser.children("c:dPt") {
            let i = dpt.child("c:idx").and_then(|e| e.attr("val")).and_then(int).and_then(|i| usize::try_from(i).ok());
            let c = dpt.child("c:spPr").and_then(|p| fill_colour(p, theme));
            if let Some(slot) = i.and_then(|i| points.get_mut(i)) {
                *slot = c;
            }
        }
        let labels = show_values(ser.child("c:dLbls"));
        let label_pos = label_pos(ser.child("c:dLbls"));
        let rounded = sp.and_then(|p| p.child("a:prstGeom")).and_then(|g| g.attr("prst")) == Some("roundRect");
        // A linear gradient (areas are painted with it), or a path one (radar shapes).
        let gradient = sp.and_then(|p| p.child("a:gradFill")).and_then(|g| dml_fill(g, theme, None)).and_then(|f| match f {
            wordcraft_doc::fill::Fill::Gradient(g) if !g.stops.is_empty() => Some(g),
            _ => None,
        });
        let edge = ln.filter(|l| l.child("a:noFill").is_none()).and_then(|l| fill_colour(l, theme)).map(|c| (c, line_w));
        // No fill and no line: a helper series that only lifts the others (a waterfall's base).
        let hidden = sp.is_some_and(|p| p.child("a:noFill").is_some() && p.child("a:ln").is_none_or(|l| l.child("a:noFill").is_some()));
        let sizes = if scatter { numbers(ser.child("c:bubbleSize"), cap).0 } else { Vec::new() };
        Series {
            name,
            idx: idx.unwrap_or(usize::MAX),
            hidden,
            group: Kind::Bar,
            secondary: false,
            sizes,
            color,
            line_w,
            smooth,
            rounded,
            gradient,
            edge,
            marker,
            line,
            values,
            xs,
            points,
            fill_alpha,
            fmt,
            x_fmt,
            labels,
            label_pos,
            raw: Vec::new(),
        }
    }
}

/// Longest number-format code honoured (the limit spreadsheet apps use); longer ones are ignored.
const MAX_FORMAT_CODE: usize = 255;

/// Category labels of a `c:cat` (string or number cache), capped. Numbers show in the cache's
/// format code (dates included); text and `General` numbers as they are stored.
pub(super) fn labels(el: Option<&El>, cap: usize, date1904: bool) -> Vec<String> {
    let Some(el) = el else { return Vec::new() };
    let Some(cache) = el.find("c:strCache").or_else(|| el.find("c:numCache")).or_else(|| el.find("c:strLit")).or_else(|| el.find("c:numLit")) else {
        return Vec::new();
    };
    let code =
        cache.child("c:formatCode").map(|f| f.text()).filter(|f| !f.is_empty() && f.len() <= MAX_FORMAT_CODE && !f.eq_ignore_ascii_case("General"));
    pts(cache, cap)
        .into_iter()
        .map(|p| {
            let text = p.unwrap_or_default();
            match (code.as_deref(), text.trim().parse::<f64>()) {
                (Some(code), Ok(v)) if v.is_finite() => label(&category_text(v, code, 1.0, date1904)),
                _ => label(&text),
            }
        })
        .collect()
}

/// A number in its format code: a date or time for date codes (serial days, from 1904 when
/// `date1904`), else as [`format_value`] with tick step `step`.
pub(super) fn category_text(v: f64, code: &str, step: f64, date1904: bool) -> String {
    match excel_stamp(v, date1904) {
        Some(t) if is_date_code(code) => date_text(t, code),
        _ => format_value(v, code, step),
    }
}

/// Values of a `c:val`-style element (number cache or literal), capped; with its format code.
pub(super) fn numbers(el: Option<&El>, cap: usize) -> (Vec<Option<f64>>, Option<String>) {
    let Some(cache) = el.and_then(|e| e.find("c:numCache").or_else(|| e.find("c:numLit"))) else {
        return (Vec::new(), None);
    };
    let fmt = cache.child("c:formatCode").map(|f| f.text()).filter(|f| !f.is_empty() && f.len() <= MAX_FORMAT_CODE);
    let vals = pts(cache, cap).into_iter().map(|p| p.and_then(|t| t.trim().parse::<f64>().ok()).filter(|v| v.is_finite())).collect();
    (vals, fmt)
}

/// The `c:pt` texts of a cache by index. Indexes and counts from the file are capped.
fn pts(cache: &El, cap: usize) -> Vec<Option<String>> {
    let declared = cache.child("c:ptCount").and_then(|c| c.attr("val")).and_then(int).unwrap_or(0).clamp(0, cap as i64) as usize;
    let mut out: Vec<Option<String>> = vec![None; declared];
    for p in cache.children("c:pt") {
        let Some(i) = p.attr("idx").and_then(int).and_then(|i| usize::try_from(i).ok()) else { continue };
        if i >= cap {
            continue;
        }
        if out.len() <= i {
            out.resize(i + 1, None);
        }
        if let Some(slot) = out.get_mut(i) {
            *slot = p.child("c:v").map(|v| v.text());
        }
    }
    out
}

pub(super) fn axis(a: Option<&El>, theme: &[Rgb]) -> Axis {
    let Some(a) = a else { return Axis { deleted: true, labels: true, ..Default::default() } };
    let scaling = a.child("c:scaling");
    let grid_ln = a.child("c:majorGridlines").and_then(|g| g.child("c:spPr")).and_then(|p| p.child("a:ln"));
    let axis_ln = a.child("c:spPr").and_then(|p| p.child("a:ln"));
    let width = |l: &El| l.attr("w").and_then(|v| measure(v, 12_700.0)).filter(|w| w.is_finite()).map(|w| w.clamp(0.0, 20.0));
    let num =
        |n: &str| scaling.and_then(|s| s.child(n)).and_then(|e| e.attr("val")).and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite());
    Axis {
        deleted: a.child("c:delete").and_then(|d| d.attr("val")).is_some_and(truthy),
        labels: a.child("c:tickLblPos").and_then(|t| t.attr("val")) != Some("none"),
        grid: a.child("c:majorGridlines").is_some(),
        line: a.child("c:spPr").and_then(|p| p.child("a:ln")).and_then(|l| l.child("a:noFill")).is_none(),
        mid: a.child("c:crossBetween").and_then(|c| c.attr("val")) == Some("midCat"),
        min: num("c:min"),
        max: num("c:max"),
        step: a.child("c:majorUnit").and_then(|e| e.attr("val")).and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0),
        grid_dash: grid_ln
            .is_some_and(|l| l.child("a:custDash").is_some() || l.child("a:prstDash").and_then(|d| d.attr("val")).is_some_and(|v| v != "solid")),
        grid_color: grid_ln.and_then(|l| fill_colour(l, theme)),
        grid_w: grid_ln.and_then(width),
        line_color: axis_ln.and_then(|l| fill_colour(l, theme)),
        line_w: axis_ln.and_then(width),
        text_color: txpr_colour(Some(a), theme),
        look: look_of(a.child("c:txPr").and_then(|t| t.find("a:defRPr")), theme),
        // A turn either way slants the labels up (ECMA-376 ST_Angle, 60,000ths of a degree); a
        // value past a quarter turn (programs write -60000000 for "automatic") leaves them level.
        slant: a
            .child("c:txPr")
            .and_then(|t| t.child("a:bodyPr"))
            .and_then(|b| b.attr("rot"))
            .and_then(int)
            .filter(|r| r.unsigned_abs() <= 5_400_000)
            .map_or(0.0, |r| r.unsigned_abs() as f32 / 60_000.0),
        ticks: match a.child("c:majorTickMark").and_then(|t| t.attr("val")) {
            Some("out") => Ticks::Out,
            Some("in") => Ticks::In,
            Some("cross") => Ticks::Cross,
            _ => Ticks::None,
        },
        fmt: a
            .child("c:numFmt")
            .filter(|f| f.attr("sourceLinked") != Some("1"))
            .and_then(|f| f.attr("formatCode"))
            .filter(|f| f.len() <= MAX_FORMAT_CODE)
            .map(str::to_string),
    }
}

fn title_text(t: &El) -> Option<String> {
    let tx = t.child("c:tx")?;
    let s = match tx.child("c:rich") {
        Some(rich) => {
            let mut s = String::new();
            rich_into(rich, &mut s);
            s
        }
        None => tx.find("c:v").map(|v| v.text()).unwrap_or_default(),
    };
    let s = label(s.trim());
    (!s.is_empty()).then_some(s)
}

/// The `a:t` runs under an element, in order.
fn rich_into(e: &El, s: &mut String) {
    for c in e.els() {
        if c.name == "a:t" {
            s.push_str(&c.text());
        } else {
            rich_into(c, s);
        }
    }
}

fn truthy(v: &str) -> bool {
    matches!(v.trim(), "1" | "true")
}
