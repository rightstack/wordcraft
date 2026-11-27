//! Pyramid layouts.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Paint, Sh, graded, graded_light, lines_of, plain, tint, tops};

/// The `trapezoid` preset's `adj` that sets its short side `inset` in from each end, for a box
/// `w` × `h`.
fn trapezoid_adj(inset: f32, w: f32, h: f32) -> i64 {
    let ss = w.min(h).max(0.01);
    (inset / ss * 100_000.0).round().clamp(0.0, 10_000_000.0) as i64
}

/// Bands stacked into an upside-down triangle: the first item in the wide band at the top, the
/// last in the point at the bottom.
pub(crate) fn pyramid_inverted(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1) as f32;
    let tw = w.min(h * 1.3);
    let x0 = (w - tw) / 2.0;
    let bh = h / n;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (top_w, bot_w) = (tw * (n - k as f32) / n, tw * (n - k as f32 - 1.0) / n);
        let b = [x0 + (tw - top_w) / 2.0, k as f32 * bh, top_w, bh];
        let p = graded(spec, k, n as usize);
        let last = k + 1 == tops.len();
        let mut s = if last {
            // The point: a triangle turned upside down, its text in the wide upper part.
            let mut s = plain("triangle", b, p).turned(180.0);
            s.tb = [b[0] + top_w * 0.11, b[1] + bh * 0.04, top_w * 0.78, bh * 0.32];
            s
        } else {
            // A trapezoid turned over: the long side at the top.
            let mut s = plain("trapezoid", b, p).turned(180.0);
            s.adj = vec![("adj", trapezoid_adj((top_w - bot_w) / 2.0, top_w, bh))];
            let mid_w = (top_w + bot_w) / 2.0 * 0.8;
            s.tb = [w / 2.0 - mid_w / 2.0, b[1] + bh * 0.08, mid_w, bh * 0.84];
            s
        };
        s.text = lines_of(spec, i, true, true);
        s.item = Some(i);
        s.algn = Some("ctr");
        s.line = Paint::Light;
        s.line_w = 1.0;
        out.push(s);
    }
    out
}

/// A tinted triangle with a light bar per item across it, from its middle out to the right.
pub(crate) fn pyramid_list(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let tw = (h * 1.15).min(w * 0.62);
    let mut out = vec![plain("triangle", [0.0, 0.0, tw, h], graded_light(spec, 0, 1, 45_000, 45_000))];
    // Bars share the lower part of the triangle (the point stays clear).
    let (top, bottom) = (h * 0.2, h * 0.98);
    let rh = (bottom - top) / n as f32;
    let bx = tw * 0.32;
    for (k, &i) in tops.iter().enumerate() {
        let bh = rh * 0.78;
        let b = [bx, top + rh * k as f32 + (rh - bh) / 2.0, (w * 0.99 - bx).max(0.0), bh];
        let mut bar = plain("roundRect", b, Paint::Light);
        bar.adj = vec![("adj", 12_000)];
        bar.line = Paint::Accent(spec.colors.accent(k), None);
        bar.line_w = 1.5;
        bar.text = lines_of(spec, i, true, true);
        bar.item = Some(i);
        bar.tb = [b[0] + b[2] * 0.05, b[1], b[2] * 0.9, bh];
        bar.algn = Some("l");
        out.push(bar);
    }
    out
}

/// A triangle cut into rows of smaller triangles (one, three, five…), pointing up and down in
/// turn: one per item, the rest left pale.
pub(crate) fn pyramid_segmented(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rows = (1..=n).find(|r| r * r >= n).unwrap_or(n).max(1);
    let tw = w.min(h * 1.15);
    let (sw, sh) = (tw / rows as f32, h / rows as f32);
    let x0 = (w - tw) / 2.0;
    let mut out = Vec::new();
    let mut k = 0usize;
    for r in 0..rows {
        let base = x0 + (tw - (r + 1) as f32 * sw) / 2.0;
        let y = r as f32 * sh;
        for j in 0..(2 * r + 1) {
            let up = j.is_multiple_of(2);
            let x = base + (j / 2) as f32 * sw + if up { 0.0 } else { sw / 2.0 };
            let b = [x, y, sw, sh];
            let item = tops.get(k).copied();
            k += 1;
            let p = match item {
                Some(_) => graded(spec, k - 1, n),
                None => tint(spec, 0, 15_000),
            };
            let mut s = plain("triangle", b, p);
            if !up {
                s = s.turned(180.0);
            }
            s.line = Paint::Light;
            s.line_w = 1.5;
            if let Some(i) = item {
                s.text = lines_of(spec, i, true, true);
                s.item = Some(i);
                s.tb = if up { [x + sw * 0.22, y + sh * 0.45, sw * 0.56, sh * 0.5] } else { [x + sw * 0.22, y + sh * 0.05, sw * 0.56, sh * 0.5] };
                s.algn = Some("ctr");
            }
            out.push(s);
        }
    }
    out
}
