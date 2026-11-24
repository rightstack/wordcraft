use super::canvas::Rect;
use super::{Rgb, TextAlign};

/// A data label to draw once the chart is ([`super::labels::draw_data_labels`]).
pub(super) enum DataLabel {
    /// The value of the point at (`x`, `y`) of category `cat` (line, area, scatter, radar): just
    /// above it, unless the file asks for another place (`pos`); labels of a category that
    /// would overlap are moved apart. `series` is its series' colour (line and area labels): one
    /// that lands on a bar sits on a small white chip outlined in it.
    Point { x: f32, y: f32, text: String, cat: usize, pos: Option<LabelPos>, series: Option<Rgb> },
    /// A label already placed (on a bar, a pie slice), in its colour.
    Placed { rect: Rect, text: String, color: Rgb, align: TextAlign },
}

/// Where a file puts data labels (`c:dLblPos`, ECMA-376 ST_DLblPos).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LabelPos {
    Top,
    Bottom,
    Left,
    Right,
    Centre,
    InEnd,
    InBase,
    OutEnd,
    BestFit,
}

impl LabelPos {
    pub(super) fn of(v: &str) -> Option<LabelPos> {
        Some(match v {
            "t" => LabelPos::Top,
            "b" => LabelPos::Bottom,
            "l" => LabelPos::Left,
            "r" => LabelPos::Right,
            "ctr" => LabelPos::Centre,
            "inEnd" => LabelPos::InEnd,
            "inBase" => LabelPos::InBase,
            "outEnd" => LabelPos::OutEnd,
            "bestFit" => LabelPos::BestFit,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Bar,
    Line,
    Area,
    Pie,
    Doughnut,
    Scatter,
    Radar,
    Bubble,
    /// Pie of pie or bar of pie.
    OfPie,
}

impl Kind {
    pub(super) fn of(name: &str) -> Option<Kind> {
        Some(match name {
            "c:barChart" => Kind::Bar,
            "c:lineChart" => Kind::Line,
            "c:areaChart" => Kind::Area,
            "c:pieChart" => Kind::Pie,
            "c:doughnutChart" => Kind::Doughnut,
            "c:scatterChart" => Kind::Scatter,
            "c:radarChart" => Kind::Radar,
            "c:bubbleChart" => Kind::Bubble,
            "c:ofPieChart" => Kind::OfPie,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Grouping {
    Clustered,
    Stacked,
    Percent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Pos {
    Right,
    Left,
    Top,
    Bottom,
}

/// One value axis (or category axis) as the file states it.
#[derive(Clone, Default)]
pub(super) struct Axis {
    pub(super) deleted: bool,
    /// Tick labels are shown (`c:tickLblPos` other than `none`).
    pub(super) labels: bool,
    pub(super) grid: bool,
    pub(super) line: bool,
    /// `c:crossBetween val="midCat"`: categories sit on the tick marks, not between them.
    pub(super) mid: bool,
    pub(super) min: Option<f64>,
    pub(super) max: Option<f64>,
    /// Distance between major ticks (`c:majorUnit`), when the file sets it.
    pub(super) step: Option<f64>,
    /// The major gridlines are dashed (`a:prstDash` other than `solid`, or `a:custDash`).
    pub(super) grid_dash: bool,
    /// Colour and width (points) of the major gridlines, when the file sets them.
    pub(super) grid_color: Option<Rgb>,
    pub(super) grid_w: Option<f32>,
    /// Colour and width (points) of the axis line, when the file sets them.
    pub(super) line_color: Option<Rgb>,
    pub(super) line_w: Option<f32>,
    /// Major tick marks (`c:majorTickMark`).
    pub(super) ticks: Ticks,
    /// Colour of the tick labels (`c:txPr`), when the file sets one.
    pub(super) text_color: Option<Rgb>,
    /// Number format of the tick labels, when not linked to the source data.
    pub(super) fmt: Option<String>,
    /// The tick labels' size, weight and font (`c:txPr`), where the file sets them.
    pub(super) look: Look,
    /// The tick labels are slanted up by this many degrees (`a:bodyPr/@rot`, 0..90; 0: level).
    pub(super) slant: f32,
}

/// How an element's text looks where its run properties say (`a:defRPr`, a title's `a:rPr`):
/// size (points), weight, colour and font. Unset: the chart's own.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Look {
    pub(super) size: Option<f32>,
    pub(super) bold: Option<bool>,
    pub(super) color: Option<Rgb>,
    pub(super) font: Option<String>,
}

/// Where an axis's major tick marks sit across its line (ECMA-376 ST_TickMark).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Ticks {
    /// No tick marks; also when the file doesn't say (WordCraft drew none before it read them).
    #[default]
    None,
    /// Away from the plot.
    Out,
    /// Into the plot.
    In,
    /// Across the line, both sides.
    Cross,
}

impl Axis {
    /// Whether the axis's labels are drawn: it isn't deleted and its labels aren't hidden.
    pub(super) fn labelled(&self) -> bool {
        !self.deleted && self.labels
    }
}

impl Ticks {
    /// How far the marks reach out of the plot and into it, points, for marks `len` long.
    pub(super) fn reach(self, len: f32) -> (f32, f32) {
        match self {
            Ticks::None => (0.0, 0.0),
            Ticks::Out => (len, 0.0),
            Ticks::In => (0.0, len),
            Ticks::Cross => (len, len),
        }
    }
}

/// A data point marker (`c:marker`) the file asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Marker {
    /// Width and height, points.
    pub(super) size: f32,
    /// A square (`c:symbol val="square"`); every other symbol draws as a circle.
    pub(super) square: bool,
    pub(super) fill: Option<Rgb>,
}

#[derive(Clone)]
pub(super) struct Series {
    pub(super) name: String,
    /// Which theme accent it takes when it has no colour of its own (`c:idx`).
    pub(super) idx: usize,
    /// A helper series drawn as nothing (no fill and no line): a waterfall's base.
    pub(super) hidden: bool,
    /// The chart type element it is in (combo charts mix them) and whether that one is on the
    /// secondary value axis.
    pub(super) group: Kind,
    pub(super) secondary: bool,
    /// Bubble sizes (bubble charts).
    pub(super) sizes: Vec<Option<f64>>,
    pub(super) color: Option<Rgb>,
    pub(super) line_w: f32,
    /// The line is a smooth curve through the points (`c:smooth`).
    pub(super) smooth: bool,
    /// Bars drawn with rounded corners (`a:prstGeom prst="roundRect"` in the series' `c:spPr`).
    pub(super) rounded: bool,
    /// A linear gradient fill (`a:gradFill` with `a:lin` in the series' `c:spPr`): area series
    /// are painted with it.
    pub(super) gradient: Option<std::sync::Arc<wordcraft_doc::fill::Gradient>>,
    /// The series' outline colour and width (`a:ln` with a fill): drawn along an area's top edge.
    pub(super) edge: Option<(Rgb, f32)>,
    /// Markers on the points, when the file names a symbol other than `none`.
    pub(super) marker: Option<Marker>,
    /// Whether the series is drawn as a line (scatter series without one get markers).
    pub(super) line: bool,
    pub(super) values: Vec<Option<f64>>,
    /// Scatter x values (index + 1 when the file has none).
    pub(super) xs: Vec<Option<f64>>,
    /// Per-point colours from `c:dPt`.
    pub(super) points: Vec<Option<Rgb>>,
    /// The fill's opacity (0..1, its solid fill's alpha, else the first point's): bubbles are
    /// drawn with it, over a solid outline when below 1.
    pub(super) fill_alpha: f32,
    pub(super) fmt: Option<String>,
    /// Format code of the scatter x values' cache.
    pub(super) x_fmt: Option<String>,
    /// Data labels on or off for this series (`c:dLbls` of its own), when it says.
    pub(super) labels: Option<bool>,
    /// Where its data labels go (`c:dLblPos` of its own, else of its chart type), when the file says.
    pub(super) label_pos: Option<LabelPos>,
    /// The values as the file has them, for data labels (`values` become shares in a 100%
    /// stacked chart); empty when the series shows no labels.
    pub(super) raw: Vec<Option<f64>>,
}
