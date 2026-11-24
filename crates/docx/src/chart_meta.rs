//! Chart settings that only WordCraft draws (round legend keys, the palette as chosen, a hidden
//! title's text, rounded bars), kept in a document-level custom XML data part as well as in the
//! chart part's own extension.
//!
//! Another program may drop an unknown extension from a chart part when it saves it; a custom
//! XML data part (ECMA-376 Part 1 §22.5, `customXml/itemN.xml` with its properties part) is
//! document data a consumer keeps. Whether a given program really keeps it unchanged is for a
//! test with that program to show.
//!
//! One record per chart, keyed by the name of its drawing (`wp:docPr/@name`, which the writer
//! makes unique per document) with a signature of the chart (type, series, points) as the
//! fallback when the name changed. A record is applied only to the one chart it matches, only
//! when that chart's part lacks the extension, and only where the chart still agrees with it
//! (its colours are the record's palette in turn; it shows no title of its own). Records are
//! written again from the document on every save, so a deleted chart's record goes with it.

use std::sync::Arc;

use wordcraft_doc::Document;
use wordcraft_doc::chart::{ChartSpec, ChartType, LegendShape, MAX_COLORS, MAX_TEXT, clean_text, hex_color};
use wordcraft_doc::graphic::Graphic;
use wordcraft_doc::props::Rgb;

use crate::xml::{El, W};

/// The namespace of the records part.
const NS: &str = "urn:wordcraft:charts";
/// The root element's local name, which reading looks for.
const ROOT: &str = "wordcraftCharts";
/// The data store item id of the records part (its properties part).
const ITEM_ID: &str = "{6A1E3B2C-4D5F-4E7A-9B8C-0D1E2F3A4B5C}";
/// Most records read from one part.
const MAX_RECORDS: usize = 4096;

/// A chart's WordCraft-only settings.
#[derive(Clone, Debug, PartialEq, Default)]
pub(crate) struct Record {
    /// The chart drawing's name (`wp:docPr/@name`).
    pub(crate) name: String,
    /// Type, series count and point count, for a chart whose drawing was renamed.
    pub(crate) signature: String,
    pub(crate) circle_keys: bool,
    pub(crate) colors: Vec<String>,
    pub(crate) hidden_title: Option<String>,
    pub(crate) rounded_bars: bool,
    /// An area chart's smooth top edge and dots (the format has them for lines only).
    pub(crate) area_smooth: bool,
    pub(crate) area_markers: bool,
    /// The whole chart's opacity, below 1.
    pub(crate) opacity: Option<f32>,
    /// A waterfall or funnel (written as a stacked chart) or a line radar (written filled, for
    /// its gradient), and a waterfall's total bar.
    pub(crate) drawn: Option<ChartType>,
    pub(crate) total: bool,
}

/// The signature a record matches a chart by when its name doesn't.
pub(crate) fn signature(spec: &ChartSpec) -> String {
    format!("{}/{}/{}", spec.kind.id(), spec.series.len(), spec.points())
}

/// The record for the chart `spec` drawn as `name`, when it has a setting only WordCraft keeps.
pub(crate) fn record_for(name: &str, spec: &ChartSpec) -> Option<Record> {
    let r = Record {
        name: name.to_string(),
        // As the chart part has it (a waterfall or funnel as the stacked chart it's written as).
        signature: signature(&crate::chart_spec::written(spec).spec),
        circle_keys: spec.legend_shape == LegendShape::Circle,
        colors: spec.colors.clone(),
        hidden_title: spec.title.clone().filter(|_| !spec.title_visible),
        rounded_bars: spec.rounded_bars,
        area_smooth: spec.kind.is_area() && spec.smooth,
        area_markers: spec.kind.is_area() && spec.markers,
        opacity: (spec.opacity < 1.0).then_some(spec.opacity),
        drawn: (matches!(spec.kind, ChartType::Waterfall | ChartType::Funnel) || crate::chart_spec::radar_written_filled(spec)).then_some(spec.kind),
        total: spec.waterfall_total,
    };
    (r.circle_keys
        || !r.colors.is_empty()
        || r.hidden_title.is_some()
        || r.rounded_bars
        || r.area_smooth
        || r.area_markers
        || r.opacity.is_some()
        || r.drawn.is_some())
    .then_some(r)
}

/// The records part.
pub(crate) fn part_xml(records: &[Record]) -> Vec<u8> {
    let mut w = W::new();
    w.open(&format!("wc:{ROOT}"), &[("xmlns:wc", NS)]);
    for r in records {
        let mut attrs = vec![("name", r.name.as_str()), ("signature", r.signature.as_str())];
        if r.circle_keys {
            attrs.push(("legendShape", "circle"));
        }
        if r.rounded_bars {
            attrs.push(("roundedBars", "1"));
        }
        if r.area_smooth {
            attrs.push(("smooth", "1"));
        }
        if r.area_markers {
            attrs.push(("markers", "1"));
        }
        let opacity = r.opacity.map(|o| o.to_string());
        if let Some(o) = &opacity {
            attrs.push(("opacity", o));
        }
        if let Some(k) = r.drawn {
            attrs.push(("kind", k.id()));
        }
        if r.total {
            attrs.push(("total", "1"));
        }
        w.open("wc:chart", &attrs);
        for c in &r.colors {
            w.empty("wc:color", &[("val", c)]);
        }
        if let Some(t) = &r.hidden_title {
            w.leaf("wc:hiddenTitle", &[], t);
        }
        w.close("wc:chart");
    }
    w.close(&format!("wc:{ROOT}"));
    w.into_bytes()
}

/// The records part's properties part.
pub(crate) fn props_xml() -> Vec<u8> {
    let mut w = W::new();
    w.open("ds:datastoreItem", &[("xmlns:ds", "http://schemas.openxmlformats.org/officeDocument/2006/customXml"), ("ds:itemID", ITEM_ID)]);
    w.open("ds:schemaRefs", &[]);
    w.empty("ds:schemaRef", &[("ds:uri", NS)]);
    w.close("ds:schemaRefs");
    w.close("ds:datastoreItem");
    w.into_bytes()
}

/// Whether a custom XML part's root is the records part (its elements are in a namespace the
/// reader doesn't know: `?:`).
pub(crate) fn is_records(root: &El) -> bool {
    root.name == format!("?:{ROOT}")
}

/// The records a records part holds, cleaned (text and colours as the chart model keeps them).
pub(crate) fn read_part(root: &El) -> Vec<Record> {
    if !is_records(root) {
        return Vec::new();
    }
    root.children("?:chart")
        .take(MAX_RECORDS)
        .filter_map(|c| {
            let name = clean_text(c.attr("name")?);
            (!name.is_empty()).then(|| Record {
                name,
                signature: c.attr("signature").map(clean_text).unwrap_or_default(),
                circle_keys: c.attr("legendShape") == Some("circle"),
                colors: c.children("?:color").take(MAX_COLORS).filter_map(|e| e.attr("val").and_then(hex_color)).collect(),
                hidden_title: c.child("?:hiddenTitle").map(|t| clean_text(&t.text())).filter(|t| !t.is_empty() && t.chars().count() <= MAX_TEXT),
                rounded_bars: c.attr("roundedBars") == Some("1"),
                area_smooth: c.attr("smooth") == Some("1"),
                area_markers: c.attr("markers") == Some("1"),
                opacity: c
                    .attr("opacity")
                    .and_then(|o| o.parse::<f32>().ok())
                    .filter(|o| o.is_finite())
                    .map(|o| wordcraft_doc::chart::opacity_of(o, 1.0))
                    .filter(|o| *o < 1.0),
                drawn: c
                    .attr("kind")
                    .and_then(ChartType::from_id)
                    .filter(|k| matches!(k, ChartType::Waterfall | ChartType::Funnel | ChartType::Radar | ChartType::RadarMarkers)),
                total: c.attr("total") == Some("1"),
            })
        })
        .collect()
}

/// A chart read from the file: its drawing's name, its graphic and whether its part had our
/// extension (which then wins over any record).
pub(crate) type Seen = (String, Arc<Graphic>, bool);

/// Apply `records` to the charts `seen` in `doc`: each record to the one chart whose drawing has
/// its name, else to the one chart with its signature; never to a chart whose part has our
/// extension, nor to two charts. The chart is drawn again with its settings in `theme`.
pub(crate) fn apply(doc: &mut Document, seen: &[Seen], records: &[Record], theme: &[Rgb]) {
    let mut changed: Vec<(Arc<Graphic>, Arc<Graphic>)> = Vec::new();
    let mut used = std::collections::HashSet::new();
    for r in records {
        let by_name: Vec<&Seen> = seen.iter().filter(|s| s.0 == r.name).collect();
        let by_sig: Vec<&Seen> = seen.iter().filter(|s| s.1.chart.as_deref().is_some_and(|c| signature(c) == r.signature)).collect();
        let target = match (by_name.as_slice(), by_sig.as_slice()) {
            ([one], _) => *one,
            ([], [one]) if !r.signature.is_empty() => *one,
            _ => continue,
        };
        let (_, graphic, had_ext) = target;
        if *had_ext || !used.insert(Arc::as_ptr(graphic)) {
            continue;
        }
        let Some(spec) = graphic.chart.as_deref() else { continue };
        let next = settled(spec, r);
        if next != *spec {
            let items = crate::chart_items(&next, theme, graphic.w, graphic.h);
            let g = Graphic { items, chart: Some(Arc::new(next)), ..(**graphic).clone() };
            changed.push((graphic.clone(), Arc::new(g)));
        }
    }
    if changed.is_empty() {
        return;
    }
    let wants = |o: &wordcraft_doc::InlineObject| matches!(o, wordcraft_doc::InlineObject::Graphic { graphic, .. } if changed.iter().any(|(a, _)| Arc::ptr_eq(a, graphic)));
    doc.objects_mut(&wants, &mut |o| {
        if let wordcraft_doc::InlineObject::Graphic { graphic, .. } = o
            && let Some((_, b)) = changed.iter().find(|(a, _)| Arc::ptr_eq(a, graphic))
        {
            *graphic = b.clone();
        }
    });
}

/// `spec` with what record `r` keeps, where the chart still agrees with it.
fn settled(spec: &ChartSpec, r: &Record) -> ChartSpec {
    let mut s = spec.clone();
    if r.circle_keys {
        s.legend_shape = LegendShape::Circle;
    }
    if r.rounded_bars {
        s.rounded_bars = true;
    }
    if s.kind.is_area() {
        s.smooth |= r.area_smooth;
        s.markers |= r.area_markers;
    }
    if let Some(o) = r.opacity
        && s.opacity >= 1.0
    {
        s.opacity = o;
    }
    // The palette as chosen, when the chart's colours are still that palette in turn (the
    // series or slices show only as many colours as they need).
    if !r.colors.is_empty() {
        let n = if s.kind.is_round() { s.points() } else { s.series.len() };
        let cycled: Vec<&String> = (0..n).filter_map(|i| r.colors.get(i % r.colors.len())).collect();
        if s.colors.iter().collect::<Vec<_>>() == cycled {
            s.colors.clone_from(&r.colors);
        }
    }
    // A waterfall or funnel read as the stacked chart it was written as: rebuilt, in its palette.
    // A line radar read as the filled one it was written as (for its gradient).
    if let Some(k @ (ChartType::Radar | ChartType::RadarMarkers)) = r.drawn
        && s.kind == ChartType::RadarFilled
    {
        s.kind = k;
    }
    if let Some(k) = r.drawn
        && let Some(x) = crate::chart_spec::rebuilt(&s, k, r.total)
    {
        s = x;
        s.colors.clone_from(&r.colors);
    }
    // A hidden title's text, when the chart shows no title of its own.
    if let Some(t) = &r.hidden_title
        && s.title.is_none()
    {
        s.title = Some(t.clone());
        s.title_visible = false;
    }
    s.sanitized()
}
