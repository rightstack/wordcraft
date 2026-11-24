use super::*;
use wordcraft_doc::THEME_COLORS;

const NS: &str =
    r#"xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#;

fn space(plot: &str) -> El {
    let x = format!(r#"<c:chartSpace {NS}><c:chart><c:plotArea>{plot}</c:plotArea></c:chart></c:chartSpace>"#);
    crate::xml::parse(x.as_bytes()).unwrap()
}

fn theme() -> Vec<Rgb> {
    THEME_COLORS.to_vec()
}

fn ser(i: usize, name: &str, color: Option<&str>, cats: &[&str], vals: &[&str]) -> String {
    let fill = color.map_or(String::new(), |c| format!(r#"<c:spPr><a:solidFill><a:srgbClr val="{c}"/></a:solidFill></c:spPr>"#));
    let cat_pts: String = cats.iter().enumerate().map(|(k, c)| format!(r#"<c:pt idx="{k}"><c:v>{c}</c:v></c:pt>"#)).collect();
    let val_pts: String = vals.iter().enumerate().map(|(k, v)| format!(r#"<c:pt idx="{k}"><c:v>{v}</c:v></c:pt>"#)).collect();
    format!(
        r#"<c:ser><c:idx val="{i}"/><c:order val="{i}"/><c:tx><c:strRef><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>{name}</c:v></c:pt></c:strCache></c:strRef></c:tx>{fill}<c:cat><c:strRef><c:strCache><c:ptCount val="{nc}"/>{cat_pts}</c:strCache></c:strRef></c:cat><c:val><c:numRef><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="{nv}"/>{val_pts}</c:numCache></c:numRef></c:val></c:ser>"#,
        nc = cats.len(),
        nv = vals.len()
    )
}

fn filled(items: &[GraphicItem]) -> Vec<(Rgb, f32, f32)> {
    items
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Path { segs, fill: Some(c), .. } => {
                let ys: Vec<f32> = segs
                    .iter()
                    .filter_map(|s| match s {
                        PathSeg::Move(_, y) | PathSeg::Line(_, y) => Some(*y),
                        _ => None,
                    })
                    .collect();
                let lo = ys.iter().copied().fold(f32::INFINITY, f32::min);
                let hi = ys.iter().copied().fold(f32::NEG_INFINITY, f32::max);
                Some((*c, lo, hi))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn clustered_columns_draw_one_bar_per_point() {
    let plot = format!(
        r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{}{}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/><c:majorGridlines/></c:valAx>"#,
        ser(0, "North", Some("FF0000"), &["Q1", "Q2", "Q3"], &["4", "8", "2"]),
        ser(1, "South", Some("00FF00"), &["Q1", "Q2", "Q3"], &["2", "6", "4"]),
    );
    let items = chart_items(&space(&plot), &theme(), 300.0, 200.0);
    let bars = filled(&items);
    assert_eq!(bars.len(), 6);
    let heights: Vec<f32> = bars.iter().map(|(_, lo, hi)| hi - lo).collect();
    // Scale 0..8: North Q2 is the full plot height, North Q1 half of it.
    assert!((heights[1] / heights[0] - 2.0).abs() < 1e-3, "{heights:?}");
    // South (second series) Q1 is 2 of 8, Q2 is 6 of 8.
    assert!((heights[3] / heights[1] - 0.25).abs() < 1e-3, "{heights:?}");
    assert!((heights[4] / heights[1] - 0.75).abs() < 1e-3, "{heights:?}");
    assert_eq!(bars[0].0, Rgb(0xFF, 0, 0));
    assert_eq!(bars[3].0, Rgb(0, 0xFF, 0));
    assert!(items.iter().any(|it| matches!(it, GraphicItem::Text { text, .. } if text == "Q2")));
}

#[test]
fn data_labels_show_the_values() {
    let texts = |items: &[GraphicItem]| -> Vec<String> {
        items.iter().filter_map(|it| if let GraphicItem::Text { text, .. } = it { Some(text.clone()) } else { None }).collect()
    };
    let labels = r#"<c:dLbls><c:showLegendKey val="0"/><c:showVal val="1"/></c:dLbls>"#;
    let plot = format!(
        r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{}{labels}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#,
        ser(0, "North", None, &["Q1", "Q2"], &["4.25", "17"]),
    );
    let shown = texts(&chart_items(&space(&plot), &theme(), 300.0, 200.0));
    assert!(shown.contains(&"4.25".to_string()) && shown.contains(&"17".to_string()), "{shown:?}");
    let pie = format!(r#"<c:pieChart><c:varyColors val="1"/>{}{labels}</c:pieChart>"#, ser(0, "S", None, &["A", "B"], &["1", "3"]));
    let shown = texts(&chart_items(&space(&pie), &theme(), 200.0, 150.0));
    assert!(shown.contains(&"3".to_string()), "{shown:?}");
    // Without `c:dLbls`, no values are written next to the points.
    let plain = plot.replace(labels, "");
    assert!(!texts(&chart_items(&space(&plain), &theme(), 300.0, 200.0)).contains(&"4.25".to_string()));
}

#[test]
fn pie_draws_one_slice_per_point() {
    let plot =
        format!(r#"<c:pieChart><c:varyColors val="1"/>{}</c:pieChart>"#, ser(0, "Share", None, &["A", "B", "C", "D"], &["50", "25", "15", "10"]));
    let items = chart_items(&space(&plot), &theme(), 200.0, 150.0);
    let slices = filled(&items);
    assert_eq!(slices.len(), 4);
    assert_eq!(slices[0].0, theme()[4], "first slice is accent1");
}

#[test]
fn doughnut_slices_are_annuli() {
    let plot =
        format!(r#"<c:doughnutChart><c:varyColors val="1"/>{}<c:holeSize val="50"/></c:doughnutChart>"#, ser(0, "S", None, &["A", "B"], &["1", "3"]));
    let items = chart_items(&space(&plot), &theme(), 200.0, 150.0);
    assert_eq!(items.iter().filter(|it| matches!(it, GraphicItem::Path { fill: Some(_), .. })).count(), 2);
}

#[test]
fn nice_scale_rounds_to_round_steps() {
    assert_eq!(nice_scale(0.0, 22.3, true, None, None), Scale { lo: 0.0, hi: 25.0, step: 5.0 });
    assert_eq!(nice_scale(0.0, 100.0, true, None, None), Scale { lo: 0.0, hi: 120.0, step: 20.0 });
    assert_eq!(nice_scale(0.0, 22.3, true, Some(0.0), Some(30.0)).hi, 30.0);
    let flat = nice_scale(0.0, 0.0, true, None, None);
    assert!(flat.lo == 0.0 && (flat.hi - 1.2).abs() < 1e-9 && flat.step == 0.2, "{flat:?}");
}

#[test]
fn number_formats() {
    assert_eq!(format_value(1234567.0, "#,##0", 1.0), "1,234,567");
    assert_eq!(format_value(-1500.0, "#,##0", 500.0), "-1,500");
    assert_eq!(format_value(0.25, "0%", 0.05), "25%");
    assert_eq!(format_value(0.5, "General", 0.25), "0.50");
    assert_eq!(format_value(10.0, "\"€\"0.00", 5.0), "€10.00");
}

#[test]
fn hostile_values_never_panic() {
    let bodies = [
        format!(
            r#"<c:barChart><c:barDir val="col"/><c:grouping val="stacked"/><c:gapWidth val="-9999999999999999999999"/><c:overlap val="NaN"/>{}<c:axId val="1"/></c:barChart><c:valAx><c:axId val="1"/><c:scaling><c:max val="1e400"/><c:min val="-1e308"/></c:scaling></c:valAx>"#,
            r#"<c:ser><c:tx><c:v>x</c:v></c:tx><c:cat><c:strRef><c:strCache><c:ptCount val="99999999999"/><c:pt idx="18446744073709551615"><c:v>a</c:v></c:pt><c:pt idx="-3"><c:v>b</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:numCache><c:ptCount val="3"/><c:pt idx="0"><c:v>1e400</c:v></c:pt><c:pt idx="1"><c:v>NaN</c:v></c:pt><c:pt idx="2"><c:v>-1e308</c:v></c:pt><c:pt idx="99999"><c:v>5</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser><c:ser><c:val><c:numRef><c:numCache><c:pt idx="0"><c:v>1e308</c:v></c:pt><c:pt idx="0"><c:v>1e308</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>"#
        ),
        r#"<c:scatterChart><c:ser><c:xVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>-1e308</c:v></c:pt><c:pt idx="1"><c:v>1e308</c:v></c:pt></c:numLit></c:xVal><c:yVal><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numLit></c:yVal></c:ser></c:scatterChart>"#.to_string(),
        r#"<c:pieChart><c:ser><c:val><c:numLit><c:ptCount val="2"/><c:pt idx="0"><c:v>-4</c:v></c:pt><c:pt idx="1"><c:v>1e308</c:v></c:pt></c:numLit></c:val><c:dPt><c:idx val="9999999999999"/></c:dPt></c:ser><c:firstSliceAng val="-9"/></c:pieChart>"#.to_string(),
    ];
    for body in &bodies {
        let s = space(body);
        for (w, h) in [(0.0, 10.0), (f32::NAN, 10.0), (-1.0, 1.0), (1e30, 1e30), (50.0, 40.0)] {
            let _ = chart_items(&s, &theme(), w, h);
        }
    }
    assert!(chart_items(&space(&bodies[0]), &theme(), f32::NAN, 10.0).is_empty());
    assert!(chart_items(&El::default(), &theme(), 100.0, 100.0).is_empty());
}

fn texts(items: &[GraphicItem]) -> Vec<&str> {
    items.iter().filter_map(|it| if let GraphicItem::Text { text, .. } = it { Some(text.as_str()) } else { None }).collect()
}

#[test]
fn unsupported_chart_types_draw_a_placeholder() {
    let ser = r#"<c:ser><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser>"#;
    let title = r#"<c:title><c:tx><c:rich><a:p><a:r><a:t>Skills</a:t></a:r></a:p></c:rich></c:tx></c:title>"#;
    let x = format!(r#"<c:chartSpace {NS}><c:chart>{title}<c:plotArea><c:surfaceChart>{ser}</c:surfaceChart></c:plotArea></c:chart></c:chartSpace>"#);
    let bubble = chart_items(&crate::xml::parse(x.as_bytes()).unwrap(), &theme(), 300.0, 200.0);
    assert_eq!(texts(&bubble), ["Skills", "Surface chart (not supported yet)"]);
    let framed = bubble.iter().any(|it| matches!(it, GraphicItem::Shape { rect, stroke: Some(_), .. } if *rect == [0.0, 0.0, 300.0, 200.0]));
    assert!(framed, "a light border marks the chart's frame: {bubble:?}");

    let stock = chart_items(&space(&format!("<c:stockChart>{ser}{ser}{ser}</c:stockChart>")), &theme(), 300.0, 200.0);
    assert_eq!(texts(&stock), ["Stock chart (not supported yet)"]);
    let other = chart_items(&space(&format!("<c:fancyChart>{ser}</c:fancyChart>")), &theme(), 300.0, 200.0);
    assert_eq!(texts(&other), ["Chart (not supported yet)"]);
    // A supported type in the plot area is drawn, an unsupported one beside it is skipped.
    let combo = chart_items(&space(&format!("<c:stockChart>{ser}</c:stockChart><c:lineChart>{ser}</c:lineChart>")), &theme(), 300.0, 200.0);
    assert!(!texts(&combo).iter().any(|t| t.contains("not supported")), "{combo:?}");
}

fn radar_chart(style: &str, sers: &[String]) -> Vec<GraphicItem> {
    let plot = format!(r#"<c:radarChart><c:radarStyle val="{style}"/>{}</c:radarChart><c:valAx><c:majorGridlines/></c:valAx>"#, sers.concat());
    chart_items(&space(&plot), &theme(), 300.0, 220.0)
}

#[test]
fn radar_charts_draw_spokes_grid_and_series_polygons() {
    let cats = ["Speed", "Power", "Range", "Armor", "Luck"];
    let sers = [ser(0, "A", Some("FF0000"), &cats, &["3", "4", "5", "2", "1"]), ser(1, "B", Some("0000FF"), &cats, &["2", "2", "3", "5", "4"])];
    let std = radar_chart("standard", &sers);
    assert!(!texts(&std).iter().any(|t| t.contains("not supported")), "{std:?}");
    for c in cats {
        assert!(texts(&std).contains(&c), "category label {c}: {:?}", texts(&std));
    }
    let stroked = |items: &[GraphicItem], col: Rgb| {
        items.iter().filter(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == col && segs.len() == 6)).count()
    };
    assert_eq!(stroked(&std, Rgb(0xFF, 0, 0)), 1, "one closed pentagon per series: {std:?}");
    assert_eq!(stroked(&std, Rgb(0, 0, 0xFF)), 1);
    assert!(filled(&std).is_empty(), "standard radar is not filled");
    assert!(!std.iter().any(|it| matches!(it, GraphicItem::Shape { kind: ShapeKind::Ellipse, .. })));
    // 5 spokes (2 segs each) plus grid pentagons.
    let spokes = std.iter().filter(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == GRID && segs.len() == 2)).count();
    assert_eq!(spokes, 5);
    assert!(std.iter().any(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == GRID && segs.len() == 6)));

    let marked = radar_chart("marker", &sers);
    let dots = marked.iter().filter(|it| matches!(it, GraphicItem::Shape { kind: ShapeKind::Ellipse, .. })).count();
    assert_eq!(dots, 10, "a marker per point");

    let solid = radar_chart("filled", &sers);
    let f = filled(&solid);
    assert_eq!(f.len(), 2);
    assert!(f.iter().any(|(c, ..)| *c == Rgb(0xFF, 0, 0)) && f.iter().any(|(c, ..)| *c == Rgb(0, 0, 0xFF)));
}

#[test]
fn radar_charts_show_a_legend() {
    let cats = ["a", "b", "c"];
    let plot = format!("<c:radarChart>{}</c:radarChart>", ser(0, "Team", None, &cats, &["1", "2", "3"]));
    let items = with_legend(&plot, "r", 300.0, 200.0);
    assert!(texts(&items).contains(&"Team"), "{:?}", texts(&items));
}

#[test]
fn hostile_radar_charts_never_panic() {
    // No categories and no values.
    let empty = r#"<c:ser><c:cat><c:strLit><c:ptCount val="0"/></c:strLit></c:cat><c:val><c:numLit><c:ptCount val="0"/></c:numLit></c:val></c:ser>"#;
    let _ = chart_items(&space(&format!("<c:radarChart>{empty}</c:radarChart>")), &theme(), 300.0, 200.0);
    let none = chart_items(&space("<c:radarChart/>"), &theme(), 300.0, 200.0);
    assert!(none.is_empty(), "{none:?}");
    // One category: a degenerate polygon, nothing to crash on; two as well.
    for n in 1..=2 {
        let cats: Vec<String> = (0..n).map(|i| format!("c{i}")).collect();
        let refs: Vec<&str> = cats.iter().map(String::as_str).collect();
        let vals = vec!["5"; n];
        let items = radar_chart("filled", &[ser(0, "S", None, &refs, &vals)]);
        assert!(texts(&items).contains(&"c0"), "{items:?}");
    }
    // NaN, infinities, junk and a tiny frame.
    let junk = ser(0, "S", None, &["a", "b", "c"], &["NaN", "inf", "1e999"]);
    let _ = radar_chart("filled", &[junk]);
    let small =
        chart_items(&space(&format!("<c:radarChart>{}</c:radarChart>", ser(0, "S", None, &["a", "b", "c"], &["1", "2", "3"]))), &theme(), 20.0, 20.0);
    let _ = small;
    // Caps: thousands of categories and series stay bounded.
    let cats: Vec<String> = (0..5000).map(|i| format!("c{i}")).collect();
    let refs: Vec<&str> = cats.iter().map(String::as_str).collect();
    let vals = vec!["1"; 5000];
    let many: Vec<String> = (0..300).map(|i| ser(i, "S", None, &refs, &vals)).collect();
    let items = radar_chart("marker", &many);
    let polygons = items.iter().filter(|it| matches!(it, GraphicItem::Path { stroke: Some(c), .. } if *c != GRID)).count();
    assert!(polygons <= MAX_RADAR_SERIES, "{polygons}");
    let spokes = items.iter().filter(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == GRID && segs.len() == 2)).count();
    assert_eq!(spokes, MAX_SPOKES);
}

/// Chart items for a chart with a legend at `pos` (`r`, `b`…).
fn with_legend(plot: &str, pos: &str, w: f32, h: f32) -> Vec<GraphicItem> {
    let x = format!(
        r#"<c:chartSpace {NS}><c:chart><c:plotArea>{plot}</c:plotArea><c:legend><c:legendPos val="{pos}"/></c:legend></c:chart></c:chartSpace>"#
    );
    chart_items(&crate::xml::parse(x.as_bytes()).unwrap(), &theme(), w, h)
}

#[test]
fn series_are_capped_to_the_cell_budget() {
    // 256 series each declaring 10,000 points: only the 10 that fit the cell budget are kept.
    let sers: String = (0..256)
        .map(|i| format!(r#"<c:ser><c:idx val="{i}"/><c:val><c:numRef><c:numCache><c:ptCount val="10000"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser>"#))
        .collect();
    let plot = format!(r#"<c:lineChart><c:grouping val="standard"/>{sers}<c:axId val="1"/></c:lineChart>"#);
    let items = chart_items(&space(&plot), &theme(), 300.0, 200.0);
    let strokes = items.iter().filter(|it| matches!(it, GraphicItem::Path { fill: None, stroke: Some(_), .. })).count();
    assert_eq!(strokes, MAX_CELLS / 10_000);
}

#[test]
fn series_without_values_draw_nothing() {
    let plot = format!(
        r#"<c:areaChart><c:grouping val="standard"/>{}{}<c:axId val="1"/></c:areaChart>"#,
        ser(0, "A", Some("FF0000"), &["Q1", "Q2"], &["4", "8"]),
        ser(1, "Empty", Some("00FF00"), &["Q1", "Q2"], &[]),
    );
    let fills = filled(&chart_items(&space(&plot), &theme(), 300.0, 200.0));
    assert_eq!(fills.len(), 1);
    assert_eq!(fills[0].0, Rgb(0xFF, 0, 0));
}

#[test]
fn value_scale_starts_bars_at_zero_for_negative_data() {
    // All-negative data pads below only: the axis ends at zero, as positive data's starts there.
    assert_eq!(nice_scale(-10.0, -2.0, true, None, None), Scale { lo: -12.0, hi: 0.0, step: 2.0 });
    assert_eq!(nice_scale(-1050.0, -1000.0, false, None, None).hi, -1000.0);
}

#[test]
fn narrow_line_scales_leave_zero_out_but_bars_keep_it() {
    assert!(narrow(1000.0, 1050.0) && narrow(-1050.0, -1000.0));
    assert!(!narrow(10.0, 100.0) && !narrow(0.0, 1.0));
    assert_eq!(nice_scale(1000.0, 1050.0, false, None, None).lo, 1000.0);
    assert_eq!(nice_scale(1000.0, 1050.0, true, None, None).lo, 0.0);
}

#[test]
fn horizontal_bar_labels_wrap_in_a_column_of_forty_percent() {
    let long = "A category label that is much too long for its column";
    let plot = format!(
        r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/>{}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#,
        ser(0, "S", Some("FF0000"), &[long, "Short"], &["4", "8"]),
    );
    let items = chart_items(&space(&plot), &theme(), 300.0, 200.0);
    let labels: Vec<(&str, f32)> = items
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Text { rect, text, align: TextAlign::Right, .. } => Some((text.as_str(), rect[2])),
            _ => None,
        })
        .collect();
    assert!(labels.iter().all(|(_, w)| *w <= 0.4 * 300.0), "{labels:?}");
    assert!(labels.iter().filter(|(t, _)| long.contains(t)).count() > 1, "wrapped: {labels:?}");
}

#[test]
fn stacked_lines_add_up_like_areas() {
    let stacked = format!(
        r#"<c:lineChart><c:grouping val="stacked"/>{}{}<c:axId val="1"/></c:lineChart>"#,
        ser(0, "A", Some("FF0000"), &["Q1", "Q2"], &["0", "10"]),
        ser(1, "B", Some("00FF00"), &["Q1", "Q2"], &["0", "5"]),
    );
    let single = format!(
        r#"<c:lineChart><c:grouping val="standard"/>{}<c:axId val="1"/></c:lineChart>"#,
        ser(0, "T", Some("0000FF"), &["Q1", "Q2"], &["0", "15"])
    );
    // Where the last point of the line drawn in `colour` is.
    let end_y = |items: &[GraphicItem], colour: Rgb| {
        items.iter().find_map(|it| match it {
            GraphicItem::Path { segs, stroke: Some(c), .. } if *c == colour => match segs.last() {
                Some(PathSeg::Line(_, y)) => Some(*y),
                _ => None,
            },
            _ => None,
        })
    };
    let top = end_y(&chart_items(&space(&stacked), &theme(), 300.0, 200.0), Rgb(0, 0xFF, 0)).expect("stacked line");
    let total = end_y(&chart_items(&space(&single), &theme(), 300.0, 200.0), Rgb(0, 0, 0xFF)).expect("single line");
    assert!((top - total).abs() < 1e-3, "stacked {top} vs {total}");
}

#[test]
fn category_numbers_show_in_their_format() {
    // Excel serial days: 45292 is 1 January 2024, 45323 is 1 February 2024.
    let cat = |code: &str| -> Vec<String> {
        let x = format!(
            r#"<c:cat {NS}><c:numRef><c:numCache><c:formatCode>{code}</c:formatCode><c:ptCount val="2"/><c:pt idx="0"><c:v>45292</c:v></c:pt><c:pt idx="1"><c:v>45323</c:v></c:pt></c:numCache></c:numRef></c:cat>"#
        );
        labels(Some(&crate::xml::parse(x.as_bytes()).unwrap()), 10, false)
    };
    assert_eq!(cat("m/d/yyyy"), ["1/1/2024", "2/1/2024"]);
    assert_eq!(cat("d/m/yyyy"), ["1/1/2024", "1/2/2024"]);
    assert_eq!(cat("mmm-yy"), ["Jan-24", "Feb-24"]);
    assert_eq!(cat("yyyy"), ["2024", "2024"]);
    assert_eq!(cat("0.00"), ["45292.00", "45323.00"]);
    let ymd = |v: f64| excel_stamp(v, false).map(|t| (t.y, t.m, t.d));
    assert_eq!(ymd(61.0), Some((1900, 3, 1)));
    assert_eq!(ymd(45292.0), Some((2024, 1, 1)));
}

#[test]
fn category_format_code_length_is_capped() {
    let cat = |code: &str| -> Vec<String> {
        let x = format!(
            r#"<c:cat {NS}><c:numRef><c:numCache><c:formatCode>{code}</c:formatCode><c:ptCount val="1"/><c:pt idx="0"><c:v>5</c:v></c:pt></c:numCache></c:numRef></c:cat>"#
        );
        labels(Some(&crate::xml::parse(x.as_bytes()).unwrap()), 10, false)
    };
    let ok = format!("{}0", "x".repeat(254));
    assert!(cat(&ok)[0].starts_with("xxx"));
    let long = format!("{}0", "x".repeat(300));
    assert_eq!(cat(&long), ["5"]);
}

#[test]
fn value_format_code_length_is_capped() {
    let val = |code: &str| -> Option<String> {
        let x = format!(
            r#"<c:val {NS}><c:numRef><c:numCache><c:formatCode>{code}</c:formatCode><c:ptCount val="1"/><c:pt idx="0"><c:v>5</c:v></c:pt></c:numCache></c:numRef></c:val>"#
        );
        numbers(Some(&crate::xml::parse(x.as_bytes()).unwrap()), 10).1
    };
    let ok = format!("{}0", "x".repeat(254));
    assert_eq!(val(&ok), Some(ok.clone()));
    assert_eq!(val(&format!("{}0", "x".repeat(300))), None);
}

#[test]
fn axis_format_code_length_is_capped() {
    let fmt = |code: &str| -> Option<String> {
        let x = format!(r#"<c:valAx {NS}><c:numFmt formatCode="{code}" sourceLinked="0"/></c:valAx>"#);
        axis(Some(&crate::xml::parse(x.as_bytes()).unwrap()), &theme()).fmt
    };
    let ok = format!("{}0", "x".repeat(254));
    assert_eq!(fmt(&ok), Some(ok.clone()));
    assert_eq!(fmt(&format!("{}0", "x".repeat(300))), None);
}

#[test]
fn civil_is_exact_beyond_chronos_range() {
    // Serial 1 is 1899-12-31, serial 60 Excel's phantom 1900-02-29 handled by `excel_stamp`.
    assert_eq!(civil(1), (1899, 12, 31));
    // Past `NaiveDate`'s ±262143 the 400-year cycles carry the year, exactly, both sides of
    // the epoch; values from the pre-chrono proleptic Gregorian formula.
    assert_eq!(civil(100_000_000), (275_690, 9, 11));
    assert_eq!(civil(-100_000_000), (-271_891, 4, 18));
    // The extremes must not overflow or panic (i128 intermediates).
    let _ = (civil(i64::MAX), civil(i64::MIN));
}

#[test]
fn number_formats_keep_literal_text_and_currency() {
    assert_eq!(format_value(1234.5, "#,##0.00 [$€-407]", 0.25), "1,234.50 €");
    assert_eq!(format_value(5.0, "[Red]0", 1.0), "5");
    assert_eq!(format_value(3.0, "\"Items: \"0", 1.0), "Items: 3");
    assert_eq!(format_value(3.0, "0\" units\"", 1.0), "3 units");
}

#[test]
fn side_legend_keeps_only_the_entries_that_fit() {
    let sers: String = (0..10).map(|i| ser(i, &format!("S{i}"), Some("FF0000"), &["Q1"], &["1"])).collect();
    let plot = format!(
        r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{sers}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#
    );
    let items = with_legend(&plot, "r", 300.0, 60.0);
    let swatches = items.iter().filter(|it| matches!(it, GraphicItem::Shape { rect, .. } if rect[2] == 7.0)).count();
    assert!(swatches > 0 && swatches < 10, "{swatches} swatches");
    let bottom =
        items.iter().filter_map(|it| if let GraphicItem::Text { rect, .. } = it { Some(rect[1] + rect[3]) } else { None }).fold(0.0, f32::max);
    assert!(bottom <= 60.0, "legend text passes the chart's bottom: {bottom}");
}

/// The swatches of a legend: their (x, y).
fn swatches(items: &[GraphicItem]) -> Vec<(f32, f32)> {
    items
        .iter()
        .filter_map(|it| if let GraphicItem::Shape { rect, .. } = it { (rect[2] == SWATCH).then_some((rect[0], rect[1])) } else { None })
        .collect()
}

/// A column chart of one category with a series per name.
fn columns(names: &[&str]) -> String {
    let sers: String = names.iter().enumerate().map(|(i, n)| ser(i, n, Some("FF0000"), &["Q1"], &["1"])).collect();
    format!(
        r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{sers}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#
    )
}

#[test]
fn bottom_legend_wraps_into_rows_inside_the_chart() {
    let names: Vec<String> = (0..12).map(|i| format!("Region number {i}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let items = with_legend(&columns(&names), "b", 200.0, 200.0);
    let sw = swatches(&items);
    let rows: std::collections::BTreeSet<i32> = sw.iter().map(|(_, y)| *y as i32).collect();
    assert!(rows.len() > 1 && rows.len() <= MAX_LEGEND_ROWS, "rows {rows:?}");
    assert!(sw.iter().all(|(x, _)| *x >= 0.0 && *x <= 200.0), "{sw:?}");
    for it in &items {
        if let GraphicItem::Text { rect, text, .. } = it {
            assert!(rect[0] >= 0.0 && rect[0] + text_w(text, FONT) <= 200.0 + 1e-3, "{text:?} at {rect:?}");
        }
    }
}

#[test]
fn long_legend_names_are_cut_to_a_share_of_the_chart() {
    let long = "A very long series name that would push the legend past the left edge of the chart";
    let items = with_legend(&columns(&[long, "Short"]), "r", 200.0, 150.0);
    assert!(swatches(&items).iter().all(|(x, _)| *x >= 0.0), "{:?}", swatches(&items));
    let cut = items.iter().find_map(|it| match it {
        GraphicItem::Text { text, rect, .. } if text.ends_with('…') => Some((text.clone(), rect[2])),
        _ => None,
    });
    let (text, w) = cut.expect("the long name is cut");
    assert!(text_w(&text, FONT) <= 200.0 * LEGEND_SHARE && w <= 200.0 * LEGEND_SHARE + 2.0, "{text:?} {w}");
}

#[test]
fn scatter_dates_show_as_dates() {
    let pts = |vals: &[&str]| -> String { vals.iter().enumerate().map(|(k, v)| format!(r#"<c:pt idx="{k}"><c:v>{v}</c:v></c:pt>"#)).collect() };
    let plot = format!(
        r#"<c:scatterChart><c:ser><c:idx val="0"/><c:xVal><c:numRef><c:numCache><c:formatCode>m/d/yyyy</c:formatCode><c:ptCount val="3"/>{}</c:numCache></c:numRef></c:xVal><c:yVal><c:numRef><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/>{}</c:numCache></c:numRef></c:yVal></c:ser><c:axId val="1"/><c:axId val="2"/></c:scatterChart><c:valAx><c:axId val="1"/><c:axPos val="b"/></c:valAx><c:valAx><c:axId val="2"/><c:axPos val="l"/></c:valAx>"#,
        pts(&["45292", "45323", "45352"]),
        pts(&["1", "2", "3"])
    );
    let items = chart_items(&space(&plot), &theme(), 300.0, 200.0);
    let texts: Vec<&str> = items.iter().filter_map(|it| if let GraphicItem::Text { text, .. } = it { Some(text.as_str()) } else { None }).collect();
    assert!(texts.iter().any(|t| t.ends_with("/2024")), "{texts:?}");
    assert!(!texts.iter().any(|t| t.starts_with("45")), "serials shown: {texts:?}");
}

#[test]
fn date1904_counts_from_1904() {
    assert_eq!(category_text(0.0, "m/d/yyyy", 1.0, true), "1/1/1904");
    assert_eq!(category_text(1462.0, "m/d/yyyy", 1.0, false), "1/1/1904");
    let x = format!(
        r#"<c:chartSpace {NS}><c:date1904 val="1"/><c:chart><c:plotArea><c:lineChart><c:ser><c:cat><c:numRef><c:numCache><c:formatCode>yyyy-mm-dd</c:formatCode><c:ptCount val="1"/><c:pt idx="0"><c:v>366</c:v></c:pt></c:numCache></c:numRef></c:cat><c:val><c:numLit><c:ptCount val="1"/><c:pt idx="0"><c:v>1</c:v></c:pt></c:numLit></c:val></c:ser></c:lineChart><c:catAx/><c:valAx/></c:plotArea></c:chart></c:chartSpace>"#
    );
    let chart = Chart::parse(&crate::xml::parse(x.as_bytes()).unwrap(), &theme()).expect("chart");
    assert_eq!(chart.cats, ["1905-01-01"]);
}

#[test]
fn date_codes_show_times_weekdays_and_early_serials() {
    let t = |v: f64, code: &str| category_text(v, code, 1.0, false);
    // 45292 is Monday 1 January 2024; 13:05:09 into it.
    let at = 45292.0 + (13.0 * 3600.0 + 5.0 * 60.0 + 9.0) / 86_400.0;
    assert_eq!(t(at, "hh:mm:ss"), "13:05:09");
    assert_eq!(t(at, "h:mm AM/PM"), "1:05 PM");
    assert_eq!(t(at, "m:ss"), "5:09");
    assert_eq!(t(45292.0, "h:mm a/p"), "12:00 A");
    assert_eq!(t(at, "mm/dd"), "01/01");
    assert_eq!(t(45292.0, "dddd, mmmm d"), "Monday, January 1");
    assert_eq!(t(45292.0, "ddd d mmm"), "Mon 1 Jan");
    // Serials before March 1900, Excel's phantom 29 February included.
    assert_eq!(t(0.0, "m/d/yyyy"), "1/0/1900");
    assert_eq!(t(1.0, "m/d/yyyy"), "1/1/1900");
    assert_eq!(t(59.0, "m/d/yyyy"), "2/28/1900");
    assert_eq!(t(60.0, "m/d/yyyy"), "2/29/1900");
    assert_eq!(t(61.0, "m/d/yyyy dddd"), "3/1/1900 Thursday");
    // Outside the calendar: a number.
    assert_eq!(t(-1.0, "m/d/yyyy"), "-1");
}

#[test]
fn negative_sections_and_scientific_numbers() {
    assert_eq!(format_value(-5.0, "#,##0;(#,##0)", 1.0), "(5)");
    assert_eq!(format_value(5.0, "#,##0;(#,##0)", 1.0), "5");
    assert_eq!(format_value(-1500.0, "#,##0_);(#,##0)", 500.0), "(1,500)");
    assert_eq!(format_value(-2.0, "0\";\"0;-0", 1.0), "-2");
    assert_eq!(format_value(12345.0, "0.00E+00", 1.0), "1.23E+04");
    assert_eq!(format_value(0.00012, "0.0E+0", 0.0001), "1.2E-4");
    assert_eq!(format_value(0.0, "0.00E+00", 1.0), "0.00E+00");
    assert_eq!(format_value(-12345.0, "0.0E-00", 1.0), "-1.2E04");
}

#[test]
fn a_major_unit_sets_the_step_and_is_kept_under_the_tick_limit() {
    assert_eq!(scale(0.0, 22.3, true, None, None, Some(4.0)), Scale { lo: 0.0, hi: 24.0, step: 4.0 });
    assert_eq!(scale(0.0, 22.3, true, Some(-10.0), Some(50.0), Some(10.0)), Scale { lo: -10.0, hi: 50.0, step: 10.0 });
    let tiny = scale(0.0, 100.0, true, None, None, Some(1e-9));
    assert!(tiny.ticks().len() <= MAX_TICKS + 1 && tiny.step > 1.0, "{tiny:?}");
    // A unit that isn't a positive finite number is automatic; a minimum above the data still scales.
    assert_eq!(scale(0.0, 22.3, true, None, None, Some(-1.0)), nice_scale(0.0, 22.3, true, None, None));
    let above = scale(0.0, 10.0, true, Some(50.0), None, None);
    assert!(above.lo == 50.0 && above.hi > 50.0, "{above:?}");
    for unit in [f64::NAN, f64::INFINITY, 0.0, f64::MIN_POSITIVE, 1e300] {
        let s = scale(-1e300, 1e300, true, None, None, Some(unit));
        assert!(s.step.is_finite() && s.step > 0.0 && s.ticks().len() <= MAX_TICKS + 1, "{unit}: {s:?}");
    }
}

#[test]
fn dashes_split_a_line_into_pieces() {
    let d = canvas::dashes(0.0, 0.0, 70.0, 0.0, 4.0, 3.0);
    assert_eq!(d.len(), 20, "10 dashes of 4 with gaps of 3");
    assert_eq!(&d[..4], &[PathSeg::Move(0.0, 0.0), PathSeg::Line(4.0, 0.0), PathSeg::Move(7.0, 0.0), PathSeg::Line(11.0, 0.0)]);
    // A huge line is held to the dash limit; a degenerate one is one piece.
    assert!(canvas::dashes(0.0, 0.0, 1e9, 0.0, 4.0, 3.0).len() <= 4002);
    assert_eq!(canvas::dashes(1.0, 1.0, 1.0, 1.0, 4.0, 3.0).len(), 2);
    assert_eq!(canvas::dashes(0.0, 0.0, f32::NAN, 0.0, 4.0, 3.0).len(), 2);
}

/// Which corners of a filled shape are rounded: top-left, top-right, bottom-right, bottom-left.
/// A square corner is a point of the path; a rounded one isn't.
fn rounded_corners(segs: &[PathSeg]) -> [bool; 4] {
    let pts: Vec<(f32, f32)> = segs
        .iter()
        .filter_map(|s| match s {
            PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(.., x, y) => Some((*x, *y)),
            PathSeg::Close => None,
        })
        .collect();
    let (x0, x1) = (pts.iter().map(|p| p.0).fold(f32::INFINITY, f32::min), pts.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max));
    let (y0, y1) = (pts.iter().map(|p| p.1).fold(f32::INFINITY, f32::min), pts.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max));
    let round = |c: (f32, f32)| !pts.iter().any(|p| (p.0 - c.0).abs() < 1e-4 && (p.1 - c.1).abs() < 1e-4);
    [round((x0, y0)), round((x1, y0)), round((x1, y1)), round((x0, y1))]
}

#[test]
fn rounded_ends_round_only_their_side() {
    const TOP: [bool; 4] = [true, true, false, false];
    let r = canvas::round_end_segs(0.0, 0.0, 20.0, 100.0, 4.0, canvas::End::Top);
    assert_eq!(rounded_corners(&r), TOP);
    assert_eq!(r.first(), Some(&PathSeg::Move(4.0, 0.0)));
    assert_eq!(rounded_corners(&canvas::round_end_segs(0.0, 0.0, 20.0, 100.0, 4.0, canvas::End::Bottom)), [false, false, true, true]);
    assert_eq!(rounded_corners(&canvas::round_end_segs(0.0, 0.0, 100.0, 20.0, 4.0, canvas::End::Left)), [true, false, false, true]);
    assert_eq!(rounded_corners(&canvas::round_end_segs(0.0, 0.0, 100.0, 20.0, 4.0, canvas::End::Right)), [false, true, true, false]);
    // The radius is held to half the width across the end and to the length along the bar.
    assert_eq!(canvas::round_end_segs(0.0, 0.0, 2.0, 100.0, 4.0, canvas::End::Top).first(), Some(&PathSeg::Move(1.0, 0.0)));
    assert_eq!(canvas::round_end_segs(0.0, 0.0, 20.0, 1.5, 4.0, canvas::End::Top).first(), Some(&PathSeg::Move(1.5, 0.0)));
    // A flat or broken one is a plain rectangle; upside-down corners are put in order.
    assert_eq!(canvas::round_end_segs(0.0, 5.0, 20.0, 5.0, 4.0, canvas::End::Top), canvas::rect_segs(0.0, 5.0, 20.0, 5.0));
    assert_eq!(canvas::round_end_segs(0.0, 0.0, f32::NAN, 5.0, 4.0, canvas::End::Top).len(), 5);
    assert_eq!(canvas::round_end_segs(20.0, 100.0, 0.0, 0.0, 4.0, canvas::End::Top), r);
}

/// The filled bars of a bar chart, per series colour in drawing order.
fn rounded_bars(dir: &str, grouping: &str, sers: &[(&str, &[&str])]) -> Vec<(Rgb, [bool; 4])> {
    let geom = |c: &str| {
        format!(r#"<c:spPr><a:prstGeom prst="roundRect"><a:avLst/></a:prstGeom><a:solidFill><a:srgbClr val="{c}"/></a:solidFill></c:spPr>"#)
    };
    let body: String = sers
        .iter()
        .enumerate()
        .map(|(j, (c, vals))| ser(j, "S", None, &["a", "b"], vals).replace("<c:cat>", &format!("{}<c:cat>", geom(c))))
        .collect();
    let overlap = if grouping == "stacked" { r#"<c:overlap val="100"/>"# } else { "" };
    let plot = format!(
        r#"<c:barChart><c:barDir val="{dir}"/><c:grouping val="{grouping}"/>{body}{overlap}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#
    );
    chart_items(&space(&plot), &theme(), 300.0, 200.0)
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Path { segs, fill: Some(c), .. } if *c != Rgb::WHITE => Some((*c, rounded_corners(segs))),
            _ => None,
        })
        .collect()
}

#[test]
fn rounded_bars_round_the_value_end_and_keep_the_axis_side_square() {
    const NONE: [bool; 4] = [false; 4];
    let (a, b) = (Rgb(0x22, 0x33, 0x44), Rgb(0x55, 0x66, 0x77));
    // Columns: top corners up, bottom corners for a negative value.
    let col = rounded_bars("col", "clustered", &[("223344", &["3", "-2"])]);
    assert_eq!(col, vec![(a, [true, true, false, false]), (a, [false, false, true, true])]);
    // Horizontal bars: right corners, left for a negative value.
    let bar = rounded_bars("bar", "clustered", &[("223344", &["3", "-2"])]);
    assert_eq!(bar, vec![(a, [false, true, true, false]), (a, [true, false, false, true])]);
    // A zero value draws no rounded corners.
    assert_eq!(rounded_bars("col", "clustered", &[("223344", &["1", "0"])])[1], (a, NONE));
    // Stacks: only the outermost segment in each direction, at its outer end.
    let stack = rounded_bars("col", "stacked", &[("223344", &["2", "2"]), ("556677", &["1", "-1"])]);
    assert_eq!(
        stack,
        vec![
            (a, NONE),                       // category a, inner positive segment
            (a, [true, true, false, false]), // category b: the top of its positive stack
            (b, [true, true, false, false]), // category a: the top of its positive stack
            (b, [false, false, true, true]), // category b: the bottom of its negative stack
        ]
    );
    let hstack = rounded_bars("bar", "stacked", &[("223344", &["2", "2"]), ("556677", &["1", "-1"])]);
    assert_eq!(hstack, vec![(a, NONE), (a, [false, true, true, false]), (b, [false, true, true, false]), (b, [true, false, false, true])]);
}

#[test]
fn curves_pass_through_every_point_and_break_at_gaps() {
    let pts = [Some((0.0, 0.0)), Some((10.0, 10.0)), Some((20.0, 0.0)), None, Some((40.0, 5.0)), Some((50.0, 5.0))];
    let segs = canvas::curves(pts.into_iter());
    let moves = segs.iter().filter(|s| matches!(s, PathSeg::Move(..))).count();
    let ends: Vec<(f32, f32)> = segs.iter().filter_map(|s| if let PathSeg::Cubic(.., x, y) = s { Some((*x, *y)) } else { None }).collect();
    assert_eq!(moves, 2);
    assert_eq!(ends, vec![(10.0, 10.0), (20.0, 0.0), (50.0, 5.0)]);
    // At the peak the tangent is level: both handles at the point's height.
    let PathSeg::Cubic(_, _, _, c2y, ..) = segs[1] else { panic!() };
    let PathSeg::Cubic(_, c1y, ..) = segs[2] else { panic!() };
    assert_eq!((c2y, c1y), (10.0, 10.0));
}

/// A line chart whose series has `extra` markup before its categories and `smooth` after its
/// values, with `val_ax` inside its value axis.
fn line_chart(extra: &str, smooth: &str, val_ax: &str) -> Vec<GraphicItem> {
    let s = ser(0, "A", None, &["a", "b", "c", "d"], &["1", "3", "2", "4"])
        .replace("</c:val>", &format!("</c:val>{smooth}"))
        .replace("<c:cat>", &format!("{extra}<c:cat>"));
    let plot = format!(
        r#"<c:lineChart><c:grouping val="standard"/>{s}<c:axId val="1"/><c:axId val="2"/></c:lineChart><c:catAx><c:axId val="1"/><c:crossAx val="2"/></c:catAx><c:valAx><c:axId val="2"/>{val_ax}<c:crossAx val="1"/></c:valAx>"#
    );
    chart_items(&space(&plot), &theme(), 300.0, 200.0)
}

fn cubics(items: &[GraphicItem]) -> usize {
    items
        .iter()
        .map(|it| if let GraphicItem::Path { segs, .. } = it { segs.iter().filter(|s| matches!(s, PathSeg::Cubic(..))).count() } else { 0 })
        .sum()
}

fn ellipses(items: &[GraphicItem]) -> Vec<(Rgb, f32)> {
    items
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Shape { kind: ShapeKind::Ellipse, fill: Some(c), rect, .. } => Some((*c, rect[2])),
            _ => None,
        })
        .collect()
}

#[test]
fn smooth_lines_are_curves_and_markers_are_circles() {
    assert_eq!(cubics(&line_chart("", r#"<c:smooth val="0"/>"#, "")), 0);
    assert_eq!(cubics(&line_chart("", r#"<c:smooth val="1"/>"#, "")), 3);
    assert!(ellipses(&line_chart(r#"<c:marker><c:symbol val="none"/></c:marker>"#, "", "")).is_empty());
    let m = ellipses(&line_chart(
        r#"<c:marker><c:symbol val="circle"/><c:size val="7"/><c:spPr><a:solidFill><a:srgbClr val="112233"/></a:solidFill></c:spPr></c:marker>"#,
        "",
        "",
    ));
    assert_eq!(m, vec![(Rgb(0x11, 0x22, 0x33), 7.0); 4]);
    let huge = ellipses(&line_chart(r#"<c:marker><c:symbol val="circle"/><c:size val="99999999999"/></c:marker>"#, "", ""));
    assert!(huge.len() == 4 && huge.iter().all(|(_, d)| *d == 72.0), "{huge:?}");
}

/// Paths in the gridline colour, and how many sub-paths each has.
fn gridlines(items: &[GraphicItem]) -> Vec<usize> {
    items
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Path { segs, stroke: Some(c), .. } if *c == GRID => Some(segs.iter().filter(|s| matches!(s, PathSeg::Move(..))).count()),
            _ => None,
        })
        .collect()
}

#[test]
fn dashed_gridlines_are_drawn_in_dashes_and_absent_ones_not_at_all() {
    let solid = gridlines(&line_chart("", "", "<c:majorGridlines/>"));
    assert!(!solid.is_empty() && solid.iter().all(|n| *n == 1), "{solid:?}");
    let dashed = gridlines(&line_chart(
        "",
        "",
        r#"<c:majorGridlines><c:spPr><a:ln w="9525"><a:solidFill><a:srgbClr val="D9D9D9"/></a:solidFill><a:prstDash val="dash"/></a:ln></c:spPr></c:majorGridlines>"#,
    ));
    assert_eq!(dashed.len(), solid.len());
    assert!(dashed.iter().all(|n| *n > 10), "{dashed:?}");
    assert!(gridlines(&line_chart("", "", "")).is_empty());
}

#[test]
fn axis_scaling_and_major_unit_set_the_ticks() {
    let auto = line_chart("", "", "<c:majorGridlines/>");
    assert!(texts(&auto).contains(&"5") && !texts(&auto).contains(&"7"), "{:?}", texts(&auto));
    let set = line_chart(
        "",
        "",
        r#"<c:scaling><c:orientation val="minMax"/><c:max val="10"/><c:min val="-2"/></c:scaling><c:majorGridlines/><c:majorUnit val="3"/>"#,
    );
    let t = texts(&set);
    for want in ["-2", "1", "4", "7", "10"] {
        assert!(t.contains(&want), "{want} in {t:?}");
    }
    assert!(!t.contains(&"5"));
    assert_eq!(gridlines(&set).len(), 5);
}

#[test]
fn stacked_areas_sit_on_each_other_and_take_their_colours() {
    let a = ser(0, "A", Some("102030"), &["a", "b"], &["2", "2"]);
    let b = ser(1, "B", Some("405060"), &["a", "b"], &["3", "3"]);
    let plot = format!(
        r#"<c:areaChart><c:grouping val="stacked"/>{a}{b}<c:axId val="1"/><c:axId val="2"/></c:areaChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/><c:scaling><c:max val="10"/><c:min val="0"/></c:scaling></c:valAx>"#
    );
    let f = filled(&chart_items(&space(&plot), &theme(), 300.0, 200.0));
    let lower = f.iter().find(|x| x.0 == Rgb(0x10, 0x20, 0x30)).unwrap();
    let upper = f.iter().find(|x| x.0 == Rgb(0x40, 0x50, 0x60)).unwrap();
    // The upper area's bottom edge is the lower area's top edge.
    assert!((upper.2 - lower.1).abs() < 0.01, "{lower:?} {upper:?}");
    assert!(upper.1 < lower.1);
}

#[test]
fn hangul_is_measured_full_width_and_legend_gaps_are_even() {
    // Wide characters take a full em; others keep the estimate (digits and Latin unchanged).
    assert_eq!(text_w("아시아", 10.0), 30.0);
    assert_eq!(text_w("2020", 10.0), 4.0 * 10.0 * CHAR_W);
    assert!((text_w("K-팝", 10.0) - (2.0 * CHAR_W + 1.0) * 10.0).abs() < 1e-4);
    assert_eq!(text_w("漢字かな", 10.0), 40.0);
    // A cut label fits its room by the same widths.
    let cut = canvas::clip_text("아시아태평양지역", 40.0, 10.0);
    assert!(text_w(&cut, 10.0) <= 40.0 && cut.ends_with('…'), "{cut}");
    // A bottom legend: every gap from the end of a label to the next key is the same, whatever the script.
    let names = ["아시아", "유럽", "게임", "K-팝", "드라마", "2020"];
    let items = with_legend(&columns(&names), "b", 400.0, 200.0);
    let keys = swatches(&items);
    let labels: Vec<(f32, &str)> = items
        .iter()
        .filter_map(|it| match it {
            GraphicItem::Text { rect, text, .. } if names.contains(&text.as_str()) => Some((rect[0], text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!((keys.len(), labels.len()), (6, 6), "one row: {keys:?}");
    let size = FONT;
    let gaps: Vec<f32> = labels.windows(2).zip(keys.iter().skip(1)).map(|(w, key)| key.0 - (w[0].0 + text_w(w[0].1, size))).collect();
    assert!(gaps.iter().all(|g| (g - gaps[0]).abs() < 1e-3 && *g > 5.0), "{gaps:?}");
}

#[test]
fn scatter_x_axis_leaves_room_left_of_the_first_points() {
    let scatter = |x_axis: &str| -> Vec<GraphicItem> {
        let pts = |vals: &[&str]| -> String { vals.iter().enumerate().map(|(k, v)| format!(r#"<c:pt idx="{k}"><c:v>{v}</c:v></c:pt>"#)).collect() };
        let plot = format!(
            r#"<c:scatterChart><c:ser><c:idx val="0"/><c:spPr><a:ln><a:noFill/></a:ln></c:spPr><c:xVal><c:numLit><c:ptCount val="4"/>{}</c:numLit></c:xVal><c:yVal><c:numLit><c:ptCount val="4"/>{}</c:numLit></c:yVal></c:ser><c:axId val="1"/><c:axId val="2"/></c:scatterChart><c:valAx><c:axId val="1"/>{x_axis}<c:axPos val="b"/></c:valAx><c:valAx><c:axId val="2"/><c:axPos val="l"/></c:valAx>"#,
            pts(&["40", "50", "60", "70"]),
            pts(&["1", "2", "3", "4"])
        );
        chart_items(&space(&plot), &theme(), 300.0, 200.0)
    };
    let items = scatter("");
    let t = texts(&items);
    assert!(t.contains(&"35") && t.contains(&"75") && !t.contains(&"30"), "x ticks 35..75: {t:?}");
    // The first point is right of the y axis (the plot's left edge, where the gridlines start).
    let left = items
        .iter()
        .filter_map(|it| {
            if let GraphicItem::Path { segs, stroke: Some(_), .. } = it {
                segs.first().and_then(|s| if let PathSeg::Move(x, _) = s { Some(*x) } else { None })
            } else {
                None
            }
        })
        .fold(f32::INFINITY, f32::min);
    let first = items
        .iter()
        .filter_map(|it| if let GraphicItem::Shape { kind: ShapeKind::Ellipse, rect, .. } = it { Some(rect[0] + rect[2] / 2.0) } else { None })
        .fold(f32::INFINITY, f32::min);
    assert!(first > left + 5.0, "first point at {first}, axis at {left}");
    // A minimum the file sets is kept; data from zero starts at zero.
    let t = texts(&scatter(r#"<c:scaling><c:orientation val="minMax"/><c:min val="40"/></c:scaling>"#)).join(" ");
    assert!(t.contains("40") && !t.contains("35"), "{t}");
}

#[test]
fn text_takes_the_colour_its_element_gives() {
    let txpr = |c: &str| format!(r#"<c:txPr><a:bodyPr/><a:p><a:pPr><a:defRPr><a:solidFill>{c}</a:solidFill></a:defRPr></a:pPr></a:p></c:txPr>"#);
    let red = r#"<a:srgbClr val="FF0000"/>"#;
    let blue = r#"<a:srgbClr val="0000FF"/>"#;
    let s = ser(0, "Units", None, &["Q1", "Q2"], &["1", "2"]);
    let chart = |cat: &str, val: &str, legend: &str, title: &str| -> Vec<(String, Rgb)> {
        let x = format!(
            r#"<c:chartSpace {NS}><c:chart><c:title><c:tx><c:rich><a:bodyPr/><a:p><a:r>{title}<a:t>Title</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{s}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/>{cat}</c:catAx><c:valAx><c:axId val="2"/>{val}</c:valAx></c:plotArea><c:legend><c:legendPos val="r"/>{legend}</c:legend></c:chart></c:chartSpace>"#
        );
        chart_items(&crate::xml::parse(x.as_bytes()).unwrap(), &theme(), 300.0, 200.0)
            .into_iter()
            .filter_map(|it| if let GraphicItem::Text { text, color, .. } = it { Some((text, color)) } else { None })
            .collect()
    };
    let colour_of = |items: &[(String, Rgb)], t: &str| items.iter().find(|(s, _)| s == t).map(|(_, c)| *c);
    let items = chart(&txpr(red), &txpr(blue), &txpr(r#"<a:schemeClr val="accent1"/>"#), &format!("<a:rPr><a:solidFill>{red}</a:solidFill></a:rPr>"));
    assert_eq!(colour_of(&items, "Q1"), Some(Rgb(0xFF, 0, 0)), "category labels: {items:?}");
    assert_eq!(colour_of(&items, "Title"), Some(Rgb(0xFF, 0, 0)));
    assert_eq!(colour_of(&items, "Units"), Some(theme()[4]), "legend in the theme's accent 1");
    let ticks: Vec<Rgb> = items.iter().filter(|(s, _)| s.parse::<f64>().is_ok()).map(|(_, c)| *c).collect();
    assert!(!ticks.is_empty() && ticks.iter().all(|c| *c == Rgb(0, 0, 0xFF)), "value labels {ticks:?}");
    // None given: the default grey.
    let plain = chart("", "", "", "");
    assert!(plain.iter().all(|(_, c)| *c == TEXT), "{plain:?}");
}

#[test]
fn a_title_is_bold_unless_its_run_says_not() {
    let s = ser(0, "Units", None, &["Q1"], &["1"]);
    let title_bold = |title: &str| -> Option<bool> {
        let x = format!(
            r#"<c:chartSpace {NS}><c:chart><c:title>{title}</c:title><c:plotArea><c:barChart><c:barDir val="col"/>{s}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx></c:plotArea></c:chart></c:chartSpace>"#
        );
        chart_items(&crate::xml::parse(x.as_bytes()).unwrap(), &theme(), 300.0, 200.0)
            .into_iter()
            .find_map(|it| if let GraphicItem::Text { text, bold, .. } = it { (text == "Title").then_some(bold) } else { None })
    };
    let rich = |rpr: &str, defrpr: &str| {
        format!(r#"<c:tx><c:rich><a:bodyPr/><a:p><a:pPr>{defrpr}</a:pPr><a:r>{rpr}<a:t>Title</a:t></a:r></a:p></c:rich></c:tx>"#)
    };
    assert_eq!(title_bold(&rich("", "")), Some(true), "no b: Word's bold");
    assert_eq!(title_bold(&rich(r#"<a:rPr lang="en-US"/>"#, "")), Some(true));
    assert_eq!(title_bold(&rich(r#"<a:rPr b="0"/>"#, "")), Some(false));
    assert_eq!(title_bold(&rich(r#"<a:rPr b="1"/>"#, r#"<a:defRPr b="0"/>"#)), Some(true), "the run wins");
    assert_eq!(title_bold(&rich(r#"<a:rPr/>"#, r#"<a:defRPr b="0"/>"#)), Some(false), "else the default run properties");
    let txpr = r#"<c:txPr><a:bodyPr/><a:p><a:pPr><a:defRPr b="false"/></a:pPr></a:p></c:txPr>"#;
    assert_eq!(title_bold(&format!("{}{txpr}", rich("", ""))), Some(false), "or the title's txPr");
}

#[test]
fn foreign_area_fills_stay_solid_and_gradients_are_drawn() {
    let area = |sp: &str| -> Vec<GraphicItem> {
        let s = ser(0, "A", None, &["a", "b", "c"], &["1", "3", "2"]).replace("<c:cat>", &format!("{sp}<c:cat>"));
        let plot = format!(
            r#"<c:areaChart><c:grouping val="standard"/>{s}<c:axId val="1"/><c:axId val="2"/></c:areaChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#
        );
        chart_items(&space(&plot), &theme(), 300.0, 200.0)
    };
    let filled = |items: &[GraphicItem]| -> Vec<(Option<Rgb>, bool)> {
        items
            .iter()
            .filter_map(|it| if let GraphicItem::Path { fill: Some(c), gradient, .. } = it { Some((Some(*c), gradient.is_some())) } else { None })
            .collect()
    };
    // A solid fill (no line): one solid path, no gradient, no edge.
    let solid = area(r#"<c:spPr><a:solidFill><a:srgbClr val="336699"/></a:solidFill><a:ln><a:noFill/></a:ln></c:spPr>"#);
    assert_eq!(filled(&solid), [(Some(Rgb(0x33, 0x66, 0x99)), false)]);
    assert!(!solid.iter().any(|it| matches!(it, GraphicItem::Path { stroke: Some(c), .. } if *c == Rgb(0x33, 0x66, 0x99))));
    // A gradient at 90 degrees with a line: a gradient path down the plot, its plain fill half
    // way to white, and the line along the top.
    let grad = area(
        r#"<c:spPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="336699"><a:alpha val="80000"/></a:srgbClr></a:gs><a:gs pos="100000"><a:srgbClr val="336699"><a:alpha val="15000"/></a:srgbClr></a:gs></a:gsLst><a:lin ang="5400000" scaled="0"/></a:gradFill><a:ln w="19050"><a:solidFill><a:srgbClr val="336699"/></a:solidFill></a:ln></c:spPr>"#,
    );
    let g =
        grad.iter().find_map(|it| if let GraphicItem::Path { gradient: Some(g), fill, .. } = it { Some((g.clone(), *fill)) } else { None }).unwrap();
    assert_eq!(g.1, Some(Rgb(0x99, 0xB3, 0xCC)));
    assert!(g.0.to[1] > g.0.from[1] && g.0.from[0] == g.0.to[0], "top to bottom: {:?}", g.0);
    assert_eq!(g.0.stops.iter().map(|s| (s.offset, (s.alpha * 100.0).round())).collect::<Vec<_>>(), [(0.0, 80.0), (1.0, 15.0)]);
    let edge = grad.iter().find(|it| matches!(it, GraphicItem::Path { stroke: Some(c), fill: None, .. } if *c == Rgb(0x33, 0x66, 0x99)));
    assert!(matches!(edge, Some(GraphicItem::Path { segs, stroke_width, .. }) if segs.len() == 3 && (*stroke_width - 1.5).abs() < 1e-3), "{edge:?}");
    // A path gradient fills solid in its first colour.
    let path = area(
        r#"<c:spPr><a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="336699"/></a:gs><a:gs pos="100000"><a:srgbClr val="FFFFFF"/></a:gs></a:gsLst><a:path path="circle"/></a:gradFill></c:spPr>"#,
    );
    assert_eq!(filled(&path), [(Some(Rgb(0x33, 0x66, 0x99)), false)]);
}

#[test]
fn mid_cat_puts_the_end_categories_on_the_plot_edges() {
    // The line's points, and the plot's left and right edges (where the gridlines run).
    let line_pts = |val_ax: &str| -> (Vec<f32>, f32, f32) {
        let items = line_chart("", "", &format!("<c:majorGridlines/>{val_ax}"));
        let line = items
            .iter()
            .find_map(|it| match it {
                GraphicItem::Path { segs, stroke: Some(c), .. } if *c == theme()[4] => Some(
                    segs.iter().filter_map(|s| if let PathSeg::Move(x, _) | PathSeg::Line(x, _) = s { Some(*x) } else { None }).collect::<Vec<_>>(),
                ),
                _ => None,
            })
            .unwrap();
        let grid = items.iter().find_map(|it| match it {
            GraphicItem::Path { segs, stroke: Some(c), .. } if *c == GRID => match segs.as_slice() {
                [PathSeg::Move(x0, _), PathSeg::Line(x1, _)] => Some((*x0, *x1)),
                _ => None,
            },
            _ => None,
        });
        let (left, right) = grid.unwrap();
        (line, left, right)
    };
    let (pts, left, right) = line_pts(r#"<c:crossBetween val="midCat"/>"#);
    assert_eq!(pts.len(), 4);
    assert!((pts[0] - left).abs() < 1e-3 && (pts[3] - right).abs() < 1e-3, "{pts:?} {left}..{right}");
    assert!(((pts[1] - pts[0]) - (right - left) / 3.0).abs() < 1e-3, "evenly spread");
    // Between: half a category in from each edge.
    let (pts, left, right) = line_pts(r#"<c:crossBetween val="between"/>"#);
    let band = (right - left) / 4.0;
    assert!((pts[0] - left - band / 2.0).abs() < 1e-3 && (right - pts[3] - band / 2.0).abs() < 1e-3);
    // Category labels sit under their points.
    let items = line_chart("", "", r#"<c:crossBetween val="midCat"/>"#);
    let a = items
        .iter()
        .find_map(|it| if let GraphicItem::Text { text, rect, .. } = it { (text == "a").then_some(rect[0] + rect[2] / 2.0) } else { None })
        .unwrap();
    let (pts, ..) = line_pts(r#"<c:crossBetween val="midCat"/>"#);
    assert!((a - pts[0]).abs() < 1e-3, "label a at {a}, point at {}", pts[0]);
    // Columns don't move: the same drawing with midCat as with between.
    let columns = |cross: &str| {
        let s = ser(0, "A", Some("102030"), &["a", "b", "c"], &["1", "3", "2"]);
        let plot = format!(
            r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{s}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/><c:crossBetween val="{cross}"/></c:valAx>"#
        );
        chart_items(&space(&plot), &theme(), 300.0, 200.0)
    };
    assert_eq!(columns("midCat"), columns("between"));
}

#[test]
fn a_plot_area_fill_is_drawn_in_its_own_colour_and_opacity() {
    let chart = |sp: &str| {
        let s = ser(0, "A", Some("102030"), &["a", "b"], &["1", "2"]);
        let plot = format!(
            r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>{s}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>{sp}"#
        );
        chart_items(&space(&plot), &theme(), 300.0, 200.0)
    };
    // Opaque: a plain fill, before the bars.
    let items = chart(r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"/></a:solidFill></c:spPr>"#);
    let red = items.iter().position(|it| matches!(it, GraphicItem::Path { fill: Some(c), gradient: None, .. } if *c == Rgb(0xFF, 0, 0))).unwrap();
    let bar = items.iter().position(|it| matches!(it, GraphicItem::Path { fill: Some(c), .. } if *c == Rgb(0x10, 0x20, 0x30))).unwrap();
    assert!(red < bar);
    // Translucent: its opacity, carried as an even gradient.
    let items = chart(r#"<c:spPr><a:solidFill><a:srgbClr val="FF0000"><a:alpha val="50000"/></a:srgbClr></a:solidFill></c:spPr>"#);
    let g = items.iter().find_map(|it| if let GraphicItem::Path { gradient: Some(g), .. } = it { Some(g.clone()) } else { None }).unwrap();
    assert!(g.stops.iter().all(|s| s.color == Rgb(0xFF, 0, 0) && (s.alpha - 0.5).abs() < 1e-4), "{g:?}");
    // No fill, or noFill: nothing behind the plot.
    for sp in ["", r#"<c:spPr><a:noFill/></c:spPr>"#] {
        let items = chart(sp);
        assert!(!items.iter().any(|it| matches!(it, GraphicItem::Path { gradient: Some(_), .. })));
        assert!(!items.iter().any(|it| matches!(it, GraphicItem::Path { fill: Some(c), .. } if *c != Rgb(0x10, 0x20, 0x30))), "{sp}");
    }
}

/// A chart of type element `ty` (with `extra` markup after the series), data labels on, drawn
/// `w` × `h`; its label texts in `vals` with their boxes and colours.
fn labelled(ty: &str, extra: &str, sers: &[(&str, &[&str])], vals: &[&str], (w, h): (f32, f32)) -> (Vec<GraphicItem>, Vec<(String, Rect, Rgb)>) {
    let cats = ["a", "b", "c", "d"];
    let body: String = sers
        .iter()
        .enumerate()
        .map(|(j, (fill, v))| {
            let fill = (!fill.is_empty()).then_some(*fill);
            ser(j, &format!("S{j}"), fill, &cats[..v.len()], v)
        })
        .collect();
    let axes = if ty.starts_with("<c:pie") { "" } else { r#"<c:axId val="1"/><c:axId val="2"/>"# };
    let close = ty.trim_start_matches('<').split([' ', '>']).next().unwrap_or_default().to_string();
    let plot = format!(
        r#"{ty}{body}<c:dLbls>{extra}<c:showVal val="1"/></c:dLbls>{axes}</{close}>{}"#,
        if axes.is_empty() { "" } else { r#"<c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"# }
    );
    let items = chart_items(&space(&plot), &theme(), w, h);
    let labels = items
        .iter()
        .filter_map(|it| match it {
            // Data labels, not the (grey) axis labels.
            GraphicItem::Text { text, rect, color, .. } if vals.contains(&text.as_str()) && *color != TEXT => {
                Some((text.clone(), Rect { x: rect[0], y: rect[1], w: rect[2], h: rect[3] }, *color))
            }
            _ => None,
        })
        .collect();
    (items, labels)
}

#[test]
fn area_labels_sit_above_their_points_apart_and_inside() {
    let vals = ["130", "60", "50", "25", "150", "55", "65", "15"];
    let (w, h) = (360.0, 220.0);
    for grouping in ["standard", "stacked"] {
        let (_, labels) = labelled(
            &format!(r#"<c:areaChart><c:grouping val="{grouping}"/>"#),
            "",
            &[("", &["130", "60", "50", "25"]), ("", &["150", "55", "65", "15"])],
            &vals,
            (w, h),
        );
        assert_eq!(labels.len(), 8, "{grouping}: {labels:?}");
        for (k, (t, a, c)) in labels.iter().enumerate() {
            assert_eq!(*c, Rgb(0x1F, 0x1F, 0x1F), "{t}: dark text");
            assert!(a.x >= 0.0 && a.y >= 0.0 && a.right() <= w && a.bottom() <= h, "{t} inside: {a:?}");
            for (u, b, _) in labels.iter().skip(k + 1) {
                assert!(!labels::overlaps(*a, *b), "{grouping}: {t} {a:?} overlaps {u} {b:?}");
            }
        }
    }
}

#[test]
fn line_labels_go_above_and_the_lower_one_below_when_they_would_meet() {
    let (_, labels) = labelled(
        r#"<c:lineChart><c:grouping val="standard"/>"#,
        "",
        &[("", &["60", "10"]), ("", &["55", "90"])],
        &["60", "10", "55", "90"],
        (300.0, 200.0),
    );
    let at = |t: &str| labels.iter().find(|l| l.0 == t).map(|l| l.1).unwrap();
    // 60 and 55 are close: 60 keeps the place above, 55 goes below its point.
    assert!(!labels::overlaps(at("60"), at("55")));
    assert!(at("60").bottom() <= at("55").y);
}

#[test]
fn column_labels_read_on_their_fill_and_move_out_of_short_bars() {
    let (w, h) = (300.0, 200.0);
    let col = r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>"#;
    // A light fill: dark text inside the end; a dark fill: white text.
    let (_, light) = labelled(col, "", &[("FFEE88", &["100", "90"])], &["100", "90"], (w, h));
    assert!(light.iter().all(|l| l.2 == Rgb(0x1F, 0x1F, 0x1F)), "{light:?}");
    let (items, dark) = labelled(col, "", &[("102030", &["100", "90"])], &["100", "90"], (w, h));
    assert!(dark.iter().all(|l| l.2 == Rgb::WHITE), "{dark:?}");
    // Inside the bar's end: below its top.
    let bar_top = items
        .iter()
        .filter_map(|it| {
            if let GraphicItem::Path { segs, fill: Some(c), .. } = it {
                (*c == Rgb(0x10, 0x20, 0x30)).then(|| {
                    segs.iter()
                        .filter_map(|s| if let PathSeg::Move(_, y) | PathSeg::Line(_, y) = s { Some(*y) } else { None })
                        .fold(f32::INFINITY, f32::min)
                })
            } else {
                None
            }
        })
        .fold(f32::INFINITY, f32::min);
    let hundred = dark.iter().find(|l| l.0 == "100").unwrap().1;
    assert!(hundred.y > bar_top, "inside: {hundred:?} under {bar_top}");
    // A short bar: its label goes above it, dark.
    let (items, short) = labelled(col, "", &[("102030", &["100", "1"])], &["100", "1"], (w, h));
    let one = short.iter().find(|l| l.0 == "1").unwrap();
    let short_top = items
        .iter()
        .filter_map(|it| {
            if let GraphicItem::Path { segs, fill: Some(c), .. } = it {
                (*c == Rgb(0x10, 0x20, 0x30)).then(|| {
                    segs.iter()
                        .filter_map(|s| if let PathSeg::Move(_, y) | PathSeg::Line(_, y) = s { Some(*y) } else { None })
                        .fold(f32::INFINITY, f32::min)
                })
            } else {
                None
            }
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(one.1.bottom() <= short_top + 0.01, "outside the end: {:?} above {short_top}", one.1);
    assert_eq!(one.2, Rgb(0x1F, 0x1F, 0x1F));
    // A file's outEnd puts every label outside.
    let (_, out) = labelled(col, r#"<c:dLblPos val="outEnd"/>"#, &[("102030", &["100", "90"])], &["100", "90"], (w, h));
    assert!(out.iter().all(|l| l.2 == Rgb(0x1F, 0x1F, 0x1F)), "outside the bars, dark: {out:?}");
}

#[test]
fn stacked_segments_too_small_for_their_label_show_none() {
    let (_, labels) = labelled(
        r#"<c:barChart><c:barDir val="col"/><c:grouping val="stacked"/><c:overlap val="100"/>"#,
        "",
        &[("102030", &["100"]), ("FFEE88", &["0.5"])],
        &["100", "0.5"],
        (300.0, 200.0),
    );
    assert_eq!(labels.iter().map(|l| l.0.as_str()).collect::<Vec<_>>(), ["100"], "the tiny segment's label is left out");
    assert_eq!(labels[0].2, Rgb::WHITE, "centred on the dark segment");
}

#[test]
fn thin_pie_slices_get_their_label_outside_with_a_leader() {
    let (w, h) = (300.0, 200.0);
    let (items, labels) = labelled(r#"<c:pieChart><c:varyColors val="1"/>"#, "", &[("", &["100", "1"])], &["100", "1"], (w, h));
    let one = labels.iter().find(|l| l.0 == "1").unwrap();
    let big = labels.iter().find(|l| l.0 == "100").unwrap();
    // The pie: centred in the chart; the big slice's label inside it, the thin one outside.
    let (cx, cy) = (w / 2.0, h / 2.0);
    let dist = |r: &Rect| (r.x + r.w / 2.0 - cx).hypot(r.y + r.h / 2.0 - cy);
    let radius = h / 2.0 - 6.0 - 2.0;
    assert!(dist(&big.1) < radius && dist(&one.1) > radius * 0.9, "{big:?} {one:?}");
    assert_eq!(one.2, Rgb(0x1F, 0x1F, 0x1F));
    let leaders = items.iter().filter(|it| matches!(it, GraphicItem::Path { stroke: Some(c), segs, .. } if *c == AXIS && segs.len() == 2)).count();
    assert_eq!(leaders, 1, "one leader line");
    assert!(one.1.x >= 0.0 && one.1.right() <= w && one.1.y >= 0.0 && one.1.bottom() <= h);
}

#[test]
fn data_labels_stay_between_the_axes_and_clear_of_the_axis_labels() {
    let (w, h) = (360.0, 220.0);
    let vals = ["130", "60", "50", "25", "150", "55", "65", "15"];
    for ty in [
        r#"<c:areaChart><c:grouping val="stacked"/>"#,
        r#"<c:areaChart><c:grouping val="standard"/>"#,
        r#"<c:lineChart><c:grouping val="standard"/>"#,
    ] {
        // Categories on the tick marks (the first point on the value axis), as our area charts write.
        let cross = r#"<c:crossBetween val="midCat"/>"#;
        let s = |j: usize, v: &[&str]| ser(j, &format!("S{j}"), None, &["a", "b", "c", "d"], v);
        let close = ty.trim_start_matches('<').split([' ', '>']).next().unwrap_or_default().to_string();
        let plot = format!(
            r#"{ty}{}{}<c:dLbls><c:showVal val="1"/></c:dLbls><c:axId val="1"/><c:axId val="2"/></{close}><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/>{cross}</c:valAx>"#,
            s(0, &["130", "60", "50", "25"]),
            s(1, &["150", "55", "65", "15"])
        );
        let sp = space(&plot);
        let items = chart_items(&sp, &theme(), w, h);
        let p = chart_plot_rect(&sp, &theme(), w, h).unwrap();
        let rect = |r: &[f32; 4]| Rect { x: r[0], y: r[1], w: r[2], h: r[3] };
        let data: Vec<(&str, Rect)> = items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Text { text, rect: r, color, .. } if *color != TEXT && vals.contains(&text.as_str()) => Some((text.as_str(), rect(r))),
                _ => None,
            })
            .collect();
        let axis: Vec<Rect> = items
            .iter()
            .filter_map(|it| match it {
                GraphicItem::Text { rect: r, color, .. } if *color == TEXT => Some(rect(r)),
                _ => None,
            })
            .collect();
        assert_eq!(data.len(), 8, "{ty}: {data:?}");
        for (t, r) in &data {
            assert!(r.x >= p[0] && r.right() <= p[0] + p[2], "{ty}: {t} {r:?} between the axes {p:?}");
            assert!(r.bottom() <= p[1] + p[3] + 0.01, "{ty}: {t} above the category axis");
            for a in &axis {
                assert!(!labels::overlaps(*r, *a), "{ty}: {t} {r:?} covers an axis label {a:?}");
            }
        }
        // The first point's labels start a little right of the value axis.
        let first = data.iter().filter(|(t, _)| *t == "130" || *t == "150").map(|(_, r)| r.x).fold(f32::INFINITY, f32::min);
        assert!((first - p[0] - labels::PAD).abs() < 0.01, "{ty}: {first} vs axis {}", p[0]);
    }
    // Horizontal bars: an end label never crosses the plot's right edge.
    let (_, bars) = labelled(
        r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/>"#,
        r#"<c:dLblPos val="outEnd"/>"#,
        &[("102030", &["100", "99"])],
        &["100", "99"],
        (w, h),
    );
    let s = space(&format!(
        r#"<c:barChart><c:barDir val="bar"/><c:grouping val="clustered"/>{}<c:axId val="1"/><c:axId val="2"/></c:barChart><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/></c:valAx>"#,
        ser(0, "S0", Some("102030"), &["a", "b"], &["100", "99"])
    ));
    let p = chart_plot_rect(&s, &theme(), w, h).unwrap();
    assert_eq!(bars.len(), 2);
    assert!(bars.iter().all(|(_, r, _)| r.right() <= p[0] + p[2] + 0.01), "{bars:?} within {p:?}");
}

#[test]
fn category_labels_clear_the_lowest_value_label() {
    let (w, h) = (360.0, 220.0);
    let chart = |ty: &str, cross: &str| {
        let close = ty.trim_start_matches('<').split([' ', '>']).next().unwrap_or_default().to_string();
        let s = |j: usize, v: &[&str]| ser(j, &format!("S{j}"), None, &["탄수화물", "b", "c", "d"], v);
        let plot = format!(
            r#"{ty}{}{}<c:axId val="1"/><c:axId val="2"/></{close}><c:catAx><c:axId val="1"/></c:catAx><c:valAx><c:axId val="2"/><c:crossBetween val="{cross}"/></c:valAx>"#,
            s(0, &["130", "60", "50", "25"]),
            s(1, &["150", "55", "65", "15"])
        );
        let sp = space(&plot);
        let items = chart_items(&sp, &theme(), w, h);
        let find = |t: &str| {
            items
                .iter()
                .find_map(|it| {
                    if let GraphicItem::Text { text, rect, .. } = it {
                        (text == t).then(|| Rect { x: rect[0], y: rect[1], w: rect[2], h: rect[3] })
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        let p = chart_plot_rect(&sp, &theme(), w, h).unwrap();
        (find("탄수화물"), find("0"), p)
    };
    for ty in [r#"<c:areaChart><c:grouping val="standard"/>"#, r#"<c:areaChart><c:grouping val="stacked"/>"#] {
        let (cat, zero, p) = chart(ty, "midCat");
        assert!(!labels::overlaps(cat, zero), "{ty}: {cat:?} {zero:?}");
        assert!(cat.y - zero.bottom() >= 3.0 - 1e-3, "{ty}: gap {}", cat.y - zero.bottom());
        assert!(cat.y - (p[1] + p[3]) >= 6.0, "below the axis line");
        assert!(cat.bottom() <= h, "inside the chart");
    }
    // Columns: the labels sit lower by the new minimum, still inside the chart.
    let (cat, zero, _) = chart(r#"<c:barChart><c:barDir val="col"/><c:grouping val="clustered"/>"#, "between");
    assert!(cat.y >= zero.bottom() && cat.bottom() <= h);
}
