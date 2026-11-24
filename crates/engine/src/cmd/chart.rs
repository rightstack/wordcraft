//! Insert › Chart and the Chart Design tab: insert a chart, change its type, data, title, legend
//! and data labels.
//!
//! A chart is an inline [`InlineObject::Graphic`] whose graphic keeps its model
//! ([`ChartSpec`]); it is drawn by the same code that draws charts read from files, and saved as
//! a chart part with its data in it (see `wordcraft_docx::chart_xml`). Charts made by other
//! programs are shown as they are; the chart commands only edit charts with a model.

use std::sync::Arc;

use serde_json::{Value, json};
use wordcraft_doc::chart::{
    CatLabelPos, ChartSeries, ChartSpec, ChartType, Gridlines, LegendPos, LegendShape, MAX_COLORS, MAX_POINTS, MAX_SERIES, SeriesKind, TextStyle,
    axis_number, hex_color,
};
use wordcraft_doc::graphic::{Graphic, GraphicKind};
use wordcraft_doc::para::{Float, InlineObject};
use wordcraft_doc::props::Rgb;

use super::objects::selected;
use super::{delete_selection, sel_result};
use crate::{CmdError, CmdResult, CommandSpec, Selection, Session, p};

/// A new chart's size: 15 × 7.5 cm, points.
pub const DEFAULT_W: f32 = 15.0 / 2.54 * 72.0;
pub const DEFAULT_H: f32 = 7.5 / 2.54 * 72.0;

const TYPES: &str = "column|columnStacked|columnPercent|bar|barStacked|barPercent|line|lineStacked|linePercent|area|areaStacked|areaPercent|pie|doughnut|pieOfPie|barOfPie|scatter|scatterLines|scatterSmooth|bubble|radar|radarMarkers|radarFilled|waterfall|funnel|combo";
const GRIDLINES: &str = "solid|dashed|none";
const LEGEND_SHAPES: &str = "square|circle";
const DATA: &str = r#"{"categories": [string], "series": [{"name": string, "values": [number|null], "sizes"?: [number|null] (bubble), "kind"?: "column|line|area" (combo), "secondary"?: bool (combo)}]}"#;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("insert.chart", "Chart", "Insert › Illustrations", insert).params(
            r#"{"type"?: "column|columnStacked|columnPercent|bar|barStacked|barPercent|line|lineStacked|linePercent|area|areaStacked|areaPercent|pie|doughnut|pieOfPie|barOfPie|scatter|scatterLines|scatterSmooth|bubble|radar|radarMarkers|radarFilled|waterfall|funnel|combo", "data"?: {"categories": [string], "series": [{"name": string, "values": [number|null]}]}, "title"?: string, "legend"?: "right|left|top|bottom|none", "legendShape"?: "square|circle", "dataLabels"?: bool, "colors"?: ["rrggbb"], "smooth"?: bool, "markers"?: bool, "roundedBars"?: bool, "seriesGap"?: 0-100, "gridlines"?: "solid|dashed|none", "axisMin"?: number|null, "axisMax"?: number|null, "axisStep"?: number|null, "waterfallTotal"?: bool, "splitPos"?: n, "radarGradient"?: bool, "width"?: pt, "height"?: pt} (no params: the chart type picker)"#,
        ),
        CommandSpec::new("chart.colors", "Change Colors", "Chart Design › Chart Styles", |s, v| {
            let colors = match v.get("colors") {
                Some(c) => colors(c)?,
                None => return Err(CmdError::Params("`colors` is required: [\"rrggbb\"], or null for the theme's colours".into())),
            };
            edit(s, move |c| {
                c.colors = colors.clone();
                Ok(())
            })
        })
        .params(r#"{"colors": ["rrggbb"]|null} (null or []: the theme's accent colours)"#)
        .when(has_chart),
        CommandSpec::new("chart.preview", "Preview Colors", "Chart Design › Chart Styles", preview)
            .params(
                r#"{"colors": ["rrggbb"]|null} (draw the selected chart in these colours for now, not an edit: no undo step, no unsaved change; null ends the preview, and so does any other command or selecting something else)"#,
            )
            .pure()
            .when(has_chart),
        CommandSpec::new("chart.line", "Line Options", "Chart Design › Chart Styles", |s, v| {
            let smooth = opt_bool(v, "smooth")?;
            let markers = opt_bool(v, "markers")?;
            edit(s, move |c| {
                c.smooth = smooth.unwrap_or(c.smooth);
                c.markers = markers.unwrap_or(c.markers);
                Ok(())
            })
        })
        .params(r#"{"smooth"?: bool, "markers"?: bool} (line charts; a field left out stays as it is)"#)
        .when(has_chart),
        CommandSpec::new("chart.bars", "Bar Options", "Chart Design › Chart Styles", |s, v| {
            let rounded = opt_bool(v, "rounded")?;
            let gap = gap_param(v, "gap")?;
            if rounded.is_none() && gap.is_none() {
                return Err(CmdError::Params("`rounded` (true or false) or `gap` (0-100) is required".into()));
            }
            edit(s, move |c| {
                c.rounded_bars = rounded.unwrap_or(c.rounded_bars);
                c.series_gap = gap.unwrap_or(c.series_gap);
                Ok(())
            })
        })
        .params(
            r#"{"rounded"?: bool, "gap"?: 0-100} (column and bar charts: rounded value ends; gap between the series of a category, percent of a bar's thickness; at least one)"#,
        )
        .when(has_chart),
        CommandSpec::new("chart.gridlines", "Gridlines", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let style = match p::str(v, "style") {
                Some(id) => Gridlines::from_id(id).ok_or_else(|| CmdError::Params(format!("`style`: {GRIDLINES}")))?,
                None => return Err(CmdError::Params(format!("`style` is required: {GRIDLINES}"))),
            };
            edit(s, move |c| {
                c.gridlines = style;
                Ok(())
            })
        })
        .params(r#"{"style": "solid|dashed|none"}"#)
        .when(has_chart),
        CommandSpec::new("chart.axis", "Axis Options", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let min = axis_param(v, "min")?;
            let max = axis_param(v, "max")?;
            let step = axis_param(v, "step")?;
            edit(s, move |c| {
                let lo = min.unwrap_or(c.axis_min);
                let hi = max.unwrap_or(c.axis_max);
                let st = step.unwrap_or(c.axis_step);
                check_axis(lo, hi, st)?;
                (c.axis_min, c.axis_max, c.axis_step) = (lo, hi, st);
                Ok(())
            })
        })
        .params(r#"{"min"?: number|null, "max"?: number|null, "step"?: number|null} (value axis; null: automatic; a field left out stays as it is)"#)
        .when(has_chart),
        CommandSpec::new("chart.type", "Change Chart Type", "Chart Design › Type", |s, v| {
            let kind = chart_type(v)?.ok_or_else(|| CmdError::Params(format!("`type` is required: {TYPES}")))?;
            edit(s, |c| {
                c.kind = kind;
                Ok(())
            })
        })
        .params(r#"{"type": "column|columnStacked|columnPercent|bar|barStacked|barPercent|line|lineStacked|linePercent|area|areaStacked|areaPercent|pie|doughnut|pieOfPie|barOfPie|scatter|scatterLines|scatterSmooth|bubble|radar|radarMarkers|radarFilled|waterfall|funnel|combo"}"#)
        .when(has_chart),
        CommandSpec::new("chart.editData", "Edit Data", "Chart Design › Data", |s, v| {
            let d = v.get("data").unwrap_or(v);
            let (cats, series) = (d.get("categories"), d.get("series"));
            if cats.is_none() && series.is_none() {
                return Err(CmdError::Params(format!("`data` is required: {DATA}")));
            }
            let cats = cats.map(categories).transpose()?;
            let series = series.map(series_list).transpose()?;
            edit(s, move |c| {
                if let Some(cats) = &cats {
                    c.categories = cats.clone();
                }
                if let Some(series) = &series {
                    c.series = series.clone();
                }
                Ok(())
            })
        })
        .params(r#"{"data": {"categories"?: [string], "series"?: [{"name": string, "values": [number|null]}]}}"#)
        .when(has_chart),
        CommandSpec::new("chart.title", "Chart Title", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let text = match v.get("text") {
                Some(Value::String(t)) => Some(Some(t.clone())),
                Some(Value::Null) => Some(None),
                None => None,
                Some(_) => return Err(CmdError::Params("`text`: a string, or null for no title".into())),
            };
            edit(s, move |c| {
                c.title = match &text {
                    Some(t) => t.clone(),
                    // No text given: switch the title on (placeholder) or off.
                    None if c.title.is_some() && c.title_visible => None,
                    // A hidden title is shown again with its text.
                    None if c.title.is_some() => c.title.clone(),
                    None => Some("Chart Title".into()),
                };
                // Setting or switching on the title shows it.
                if c.title.is_some() {
                    c.title_visible = true;
                }
                Ok(())
            })
        })
        .params(r#"{"text"?: string|null} (no text: switch the title on or off)"#)
        .when(has_chart),
        CommandSpec::new("chart.legend", "Legend", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let pos = match p::str(v, "pos").or_else(|| p::str(v, "position")) {
                Some(id) if id.eq_ignore_ascii_case("none") => Some(None),
                Some(id) => Some(Some(LegendPos::from_id(id).ok_or_else(|| CmdError::Params("`pos`: right|left|top|bottom|none".into()))?)),
                None => None,
            };
            edit(s, move |c| {
                c.legend = match pos {
                    Some(p) => p,
                    None if c.legend.is_some() => None,
                    None => Some(LegendPos::Bottom),
                };
                Ok(())
            })
        })
        .params(r#"{"pos"?: "right|left|top|bottom|none"} (no pos: switch the legend on or off)"#)
        .when(has_chart),
        CommandSpec::new("chart.elements", "Chart Elements", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            const KEYS: [&str; 6] = ["titleVisible", "background", "catLabels", "catAxis", "valLabels", "valAxis"];
            let mut set = [None; 6];
            for (slot, k) in set.iter_mut().zip(KEYS) {
                *slot = opt_bool(v, k)?;
            }
            if set.iter().all(Option::is_none) {
                return Err(CmdError::Params(format!("give at least one of {} (true or false)", KEYS.join(", "))));
            }
            edit(s, move |c| {
                let [title, background, cat_labels, cat_axis, val_labels, val_axis] = set;
                c.title_visible = title.unwrap_or(c.title_visible);
                c.background = background.unwrap_or(c.background);
                c.cat_labels = cat_labels.unwrap_or(c.cat_labels);
                c.cat_axis = cat_axis.unwrap_or(c.cat_axis);
                c.val_labels = val_labels.unwrap_or(c.val_labels);
                c.val_axis = val_axis.unwrap_or(c.val_axis);
                Ok(())
            })
        })
        .params(
            r#"{"titleVisible"?: bool, "background"?: bool, "catLabels"?: bool, "catAxis"?: bool, "valLabels"?: bool, "valAxis"?: bool} (at least one; the others stay. Scatter charts: cat* is the x axis, val* the y axis; pie and doughnut charts have no axes)"#,
        )
        .when(has_chart),
        CommandSpec::new("chart.legendShape", "Legend Key Shape", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let shape = match p::str(v, "shape") {
                Some(id) => LegendShape::from_id(id).ok_or_else(|| CmdError::Params(format!("`shape`: {LEGEND_SHAPES}")))?,
                None => return Err(CmdError::Params(format!("`shape` is required: {LEGEND_SHAPES}"))),
            };
            edit(s, move |c| {
                c.legend_shape = shape;
                Ok(())
            })
        })
        .params(r#"{"shape": "square|circle"} (the legend keys' shape; WordCraft's drawing only, Word keeps its own keys)"#)
        .when(has_chart),
        CommandSpec::new("chart.style", "Format Chart", "Chart Format › Current Selection", |s, v| {
            let st = StyleEdit::parse(v)?;
            if st.is_empty() {
                return Err(CmdError::Params(format!("give at least one of {}", STYLE_KEYS.join(", "))));
            }
            edit(s, move |c| {
                st.apply(c);
                Ok(())
            })
        })
        .params(STYLE_PARAMS)
        .when(has_chart),
        CommandSpec::new("chart.reset", "Reset to Match Style", "Chart Format › Current Selection", |s, v| {
            let look = Look::parse(v)?;
            let st = StyleEdit::parse(v)?;
            edit(s, move |c| {
                let mut next = ChartSpec {
                    kind: c.kind,
                    categories: std::mem::take(&mut c.categories),
                    series: std::mem::take(&mut c.series),
                    title: c.title.take(),
                    title_visible: c.title_visible,
                    ..ChartSpec::sample(c.kind)
                };
                look.apply(&mut next)?;
                st.apply(&mut next);
                *c = next;
                Ok(())
            })
        })
        .params(
            r#"{...insert.chart's look params ("legend", "legendShape", "dataLabels", "colors", "smooth", "markers", "roundedBars", "seriesGap", "gridlines", "axisMin", "axisMax", "axisStep"), ...chart.style's params} (every look setting back to a new chart's, then these as insert.chart and chart.style set them; the type, data and title stay)"#,
        )
        .when(has_chart),
        CommandSpec::new("chart.dataLabels", "Data Labels", "Chart Design › Chart Layouts › Add Chart Element", |s, v| {
            let on = p::bool(v, "on").or_else(|| p::bool(v, "value"));
            edit(s, move |c| {
                c.data_labels = on.unwrap_or(!c.data_labels);
                Ok(())
            })
        })
        .params(r#"{"on"?: bool} (no value: switch the data labels on or off)"#)
        .when(has_chart),
    ]
}

/// `chart.preview`: the selected chart drawn in `colors` until the preview ends (see
/// [`Session::set_chart_preview`]); `null` or `[]` ends it.
fn preview(s: &mut Session, v: &Value) -> CmdResult {
    let colors = match v.get("colors") {
        Some(c) => colors(c)?,
        None => return Err(CmdError::Params("`colors` is required: [\"rrggbb\"], or null to end the preview".into())),
    };
    if colors.is_empty() {
        s.end_chart_preview();
        return Ok(json!({"previewing": false}));
    }
    let Some((pos, InlineObject::Graphic { w, h, graphic, .. })) = selected(s) else { return Err(CmdError::Disabled("select a chart first".into())) };
    let mut spec = graphic.chart.as_deref().cloned().ok_or_else(|| CmdError::Disabled("this chart can't be edited".into()))?;
    spec.colors = colors;
    let shown = chart_graphic(spec, &s.doc.settings.theme_colors, w, h);
    s.set_chart_preview(pos, Arc::new(shown));
    Ok(json!({"previewing": true}))
}

/// Whether the selected chart is being shown in previewed colours (`chart.preview`): the
/// selected-chart state's `previewing`.
pub fn chart_previewing(s: &Session) -> bool {
    s.chart_previewing() && selected_chart(s).is_some()
}

/// The selected chart's model, when it has one.
pub fn selected_chart(s: &Session) -> Option<Arc<ChartSpec>> {
    match selected(s) {
        Some((_, InlineObject::Graphic { graphic, .. })) => graphic.chart.clone(),
        _ => None,
    }
}

/// The plot area of the selected chart (one with a model), `[x, y, w, h]` in points from the
/// chart's top-left: the rectangle between the axes, or a pie's bounding square. Laid out as the
/// chart is drawn, at the graphic's size in the document's theme colours.
pub fn selected_chart_plot(s: &Session) -> Option<[f32; 4]> {
    match selected(s) {
        Some((_, InlineObject::Graphic { graphic, .. })) => {
            let spec = graphic.chart.as_deref()?;
            wordcraft_docx::chart_plot_rect(spec, &s.doc.settings.theme_colors, graphic.w, graphic.h)
        }
        _ => None,
    }
}

/// Where the selected chart (any chart, one from another program too) sits:
/// `{"floating": false, "locked", "width", "height"}` when it is In Line with Text, else
/// `{"floating": true, "locked", "page", "x", "y", "width", "height"}` with its top-left in points from
/// the top-left of page `page` (0-based), as `object.position` takes it.
pub fn selected_chart_frame(s: &mut Session) -> Option<Value> {
    let (pos, obj) = selected(s)?;
    let InlineObject::Graphic { w, h, graphic, float, .. } = &obj else { return None };
    if graphic.kind != GraphicKind::Chart {
        return None;
    }
    let locked = float.locked;
    if !obj.is_floating() {
        return Some(json!({"floating": false, "locked": locked, "width": w, "height": h}));
    }
    let hint = s.page_hint;
    let hit = s.layout().object(&pos, hint).map(|h| (h.page, h.rect.x, h.rect.y));
    Some(match hit {
        Some((page, x, y)) => json!({"floating": true, "locked": locked, "page": page, "x": x, "y": y, "width": w, "height": h}),
        None => json!({"floating": true, "locked": locked, "width": w, "height": h}),
    })
}

fn has_chart(s: &Session) -> Option<&'static str> {
    match selected(s) {
        Some((_, InlineObject::Graphic { graphic, .. })) if graphic.chart.is_some() => None,
        Some((_, InlineObject::Graphic { graphic, .. })) if graphic.kind == GraphicKind::Chart => {
            Some("this chart was made in another program: WordCraft shows it but can't edit it")
        }
        _ => Some("select a chart first"),
    }
}

/// The graphic for `spec` drawn `w` × `h` points in the theme's colours.
pub fn chart_graphic(spec: ChartSpec, theme: &[Rgb], w: f32, h: f32) -> Graphic {
    let spec = spec.sanitized();
    let items = wordcraft_docx::chart_items(&spec, theme, w, h);
    Graphic { kind: GraphicKind::Chart, items, w, h, source: None, chart: Some(Arc::new(spec)), smart_art: None }
}

/// `type` of the params, if given.
fn chart_type(v: &Value) -> Result<Option<ChartType>, CmdError> {
    match p::str(v, "type") {
        Some(id) => ChartType::from_id(id).map(Some).ok_or_else(|| CmdError::Params(format!("unknown chart type `{id}`: {TYPES}"))),
        None if v.get("type").is_some_and(|t| !t.is_null()) => Err(CmdError::Params(format!("`type`: {TYPES}"))),
        None => Ok(None),
    }
}

fn insert(s: &mut Session, v: &Value) -> CmdResult {
    let kind = chart_type(v)?;
    // No type and no data: Insert › Chart's picker, as the ribbon button does.
    let styled = ["colors", "legendShape", "smooth", "markers", "roundedBars", "seriesGap", "gridlines", "axisMin", "axisMax", "axisStep"]
        .iter()
        .any(|k| v.get(*k).is_some_and(|x| !x.is_null()));
    if kind.is_none() && v.get("data").is_none() && !styled {
        s.push_ui_request(json!({"open": "insertChart"}));
        return sel_result(s);
    }
    let kind = kind.unwrap_or_default();
    let mut spec = ChartSpec::sample(kind);
    if let Some(d) = v.get("data") {
        spec.categories = d.get("categories").map(categories).transpose()?.unwrap_or_default();
        spec.series = d.get("series").map(series_list).transpose()?.unwrap_or_default();
        if spec.series.is_empty() {
            return Err(CmdError::Params(format!("`data.series` needs at least one series: {DATA}")));
        }
    }
    spec.title = p::str(v, "title").map(str::to_string);
    Look::parse(v)?.apply(&mut spec)?;
    // Fitted to the text width, as Word sizes a new chart.
    let max_w = s.doc.sections().first().map(|(_, sp)| sp.text_width()).unwrap_or(468.0).max(36.0);
    let w = p::f32(v, "width").unwrap_or(DEFAULT_W).clamp(36.0, 2000.0).min(max_w);
    let h = p::f32(v, "height").unwrap_or(DEFAULT_H).clamp(36.0, 2000.0);
    let graphic = chart_graphic(spec, &s.doc.settings.theme_colors, w, h);
    let out = json!({"chart": graphic.chart.as_deref(), "width": w, "height": h});
    let props = s.typing_props();
    let at = delete_selection(s)?;
    let obj = InlineObject::Graphic { w, h, alt: String::new(), float: Float::default(), graphic: Arc::new(graphic) };
    let end = s.doc.insert_object(&at, obj, &props)?;
    s.sel = Selection { anchor: at, focus: end };
    Ok(out)
}

/// The look params `insert.chart` takes (and `chart.reset`), as given: each `None` when left out.
struct Look {
    legend: Option<Option<LegendPos>>,
    data_labels: Option<bool>,
    legend_shape: Option<LegendShape>,
    colors: Option<Vec<String>>,
    smooth: Option<bool>,
    markers: Option<bool>,
    rounded_bars: Option<bool>,
    series_gap: Option<f64>,
    gridlines: Option<Gridlines>,
    axis: (Option<f64>, Option<f64>, Option<f64>),
    waterfall_total: Option<bool>,
    split_pos: Option<u32>,
    radar_gradient: Option<bool>,
}

impl Look {
    fn parse(v: &Value) -> Result<Look, CmdError> {
        let legend = match p::str(v, "legend") {
            Some(id) if id.eq_ignore_ascii_case("none") => Some(None),
            Some(id) => Some(Some(LegendPos::from_id(id).ok_or_else(|| CmdError::Params("`legend`: right|left|top|bottom|none".into()))?)),
            None => None,
        };
        let legend_shape = match v.get("legendShape").filter(|x| !x.is_null()) {
            Some(shape) => {
                Some(shape.as_str().and_then(LegendShape::from_id).ok_or_else(|| CmdError::Params(format!("`legendShape`: {LEGEND_SHAPES}")))?)
            }
            None => None,
        };
        let gridlines = match v.get("gridlines") {
            None | Some(Value::Null) => None,
            Some(g) => Some(g.as_str().and_then(Gridlines::from_id).ok_or_else(|| CmdError::Params(format!("`gridlines`: {GRIDLINES}")))?),
        };
        let axis = (axis_param(v, "axisMin")?.flatten(), axis_param(v, "axisMax")?.flatten(), axis_param(v, "axisStep")?.flatten());
        check_axis(axis.0, axis.1, axis.2)?;
        Ok(Look {
            legend,
            data_labels: p::bool(v, "dataLabels"),
            legend_shape,
            colors: v.get("colors").map(colors).transpose()?,
            smooth: opt_bool(v, "smooth")?,
            markers: opt_bool(v, "markers")?,
            rounded_bars: opt_bool(v, "roundedBars")?,
            series_gap: gap_param(v, "seriesGap")?,
            gridlines,
            axis,
            waterfall_total: opt_bool(v, "waterfallTotal")?,
            split_pos: split_param(v)?,
            radar_gradient: opt_bool(v, "radarGradient")?,
        })
    }

    /// Set what was given on `spec` (a new chart's look: what's left out stays as it is).
    fn apply(&self, spec: &mut ChartSpec) -> Result<(), CmdError> {
        if let Some(l) = self.legend {
            spec.legend = l;
        }
        spec.data_labels = self.data_labels.unwrap_or(spec.data_labels);
        spec.legend_shape = self.legend_shape.unwrap_or(spec.legend_shape);
        if let Some(c) = &self.colors {
            spec.colors.clone_from(c);
        }
        spec.smooth = self.smooth.unwrap_or(spec.smooth);
        spec.markers = self.markers.unwrap_or(spec.markers);
        spec.rounded_bars = self.rounded_bars.unwrap_or(spec.rounded_bars);
        spec.series_gap = self.series_gap.unwrap_or(spec.series_gap);
        spec.gridlines = self.gridlines.unwrap_or(spec.gridlines);
        (spec.axis_min, spec.axis_max, spec.axis_step) = self.axis;
        spec.waterfall_total = self.waterfall_total.unwrap_or(spec.waterfall_total);
        spec.split_pos = self.split_pos.unwrap_or(spec.split_pos);
        spec.radar_gradient = self.radar_gradient.unwrap_or(spec.radar_gradient);
        Ok(())
    }
}

/// The params of `chart.style`.
const STYLE_KEYS: [&str; 24] = [
    "legendStyle",
    "dataLabelsStyle",
    "catLabelsStyle",
    "valLabelsStyle",
    "titleStyle",
    "dataLabelsOutline",
    "catLabelPos",
    "catAxisColor",
    "valAxisColor",
    "catTicks",
    "valTicks",
    "background",
    "backgroundColor",
    "lineWidth",
    "areaOpacity",
    "opacity",
    "bubbleOpacity",
    "holeSize",
    "sliceGap",
    "smooth",
    "markers",
    "waterfallTotal",
    "splitPos",
    "radarGradient",
];
const STYLE_PARAMS: &str = r#"{"legendStyle"?: Text, "dataLabelsStyle"?: Text, "catLabelsStyle"?: Text, "valLabelsStyle"?: Text, "titleStyle"?: Text, "dataLabelsOutline"?: bool, "catLabelPos"?: "auto|rotate", "catAxisColor"?: "rrggbb"|null, "valAxisColor"?: "rrggbb"|null, "catTicks"?: bool, "valTicks"?: bool, "background"?: bool, "backgroundColor"?: "rrggbb"|null, "lineWidth"?: 0.75-15 pt|null, "areaOpacity"?: 0-1, "opacity"?: 0-1, "bubbleOpacity"?: 0-1, "holeSize"?: 10-90 (percent), "sliceGap"?: 0-10 pt, "smooth"?: bool, "markers"?: bool, "waterfallTotal"?: bool, "splitPos"?: n, "radarGradient"?: bool} where Text = {"font"?: string|null, "size"?: 4-96 pt|null, "color"?: "rrggbb"|null, "bold"?: bool|null}|null (at least one; a field left out stays; a Text's fields merge into the element's, null clears one; a Text of null clears them all. Scatter charts: cat* is the x axis. One undo step)"#;

/// A text style change: each field `None` when left out, `Some(None)` for null (back to the
/// chart's own).
#[derive(Clone, Default)]
struct TextEdit {
    reset: bool,
    font: Option<Option<String>>,
    size: Option<Option<f32>>,
    color: Option<Option<String>>,
    bold: Option<Option<bool>>,
}

impl TextEdit {
    fn parse(v: &Value, k: &str) -> Result<Option<TextEdit>, CmdError> {
        let bad = |what: &str| CmdError::Params(format!("`{k}.{what}`"));
        let o = match v.get(k) {
            None => return Ok(None),
            Some(Value::Null) => return Ok(Some(TextEdit { reset: true, ..Default::default() })),
            Some(Value::Object(o)) => o,
            Some(_) => return Err(CmdError::Params(format!("`{k}`: {{\"font\"?, \"size\"?, \"color\"?, \"bold\"?}} or null"))),
        };
        let font = match o.get("font") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(Value::String(f)) => Some(Some(f.chars().take(wordcraft_doc::chart::MAX_FONT_NAME * 4).collect())),
            Some(_) => return Err(bad("font: a font name, or null")),
        };
        let size = match o.get("size") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(x) => Some(Some(x.as_f64().filter(|n| n.is_finite()).ok_or_else(|| bad("size: points from 4 to 96, or null"))? as f32)),
        };
        let color = match o.get("color") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(x) => Some(Some(x.as_str().and_then(hex_color).ok_or_else(|| bad("color: \"rrggbb\", or null"))?)),
        };
        let bold = match o.get("bold") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(Value::Bool(b)) => Some(Some(*b)),
            Some(_) => return Err(bad("bold: true, false or null")),
        };
        Ok(Some(TextEdit { reset: false, font, size, color, bold }))
    }

    fn apply(&self, s: &mut TextStyle) {
        if self.reset {
            *s = TextStyle::default();
        }
        if let Some(f) = &self.font {
            s.font.clone_from(f);
        }
        if let Some(z) = self.size {
            s.size = z;
        }
        if let Some(c) = &self.color {
            s.color.clone_from(c);
        }
        if let Some(b) = self.bold {
            s.bold = b;
        }
    }
}

/// The params of `chart.style`, as given.
#[derive(Clone, Default)]
struct StyleEdit {
    legend: Option<TextEdit>,
    data_labels: Option<TextEdit>,
    cat_labels: Option<TextEdit>,
    val_labels: Option<TextEdit>,
    title: Option<TextEdit>,
    outline: Option<bool>,
    cat_label_pos: Option<CatLabelPos>,
    cat_axis_color: Option<Option<String>>,
    val_axis_color: Option<Option<String>>,
    cat_ticks: Option<bool>,
    val_ticks: Option<bool>,
    background: Option<bool>,
    background_color: Option<Option<String>>,
    line_width: Option<Option<f32>>,
    area_opacity: Option<f32>,
    opacity: Option<f32>,
    bubble_opacity: Option<f32>,
    hole_size: Option<u32>,
    slice_gap: Option<f32>,
    smooth: Option<bool>,
    markers: Option<bool>,
    waterfall_total: Option<bool>,
    split_pos: Option<u32>,
    radar_gradient: Option<bool>,
}

impl StyleEdit {
    fn parse(v: &Value) -> Result<StyleEdit, CmdError> {
        let color = |k: &str| -> Result<Option<Option<String>>, CmdError> {
            match v.get(k) {
                None => Ok(None),
                Some(Value::Null) => Ok(Some(None)),
                Some(x) => {
                    x.as_str().and_then(hex_color).map(|c| Some(Some(c))).ok_or_else(|| CmdError::Params(format!("`{k}`: \"rrggbb\", or null")))
                }
            }
        };
        let share = |k: &str| -> Result<Option<f32>, CmdError> {
            match v.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(x) => x
                    .as_f64()
                    .filter(|n| n.is_finite())
                    .map(|n| Some(n as f32))
                    .ok_or_else(|| CmdError::Params(format!("`{k}`: a number from 0 to 1"))),
            }
        };
        let cat_label_pos = match v.get("catLabelPos") {
            None | Some(Value::Null) => None,
            Some(x) => Some(x.as_str().and_then(CatLabelPos::from_id).ok_or_else(|| CmdError::Params("`catLabelPos`: auto|rotate".into()))?),
        };
        let line_width = match v.get("lineWidth") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(x) => Some(Some(
                x.as_f64().filter(|n| n.is_finite()).ok_or_else(|| CmdError::Params("`lineWidth`: points from 0.75 to 15, or null".into()))? as f32,
            )),
        };
        Ok(StyleEdit {
            legend: TextEdit::parse(v, "legendStyle")?,
            data_labels: TextEdit::parse(v, "dataLabelsStyle")?,
            cat_labels: TextEdit::parse(v, "catLabelsStyle")?,
            val_labels: TextEdit::parse(v, "valLabelsStyle")?,
            title: TextEdit::parse(v, "titleStyle")?,
            outline: opt_bool(v, "dataLabelsOutline")?,
            cat_label_pos,
            cat_axis_color: color("catAxisColor")?,
            val_axis_color: color("valAxisColor")?,
            cat_ticks: opt_bool(v, "catTicks")?,
            val_ticks: opt_bool(v, "valTicks")?,
            background: opt_bool(v, "background")?,
            background_color: color("backgroundColor")?,
            line_width,
            area_opacity: share("areaOpacity")?,
            opacity: share("opacity")?,
            bubble_opacity: share("bubbleOpacity")?,
            hole_size: number_param(v, "holeSize", "the doughnut hole, 10 to 90 percent")?.map(|n| n.clamp(10.0, 90.0).round() as u32),
            slice_gap: number_param(v, "sliceGap", "the gap between slices, 0 to 10 points")?.map(|n| n.clamp(0.0, 10.0) as f32),
            smooth: opt_bool(v, "smooth")?,
            markers: opt_bool(v, "markers")?,
            waterfall_total: opt_bool(v, "waterfallTotal")?,
            split_pos: split_param(v)?,
            radar_gradient: opt_bool(v, "radarGradient")?,
        })
    }

    fn is_empty(&self) -> bool {
        [&self.legend, &self.data_labels, &self.cat_labels, &self.val_labels, &self.title].iter().all(|t| t.is_none())
            && self.outline.is_none()
            && self.cat_label_pos.is_none()
            && self.cat_axis_color.is_none()
            && self.val_axis_color.is_none()
            && self.cat_ticks.is_none()
            && self.val_ticks.is_none()
            && self.background.is_none()
            && self.background_color.is_none()
            && self.line_width.is_none()
            && self.area_opacity.is_none()
            && self.opacity.is_none()
            && self.bubble_opacity.is_none()
            && self.hole_size.is_none()
            && self.slice_gap.is_none()
            && self.smooth.is_none()
            && self.markers.is_none()
            && self.waterfall_total.is_none()
            && self.split_pos.is_none()
            && self.radar_gradient.is_none()
    }

    fn apply(&self, c: &mut ChartSpec) {
        let texts = [
            (&self.legend, &mut c.legend_style),
            (&self.data_labels, &mut c.data_labels_style),
            (&self.cat_labels, &mut c.cat_labels_style),
            (&self.val_labels, &mut c.val_labels_style),
            (&self.title, &mut c.title_style),
        ];
        for (e, style) in texts {
            if let Some(e) = e {
                e.apply(style);
            }
        }
        c.data_labels_outline = self.outline.unwrap_or(c.data_labels_outline);
        c.cat_label_pos = self.cat_label_pos.unwrap_or(c.cat_label_pos);
        if let Some(x) = &self.cat_axis_color {
            c.cat_axis_color.clone_from(x);
        }
        if let Some(x) = &self.val_axis_color {
            c.val_axis_color.clone_from(x);
        }
        c.cat_ticks = self.cat_ticks.unwrap_or(c.cat_ticks);
        c.val_ticks = self.val_ticks.unwrap_or(c.val_ticks);
        c.background = self.background.unwrap_or(c.background);
        if let Some(x) = &self.background_color {
            c.background_color.clone_from(x);
        }
        if let Some(x) = self.line_width {
            c.line_width = x;
        }
        c.area_opacity = self.area_opacity.unwrap_or(c.area_opacity);
        c.opacity = self.opacity.unwrap_or(c.opacity);
        c.bubble_opacity = self.bubble_opacity.unwrap_or(c.bubble_opacity);
        c.hole_size = self.hole_size.unwrap_or(c.hole_size);
        c.slice_gap = self.slice_gap.unwrap_or(c.slice_gap);
        c.smooth = self.smooth.unwrap_or(c.smooth);
        c.markers = self.markers.unwrap_or(c.markers);
        c.waterfall_total = self.waterfall_total.unwrap_or(c.waterfall_total);
        c.split_pos = self.split_pos.unwrap_or(c.split_pos);
        c.radar_gradient = self.radar_gradient.unwrap_or(c.radar_gradient);
    }
}

/// Change the selected chart's model with `f`, then draw it again at its size.
fn edit(s: &mut Session, f: impl Fn(&mut ChartSpec) -> Result<(), CmdError>) -> CmdResult {
    let (pos, obj) = selected(s).ok_or_else(|| CmdError::Disabled("select a chart first".into()))?;
    let InlineObject::Graphic { w, h, graphic, .. } = obj else { return Err(CmdError::Disabled("select a chart first".into())) };
    let mut spec = graphic.chart.as_deref().cloned().ok_or_else(|| CmdError::Disabled("this chart can't be edited".into()))?;
    f(&mut spec)?;
    let next = Arc::new(chart_graphic(spec, &s.doc.settings.theme_colors, w, h));
    let out = json!({"chart": next.chart.as_deref()});
    let para = s.doc.para_mut(pos.story, &pos.path)?;
    if let Some(InlineObject::Graphic { graphic, .. }) = para.object_at_mut(pos.off) {
        *graphic = next;
    }
    para.touch();
    Ok(out)
}

/// Draw the chart of `o` again at its current size (after a resize), if it has a model.
pub fn redraw(o: &mut InlineObject, theme: &[Rgb]) {
    if let InlineObject::Graphic { w, h, graphic, .. } = o
        && let Some(spec) = graphic.chart.as_deref()
    {
        *graphic = Arc::new(chart_graphic(spec.clone(), theme, *w, *h));
    }
}

/// Category labels from JSON: strings, or numbers written as text.
fn categories(v: &Value) -> Result<Vec<String>, CmdError> {
    let a = v.as_array().ok_or_else(|| CmdError::Params("`categories`: an array of strings".into()))?;
    Ok(a.iter().take(MAX_POINTS).map(cell_text).collect())
}

fn cell_text(v: &Value) -> String {
    match v {
        Value::String(t) => wordcraft_doc::chart::clean_text(t),
        Value::Null => String::new(),
        other => wordcraft_doc::chart::clean_text(&other.to_string()),
    }
}

/// Series from JSON: `[{"name", "values": [number|null|numeric string]}]`.
fn series_list(v: &Value) -> Result<Vec<ChartSeries>, CmdError> {
    let a = v.as_array().ok_or_else(|| CmdError::Params(format!("`series`: {DATA}")))?;
    a.iter()
        .take(MAX_SERIES)
        .map(|s| {
            let name = s.get("name").map(cell_text).unwrap_or_default();
            let list = |k: &str| match s.get(k) {
                Some(Value::Array(vals)) => Ok(vals.iter().take(MAX_POINTS).map(number).collect()),
                None | Some(Value::Null) => Ok(Vec::new()),
                Some(_) => Err(CmdError::Params(format!("`{k}`: an array of numbers (null for none)"))),
            };
            let kind = match s.get("kind") {
                None | Some(Value::Null) => None,
                Some(k) => Some(k.as_str().and_then(SeriesKind::from_id).ok_or_else(|| CmdError::Params("`kind`: column|line|area".into()))?),
            };
            let secondary = opt_bool(s, "secondary")?.unwrap_or(false);
            Ok(ChartSeries { name, values: list("values")?, sizes: list("sizes")?, kind, secondary })
        })
        .collect()
}

/// `splitPos` (pie of pie: points in the second plot): a whole number, held to what the chart
/// has; `None` when left out.
/// An optional finite number param `key` (`what` explains it in the error).
fn number_param(v: &Value, key: &str, what: &str) -> Result<Option<f64>, CmdError> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => x.as_f64().filter(|n| n.is_finite()).map(Some).ok_or_else(|| CmdError::Params(format!("`{key}`: {what}"))),
    }
}

fn split_param(v: &Value) -> Result<Option<u32>, CmdError> {
    match v.get("splitPos") {
        None | Some(Value::Null) => Ok(None),
        Some(x) => match x.as_f64().filter(|n| n.is_finite()) {
            Some(n) => Ok(Some(n.clamp(1.0, f64::from(u32::MAX)) as u32)),
            None => Err(CmdError::Params("`splitPos`: how many of the last points go to the second plot".into())),
        },
    }
}

/// A palette from JSON: `null` or `[]` is none (the theme's accents); else up to
/// [`MAX_COLORS`] colours, each `rrggbb` (a leading `#` is allowed).
fn colors(v: &Value) -> Result<Vec<String>, CmdError> {
    const MSG: &str = "`colors`: an array of 6-digit hex colours (\"rrggbb\"), or null";
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Array(a) if a.len() > MAX_COLORS => Err(CmdError::Params(format!("`colors`: at most {MAX_COLORS} colours"))),
        Value::Array(a) => a.iter().map(|c| c.as_str().and_then(hex_color).ok_or_else(|| CmdError::Params(format!("{MSG}: got {c}")))).collect(),
        _ => Err(CmdError::Params(MSG.into())),
    }
}

/// A boolean param: `None` when left out (or null), an error when it isn't a boolean.
fn opt_bool(v: &Value, k: &str) -> Result<Option<bool>, CmdError> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(CmdError::Params(format!("`{k}`: true or false"))),
    }
}

/// An axis number param: `None` when left out, `Some(None)` for null (automatic), else a
/// finite number, clamped to ±[`wordcraft_doc::chart::MAX_AXIS`].
fn axis_param(v: &Value, k: &str) -> Result<Option<Option<f64>>, CmdError> {
    match v.get(k) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(x) => match x.as_f64().and_then(axis_number) {
            Some(n) => Ok(Some(Some(n))),
            None => Err(CmdError::Params(format!("`{k}`: a number, or null for automatic"))),
        },
    }
}

/// A series gap param: `None` when left out (or null), else a number clamped to 0–100 and
/// rounded to a whole percent; anything else is an error.
fn gap_param(v: &Value, k: &str) -> Result<Option<f64>, CmdError> {
    match v.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(x) => match x.as_f64().filter(|n| n.is_finite()) {
            Some(n) => Ok(Some(wordcraft_doc::chart::series_gap(n))),
            None => Err(CmdError::Params(format!("`{k}`: a number from 0 to 100"))),
        },
    }
}

/// Value axis bounds and step that make a scale: `min` below `max`, a positive step.
fn check_axis(min: Option<f64>, max: Option<f64>, step: Option<f64>) -> Result<(), CmdError> {
    if let (Some(lo), Some(hi)) = (min, max)
        && lo >= hi
    {
        return Err(CmdError::Params(format!("axis minimum ({lo}) must be below the maximum ({hi})")));
    }
    if step.is_some_and(|s| s <= 0.0) {
        return Err(CmdError::Params("axis `step` must be greater than 0".into()));
    }
    Ok(())
}

/// A value: a number, or text that reads as one; anything else (or not finite) is no value.
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(t) => t.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|x| x.is_finite())
}
