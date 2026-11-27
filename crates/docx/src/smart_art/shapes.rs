//! The shapes a layout is made of and the helpers layouts share.

use wordcraft_doc::smart_art::{SmartArtColors, SmartArtSpec};

// ---- layout ----

/// What a shape is filled or outlined with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Paint {
    None,
    /// Theme accent `n` (1–6), lightened to `tint` (thousandths of a percent kept) when set.
    Accent(usize, Option<u32>),
    /// The theme's light background colour.
    Light,
    /// Theme accent `n` (1–6) darkened to `lum` (thousandths of a percent of its luminance kept),
    /// where the plain accent wouldn't stand out from what is behind it (see [`fit_colours`]).
    Shade(usize, u32),
}

/// The colour of a shape's text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Ink {
    /// The theme's light colour (on filled shapes).
    Light,
    /// The theme's dark text colour (on light fills or no fill).
    Dark,
    /// Theme accent `n` (1–6).
    Accent(usize),
    /// Theme accent `n` darkened to `lum` (see [`Paint::Shade`]).
    Shade(usize, u32),
    /// The theme's dark text colour darker still (`lum` of its luminance kept), on mid-tone
    /// fills where neither it nor the light colour reads well enough.
    Darker(u32),
}

impl Ink {
    /// The theme colour name and the luminance modulation (`a:lumMod`) to write, if any.
    pub(crate) fn scheme(self) -> (String, Option<u32>) {
        match self {
            Ink::Light => ("lt1".into(), None),
            Ink::Dark => ("tx1".into(), None),
            Ink::Accent(n) => (format!("accent{}", n.clamp(1, 6)), None),
            Ink::Shade(n, lum) => (format!("accent{}", n.clamp(1, 6)), Some(lum)),
            Ink::Darker(lum) => ("tx1".into(), Some(lum)),
        }
    }
}

/// One shape of a laid-out graphic, in points from the graphic's top-left corner.
#[derive(Clone, Debug)]
pub(crate) struct Sh {
    pub(crate) prst: &'static str,
    pub(crate) b: [f32; 4],
    /// Clockwise turn, degrees.
    pub(crate) rot: f32,
    pub(crate) flip_h: bool,
    /// The preset's adjust values (`adj`, `adj1`…) that differ from its defaults.
    pub(crate) adj: Vec<(&'static str, i64)>,
    pub(crate) fill: Paint,
    /// Fill opacity (thousandths of a percent) for readers that blend.
    pub(crate) alpha: Option<u32>,
    pub(crate) line: Paint,
    pub(crate) line_w: f32,
    /// Paragraphs: text and whether it's a sub-item (smaller, bulleted, left-aligned).
    pub(crate) text: Vec<(String, bool)>,
    /// Where the text goes.
    pub(crate) tb: [f32; 4],
    pub(crate) ink: Ink,
    /// Every paragraph's alignment (`l`, `ctr`, `r`); `None`: the item's line centred, sub-items left.
    pub(crate) algn: Option<&'static str>,
    /// Where the text sits in its box (`t`, `ctr`, `b`).
    pub(crate) anchor: &'static str,
    /// The text size against the graphic's (large numbers are more than 1).
    pub(crate) scale: f32,
    /// The outline item this shape stands for (a node or its text), for picking one item.
    pub(crate) item: Option<usize>,
    /// The box to pick the item by when not the shape's own (a wedge's or ring's box holds its neighbours too).
    pub(crate) pick: Option<[f32; 4]>,
}

impl Sh {
    /// A filled node shape with a thin light outline.
    pub(crate) fn node(prst: &'static str, b: [f32; 4], accent: usize) -> Sh {
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
            pick: None,
        }
    }

    /// A straight connector from (`x0`, `y0`) to (`x1`, `y1`).
    pub(crate) fn line(x0: f32, y0: f32, x1: f32, y1: f32, accent: usize) -> Sh {
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
            pick: None,
        }
    }

    /// The text of item `i` (and, with `with_children`, the items under it as bullets), and
    /// the item this shape stands for.
    pub(crate) fn with_item(mut self, spec: &SmartArtSpec, i: usize, with_children: bool) -> Sh {
        self.text = paras(spec, i, with_children);
        self.item = Some(i);
        self
    }

    pub(crate) fn of_item(mut self, i: usize) -> Sh {
        self.item = Some(i);
        self
    }

    pub(crate) fn text_box(mut self, tb: [f32; 4]) -> Sh {
        self.tb = tb;
        self
    }
}

/// `b` shrunk by `f` of its width and height on every side.
pub(crate) fn inset(b: [f32; 4], f: f32) -> [f32; 4] {
    [b[0] + b[2] * f, b[1] + b[3] * f, b[2] * (1.0 - 2.0 * f), b[3] * (1.0 - 2.0 * f)]
}

/// The inscribed square's share of a circle's box to leave on each side.
pub(crate) const ROUND_INSET: f32 = 0.15;

/// The text of item `i`: its own line, then the items under it as bulleted sub-items.
pub(crate) fn paras(spec: &SmartArtSpec, i: usize, with_children: bool) -> Vec<(String, bool)> {
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
pub(crate) fn tops(spec: &SmartArtSpec) -> Vec<usize> {
    spec.items.iter().enumerate().filter(|(_, it)| it.level == 0).map(|(i, _)| i).collect()
}

/// The angle (degrees, clockwise from three o'clock) of place `k` of `n` round a circle, the
/// first at twelve o'clock.
pub(crate) fn angle(k: usize, n: usize) -> f32 {
    -90.0 + 360.0 * k as f32 / n.max(1) as f32
}

/// The point at `r` from (`cx`, `cy`) at `deg`.
pub(crate) fn polar(cx: f32, cy: f32, r: f32, deg: f32) -> (f32, f32) {
    let (s, c) = deg.to_radians().sin_cos();
    (cx + r * c, cy + r * s)
}

// ---- timelines, team and text cards ----

/// The paint of top item `k` of `n`: its accent, graded from full strength to light across the
/// items when the graphic is in one accent.
pub(crate) fn graded(spec: &SmartArtSpec, k: usize, n: usize) -> Paint {
    let a = spec.colors.accent(k);
    if spec.colors == SmartArtColors::Colorful || n <= 1 {
        return Paint::Accent(a, None);
    }
    let t = 100_000 - 50_000 * k.min(n - 1) as u64 / (n - 1) as u64;
    Paint::Accent(a, u32::try_from(t).ok().filter(|t| *t < 100_000))
}

/// A light tint of top item `k`'s accent (`most` to `least` kept), lighter for later items in a
/// one-accent graphic.
pub(crate) fn graded_light(spec: &SmartArtSpec, k: usize, n: usize, most: u32, least: u32) -> Paint {
    let a = spec.colors.accent(k);
    if spec.colors == SmartArtColors::Colorful || n <= 1 {
        return Paint::Accent(a, Some(most));
    }
    let span = u64::from(most.saturating_sub(least));
    let t = u64::from(most) - span * k.min(n - 1) as u64 / (n - 1) as u64;
    Paint::Accent(a, Some(u32::try_from(t).unwrap_or(most)))
}

/// Text that reads on `p`: dark on any lightened accent and on no fill, light on full accents.
pub(crate) fn ink_on(p: Paint) -> Ink {
    match p {
        Paint::Accent(_, Some(t)) if t < 90_000 => Ink::Dark,
        Paint::Accent(..) | Paint::Shade(..) => Ink::Light,
        Paint::Light | Paint::None => Ink::Dark,
    }
}

/// A shape filled with `fill`, no outline, text in a colour that reads on it.
pub(crate) fn plain(prst: &'static str, b: [f32; 4], fill: Paint) -> Sh {
    let mut s = Sh::node(prst, b, 1);
    s.fill = fill;
    s.line = Paint::None;
    s.ink = ink_on(fill);
    s
}

/// Item `i`'s own line (`own`) and the lines of the items under it (`kids`), without bullets.
pub(crate) fn lines_of(spec: &SmartArtSpec, i: usize, own: bool, kids: bool) -> Vec<(String, bool)> {
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
pub(crate) fn label(spec: &SmartArtSpec, i: usize, b: [f32; 4], algn: &'static str, anchor: &'static str) -> Sh {
    let mut s = plain("rect", b, Paint::None);
    s.text = lines_of(spec, i, true, true);
    s.item = Some(i);
    s.tb = b;
    s.algn = Some(algn);
    s.anchor = anchor;
    s
}

/// `s` with one line of `text` in `algn`, in `ink`, `scale` times the graphic's text size.
pub(crate) fn with_text(mut s: Sh, text: String, algn: &'static str, ink: Ink, scale: f32) -> Sh {
    s.text = vec![(text, false)];
    s.algn = Some(algn);
    s.ink = ink;
    s.scale = scale;
    s
}

/// A person placeholder in the square `b`: a circle in `bg` with a head and shoulders in `fg`.
pub(crate) fn avatar(b: [f32; 4], bg: Paint, fg: Paint, item: usize) -> Vec<Sh> {
    let [x, y, s, _] = b;
    let cx = x + s / 2.0;
    let circle = plain("ellipse", [x, y, s, s], bg).of_item(item);
    let head = plain("ellipse", [cx - 0.17 * s, y + 0.18 * s, 0.34 * s, 0.34 * s], fg);
    // The upper half of an ellipse whose box ends at the circle's bottom (no box leaves the circle's).
    let mut shoulders = plain("pie", [cx - 0.3 * s, y + 0.58 * s, 0.6 * s, 0.42 * s], fg);
    shoulders.adj = vec![("adj1", 10_800_000), ("adj2", 0)];
    vec![circle, head, shoulders]
}

// ---- shared by the later layouts ----

impl Sh {
    /// The shape picked by `b` rather than by its own box (see [`Sh::pick`]).
    pub(crate) fn picked_by(mut self, b: [f32; 4]) -> Sh {
        self.pick = Some(b);
        self
    }

    /// The shape turned `deg` degrees clockwise about its box's centre.
    pub(crate) fn turned(mut self, deg: f32) -> Sh {
        self.rot = if deg.is_finite() { deg.rem_euclid(360.0) } else { 0.0 };
        self
    }
}

/// Item `i`'s own text.
pub(crate) fn own(spec: &SmartArtSpec, i: usize) -> String {
    spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default()
}

/// A shape filled with `fill` holding item `i`'s line and the lines under it, centred, in a
/// text box a little inside its box.
pub(crate) fn card(prst: &'static str, b: [f32; 4], fill: Paint, spec: &SmartArtSpec, i: usize) -> Sh {
    let mut s = plain(prst, b, fill);
    s.text = lines_of(spec, i, true, true);
    s.item = Some(i);
    s.tb = inset(b, 0.08);
    s.algn = Some("ctr");
    s
}

/// A shape filled with `fill` holding item `i`'s own line only.
pub(crate) fn tag(prst: &'static str, b: [f32; 4], fill: Paint, spec: &SmartArtSpec, i: usize) -> Sh {
    let mut s = card(prst, b, fill, spec, i);
    s.text = lines_of(spec, i, true, false);
    s
}

/// A picture placeholder in `b` (`prst`: `rect`, `roundRect` or `ellipse`): a frame in a light
/// tint of `accent` with a sun and two hills in stronger tints. No image: the graphic's model has
/// none yet, so it shows where one goes.
pub(crate) fn picture(b: [f32; 4], prst: &'static str, accent: usize, item: usize) -> Vec<Sh> {
    let [x, y, w, h] = b;
    let mut frame = plain(prst, b, Paint::Accent(accent, Some(22_000))).of_item(item);
    if prst == "roundRect" {
        frame.adj = vec![("adj", 8_000)];
    }
    // The glyph in a square in the middle (inside the inscribed square of a round frame).
    let s = w.min(h) * if prst == "ellipse" { 0.56 } else { 0.62 };
    let (gx, gy) = (x + (w - s) / 2.0, y + (h - s) / 2.0);
    let sun = plain("ellipse", [gx + s * 0.64, gy + s * 0.08, s * 0.24, s * 0.24], Paint::Accent(accent, Some(55_000)));
    let big = plain("triangle", [gx, gy + s * 0.3, s * 0.74, s * 0.62], Paint::Accent(accent, Some(75_000)));
    let small = plain("triangle", [gx + s * 0.42, gy + s * 0.5, s * 0.58, s * 0.42], Paint::Accent(accent, Some(50_000)));
    vec![frame, sun, big, small]
}

/// An outline as a tree: each item's children, the top items, how many leaves each item spans
/// and how many levels there are.
pub(crate) struct Tree {
    pub(crate) kids: Vec<Vec<usize>>,
    pub(crate) roots: Vec<usize>,
    pub(crate) leaves: Vec<usize>,
    pub(crate) depth: usize,
}

impl Tree {
    pub(crate) fn of(spec: &SmartArtSpec) -> Tree {
        let n = spec.items.len();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut roots = Vec::new();
        for (i, p) in spec.parents().iter().enumerate() {
            match p.and_then(|p| kids.get_mut(p)) {
                Some(k) => k.push(i),
                None => roots.push(i),
            }
        }
        // Children come after their parent, so from the end.
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
        Tree { kids, roots, leaves, depth }
    }

    pub(crate) fn kids(&self, i: usize) -> &[usize] {
        self.kids.get(i).map_or(&[], Vec::as_slice)
    }

    pub(crate) fn leaves(&self, i: usize) -> usize {
        self.leaves.get(i).copied().unwrap_or(1)
    }

    /// All the leaves.
    pub(crate) fn total(&self) -> usize {
        self.roots.iter().map(|&r| self.leaves(r)).sum::<usize>().max(1)
    }

    /// Where each item's run of leaves starts, in leaves from the first.
    pub(crate) fn starts(&self) -> Vec<f32> {
        let mut start = vec![0.0f32; self.leaves.len()];
        let mut at = 0.0;
        for &r in &self.roots {
            if let Some(s) = start.get_mut(r) {
                *s = at;
            }
            at += self.leaves(r) as f32;
        }
        for i in 0..self.leaves.len() {
            let mut at = start.get(i).copied().unwrap_or(0.0);
            for &k in self.kids(i) {
                if let Some(s) = start.get_mut(k) {
                    *s = at;
                }
                at += self.leaves(k) as f32;
            }
        }
        start
    }
}

/// Elbow lines from a parent's bottom middle (`px`, `py`) down to children's tops at `xs`, `cy`.
pub(crate) fn elbows_down(px: f32, py: f32, xs: &[f32], cy: f32, accent: usize) -> Vec<Sh> {
    let mid = (py + cy) / 2.0;
    let mut out = vec![Sh::line(px, py, px, mid, accent)];
    let (lo, hi) = xs.iter().fold((px, px), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    if hi - lo > 0.5 {
        out.push(Sh::line(lo, mid, hi, mid, accent));
    }
    out.extend(xs.iter().map(|&x| Sh::line(x, mid, x, cy, accent)));
    out
}

/// Top item `k`'s accent at `keep` (thousandths of a percent) of its strength.
pub(crate) fn tint(spec: &SmartArtSpec, k: usize, keep: u32) -> Paint {
    Paint::Accent(spec.colors.accent(k), Some(keep).filter(|t| *t < 100_000))
}

/// Place `k` of `n` in a grid of `cols` columns filling `[x0, y0, w, h]`, the last row centred:
/// the cell's box.
pub(crate) fn grid_cell(k: usize, n: usize, cols: usize, b: [f32; 4]) -> [f32; 4] {
    let cols = cols.max(1);
    let rows = n.max(1).div_ceil(cols);
    let (cw, rh) = (b[2] / cols as f32, b[3] / rows as f32);
    let (r, c) = (k / cols, k % cols);
    let in_row = if r + 1 == rows { n.max(1) - r * cols } else { cols };
    let x0 = b[0] + (cols - in_row.min(cols)) as f32 * cw / 2.0;
    [x0 + c as f32 * cw, b[1] + r as f32 * rh, cw, rh]
}

/// The number of columns (1 to `n`) that makes cells of aspect `aspect` (width over height) the
/// largest in a `w` × `h` box.
pub(crate) fn best_cols(n: usize, w: f32, h: f32, aspect: f32) -> usize {
    let n = n.max(1);
    let size = |c: usize| (w / c as f32).min(h / n.div_ceil(c) as f32 * aspect);
    (1..=n).max_by(|a, b| size(*a).total_cmp(&size(*b))).unwrap_or(1)
}
