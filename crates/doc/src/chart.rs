//! Charts WordCraft makes and edits (Insert › Chart): the chart type, its data (categories and
//! series), title, legend and data labels.
//!
//! A chart object ([`crate::graphic::Graphic`] with `chart` set) keeps this model next to the
//! items it is drawn with; the DOCX writer turns it into a chart part (`c:chartSpace`) with the
//! data cached in the part itself, and the reader turns a chart part back into it when the part
//! holds nothing this model leaves out. Charts from other programs stay as they were read.
//!
//! Every value from outside (commands, files) goes through [`ChartSpec::sanitize`]: series,
//! points and text lengths are capped and values that aren't finite numbers become gaps.

use serde::{Deserialize, Serialize};

/// Most series in a chart.
pub const MAX_SERIES: usize = 255;
/// Most categories (points per series) in a chart.
pub const MAX_POINTS: usize = 4000;
/// Most cells (series × categories) in a chart: what the chart drawing reads at most.
pub const MAX_CELLS: usize = 100_000;
/// Longest title, series name or category label, characters.
pub const MAX_TEXT: usize = 255;
/// Most colours in a chart's palette.
pub const MAX_COLORS: usize = 64;
/// Largest axis bound or major unit kept (larger ones are clamped to it).
pub const MAX_AXIS: f64 = 1e15;

/// What a chart shows its data as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChartType {
    /// Clustered columns.
    #[default]
    Column,
    /// Stacked columns.
    ColumnStacked,
    /// Clustered horizontal bars.
    Bar,
    /// Stacked horizontal bars.
    BarStacked,
    Line,
    Area,
    /// Stacked areas: each series on top of the ones before it.
    AreaStacked,
    Pie,
    Doughnut,
    /// XY scatter with markers: the categories are the x values.
    Scatter,
    /// 100% stacked columns, bars, areas and lines: each category's values as shares of its total.
    ColumnPercent,
    BarPercent,
    AreaPercent,
    LinePercent,
    /// Stacked lines: each series at the running total of the ones before it.
    LineStacked,
    /// XY scatter joined by straight lines, or by a smooth curve, in x order (`markers` adds dots).
    ScatterLines,
    ScatterSmooth,
    /// Radar: a spoke per category, a ring per value tick; lines, lines with markers, or filled.
    Radar,
    RadarMarkers,
    RadarFilled,
    /// Bubbles at (x, y) sized by area by each series' `sizes`.
    Bubble,
    /// Waterfall: the first series' values as steps up and down from a running total (the last
    /// one a total bar with `waterfall_total`).
    Waterfall,
    /// Funnel: the first series' values as centred bars, the first category on top.
    Funnel,
    /// Columns, lines and stacked areas together: each series' own `kind`, on the secondary value
    /// axis with `secondary`.
    Combo,
    /// A pie whose last `split_pos` slices are pulled out into a second pie, or a stacked bar.
    PieOfPie,
    BarOfPie,
}

impl ChartType {
    pub const ALL: [ChartType; 26] = [
        ChartType::Column,
        ChartType::ColumnStacked,
        ChartType::Bar,
        ChartType::BarStacked,
        ChartType::Line,
        ChartType::Area,
        ChartType::AreaStacked,
        ChartType::Pie,
        ChartType::Doughnut,
        ChartType::Scatter,
        ChartType::ColumnPercent,
        ChartType::BarPercent,
        ChartType::AreaPercent,
        ChartType::LinePercent,
        ChartType::LineStacked,
        ChartType::ScatterLines,
        ChartType::ScatterSmooth,
        ChartType::Radar,
        ChartType::RadarMarkers,
        ChartType::RadarFilled,
        ChartType::Bubble,
        ChartType::Waterfall,
        ChartType::Funnel,
        ChartType::Combo,
        ChartType::PieOfPie,
        ChartType::BarOfPie,
    ];

    /// The id commands use (`column`, `barStacked`…).
    pub fn id(self) -> &'static str {
        match self {
            ChartType::Column => "column",
            ChartType::ColumnStacked => "columnStacked",
            ChartType::Bar => "bar",
            ChartType::BarStacked => "barStacked",
            ChartType::Line => "line",
            ChartType::Area => "area",
            ChartType::AreaStacked => "areaStacked",
            ChartType::Pie => "pie",
            ChartType::Doughnut => "doughnut",
            ChartType::Scatter => "scatter",
            ChartType::ColumnPercent => "columnPercent",
            ChartType::BarPercent => "barPercent",
            ChartType::AreaPercent => "areaPercent",
            ChartType::LinePercent => "linePercent",
            ChartType::LineStacked => "lineStacked",
            ChartType::ScatterLines => "scatterLines",
            ChartType::ScatterSmooth => "scatterSmooth",
            ChartType::Radar => "radar",
            ChartType::RadarMarkers => "radarMarkers",
            ChartType::RadarFilled => "radarFilled",
            ChartType::Bubble => "bubble",
            ChartType::Waterfall => "waterfall",
            ChartType::Funnel => "funnel",
            ChartType::Combo => "combo",
            ChartType::PieOfPie => "pieOfPie",
            ChartType::BarOfPie => "barOfPie",
        }
    }

    /// The type an id names (any ASCII case).
    pub fn from_id(id: &str) -> Option<ChartType> {
        ChartType::ALL.into_iter().find(|t| t.id().eq_ignore_ascii_case(id.trim()))
    }

    /// The name shown for it.
    pub fn label(self) -> &'static str {
        match self {
            ChartType::Column => "Clustered Column",
            ChartType::ColumnStacked => "Stacked Column",
            ChartType::Bar => "Clustered Bar",
            ChartType::BarStacked => "Stacked Bar",
            ChartType::Line => "Line",
            ChartType::Area => "Area",
            ChartType::AreaStacked => "Stacked Area",
            ChartType::Pie => "Pie",
            ChartType::Doughnut => "Doughnut",
            ChartType::Scatter => "Scatter",
            ChartType::ColumnPercent => "100% Stacked Column",
            ChartType::BarPercent => "100% Stacked Bar",
            ChartType::AreaPercent => "100% Stacked Area",
            ChartType::LinePercent => "100% Stacked Line",
            ChartType::LineStacked => "Stacked Line",
            ChartType::ScatterLines => "Scatter with Straight Lines",
            ChartType::ScatterSmooth => "Scatter with Smooth Lines",
            ChartType::Radar => "Radar",
            ChartType::RadarMarkers => "Radar with Markers",
            ChartType::RadarFilled => "Filled Radar",
            ChartType::Bubble => "Bubble",
            ChartType::Waterfall => "Waterfall",
            ChartType::Funnel => "Funnel",
            ChartType::Combo => "Combo",
            ChartType::PieOfPie => "Pie of Pie",
            ChartType::BarOfPie => "Bar of Pie",
        }
    }

    /// Charts that show one series, a slice per category (no axes): pie, doughnut, pie of pie
    /// and bar of pie.
    pub fn is_round(self) -> bool {
        matches!(self, ChartType::Pie | ChartType::Doughnut | ChartType::PieOfPie | ChartType::BarOfPie)
    }

    /// Area charts, clustered, stacked or 100% stacked.
    pub fn is_area(self) -> bool {
        matches!(self, ChartType::Area | ChartType::AreaStacked | ChartType::AreaPercent)
    }

    /// Line charts, standard, stacked or 100% stacked.
    pub fn is_line(self) -> bool {
        matches!(self, ChartType::Line | ChartType::LineStacked | ChartType::LinePercent)
    }

    /// Column and bar charts, clustered, stacked or 100% stacked.
    pub fn is_bar(self) -> bool {
        matches!(
            self,
            ChartType::Column | ChartType::ColumnStacked | ChartType::ColumnPercent | ChartType::Bar | ChartType::BarStacked | ChartType::BarPercent
        )
    }

    /// 100% stacked: values drawn as shares of their category's total.
    pub fn is_percent(self) -> bool {
        matches!(self, ChartType::ColumnPercent | ChartType::BarPercent | ChartType::AreaPercent | ChartType::LinePercent)
    }

    /// XY charts: the categories are x values (scatter, scatter with lines, bubble).
    pub fn is_xy(self) -> bool {
        matches!(self, ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth | ChartType::Bubble)
    }

    /// Radar charts.
    pub fn is_radar(self) -> bool {
        matches!(self, ChartType::Radar | ChartType::RadarMarkers | ChartType::RadarFilled)
    }

    /// Pie of pie and bar of pie.
    pub fn is_of_pie(self) -> bool {
        matches!(self, ChartType::PieOfPie | ChartType::BarOfPie)
    }
}

/// Where a chart's legend sits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LegendPos {
    Right,
    Left,
    Top,
    #[default]
    Bottom,
}

impl LegendPos {
    pub const ALL: [LegendPos; 4] = [LegendPos::Right, LegendPos::Top, LegendPos::Left, LegendPos::Bottom];

    pub fn id(self) -> &'static str {
        match self {
            LegendPos::Right => "right",
            LegendPos::Left => "left",
            LegendPos::Top => "top",
            LegendPos::Bottom => "bottom",
        }
    }

    pub fn from_id(id: &str) -> Option<LegendPos> {
        LegendPos::ALL.into_iter().find(|p| p.id().eq_ignore_ascii_case(id.trim()))
    }
}

/// The shape of a chart's legend keys.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LegendShape {
    #[default]
    Square,
    Circle,
}

impl LegendShape {
    pub const ALL: [LegendShape; 2] = [LegendShape::Square, LegendShape::Circle];

    pub fn id(self) -> &'static str {
        match self {
            LegendShape::Square => "square",
            LegendShape::Circle => "circle",
        }
    }

    pub fn from_id(id: &str) -> Option<LegendShape> {
        LegendShape::ALL.into_iter().find(|s| s.id().eq_ignore_ascii_case(id.trim()))
    }

    fn is_square(&self) -> bool {
        *self == LegendShape::Square
    }
}

/// How the value axis's major gridlines are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gridlines {
    #[default]
    Solid,
    Dashed,
    None,
}

impl Gridlines {
    pub const ALL: [Gridlines; 3] = [Gridlines::Solid, Gridlines::Dashed, Gridlines::None];

    pub fn id(self) -> &'static str {
        match self {
            Gridlines::Solid => "solid",
            Gridlines::Dashed => "dashed",
            Gridlines::None => "none",
        }
    }

    pub fn from_id(id: &str) -> Option<Gridlines> {
        Gridlines::ALL.into_iter().find(|g| g.id().eq_ignore_ascii_case(id.trim()))
    }

    fn is_solid(&self) -> bool {
        *self == Gridlines::Solid
    }
}

/// One series: its name and a value per category (`None` = no value, a gap).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSeries {
    pub name: String,
    #[serde(default)]
    pub values: Vec<Option<f64>>,
    /// Bubble charts: each point's bubble size (its area), as long as the values; empty
    /// elsewhere.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sizes: Vec<Option<f64>>,
    /// Combo charts: drawn as columns (`None`, the default), a line or an area (areas stacked).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<SeriesKind>,
    /// Combo charts: on the secondary value axis, at the right.
    #[serde(default, skip_serializing_if = "is_false")]
    pub secondary: bool,
}

impl ChartSeries {
    /// A series of `values`, nothing else set.
    pub fn of(name: &str, values: Vec<Option<f64>>) -> ChartSeries {
        ChartSeries { name: name.to_string(), values, ..Default::default() }
    }
}

/// What a combo chart's series is drawn as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SeriesKind {
    #[default]
    Column,
    Line,
    /// An area; a combo chart's areas are stacked on each other.
    Area,
}

impl SeriesKind {
    /// The kind an id names (`column`, `line`).
    pub fn from_id(id: &str) -> Option<SeriesKind> {
        match id.trim() {
            s if s.eq_ignore_ascii_case("column") => Some(SeriesKind::Column),
            s if s.eq_ignore_ascii_case("line") => Some(SeriesKind::Line),
            s if s.eq_ignore_ascii_case("area") => Some(SeriesKind::Area),
            _ => None,
        }
    }
}

/// A chart WordCraft can edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChartSpec {
    #[serde(rename = "type")]
    pub kind: ChartType,
    /// Category labels (scatter charts: the x values).
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub series: Vec<ChartSeries>,
    #[serde(default)]
    pub title: Option<String>,
    /// `None`: no legend.
    #[serde(default)]
    pub legend: Option<LegendPos>,
    /// The shape of the legend keys (WordCraft's drawing only: the file format has none).
    #[serde(default, skip_serializing_if = "LegendShape::is_square")]
    pub legend_shape: LegendShape,
    /// Show each point's value next to it.
    #[serde(default)]
    pub data_labels: bool,
    /// Series colours (pie and doughnut: slice colours) as `rrggbb`, used in turn; empty: the
    /// theme's accents.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<String>,
    /// Line charts: smoothed curves through the points.
    #[serde(default, skip_serializing_if = "is_false")]
    pub smooth: bool,
    /// Line charts: a circle on every point.
    #[serde(default, skip_serializing_if = "is_false")]
    pub markers: bool,
    /// Column and bar charts: bars with rounded corners.
    #[serde(default, skip_serializing_if = "is_false")]
    pub rounded_bars: bool,
    /// Clustered column and bar charts: the gap between neighbouring series in a category, percent
    /// of a bar's thickness (0–100, whole numbers).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub series_gap: f64,
    /// The value axis's major gridlines.
    #[serde(default, skip_serializing_if = "Gridlines::is_solid")]
    pub gridlines: Gridlines,
    /// Lowest value on the value axis; `None`: automatic.
    #[serde(default)]
    pub axis_min: Option<f64>,
    /// Highest value on the value axis; `None`: automatic.
    #[serde(default)]
    pub axis_max: Option<f64>,
    /// Distance between the value axis's major ticks (and gridlines); `None`: automatic.
    #[serde(default)]
    pub axis_step: Option<f64>,
    /// The title is shown; when not, its text is kept (to show it again) but not drawn.
    #[serde(default = "yes")]
    pub title_visible: bool,
    /// The chart area has a background (the theme's light colour); else it's transparent.
    #[serde(default)]
    pub background: bool,
    /// The category axis's labels (scatter charts: the x axis's).
    #[serde(default = "yes")]
    pub cat_labels: bool,
    /// The category axis's line (scatter charts: the x axis's).
    #[serde(default = "yes")]
    pub cat_axis: bool,
    /// The value axis's tick labels (scatter charts: the y axis's).
    #[serde(default = "yes")]
    pub val_labels: bool,
    /// The value axis's line and tick marks (scatter charts: the y axis's).
    #[serde(default = "yes")]
    pub val_axis: bool,
    /// Text of the legend, the data labels, the category and value axis labels and the title.
    #[serde(default)]
    pub legend_style: TextStyle,
    #[serde(default)]
    pub data_labels_style: TextStyle,
    #[serde(default)]
    pub cat_labels_style: TextStyle,
    #[serde(default)]
    pub val_labels_style: TextStyle,
    #[serde(default)]
    pub title_style: TextStyle,
    /// A white outline (halo) around the data labels' text.
    #[serde(default)]
    pub data_labels_outline: bool,
    /// How the category labels are set: level, or turned 45° up.
    #[serde(default)]
    pub cat_label_pos: CatLabelPos,
    /// Colours of the axis lines (and tick marks), `rrggbb`; `None`: near black.
    #[serde(default)]
    pub cat_axis_color: Option<String>,
    #[serde(default)]
    pub val_axis_color: Option<String>,
    /// Tick marks on the category axis, and on the value axis.
    #[serde(default)]
    pub cat_ticks: bool,
    #[serde(default = "yes")]
    pub val_ticks: bool,
    /// The plot area background's colour (with `background`), `rrggbb`; `None`: a faint grey.
    #[serde(default)]
    pub background_color: Option<String>,
    /// Width of line series and of area charts' top edges, points; `None`: the default.
    #[serde(default)]
    pub line_width: Option<f32>,
    /// Opacity of an area's fill at its top (it fades downwards in proportion), 0..1.
    #[serde(default = "default_area_opacity")]
    pub area_opacity: f32,
    /// Opacity of the whole chart, 0..1.
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    /// Bubble charts: the bubbles' fill opacity, 0..1 (their outline stays solid), so bubbles
    /// that overlap show through each other.
    #[serde(default = "default_bubble_opacity")]
    pub bubble_opacity: f32,
    /// Doughnut charts: the hole's diameter, percent of the doughnut's (10–90).
    #[serde(default = "default_hole_size")]
    pub hole_size: u32,
    /// Pie and doughnut charts: a gap between the slices, points (0–10): a white outline.
    #[serde(default)]
    pub slice_gap: f32,
    /// Waterfall charts: the last point is a total bar (from zero to its value), not a step.
    #[serde(default, skip_serializing_if = "is_false")]
    pub waterfall_total: bool,
    /// Pie of pie and bar of pie: how many of the last points go to the second plot (1 up to
    /// one less than the points).
    #[serde(default = "default_split", skip_serializing_if = "is_default_split")]
    pub split_pos: u32,
    /// Radar charts: each series' shape filled with its colour fading in from the centre (8% to
    /// 35% opacity at the edge), over its outline.
    #[serde(default, skip_serializing_if = "is_false")]
    pub radar_gradient: bool,
}

/// The points a pie of pie's second plot takes by default.
pub const DEFAULT_SPLIT: u32 = 2;

fn default_split() -> u32 {
    DEFAULT_SPLIT
}

fn is_default_split(n: &u32) -> bool {
    *n == DEFAULT_SPLIT
}

/// The look of a chart text: font, size (points), colour (`rrggbb`) and weight; `None`: the
/// chart's default.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bold: Option<bool>,
}

/// Smallest and largest chart text size, points.
pub const TEXT_SIZES: (f32, f32) = (4.0, 96.0);
/// Thinnest and widest line width, points.
pub const LINE_WIDTHS: (f32, f32) = (0.75, 15.0);
/// Longest font name, characters.
pub const MAX_FONT_NAME: usize = 64;

impl TextStyle {
    /// Whether it sets nothing (the chart's defaults).
    pub fn is_default(&self) -> bool {
        *self == TextStyle::default()
    }

    /// Made safe: a font name cleaned and cut, a size within [`TEXT_SIZES`] (one not a number is
    /// none), a colour of six hex digits (another is none), bold only when it is (chart text is
    /// regular by default, so not bold is none).
    pub fn sanitize(&mut self) {
        self.font = self.font.as_deref().map(|f| clean_text(f).chars().take(MAX_FONT_NAME).collect::<String>()).filter(|f| !f.is_empty());
        self.size = self.size.filter(|s| s.is_finite()).map(|s| (s.clamp(TEXT_SIZES.0, TEXT_SIZES.1) * 10.0).round() / 10.0);
        self.color = self.color.as_deref().and_then(hex_color);
        self.bold = self.bold.filter(|b| *b);
    }
}

/// How category labels are set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CatLabelPos {
    /// Level.
    #[default]
    Auto,
    /// Turned 45° up (reading upwards to the right).
    Rotate,
}

impl CatLabelPos {
    pub fn from_id(id: &str) -> Option<CatLabelPos> {
        match id.trim() {
            s if s.eq_ignore_ascii_case("auto") => Some(CatLabelPos::Auto),
            s if s.eq_ignore_ascii_case("rotate") => Some(CatLabelPos::Rotate),
            _ => None,
        }
    }
}

/// An area's default fill opacity at its top.
pub const DEFAULT_AREA_OPACITY: f32 = 0.8;

fn default_area_opacity() -> f32 {
    DEFAULT_AREA_OPACITY
}

fn default_opacity() -> f32 {
    1.0
}

/// A doughnut's default hole, percent of its diameter.
pub const DEFAULT_HOLE_SIZE: u32 = 50;

fn default_hole_size() -> u32 {
    DEFAULT_HOLE_SIZE
}

/// The widest gap between pie slices, points.
pub const MAX_SLICE_GAP: f32 = 10.0;

/// A bubble's default fill opacity: overlapping bubbles show through.
pub const DEFAULT_BUBBLE_OPACITY: f32 = 0.6;

fn default_bubble_opacity() -> f32 {
    DEFAULT_BUBBLE_OPACITY
}

/// An opacity as kept: within 0..1 to three decimals; one not a number is `default`.
pub fn opacity_of(v: f32, default: f32) -> f32 {
    if v.is_finite() { (v.clamp(0.0, 1.0) * 1000.0).round() / 1000.0 } else { default }
}

impl Default for ChartSpec {
    fn default() -> ChartSpec {
        ChartSpec {
            kind: ChartType::default(),
            categories: Vec::new(),
            series: Vec::new(),
            title: None,
            legend: None,
            legend_shape: LegendShape::default(),
            data_labels: false,
            colors: Vec::new(),
            smooth: false,
            markers: false,
            rounded_bars: false,
            series_gap: 0.0,
            gridlines: Gridlines::default(),
            axis_min: None,
            axis_max: None,
            axis_step: None,
            title_visible: true,
            background: false,
            cat_labels: true,
            cat_axis: true,
            val_labels: true,
            val_axis: true,
            legend_style: TextStyle::default(),
            data_labels_style: TextStyle::default(),
            cat_labels_style: TextStyle::default(),
            val_labels_style: TextStyle::default(),
            title_style: TextStyle::default(),
            data_labels_outline: false,
            cat_label_pos: CatLabelPos::Auto,
            cat_axis_color: None,
            val_axis_color: None,
            cat_ticks: false,
            val_ticks: true,
            background_color: None,
            line_width: None,
            area_opacity: DEFAULT_AREA_OPACITY,
            opacity: 1.0,
            bubble_opacity: DEFAULT_BUBBLE_OPACITY,
            hole_size: DEFAULT_HOLE_SIZE,
            slice_gap: 0.0,
            waterfall_total: false,
            split_pos: DEFAULT_SPLIT,
            radar_gradient: false,
        }
    }
}

fn yes() -> bool {
    true
}

fn is_false(b: &bool) -> bool {
    !*b
}

fn is_zero(v: &f64) -> bool {
    *v == 0.0
}

/// A series gap as kept: a whole percent from 0 to 100 (not a number: 0).
pub fn series_gap(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 100.0).round() } else { 0.0 }
}

/// `s` as a palette colour: six hex digits (a leading `#` is dropped), lower case.
pub fn hex_color(s: &str) -> Option<String> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    (s.len() == 6 && s.bytes().all(|b| b.is_ascii_hexdigit())).then(|| s.to_ascii_lowercase())
}

/// An axis number as kept: finite, within ±[`MAX_AXIS`].
pub fn axis_number(v: f64) -> Option<f64> {
    v.is_finite().then(|| v.clamp(-MAX_AXIS, MAX_AXIS))
}

impl ChartSpec {
    /// A new chart of type `kind` with WordCraft's sample data: four quarters and three series
    /// (one for pie and doughnut charts, x values for scatter charts), legend at the bottom.
    pub fn sample(kind: ChartType) -> ChartSpec {
        let some = |v: &[f64]| v.iter().copied().map(Some).collect::<Vec<_>>();
        let (categories, series): (Vec<&str>, Vec<ChartSeries>) = match kind {
            ChartType::Pie | ChartType::Doughnut => {
                (vec!["North", "East", "South", "West"], vec![ChartSeries::of("Share", some(&[38.0, 24.0, 21.0, 17.0]))])
            }
            ChartType::PieOfPie | ChartType::BarOfPie => (
                vec!["North", "East", "South", "West", "Islands", "Abroad"],
                vec![ChartSeries::of("Share", some(&[34.0, 24.0, 21.0, 13.0, 5.0, 3.0]))],
            ),
            ChartType::Scatter | ChartType::ScatterLines | ChartType::ScatterSmooth => (
                vec!["1", "2", "3", "4"],
                vec![ChartSeries::of("Series 1", some(&[2.1, 3.4, 3.0, 4.6])), ChartSeries::of("Series 2", some(&[1.2, 1.9, 2.8, 3.1]))],
            ),
            ChartType::Bubble => (
                vec!["1", "2", "3", "4"],
                vec![
                    ChartSeries { sizes: some(&[4.0, 9.0, 2.0, 6.0]), ..ChartSeries::of("Series 1", some(&[2.1, 3.4, 3.0, 4.6])) },
                    ChartSeries { sizes: some(&[3.0, 5.0, 8.0, 1.5]), ..ChartSeries::of("Series 2", some(&[1.2, 1.9, 2.8, 3.1])) },
                ],
            ),
            ChartType::Radar | ChartType::RadarMarkers | ChartType::RadarFilled => (
                vec!["Speed", "Range", "Comfort", "Safety", "Price"],
                vec![ChartSeries::of("Model A", some(&[4.0, 3.0, 5.0, 4.0, 2.0])), ChartSeries::of("Model B", some(&[3.0, 5.0, 3.0, 4.0, 4.0]))],
            ),
            ChartType::Waterfall => (
                vec!["Start", "Sales", "Returns", "Services", "Costs", "End"],
                vec![ChartSeries::of("Amount", some(&[10.0, 6.0, -2.0, 3.0, -4.0, 13.0]))],
            ),
            ChartType::Funnel => {
                (vec!["Visits", "Sign-ups", "Trials", "Customers"], vec![ChartSeries::of("People", some(&[1000.0, 620.0, 310.0, 120.0]))])
            }
            ChartType::Combo => (
                vec!["Q1", "Q2", "Q3", "Q4"],
                vec![
                    ChartSeries::of("Series 1", some(&[3.2, 4.1, 3.6, 5.0])),
                    ChartSeries::of("Series 2", some(&[2.0, 2.7, 3.9, 3.3])),
                    ChartSeries { kind: Some(SeriesKind::Line), secondary: true, ..ChartSeries::of("Growth %", some(&[12.0, 18.0, 15.0, 22.0])) },
                ],
            ),
            _ => (
                vec!["Q1", "Q2", "Q3", "Q4"],
                vec![
                    ChartSeries::of("Series 1", some(&[3.2, 4.1, 3.6, 5.0])),
                    ChartSeries::of("Series 2", some(&[2.0, 2.7, 3.9, 3.3])),
                    ChartSeries::of("Series 3", some(&[1.5, 2.2, 2.4, 3.8])),
                ],
            ),
        };
        ChartSpec {
            kind,
            categories: categories.into_iter().map(str::to_string).collect(),
            series,
            title: None,
            legend: if matches!(kind, ChartType::Waterfall | ChartType::Funnel) { None } else { Some(LegendPos::Bottom) },
            waterfall_total: kind == ChartType::Waterfall,
            // Radar outlines a hairline (1 px at 96 dpi).
            line_width: kind.is_radar().then_some(LINE_WIDTHS.0),
            ..Default::default()
        }
    }

    /// Number of categories: every series has as many values.
    pub fn points(&self) -> usize {
        self.categories.len()
    }

    /// Make the chart safe to store and draw: text cut to [`MAX_TEXT`] characters with control
    /// characters turned into spaces and the ends trimmed (an empty title is none); at most
    /// [`MAX_POINTS`] categories, [`MAX_SERIES`] series and [`MAX_CELLS`] cells; values that
    /// aren't finite become gaps; every series as long as the categories (padded with gaps,
    /// categories padded with empty labels to the longest series); at most [`MAX_COLORS`]
    /// colours, each six hex digits (others dropped); axis numbers finite and clamped, a step
    /// that isn't positive is automatic, and bounds with `min >= max` are both automatic.
    pub fn sanitize(&mut self) {
        self.colors = self.colors.iter().take(MAX_COLORS * 4).filter_map(|c| hex_color(c)).take(MAX_COLORS).collect();
        self.axis_min = self.axis_min.and_then(axis_number);
        self.axis_max = self.axis_max.and_then(axis_number);
        self.axis_step = self.axis_step.and_then(axis_number).filter(|s| *s > 0.0);
        self.series_gap = series_gap(self.series_gap);
        for st in [&mut self.legend_style, &mut self.data_labels_style, &mut self.cat_labels_style, &mut self.val_labels_style, &mut self.title_style]
        {
            st.sanitize();
        }
        self.cat_axis_color = self.cat_axis_color.as_deref().and_then(hex_color);
        self.val_axis_color = self.val_axis_color.as_deref().and_then(hex_color);
        self.background_color = self.background_color.as_deref().and_then(hex_color);
        self.line_width = self.line_width.filter(|w| w.is_finite()).map(|w| (w.clamp(LINE_WIDTHS.0, LINE_WIDTHS.1) * 100.0).round() / 100.0);
        self.area_opacity = opacity_of(self.area_opacity, DEFAULT_AREA_OPACITY);
        self.opacity = opacity_of(self.opacity, 1.0);
        self.bubble_opacity = opacity_of(self.bubble_opacity, DEFAULT_BUBBLE_OPACITY);
        self.hole_size = self.hole_size.clamp(10, 90);
        self.slice_gap = if self.slice_gap.is_finite() { (self.slice_gap.clamp(0.0, MAX_SLICE_GAP) * 100.0).round() / 100.0 } else { 0.0 };
        if let (Some(lo), Some(hi)) = (self.axis_min, self.axis_max)
            && lo >= hi
        {
            self.axis_min = None;
            self.axis_max = None;
        }
        self.series.truncate(MAX_SERIES);
        let longest = self.series.iter().map(|s| s.values.len()).max().unwrap_or(0);
        let n = self.categories.len().max(longest).min(MAX_POINTS);
        let keep = MAX_CELLS.checked_div(n).unwrap_or(MAX_SERIES).clamp(1, MAX_SERIES);
        self.series.truncate(keep);
        self.categories.truncate(n);
        for c in &mut self.categories {
            *c = clean_text(c);
        }
        self.categories.resize(n, String::new());
        let (bubble, combo) = (self.kind == ChartType::Bubble, self.kind == ChartType::Combo);
        for s in &mut self.series {
            s.name = clean_text(&s.name);
            s.values.truncate(n);
            for v in &mut s.values {
                *v = v.filter(|x| x.is_finite());
            }
            s.values.resize(n, None);
            // Bubble sizes on bubble charts only, positive; none at all: every bubble the same.
            if bubble {
                s.sizes.truncate(n);
                for v in &mut s.sizes {
                    *v = v.filter(|x| x.is_finite() && *x > 0.0);
                }
                s.sizes.resize(n, None);
                if s.sizes.iter().all(Option::is_none) {
                    s.sizes = vec![Some(1.0); n];
                }
            } else {
                s.sizes.clear();
            }
            // Series kinds and the secondary axis on combo charts only.
            if !combo {
                (s.kind, s.secondary) = (None, false);
            } else if s.kind == Some(SeriesKind::Column) {
                s.kind = None;
            }
        }
        self.title = self.title.as_deref().map(clean_text).filter(|t| !t.is_empty());
        self.waterfall_total &= self.kind == ChartType::Waterfall;
        self.radar_gradient &= self.kind.is_radar();
        self.split_pos = if self.kind.is_of_pie() {
            self.split_pos.clamp(1, u32::try_from(n.saturating_sub(1)).unwrap_or(u32::MAX).max(1))
        } else {
            DEFAULT_SPLIT
        };
        // Scatter lines join the points in x order: the data is kept in that order.
        if matches!(self.kind, ChartType::ScatterLines | ChartType::ScatterSmooth) {
            self.sort_by_x();
        }
    }

    /// Categories and every series' values (and sizes) in ascending x order, when every category
    /// is a number (a stable sort: equal x values keep their order).
    fn sort_by_x(&mut self) {
        let xs: Option<Vec<f64>> = self.categories.iter().map(|c| c.trim().parse::<f64>().ok().filter(|x| x.is_finite())).collect();
        let Some(xs) = xs else { return };
        let mut order: Vec<usize> = (0..xs.len()).collect();
        order.sort_by(|a, b| xs.get(*a).unwrap_or(&0.0).total_cmp(xs.get(*b).unwrap_or(&0.0)));
        if order.iter().enumerate().all(|(i, o)| i == *o) {
            return;
        }
        let pick = |v: &[Option<f64>]| order.iter().map(|i| v.get(*i).copied().flatten()).collect::<Vec<_>>();
        self.categories = order.iter().filter_map(|i| self.categories.get(*i).cloned()).collect();
        for s in &mut self.series {
            s.values = pick(&s.values);
            if !s.sizes.is_empty() {
                s.sizes = pick(&s.sizes);
            }
        }
    }

    /// The chart after `sanitize`.
    pub fn sanitized(mut self) -> ChartSpec {
        self.sanitize();
        self
    }
}

/// `s` as chart text: control characters as spaces, ends trimmed, at most [`MAX_TEXT`] characters.
pub fn clean_text(s: &str) -> String {
    let s: String = s.chars().take(MAX_TEXT * 4).map(|c| if c.is_control() { ' ' } else { c }).collect();
    s.trim().chars().take(MAX_TEXT).collect::<String>().trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_caps_sizes_and_drops_non_finite_values() {
        let mut c = ChartSpec {
            categories: vec!["a".into(); MAX_POINTS + 10],
            series: (0..MAX_SERIES + 5)
                .map(|i| ChartSeries {
                    name: format!("{}\n{}", "x".repeat(400), i),
                    values: vec![Some(f64::NAN), Some(f64::INFINITY), Some(1.0)],
                    ..Default::default()
                })
                .collect(),
            title: Some("  ".into()),
            ..Default::default()
        };
        c.sanitize();
        assert_eq!(c.points(), MAX_POINTS);
        assert!(c.series.len() * c.points() <= MAX_CELLS);
        assert!(!c.series.is_empty());
        let s = &c.series[0];
        assert_eq!(s.values.len(), MAX_POINTS);
        assert_eq!(&s.values[..3], &[None, None, Some(1.0)]);
        assert_eq!(s.name.chars().count(), MAX_TEXT);
        assert!(!s.name.contains('\n'));
        assert_eq!(c.title, None);
    }

    #[test]
    fn short_categories_are_padded_to_the_longest_series() {
        let c = ChartSpec {
            categories: vec!["a".into()],
            series: vec![ChartSeries { name: "s".into(), values: vec![Some(1.0), Some(2.0)], ..Default::default() }],
            ..Default::default()
        }
        .sanitized();
        assert_eq!(c.categories, vec!["a".to_string(), String::new()]);
        assert_eq!(ChartType::from_id("BARSTACKED"), Some(ChartType::BarStacked));
        assert_eq!(LegendPos::from_id("top"), Some(LegendPos::Top));
        assert_eq!(ChartType::from_id("areastacked"), Some(ChartType::AreaStacked));
        assert_eq!(Gridlines::from_id("Dashed"), Some(Gridlines::Dashed));
    }

    #[test]
    fn sanitize_cleans_colours_and_axis() {
        let c = ChartSpec {
            colors: vec!["#FFAA00".into(), "zzz".into(), "123456".into(), "1234567".into()],
            axis_min: Some(f64::NAN),
            axis_max: Some(1e300),
            axis_step: Some(-2.0),
            ..ChartSpec::sample(ChartType::Line)
        }
        .sanitized();
        assert_eq!(c.colors, vec!["ffaa00".to_string(), "123456".to_string()]);
        assert_eq!((c.axis_min, c.axis_max, c.axis_step), (None, Some(MAX_AXIS), None));
        let c = ChartSpec { colors: vec!["000000".into(); 500], axis_min: Some(5.0), axis_max: Some(5.0), ..Default::default() }.sanitized();
        assert_eq!(c.colors.len(), MAX_COLORS);
        assert_eq!((c.axis_min, c.axis_max), (None, None));
    }

    #[test]
    fn legend_shape_defaults_to_square_and_is_written_only_when_round() {
        let old: ChartSpec = serde_json::from_str(r#"{"type":"pie","legend":"right"}"#).unwrap();
        assert_eq!(old.legend_shape, LegendShape::Square);
        assert!(serde_json::to_value(&old).unwrap().get("legendShape").is_none());
        let round = ChartSpec { legend_shape: LegendShape::Circle, ..Default::default() };
        let j = serde_json::to_value(&round).unwrap();
        assert_eq!(j["legendShape"], "circle");
        assert_eq!(serde_json::from_value::<ChartSpec>(j).unwrap(), round);
        assert_eq!(LegendShape::from_id("Circle"), Some(LegendShape::Circle));
        assert_eq!(LegendShape::from_id("dot"), None);
    }

    #[test]
    fn series_gap_is_a_whole_percent() {
        for (v, want) in [(150.4, 100.0), (-3.0, 0.0), (f64::NAN, 0.0), (12.6, 13.0)] {
            assert_eq!(ChartSpec { series_gap: v, ..Default::default() }.sanitized().series_gap, want, "{v}");
        }
        let j = serde_json::to_value(ChartSpec { series_gap: 20.0, ..Default::default() }).unwrap();
        assert_eq!(j["seriesGap"], 20.0);
        assert!(serde_json::to_value(ChartSpec::default()).unwrap().get("seriesGap").is_none());
    }

    #[test]
    fn element_switches_default_on_except_the_background() {
        let old: ChartSpec = serde_json::from_str(r#"{"type":"column","title":"T"}"#).unwrap();
        assert_eq!(old, ChartSpec { title: Some("T".into()), ..Default::default() });
        assert!(old.title_visible && old.cat_labels && old.cat_axis && old.val_labels && old.val_axis && !old.background);
        let j = serde_json::to_value(&old).unwrap();
        for (k, v) in [("titleVisible", true), ("background", false), ("catLabels", true), ("catAxis", true), ("valLabels", true), ("valAxis", true)]
        {
            assert_eq!(j[k], v, "{k} is in the JSON");
        }
    }

    #[test]
    fn old_json_loads_and_defaults_are_not_written() {
        let c: ChartSpec = serde_json::from_str(r#"{"type":"line","categories":[],"series":[]}"#).unwrap();
        assert_eq!((c.gridlines, c.smooth, c.colors.len(), c.axis_step), (Gridlines::Solid, false, 0, None));
        let spec = ChartSpec {
            kind: ChartType::AreaStacked,
            gridlines: Gridlines::Dashed,
            smooth: true,
            colors: vec!["000000".into()],
            axis_max: Some(9.0),
            ..Default::default()
        };
        let j = serde_json::to_value(&spec).unwrap();
        assert_eq!(j["type"], "areaStacked");
        assert_eq!(j["gridlines"], "dashed");
        assert_eq!(j["smooth"], true);
        assert_eq!(j["colors"][0], "000000");
        assert_eq!(j["axisMax"], 9.0);
        assert!(j["axisMin"].is_null());
        assert!(j.get("markers").is_none());
        assert_eq!(serde_json::from_value::<ChartSpec>(j).unwrap(), spec);
    }

    #[test]
    fn look_fields_load_from_old_json_and_hostile_values_are_held() {
        // A model saved before the look fields: their defaults.
        let mut j = serde_json::to_value(ChartSpec::sample(ChartType::Line)).unwrap();
        for k in [
            "legendStyle",
            "dataLabelsStyle",
            "catLabelsStyle",
            "valLabelsStyle",
            "titleStyle",
            "dataLabelsOutline",
            "catLabelPos",
            "catTicks",
            "valTicks",
            "areaOpacity",
            "opacity",
            "bubbleOpacity",
            "holeSize",
            "sliceGap",
        ] {
            j.as_object_mut().unwrap().remove(k);
        }
        let old: ChartSpec = serde_json::from_value(j).unwrap();
        assert_eq!(old, ChartSpec::sample(ChartType::Line));
        assert!(old.val_ticks && !old.cat_ticks && old.area_opacity == DEFAULT_AREA_OPACITY && old.opacity == 1.0);
        // Every field in camelCase, set ones in the JSON.
        let set = ChartSpec {
            legend_style: TextStyle { font: Some("Noto Sans".into()), size: Some(12.0), color: Some("ff0000".into()), bold: Some(true) },
            cat_label_pos: CatLabelPos::Rotate,
            line_width: Some(3.0),
            ..ChartSpec::sample(ChartType::Line)
        };
        let j = serde_json::to_value(&set).unwrap();
        assert_eq!(j["legendStyle"], serde_json::json!({"font": "Noto Sans", "size": 12.0, "color": "ff0000", "bold": true}));
        assert_eq!((j["catLabelPos"].as_str(), j["lineWidth"].as_f64(), j["valTicks"].as_bool()), (Some("rotate"), Some(3.0), Some(true)));
        assert_eq!(j["titleStyle"], serde_json::json!({}));
        // Hostile values are held to what the chart takes.
        let mut c = ChartSpec {
            legend_style: TextStyle { font: Some(format!("{}\u{0}x", "f".repeat(500))), size: Some(f32::NAN), color: Some("zz".into()), bold: None },
            title_style: TextStyle { font: Some("  ".into()), size: Some(1e30), color: Some("#ABCDEF".into()), bold: Some(false) },
            data_labels_style: TextStyle { size: Some(-3.0), ..Default::default() },
            cat_axis_color: Some("12345".into()),
            background_color: Some("GGGGGG".into()),
            line_width: Some(f32::INFINITY),
            area_opacity: f32::NAN,
            opacity: -4.0,
            ..ChartSpec::sample(ChartType::Area)
        };
        c.sanitize();
        assert_eq!(c.legend_style.font.as_ref().map(|f| f.chars().count()), Some(MAX_FONT_NAME));
        assert_eq!((c.legend_style.size, c.legend_style.color.as_deref()), (None, None));
        assert_eq!((c.title_style.font.as_deref(), c.title_style.size, c.title_style.color.as_deref()), (None, Some(96.0), Some("abcdef")));
        assert_eq!(c.data_labels_style.size, Some(4.0));
        assert_eq!((c.cat_axis_color.as_deref(), c.background_color.as_deref(), c.line_width), (None, None, None));
        assert_eq!((c.area_opacity, c.opacity), (DEFAULT_AREA_OPACITY, 0.0));
        let mut w = ChartSpec { line_width: Some(0.1), opacity: 0.123_456, ..Default::default() };
        w.sanitize();
        assert_eq!((w.line_width, w.opacity), (Some(0.75), 0.123));
    }
}
