use super::model::Pos;
use super::{CHAR_W, GAP, GraphicItem, LEGEND_SHARE, LINE, MAX_LEGEND_ROWS, PAD, PathSeg, Rgb, ShapeKind, TEXT, TextAlign};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Rect {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
}

impl Rect {
    pub(super) fn right(self) -> f32 {
        self.x + self.w
    }
    pub(super) fn bottom(self) -> f32 {
        self.y + self.h
    }
    pub(super) fn arr(self) -> [f32; 4] {
        [self.x, self.y, self.w, self.h]
    }
}

/// Estimated text width, points: text is placed before it's shaped (chart items are made
/// without fonts), at [`CHAR_W`] a character, and a full em for a wide East Asian character
/// (Hangul, kana, CJK ideographs, full-width forms), which fonts set square.
pub(in super::super) fn text_w(s: &str, size: f32) -> f32 {
    s.chars().map(char_em).sum::<f32>() * size
}

/// Estimated advance of `c`, em.
fn char_em(c: char) -> f32 {
    if is_wide(c) { 1.0 } else { CHAR_W }
}

/// East Asian wide and full-width characters (Unicode UAX #11 W/F, the common blocks).
fn is_wide(c: char) -> bool {
    matches!(u32::from(c),
        0x1100..=0x115F // Hangul Jamo (leading consonants)
        | 0x2E80..=0x303E // CJK radicals, Kangxi, ideographic description, CJK symbols and punctuation
        | 0x3041..=0x33FF // kana, Bopomofo, Hangul compatibility Jamo, Kanbun, CJK strokes, enclosed and compatibility
        | 0x3400..=0x4DBF // CJK Extension A
        | 0x4E00..=0x9FFF // CJK Unified Ideographs
        | 0xA960..=0xA97F // Hangul Jamo Extended-A
        | 0xAC00..=0xD7A3 // Hangul syllables
        | 0xF900..=0xFAFF // CJK compatibility ideographs
        | 0xFE30..=0xFE4F // CJK compatibility forms
        | 0xFF00..=0xFF60 // full-width forms
        | 0xFFE0..=0xFFE6
        | 0x20000..=0x3FFFD // CJK Extensions B and later
    )
}

/// `s`, cut short with an ellipsis to fit `width` points by the [`text_w`] estimate.
pub(super) fn clip_text(s: &str, width: f32, size: f32) -> String {
    if text_w(s, size) <= width {
        return s.to_string();
    }
    let room = width - size * CHAR_W; // the ellipsis
    let mut out = String::new();
    let mut used = 0.0;
    for c in s.chars() {
        used += char_em(c) * size;
        if used > room {
            break;
        }
        out.push(c);
    }
    out.push('…');
    out
}

pub(super) fn widest(items: &[String], size: f32) -> f32 {
    items.iter().map(|s| text_w(s, size)).fold(0.0, f32::max)
}

/// Text in the default chart text colour ([`TEXT`]).
pub(super) fn text(out: &mut Vec<GraphicItem>, rect: Rect, s: &str, size: f32, align: TextAlign, bold: bool) {
    text_in(out, rect, s, size, align, bold, TEXT);
}

/// Text in `color`.
pub(super) fn text_in(out: &mut Vec<GraphicItem>, rect: Rect, s: &str, size: f32, align: TextAlign, bold: bool, color: Rgb) {
    out.push(GraphicItem::Text { rect: rect.arr(), text: s.to_string(), size, color, bold, align, font: None, rotation: 0.0 });
}

/// Text in `color` centred on `cx` with its top at `y`.
pub(super) fn text_at(out: &mut Vec<GraphicItem>, cx: f32, y: f32, s: &str, size: f32, color: Rgb) {
    let w = text_w(s, size) + 4.0;
    text_in(out, Rect { x: cx - w / 2.0, y, w, h: size * LINE }, s, size, TextAlign::Center, false, color);
}

/// A closed circle around (`cx`, `cy`) of radius `r`: four cubic quarter arcs.
pub(super) fn circle_segs(cx: f32, cy: f32, r: f32) -> Vec<PathSeg> {
    // The control distance of a quarter arc (4/3·tan(π/8)).
    let k = 0.552_284_8 * r;
    vec![
        PathSeg::Move(cx + r, cy),
        PathSeg::Cubic(cx + r, cy + k, cx + k, cy + r, cx, cy + r),
        PathSeg::Cubic(cx - k, cy + r, cx - r, cy + k, cx - r, cy),
        PathSeg::Cubic(cx - r, cy - k, cx - k, cy - r, cx, cy - r),
        PathSeg::Cubic(cx + k, cy - r, cx + r, cy - k, cx + r, cy),
        PathSeg::Close,
    ]
}

pub(super) fn fill(out: &mut Vec<GraphicItem>, segs: Vec<PathSeg>, color: Rgb) {
    if !segs.is_empty() {
        out.push(GraphicItem::Path { segs, fill: Some(color), stroke: None, stroke_width: 0.0, gradient: None });
    }
}

pub(super) fn stroke(out: &mut Vec<GraphicItem>, segs: Vec<PathSeg>, color: Rgb, width: f32) {
    if !segs.is_empty() {
        out.push(GraphicItem::Path { segs, fill: None, stroke: Some(color), stroke_width: width, gradient: None });
    }
}

/// A legend key at (`x`, `y`), [`SWATCH`] wide: a square, or a circle of that diameter (`round`).
fn swatch(out: &mut Vec<GraphicItem>, x: f32, y: f32, color: Rgb, round: bool) {
    let kind = if round { ShapeKind::Ellipse } else { ShapeKind::Rectangle };
    out.push(GraphicItem::Shape { rect: [x, y, SWATCH, SWATCH], kind, fill: Some(color), stroke: None, stroke_width: 0.0 });
}

pub(super) fn seg(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<PathSeg> {
    vec![PathSeg::Move(x0, y0), PathSeg::Line(x1, y1)]
}

pub(super) fn rect_segs(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<PathSeg> {
    vec![PathSeg::Move(x0, y0), PathSeg::Line(x1, y0), PathSeg::Line(x1, y1), PathSeg::Line(x0, y1), PathSeg::Close]
}

/// Polyline pieces: a gap (`None`) starts a new sub-path.
pub(super) fn runs(pts: impl Iterator<Item = Option<(f32, f32)>>) -> Vec<PathSeg> {
    let mut segs = Vec::new();
    let mut open = false;
    for p in pts {
        match p {
            Some((x, y)) => {
                segs.push(if open { PathSeg::Line(x, y) } else { PathSeg::Move(x, y) });
                open = true;
            }
            None => open = false,
        }
    }
    segs
}

/// The side of a bar whose corners are rounded: its value end, away from the axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum End {
    Top,
    Bottom,
    Left,
    Right,
}

/// A rectangle with the two corners on side `end` rounded by `r` points, the others square. The
/// radius is held to half the rectangle's width across that side and to its length along it,
/// so a thin or zero-length bar stays a valid shape; quarter circles as cubic Béziers.
pub(super) fn round_end_segs(x0: f32, y0: f32, x1: f32, y1: f32, r: f32, end: End) -> Vec<PathSeg> {
    let (x0, x1) = (x0.min(x1), x0.max(x1));
    let (y0, y1) = (y0.min(y1), y0.max(y1));
    let (w, h) = (x1 - x0, y1 - y0);
    let r = match end {
        End::Top | End::Bottom => r.min(w / 2.0).min(h),
        End::Left | End::Right => r.min(h / 2.0).min(w),
    };
    if !(r.is_finite() && r > 0.0) {
        return rect_segs(x0, y0, x1, y1);
    }
    // Radii of the corners top-left, top-right, bottom-right, bottom-left.
    let [tl, tr, br, bl] = match end {
        End::Top => [r, r, 0.0, 0.0],
        End::Bottom => [0.0, 0.0, r, r],
        End::Left => [r, 0.0, 0.0, r],
        End::Right => [0.0, r, r, 0.0],
    };
    // Control point distance of a quarter circle drawn as one cubic.
    let k = |r: f32| r * 0.552_284_8;
    let mut segs = vec![PathSeg::Move(x0 + tl, y0), PathSeg::Line(x1 - tr, y0)];
    if tr > 0.0 {
        segs.push(PathSeg::Cubic(x1 - tr + k(tr), y0, x1, y0 + tr - k(tr), x1, y0 + tr));
    }
    segs.push(PathSeg::Line(x1, y1 - br));
    if br > 0.0 {
        segs.push(PathSeg::Cubic(x1, y1 - br + k(br), x1 - br + k(br), y1, x1 - br, y1));
    }
    segs.push(PathSeg::Line(x0 + bl, y1));
    if bl > 0.0 {
        segs.push(PathSeg::Cubic(x0 + bl - k(bl), y1, x0, y1 - bl + k(bl), x0, y1 - bl));
    }
    segs.push(PathSeg::Line(x0, y0 + tl));
    if tl > 0.0 {
        segs.push(PathSeg::Cubic(x0, y0 + tl - k(tl), x0 + tl - k(tl), y0, x0 + tl, y0));
    }
    segs.push(PathSeg::Close);
    segs
}

/// Smooth curves through the points: each run between gaps (`None`) as cubic Béziers of a
/// Catmull-Rom spline (the curve passes through every point, its tangent at a point parallel to
/// the line between the points either side; an end uses the end point itself).
pub(super) fn curves(pts: impl Iterator<Item = Option<(f32, f32)>>) -> Vec<PathSeg> {
    let mut segs = Vec::new();
    let mut run: Vec<(f32, f32)> = Vec::new();
    for p in pts.chain(std::iter::once(None)) {
        match p {
            Some(p) => run.push(p),
            None => {
                spline(&run, &mut segs);
                run.clear();
            }
        }
    }
    segs
}

fn spline(run: &[(f32, f32)], segs: &mut Vec<PathSeg>) {
    let Some(&(x, y)) = run.first() else { return };
    segs.push(PathSeg::Move(x, y));
    for i in 1..run.len() {
        let at = |k: usize| run.get(k).copied();
        let (Some(p1), Some(p2)) = (at(i - 1), at(i)) else { continue };
        let p0 = i.checked_sub(2).and_then(at).unwrap_or(p1);
        let p3 = at(i + 1).unwrap_or(p2);
        let c1 = (p1.0 + (p2.0 - p0.0) / 6.0, p1.1 + (p2.1 - p0.1) / 6.0);
        let c2 = (p2.0 - (p3.0 - p1.0) / 6.0, p2.1 - (p3.1 - p1.1) / 6.0);
        segs.push(PathSeg::Cubic(c1.0, c1.1, c2.0, c2.1, p2.0, p2.1));
    }
}

/// Most dashes in one dashed line; a longer line gets longer dashes and gaps.
const MAX_DASHES: f32 = 2000.0;

/// The line from (`x0`, `y0`) to (`x1`, `y1`) as dashes `on` points long with gaps `off` points
/// long between them, each dash its own sub-path.
pub(super) fn dashes(x0: f32, y0: f32, x1: f32, y1: f32, on: f32, off: f32) -> Vec<PathSeg> {
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = dx.hypot(dy);
    if !(len.is_finite() && len > 0.0 && on > 0.0 && off >= 0.0 && (on + off).is_finite()) {
        return seg(x0, y0, x1, y1);
    }
    let period = (on + off).max(len / MAX_DASHES);
    let on = on * period / (on + off);
    let n = (len / period).ceil() as usize;
    let mut segs = Vec::with_capacity(2 * n);
    for k in 0..n {
        let t0 = k as f32 * period;
        let t1 = (t0 + on).min(len);
        segs.push(PathSeg::Move(x0 + dx * t0 / len, y0 + dy * t0 / len));
        segs.push(PathSeg::Line(x0 + dx * t1 / len, y0 + dy * t1 / len));
    }
    segs
}

/// Words of `s` in lines of at most `width` points (a word wider than that keeps its own line).
pub(super) fn wrap(s: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let joined = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if cur.is_empty() || text_w(&joined, size) <= width {
            cur = joined;
        } else {
            lines.push(std::mem::take(&mut cur));
            cur = word.to_string();
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines.truncate(3);
    lines
}

pub(super) const SWATCH: f32 = 7.0;

/// Space between legend entries in a row, points.
const SEP: f32 = 10.0;

/// Legend in the given side of `area`; returns the area left for the plot.
pub(super) fn legend(out: &mut Vec<GraphicItem>, entries: &[(String, Rgb)], pos: Pos, area: Rect, fs: f32, round: bool, color: Rgb) -> Rect {
    if entries.is_empty() {
        return area;
    }
    // Entry texts are cut to a share of the chart, so a long name can't push the legend out.
    let entries: Vec<(String, Rgb)> = entries.iter().map(|(s, c)| (clip_text(s, area.w * LEGEND_SHARE, fs), *c)).collect();
    match pos {
        Pos::Right | Pos::Left => side_legend(out, &entries, pos == Pos::Left, area, fs, round, color),
        Pos::Top | Pos::Bottom => row_legend(out, &entries, pos == Pos::Top, area, fs, round, color),
    }
}

/// A legend in a column at the left or right of `area`, keeping only the entries that fit above
/// its bottom; returns the area left for the plot.
fn side_legend(out: &mut Vec<GraphicItem>, entries: &[(String, Rgb)], left: bool, area: Rect, fs: f32, round: bool, color: Rgb) -> Rect {
    let lh = fs * LINE;
    let entries = entries.get(..((area.h / lh).floor().max(0.0) as usize).min(entries.len())).unwrap_or(&[]);
    if entries.is_empty() {
        return area;
    }
    let tw = entries.iter().map(|(s, _)| text_w(s, fs)).fold(0.0, f32::max);
    let block_w = SWATCH + GAP + tw;
    let block_h = lh * entries.len() as f32;
    let x = if left { area.x } else { area.right() - block_w };
    let y0 = area.y + ((area.h - block_h) / 2.0).max(0.0);
    for (k, (s, c)) in entries.iter().enumerate() {
        let y = y0 + lh * k as f32;
        swatch(out, x, y + (lh - SWATCH) / 2.0, *c, round);
        text_in(out, Rect { x: x + SWATCH + GAP, y, w: tw + 2.0, h: lh }, s, fs, TextAlign::Left, false, color);
    }
    let used = block_w + PAD;
    let mut area = area;
    area.w = (area.w - used).max(1.0);
    if left {
        area.x += used;
    }
    area
}

/// A legend in centred rows the width of `area` above or below it (at most
/// [`MAX_LEGEND_ROWS`], and half its height); returns the area left for the plot.
fn row_legend(out: &mut Vec<GraphicItem>, entries: &[(String, Rgb)], top: bool, area: Rect, fs: f32, round: bool, color: Rgb) -> Rect {
    let lh = fs * LINE;
    let max_rows = MAX_LEGEND_ROWS.min((area.h / 2.0 / lh).floor().max(1.0) as usize);
    let rows = legend_rows(entries, area.w, fs, max_rows);
    let y0 = if top { area.y } else { area.bottom() - lh * rows.len() as f32 };
    for (r, row) in rows.iter().enumerate() {
        let total = row.iter().map(|(_, _, w)| SWATCH + GAP + w).sum::<f32>() + SEP * row.len().saturating_sub(1) as f32;
        let mut x = area.x + ((area.w - total) / 2.0).max(0.0);
        let y = y0 + lh * r as f32;
        for (s, c, w) in row {
            swatch(out, x, y + (lh - SWATCH) / 2.0, *c, round);
            text_in(out, Rect { x: x + SWATCH + GAP, y, w: w + 2.0, h: lh }, s, fs, TextAlign::Left, false, color);
            x += SWATCH + GAP + w + SEP;
        }
    }
    let used = lh * rows.len() as f32 + PAD;
    let mut area = area;
    area.h = (area.h - used).max(1.0);
    if top {
        area.y += used;
    }
    area
}

/// Legend entries (text, colour, text width) packed into at most `max_rows` rows of `width`
/// points; the entries past the last row are left out.
fn legend_rows(entries: &[(String, Rgb)], width: f32, fs: f32, max_rows: usize) -> Vec<Vec<(String, Rgb, f32)>> {
    let mut rows: Vec<Vec<(String, Rgb, f32)>> = Vec::new();
    let mut row_w = 0.0;
    for (s, c) in entries {
        let w = text_w(s, fs);
        let entry_w = SWATCH + GAP + w;
        let fits = !rows.is_empty() && row_w + SEP + entry_w <= width;
        if fits {
            row_w += SEP + entry_w;
        } else if rows.len() < max_rows {
            rows.push(Vec::new());
            row_w = entry_w;
        } else {
            break;
        }
        if let Some(row) = rows.last_mut() {
            row.push((s.clone(), *c, w));
        }
    }
    rows
}
