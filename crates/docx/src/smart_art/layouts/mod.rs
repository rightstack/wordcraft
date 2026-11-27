//! The layouts, a module per gallery category: each draws a graphic's shapes in a `w` × `h`
//! points box, back to front.
//!
//! Rules every layout keeps: each shape that stands for an item (or carries its text) names it
//! (`with_item` / `of_item`) so picking works; text says which theme colour it wants and
//! `fit_colours` makes it read; no shape's box leaves the frame; sizes come out finite for any
//! frame and any number of items.

mod basic;
mod cards;
mod cycle;
mod hierarchy;
mod list;
mod matrix;
mod picture;
mod process;
mod pyramid;
mod relationship;

use wordcraft_doc::smart_art::{SmartArtLayout as L, SmartArtSpec};

use super::shapes::Sh;

/// The shapes of `spec` in a `w` × `h` box (both finite, at least 1).
pub(crate) fn draw(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    match spec.layout {
        L::BasicList => basic::basic_list(spec, w, h),
        L::Process => basic::process(spec, w, h),
        L::Cycle => basic::cycle(spec, w, h),
        L::Hierarchy => basic::hierarchy(spec, w, h),
        L::Pyramid => basic::pyramid(spec, w, h),
        L::Radial => basic::radial(spec, w, h),
        L::Matrix => basic::matrix(spec, w, h),
        L::Venn => basic::venn(spec, w, h),
        L::TimelineDots => cards::timeline_dots(spec, w, h),
        L::TimelineNumbered => cards::timeline_numbered(spec, w, h),
        L::TimelineArrow => cards::timeline_arrow(spec, w, h),
        L::TimelineVertical => cards::timeline_vertical(spec, w, h),
        L::TeamCircles => cards::team_circles(spec, w, h),
        L::TeamCards => cards::team_cards(spec, w, h),
        L::TeamPills => cards::team_pills(spec, w, h),
        L::CardNumbered => cards::card_numbered(spec, w, h, false),
        L::CardBigNumbers => cards::card_numbered(spec, w, h, true),
        L::CardHeader => cards::card_header(spec, w, h),
        L::CardTall => cards::card_tall(spec, w, h),
        L::CardRows => cards::card_rows(spec, w, h),
        L::TimelineStems => cards::timeline_stems(spec, w, h),
        L::TimelineCircles => cards::timeline_circles(spec, w, h),
        L::TimelineZigzag => cards::timeline_zigzag(spec, w, h),
        L::TimelineLine => cards::timeline_line(spec, w, h),
        L::CardEmphasis => cards::card_emphasis(spec, w, h),
        L::CardMarkerRows => cards::card_marker_rows(spec, w, h),
        L::CardSquares => cards::card_squares(spec, w, h),
        L::PictureCaptions => picture::picture_captions(spec, w, h),
        L::PictureGrid => picture::picture_grid(spec, w, h),
        L::PictureCircles => picture::picture_circles(spec, w, h),
        L::PictureRows => picture::picture_rows(spec, w, h),
        L::PictureStrips => picture::picture_strips(spec, w, h),
        L::PictureStaggered => picture::picture_staggered(spec, w, h),
        L::MatrixTitled => matrix::matrix_titled(spec, w, h),
        L::MatrixGrid => matrix::matrix_grid(spec, w, h),
        L::MatrixCycle => matrix::matrix_cycle(spec, w, h),
        L::PyramidInverted => pyramid::pyramid_inverted(spec, w, h),
        L::PyramidList => pyramid::pyramid_list(spec, w, h),
        L::PyramidSegmented => pyramid::pyramid_segmented(spec, w, h),
        L::Balance => relationship::balance(spec, w, h),
        L::Funnel => relationship::funnel(spec, w, h),
        L::Gears => relationship::gears(spec, w, h),
        L::ArrowsOpposing => relationship::arrows_opposing(spec, w, h),
        L::Equation => relationship::equation(spec, w, h),
        L::VennLinear => relationship::venn_linear(spec, w, h),
        L::Target => relationship::target(spec, w, h),
        L::RadialDiverging => relationship::radial_diverging(spec, w, h),
        L::OrgChart => hierarchy::org_chart(spec, w, h),
        L::HierarchyHorizontal => hierarchy::hierarchy_horizontal(spec, w, h),
        L::HierarchyList => hierarchy::hierarchy_list(spec, w, h),
        L::HierarchyTable => hierarchy::hierarchy_table(spec, w, h),
        L::HierarchyCircles => hierarchy::hierarchy_circles(spec, w, h),
        L::LinedList => hierarchy::lined_list(spec, w, h),
        L::CycleText => cycle::cycle_text(spec, w, h),
        L::CycleContinuous => cycle::cycle_continuous(spec, w, h),
        L::CycleSegmented => cycle::cycle_segmented(spec, w, h),
        L::CycleMultidirectional => cycle::cycle_multidirectional(spec, w, h),
        L::CycleDots => cycle::cycle_dots(spec, w, h),
        L::CycleRadial => cycle::cycle_radial(spec, w, h),
        L::Chevrons => process::chevrons(spec, w, h),
        L::ChevronList => process::chevron_list(spec, w, h),
        L::StepUp => process::step_up(spec, w, h),
        L::ProcessVertical => process::process_vertical(spec, w, h),
        L::ProcessBending => process::process_bending(spec, w, h),
        L::ProcessContinuous => process::process_continuous(spec, w, h),
        L::ArrowUpward => process::arrow_upward(spec, w, h),
        L::ProcessArrows => process::process_arrows(spec, w, h),
        L::ListHexagon => list::list_hexagon(spec, w, h),
        L::ListVerticalBullet => list::list_vertical_bullet(spec, w, h),
        L::ListGrouped => list::list_grouped(spec, w, h),
        L::ListTrapezoid => list::list_trapezoid(spec, w, h),
        L::ListDescending => list::list_descending(spec, w, h),
        L::ListTable => list::list_table(spec, w, h),
        L::ListHexagonAlternating => list::list_hexagon_alternating(spec, w, h),
        L::ListVerticalBox => list::list_vertical_box(spec, w, h),
        L::ListHorizontalBullet => list::list_horizontal_bullet(spec, w, h),
        L::ListSquareAccent => list::list_square_accent(spec, w, h),
        L::ListStacked => list::list_stacked(spec, w, h),
        L::ListVerticalBlock => list::list_vertical_block(spec, w, h),
        L::ListVerticalArrow => list::list_vertical_arrow(spec, w, h),
        L::ListTarget => list::list_target(spec, w, h),
        L::ListVerticalCircle => list::list_vertical_circle(spec, w, h),
        L::ListVaryingWidth => list::list_varying_width(spec, w, h),
        L::ListTab => list::list_tab(spec, w, h),
        L::ListCurved => list::list_curved(spec, w, h),
        L::ProcessAccent => process::process_accent(spec, w, h),
        L::ProcessAlternatingFlow => process::process_alternating_flow(spec, w, h),
        L::ChevronsClosed => process::chevrons_closed(spec, w, h),
        L::ChevronsDetailed => process::chevrons_detailed(spec, w, h),
        L::ProcessCircle => process::process_circle(spec, w, h),
        L::ProcessSegmented => process::process_segmented(spec, w, h),
        L::ProcessSnake => process::process_snake(spec, w, h),
        L::ProcessBendingVertical => process::process_bending_vertical(spec, w, h),
        L::StepDown => process::step_down(spec, w, h),
        L::ProcessIncreasingArrows => process::process_increasing_arrows(spec, w, h),
        L::ProcessContinuousArrow => process::process_continuous_arrow(spec, w, h),
        L::ProcessList => process::process_list(spec, w, h),
        L::ArrowDownward => process::arrow_downward(spec, w, h),
        L::ProcessCirclesIncreasing => process::process_circles_increasing(spec, w, h),
        L::CycleBlock => cycle::cycle_block(spec, w, h),
        L::CycleNondirectional => cycle::cycle_nondirectional(spec, w, h),
        L::CyclePie => cycle::cycle_pie(spec, w, h),
        L::CycleHexagonRadial => cycle::cycle_hexagon_radial(spec, w, h),
        L::OrgChartHalfCircle => hierarchy::org_chart_half_circle(spec, w, h),
        L::HierarchyLabeled => hierarchy::hierarchy_labeled(spec, w, h),
        L::HierarchyLabeledHorizontal => hierarchy::hierarchy_labeled_horizontal(spec, w, h),
        L::HierarchyArchitecture => hierarchy::hierarchy_architecture(spec, w, h),
        L::HierarchyTableHorizontal => hierarchy::hierarchy_table_horizontal(spec, w, h),
        L::ArrowsConverging => relationship::arrows_converging(spec, w, h),
        L::ArrowsDiverging => relationship::arrows_diverging(spec, w, h),
        L::ArrowsCounterbalance => relationship::arrows_counterbalance(spec, w, h),
        L::VennRadial => relationship::venn_radial(spec, w, h),
        L::VennStacked => relationship::venn_stacked(spec, w, h),
        L::TargetNested => relationship::target_nested(spec, w, h),
        L::CircleRelationship => relationship::circle_relationship(spec, w, h),
        L::PlusMinus => relationship::plus_minus(spec, w, h),
        L::TextConverging => relationship::text_converging(spec, w, h),
        L::RadialList => relationship::radial_list(spec, w, h),
        L::EquationVertical => relationship::equation_vertical(spec, w, h),
        L::RingsInterconnected => relationship::rings_interconnected(spec, w, h),
        L::RadialCluster => relationship::radial_cluster(spec, w, h),
        L::RadialConverging => relationship::radial_converging(spec, w, h),
        L::TimelineCircleAccent => cards::timeline_circle_accent(spec, w, h),
    }
}

/// How a layout definition walks the outline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tree {
    /// Each top item (its text with the items under it as lines).
    Items,
    /// A tree: each item, then its children (`hierRoot` / `hierChild`).
    Hier,
    /// The first item in the middle, the items under it round it.
    Radial,
}

/// What the layout definition (`dgm:layoutDef`) we write for a layout says: its algorithm and
/// parameters, the shape of its nodes and how it walks the outline. The drawing is ours; this is
/// what a reader that lays the graphic out again (Word, once the graphic is edited there) uses.
pub(crate) struct Def {
    pub(crate) alg: &'static str,
    pub(crate) params: &'static [(&'static str, &'static str)],
    pub(crate) shape: &'static str,
    pub(crate) tree: Tree,
}

const ROW: &[(&str, &str)] = &[("linDir", "fromL"), ("nodeVertAlign", "mid")];
const COLUMN: &[(&str, &str)] = &[("linDir", "fromT")];
const SNAKE: &[(&str, &str)] = &[("grDir", "tL"), ("flowDir", "row"), ("contDir", "sameDir")];
const RING: &[(&str, &str)] = &[("stAng", "0"), ("spanAng", "360")];
const CENTRE: &[(&str, &str)] = &[("stAng", "0"), ("spanAng", "360"), ("ctrShpMap", "fNode")];
const TREE: &[(&str, &str)] = &[("linDir", "fromL"), ("chAlign", "t")];
const TREE_ACROSS: &[(&str, &str)] = &[("linDir", "fromT"), ("chAlign", "l")];

/// The layout definition of the layouts from the third batch on (the earlier ones are spelled
/// out where the definition is written).
pub(crate) fn def(l: L) -> Def {
    let d = |alg, params, shape, tree| Def { alg, params, shape, tree };
    match l {
        L::PictureCaptions | L::PictureCircles | L::PictureStrips | L::PictureStaggered => d("lin", ROW, "roundRect", Tree::Items),
        L::PictureGrid | L::MatrixGrid | L::ListHexagon => d("snake", SNAKE, if l == L::ListHexagon { "hexagon" } else { "rect" }, Tree::Items),
        L::PictureRows | L::ProcessVertical | L::ListVerticalBullet | L::ChevronList => d("lin", COLUMN, "rect", Tree::Items),
        L::MatrixTitled => d("cycle", CENTRE, "roundRect", Tree::Radial),
        L::MatrixCycle | L::CycleSegmented => d("cycle", RING, "pie", Tree::Items),
        L::PyramidInverted | L::PyramidList | L::PyramidSegmented => d("pyra", COLUMN, "trapezoid", Tree::Items),
        L::Balance | L::ArrowsOpposing | L::Equation | L::VennLinear | L::Funnel => d("lin", ROW, "ellipse", Tree::Items),
        L::Gears | L::CycleText | L::CycleContinuous | L::CycleMultidirectional | L::CycleDots => d("cycle", RING, "ellipse", Tree::Items),
        L::Target => d("lin", COLUMN, "ellipse", Tree::Items),
        L::RadialDiverging | L::CycleRadial => d("cycle", CENTRE, "ellipse", Tree::Radial),
        L::OrgChart | L::HierarchyTable | L::HierarchyCircles => d("hierChild", TREE, "roundRect", Tree::Hier),
        L::HierarchyHorizontal | L::HierarchyList | L::LinedList | L::ListGrouped => d("hierChild", TREE_ACROSS, "rect", Tree::Hier),
        L::Chevrons | L::StepUp | L::ProcessBending | L::ProcessContinuous | L::ArrowUpward | L::ProcessArrows => {
            d("lin", ROW, if l == L::Chevrons { "chevron" } else { "roundRect" }, Tree::Items)
        }
        L::ListTrapezoid | L::ListDescending | L::ListTable => d("lin", ROW, "rect", Tree::Items),
        L::ListVerticalBox
        | L::ListVerticalBlock
        | L::ListVerticalArrow
        | L::ListTarget
        | L::ListVerticalCircle
        | L::ListVaryingWidth
        | L::ListTab => d("lin", COLUMN, "rect", Tree::Items),
        L::ListCurved | L::ProcessCirclesIncreasing => d("lin", COLUMN, "ellipse", Tree::Items),
        L::ListHexagonAlternating => d("lin", ROW, "hexagon", Tree::Items),
        L::ListHorizontalBullet | L::ListSquareAccent | L::ListStacked | L::ProcessAccent | L::ProcessAlternatingFlow | L::PlusMinus => {
            d("lin", ROW, "rect", Tree::Items)
        }
        L::ChevronsClosed | L::ChevronsDetailed => d("lin", ROW, "chevron", Tree::Items),
        L::ProcessCircle | L::StepDown | L::ArrowDownward | L::RingsInterconnected | L::VennStacked => d("lin", ROW, "ellipse", Tree::Items),
        L::ProcessSnake | L::ProcessBendingVertical => d("snake", SNAKE, "roundRect", Tree::Items),
        L::ProcessIncreasingArrows | L::ProcessContinuousArrow | L::ArrowsConverging | L::ArrowsDiverging => d("lin", ROW, "rightArrow", Tree::Items),
        L::ArrowsCounterbalance | L::TargetNested | L::EquationVertical => d("lin", COLUMN, "roundRect", Tree::Items),
        L::ProcessSegmented | L::TextConverging => d("hierChild", TREE_ACROSS, "roundRect", Tree::Hier),
        L::ProcessList | L::OrgChartHalfCircle | L::HierarchyLabeled | L::HierarchyArchitecture | L::TimelineCircleAccent => {
            d("hierChild", TREE, if l == L::TimelineCircleAccent { "ellipse" } else { "roundRect" }, Tree::Hier)
        }
        L::HierarchyLabeledHorizontal | L::HierarchyTableHorizontal => d("hierChild", TREE_ACROSS, "rect", Tree::Hier),
        L::CycleBlock => d("cycle", RING, "roundRect", Tree::Items),
        L::CycleNondirectional => d("cycle", RING, "ellipse", Tree::Items),
        L::CyclePie => d("cycle", RING, "pie", Tree::Items),
        L::CycleHexagonRadial => d("cycle", CENTRE, "hexagon", Tree::Radial),
        L::VennRadial | L::CircleRelationship | L::RadialList | L::RadialConverging => d("cycle", CENTRE, "ellipse", Tree::Radial),
        L::RadialCluster => d("cycle", CENTRE, "roundRect", Tree::Radial),
        _ => d("lin", ROW, "rect", Tree::Items),
    }
}
