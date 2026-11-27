//! SmartArt graphics WordCraft makes and edits (Insert › SmartArt): a layout, an outline of items
//! (text and level) and a colour variant.
//!
//! A SmartArt object ([`crate::graphic::Graphic`] with `smart_art` set) keeps this model next to
//! the items it is drawn with. WordCraft lays it out with its own algorithms; the DOCX writer
//! saves it as diagram parts (data, layout, style, colours and the drawing of the shapes) and the
//! reader turns a diagram back into it only when its data part is exactly what WordCraft writes.
//! SmartArt from other programs stays as it was read.
//!
//! Every value from outside (commands, files) goes through [`SmartArtSpec::sanitize`]: item
//! count, text length and levels are capped.

use serde::{Deserialize, Serialize};

/// Most items in a SmartArt graphic.
pub const MAX_ITEMS: usize = 100;
/// Longest item text, characters.
pub const MAX_TEXT: usize = 255;
/// Deepest item level (0 is the top).
pub const MAX_LEVEL: u8 = 7;

/// How a SmartArt graphic arranges its items.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SmartArtLayout {
    /// Blocks in rows.
    #[default]
    BasicList,
    /// Steps left to right with arrows between.
    Process,
    /// Steps around a circle with arrows between.
    Cycle,
    /// A tree: each item under the item one level up before it.
    Hierarchy,
    /// Stacked bands narrowing to the top.
    Pyramid,
    /// The first item in the middle, the items under it around it.
    Radial,
    /// Quadrants of a square.
    Matrix,
    /// Overlapping circles.
    Venn,
    /// Dots on a horizontal line, each item's text above or below in turn.
    TimelineDots,
    /// Numbered circles joined by a line, each item's text under its circle.
    TimelineNumbered,
    /// Dots on an arrow-shaped band, each item's text under its dot.
    TimelineArrow,
    /// Dots down a vertical line, each item's text to one side and then the other.
    TimelineVertical,
    /// A person placeholder per item in a row, the item's text under it.
    TeamCircles,
    /// A card per item, stacked, with a person placeholder and the item's text.
    TeamCards,
    /// Tall rounded cards: the item's text on top, a person placeholder at the bottom.
    TeamPills,
    /// Columns with a number and a bar beside the item's text.
    CardNumbered,
    /// Columns under large two-digit numbers.
    CardBigNumbers,
    /// Cards with the item's text in a coloured band and the items under it below.
    CardHeader,
    /// Tall tinted cards with a numbered circle on top.
    CardTall,
    /// Rows, each with a numbered square and the item's text beside it.
    CardRows,
    /// Stems up and down from a horizontal line in turn, each item's text at its stem's end.
    TimelineStems,
    /// Overlapping numbered circles on a line, larger and stronger towards the middle.
    TimelineCircles,
    /// Numbered circles in a zigzag joined by slanted strokes with a small dot on each.
    TimelineZigzag,
    /// Small dots on a thin line near the top, each item's text under its dot.
    TimelineLine,
    /// Columns of text whose last line is large and in the item's colour.
    CardEmphasis,
    /// Rows of text, each with a small square marker at its right end.
    CardMarkerRows,
    /// Tinted squares, each with a large number and the item's text inside.
    CardSquares,
    /// Pictures in a row, a caption under each.
    PictureCaptions,
    /// Picture cards in a grid, each with a coloured caption band.
    PictureGrid,
    /// Tinted cards with a round picture over each card's top edge.
    PictureCircles,
    /// Rows, each with a picture on the left and the item's text beside it.
    PictureRows,
    /// Tall picture strips, each with a coloured caption at the bottom.
    PictureStrips,
    /// Pictures high and low in turn, the item's text in the other half.
    PictureStaggered,
    /// The first item in a circle over the middle of quadrants that hold the items under it.
    MatrixTitled,
    /// Tinted quadrants on two axes, lines between them.
    MatrixGrid,
    /// A circle cut into wedges, each item's lines in a box at its corner.
    MatrixCycle,
    /// Bands stacked into an upside-down triangle: the first item at the wide top.
    PyramidInverted,
    /// A tinted triangle with a bar per item across it.
    PyramidList,
    /// A triangle cut into smaller triangles, one per item.
    PyramidSegmented,
    /// Two sides on a beam over a stand, tipped towards the side with more lines.
    Balance,
    /// Items in circles above a funnel; the last item comes out below.
    Funnel,
    /// Interlocking gears, one per item.
    Gears,
    /// Arrows up and down in turn, each item's text beside its arrow.
    ArrowsOpposing,
    /// Items joined by plus signs, the last after an equals sign.
    Equation,
    /// Circles overlapping in a row.
    VennLinear,
    /// Rings inside each other, each item's text to the right with a leader line.
    Target,
    /// The first item in the middle, arrows out to the items under it.
    RadialDiverging,
    /// A tree whose third and deeper levels hang in a column under their item.
    OrgChart,
    /// A tree from left to right.
    HierarchyHorizontal,
    /// Each top item heads a column; the items under it hang below.
    HierarchyList,
    /// Blocks in rows, each as wide as the items under it.
    HierarchyTable,
    /// A tree of circles joined by lines.
    HierarchyCircles,
    /// Each top item on the left, the items under it as ruled rows beside it.
    LinedList,
    /// Items' text round a circle with curved arrows between.
    CycleText,
    /// Blocks on a ring that ends in an arrow.
    CycleContinuous,
    /// A ring cut into segments, each item's text in its segment.
    CycleSegmented,
    /// Blocks round a circle with two-headed arrows between.
    CycleMultidirectional,
    /// Dots on a ring, each item's text outside its dot.
    CycleDots,
    /// The first item in the middle, the items under it on a ring round it.
    CycleRadial,
    /// Chevrons in a row, nested point to notch.
    Chevrons,
    /// Chevrons pointing down, each item's lines in a box beside it.
    ChevronList,
    /// Blocks rising like stairs, joined by steps.
    StepUp,
    /// Blocks from top to bottom with arrows between.
    ProcessVertical,
    /// Blocks in rows, each row left to right, joined row to row.
    ProcessBending,
    /// Blocks on a wide arrow across the graphic.
    ProcessContinuous,
    /// Dots along a rising arrow, each item's text above its dot.
    ArrowUpward,
    /// Columns: the item in a circle over an arrow holding its lines.
    ProcessArrows,
    /// Hexagons fitted together like a honeycomb.
    ListHexagon,
    /// Coloured bars stacked, each item's lines under its bar.
    ListVerticalBullet,
    /// Tinted columns headed by each top item, the items under it as boxes inside.
    ListGrouped,
    /// Columns of trapezoids holding each item's text.
    ListTrapezoid,
    /// Columns that get shorter from left to right.
    ListDescending,
    /// The first item as a header across the top, the others as columns under it.
    ListTable,
    /// A zigzag of hexagons: each item's line in a filled one, its lines in a light one beside it.
    ListHexagonAlternating,
    /// Rows: a light box with the item's lines, its own line on a tab over the box's corner.
    ListVerticalBox,
    /// Columns: the item's line in a header, its lines as bullets in a light box below.
    ListHorizontalBullet,
    /// Columns: a small square and the item's line over a rule, its lines below after small squares.
    ListSquareAccent,
    /// Columns: a tall light card with the item's lines and a circle with its own line over its top.
    ListStacked,
    /// Rows: the item's line in a block, its lines in a light block beside it.
    ListVerticalBlock,
    /// Rows: the item's line in an arrow pointing into a light box with its lines.
    ListVerticalArrow,
    /// Quarter rings in the lower left corner; each item's text in a row to the right, joined to its ring.
    ListTarget,
    /// A line down the left with a circle per item, the item's text beside its circle over a rule.
    ListVerticalCircle,
    /// Centred bars stacked, each as wide as its text needs.
    ListVaryingWidth,
    /// Rows: the item's line on the left, its lines on the right, a rule under each with a tab at its start.
    ListTab,
    /// A curve down the left with a circle per item and a light bar beside it holding the item's text.
    ListCurved,
    /// Columns: the item's line in a block over a light box with its lines, arrows between.
    ProcessAccent,
    /// Blocks in a row with arrows between, each item's lines above and below in turn.
    ProcessAlternatingFlow,
    /// A flat-backed arrow then chevrons, nested point to notch, each item's text inside.
    ChevronsClosed,
    /// Light chevrons in a row with the item's line inside and its lines below.
    ChevronsDetailed,
    /// Circles in a row with arrows between, each item's lines under its circle.
    ProcessCircle,
    /// Rows: each top item's bar, the items under it as boxes below it, arrows down between rows.
    ProcessSegmented,
    /// Blocks in rows that turn back at each end, arrows between.
    ProcessSnake,
    /// Blocks in columns, down then up, arrows between.
    ProcessBendingVertical,
    /// Circles stepping down to the right, joined by elbow lines, each item's lines beside its circle.
    StepDown,
    /// Stacked arrows to the right edge, each starting a column later; each item's lines in its column below.
    ProcessIncreasingArrows,
    /// One wide arrow across, each item's text in a section of it.
    ProcessContinuousArrow,
    /// Columns: each top item in a header with arrows between, the items under it as boxes below.
    ProcessList,
    /// Dots along a falling arrow, each item's text beside its dot.
    ArrowDownward,
    /// Rows: a ring on the left filled further each row, the item's text beside it.
    ProcessCirclesIncreasing,
    /// Blocks round a circle with curved arrows between.
    CycleBlock,
    /// Circles on a ring, each item's text inside its circle.
    CycleNondirectional,
    /// A circle cut into wedges, each item's text in its wedge.
    CyclePie,
    /// The first item in a large hexagon, the items under it in hexagons round it.
    CycleHexagonRadial,
    /// An organization chart whose items sit inside half rings.
    OrgChartHalfCircle,
    /// A tree from the top over a band per level; later top items with nothing under them name the bands.
    HierarchyLabeled,
    /// A tree from the left over a column per level; later top items with nothing under them name the columns.
    HierarchyLabeledHorizontal,
    /// Blocks stacked from the bottom, each as wide as the items above it.
    HierarchyArchitecture,
    /// Blocks in a column per level from the left, each as tall as the items beside it.
    HierarchyTableHorizontal,
    /// Arrows pointing in to the middle from both sides, each item's text inside.
    ArrowsConverging,
    /// Arrows pointing out from the middle to both sides, each item's text inside.
    ArrowsDiverging,
    /// Wide arrows stacked, pointing right and left in turn, each item's text inside.
    ArrowsCounterbalance,
    /// The first item in a large circle, the items under it in see-through circles over its edge.
    VennRadial,
    /// See-through circles inside each other on one bottom point, each item's text at the top of its band.
    VennStacked,
    /// Rounded rectangles inside each other, each item's text at the top of its band.
    TargetNested,
    /// The first item in a large circle, the items under it in small circles round its edge.
    CircleRelationship,
    /// Columns under a plus and a minus sign in turn, each item's text in a box under its sign.
    PlusMinus,
    /// Rows: the items under each top item in boxes, arrows pointing into a circle with the top item.
    TextConverging,
    /// The first item in a large circle, the items under it in small circles on an arc beside it.
    RadialList,
    /// Circles top to bottom joined by plus signs, an arrow to the last item in a larger circle.
    EquationVertical,
    /// Thick rings linked in a row, each item's text inside its ring.
    RingsInterconnected,
    /// The first item in a block, the items under it in smaller blocks round it, joined by lines.
    RadialCluster,
    /// The items under the first on an arc above, arrows pointing down to the first item.
    RadialConverging,
    /// A line with a large circle per top item and small circles for the items under it.
    TimelineCircleAccent,
}

impl SmartArtLayout {
    pub const ALL: [SmartArtLayout; 123] = [
        SmartArtLayout::BasicList,
        SmartArtLayout::Process,
        SmartArtLayout::Cycle,
        SmartArtLayout::Hierarchy,
        SmartArtLayout::Pyramid,
        SmartArtLayout::Radial,
        SmartArtLayout::Matrix,
        SmartArtLayout::Venn,
        SmartArtLayout::TimelineDots,
        SmartArtLayout::TimelineNumbered,
        SmartArtLayout::TimelineArrow,
        SmartArtLayout::TimelineVertical,
        SmartArtLayout::TeamCircles,
        SmartArtLayout::TeamCards,
        SmartArtLayout::TeamPills,
        SmartArtLayout::CardNumbered,
        SmartArtLayout::CardBigNumbers,
        SmartArtLayout::CardHeader,
        SmartArtLayout::CardTall,
        SmartArtLayout::CardRows,
        SmartArtLayout::TimelineStems,
        SmartArtLayout::TimelineCircles,
        SmartArtLayout::TimelineZigzag,
        SmartArtLayout::TimelineLine,
        SmartArtLayout::CardEmphasis,
        SmartArtLayout::CardMarkerRows,
        SmartArtLayout::CardSquares,
        SmartArtLayout::PictureCaptions,
        SmartArtLayout::PictureGrid,
        SmartArtLayout::PictureCircles,
        SmartArtLayout::PictureRows,
        SmartArtLayout::PictureStrips,
        SmartArtLayout::PictureStaggered,
        SmartArtLayout::MatrixTitled,
        SmartArtLayout::MatrixGrid,
        SmartArtLayout::MatrixCycle,
        SmartArtLayout::PyramidInverted,
        SmartArtLayout::PyramidList,
        SmartArtLayout::PyramidSegmented,
        SmartArtLayout::Balance,
        SmartArtLayout::Funnel,
        SmartArtLayout::Gears,
        SmartArtLayout::ArrowsOpposing,
        SmartArtLayout::Equation,
        SmartArtLayout::VennLinear,
        SmartArtLayout::Target,
        SmartArtLayout::RadialDiverging,
        SmartArtLayout::OrgChart,
        SmartArtLayout::HierarchyHorizontal,
        SmartArtLayout::HierarchyList,
        SmartArtLayout::HierarchyTable,
        SmartArtLayout::HierarchyCircles,
        SmartArtLayout::LinedList,
        SmartArtLayout::CycleText,
        SmartArtLayout::CycleContinuous,
        SmartArtLayout::CycleSegmented,
        SmartArtLayout::CycleMultidirectional,
        SmartArtLayout::CycleDots,
        SmartArtLayout::CycleRadial,
        SmartArtLayout::Chevrons,
        SmartArtLayout::ChevronList,
        SmartArtLayout::StepUp,
        SmartArtLayout::ProcessVertical,
        SmartArtLayout::ProcessBending,
        SmartArtLayout::ProcessContinuous,
        SmartArtLayout::ArrowUpward,
        SmartArtLayout::ProcessArrows,
        SmartArtLayout::ListHexagon,
        SmartArtLayout::ListVerticalBullet,
        SmartArtLayout::ListGrouped,
        SmartArtLayout::ListTrapezoid,
        SmartArtLayout::ListDescending,
        SmartArtLayout::ListTable,
        SmartArtLayout::ListHexagonAlternating,
        SmartArtLayout::ListVerticalBox,
        SmartArtLayout::ListHorizontalBullet,
        SmartArtLayout::ListSquareAccent,
        SmartArtLayout::ListStacked,
        SmartArtLayout::ListVerticalBlock,
        SmartArtLayout::ListVerticalArrow,
        SmartArtLayout::ListTarget,
        SmartArtLayout::ListVerticalCircle,
        SmartArtLayout::ListVaryingWidth,
        SmartArtLayout::ListTab,
        SmartArtLayout::ListCurved,
        SmartArtLayout::ProcessAccent,
        SmartArtLayout::ProcessAlternatingFlow,
        SmartArtLayout::ChevronsClosed,
        SmartArtLayout::ChevronsDetailed,
        SmartArtLayout::ProcessCircle,
        SmartArtLayout::ProcessSegmented,
        SmartArtLayout::ProcessSnake,
        SmartArtLayout::ProcessBendingVertical,
        SmartArtLayout::StepDown,
        SmartArtLayout::ProcessIncreasingArrows,
        SmartArtLayout::ProcessContinuousArrow,
        SmartArtLayout::ProcessList,
        SmartArtLayout::ArrowDownward,
        SmartArtLayout::ProcessCirclesIncreasing,
        SmartArtLayout::CycleBlock,
        SmartArtLayout::CycleNondirectional,
        SmartArtLayout::CyclePie,
        SmartArtLayout::CycleHexagonRadial,
        SmartArtLayout::OrgChartHalfCircle,
        SmartArtLayout::HierarchyLabeled,
        SmartArtLayout::HierarchyLabeledHorizontal,
        SmartArtLayout::HierarchyArchitecture,
        SmartArtLayout::HierarchyTableHorizontal,
        SmartArtLayout::ArrowsConverging,
        SmartArtLayout::ArrowsDiverging,
        SmartArtLayout::ArrowsCounterbalance,
        SmartArtLayout::VennRadial,
        SmartArtLayout::VennStacked,
        SmartArtLayout::TargetNested,
        SmartArtLayout::CircleRelationship,
        SmartArtLayout::PlusMinus,
        SmartArtLayout::TextConverging,
        SmartArtLayout::RadialList,
        SmartArtLayout::EquationVertical,
        SmartArtLayout::RingsInterconnected,
        SmartArtLayout::RadialCluster,
        SmartArtLayout::RadialConverging,
        SmartArtLayout::TimelineCircleAccent,
    ];

    /// The id commands use (`basicList`, `process`…).
    pub fn id(self) -> &'static str {
        match self {
            SmartArtLayout::BasicList => "basicList",
            SmartArtLayout::Process => "process",
            SmartArtLayout::Cycle => "cycle",
            SmartArtLayout::Hierarchy => "hierarchy",
            SmartArtLayout::Pyramid => "pyramid",
            SmartArtLayout::Radial => "radial",
            SmartArtLayout::Matrix => "matrix",
            SmartArtLayout::Venn => "venn",
            SmartArtLayout::TimelineDots => "timelineDots",
            SmartArtLayout::TimelineNumbered => "timelineNumbered",
            SmartArtLayout::TimelineArrow => "timelineArrow",
            SmartArtLayout::TimelineVertical => "timelineVertical",
            SmartArtLayout::TeamCircles => "teamCircles",
            SmartArtLayout::TeamCards => "teamCards",
            SmartArtLayout::TeamPills => "teamPills",
            SmartArtLayout::CardNumbered => "cardNumbered",
            SmartArtLayout::CardBigNumbers => "cardBigNumbers",
            SmartArtLayout::CardHeader => "cardHeader",
            SmartArtLayout::CardTall => "cardTall",
            SmartArtLayout::CardRows => "cardRows",
            SmartArtLayout::TimelineStems => "timelineStems",
            SmartArtLayout::TimelineCircles => "timelineCircles",
            SmartArtLayout::TimelineZigzag => "timelineZigzag",
            SmartArtLayout::TimelineLine => "timelineLine",
            SmartArtLayout::CardEmphasis => "cardEmphasis",
            SmartArtLayout::CardMarkerRows => "cardMarkerRows",
            SmartArtLayout::CardSquares => "cardSquares",
            SmartArtLayout::PictureCaptions => "pictureCaptions",
            SmartArtLayout::PictureGrid => "pictureGrid",
            SmartArtLayout::PictureCircles => "pictureCircles",
            SmartArtLayout::PictureRows => "pictureRows",
            SmartArtLayout::PictureStrips => "pictureStrips",
            SmartArtLayout::PictureStaggered => "pictureStaggered",
            SmartArtLayout::MatrixTitled => "matrixTitled",
            SmartArtLayout::MatrixGrid => "matrixGrid",
            SmartArtLayout::MatrixCycle => "matrixCycle",
            SmartArtLayout::PyramidInverted => "pyramidInverted",
            SmartArtLayout::PyramidList => "pyramidList",
            SmartArtLayout::PyramidSegmented => "pyramidSegmented",
            SmartArtLayout::Balance => "balance",
            SmartArtLayout::Funnel => "funnel",
            SmartArtLayout::Gears => "gears",
            SmartArtLayout::ArrowsOpposing => "arrowsOpposing",
            SmartArtLayout::Equation => "equation",
            SmartArtLayout::VennLinear => "vennLinear",
            SmartArtLayout::Target => "target",
            SmartArtLayout::RadialDiverging => "radialDiverging",
            SmartArtLayout::OrgChart => "orgChart",
            SmartArtLayout::HierarchyHorizontal => "hierarchyHorizontal",
            SmartArtLayout::HierarchyList => "hierarchyList",
            SmartArtLayout::HierarchyTable => "hierarchyTable",
            SmartArtLayout::HierarchyCircles => "hierarchyCircles",
            SmartArtLayout::LinedList => "linedList",
            SmartArtLayout::CycleText => "cycleText",
            SmartArtLayout::CycleContinuous => "cycleContinuous",
            SmartArtLayout::CycleSegmented => "cycleSegmented",
            SmartArtLayout::CycleMultidirectional => "cycleMultidirectional",
            SmartArtLayout::CycleDots => "cycleDots",
            SmartArtLayout::CycleRadial => "cycleRadial",
            SmartArtLayout::Chevrons => "chevrons",
            SmartArtLayout::ChevronList => "chevronList",
            SmartArtLayout::StepUp => "stepUp",
            SmartArtLayout::ProcessVertical => "processVertical",
            SmartArtLayout::ProcessBending => "processBending",
            SmartArtLayout::ProcessContinuous => "processContinuous",
            SmartArtLayout::ArrowUpward => "arrowUpward",
            SmartArtLayout::ProcessArrows => "processArrows",
            SmartArtLayout::ListHexagon => "listHexagon",
            SmartArtLayout::ListVerticalBullet => "listVerticalBullet",
            SmartArtLayout::ListGrouped => "listGrouped",
            SmartArtLayout::ListTrapezoid => "listTrapezoid",
            SmartArtLayout::ListDescending => "listDescending",
            SmartArtLayout::ListTable => "listTable",
            SmartArtLayout::ListHexagonAlternating => "listHexagonAlternating",
            SmartArtLayout::ListVerticalBox => "listVerticalBox",
            SmartArtLayout::ListHorizontalBullet => "listHorizontalBullet",
            SmartArtLayout::ListSquareAccent => "listSquareAccent",
            SmartArtLayout::ListStacked => "listStacked",
            SmartArtLayout::ListVerticalBlock => "listVerticalBlock",
            SmartArtLayout::ListVerticalArrow => "listVerticalArrow",
            SmartArtLayout::ListTarget => "listTarget",
            SmartArtLayout::ListVerticalCircle => "listVerticalCircle",
            SmartArtLayout::ListVaryingWidth => "listVaryingWidth",
            SmartArtLayout::ListTab => "listTab",
            SmartArtLayout::ListCurved => "listCurved",
            SmartArtLayout::ProcessAccent => "processAccent",
            SmartArtLayout::ProcessAlternatingFlow => "processAlternatingFlow",
            SmartArtLayout::ChevronsClosed => "chevronsClosed",
            SmartArtLayout::ChevronsDetailed => "chevronsDetailed",
            SmartArtLayout::ProcessCircle => "processCircle",
            SmartArtLayout::ProcessSegmented => "processSegmented",
            SmartArtLayout::ProcessSnake => "processSnake",
            SmartArtLayout::ProcessBendingVertical => "processBendingVertical",
            SmartArtLayout::StepDown => "stepDown",
            SmartArtLayout::ProcessIncreasingArrows => "processIncreasingArrows",
            SmartArtLayout::ProcessContinuousArrow => "processContinuousArrow",
            SmartArtLayout::ProcessList => "processList",
            SmartArtLayout::ArrowDownward => "arrowDownward",
            SmartArtLayout::ProcessCirclesIncreasing => "processCirclesIncreasing",
            SmartArtLayout::CycleBlock => "cycleBlock",
            SmartArtLayout::CycleNondirectional => "cycleNondirectional",
            SmartArtLayout::CyclePie => "cyclePie",
            SmartArtLayout::CycleHexagonRadial => "cycleHexagonRadial",
            SmartArtLayout::OrgChartHalfCircle => "orgChartHalfCircle",
            SmartArtLayout::HierarchyLabeled => "hierarchyLabeled",
            SmartArtLayout::HierarchyLabeledHorizontal => "hierarchyLabeledHorizontal",
            SmartArtLayout::HierarchyArchitecture => "hierarchyArchitecture",
            SmartArtLayout::HierarchyTableHorizontal => "hierarchyTableHorizontal",
            SmartArtLayout::ArrowsConverging => "arrowsConverging",
            SmartArtLayout::ArrowsDiverging => "arrowsDiverging",
            SmartArtLayout::ArrowsCounterbalance => "arrowsCounterbalance",
            SmartArtLayout::VennRadial => "vennRadial",
            SmartArtLayout::VennStacked => "vennStacked",
            SmartArtLayout::TargetNested => "targetNested",
            SmartArtLayout::CircleRelationship => "circleRelationship",
            SmartArtLayout::PlusMinus => "plusMinus",
            SmartArtLayout::TextConverging => "textConverging",
            SmartArtLayout::RadialList => "radialList",
            SmartArtLayout::EquationVertical => "equationVertical",
            SmartArtLayout::RingsInterconnected => "ringsInterconnected",
            SmartArtLayout::RadialCluster => "radialCluster",
            SmartArtLayout::RadialConverging => "radialConverging",
            SmartArtLayout::TimelineCircleAccent => "timelineCircleAccent",
        }
    }

    /// The layout an id names (any ASCII case, ignoring hyphens, underscores and spaces;
    /// `list` is the basic list; `basic process` or `process` is process, etc.).
    pub fn from_id(id: &str) -> Option<SmartArtLayout> {
        let norm = id.chars().filter(|c| !c.is_whitespace() && *c != '-' && *c != '_').collect::<String>();
        if norm.eq_ignore_ascii_case("list") {
            return Some(SmartArtLayout::BasicList);
        }
        SmartArtLayout::ALL.into_iter().find(|l| l.id().eq_ignore_ascii_case(&norm) || l.label().replace(' ', "").eq_ignore_ascii_case(&norm))
    }

    /// The name shown for it.
    pub fn label(self) -> &'static str {
        match self {
            SmartArtLayout::BasicList => "Basic List",
            SmartArtLayout::Process => "Basic Process",
            SmartArtLayout::Cycle => "Basic Cycle",
            SmartArtLayout::Hierarchy => "Hierarchy",
            SmartArtLayout::Pyramid => "Basic Pyramid",
            SmartArtLayout::Radial => "Basic Radial",
            SmartArtLayout::Matrix => "Basic Matrix",
            SmartArtLayout::Venn => "Basic Venn",
            SmartArtLayout::TimelineDots => "Dot Timeline",
            SmartArtLayout::TimelineNumbered => "Numbered Timeline",
            SmartArtLayout::TimelineArrow => "Arrow Timeline",
            SmartArtLayout::TimelineVertical => "Vertical Timeline",
            SmartArtLayout::TeamCircles => "Team Row",
            SmartArtLayout::TeamCards => "Team Cards",
            SmartArtLayout::TeamPills => "Team Pills",
            SmartArtLayout::CardNumbered => "Numbered Columns",
            SmartArtLayout::CardBigNumbers => "Big Number Columns",
            SmartArtLayout::CardHeader => "Header Cards",
            SmartArtLayout::CardTall => "Tall Cards",
            SmartArtLayout::CardRows => "Numbered Rows",
            SmartArtLayout::TimelineStems => "Stem Timeline",
            SmartArtLayout::TimelineCircles => "Overlapping Circles Timeline",
            SmartArtLayout::TimelineZigzag => "Zigzag Timeline",
            SmartArtLayout::TimelineLine => "Line Timeline",
            SmartArtLayout::CardEmphasis => "Emphasis Columns",
            SmartArtLayout::CardMarkerRows => "Marker Rows",
            SmartArtLayout::CardSquares => "Numbered Squares",
            SmartArtLayout::PictureCaptions => "Picture Captions",
            SmartArtLayout::PictureGrid => "Picture Grid",
            SmartArtLayout::PictureCircles => "Circle Picture Cards",
            SmartArtLayout::PictureRows => "Picture Rows",
            SmartArtLayout::PictureStrips => "Picture Strips",
            SmartArtLayout::PictureStaggered => "Staggered Pictures",
            SmartArtLayout::MatrixTitled => "Titled Matrix",
            SmartArtLayout::MatrixGrid => "Grid Matrix",
            SmartArtLayout::MatrixCycle => "Cycle Matrix",
            SmartArtLayout::PyramidInverted => "Inverted Pyramid",
            SmartArtLayout::PyramidList => "Pyramid List",
            SmartArtLayout::PyramidSegmented => "Segmented Pyramid",
            SmartArtLayout::Balance => "Balance",
            SmartArtLayout::Funnel => "Funnel",
            SmartArtLayout::Gears => "Gears",
            SmartArtLayout::ArrowsOpposing => "Opposing Arrows",
            SmartArtLayout::Equation => "Equation",
            SmartArtLayout::VennLinear => "Linear Venn",
            SmartArtLayout::Target => "Target",
            SmartArtLayout::RadialDiverging => "Diverging Radial",
            SmartArtLayout::OrgChart => "Organization Chart",
            SmartArtLayout::HierarchyHorizontal => "Horizontal Hierarchy",
            SmartArtLayout::HierarchyList => "Hierarchy List",
            SmartArtLayout::HierarchyTable => "Table Hierarchy",
            SmartArtLayout::HierarchyCircles => "Circle Hierarchy",
            SmartArtLayout::LinedList => "Lined List",
            SmartArtLayout::CycleText => "Text Cycle",
            SmartArtLayout::CycleContinuous => "Continuous Cycle",
            SmartArtLayout::CycleSegmented => "Segmented Cycle",
            SmartArtLayout::CycleMultidirectional => "Multidirectional Cycle",
            SmartArtLayout::CycleDots => "Dot Cycle",
            SmartArtLayout::CycleRadial => "Radial Cycle",
            SmartArtLayout::Chevrons => "Chevron Process",
            SmartArtLayout::ChevronList => "Vertical Chevron List",
            SmartArtLayout::StepUp => "Step Up Process",
            SmartArtLayout::ProcessVertical => "Vertical Process",
            SmartArtLayout::ProcessBending => "Bending Process",
            SmartArtLayout::ProcessContinuous => "Continuous Block Process",
            SmartArtLayout::ArrowUpward => "Upward Arrow",
            SmartArtLayout::ProcessArrows => "Process Arrows",
            SmartArtLayout::ListHexagon => "Hexagon List",
            SmartArtLayout::ListVerticalBullet => "Vertical Bullet List",
            SmartArtLayout::ListGrouped => "Grouped List",
            SmartArtLayout::ListTrapezoid => "Trapezoid List",
            SmartArtLayout::ListDescending => "Descending Blocks",
            SmartArtLayout::ListTable => "Table List",
            SmartArtLayout::ListHexagonAlternating => "Alternating Hexagons",
            SmartArtLayout::ListVerticalBox => "Vertical Box List",
            SmartArtLayout::ListHorizontalBullet => "Horizontal Bullet List",
            SmartArtLayout::ListSquareAccent => "Square Accent List",
            SmartArtLayout::ListStacked => "Stacked List",
            SmartArtLayout::ListVerticalBlock => "Vertical Block List",
            SmartArtLayout::ListVerticalArrow => "Vertical Arrow List",
            SmartArtLayout::ListTarget => "Target List",
            SmartArtLayout::ListVerticalCircle => "Vertical Circle List",
            SmartArtLayout::ListVaryingWidth => "Varying Width List",
            SmartArtLayout::ListTab => "Tab List",
            SmartArtLayout::ListCurved => "Vertical Curved List",
            SmartArtLayout::ProcessAccent => "Accent Process",
            SmartArtLayout::ProcessAlternatingFlow => "Alternating Flow",
            SmartArtLayout::ChevronsClosed => "Closed Chevron Process",
            SmartArtLayout::ChevronsDetailed => "Basic Chevron Process",
            SmartArtLayout::ProcessCircle => "Circle Process",
            SmartArtLayout::ProcessSegmented => "Segmented Process",
            SmartArtLayout::ProcessSnake => "Snaking Process",
            SmartArtLayout::ProcessBendingVertical => "Vertical Bending Process",
            SmartArtLayout::StepDown => "Step Down Process",
            SmartArtLayout::ProcessIncreasingArrows => "Increasing Arrows Process",
            SmartArtLayout::ProcessContinuousArrow => "Continuous Arrow Process",
            SmartArtLayout::ProcessList => "Process List",
            SmartArtLayout::ArrowDownward => "Downward Arrow",
            SmartArtLayout::ProcessCirclesIncreasing => "Increasing Circle Process",
            SmartArtLayout::CycleBlock => "Block Cycle",
            SmartArtLayout::CycleNondirectional => "Nondirectional Cycle",
            SmartArtLayout::CyclePie => "Basic Pie",
            SmartArtLayout::CycleHexagonRadial => "Hexagon Radial",
            SmartArtLayout::OrgChartHalfCircle => "Half Circle Organization Chart",
            SmartArtLayout::HierarchyLabeled => "Labeled Hierarchy",
            SmartArtLayout::HierarchyLabeledHorizontal => "Horizontal Labeled Hierarchy",
            SmartArtLayout::HierarchyArchitecture => "Architecture Layout",
            SmartArtLayout::HierarchyTableHorizontal => "Horizontal Multi-Level Hierarchy",
            SmartArtLayout::ArrowsConverging => "Converging Arrows",
            SmartArtLayout::ArrowsDiverging => "Diverging Arrows",
            SmartArtLayout::ArrowsCounterbalance => "Counterbalance Arrows",
            SmartArtLayout::VennRadial => "Radial Venn",
            SmartArtLayout::VennStacked => "Stacked Venn",
            SmartArtLayout::TargetNested => "Nested Target",
            SmartArtLayout::CircleRelationship => "Circle Relationship",
            SmartArtLayout::PlusMinus => "Plus and Minus",
            SmartArtLayout::TextConverging => "Converging Text",
            SmartArtLayout::RadialList => "Radial List",
            SmartArtLayout::EquationVertical => "Vertical Equation",
            SmartArtLayout::RingsInterconnected => "Interconnected Rings",
            SmartArtLayout::RadialCluster => "Radial Cluster",
            SmartArtLayout::RadialConverging => "Converging Radial",
            SmartArtLayout::TimelineCircleAccent => "Circle Accent Timeline",
        }
    }

    /// The gallery category it's shown under.
    pub fn category(self) -> &'static str {
        match self {
            SmartArtLayout::BasicList => "List",
            SmartArtLayout::Process => "Process",
            SmartArtLayout::Cycle => "Cycle",
            SmartArtLayout::Hierarchy => "Hierarchy",
            SmartArtLayout::Pyramid => "Pyramid",
            SmartArtLayout::Radial | SmartArtLayout::Venn => "Relationship",
            SmartArtLayout::Matrix => "Matrix",
            SmartArtLayout::TimelineDots
            | SmartArtLayout::TimelineNumbered
            | SmartArtLayout::TimelineArrow
            | SmartArtLayout::TimelineVertical
            | SmartArtLayout::TimelineStems
            | SmartArtLayout::TimelineCircles
            | SmartArtLayout::TimelineZigzag
            | SmartArtLayout::TimelineLine => "Timeline",
            SmartArtLayout::TeamCircles | SmartArtLayout::TeamCards | SmartArtLayout::TeamPills => "Meet the Team",
            SmartArtLayout::CardNumbered
            | SmartArtLayout::CardBigNumbers
            | SmartArtLayout::CardHeader
            | SmartArtLayout::CardTall
            | SmartArtLayout::CardRows
            | SmartArtLayout::CardEmphasis
            | SmartArtLayout::CardMarkerRows
            | SmartArtLayout::CardSquares => "Text Cards",
            SmartArtLayout::PictureCaptions
            | SmartArtLayout::PictureGrid
            | SmartArtLayout::PictureCircles
            | SmartArtLayout::PictureRows
            | SmartArtLayout::PictureStrips
            | SmartArtLayout::PictureStaggered => "Picture",
            SmartArtLayout::MatrixTitled | SmartArtLayout::MatrixGrid | SmartArtLayout::MatrixCycle => "Matrix",
            SmartArtLayout::PyramidInverted | SmartArtLayout::PyramidList | SmartArtLayout::PyramidSegmented => "Pyramid",
            SmartArtLayout::Balance
            | SmartArtLayout::Funnel
            | SmartArtLayout::Gears
            | SmartArtLayout::ArrowsOpposing
            | SmartArtLayout::Equation
            | SmartArtLayout::VennLinear
            | SmartArtLayout::Target
            | SmartArtLayout::RadialDiverging => "Relationship",
            SmartArtLayout::OrgChart
            | SmartArtLayout::HierarchyHorizontal
            | SmartArtLayout::HierarchyList
            | SmartArtLayout::HierarchyTable
            | SmartArtLayout::HierarchyCircles
            | SmartArtLayout::LinedList => "Hierarchy",
            SmartArtLayout::CycleText
            | SmartArtLayout::CycleContinuous
            | SmartArtLayout::CycleSegmented
            | SmartArtLayout::CycleMultidirectional
            | SmartArtLayout::CycleDots
            | SmartArtLayout::CycleRadial => "Cycle",
            SmartArtLayout::Chevrons
            | SmartArtLayout::ChevronList
            | SmartArtLayout::StepUp
            | SmartArtLayout::ProcessVertical
            | SmartArtLayout::ProcessBending
            | SmartArtLayout::ProcessContinuous
            | SmartArtLayout::ArrowUpward
            | SmartArtLayout::ProcessArrows => "Process",
            SmartArtLayout::ListHexagon
            | SmartArtLayout::ListVerticalBullet
            | SmartArtLayout::ListGrouped
            | SmartArtLayout::ListTrapezoid
            | SmartArtLayout::ListDescending
            | SmartArtLayout::ListTable => "List",
            SmartArtLayout::ListHexagonAlternating
            | SmartArtLayout::ListVerticalBox
            | SmartArtLayout::ListHorizontalBullet
            | SmartArtLayout::ListSquareAccent
            | SmartArtLayout::ListStacked
            | SmartArtLayout::ListVerticalBlock
            | SmartArtLayout::ListVerticalArrow
            | SmartArtLayout::ListTarget
            | SmartArtLayout::ListVerticalCircle
            | SmartArtLayout::ListVaryingWidth
            | SmartArtLayout::ListTab
            | SmartArtLayout::ListCurved => "List",
            SmartArtLayout::ProcessAccent
            | SmartArtLayout::ProcessAlternatingFlow
            | SmartArtLayout::ChevronsClosed
            | SmartArtLayout::ChevronsDetailed
            | SmartArtLayout::ProcessCircle
            | SmartArtLayout::ProcessSegmented
            | SmartArtLayout::ProcessSnake
            | SmartArtLayout::ProcessBendingVertical
            | SmartArtLayout::StepDown
            | SmartArtLayout::ProcessIncreasingArrows
            | SmartArtLayout::ProcessContinuousArrow
            | SmartArtLayout::ProcessList
            | SmartArtLayout::ArrowDownward
            | SmartArtLayout::ProcessCirclesIncreasing => "Process",
            SmartArtLayout::CycleBlock | SmartArtLayout::CycleNondirectional | SmartArtLayout::CyclePie | SmartArtLayout::CycleHexagonRadial => {
                "Cycle"
            }
            SmartArtLayout::OrgChartHalfCircle
            | SmartArtLayout::HierarchyLabeled
            | SmartArtLayout::HierarchyLabeledHorizontal
            | SmartArtLayout::HierarchyArchitecture
            | SmartArtLayout::HierarchyTableHorizontal => "Hierarchy",
            SmartArtLayout::ArrowsConverging
            | SmartArtLayout::ArrowsDiverging
            | SmartArtLayout::ArrowsCounterbalance
            | SmartArtLayout::VennRadial
            | SmartArtLayout::VennStacked
            | SmartArtLayout::TargetNested
            | SmartArtLayout::CircleRelationship
            | SmartArtLayout::PlusMinus
            | SmartArtLayout::TextConverging
            | SmartArtLayout::RadialList
            | SmartArtLayout::EquationVertical
            | SmartArtLayout::RingsInterconnected
            | SmartArtLayout::RadialCluster
            | SmartArtLayout::RadialConverging => "Relationship",
            SmartArtLayout::TimelineCircleAccent => "Timeline",
        }
    }

    /// What it's for, in a sentence (the picker shows it).
    pub fn description(self) -> &'static str {
        match self {
            SmartArtLayout::BasicList => "Shows items of equal weight as blocks in rows.",
            SmartArtLayout::Process => "Shows steps in order, left to right, with arrows between them.",
            SmartArtLayout::Cycle => "Shows steps that repeat, around a circle.",
            SmartArtLayout::Hierarchy => "Shows a tree: each item below the item it belongs to.",
            SmartArtLayout::Pyramid => "Shows parts that build on each other, largest at the bottom.",
            SmartArtLayout::Radial => "Shows items around a central idea: the first item is the centre.",
            SmartArtLayout::Matrix => "Shows parts of a whole as quadrants.",
            SmartArtLayout::Venn => "Shows overlapping ideas as overlapping circles.",
            SmartArtLayout::TimelineDots => "Shows events in time as dots on a line, the text above and below in turn.",
            SmartArtLayout::TimelineNumbered => "Shows stages in order as numbered circles on a line.",
            SmartArtLayout::TimelineArrow => "Shows milestones as dots along an arrow.",
            SmartArtLayout::TimelineVertical => "Shows events from top to bottom, the text on either side of a line.",
            SmartArtLayout::TeamCircles => "Introduces people in a row: a name and role under each picture.",
            SmartArtLayout::TeamCards => "Introduces people as stacked cards with a picture and text.",
            SmartArtLayout::TeamPills => "Introduces people as tall cards: text on top, a picture at the bottom.",
            SmartArtLayout::CardNumbered => "Shows a few ideas as numbered columns of text.",
            SmartArtLayout::CardBigNumbers => "Shows a few ideas under large numbers.",
            SmartArtLayout::CardHeader => "Shows a few ideas as cards with a coloured heading.",
            SmartArtLayout::CardTall => "Shows a few ideas as tall tinted cards, numbered.",
            SmartArtLayout::CardRows => "Shows ideas as numbered rows of text.",
            SmartArtLayout::TimelineStems => "Shows events as stems off a line, the text at the stem ends above and below in turn.",
            SmartArtLayout::TimelineCircles => "Shows stages as overlapping numbered circles, the middle ones largest.",
            SmartArtLayout::TimelineZigzag => "Shows steps as numbered circles in a zigzag, joined by slanted strokes.",
            SmartArtLayout::TimelineLine => "Shows events as small dots on a thin line, the text under each dot.",
            SmartArtLayout::CardEmphasis => "Shows a few ideas as columns, each ending in a large figure or phrase.",
            SmartArtLayout::CardMarkerRows => "Shows ideas as rows of text with a square marker at the right.",
            SmartArtLayout::CardSquares => "Shows a few ideas as tinted numbered squares.",
            SmartArtLayout::PictureCaptions => "Shows pictures in a row with a caption under each.",
            SmartArtLayout::PictureGrid => "Shows pictures as a grid of cards with a coloured caption.",
            SmartArtLayout::PictureCircles => "Shows round pictures over tinted cards with the text inside.",
            SmartArtLayout::PictureRows => "Shows rows, each with a picture and text beside it.",
            SmartArtLayout::PictureStrips => "Shows tall pictures with a coloured caption at the bottom.",
            SmartArtLayout::PictureStaggered => "Shows pictures and their text above and below in turn.",
            SmartArtLayout::MatrixTitled => "Shows parts of a whole around a title: the first item is the title.",
            SmartArtLayout::MatrixGrid => "Shows items as quadrants on two axes.",
            SmartArtLayout::MatrixCycle => "Shows parts of a cycle as wedges, with details in the corners.",
            SmartArtLayout::PyramidInverted => "Shows a narrowing from many to few, the widest at the top.",
            SmartArtLayout::PyramidList => "Shows parts that build on each other as bars across a pyramid.",
            SmartArtLayout::PyramidSegmented => "Shows parts of a whole as pieces of a triangle.",
            SmartArtLayout::Balance => "Compares two sides on a scale, tipped towards the heavier.",
            SmartArtLayout::Funnel => "Shows things that combine into one result through a funnel.",
            SmartArtLayout::Gears => "Shows parts that work together as interlocking gears.",
            SmartArtLayout::ArrowsOpposing => "Shows opposing forces or trends as up and down arrows.",
            SmartArtLayout::Equation => "Shows parts that add up to a result: A + B = C.",
            SmartArtLayout::VennLinear => "Shows overlapping ideas as circles in a row.",
            SmartArtLayout::Target => "Shows layers from the outside in as rings of a target.",
            SmartArtLayout::RadialDiverging => "Shows ideas that spread out from a central one.",
            SmartArtLayout::OrgChart => "Shows an organisation: lower levels hang under their manager.",
            SmartArtLayout::HierarchyHorizontal => "Shows a tree from left to right.",
            SmartArtLayout::HierarchyList => "Shows groups with their members listed underneath.",
            SmartArtLayout::HierarchyTable => "Shows a tree as stacked blocks, each as wide as its parts.",
            SmartArtLayout::HierarchyCircles => "Shows a tree of circles.",
            SmartArtLayout::LinedList => "Shows groups with their members as ruled rows.",
            SmartArtLayout::CycleText => "Shows steps that repeat as text joined by curved arrows.",
            SmartArtLayout::CycleContinuous => "Shows a continuing cycle as blocks on an arrow ring.",
            SmartArtLayout::CycleSegmented => "Shows the parts of a cycle as segments of a ring.",
            SmartArtLayout::CycleMultidirectional => "Shows steps that can go either way round.",
            SmartArtLayout::CycleDots => "Shows steps that repeat as dots on a ring.",
            SmartArtLayout::CycleRadial => "Shows a cycle around a central idea.",
            SmartArtLayout::Chevrons => "Shows steps in order as chevrons.",
            SmartArtLayout::ChevronList => "Shows steps top to bottom with details beside each.",
            SmartArtLayout::StepUp => "Shows steps that build up, rising like stairs.",
            SmartArtLayout::ProcessVertical => "Shows steps in order from top to bottom.",
            SmartArtLayout::ProcessBending => "Shows many steps in rows that bend to the next line.",
            SmartArtLayout::ProcessContinuous => "Shows steps on one continuous arrow.",
            SmartArtLayout::ArrowUpward => "Shows growth or progress along a rising arrow.",
            SmartArtLayout::ProcessArrows => "Shows steps as circles over arrows that hold the details.",
            SmartArtLayout::ListHexagon => "Shows items as hexagons fitted together.",
            SmartArtLayout::ListVerticalBullet => "Shows headed groups of text, one under another.",
            SmartArtLayout::ListGrouped => "Shows groups with their members as boxes inside.",
            SmartArtLayout::ListTrapezoid => "Shows items side by side as trapezoids.",
            SmartArtLayout::ListDescending => "Shows items in order of size, the largest first.",
            SmartArtLayout::ListTable => "Shows a heading over items side by side.",
            SmartArtLayout::ListHexagonAlternating => "Shows items and their details as hexagons in a zigzag.",
            SmartArtLayout::ListVerticalBox => "Shows headed groups of text as boxes with a tab.",
            SmartArtLayout::ListHorizontalBullet => "Shows headed groups of bullets side by side.",
            SmartArtLayout::ListSquareAccent => "Shows headed groups of text with square markers.",
            SmartArtLayout::ListStacked => "Shows groups of text under circled headings.",
            SmartArtLayout::ListVerticalBlock => "Shows groups of text as blocks beside their heading.",
            SmartArtLayout::ListVerticalArrow => "Shows headings that point to their details.",
            SmartArtLayout::ListTarget => "Shows layers from the outside in with text beside each.",
            SmartArtLayout::ListVerticalCircle => "Shows items in order down a line of circles.",
            SmartArtLayout::ListVaryingWidth => "Shows items as centred bars that fit their text.",
            SmartArtLayout::ListTab => "Shows headings with their details as ruled rows.",
            SmartArtLayout::ListCurved => "Shows items along a curve.",
            SmartArtLayout::ProcessAccent => "Shows steps in order with details under each.",
            SmartArtLayout::ProcessAlternatingFlow => "Shows steps in order with details above and below in turn.",
            SmartArtLayout::ChevronsClosed => "Shows steps in order from a starting point.",
            SmartArtLayout::ChevronsDetailed => "Shows steps in order as chevrons with details below.",
            SmartArtLayout::ProcessCircle => "Shows steps in order as circles.",
            SmartArtLayout::ProcessSegmented => "Shows steps in order, each split into its parts.",
            SmartArtLayout::ProcessSnake => "Shows many steps in rows that wind back and forth.",
            SmartArtLayout::ProcessBendingVertical => "Shows many steps in columns that wind down and up.",
            SmartArtLayout::StepDown => "Shows steps that lead down one after another.",
            SmartArtLayout::ProcessIncreasingArrows => "Shows stages that start in turn and run on.",
            SmartArtLayout::ProcessContinuousArrow => "Shows steps along one continuous arrow.",
            SmartArtLayout::ProcessList => "Shows steps in order, each with its tasks listed.",
            SmartArtLayout::ArrowDownward => "Shows decline or steps down along a falling arrow.",
            SmartArtLayout::ProcessCirclesIncreasing => "Shows progress that builds up step by step.",
            SmartArtLayout::CycleBlock => "Shows steps that repeat as blocks joined by curved arrows.",
            SmartArtLayout::CycleNondirectional => "Shows parts of a cycle with no set direction.",
            SmartArtLayout::CyclePie => "Shows parts of a whole as slices of a pie.",
            SmartArtLayout::CycleHexagonRadial => "Shows items round a central idea as hexagons.",
            SmartArtLayout::OrgChartHalfCircle => "Shows an organisation with each person in open rings.",
            SmartArtLayout::HierarchyLabeled => "Shows a tree with a named band for each level.",
            SmartArtLayout::HierarchyLabeledHorizontal => "Shows a tree from left to right with a named column per level.",
            SmartArtLayout::HierarchyArchitecture => "Shows layers that build on each other, the base at the bottom.",
            SmartArtLayout::HierarchyTableHorizontal => "Shows a tree from left to right as stacked blocks.",
            SmartArtLayout::ArrowsConverging => "Shows ideas that come together.",
            SmartArtLayout::ArrowsDiverging => "Shows ideas that move apart.",
            SmartArtLayout::ArrowsCounterbalance => "Shows two forces that balance each other.",
            SmartArtLayout::VennRadial => "Shows ideas that overlap a central one.",
            SmartArtLayout::VennStacked => "Shows parts that grow from one another.",
            SmartArtLayout::TargetNested => "Shows layers inside each other.",
            SmartArtLayout::CircleRelationship => "Shows parts that belong to a whole.",
            SmartArtLayout::PlusMinus => "Shows the pros and cons of an idea.",
            SmartArtLayout::TextConverging => "Shows parts that combine into one result.",
            SmartArtLayout::RadialList => "Shows items that come from a central idea.",
            SmartArtLayout::EquationVertical => "Shows parts that add up to a result, top to bottom.",
            SmartArtLayout::RingsInterconnected => "Shows ideas that link together.",
            SmartArtLayout::RadialCluster => "Shows ideas linked to a central one.",
            SmartArtLayout::RadialConverging => "Shows ideas that lead to a central one.",
            SmartArtLayout::TimelineCircleAccent => "Shows events in time with the steps of each.",
        }
    }
    /// Whether items under a top item are drawn as shapes of their own (trees, a centre with
    /// items round it, groups of boxes) rather than as lines in their item's text.
    pub fn children_are_shapes(self) -> bool {
        matches!(
            self,
            SmartArtLayout::Hierarchy
                | SmartArtLayout::Radial
                | SmartArtLayout::MatrixTitled
                | SmartArtLayout::RadialDiverging
                | SmartArtLayout::OrgChart
                | SmartArtLayout::HierarchyHorizontal
                | SmartArtLayout::HierarchyList
                | SmartArtLayout::HierarchyTable
                | SmartArtLayout::HierarchyCircles
                | SmartArtLayout::LinedList
                | SmartArtLayout::CycleRadial
                | SmartArtLayout::ListGrouped
                | SmartArtLayout::ProcessSegmented
                | SmartArtLayout::ProcessList
                | SmartArtLayout::CycleHexagonRadial
                | SmartArtLayout::OrgChartHalfCircle
                | SmartArtLayout::HierarchyLabeled
                | SmartArtLayout::HierarchyLabeledHorizontal
                | SmartArtLayout::HierarchyArchitecture
                | SmartArtLayout::HierarchyTableHorizontal
                | SmartArtLayout::VennRadial
                | SmartArtLayout::CircleRelationship
                | SmartArtLayout::TextConverging
                | SmartArtLayout::RadialList
                | SmartArtLayout::RadialCluster
                | SmartArtLayout::RadialConverging
                | SmartArtLayout::TimelineCircleAccent
        )
    }
}

/// The colours a SmartArt graphic takes from the document's theme.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SmartArtColors {
    /// Every shape in accent `n` (1–6).
    #[default]
    Accent1,
    Accent2,
    Accent3,
    Accent4,
    Accent5,
    Accent6,
    /// Shapes take the accents in turn.
    Colorful,
}

impl SmartArtColors {
    pub const ALL: [SmartArtColors; 7] = [
        SmartArtColors::Accent1,
        SmartArtColors::Accent2,
        SmartArtColors::Accent3,
        SmartArtColors::Accent4,
        SmartArtColors::Accent5,
        SmartArtColors::Accent6,
        SmartArtColors::Colorful,
    ];

    pub fn id(self) -> &'static str {
        match self {
            SmartArtColors::Accent1 => "accent1",
            SmartArtColors::Accent2 => "accent2",
            SmartArtColors::Accent3 => "accent3",
            SmartArtColors::Accent4 => "accent4",
            SmartArtColors::Accent5 => "accent5",
            SmartArtColors::Accent6 => "accent6",
            SmartArtColors::Colorful => "colorful",
        }
    }

    pub fn from_id(id: &str) -> Option<SmartArtColors> {
        let norm = id.chars().filter(|c| !c.is_whitespace() && *c != '-' && *c != '_').collect::<String>();
        SmartArtColors::ALL.into_iter().find(|c| c.id().eq_ignore_ascii_case(&norm) || c.label().replace(' ', "").eq_ignore_ascii_case(&norm))
    }

    /// The name shown for it.
    pub fn label(self) -> &'static str {
        match self {
            SmartArtColors::Accent1 => "Accent 1",
            SmartArtColors::Accent2 => "Accent 2",
            SmartArtColors::Accent3 => "Accent 3",
            SmartArtColors::Accent4 => "Accent 4",
            SmartArtColors::Accent5 => "Accent 5",
            SmartArtColors::Accent6 => "Accent 6",
            SmartArtColors::Colorful => "Colorful",
        }
    }

    /// The theme accent (1–6) of shape `i`.
    pub fn accent(self, i: usize) -> usize {
        match self {
            SmartArtColors::Accent1 => 1,
            SmartArtColors::Accent2 => 2,
            SmartArtColors::Accent3 => 3,
            SmartArtColors::Accent4 => 4,
            SmartArtColors::Accent5 => 5,
            SmartArtColors::Accent6 => 6,
            SmartArtColors::Colorful => i % 6 + 1,
        }
    }
}

/// One item of the outline: its text and level (0 is the top).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartArtItem {
    pub text: String,
    #[serde(default)]
    pub level: u8,
}

impl SmartArtItem {
    pub fn new(text: impl Into<String>, level: u8) -> SmartArtItem {
        SmartArtItem { text: text.into(), level }
    }
}

/// A SmartArt graphic WordCraft can edit.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SmartArtSpec {
    pub layout: SmartArtLayout,
    #[serde(default)]
    pub items: Vec<SmartArtItem>,
    #[serde(default)]
    pub colors: SmartArtColors,
}

impl SmartArtSpec {
    /// A new graphic of `layout` with WordCraft's sample items.
    pub fn sample(layout: SmartArtLayout) -> SmartArtSpec {
        let items: Vec<SmartArtItem> = match layout {
            SmartArtLayout::Hierarchy => vec![
                SmartArtItem::new("Lead", 0),
                SmartArtItem::new("Design", 1),
                SmartArtItem::new("Sketches", 2),
                SmartArtItem::new("Build", 1),
                SmartArtItem::new("Code", 2),
                SmartArtItem::new("Tests", 2),
            ],
            SmartArtLayout::Radial => {
                let mut v = vec![SmartArtItem::new("Goal", 0)];
                v.extend(["Plan", "Build", "Test", "Ship"].map(|t| SmartArtItem::new(t, 1)));
                v
            }
            SmartArtLayout::Process | SmartArtLayout::Cycle => ["Plan", "Build", "Test", "Ship"].map(|t| SmartArtItem::new(t, 0)).to_vec(),
            SmartArtLayout::Pyramid | SmartArtLayout::Venn => ["People", "Process", "Tools"].map(|t| SmartArtItem::new(t, 0)).to_vec(),
            SmartArtLayout::Matrix => ["Strengths", "Weaknesses", "Chances", "Risks"].map(|t| SmartArtItem::new(t, 0)).to_vec(),
            SmartArtLayout::BasicList => ["Ideas", "Notes", "Drafts", "Edits", "Final"].map(|t| SmartArtItem::new(t, 0)).to_vec(),
            SmartArtLayout::TimelineDots
            | SmartArtLayout::TimelineNumbered
            | SmartArtLayout::TimelineArrow
            | SmartArtLayout::TimelineVertical
            | SmartArtLayout::TimelineStems
            | SmartArtLayout::TimelineCircles
            | SmartArtLayout::TimelineZigzag
            | SmartArtLayout::TimelineLine => {
                with_lines(&[("Kickoff", "Goals agreed"), ("Design", "First sketches"), ("Build", "Working draft"), ("Launch", "Out to everyone")])
            }
            SmartArtLayout::TeamCircles | SmartArtLayout::TeamCards | SmartArtLayout::TeamPills => {
                with_lines(&[("Alex Kim", "Team lead"), ("Sam Lee", "Design"), ("Jo Park", "Engineering")])
            }
            SmartArtLayout::CardNumbered
            | SmartArtLayout::CardBigNumbers
            | SmartArtLayout::CardHeader
            | SmartArtLayout::CardTall
            | SmartArtLayout::CardRows
            | SmartArtLayout::CardMarkerRows
            | SmartArtLayout::CardSquares => with_lines(&[("Idea", "Why it matters"), ("Plan", "How we get there"), ("Result", "What changes")]),
            SmartArtLayout::CardEmphasis => {
                [("Reach", "People who saw it", "12k"), ("Growth", "Since last year", "+24%"), ("Rating", "Average score", "4.8")]
                    .iter()
                    .flat_map(|(a, b, c)| [SmartArtItem::new(*a, 0), SmartArtItem::new(*b, 1), SmartArtItem::new(*c, 1)])
                    .collect()
            }
            SmartArtLayout::PictureCaptions
            | SmartArtLayout::PictureGrid
            | SmartArtLayout::PictureCircles
            | SmartArtLayout::PictureRows
            | SmartArtLayout::PictureStrips
            | SmartArtLayout::PictureStaggered => items(&[
                ("Mountains", 0),
                ("Trail day", 1),
                ("Lake", 0),
                ("Quiet water", 1),
                ("City", 0),
                ("Night lights", 1),
                ("Coast", 0),
                ("Sea air", 1),
            ]),
            SmartArtLayout::MatrixTitled => items(&[("Goal", 0), ("Strengths", 1), ("Weaknesses", 1), ("Chances", 1), ("Risks", 1)]),
            SmartArtLayout::MatrixGrid => items(&[("Do first", 0), ("Plan", 0), ("Delegate", 0), ("Drop", 0)]),
            SmartArtLayout::MatrixCycle => {
                items(&[("Plan", 0), ("Set goals", 1), ("Do", 0), ("Try it", 1), ("Check", 0), ("Measure", 1), ("Act", 0), ("Adjust", 1)])
            }
            SmartArtLayout::PyramidInverted => items(&[("Leads", 0), ("Prospects", 0), ("Customers", 0)]),
            SmartArtLayout::PyramidList => items(&[("Vision", 0), ("Strategy", 0), ("Tasks", 0)]),
            SmartArtLayout::PyramidSegmented => items(&[("Purpose", 0), ("People", 0), ("Process", 0), ("Tools", 0)]),
            SmartArtLayout::Balance => items(&[("Pros", 0), ("Faster", 1), ("Cheaper", 1), ("Cons", 0), ("Riskier", 1)]),
            SmartArtLayout::Funnel => items(&[("Ideas", 0), ("Research", 0), ("Tests", 0), ("Product", 0)]),
            SmartArtLayout::Gears | SmartArtLayout::CycleMultidirectional => items(&[("Design", 0), ("Build", 0), ("Test", 0)]),
            SmartArtLayout::ArrowsOpposing => items(&[("Rise", 0), ("More users", 1), ("Fall", 0), ("Lower costs", 1)]),
            SmartArtLayout::Equation => items(&[("Idea", 0), ("Work", 0), ("Result", 0)]),
            SmartArtLayout::VennLinear => items(&[("Learn", 0), ("Build", 0), ("Share", 0), ("Grow", 0)]),
            SmartArtLayout::Target => items(&[("Vision", 0), ("Goals", 0), ("Tasks", 0)]),
            SmartArtLayout::RadialDiverging | SmartArtLayout::CycleRadial => {
                items(&[("Goal", 0), ("Plan", 1), ("Build", 1), ("Test", 1), ("Ship", 1)])
            }
            SmartArtLayout::OrgChart => {
                items(&[("Lead", 0), ("Design", 1), ("Sketches", 2), ("Mockups", 2), ("Build", 1), ("Code", 2), ("Tests", 2)])
            }
            SmartArtLayout::HierarchyHorizontal | SmartArtLayout::HierarchyTable | SmartArtLayout::HierarchyCircles => {
                items(&[("Lead", 0), ("Design", 1), ("Sketches", 2), ("Build", 1), ("Code", 2), ("Tests", 2)])
            }
            SmartArtLayout::HierarchyList | SmartArtLayout::ListGrouped => {
                items(&[("Design", 0), ("Sketches", 1), ("Mockups", 1), ("Build", 0), ("Code", 1), ("Tests", 1), ("Launch", 0), ("Release", 1)])
            }
            SmartArtLayout::LinedList => items(&[("Spring", 0), ("Plan", 1), ("Hire", 1), ("Summer", 0), ("Build", 1), ("Ship", 1)]),
            SmartArtLayout::CycleText
            | SmartArtLayout::CycleContinuous
            | SmartArtLayout::CycleSegmented
            | SmartArtLayout::CycleDots
            | SmartArtLayout::Chevrons
            | SmartArtLayout::StepUp
            | SmartArtLayout::ProcessVertical
            | SmartArtLayout::ProcessContinuous => items(&[("Plan", 0), ("Build", 0), ("Test", 0), ("Ship", 0)]),
            SmartArtLayout::ChevronList | SmartArtLayout::ProcessArrows => {
                items(&[("Plan", 0), ("Set goals", 1), ("Build", 0), ("Make it", 1), ("Ship", 0), ("Release", 1)])
            }
            SmartArtLayout::ProcessBending => items(&[("Idea", 0), ("Sketch", 0), ("Draft", 0), ("Review", 0), ("Fix", 0), ("Ship", 0)]),
            SmartArtLayout::ArrowUpward => items(&[("Start", 0), ("Small team", 1), ("Grow", 0), ("New users", 1), ("Scale", 0), ("Worldwide", 1)]),
            SmartArtLayout::ListHexagon => items(&[("Ideas", 0), ("Notes", 0), ("Drafts", 0), ("Edits", 0), ("Final", 0)]),
            SmartArtLayout::ListVerticalBullet | SmartArtLayout::ListTrapezoid => {
                items(&[("Idea", 0), ("Why it matters", 1), ("Plan", 0), ("How we get there", 1), ("Result", 0), ("What changes", 1)])
            }
            SmartArtLayout::ListDescending => items(&[("Most", 0), ("More", 0), ("Some", 0), ("Few", 0)]),
            SmartArtLayout::ListTable => items(&[("Project", 0), ("Plan", 0), ("Goals", 1), ("Build", 0), ("Draft", 1), ("Ship", 0), ("Release", 1)]),
            SmartArtLayout::ListHexagonAlternating
            | SmartArtLayout::ListVerticalBox
            | SmartArtLayout::ListHorizontalBullet
            | SmartArtLayout::ListSquareAccent
            | SmartArtLayout::ListStacked
            | SmartArtLayout::ListVerticalBlock
            | SmartArtLayout::ListVerticalArrow
            | SmartArtLayout::ListTarget
            | SmartArtLayout::ListVerticalCircle
            | SmartArtLayout::ListTab => with_lines(&[("Idea", "Why it matters"), ("Plan", "How we get there"), ("Result", "What changes")]),
            SmartArtLayout::ListVaryingWidth | SmartArtLayout::ListCurved => {
                items(&[("Ideas", 0), ("Notes", 0), ("Drafts", 0), ("Edits", 0), ("Final", 0)])
            }
            SmartArtLayout::ProcessAccent
            | SmartArtLayout::ProcessAlternatingFlow
            | SmartArtLayout::ChevronsDetailed
            | SmartArtLayout::ProcessCircle
            | SmartArtLayout::StepDown
            | SmartArtLayout::ProcessIncreasingArrows
            | SmartArtLayout::ArrowDownward
            | SmartArtLayout::ProcessCirclesIncreasing => {
                items(&[("Plan", 0), ("Set goals", 1), ("Build", 0), ("Make it", 1), ("Ship", 0), ("Release", 1)])
            }
            SmartArtLayout::ChevronsClosed
            | SmartArtLayout::ProcessContinuousArrow
            | SmartArtLayout::CycleBlock
            | SmartArtLayout::CycleNondirectional
            | SmartArtLayout::CyclePie => items(&[("Plan", 0), ("Build", 0), ("Test", 0), ("Ship", 0)]),
            SmartArtLayout::ProcessSegmented | SmartArtLayout::ProcessList => {
                items(&[("Design", 0), ("Sketches", 1), ("Mockups", 1), ("Build", 0), ("Code", 1), ("Tests", 1), ("Launch", 0), ("Release", 1)])
            }
            SmartArtLayout::ProcessSnake | SmartArtLayout::ProcessBendingVertical => {
                items(&[("Idea", 0), ("Sketch", 0), ("Draft", 0), ("Review", 0), ("Fix", 0), ("Ship", 0)])
            }
            SmartArtLayout::CycleHexagonRadial
            | SmartArtLayout::VennRadial
            | SmartArtLayout::CircleRelationship
            | SmartArtLayout::RadialList
            | SmartArtLayout::RadialCluster
            | SmartArtLayout::RadialConverging => items(&[("Goal", 0), ("Plan", 1), ("Build", 1), ("Test", 1), ("Ship", 1)]),
            SmartArtLayout::OrgChartHalfCircle => {
                items(&[("Lead", 0), ("Design", 1), ("Sketches", 2), ("Mockups", 2), ("Build", 1), ("Code", 2), ("Tests", 2)])
            }
            SmartArtLayout::HierarchyLabeled | SmartArtLayout::HierarchyLabeledHorizontal => items(&[
                ("Lead", 0),
                ("Design", 1),
                ("Sketches", 2),
                ("Build", 1),
                ("Code", 2),
                ("Tests", 2),
                ("Leadership", 0),
                ("Teams", 0),
                ("Work", 0),
            ]),
            SmartArtLayout::HierarchyArchitecture | SmartArtLayout::HierarchyTableHorizontal => {
                items(&[("Lead", 0), ("Design", 1), ("Sketches", 2), ("Build", 1), ("Code", 2), ("Tests", 2)])
            }
            SmartArtLayout::ArrowsConverging | SmartArtLayout::ArrowsDiverging => {
                with_lines(&[("Skills", "What we have"), ("Needs", "What customers want")])
            }
            SmartArtLayout::ArrowsCounterbalance => with_lines(&[("Rise", "More users"), ("Fall", "Lower costs")]),
            SmartArtLayout::VennStacked => items(&[("Vision", 0), ("Strategy", 0), ("Tasks", 0)]),
            SmartArtLayout::TargetNested => items(&[("Vision", 0), ("Goals", 0), ("Tasks", 0)]),
            SmartArtLayout::PlusMinus => items(&[("Pros", 0), ("Faster", 1), ("Cheaper", 1), ("Cons", 0), ("Riskier", 1)]),
            SmartArtLayout::TextConverging => items(&[("Product", 0), ("Ideas", 1), ("Research", 1), ("Tests", 1)]),
            SmartArtLayout::EquationVertical => items(&[("Idea", 0), ("Work", 0), ("Result", 0)]),
            SmartArtLayout::RingsInterconnected => items(&[("Design", 0), ("Build", 0), ("Test", 0)]),
            SmartArtLayout::TimelineCircleAccent => {
                with_lines(&[("Kickoff", "Goals agreed"), ("Design", "First sketches"), ("Build", "Working draft"), ("Launch", "Out to everyone")])
            }
        };
        SmartArtSpec { layout, items, colors: SmartArtColors::default() }
    }

    /// Make the graphic safe to store and draw: at most [`MAX_ITEMS`] items, text cut to
    /// [`MAX_TEXT`] characters with control characters as spaces and the ends trimmed, and levels
    /// that follow on (the first item at level 0, each item at most one level below the one
    /// before it, never below [`MAX_LEVEL`]).
    pub fn sanitize(&mut self) {
        self.items.truncate(MAX_ITEMS);
        let mut prev: Option<u8> = None;
        for it in &mut self.items {
            it.text = clean_text(&it.text);
            let most = prev.map_or(0, |p| p.saturating_add(1)).min(MAX_LEVEL);
            it.level = it.level.min(most);
            prev = Some(it.level);
        }
    }

    /// The graphic after `sanitize`.
    pub fn sanitized(mut self) -> SmartArtSpec {
        self.sanitize();
        self
    }

    /// The index of the item each item belongs to (the nearest item before it one level up);
    /// `None` for the top-level items. Expects a sanitized graphic.
    pub fn parents(&self) -> Vec<Option<usize>> {
        let mut out = Vec::with_capacity(self.items.len());
        // The last item seen at each level.
        let mut last: Vec<usize> = Vec::new();
        for (i, it) in self.items.iter().enumerate() {
            let l = usize::from(it.level);
            last.truncate(l);
            out.push(if l == 0 { None } else { last.last().copied() });
            last.push(i);
        }
        out
    }

    /// The items that belong to item `i` (directly or deeper): the run after it at deeper levels.
    pub fn descendants(&self, i: usize) -> std::ops::Range<usize> {
        let Some(level) = self.items.get(i).map(|it| it.level) else { return 0..0 };
        let start = i + 1;
        let end = self.items.iter().skip(start).position(|it| it.level <= level).map_or(self.items.len(), |p| start + p);
        start..end
    }
}

/// Items from (text, level) pairs.
fn items(list: &[(&str, u8)]) -> Vec<SmartArtItem> {
    list.iter().map(|(t, l)| SmartArtItem::new(*t, *l)).collect()
}

/// Top-level items, each with one line under it.
fn with_lines(pairs: &[(&str, &str)]) -> Vec<SmartArtItem> {
    pairs.iter().flat_map(|(a, b)| [SmartArtItem::new(*a, 0), SmartArtItem::new(*b, 1)]).collect()
}

/// `s` as item text: control characters as spaces, ends trimmed, at most [`MAX_TEXT`] characters.
pub fn clean_text(s: &str) -> String {
    let trimmed = s.trim_start_matches(|c: char| c.is_whitespace() || c.is_control());
    let s: String = trimmed.chars().take(MAX_TEXT).map(|c| if c.is_control() { ' ' } else { c }).collect();
    s.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_caps_items_text_and_levels() {
        let mut s = SmartArtSpec {
            items: (0..MAX_ITEMS + 20).map(|i| SmartArtItem::new(format!("{}\n{i}", "x".repeat(400)), if i == 0 { 5 } else { 200 })).collect(),
            ..Default::default()
        };
        s.sanitize();
        assert_eq!(s.items.len(), MAX_ITEMS);
        assert_eq!(s.items[0].level, 0, "the first item is at the top");
        assert_eq!(s.items[1].level, 1, "one level below the item before");
        assert!(s.items.iter().all(|it| it.level <= MAX_LEVEL));
        assert_eq!(s.items[0].text.chars().count(), MAX_TEXT);
        assert!(!s.items[0].text.contains('\n'));
    }

    #[test]
    fn parents_and_descendants_follow_levels() {
        let s = SmartArtSpec::sample(SmartArtLayout::Hierarchy).sanitized();
        assert_eq!(s.parents(), vec![None, Some(0), Some(1), Some(0), Some(3), Some(3)]);
        assert_eq!(s.descendants(0), 1..6);
        assert_eq!(s.descendants(1), 2..3);
        assert_eq!(s.descendants(5), 6..6);
        assert_eq!(s.descendants(99), 0..0);
        assert_eq!(SmartArtLayout::from_id("VENN"), Some(SmartArtLayout::Venn));
        assert_eq!(SmartArtLayout::from_id("basic-list"), Some(SmartArtLayout::BasicList));
        assert_eq!(SmartArtLayout::from_id("Basic Process"), Some(SmartArtLayout::Process));
        assert_eq!(SmartArtLayout::from_id("basic_cycle"), Some(SmartArtLayout::Cycle));
        assert_eq!(SmartArtLayout::from_id("list"), Some(SmartArtLayout::BasicList));
        assert_eq!(SmartArtLayout::from_id("timeline-dots"), Some(SmartArtLayout::TimelineDots));
        assert_eq!(SmartArtLayout::from_id("timeline-stems"), Some(SmartArtLayout::TimelineStems));
        assert_eq!(SmartArtLayout::from_id("Numbered Squares"), Some(SmartArtLayout::CardSquares));
        assert_eq!(SmartArtLayout::from_id("Team Cards"), Some(SmartArtLayout::TeamCards));
        for l in SmartArtLayout::ALL {
            assert_eq!(SmartArtLayout::from_id(l.id()), Some(l));
            assert!(SmartArtSpec::sample(l).sanitized().items.first().is_some_and(|it| it.level == 0));
        }
        assert_eq!(SmartArtColors::from_id("colorful"), Some(SmartArtColors::Colorful));
        assert_eq!(SmartArtColors::from_id("Accent 2"), Some(SmartArtColors::Accent2));
        assert_eq!(SmartArtColors::from_id("accent-4"), Some(SmartArtColors::Accent4));
        assert_eq!(clean_text(&format!("{}Valid Text", " ".repeat(1500))), "Valid Text");
    }
}
