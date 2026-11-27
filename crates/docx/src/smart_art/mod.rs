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

use std::borrow::Cow;

use wordcraft_doc::THEME_COLORS;
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
/// Largest item text size, points: Word's Heading 1 (16 pt), so a graphic's items never read
/// louder than a document heading and their sub-items land near body text (see [`CHILD`]).
const MAX_SIZE: f32 = 16.0;
/// The item text size is at most the graphic's short side over this (a small graphic gets
/// smaller text, so it isn't crowded: 252 pt tall → the 16 pt cap, 150 pt → 10.5 pt).
const SIZE_PER_SIDE: f32 = 14.0;
/// Smallest text size a graphic is given, points.
const MIN_SIZE: f32 = 5.0;
/// A sub-item's text size against its item's (at most), so it reads clearly as the smaller line.
const CHILD: f32 = 0.65;
/// How far a sub-item's size may shrink (against its item's) so its lines don't break…
const CHILD_LEAST: f32 = 0.55;
/// …but never below this, points (unless [`CHILD`] of the item size already is).
const CHILD_FLOOR: f32 = 8.0;
/// How far the item size may shrink (against the largest that fits) so its lines don't break.
const ONE_LINE: f32 = 0.8;
/// Model ids of the drawing's shapes start here (the data model's points are below).
const SHAPE_IDS: usize = 0x10000;

/// The layout's identifier and its gallery category.
fn layout_id(l: SmartArtLayout) -> (Cow<'static, str>, &'static str) {
    let office = |name: &str| Cow::Owned(format!("urn:microsoft.com/office/officeart/2005/8/layout/{name}"));
    match l {
        SmartArtLayout::BasicList => (office("default"), "list"),
        SmartArtLayout::Process => (office("process1"), "process"),
        SmartArtLayout::Cycle => (office("cycle2"), "cycle"),
        SmartArtLayout::Hierarchy => (office("hierarchy1"), "hierarchy"),
        SmartArtLayout::Pyramid => (office("pyramid1"), "pyramid"),
        SmartArtLayout::Radial => (office("radial1"), "cycle"),
        SmartArtLayout::Matrix => (office("matrix3"), "matrix"),
        SmartArtLayout::Venn => (office("venn1"), "relationship"),
        // Layouts with no built-in counterpart we know of: identifiers of our own, so a reader
        // that doesn't know them uses the layout definition the file carries (see `layout_xml`).
        _ => {
            let cat = match l.category() {
                "Process" | "Timeline" => "process",
                "Cycle" => "cycle",
                "Hierarchy" => "hierarchy",
                "Relationship" => "relationship",
                "Matrix" => "matrix",
                "Pyramid" => "pyramid",
                "Picture" => "picture",
                _ => "list",
            };
            (Cow::Owned(format!("urn:lilyoffice.com/smartart/2026/layout/{}", l.id())), cat)
        }
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

mod layouts;
mod shapes;

use layouts::Tree;
use shapes::{Ink, Paint, Sh};

/// The shapes of `spec` laid out in a `w` × `h` points box, back to front.
fn layout(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let ok = |v: f32| if v.is_finite() { v.clamp(0.0, 20_000.0) } else { 0.0 };
    let (w, h) = (ok(w), ok(h));
    if spec.items.is_empty() || w < 1.0 || h < 1.0 {
        return Vec::new();
    }
    let mut shapes = layouts::draw(spec, w, h);
    for s in shapes.iter_mut().filter(|s| !s.text.is_empty()) {
        s.tb = padded(s.tb, s.fill != Paint::None);
    }
    shapes
}

/// A text box `tb` with room left inside it: inside a filled shape a margin on every side
/// (6% of the box, 2–8 pt across and 1–6 pt down, at most 15% of it), so text never touches
/// the shape's edge; text with nothing behind it keeps a little room either side only.
fn padded(tb: [f32; 4], filled: bool) -> [f32; 4] {
    let [x, y, w, h] = tb;
    let (w, h) = (w.max(0.0), h.max(0.0));
    let (px, py) =
        if filled { ((w * 0.06).clamp(2.0, 8.0).min(w * 0.15), (h * 0.06).clamp(1.0, 6.0).min(h * 0.15)) } else { ((w * 0.02).min(3.0), 0.0) };
    [x + px, y + py, w - 2.0 * px, h - 2.0 * py]
}

// ---- colours that read ----

/// Least WCAG contrast of a graphic's text against what is behind it (normal-size text, AA).
const TEXT_CONTRAST: f64 = 4.5;
/// Least WCAG contrast of an accent line against what is behind it (graphical objects, AA).
const LINE_CONTRAST: f64 = 3.0;

/// Theme colour `i` (0 dk1, 1 lt1, 2 dk2, 3 lt2, 4..9 accent1..6) of `theme`, else of the default theme.
fn theme_slot(theme: &[Rgb], i: usize) -> Rgb {
    theme.get(i).or_else(|| THEME_COLORS.get(i)).copied().unwrap_or(Rgb::BLACK)
}

fn accent_rgb(theme: &[Rgb], n: usize) -> Rgb {
    theme_slot(theme, 3 + n.clamp(1, 6))
}

/// `p` in the colours of `theme`, as the drawing reader resolves it; `None` for no fill.
fn paint_rgb(p: Paint, theme: &[Rgb]) -> Option<Rgb> {
    Some(match p {
        Paint::None => return None,
        Paint::Light => theme_slot(theme, 1),
        Paint::Accent(n, None) => accent_rgb(theme, n),
        Paint::Accent(n, Some(t)) => accent_rgb(theme, n).modified("tint", f64::from(t.min(100_000)) / 100_000.0),
        Paint::Shade(n, lum) => accent_rgb(theme, n).modified("lumMod", f64::from(lum.min(100_000)) / 100_000.0),
    })
}

fn ink_rgb(i: Ink, theme: &[Rgb]) -> Rgb {
    match i {
        Ink::Light => theme_slot(theme, 1),
        Ink::Dark => theme_slot(theme, 0),
        Ink::Accent(n) => accent_rgb(theme, n),
        Ink::Shade(n, lum) => paint_rgb(Paint::Shade(n, lum), theme).unwrap_or(Rgb::BLACK),
        Ink::Darker(lum) => theme_slot(theme, 0).modified("lumMod", f64::from(lum.min(100_000)) / 100_000.0),
    }
}

/// WCAG relative luminance of an sRGB colour.
fn luminance(c: Rgb) -> f64 {
    let ch = |v: u8| {
        let s = f64::from(v) / 255.0;
        if s <= 0.039_28 { s / 12.92 } else { ((s + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * ch(c.0) + 0.7152 * ch(c.1) + 0.0722 * ch(c.2)
}

/// WCAG contrast ratio of two colours (1–21).
fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// What shape `k` of `shapes` is seen against: its own fill, else the fill of the topmost shape
/// drawn before it whose box holds the middle of its text box (lines: of its box), else the page.
fn backdrop(shapes: &[Sh], k: usize, theme: &[Rgb]) -> Rgb {
    let Some(sh) = shapes.get(k) else { return Rgb::WHITE };
    if let Some(c) = paint_rgb(sh.fill, theme) {
        return c;
    }
    let r = if sh.text.is_empty() { sh.b } else { sh.tb };
    let (cx, cy) = (r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
    shapes
        .get(..k)
        .unwrap_or_default()
        .iter()
        .rev()
        .filter(|s| s.prst != "line" && s.b[0] <= cx && cx <= s.b[0] + s.b[2] && s.b[1] <= cy && cy <= s.b[1] + s.b[3])
        .find_map(|s| paint_rgb(s.fill, theme))
        .unwrap_or(Rgb::WHITE)
}

/// Accent `n` darkened just enough (in steps of 5% luminance) to reach `least` contrast against
/// the light `bg`; `None` when `bg` is dark or no step gets there.
fn darker(n: usize, bg: Rgb, theme: &[Rgb], least: f64) -> Option<u32> {
    if contrast(bg, Rgb::BLACK) < contrast(bg, Rgb::WHITE) {
        return None;
    }
    (2..=18u32).rev().map(|s| s * 5_000).find(|&lum| contrast(ink_rgb(Ink::Shade(n, lum), theme), bg) >= least)
}

/// The theme's dark text or light colour, whichever stands out more from `bg`; the dark one
/// darkened (in steps of 10%) when that is still short of [`TEXT_CONTRAST`] on a mid-tone `bg`.
fn plain_ink(bg: Rgb, theme: &[Rgb]) -> Ink {
    let dark = contrast(ink_rgb(Ink::Dark, theme), bg);
    if dark < contrast(ink_rgb(Ink::Light, theme), bg) {
        return Ink::Light;
    }
    if dark >= TEXT_CONTRAST {
        return Ink::Dark;
    }
    (0..=9u32).rev().map(|s| s * 10_000).find(|&lum| contrast(ink_rgb(Ink::Darker(lum), theme), bg) >= TEXT_CONTRAST).map_or(Ink::Dark, Ink::Darker)
}

/// Text and accent lines of laid-out `shapes` in colours that read in `theme`: the layouts name
/// theme colours (a light theme's accent can be pale yellow), so the real colours decide. Plain
/// text takes the theme's dark or light colour, whichever contrasts more with what is behind it;
/// accent text and lines keep their accent while it contrasts enough ([`TEXT_CONTRAST`],
/// [`LINE_CONTRAST`]), else a darker shade of it, else (text) the plain colour.
fn fit_colours(shapes: &mut [Sh], theme: &[Rgb]) {
    for k in 0..shapes.len() {
        let bg = backdrop(shapes, k, theme);
        let Some(sh) = shapes.get_mut(k) else { continue };
        if !sh.text.is_empty() {
            sh.ink = match sh.ink {
                Ink::Light | Ink::Dark | Ink::Darker(_) => plain_ink(bg, theme),
                Ink::Accent(n) | Ink::Shade(n, _) if contrast(accent_rgb(theme, n), bg) >= TEXT_CONTRAST => Ink::Accent(n),
                Ink::Accent(n) | Ink::Shade(n, _) => {
                    darker(n, bg, theme, TEXT_CONTRAST).map_or_else(|| plain_ink(bg, theme), |lum| Ink::Shade(n, lum))
                }
            };
        }
        if sh.fill == Paint::None
            && sh.line_w > 0.0
            && let Paint::Accent(n, _) = sh.line
            && paint_rgb(sh.line, theme).is_some_and(|c| contrast(c, bg) < LINE_CONTRAST)
        {
            sh.line = if contrast(accent_rgb(theme, n), bg) >= LINE_CONTRAST {
                Paint::Accent(n, None)
            } else {
                darker(n, bg, theme, LINE_CONTRAST).map_or(sh.line, |lum| Paint::Shade(n, lum))
            };
        }
    }
}

// ---- text ----

/// The text sizes of a graphic, points: every item line in one size and every sub-item line in
/// another (like Word, which keeps a graphic's text even), each times its shape's `scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sizes {
    item: f32,
    child: f32,
}

/// `v` to the nearest half point.
fn half(v: f32) -> f32 {
    (v * 2.0).round() / 2.0
}

/// Whether `paras` fit a `w` × `h` text box with item lines at `size` and sub-item lines at
/// `child` points. Lines wrap only at spaces (the drawing reader's [`wrap`]), so a word (a Korean
/// eojeol) wider than the box never fits: the text shrinks rather than break inside a word.
fn fits(paras: &[(String, bool)], size: f32, child: f32, w: f32, h: f32) -> bool {
    if !(size.is_finite() && child.is_finite() && w.is_finite() && h.is_finite()) {
        return false;
    }
    let mut total = 0.0;
    for (t, is_child) in paras {
        let s = if *is_child { child } else { size };
        if t.split_whitespace().any(|word| text_width(word, s) > w) {
            return false;
        }
        let lines = wrap(t, s, w, 10_000).len().max(1);
        total += lines as f32 * s * LINE;
    }
    total <= h
}

/// The shape's text scale (bounded).
fn scale_of(s: &Sh) -> f32 {
    if s.scale.is_finite() { s.scale.clamp(0.1, 10.0) } else { 1.0 }
}

/// Whether every shape's text fits at `sz`.
fn all_fit(shapes: &[Sh], sz: Sizes) -> bool {
    shapes.iter().filter(|s| !s.text.is_empty()).all(|s| {
        let k = scale_of(s);
        fits(&s.text, sz.item * k, sz.child * k, s.tb[2], s.tb[3])
    })
}

/// How many item lines (`child` false) or sub-item lines break in two or more at `sz`.
fn broken(shapes: &[Sh], sz: Sizes, child: bool) -> usize {
    shapes
        .iter()
        .map(|s| {
            let k = scale_of(s);
            let size = if child { sz.child } else { sz.item } * k;
            s.text.iter().filter(|(t, c)| *c == child && wrap(t, size, s.tb[2], 10_000).len() > 1).count()
        })
        .sum()
}

/// Of the half-point sizes from `hi` down to `lo` that keep everything fitting (`at` turns a
/// size into both sizes), the one that breaks the fewest lines of the kind `child`, the
/// largest of those.
fn least_broken(shapes: &[Sh], hi: f32, lo: f32, child: bool, at: impl Fn(f32) -> Sizes) -> f32 {
    let mut best = (usize::MAX, hi);
    let mut s = hi;
    while s >= lo - 0.01 {
        let sz = at(s);
        if all_fit(shapes, sz) {
            let n = broken(shapes, sz, child);
            if n < best.0 {
                best = (n, s);
            }
            if n == 0 {
                break;
            }
        }
        s -= 0.5;
    }
    best.1
}

/// The text sizes of a graphic laid out in a `w` × `h` box:
/// 1. the item size: the largest half point up to [`MAX_SIZE`] (and the short side over
///    [`SIZE_PER_SIDE`]) at which every shape's text fits with sub-items at [`CHILD`] of it;
/// 2. shrunk by up to [`ONE_LINE`] if that keeps item lines from breaking (fewest broken);
/// 3. the sub-item size: [`CHILD`] of the item size, shrunk down to [`CHILD_LEAST`] of it (not
///    below [`CHILD_FLOOR`]) if that keeps sub-item lines from breaking (fewest broken).
fn text_sizes(shapes: &[Sh], w: f32, h: f32) -> Sizes {
    let with_child = |s: f32| Sizes { item: s, child: half(s * CHILD).max(MIN_SIZE) };
    let short = w.min(h);
    let cap = if short.is_finite() { half(short / SIZE_PER_SIDE).clamp(MIN_SIZE, MAX_SIZE) } else { MAX_SIZE };
    let mut size = cap;
    while size > MIN_SIZE && !all_fit(shapes, with_child(size)) {
        size -= 0.5;
    }
    let item = least_broken(shapes, size, half(size * ONE_LINE).max(MIN_SIZE), false, with_child);
    let top = with_child(item).child;
    let floor = half(item * CHILD_LEAST).max(CHILD_FLOOR).min(top);
    let child = least_broken(shapes, top, floor, true, |c| Sizes { item, child: c });
    Sizes { item, child }
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
        Paint::Shade(n, lum) => {
            w.open("a:solidFill", &[]);
            scheme_clr(w, &format!("accent{}", n.clamp(1, 6)), Some(lum));
            w.close("a:solidFill");
        }
    }
}

/// `a:schemeClr` of theme colour `val`, with its luminance modulated to `lum` when set.
fn scheme_clr(w: &mut W, val: &str, lum: Option<u32>) {
    match lum {
        None => w.empty("a:schemeClr", &[("val", val)]),
        Some(l) => {
            w.open("a:schemeClr", &[("val", val)]);
            w.empty("a:lumMod", &[("val", &l.to_string())]);
            w.close("a:schemeClr");
        }
    }
}

/// The `dsp:drawing` of `spec` laid out `w` × `h` points in the colours of `theme` (text and
/// lines are coloured to read on what is behind them in that theme): its shapes, back to front.
pub fn drawing_xml(spec: &SmartArtSpec, theme: &[Rgb], w: f32, h: f32) -> String {
    let mut shapes = layout(spec, w, h);
    fit_colours(&mut shapes, theme);
    let size = text_sizes(&shapes, w, h);
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

fn shape_xml(x: &mut W, sh: &Sh, id: &str, sizes: Sizes) {
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
    let (text_clr, text_lum) = sh.ink.scheme();
    let text_clr = text_clr.as_str();
    let k = scale_of(sh);
    let (size, child_size) = (sizes.item * k, sizes.child * k);
    x.open("dsp:style", &[]);
    for (name, idx) in [("a:lnRef", "2"), ("a:fillRef", "1"), ("a:effectRef", "0")] {
        x.open(name, &[("idx", idx)]);
        x.empty("a:scrgbClr", &[("r", "0"), ("g", "0"), ("b", "0")]);
        x.close(name);
    }
    x.open("a:fontRef", &[("idx", "minor")]);
    scheme_clr(x, text_clr, text_lum);
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
            let s = if *child { child_size } else { size };
            x.open("a:p", &[]);
            let algn = sh.algn.unwrap_or(if *child { "l" } else { "ctr" });
            // eaLnBrk 0: an East Asian word (a Korean eojeol) is not broken in half (ECMA-376 §21.1.2.2.7).
            x.open("a:pPr", &[("lvl", if *child { "1" } else { "0" }), ("algn", algn), ("eaLnBrk", "0")]);
            x.empty("a:buNone", &[]);
            x.close("a:pPr");
            if t.is_empty() {
                x.empty("a:endParaRPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
            } else {
                x.open("a:r", &[]);
                x.open("a:rPr", &[("lang", "en-US"), ("sz", &((s * 100.0).round() as i64).to_string())]);
                x.open("a:solidFill", &[]);
                scheme_clr(x, text_clr, text_lum);
                x.close("a:solidFill");
                x.close("a:rPr");
                x.leaf("a:t", &[], t);
                x.close("a:r");
            }
            x.close("a:p");
        }
        x.close("dsp:txBody");
        // The text box is upright on the page (it is read that way, `read::diagram`). A turned shape
        // (inverted pyramid layers, the down-pointing chevrons) turns its text with it unless the
        // text frame turns back: `dsp:txXfrm@rot` adds to the shape's `rot`, as PowerPoint writes it.
        let back = if sh.rot != 0.0 { format!("-{rot}") } else { String::new() };
        let tx_attrs: Vec<(&str, &str)> = if back.is_empty() { Vec::new() } else { vec![("rot", &back)] };
        x.open("dsp:txXfrm", &tx_attrs);
        x.empty("a:off", &[("x", &emu(sh.tb[0])), ("y", &emu(sh.tb[1]))]);
        x.empty("a:ext", &[("cx", &emu(sh.tb[2].max(0.0))), ("cy", &emu(sh.tb[3].max(0.0)))]);
        x.close("dsp:txXfrm");
    }
    x.close("dsp:sp");
}

/// What `spec` draws as, `w` × `h` points, in the colours of `theme`: its drawing part, read the
/// way SmartArt drawings from files are.
pub fn smart_art_items(spec: &SmartArtSpec, theme: &[Rgb], w: f32, h: f32) -> Vec<GraphicItem> {
    match xml::parse(drawing_xml(spec, theme, w, h).as_bytes()) {
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
            out.push(match sh.pick {
                Some(b) => SmartArtNode { item, b, rot: 0.0, prst: "rect" },
                None => SmartArtNode { item, b: sh.b, rot: sh.rot, prst: sh.prst },
            });
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
        &[("loTypeId", &lo), ("loCatId", lo_cat), ("qsTypeId", QUICK_STYLE_ID), ("qsCatId", "simple"), ("csTypeId", &cs), ("csCatId", &cs_cat)],
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
    w.open("dgm:layoutDef", &[("xmlns:dgm", NS_DGM), ("xmlns:a", NS_A), ("xmlns:r", NS_R), ("uniqueId", &id)]);
    w.empty("dgm:title", &[("val", l.label())]);
    w.empty("dgm:desc", &[("val", l.description())]);
    w.open("dgm:catLst", &[]);
    w.empty("dgm:cat", &[("type", cat), ("pri", "1000")]);
    w.close("dgm:catLst");
    let def = layouts::def(l);
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
        | SmartArtLayout::CardTall
        | SmartArtLayout::TimelineStems
        | SmartArtLayout::TimelineCircles
        | SmartArtLayout::TimelineZigzag
        | SmartArtLayout::TimelineLine
        | SmartArtLayout::CardEmphasis
        | SmartArtLayout::CardSquares => ("lin", &[("linDir", "fromL"), ("nodeVertAlign", "t")]),
        SmartArtLayout::TimelineVertical | SmartArtLayout::TeamCards | SmartArtLayout::CardRows | SmartArtLayout::CardMarkerRows => {
            ("lin", &[("linDir", "fromT")])
        }
        _ => (def.alg, def.params),
    };
    let shape = match l {
        SmartArtLayout::BasicList
        | SmartArtLayout::CardNumbered
        | SmartArtLayout::CardBigNumbers
        | SmartArtLayout::CardHeader
        | SmartArtLayout::CardRows
        | SmartArtLayout::TimelineVertical
        | SmartArtLayout::CardEmphasis
        | SmartArtLayout::CardMarkerRows
        | SmartArtLayout::CardSquares => "rect",
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
        | SmartArtLayout::TeamCircles
        | SmartArtLayout::TimelineStems
        | SmartArtLayout::TimelineCircles
        | SmartArtLayout::TimelineZigzag
        | SmartArtLayout::TimelineLine => "ellipse",
        SmartArtLayout::Pyramid => "trapezoid",
        _ => def.shape,
    };
    let tree = match l {
        SmartArtLayout::Hierarchy => Tree::Hier,
        SmartArtLayout::Radial => Tree::Radial,
        _ => def.tree,
    };
    layout_node(&mut w, LayoutNode { name: "diagram", style: None, alg, params, shape: None, pres_of: &[] }, |w| match tree {
        Tree::Hier => {
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
        Tree::Radial => {
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
        Tree::Items => {
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

/// [`spec_of`] for a data part's bytes (another container — a presentation — holding the same parts).
pub fn spec_of_xml(data: &str) -> Option<SmartArtSpec> {
    spec_of(&xml::parse(data.as_bytes()).ok()?)
}

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
mod tests;
