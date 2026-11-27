//! SmartArt graphics WordCraft makes ([`SmartArtSpec`]) as diagram parts (ECMA-376 Part 1 §21.4)
//! and back.
//!
//! WordCraft lays a graphic out with its own algorithms ([`layout`]: blocks, steps with arrows,
//! a cycle, a tree, a pyramid, a radial, a matrix, overlapping circles) and writes five parts:
//! - the data model ([`data_xml`], `dgm:dataModel`): the document point, a node point per item
//!   with its text, and connections that give each item its parent;
//! - a layout definition ([`layout_xml`]), a quick style ([`quick_style_xml`]) and a colour
//!   definition ([`colors_xml`]) of our own, named by the identifiers ECMA-376 applications use
//!   for the matching built-in layouts, styles and colours;
//! - the drawing ([`drawing_xml`], `dsp:drawing`): every shape already placed, so any reader
//!   shows the graphic as WordCraft laid it out. WordCraft draws its own graphics from that same
//!   drawing ([`smart_art_items`]).
//!
//! [`spec_of`] turns a data part back into the model only when writing that model again gives
//! the same markup: a diagram another program made or changed stays read-only.

use wordcraft_doc::graphic::GraphicItem;
use wordcraft_doc::props::Rgb;
use wordcraft_doc::smart_art::{MAX_ITEMS, SmartArtColors, SmartArtItem, SmartArtLayout, SmartArtSpec};

use crate::read::diagram::{text_width, wrap};
use crate::xml::{self, El, MAX_DEPTH, Node, W};

const NS_DGM: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
const NS_DSP: &str = "http://schemas.microsoft.com/office/drawing/2008/diagram";
const NS_A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const NS_R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";
/// `a:graphicData` URI of a diagram.
pub(crate) const DIAGRAM_URI: &str = "http://schemas.openxmlformats.org/drawingml/2006/diagram";
/// Content types of the five parts.
pub(crate) const DATA_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramData+xml";
pub(crate) const LAYOUT_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramLayout+xml";
pub(crate) const STYLE_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramStyle+xml";
pub(crate) const COLORS_CT: &str = "application/vnd.openxmlformats-officedocument.drawingml.diagramColors+xml";
pub(crate) const DRAWING_CT: &str = "application/vnd.ms-office.drawingml.diagramDrawing+xml";
/// The extension a data model names its drawing in (ECMA-376 extension list, `dsp:dataModelExt`).
const DRAWING_EXT_URI: &str = "http://schemas.microsoft.com/office/drawing/2008/diagram";
const QUICK_STYLE_ID: &str = "urn:microsoft.com/office/officeart/2005/8/quickstyle/simple1";
const EMU_PT: f32 = 12_700.0;
/// Line height as a multiple of the font size (as the drawing reader stacks lines).
const LINE: f32 = 1.2;
/// Largest and smallest text size a graphic is given, points.
const MAX_SIZE: f32 = 24.0;
const MIN_SIZE: f32 = 5.0;
/// A sub-item's text size against its item's.
const CHILD: f32 = 0.8;
/// Model ids of the drawing's shapes start here (the data model's points are below).
const SHAPE_IDS: usize = 0x10000;

/// The layout's identifier and its gallery category.
fn layout_id(l: SmartArtLayout) -> (&'static str, &'static str) {
    match l {
        SmartArtLayout::BasicList => ("urn:microsoft.com/office/officeart/2005/8/layout/default", "list"),
        SmartArtLayout::Process => ("urn:microsoft.com/office/officeart/2005/8/layout/process1", "process"),
        SmartArtLayout::Cycle => ("urn:microsoft.com/office/officeart/2005/8/layout/cycle2", "cycle"),
        SmartArtLayout::Hierarchy => ("urn:microsoft.com/office/officeart/2005/8/layout/hierarchy1", "hierarchy"),
        SmartArtLayout::Pyramid => ("urn:microsoft.com/office/officeart/2005/8/layout/pyramid1", "pyramid"),
        SmartArtLayout::Radial => ("urn:microsoft.com/office/officeart/2005/8/layout/radial1", "cycle"),
        SmartArtLayout::Matrix => ("urn:microsoft.com/office/officeart/2005/8/layout/matrix3", "matrix"),
        SmartArtLayout::Venn => ("urn:microsoft.com/office/officeart/2005/8/layout/venn1", "relationship"),
        // Layouts with no built-in counterpart we know of: identifiers of our own, so a reader
        // that doesn't know them uses the layout definition the file carries (see `layout_xml`).
        SmartArtLayout::TimelineDots => ("urn:lilyoffice.com/smartart/2026/layout/timelineDots", "process"),
        SmartArtLayout::TimelineNumbered => ("urn:lilyoffice.com/smartart/2026/layout/timelineNumbered", "process"),
        SmartArtLayout::TimelineArrow => ("urn:lilyoffice.com/smartart/2026/layout/timelineArrow", "process"),
        SmartArtLayout::TimelineVertical => ("urn:lilyoffice.com/smartart/2026/layout/timelineVertical", "process"),
        SmartArtLayout::TeamCircles => ("urn:lilyoffice.com/smartart/2026/layout/teamCircles", "list"),
        SmartArtLayout::TeamCards => ("urn:lilyoffice.com/smartart/2026/layout/teamCards", "list"),
        SmartArtLayout::TeamPills => ("urn:lilyoffice.com/smartart/2026/layout/teamPills", "list"),
        SmartArtLayout::CardNumbered => ("urn:lilyoffice.com/smartart/2026/layout/cardNumbered", "list"),
        SmartArtLayout::CardBigNumbers => ("urn:lilyoffice.com/smartart/2026/layout/cardBigNumbers", "list"),
        SmartArtLayout::CardHeader => ("urn:lilyoffice.com/smartart/2026/layout/cardHeader", "list"),
        SmartArtLayout::CardTall => ("urn:lilyoffice.com/smartart/2026/layout/cardTall", "list"),
        SmartArtLayout::CardRows => ("urn:lilyoffice.com/smartart/2026/layout/cardRows", "list"),
    }
}

/// The colour definition's identifier and category.
fn colors_id(c: SmartArtColors) -> (String, String) {
    match c {
        SmartArtColors::Colorful => ("urn:microsoft.com/office/officeart/2005/8/colors/colorful1".into(), "colorful".into()),
        _ => {
            let n = c.accent(0);
            (format!("urn:microsoft.com/office/officeart/2005/8/colors/accent{n}_2"), format!("accent{n}"))
        }
    }
}

/// Model id number `n` as the GUID form ECMA-376 model ids take.
fn model_id(n: usize) -> String {
    format!("{{5743A57A-0000-4000-8000-{n:012X}}}")
}

// ---- layout ----

/// What a shape is filled or outlined with.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Paint {
    None,
    /// Theme accent `n` (1–6), lightened to `tint` (thousandths of a percent kept) when set.
    Accent(usize, Option<u32>),
    /// The theme's light background colour.
    Light,
}

/// The colour of a shape's text.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Ink {
    /// The theme's light colour (on filled shapes).
    Light,
    /// The theme's dark text colour (on light fills or no fill).
    Dark,
    /// Theme accent `n` (1–6).
    Accent(usize),
}

impl Ink {
    fn scheme(self) -> String {
        match self {
            Ink::Light => "lt1".into(),
            Ink::Dark => "tx1".into(),
            Ink::Accent(n) => format!("accent{}", n.clamp(1, 6)),
        }
    }
}

/// One shape of a laid-out graphic, in points from the graphic's top-left corner.
#[derive(Clone, Debug)]
struct Sh {
    prst: &'static str,
    b: [f32; 4],
    /// Clockwise turn, degrees.
    rot: f32,
    flip_h: bool,
    /// The preset's adjust values (`adj`, `adj1`…) that differ from its defaults.
    adj: Vec<(&'static str, i64)>,
    fill: Paint,
    /// Fill opacity (thousandths of a percent) for readers that blend.
    alpha: Option<u32>,
    line: Paint,
    line_w: f32,
    /// Paragraphs: text and whether it's a sub-item (smaller, bulleted, left-aligned).
    text: Vec<(String, bool)>,
    /// Where the text goes.
    tb: [f32; 4],
    ink: Ink,
    /// Every paragraph's alignment (`l`, `ctr`, `r`); `None`: the item's line centred, sub-items left.
    algn: Option<&'static str>,
    /// Where the text sits in its box (`t`, `ctr`, `b`).
    anchor: &'static str,
    /// The text size against the graphic's (large numbers are more than 1).
    scale: f32,
    /// The outline item this shape stands for (a node or its text), for picking one item.
    item: Option<usize>,
}

impl Sh {
    /// A filled node shape with a thin light outline.
    fn node(prst: &'static str, b: [f32; 4], accent: usize) -> Sh {
        Sh {
            prst,
            b,
            rot: 0.0,
            flip_h: false,
            adj: Vec::new(),
            fill: Paint::Accent(accent, None),
            alpha: None,
            line: Paint::Light,
            line_w: 1.0,
            text: Vec::new(),
            tb: inset(b, 0.06),
            ink: Ink::Light,
            algn: None,
            anchor: "ctr",
            scale: 1.0,
            item: None,
        }
    }

    /// A straight connector from (`x0`, `y0`) to (`x1`, `y1`).
    fn line(x0: f32, y0: f32, x1: f32, y1: f32, accent: usize) -> Sh {
        let b = [x0.min(x1), y0.min(y1), (x1 - x0).abs(), (y1 - y0).abs()];
        Sh {
            prst: "line",
            b,
            rot: 0.0,
            flip_h: (x1 - x0) * (y1 - y0) < 0.0,
            adj: Vec::new(),
            fill: Paint::None,
            alpha: None,
            line: Paint::Accent(accent, Some(60_000)),
            line_w: 1.5,
            text: Vec::new(),
            tb: b,
            ink: Ink::Light,
            algn: None,
            anchor: "ctr",
            scale: 1.0,
            item: None,
        }
    }

    /// The text of item `i` (and, with `with_children`, the items under it as bullets), and
    /// the item this shape stands for.
    fn with_item(mut self, spec: &SmartArtSpec, i: usize, with_children: bool) -> Sh {
        self.text = paras(spec, i, with_children);
        self.item = Some(i);
        self
    }

    fn of_item(mut self, i: usize) -> Sh {
        self.item = Some(i);
        self
    }

    fn text_box(mut self, tb: [f32; 4]) -> Sh {
        self.tb = tb;
        self
    }
}

/// `b` shrunk by `f` of its width and height on every side.
fn inset(b: [f32; 4], f: f32) -> [f32; 4] {
    [b[0] + b[2] * f, b[1] + b[3] * f, b[2] * (1.0 - 2.0 * f), b[3] * (1.0 - 2.0 * f)]
}

/// The inscribed square's share of a circle's box to leave on each side.
const ROUND_INSET: f32 = 0.15;

/// The text of item `i`: its own line, then the items under it as bulleted sub-items.
fn paras(spec: &SmartArtSpec, i: usize, with_children: bool) -> Vec<(String, bool)> {
    let mut out = vec![(spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default(), false)];
    if with_children {
        for j in spec.descendants(i) {
            if let Some(it) = spec.items.get(j) {
                out.push((format!("• {}", it.text), true));
            }
        }
    }
    out
}

/// The top-level items.
fn tops(spec: &SmartArtSpec) -> Vec<usize> {
    spec.items.iter().enumerate().filter(|(_, it)| it.level == 0).map(|(i, _)| i).collect()
}

/// The shapes of `spec` laid out in a `w` × `h` points box, back to front.
fn layout(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let ok = |v: f32| if v.is_finite() { v.clamp(0.0, 20_000.0) } else { 0.0 };
    let (w, h) = (ok(w), ok(h));
    if spec.items.is_empty() || w < 1.0 || h < 1.0 {
        return Vec::new();
    }
    match spec.layout {
        SmartArtLayout::BasicList => basic_list(spec, w, h),
        SmartArtLayout::Process => process(spec, w, h),
        SmartArtLayout::Cycle => cycle(spec, w, h),
        SmartArtLayout::Hierarchy => hierarchy(spec, w, h),
        SmartArtLayout::Pyramid => pyramid(spec, w, h),
        SmartArtLayout::Radial => radial(spec, w, h),
        SmartArtLayout::Matrix => matrix(spec, w, h),
        SmartArtLayout::Venn => venn(spec, w, h),
        SmartArtLayout::TimelineDots => timeline_dots(spec, w, h),
        SmartArtLayout::TimelineNumbered => timeline_numbered(spec, w, h),
        SmartArtLayout::TimelineArrow => timeline_arrow(spec, w, h),
        SmartArtLayout::TimelineVertical => timeline_vertical(spec, w, h),
        SmartArtLayout::TeamCircles => team_circles(spec, w, h),
        SmartArtLayout::TeamCards => team_cards(spec, w, h),
        SmartArtLayout::TeamPills => team_pills(spec, w, h),
        SmartArtLayout::CardNumbered => card_numbered(spec, w, h, false),
        SmartArtLayout::CardBigNumbers => card_numbered(spec, w, h, true),
        SmartArtLayout::CardHeader => card_header(spec, w, h),
        SmartArtLayout::CardTall => card_tall(spec, w, h),
        SmartArtLayout::CardRows => card_rows(spec, w, h),
    }
}

/// Blocks of 3:2 in rows, as large as fit; the last row centred.
fn basic_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let size = |c: usize| {
        let r = n.div_ceil(c);
        let bw = (w / c as f32 * 0.9).min(h / r as f32 * 0.9 * 1.5);
        (bw, bw / 1.5)
    };
    let cols = (1..=n).max_by(|a, b| (size(*a).0).total_cmp(&size(*b).0)).unwrap_or(1);
    let rows = n.div_ceil(cols);
    let (bw, bh) = size(cols);
    let (gx, gy) = (bw * 0.1, bh * 0.1);
    let y0 = (h - (rows as f32 * bh + (rows - 1) as f32 * gy)) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let in_row = if r + 1 == rows { n - r * cols } else { cols };
        let x0 = (w - (in_row as f32 * bw + (in_row - 1) as f32 * gx)) / 2.0;
        let b = [x0 + c as f32 * (bw + gx), y0 + r as f32 * (bh + gy), bw, bh];
        out.push(Sh::node("rect", b, spec.colors.accent(k)).with_item(spec, i, true));
    }
    out
}

/// Steps left to right with an arrow in each gap.
fn process(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    const GAP: f32 = 0.45;
    let nw = w / (n + (n - 1.0) * GAP);
    let nh = (nw * 0.6).min(h * 0.9);
    let y = (h - nh) / 2.0;
    let gap = nw * GAP;
    let mut out = Vec::new();
    for (k, _) in tops.iter().enumerate().skip(1) {
        let gx = k as f32 * (nw + gap) - gap;
        let (aw, ah) = (gap * 0.62, (nh * 0.35).min(gap * 0.7));
        let mut a = Sh::node("rightArrow", [gx + (gap - aw) / 2.0, h / 2.0 - ah / 2.0, aw, ah], spec.colors.accent(k - 1));
        a.fill = Paint::Accent(spec.colors.accent(k - 1), Some(60_000));
        a.line = Paint::None;
        out.push(a);
    }
    for (k, &i) in tops.iter().enumerate() {
        let b = [k as f32 * (nw + gap), y, nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_item(spec, i, true));
    }
    out
}

/// The angle (degrees, clockwise from three o'clock) of place `k` of `n` round a circle, the
/// first at twelve o'clock.
fn angle(k: usize, n: usize) -> f32 {
    -90.0 + 360.0 * k as f32 / n.max(1) as f32
}

/// The point at `r` from (`cx`, `cy`) at `deg`.
fn polar(cx: f32, cy: f32, r: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (cx + r * c, cy + r * s)
}

/// Circles round a circle, clockwise from the top, with a curved-path arrow between each.
fn cycle(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let step = (std::f32::consts::PI / n as f32).sin();
    let mut d = m * 0.32;
    for _ in 0..4 {
        let r = (m - d) / 2.0;
        d = if n == 1 { m * 0.5 } else { (m * 0.32).min(2.0 * r * step * 0.62) };
    }
    let r = (m - d) / 2.0;
    let mut out = Vec::new();
    if n > 1 {
        let chord = 2.0 * r * step;
        let len = (chord - d) * 0.6;
        if len > 2.0 {
            let thick = (len * 0.6).min(d * 0.25);
            for k in 0..n {
                let mid = angle(k, n) + 180.0 / n as f32;
                let (x, y) = polar(cx, cy, r, mid);
                let mut a = Sh::node("rightArrow", [x - len / 2.0, y - thick / 2.0, len, thick], spec.colors.accent(k));
                a.rot = (mid + 90.0).rem_euclid(360.0);
                a.fill = Paint::Accent(spec.colors.accent(k), Some(60_000));
                a.line = Paint::None;
                out.push(a);
            }
        }
    }
    for (k, &i) in tops.iter().enumerate() {
        let (x, y) = if n == 1 { (cx, cy) } else { polar(cx, cy, r, angle(k, n)) };
        let b = [x - d / 2.0, y - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k)).with_item(spec, i, true).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// A tree, a row per level: each item centred over the items under it, joined by elbow lines.
fn hierarchy(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n = spec.items.len();
    let parents = spec.parents();
    let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut roots = Vec::new();
    for (i, p) in parents.iter().enumerate() {
        match p.and_then(|p| kids.get_mut(p)) {
            Some(k) => k.push(i),
            None => roots.push(i),
        }
    }
    // Leaves under each item (children come after their parent, so from the end).
    let mut leaves = vec![1usize; n];
    for i in (0..n).rev() {
        let sum: usize = kids.get(i).map_or(0, |k| k.iter().map(|&c| leaves.get(c).copied().unwrap_or(1)).sum());
        if sum > 0
            && let Some(l) = leaves.get_mut(i)
        {
            *l = sum;
        }
    }
    let depth = spec.items.iter().map(|it| usize::from(it.level)).max().unwrap_or(0) + 1;
    let total: usize = roots.iter().map(|&r| leaves.get(r).copied().unwrap_or(1)).sum::<usize>().max(1);
    let unit = w / total as f32;
    let rh = h / depth as f32;
    let nw = (unit * 0.88).min(rh * 0.62 * 1.8);
    let nh = (rh * 0.62).min(nw * 0.8);
    let mut start = vec![0.0f32; n];
    let mut at = 0.0;
    for &r in &roots {
        if let Some(s) = start.get_mut(r) {
            *s = at;
        }
        at += leaves.get(r).copied().unwrap_or(1) as f32 * unit;
    }
    for i in 0..n {
        let mut at = start.get(i).copied().unwrap_or(0.0);
        for &k in kids.get(i).map(Vec::as_slice).unwrap_or(&[]) {
            if let Some(s) = start.get_mut(k) {
                *s = at;
            }
            at += leaves.get(k).copied().unwrap_or(1) as f32 * unit;
        }
    }
    let cx = |i: usize| start.get(i).copied().unwrap_or(0.0) + leaves.get(i).copied().unwrap_or(1) as f32 * unit / 2.0;
    let top = |i: usize| spec.items.get(i).map_or(0.0, |it| f32::from(it.level) * rh + (rh - nh) / 2.0);
    let mut out = Vec::new();
    for i in 0..n {
        let Some(ks) = kids.get(i).filter(|k| !k.is_empty()) else { continue };
        let accent = spec.colors.accent(i);
        let (px, pb) = (cx(i), top(i) + nh);
        let ct = ks.first().map_or(pb, |&k| top(k));
        let mid = (pb + ct) / 2.0;
        out.push(Sh::line(px, pb, px, mid, accent));
        let xs: Vec<f32> = ks.iter().map(|&k| cx(k)).collect();
        let (lo, hi) = xs.iter().fold((px, px), |(lo, hi), &x| (lo.min(x), hi.max(x)));
        if hi - lo > 0.5 {
            out.push(Sh::line(lo, mid, hi, mid, accent));
        }
        for x in xs {
            out.push(Sh::line(x, mid, x, ct, accent));
        }
    }
    for i in 0..n {
        let b = [cx(i) - nw / 2.0, top(i), nw, nh];
        out.push(Sh::node("roundRect", b, spec.colors.accent(i)).with_item(spec, i, false));
    }
    out
}

/// Bands stacked into a triangle: the first item at the narrow top.
fn pyramid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let tw = w.min(h * 1.3);
    let x0 = (w - tw) / 2.0;
    let bh = h / n;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (top_w, bot_w) = (tw * k as f32 / n, tw * (k as f32 + 1.0) / n);
        let b = [x0 + (tw - bot_w) / 2.0, k as f32 * bh, bot_w, bh];
        let accent = spec.colors.accent(k);
        let sh = if k == 0 {
            let tb = [b[0] + bot_w * 0.22, b[1] + bh * 0.4, bot_w * 0.56, bh * 0.56];
            Sh::node("triangle", b, accent).text_box(tb)
        } else {
            let inset_x = (bot_w - top_w) / 2.0;
            let ss = bot_w.min(bh).max(0.01);
            let mut s = Sh::node("trapezoid", b, accent);
            s.adj = vec![("adj", (inset_x / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64)];
            let mid_w = (top_w + bot_w) / 2.0 * 0.8;
            s.text_box([w / 2.0 - mid_w / 2.0, b[1] + bh * 0.08, mid_w, bh * 0.84])
        };
        out.push(sh.with_item(spec, i, true));
    }
    out
}

/// The first item in the middle; the items under it (and any later top-level items) around it,
/// each joined to the middle by a line.
fn radial(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n_items = spec.items.len();
    let parents = spec.parents();
    let spokes: Vec<usize> = (1..n_items).filter(|&i| matches!(parents.get(i), Some(Some(0)) | Some(None))).collect();
    let n = spokes.len();
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut out = Vec::new();
    if n == 0 {
        let d = m * 0.5;
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        out.push(Sh::node("ellipse", b, spec.colors.accent(0)).with_item(spec, 0, false).text_box(inset(b, ROUND_INSET)));
        return out;
    }
    let step = if n == 1 { 1.0 } else { (std::f32::consts::PI / n as f32).sin() };
    let mut ds = m * 0.24;
    for _ in 0..4 {
        let r = (m - ds) / 2.0;
        ds = (m * 0.24).min(2.0 * r * step * 0.85);
    }
    let r = (m - ds) / 2.0;
    let dc = (m * 0.36).min(2.0 * (r - ds / 2.0 - m * 0.03)).max(m * 0.05);
    let at = |k: usize| polar(cx, cy, r, angle(k, n));
    for k in 0..n {
        let (x, y) = at(k);
        out.push(Sh::line(cx, cy, x, y, spec.colors.accent(0)));
    }
    let bc = [cx - dc / 2.0, cy - dc / 2.0, dc, dc];
    out.push(Sh::node("ellipse", bc, spec.colors.accent(0)).with_item(spec, 0, false).text_box(inset(bc, ROUND_INSET)));
    for (k, &i) in spokes.iter().enumerate() {
        let (x, y) = at(k);
        let b = [x - ds / 2.0, y - ds / 2.0, ds, ds];
        out.push(Sh::node("ellipse", b, spec.colors.accent(k + 1)).with_item(spec, i, true).text_box(inset(b, ROUND_INSET)));
    }
    out
}

/// Quadrants of a square (more items: a larger grid).
fn matrix(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cols = (1..=n).find(|c| c * c >= n).unwrap_or(n).max(1);
    let rows = n.div_ceil(cols);
    let cell = (w / cols as f32).min(h / rows as f32);
    let gap = cell * 0.04;
    let (x0, y0) = ((w - cols as f32 * cell) / 2.0, (h - rows as f32 * cell) / 2.0);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (r, c) = (k / cols, k % cols);
        let b = [x0 + c as f32 * cell + gap / 2.0, y0 + r as f32 * cell + gap / 2.0, cell - gap, cell - gap];
        out.push(Sh::node("roundRect", b, spec.colors.accent(k)).with_item(spec, i, true).text_box(inset(b, 0.1)));
    }
    out
}

/// Overlapping circles round the middle; each one's text on its outer side.
fn venn(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let m = w.min(h);
    let (cx, cy) = (w / 2.0, h / 2.0);
    // Ring radius against circle radius: neighbours' centres 1.5 radii apart.
    let k = if n == 1 { 0.0 } else { (0.75 / (std::f32::consts::PI / n as f32).sin()).max(0.6) };
    let r = m / (2.0 * (k + 1.0));
    let start = if n == 2 { 180.0 } else { -90.0 };
    let place = |j: usize| -> (f32, f32, f32) {
        let deg = start + 360.0 * j as f32 / n as f32;
        let (x, y) = polar(cx, cy, r * k, deg);
        (x, y, deg)
    };
    let mut out = Vec::new();
    for (j, &i) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j)).of_item(i);
        s.fill = Paint::Accent(spec.colors.accent(j), Some(40_000));
        s.alpha = Some(60_000);
        s.line = Paint::None;
        out.push(s);
    }
    // Every outline over every fill, so each circle shows whole.
    for (j, _) in tops.iter().enumerate() {
        let (x, y, _) = place(j);
        let mut s = Sh::node("ellipse", [x - r, y - r, 2.0 * r, 2.0 * r], spec.colors.accent(j));
        s.fill = Paint::None;
        s.line = Paint::Accent(spec.colors.accent(j), None);
        s.line_w = 1.5;
        out.push(s);
    }
    for (j, &i) in tops.iter().enumerate() {
        let (x, y, deg) = place(j);
        let (tx, ty) = if n == 1 { (x, y) } else { polar(x, y, r * 0.35, deg) };
        let (bw, bh) = (r * 1.1, r * 0.8);
        let b = [tx - bw / 2.0, ty - bh / 2.0, bw, bh];
        let mut s = Sh::node("rect", b, spec.colors.accent(j)).with_item(spec, i, true).text_box(b);
        s.fill = Paint::None;
        s.line = Paint::None;
        s.ink = Ink::Dark;
        out.push(s);
    }
    out
}

// ---- timelines, team and text cards ----

/// The paint of top item `k` of `n`: its accent, graded from full strength to light across the
/// items when the graphic is in one accent.
fn graded(spec: &SmartArtSpec, k: usize, n: usize) -> Paint {
    let a = spec.colors.accent(k);
    if spec.colors == SmartArtColors::Colorful || n <= 1 {
        return Paint::Accent(a, None);
    }
    let t = 100_000 - 50_000 * k.min(n - 1) as u64 / (n - 1) as u64;
    Paint::Accent(a, u32::try_from(t).ok().filter(|t| *t < 100_000))
}

/// A light tint of top item `k`'s accent (`most` to `least` kept), lighter for later items in a
/// one-accent graphic.
fn graded_light(spec: &SmartArtSpec, k: usize, n: usize, most: u32, least: u32) -> Paint {
    let a = spec.colors.accent(k);
    if spec.colors == SmartArtColors::Colorful || n <= 1 {
        return Paint::Accent(a, Some(most));
    }
    let span = u64::from(most.saturating_sub(least));
    let t = u64::from(most) - span * k.min(n - 1) as u64 / (n - 1) as u64;
    Paint::Accent(a, Some(u32::try_from(t).unwrap_or(most)))
}

/// Text that reads on `p`: dark on any lightened accent and on no fill, light on full accents.
fn ink_on(p: Paint) -> Ink {
    match p {
        Paint::Accent(_, Some(t)) if t < 90_000 => Ink::Dark,
        Paint::Accent(..) => Ink::Light,
        Paint::Light | Paint::None => Ink::Dark,
    }
}

/// A shape filled with `fill`, no outline, text in a colour that reads on it.
fn plain(prst: &'static str, b: [f32; 4], fill: Paint) -> Sh {
    let mut s = Sh::node(prst, b, 1);
    s.fill = fill;
    s.line = Paint::None;
    s.ink = ink_on(fill);
    s
}

/// Item `i`'s own line (`own`) and the lines of the items under it (`kids`), without bullets.
fn lines_of(spec: &SmartArtSpec, i: usize, own: bool, kids: bool) -> Vec<(String, bool)> {
    let mut out = Vec::new();
    if own {
        out.push((spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default(), false));
    }
    if kids {
        out.extend(spec.descendants(i).filter_map(|j| spec.items.get(j)).map(|it| (it.text.clone(), true)));
    }
    out
}

/// Item `i`'s text (and the items under it) in a box with no fill, in dark text.
fn label(spec: &SmartArtSpec, i: usize, b: [f32; 4], algn: &'static str, anchor: &'static str) -> Sh {
    let mut s = plain("rect", b, Paint::None);
    s.text = lines_of(spec, i, true, true);
    s.item = Some(i);
    s.tb = b;
    s.algn = Some(algn);
    s.anchor = anchor;
    s
}

/// `s` with one line of `text` in `algn`, in `ink`, `scale` times the graphic's text size.
fn with_text(mut s: Sh, text: String, algn: &'static str, ink: Ink, scale: f32) -> Sh {
    s.text = vec![(text, false)];
    s.algn = Some(algn);
    s.ink = ink;
    s.scale = scale;
    s
}

/// A person placeholder in the square `b`: a circle in `bg` with a head and shoulders in `fg`.
fn avatar(b: [f32; 4], bg: Paint, fg: Paint, item: usize) -> Vec<Sh> {
    let [x, y, s, _] = b;
    let cx = x + s / 2.0;
    let circle = plain("ellipse", [x, y, s, s], bg).of_item(item);
    let head = plain("ellipse", [cx - 0.17 * s, y + 0.18 * s, 0.34 * s, 0.34 * s], fg);
    // The upper half of an ellipse whose box ends at the circle's bottom (no box leaves the circle's).
    let mut shoulders = plain("pie", [cx - 0.3 * s, y + 0.58 * s, 0.6 * s, 0.42 * s], fg);
    shoulders.adj = vec![("adj1", 10_800_000), ("adj2", 0)];
    vec![circle, head, shoulders]
}

/// Dots on a line across the middle; each item's text above its dot, then below, in turn,
/// joined to it by a short stem.
fn timeline_dots(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let cy = h / 2.0;
    let d = (cw * 0.16).min(h * 0.09);
    let gap = d / 2.0 + h * 0.04;
    let lw = if n > 1 { cw * 1.8 } else { cw * 0.96 }.min(w);
    let lh = (cy - gap).max(0.0);
    let mut line = Sh::line(0.0, cy, w, cy, spec.colors.accent(0));
    line.line_w = 2.0;
    let mut out = vec![line];
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        let x = (cx - lw / 2.0).clamp(0.0, (w - lw).max(0.0));
        let above = k % 2 == 0;
        let b = if above { [x, cy - gap - lh, lw, lh] } else { [x, cy + gap, lw, lh] };
        out.push(label(spec, i, b, "ctr", if above { "b" } else { "t" }));
        let end = if above { cy - gap } else { cy + gap };
        let mut stem = Sh::line(cx, cy, cx, end, spec.colors.accent(k));
        stem.line_w = 1.0;
        out.push(stem);
        let mut dot = plain("ellipse", [cx - d / 2.0, cy - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 1.5;
        out.push(dot);
    }
    out
}

/// Numbered circles joined by a line near the top; each item's text under its circle.
fn timeline_numbered(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.42).min(h * 0.3);
    // The circles and about a third of the height of text, centred top to bottom.
    let cy = ((h - d - h * 0.37) / 2.0).max(h * 0.05) + d / 2.0;
    let ty = cy + d / 2.0 + h * 0.05;
    let mut out = Vec::new();
    if n > 1 {
        let mut line = Sh::line(cw / 2.0, cy, w - cw / 2.0, cy, spec.colors.accent(0));
        line.line_w = 2.0;
        out.push(line);
    }
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        out.push(label(spec, i, [cx - cw * 0.47, ty, cw * 0.94, (h * 0.98 - ty).max(0.0)], "ctr", "t"));
        let p = graded(spec, k, n);
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        let mut c = with_text(plain("ellipse", b, p), (k + 1).to_string(), "ctr", ink_on(p), 1.0).of_item(i).text_box(inset(b, ROUND_INSET));
        c.line = Paint::Light;
        c.line_w = 2.0;
        out.push(c);
    }
    out
}

/// A light arrow-shaped band near the top with a dot per item on it; each item's text under its dot.
fn timeline_arrow(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let bh = h * 0.18;
    let by = ((h - bh - h * 0.37) / 2.0).max(h * 0.05);
    // The items share the band short of its point.
    let cw = (w - bh / 2.0).max(0.0) / n as f32;
    let d = (bh * 0.55).min(cw * 0.3);
    let cy = by + bh / 2.0;
    let ty = by + bh + h * 0.05;
    let mut out = vec![plain("homePlate", [0.0, by, w, bh], Paint::Accent(spec.colors.accent(0), Some(30_000)))];
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        out.push(label(spec, i, [cx - cw * 0.47, ty, cw * 0.94, (h * 0.98 - ty).max(0.0)], "ctr", "t"));
        let mut dot = plain("ellipse", [cx - d / 2.0, cy - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 2.0;
        out.push(dot);
    }
    out
}

/// Dots down a line in the middle; each item's text to the right of its dot, then the left, in turn.
fn timeline_vertical(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let x = w / 2.0;
    let d = (rh * 0.3).min(w * 0.05);
    let gap = d / 2.0 + w * 0.03;
    let mut out = Vec::new();
    if n > 1 {
        let mut line = Sh::line(x, rh / 2.0, x, h - rh / 2.0, spec.colors.accent(0));
        line.line_w = 2.0;
        out.push(line);
    }
    for (k, &i) in tops.iter().enumerate() {
        let (y, cy) = (rh * k as f32, rh * (k as f32 + 0.5));
        let tw = (x - gap).max(0.0);
        let sh = if k % 2 == 0 { label(spec, i, [x + gap, y, tw, rh], "l", "ctr") } else { label(spec, i, [0.0, y, tw, rh], "r", "ctr") };
        out.push(sh);
        let mut dot = plain("ellipse", [x - d / 2.0, cy - d / 2.0, d, d], graded(spec, k, n)).of_item(i);
        dot.line = Paint::Light;
        dot.line_w = 1.5;
        out.push(dot);
    }
    out
}

/// A person placeholder per item in a row; each item's text under it.
fn team_circles(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let s = (cw * 0.62).min(h * 0.5);
    let y0 = ((h - s - h * 0.37) / 2.0).max(h * 0.04);
    let ty = y0 + s + h * 0.05;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        out.push(label(spec, i, [cx - cw * 0.47, ty, cw * 0.94, (h * 0.98 - ty).max(0.0)], "ctr", "t"));
        out.extend(avatar([cx - s / 2.0, y0, s, s], graded(spec, k, n), Paint::Light, i));
    }
    out
}

/// A light card per item, stacked: a person placeholder on the left, the item's text beside it.
fn team_cards(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let ch = rh * 0.84;
    let s = (ch * 0.76).min(w * 0.3);
    let pad = ch * 0.12;
    let tx = pad + s + pad * 1.5;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32 + (rh - ch) / 2.0;
        let mut card = plain("roundRect", [0.0, y, w, ch], Paint::Accent(spec.colors.accent(k), Some(18_000)));
        card.text = lines_of(spec, i, true, true);
        card.item = Some(i);
        card.tb = [tx, y, (w - tx - pad).max(0.0), ch];
        card.algn = Some("l");
        out.push(card);
        out.extend(avatar([pad, y + (ch - s) / 2.0, s, s], graded(spec, k, n), Paint::Light, i));
    }
    out
}

/// Tall rounded cards in the items' colours: the text on top, a person placeholder at the bottom.
fn team_pills(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let pw = (cw * 0.84).min(h * 0.45);
    let (y0, ph) = (h * 0.02, h * 0.96);
    let s = pw * 0.72;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        let x = cx - pw / 2.0;
        let p = graded(spec, k, n);
        let mut pill = plain("roundRect", [x, y0, pw, ph], p);
        pill.adj = vec![("adj", 50_000)];
        pill.text = lines_of(spec, i, true, true);
        pill.item = Some(i);
        pill.tb = [x + pw * 0.1, y0 + pw * 0.3, pw * 0.8, (ph - pw * 0.45 - s).max(0.0)];
        pill.algn = Some("ctr");
        pill.anchor = "t";
        out.push(pill);
        let ay = y0 + ph - s - pw * 0.14;
        out.extend(avatar([cx - s / 2.0, ay, s, s], Paint::Light, Paint::Accent(spec.colors.accent(k), None), i));
    }
    out
}

/// Columns of text under a number in the item's colour: `big` writes large two-digit numbers
/// over a short rule; otherwise a single digit, with a bar down the column's left side.
fn card_numbered(spec: &SmartArtSpec, w: f32, h: f32, big: bool) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let bar = if big { 0.0 } else { (cw * 0.02).clamp(1.5, 4.0).min(cw * 0.1) };
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let col = cw * k as f32;
        let x0 = col + cw * 0.04;
        let tx = x0 + if big { 0.0 } else { bar + cw * 0.06 };
        let tw = (col + cw * 0.92 - tx).max(0.0);
        let body_y = if big { h * 0.4 } else { h * 0.32 };
        out.push(label(spec, i, [tx, body_y, tw, (h * 0.96 - body_y).max(0.0)], "l", "t"));
        let a = spec.colors.accent(k);
        if big {
            let mut rule = Sh::line(tx, h * 0.34, tx + tw * 0.3, h * 0.34, a);
            rule.line = graded(spec, k, n);
            rule.line_w = 2.0;
            out.push(rule);
        } else {
            out.push(plain("rect", [x0, h * 0.06, bar, h * 0.88], graded(spec, k, n)).of_item(i));
        }
        let num = if big { format!("{:02}", k + 1) } else { (k + 1).to_string() };
        let nb = [tx, h * 0.04, tw, h * 0.26];
        let mut s = with_text(plain("rect", nb, Paint::None), num, "l", Ink::Accent(a), if big { 2.0 } else { 1.5 }).text_box(nb);
        s.anchor = "b";
        out.push(s);
    }
    out
}

/// Light cards with the item's text in a band of its colour and the items under it below.
fn card_header(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let (y0, ch) = (h * 0.03, h * 0.94);
    let band = ch * 0.26;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (x0, card_w) = (cw * k as f32 + cw * 0.04, cw * 0.92);
        let mut card = plain("rect", [x0, y0, card_w, ch], Paint::Accent(spec.colors.accent(k), Some(15_000)));
        card.text = lines_of(spec, i, false, true);
        card.item = Some(i);
        card.tb = [x0 + card_w * 0.08, y0 + band + ch * 0.05, card_w * 0.84, (ch - band - ch * 0.1).max(0.0)];
        card.algn = Some("l");
        card.anchor = "t";
        out.push(card);
        let p = graded(spec, k, n);
        let bb = [x0, y0, card_w, band];
        let own = spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default();
        out.push(with_text(plain("rect", bb, p), own, "l", ink_on(p), 1.0).of_item(i).text_box([x0 + card_w * 0.08, y0, card_w * 0.84, band]));
    }
    out
}

/// Tall tinted cards, each with a numbered circle on top and the item's text under it.
fn card_tall(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let (y0, ch) = (h * 0.03, h * 0.94);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (x0, card_w) = (cw * k as f32 + cw * 0.05, cw * 0.9);
        let mut card = plain("roundRect", [x0, y0, card_w, ch], graded_light(spec, k, n, 45_000, 20_000));
        card.adj = vec![("adj", 8_000)];
        card.text = lines_of(spec, i, true, true);
        card.item = Some(i);
        card.tb = [x0 + card_w * 0.1, y0 + ch * 0.38, card_w * 0.8, ch * 0.56];
        card.algn = Some("ctr");
        card.anchor = "t";
        out.push(card);
        let d = (card_w * 0.4).min(ch * 0.24);
        let b = [x0 + (card_w - d) / 2.0, y0 + ch * 0.08, d, d];
        let a = spec.colors.accent(k);
        out.push(
            with_text(plain("ellipse", b, Paint::Light), (k + 1).to_string(), "ctr", Ink::Accent(a), 1.0).of_item(i).text_box(inset(b, ROUND_INSET)),
        );
    }
    out
}

/// Rows, each with a numbered square in the item's colour and the item's text beside it, a thin
/// rule between rows.
fn card_rows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let s = (rh * 0.7).min(w * 0.15);
    let tx = s + w * 0.04;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        out.push(label(spec, i, [tx, y, (w - tx).max(0.0), rh], "l", "ctr"));
        let p = graded(spec, k, n);
        let b = [0.0, y + (rh - s) / 2.0, s, s];
        out.push(with_text(plain("rect", b, p), (k + 1).to_string(), "ctr", ink_on(p), 1.0).of_item(i).text_box(inset(b, 0.15)));
        if k + 1 < n {
            let mut rule = Sh::line(tx, y + rh, w, y + rh, spec.colors.accent(k));
            rule.line_w = 0.75;
            out.push(rule);
        }
    }
    out
}

// ---- text ----

/// Whether `paras` fit a `w` × `h` text box at `size` points (sub-items at [`CHILD`] of it).
fn fits(paras: &[(String, bool)], size: f32, w: f32, h: f32) -> bool {
    if !(size.is_finite() && w.is_finite() && h.is_finite()) {
        return false;
    }
    let mut total = 0.0;
    for (t, child) in paras {
        let s = if *child { child_size(size) } else { size };
        if t.split_whitespace().any(|word| text_width(word, s) > w) {
            return false;
        }
        let lines = wrap(t, s, w, 10_000).len().max(1);
        total += lines as f32 * s * LINE;
    }
    total <= h
}

fn child_size(size: f32) -> f32 {
    (size * CHILD * 2.0).round() / 2.0
}

/// The largest text size (half points, [`MIN_SIZE`]–[`MAX_SIZE`]) at which every shape's text fits.
fn text_size(shapes: &[Sh]) -> f32 {
    let mut size = MAX_SIZE;
    while size > MIN_SIZE && !shapes.iter().filter(|s| !s.text.is_empty()).all(|s| fits(&s.text, size * s.scale, s.tb[2], s.tb[3])) {
        size -= 0.5;
    }
    size
}

// ---- parts ----

/// A length in points as EMU text, bounded.
fn emu(pt: f32) -> String {
    let v = if pt.is_finite() { (pt * EMU_PT).round() } else { 0.0 };
    format!("{}", v.clamp(-1.0e9, 1.0e9) as i64)
}

fn paint(w: &mut W, p: Paint, alpha: Option<u32>) {
    match p {
        Paint::None => w.empty("a:noFill", &[]),
        Paint::Light => {
            w.open("a:solidFill", &[]);
            w.empty("a:schemeClr", &[("val", "lt1")]);
            w.close("a:solidFill");
        }
        Paint::Accent(n, tint) => {
            w.open("a:solidFill", &[]);
            let val = format!("accent{}", n.clamp(1, 6));
            if tint.is_none() && alpha.is_none() {
                w.empty("a:schemeClr", &[("val", &val)]);
            } else {
                w.open("a:schemeClr", &[("val", &val)]);
                if let Some(t) = tint {
                    w.empty("a:tint", &[("val", &t.to_string())]);
                }
                if let Some(a) = alpha {
                    w.empty("a:alpha", &[("val", &a.to_string())]);
                }
                w.close("a:schemeClr");
            }
            w.close("a:solidFill");
        }
    }
}

/// The `dsp:drawing` of `spec` laid out `w` × `h` points: its shapes, back to front.
pub fn drawing_xml(spec: &SmartArtSpec, w: f32, h: f32) -> String {
    let shapes = layout(spec, w, h);
    let size = text_size(&shapes);
    let mut x = W::new();
    x.open("dsp:drawing", &[("xmlns:dgm", NS_DGM), ("xmlns:dsp", NS_DSP), ("xmlns:a", NS_A)]);
    x.open("dsp:spTree", &[]);
    x.open("dsp:nvGrpSpPr", &[]);
    x.empty("dsp:cNvPr", &[("id", "0"), ("name", "")]);
    x.empty("dsp:cNvGrpSpPr", &[]);
    x.close("dsp:nvGrpSpPr");
    x.empty("dsp:grpSpPr", &[]);
    for (k, sh) in shapes.iter().enumerate() {
        shape_xml(&mut x, sh, &model_id(SHAPE_IDS + k), size);
    }
    x.close("dsp:spTree");
    x.close("dsp:drawing");
    x.s
}

fn shape_xml(x: &mut W, sh: &Sh, id: &str, size: f32) {
    x.open("dsp:sp", &[("modelId", id)]);
    x.open("dsp:nvSpPr", &[]);
    x.empty("dsp:cNvPr", &[("id", "0"), ("name", "")]);
    x.empty("dsp:cNvSpPr", &[]);
    x.close("dsp:nvSpPr");
    x.open("dsp:spPr", &[]);
    let rot = ((sh.rot.rem_euclid(360.0) * 60_000.0).round() as i64).to_string();
    let mut attrs: Vec<(&str, &str)> = Vec::new();
    if sh.rot != 0.0 {
        attrs.push(("rot", &rot));
    }
    if sh.flip_h {
        attrs.push(("flipH", "1"));
    }
    x.open("a:xfrm", &attrs);
    x.empty("a:off", &[("x", &emu(sh.b[0])), ("y", &emu(sh.b[1]))]);
    x.empty("a:ext", &[("cx", &emu(sh.b[2].max(0.0))), ("cy", &emu(sh.b[3].max(0.0)))]);
    x.close("a:xfrm");
    x.open("a:prstGeom", &[("prst", sh.prst)]);
    if sh.adj.is_empty() {
        x.empty("a:avLst", &[]);
    } else {
        x.open("a:avLst", &[]);
        for (name, v) in &sh.adj {
            x.empty("a:gd", &[("name", name), ("fmla", &format!("val {v}"))]);
        }
        x.close("a:avLst");
    }
    x.close("a:prstGeom");
    paint(x, sh.fill, sh.alpha);
    if sh.line != Paint::None && sh.line_w > 0.0 {
        x.open("a:ln", &[("w", &emu(sh.line_w))]);
        paint(x, sh.line, None);
        x.close("a:ln");
    } else {
        x.open("a:ln", &[]);
        x.empty("a:noFill", &[]);
        x.close("a:ln");
    }
    x.close("dsp:spPr");
    let text_clr = sh.ink.scheme();
    let text_clr = text_clr.as_str();
    let size = size * if sh.scale.is_finite() { sh.scale.clamp(0.1, 10.0) } else { 1.0 };
    x.open("dsp:style", &[]);
    for (name, idx) in [("a:lnRef", "2"), ("a:fillRef", "1"), ("a:effectRef", "0")] {
        x.open(name, &[("idx", idx)]);
        x.empty("a:scrgbClr", &[("r", "0"), ("g", "0"), ("b", "0")]);
        x.close(name);
    }
    x.open("a:fontRef", &[("idx", "minor")]);
    x.empty("a:schemeClr", &[("val", text_clr)]);
    x.close("a:fontRef");
    x.close("dsp:style");
    if !sh.text.is_empty() {
        x.open("dsp:txBody", &[]);
        x.open(
            "a:bodyPr",
            &[
                ("spcFirstLastPara", "0"),
                ("vert", "horz"),
                ("wrap", "square"),
                ("lIns", "0"),
                ("tIns", "0"),
                ("rIns", "0"),
                ("bIns", "0"),
                ("numCol", "1"),
                ("spcCol", "0"),
                ("anchor", sh.anchor),
                ("anchorCtr", "0"),
            ],
        );
        x.empty("a:noAutofit", &[]);
        x.close("a:bodyPr");
        x.empty("a:lstStyle", &[]);
        for (t, child) in &sh.text {
            let s = if *child { child_size(size) } else { size };
            x.open("a:p", &[]);
            let algn = sh.algn.unwrap_or(if *child { "l" } else { "ctr" });
            x.open("a:pPr", &[("lvl", if *child { "1" } else { "0" }), ("algn", algn)]);
            x.empty("a:buNone", &[]);
            x.close("a:pPr");
            if t.is_empty() {
                x.empty("a:endParaRPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
            } else {
                x.open("a:r", &[]);
                x.open("a:rPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
                x.open("a:solidFill", &[]);
                x.empty("a:schemeClr", &[("val", text_clr)]);
                x.close("a:solidFill");
                x.close("a:rPr");
                x.leaf("a:t", &[], t);
                x.close("a:r");
            }
            x.close("a:p");
        }
        x.close("dsp:txBody");
        x.open("dsp:txXfrm", &[]);
        x.empty("a:off", &[("x", &emu(sh.tb[0])), ("y", &emu(sh.tb[1]))]);
        x.empty("a:ext", &[("cx", &emu(sh.tb[2].max(0.0))), ("cy", &emu(sh.tb[3].max(0.0)))]);
        x.close("dsp:txXfrm");
    }
    x.close("dsp:sp");
}

/// What `spec` draws as, `w` × `h` points, in the colours of `theme`: its drawing part, read the
/// way SmartArt drawings from files are.
pub fn smart_art_items(spec: &SmartArtSpec, theme: &[Rgb], w: f32, h: f32) -> Vec<GraphicItem> {
    match xml::parse(drawing_xml(spec, w, h).as_bytes()) {
        Ok(d) => crate::read::diagram::diagram_items(&d, theme, w, h, &mut |_| None),
        Err(_) => Vec::new(),
    }
}

/// One outline item's shape in a laid-out graphic, for picking an item inside it.
#[derive(Clone, Debug, PartialEq)]
pub struct SmartArtNode {
    /// The item's index in [`SmartArtSpec::items`].
    pub item: usize,
    /// Its box (`x`, `y`, `w`, `h`, points from the graphic's top-left), before `rot`.
    pub b: [f32; 4],
    /// Clockwise rotation in degrees about the box centre.
    pub rot: f32,
    /// The preset geometry (`rect`, `roundRect`, `ellipse`, `triangle`, `trapezoid`, …).
    pub prst: &'static str,
}

/// The item shapes of `spec` laid out `w` × `h` points, back to front, one per item (the first
/// shape drawn for it, e.g. a Venn item's circle rather than its label). Items drawn only as
/// bullets inside their parent have none.
pub fn smart_art_nodes(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<SmartArtNode> {
    let mut out: Vec<SmartArtNode> = Vec::new();
    for sh in layout(spec, w, h) {
        if let Some(item) = sh.item
            && !out.iter().any(|n| n.item == item)
        {
            out.push(SmartArtNode { item, b: sh.b, rot: sh.rot, prst: sh.prst });
        }
    }
    out
}

/// The item whose shape is under (`x`, `y`) (points from the graphic's top-left), the topmost
/// one when shapes overlap.
pub fn smart_art_item_at(spec: &SmartArtSpec, w: f32, h: f32, x: f32, y: f32) -> Option<usize> {
    let nodes = smart_art_nodes(spec, w, h);
    nodes.iter().rev().find(|n| node_contains(n, x, y)).map(|n| n.item)
}

fn node_contains(n: &SmartArtNode, x: f32, y: f32) -> bool {
    let [bx, by, bw, bh] = n.b;
    let (cx, cy) = (bx + bw / 2.0, by + bh / 2.0);
    // Into the box's own frame (undo its rotation).
    let (s, c) = (-n.rot).to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    let (lx, ly) = (dx * c - dy * s, dx * s + dy * c);
    let (hw, hh) = (bw / 2.0, bh / 2.0);
    if hw <= 0.0 || hh <= 0.0 || lx.abs() > hw || ly.abs() > hh {
        return false;
    }
    match n.prst {
        "ellipse" => (lx / hw).powi(2) + (ly / hh).powi(2) <= 1.0,
        // Apex at the top middle: the half-width grows from 0 at the top to `hw` at the bottom.
        "triangle" => lx.abs() <= hw * (ly + hh) / (2.0 * hh),
        _ => true,
    }
}

/// Ids of the points and connections of item `i`: (node, connection, parent transition, sibling
/// transition).
fn item_ids(i: usize) -> (String, String, String, String) {
    let b = 1 + 4 * i;
    (model_id(b), model_id(b + 1), model_id(b + 2), model_id(b + 3))
}

/// An empty point text body.
fn empty_t(w: &mut W) {
    w.open("dgm:t", &[]);
    w.empty("a:bodyPr", &[]);
    w.empty("a:lstStyle", &[]);
    w.open("a:p", &[]);
    w.empty("a:endParaRPr", &[("lang", "en-US")]);
    w.close("a:p");
    w.close("dgm:t");
}

/// The data part (`dgm:dataModel`) of `spec`; its drawing is the story part's relationship
/// `drawing_rel`.
pub fn data_xml(spec: &SmartArtSpec, drawing_rel: &str) -> String {
    let (lo, lo_cat) = layout_id(spec.layout);
    let (cs, cs_cat) = colors_id(spec.colors);
    let doc = model_id(0);
    let mut w = W::new();
    w.open("dgm:dataModel", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A)]);
    w.open("dgm:ptLst", &[]);
    w.open("dgm:pt", &[("modelId", &doc), ("type", "doc")]);
    w.empty(
        "dgm:prSet",
        &[("loTypeId", lo), ("loCatId", lo_cat), ("qsTypeId", QUICK_STYLE_ID), ("qsCatId", "simple"), ("csTypeId", &cs), ("csCatId", &cs_cat)],
    );
    w.empty("dgm:spPr", &[]);
    empty_t(&mut w);
    w.close("dgm:pt");
    for (i, it) in spec.items.iter().enumerate() {
        let (node, cxn, par, sib) = item_ids(i);
        w.open("dgm:pt", &[("modelId", &node)]);
        w.empty("dgm:prSet", &[]);
        w.empty("dgm:spPr", &[]);
        w.open("dgm:t", &[]);
        w.empty("a:bodyPr", &[]);
        w.empty("a:lstStyle", &[]);
        w.open("a:p", &[]);
        w.open("a:r", &[]);
        w.empty("a:rPr", &[("lang", "en-US")]);
        w.leaf("a:t", &[], &it.text);
        w.close("a:r");
        w.close("a:p");
        w.close("dgm:t");
        w.close("dgm:pt");
        for (id, ty) in [(&par, "parTrans"), (&sib, "sibTrans")] {
            w.open("dgm:pt", &[("modelId", id), ("type", ty), ("cxnId", &cxn)]);
            w.empty("dgm:prSet", &[]);
            w.empty("dgm:spPr", &[]);
            empty_t(&mut w);
            w.close("dgm:pt");
        }
    }
    w.close("dgm:ptLst");
    w.open("dgm:cxnLst", &[]);
    let parents = spec.parents();
    let mut order = vec![0usize; spec.items.len() + 1];
    for (i, p) in parents.iter().enumerate() {
        let (node, cxn, par, sib) = item_ids(i);
        let src = p.map_or_else(|| doc.clone(), |p| item_ids(p).0);
        let slot = p.map_or(0, |p| p + 1);
        let ord = order.get(slot).copied().unwrap_or(0);
        if let Some(o) = order.get_mut(slot) {
            *o += 1;
        }
        w.empty(
            "dgm:cxn",
            &[
                ("modelId", &cxn),
                ("srcId", &src),
                ("destId", &node),
                ("srcOrd", &ord.to_string()),
                ("destOrd", "0"),
                ("parTransId", &par),
                ("sibTransId", &sib),
            ],
        );
    }
    w.close("dgm:cxnLst");
    w.empty("dgm:bg", &[]);
    w.empty("dgm:whole", &[]);
    w.open("dgm:extLst", &[]);
    w.open("a:ext", &[("uri", DRAWING_EXT_URI)]);
    w.empty(
        "dsp:dataModelExt",
        &[("xmlns:dsp", NS_DSP), ("relId", drawing_rel), ("minVer", "http://schemas.openxmlformats.org/drawingml/2006/diagram")],
    );
    w.close("a:ext");
    w.close("dgm:extLst");
    w.close("dgm:dataModel");
    w.s
}

/// A layout node: `alg` with its parameters, a shape (`None`: no geometry), what text it shows
/// (`presOf` attributes), then `body`.
struct LayoutNode<'a> {
    name: &'a str,
    style: Option<&'a str>,
    alg: &'a str,
    params: &'a [(&'a str, &'a str)],
    shape: Option<&'a str>,
    pres_of: &'a [(&'a str, &'a str)],
}

fn layout_node(w: &mut W, node: LayoutNode<'_>, body: impl FnOnce(&mut W)) {
    let LayoutNode { name, style, alg, params, shape, pres_of } = node;
    let mut attrs = vec![("name", name)];
    if let Some(s) = style {
        attrs.push(("styleLbl", s));
    }
    w.open("dgm:layoutNode", &attrs);
    if params.is_empty() {
        w.empty("dgm:alg", &[("type", alg)]);
    } else {
        w.open("dgm:alg", &[("type", alg)]);
        for (t, v) in params {
            w.empty("dgm:param", &[("type", t), ("val", v)]);
        }
        w.close("dgm:alg");
    }
    let mut sa = vec![("r:blip", "")];
    if let Some(t) = shape {
        sa.insert(0, ("type", t));
    }
    w.open("dgm:shape", &sa);
    w.empty("dgm:adjLst", &[]);
    w.close("dgm:shape");
    w.empty("dgm:presOf", pres_of);
    w.empty("dgm:constrLst", &[]);
    w.empty("dgm:ruleLst", &[]);
    body(w);
    w.close("dgm:layoutNode");
}

/// The text node of an item: its own text and the items under it as bullets.
const ITEM_TEXT: &[(&str, &str)] = &[("axis", "desc;self"), ("ptType", "node;node"), ("st", "1;1"), ("cnt", "0;1")];

/// A layout definition (`dgm:layoutDef`) of our own for `l`, named by its layout identifier.
pub fn layout_xml(l: SmartArtLayout) -> String {
    let (id, cat) = layout_id(l);
    let mut w = W::new();
    w.open("dgm:layoutDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("xmlns:r", NS_R), ("uniqueId", id)]);
    w.empty("dgm:title", &[("val", l.label())]);
    w.empty("dgm:desc", &[("val", l.description())]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", cat), ("pri", "1000")]);
    w.close("dgm:catLst");
    let (alg, params): (&str, &[(&str, &str)]) = match l {
        SmartArtLayout::BasicList | SmartArtLayout::Matrix => ("snake", &[("grDir", "tL"), ("flowDir", "row"), ("contDir", "sameDir")]),
        SmartArtLayout::Process => ("lin", &[("linDir", "fromL"), ("nodeVertAlign", "mid")]),
        SmartArtLayout::Cycle | SmartArtLayout::Venn => ("cycle", &[("stAng", "0"), ("spanAng", "360")]),
        SmartArtLayout::Radial => ("cycle", &[("stAng", "0"), ("spanAng", "360"), ("ctrShpMap", "fNode")]),
        SmartArtLayout::Hierarchy => ("hierChild", &[("linDir", "fromL"), ("chAlign", "t")]),
        SmartArtLayout::Pyramid => ("pyra", &[("linDir", "fromT")]),
        SmartArtLayout::TimelineDots
        | SmartArtLayout::TimelineNumbered
        | SmartArtLayout::TimelineArrow
        | SmartArtLayout::TeamCircles
        | SmartArtLayout::TeamPills
        | SmartArtLayout::CardNumbered
        | SmartArtLayout::CardBigNumbers
        | SmartArtLayout::CardHeader
        | SmartArtLayout::CardTall => ("lin", &[("linDir", "fromL"), ("nodeVertAlign", "t")]),
        SmartArtLayout::TimelineVertical | SmartArtLayout::TeamCards | SmartArtLayout::CardRows => ("lin", &[("linDir", "fromT")]),
    };
    let shape = match l {
        SmartArtLayout::BasicList
        | SmartArtLayout::CardNumbered
        | SmartArtLayout::CardBigNumbers
        | SmartArtLayout::CardHeader
        | SmartArtLayout::CardRows
        | SmartArtLayout::TimelineVertical => "rect",
        SmartArtLayout::Process
        | SmartArtLayout::Hierarchy
        | SmartArtLayout::Matrix
        | SmartArtLayout::TeamCards
        | SmartArtLayout::TeamPills
        | SmartArtLayout::CardTall => "roundRect",
        SmartArtLayout::Cycle
        | SmartArtLayout::Radial
        | SmartArtLayout::Venn
        | SmartArtLayout::TimelineDots
        | SmartArtLayout::TimelineNumbered
        | SmartArtLayout::TimelineArrow
        | SmartArtLayout::TeamCircles => "ellipse",
        SmartArtLayout::Pyramid => "trapezoid",
    };
    layout_node(&mut w, LayoutNode { name: "diagram", style: None, alg, params, shape: None, pres_of: &[] }, |w| match l {
        SmartArtLayout::Hierarchy => {
            w.open("dgm:forEach", &[("name", "branches"), ("axis", "ch"), ("ptType", "node")]);
            layout_node(w, LayoutNode { name: "branch", style: None, alg: "hierRoot", params: &[], shape: None, pres_of: &[] }, |w| {
                layout_node(
                    w,
                    LayoutNode { name: "item", style: Some("node1"), alg: "tx", params: &[], shape: Some(shape), pres_of: &[("axis", "self")] },
                    |_| {},
                );
                layout_node(
                    w,
                    LayoutNode { name: "below", style: None, alg: "hierChild", params: &[("chAlign", "t")], shape: None, pres_of: &[] },
                    |w| {
                        w.open("dgm:forEach", &[("name", "joins"), ("axis", "ch"), ("ptType", "parTrans")]);
                        layout_node(
                            w,
                            LayoutNode {
                                name: "join",
                                style: Some("connector"),
                                alg: "conn",
                                params: &[("dim", "1D"), ("connRout", "bend"), ("begPts", "bCtr"), ("endPts", "tCtr")],
                                shape: Some("conn"),
                                pres_of: &[("axis", "self")],
                            },
                            |_| {},
                        );
                        w.close("dgm:forEach");
                        w.empty("dgm:forEach", &[("name", "deeper"), ("ref", "branches")]);
                    },
                );
            });
            w.close("dgm:forEach");
        }
        SmartArtLayout::Radial => {
            w.open("dgm:forEach", &[("name", "middle"), ("axis", "ch"), ("ptType", "node"), ("cnt", "1")]);
            layout_node(
                w,
                LayoutNode { name: "centre", style: Some("node1"), alg: "tx", params: &[], shape: Some(shape), pres_of: &[("axis", "self")] },
                |w| {
                    w.open("dgm:forEach", &[("name", "around"), ("axis", "ch"), ("ptType", "node")]);
                    layout_node(
                        w,
                        LayoutNode { name: "spoke", style: Some("node1"), alg: "tx", params: &[], shape: Some(shape), pres_of: ITEM_TEXT },
                        |_| {},
                    );
                    w.close("dgm:forEach");
                },
            );
            w.close("dgm:forEach");
        }
        _ => {
            w.open("dgm:forEach", &[("name", "items"), ("axis", "ch"), ("ptType", "node")]);
            let style = if l == SmartArtLayout::Venn { "venn" } else { "node1" };
            layout_node(w, LayoutNode { name: "item", style: Some(style), alg: "tx", params: &[], shape: Some(shape), pres_of: ITEM_TEXT }, |_| {});
            if matches!(l, SmartArtLayout::Process | SmartArtLayout::Cycle) {
                w.open("dgm:forEach", &[("name", "arrows"), ("axis", "followSib"), ("ptType", "sibTrans"), ("cnt", "1")]);
                layout_node(
                    w,
                    LayoutNode {
                        name: "arrow",
                        style: Some("connector"),
                        alg: "conn",
                        params: &[("dim", "2D")],
                        shape: Some("rightArrow"),
                        pres_of: &[("axis", "self")],
                    },
                    |_| {},
                );
                w.close("dgm:forEach");
            }
            w.close("dgm:forEach");
        }
    });
    w.close("dgm:layoutDef");
    w.s
}

/// The style labels our layouts name.
const STYLE_LABELS: [&str; 3] = ["node1", "connector", "venn"];

fn scheme_list(w: &mut W, tag: &str, meth: &str, vals: &[String]) {
    w.open(tag, &[("meth", meth)]);
    for v in vals {
        w.empty("a:schemeClr", &[("val", v)]);
    }
    w.close(tag);
}

/// A colour definition (`dgm:colorsDef`) of our own for `c`.
pub fn colors_xml(c: SmartArtColors) -> String {
    let (id, cat) = colors_id(c);
    let accents: Vec<String> = match c {
        SmartArtColors::Colorful => (1..=6).map(|n| format!("accent{n}")).collect(),
        _ => vec![format!("accent{}", c.accent(0))],
    };
    let meth = if c == SmartArtColors::Colorful { "cycle" } else { "repeat" };
    let mut w = W::new();
    w.open("dgm:colorsDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("uniqueId", &id)]);
    w.empty("dgm:title", &[("val", c.label())]);
    w.empty("dgm:desc", &[("val", "")]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", &cat), ("pri", "1000")]);
    w.close("dgm:catLst");
    for lbl in STYLE_LABELS {
        w.open("dgm:styleLbl", &[("name", lbl)]);
        scheme_list(&mut w, "dgm:fillClrLst", meth, &accents);
        scheme_list(&mut w, "dgm:linClrLst", "repeat", &["lt1".to_string()]);
        w.empty("dgm:effectClrLst", &[]);
        w.empty("dgm:txLinClrLst", &[]);
        scheme_list(&mut w, "dgm:txFillClrLst", "repeat", &[if lbl == "venn" { "tx1" } else { "lt1" }.to_string()]);
        w.empty("dgm:txEffectClrLst", &[]);
        w.close("dgm:styleLbl");
    }
    w.close("dgm:colorsDef");
    w.s
}

/// The quick style (`dgm:styleDef`) of our own: flat shapes in the theme's plain line and fill.
pub fn quick_style_xml() -> String {
    let mut w = W::new();
    w.open("dgm:styleDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("uniqueId", QUICK_STYLE_ID)]);
    w.empty("dgm:title", &[("val", "Flat")]);
    w.empty("dgm:desc", &[("val", "")]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", "simple"), ("pri", "1000")]);
    w.close("dgm:catLst");
    let scene = |w: &mut W| {
        w.open("dgm:scene3d", &[]);
        w.empty("a:camera", &[("prst", "orthographicFront")]);
        w.empty("a:lightRig", &[("rig", "threePt"), ("dir", "t")]);
        w.close("dgm:scene3d");
    };
    scene(&mut w);
    for lbl in STYLE_LABELS {
        w.open("dgm:styleLbl", &[("name", lbl)]);
        scene(&mut w);
        w.empty("dgm:sp3d", &[]);
        w.empty("dgm:txPr", &[]);
        w.open("dgm:style", &[]);
        for (name, idx) in [("a:lnRef", "2"), ("a:fillRef", "1"), ("a:effectRef", "0")] {
            w.open(name, &[("idx", idx)]);
            w.empty("a:scrgbClr", &[("r", "0"), ("g", "0"), ("b", "0")]);
            w.close(name);
        }
        w.open("a:fontRef", &[("idx", "minor")]);
        w.empty("a:schemeClr", &[("val", if lbl == "venn" { "tx1" } else { "lt1" })]);
        w.close("a:fontRef");
        w.close("dgm:style");
        w.close("dgm:styleLbl");
    }
    w.close("dgm:styleDef");
    w.s
}

// ---- reading back ----

/// The graphic a data part holds, when the part is exactly what [`data_xml`] writes for it (so
/// editing and saving it loses nothing). `None` for any other diagram.
pub(crate) fn spec_of(data: &El) -> Option<SmartArtSpec> {
    let pts = data.child("dgm:ptLst")?;
    let doc = pts.children("dgm:pt").find(|p| p.attr("type") == Some("doc"))?;
    let pr = doc.child("dgm:prSet")?;
    let lo = pr.attr("loTypeId")?;
    let layout = SmartArtLayout::ALL.into_iter().find(|l| layout_id(*l).0 == lo)?;
    let cs = pr.attr("csTypeId")?;
    let colors = SmartArtColors::ALL.into_iter().find(|c| colors_id(*c).0 == cs)?;
    let rel = data.find("dsp:dataModelExt")?.attr("relId")?.to_string();
    // Items are node points (no type) in order; more than the model holds can't be ours.
    let nodes: Vec<&El> = pts.children("dgm:pt").filter(|p| p.attr("type").is_none()).take(MAX_ITEMS + 1).collect();
    if nodes.len() > MAX_ITEMS {
        return None;
    }
    let doc_id = doc.attr("modelId")?;
    let index: Vec<&str> = nodes.iter().filter_map(|p| p.attr("modelId")).collect();
    let mut levels: Vec<Option<u8>> = vec![None; nodes.len()];
    for c in data.child("dgm:cxnLst")?.children("dgm:cxn").take(MAX_ITEMS + 1) {
        let dest = index.iter().position(|id| Some(*id) == c.attr("destId"))?;
        let src = c.attr("srcId")?;
        let level = if src == doc_id {
            0
        } else {
            let p = index.iter().position(|id| *id == src)?;
            levels.get(p).copied().flatten()?.checked_add(1)?
        };
        *levels.get_mut(dest)? = Some(level);
    }
    let items = nodes
        .iter()
        .zip(&levels)
        .map(|(p, l)| {
            let text: String = p.child("dgm:t")?.child("a:p")?.children("a:r").filter_map(|r| r.child("a:t")).map(|t| t.text()).collect();
            Some(SmartArtItem { text, level: (*l)? })
        })
        .collect::<Option<Vec<_>>>()?;
    let spec = SmartArtSpec { layout, items, colors }.sanitized();
    // Editable only when nothing would be lost: writing the model again gives the same part.
    let again = xml::parse(data_xml(&spec, &rel).as_bytes()).ok()?;
    same(data, &again, 0).then_some(spec)
}

/// Whether two elements are the same markup: names, attributes (in any order), child elements in
/// order and text (whitespace-only text between elements ignored).
fn same(a: &El, b: &El, depth: usize) -> bool {
    if depth > MAX_DEPTH || a.name != b.name || a.attrs.len() != b.attrs.len() {
        return false;
    }
    if !a.attrs.iter().all(|(k, v)| b.attr(k) == Some(v.as_str())) {
        return false;
    }
    let text = |e: &El| -> String {
        e.kids
            .iter()
            .filter_map(|n| match n {
                Node::Text(t) if !t.trim().is_empty() => Some(t.as_str()),
                _ => None,
            })
            .collect()
    };
    if text(a) != text(b) {
        return false;
    }
    let (mut x, mut y) = (a.els(), b.els());
    loop {
        match (x.next(), y.next()) {
            (None, None) => return true,
            (Some(p), Some(q)) if same(p, q, depth + 1) => {}
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wordcraft_doc::THEME_COLORS;

    fn parse(s: &str) -> El {
        xml::parse(s.as_bytes()).unwrap()
    }

    fn names(e: &El) -> Vec<&str> {
        e.els().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn every_layout_reads_back_and_draws_inside_its_frame() {
        for layout in SmartArtLayout::ALL {
            for colors in [SmartArtColors::Accent3, SmartArtColors::Colorful] {
                let mut spec = SmartArtSpec::sample(layout);
                spec.colors = colors;
                spec.items.push(SmartArtItem::new("Under <last> & more", 1));
                let spec = spec.sanitized();
                assert_eq!(spec_of(&parse(&data_xml(&spec, "rId9"))).as_ref(), Some(&spec), "{layout:?}");
                let items = smart_art_items(&spec, &THEME_COLORS, 400.0, 250.0);
                assert!(items.iter().any(|i| matches!(i, GraphicItem::Text { .. })), "{layout:?} has text");
                for it in &items {
                    if let GraphicItem::Shape { rect, .. } = it {
                        assert!(
                            rect[0] >= -0.5 && rect[1] >= -0.5 && rect[0] + rect[2] <= 400.5 && rect[1] + rect[3] <= 250.5,
                            "{layout:?}: {rect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn every_top_item_has_a_node_and_picking_its_centre_finds_it() {
        for layout in SmartArtLayout::ALL {
            let spec = SmartArtSpec::sample(layout).sanitized();
            let nodes = smart_art_nodes(&spec, 400.0, 250.0);
            for (i, it) in spec.items.iter().enumerate() {
                // Hierarchy and radial draw every item as its own shape; the others draw sub-items as bullets.
                let own = it.level == 0 || matches!(layout, SmartArtLayout::Hierarchy | SmartArtLayout::Radial);
                let node = nodes.iter().find(|n| n.item == i);
                // Sub-items of the other layouts are lines in their item's text, never their own node.
                if !own {
                    assert!(node.is_none(), "{layout:?}: sub-item {i} has a node");
                }
                if !own {
                    continue;
                }
                let n = node.unwrap_or_else(|| panic!("{layout:?}: item {i} has no node"));
                // The middle of a pyramid's top triangle and of a Venn circle's own label side may be overlapped;
                // the box centre lies inside its own shape for every preset we draw.
                let (cx, cy) = (n.b[0] + n.b[2] / 2.0, n.b[1] + n.b[3] / 2.0 + if n.prst == "triangle" { n.b[3] / 4.0 } else { 0.0 });
                if layout != SmartArtLayout::Venn {
                    assert_eq!(smart_art_item_at(&spec, 400.0, 250.0, cx, cy), Some(i), "{layout:?} item {i} at {cx},{cy}");
                }
            }
            assert_eq!(smart_art_item_at(&spec, 400.0, 250.0, -10.0, -10.0), None, "{layout:?}: outside");
        }
    }

    #[test]
    fn every_shape_box_stays_in_the_frame_and_text_on_light_fills_is_dark() {
        for layout in SmartArtLayout::ALL {
            for n in [1usize, 3, 7] {
                let mut spec = SmartArtSpec::sample(layout);
                spec.items = (0..n).flat_map(|k| [SmartArtItem::new(format!("Item {k}"), 0), SmartArtItem::new("line", 1)]).collect();
                let spec = spec.sanitized();
                for (w, h) in [(432.0, 252.0), (120.0, 300.0), (600.0, 80.0)] {
                    for sh in super::layout(&spec, w, h) {
                        let [x, y, bw, bh] = sh.b;
                        assert!(x >= -0.5 && y >= -0.5 && x + bw <= w + 0.5 && y + bh <= h + 0.5, "{layout:?} {n} {w}x{h}: {:?} {:?}", sh.prst, sh.b);
                        if !sh.text.is_empty()
                            && let Paint::Accent(_, Some(t)) = sh.fill
                            && t < 90_000
                        {
                            assert_ne!(sh.ink, Ink::Light, "{layout:?}: light text on a light fill");
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn sub_items_of_a_list_are_bullets_without_their_own_node() {
        let mut spec = SmartArtSpec::sample(SmartArtLayout::BasicList);
        spec.items.insert(1, SmartArtItem::new("under", 1));
        let spec = spec.sanitized();
        let nodes = smart_art_nodes(&spec, 400.0, 250.0);
        assert!(nodes.iter().all(|n| n.item != 1));
        assert!(nodes.iter().any(|n| n.item == 0) && nodes.iter().any(|n| n.item == 2));
    }

    #[test]
    fn a_diagram_with_more_than_the_model_stays_read_only() {
        let spec = SmartArtSpec::sample(SmartArtLayout::Process);
        let x = data_xml(&spec, "rId4");
        assert_eq!(spec_of(&parse(&x.replace("<dgm:bg/>", "<dgm:bg><a:noFill/></dgm:bg>"))), None);
        assert_eq!(spec_of(&parse(&x.replace("layout/process1", "layout/process9"))), None);
        // A run split in two (as another program might save it) isn't ours either.
        let split = x.replace(
            "<a:r><a:rPr lang=\"en-US\"/><a:t>Plan</a:t></a:r>",
            "<a:r><a:rPr lang=\"en-US\"/><a:t>Pl</a:t></a:r><a:r><a:rPr lang=\"en-US\"/><a:t>an</a:t></a:r>",
        );
        assert_ne!(split, x);
        assert_eq!(spec_of(&parse(&split)), None);
    }

    #[test]
    fn parts_come_in_schema_order() {
        let spec = SmartArtSpec::sample(SmartArtLayout::Hierarchy);
        let data = parse(&data_xml(&spec, "rId1"));
        assert_eq!(names(&data), ["dgm:ptLst", "dgm:cxnLst", "dgm:bg", "dgm:whole", "dgm:extLst"]);
        let pt = data.child("dgm:ptLst").unwrap().els().nth(1).unwrap();
        assert_eq!(names(pt), ["dgm:prSet", "dgm:spPr", "dgm:t"]);
        // Each item's connection names its parent: Design (item 1) under Lead (item 0).
        let cx = data.child("dgm:cxnLst").unwrap().els().nth(1).unwrap();
        assert_eq!(cx.attr("srcId"), Some(model_id(1).as_str()));
        let drawing = parse(&drawing_xml(&spec, 300.0, 200.0));
        let tree = drawing.child("dsp:spTree").unwrap();
        assert_eq!(names(tree).first(), Some(&"dsp:nvGrpSpPr"));
        let sp = tree.children("dsp:sp").last().unwrap();
        assert_eq!(names(sp), ["dsp:nvSpPr", "dsp:spPr", "dsp:style", "dsp:txBody", "dsp:txXfrm"]);
        assert_eq!(names(sp.child("dsp:spPr").unwrap()), ["a:xfrm", "a:prstGeom", "a:solidFill", "a:ln"]);
        for part in [layout_xml(SmartArtLayout::Radial), colors_xml(SmartArtColors::Colorful), quick_style_xml()] {
            let el = parse(&part);
            assert_eq!(&names(&el)[..3], ["dgm:title", "dgm:desc", "dgm:catLst"]);
        }
    }

    #[test]
    fn hostile_sizes_and_many_items_stay_finite() {
        let spec =
            SmartArtSpec { items: (0..MAX_ITEMS).map(|i| SmartArtItem::new("word ".repeat(50), (i % 9) as u8)).collect(), ..Default::default() }
                .sanitized();
        for layout in SmartArtLayout::ALL {
            let spec = SmartArtSpec { layout, ..spec.clone() };
            for (w, h) in [(f32::NAN, 10.0), (0.0, 0.0), (1.0e30, 5.0), (300.0, 200.0)] {
                for it in smart_art_items(&spec, &THEME_COLORS, w, h) {
                    if let GraphicItem::Shape { rect, .. } | GraphicItem::Text { rect, .. } = it {
                        assert!(rect.iter().all(|v| v.is_finite()), "{layout:?} {w} {h}");
                    }
                }
            }
        }
    }
}
