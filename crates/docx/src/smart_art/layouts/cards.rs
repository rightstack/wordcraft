//! Timelines, Meet the Team and Text Cards.

use wordcraft_doc::smart_art::{SmartArtColors, SmartArtSpec};

use crate::smart_art::shapes::{
    Ink, Paint, ROUND_INSET, Sh, avatar, graded, graded_light, ink_on, inset, label, line_box, lines_of, plain, tag, tint, tops, with_text,
};

/// Dots on a line across the middle; each item's text above its dot, then below, in turn,
/// joined to it by a short stem.
pub(crate) fn timeline_dots(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn timeline_numbered(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn timeline_arrow(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn timeline_vertical(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn team_circles(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn team_cards(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn team_pills(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn card_numbered(spec: &SmartArtSpec, w: f32, h: f32, big: bool) -> Vec<Sh> {
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
pub(crate) fn card_header(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn card_tall(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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
pub(crate) fn card_rows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
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

/// Stems up from a line across the middle and down, in turn, each ending in a dot in the item's
/// colour with the item's text beyond it.
pub(crate) fn timeline_stems(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let cy = h / 2.0;
    let d = (cw * 0.14).min(h * 0.07);
    let stem = h * 0.17;
    let gap = h * 0.03;
    // Labels on one side are two columns apart, so each may take nearly two columns.
    let lw = if n > 1 { cw * 1.8 } else { cw * 0.96 }.min(w);
    let mut line = Sh::line(0.0, cy, w, cy, spec.colors.accent(0));
    line.line_w = 2.0;
    let mut out = vec![line];
    for (k, &i) in tops.iter().enumerate() {
        let cx = cw * (k as f32 + 0.5);
        let up = k.is_multiple_of(2);
        let end = if up { cy - stem } else { cy + stem };
        let p = graded(spec, k, n);
        let mut s = Sh::line(cx, cy, cx, end, spec.colors.accent(k));
        s.line = p;
        s.line_w = 1.5;
        out.push(s);
        out.push(plain("ellipse", [cx - d / 2.0, end - d / 2.0, d, d], p).of_item(i));
        let x = (cx - lw / 2.0).clamp(0.0, (w - lw).max(0.0));
        let b = if up {
            let top = h * 0.02;
            [x, top, lw, (end - d / 2.0 - gap - top).max(0.0)]
        } else {
            let top = end + d / 2.0 + gap;
            [x, top, lw, (h * 0.98 - top).max(0.0)]
        };
        out.push(label(spec, i, b, "ctr", if up { "b" } else { "t" }));
    }
    out
}

/// Numbered circles overlapping along a line, larger and (in one accent) stronger towards the
/// middle; each item's text under its circle.
pub(crate) fn timeline_circles(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let mid = (n - 1) as f32 / 2.0;
    // Centres three quarters of the largest circle apart: neighbours overlap, yet each centre
    // stays clear of the circles beside it (whose radius is at most half the largest).
    let big = (w / (0.75 * (n - 1) as f32 + 1.0)).min(h * 0.42);
    let step = big * 0.75;
    let x0 = (w - step * (n - 1) as f32) / 2.0;
    let centre = |k: usize| x0 + step * k as f32;
    let lw = if n > 1 { step * 0.96 } else { w * 0.96 };
    let cy = ((h - big - h * 0.34) / 2.0).max(h * 0.03) + big / 2.0;
    let ty = cy + big / 2.0 + h * 0.04;
    let away = |k: usize| if mid > 0.0 { (k as f32 - mid).abs() / mid } else { 0.0 };
    let mut out = Vec::new();
    if n > 1 {
        let mut line = Sh::line(centre(0), cy, centre(n - 1), cy, spec.colors.accent(0));
        line.line_w = 2.0;
        out.push(line);
    }
    // The outer circles first, so the middle ones lie on top.
    let mut order: Vec<usize> = (0..tops.len()).collect();
    order.sort_by(|a, b| away(*b).total_cmp(&away(*a)));
    for k in order {
        let Some(&i) = tops.get(k) else { continue };
        let t = away(k);
        let a = spec.colors.accent(k);
        let p = if spec.colors == SmartArtColors::Colorful || n <= 1 {
            Paint::Accent(a, None)
        } else {
            Paint::Accent(a, Some(100_000 - (55_000.0 * t).round().clamp(0.0, 55_000.0) as u32).filter(|v| *v < 100_000))
        };
        let d = big * (1.0 - 0.25 * t);
        let cx = centre(k);
        let b = [cx - d / 2.0, cy - d / 2.0, d, d];
        let mut c = with_text(plain("ellipse", b, p), (k + 1).to_string(), "ctr", ink_on(p), 1.0).of_item(i).text_box(inset(b, ROUND_INSET));
        c.line = Paint::Light;
        c.line_w = 2.0;
        out.push(c);
    }
    for (k, &i) in tops.iter().enumerate() {
        let x = (centre(k) - lw / 2.0).clamp(0.0, (w - lw).max(0.0));
        out.push(label(spec, i, [x, ty, lw, (h * 0.98 - ty).max(0.0)], "ctr", "t"));
    }
    out
}

/// Numbered circles high and low in turn, joined by slanted strokes with a small dot halfway;
/// each item's text under its circle.
pub(crate) fn timeline_zigzag(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.5).min(h * 0.22);
    let hi = h * 0.04 + d / 2.0;
    let lo = if n > 1 { hi + d * 0.9 } else { hi };
    let cx = |k: usize| cw * (k as f32 + 0.5);
    let cy = |k: usize| if k.is_multiple_of(2) { hi } else { lo };
    let mut out = Vec::new();
    for k in 1..tops.len() {
        let mut s = Sh::line(cx(k - 1), cy(k - 1), cx(k), cy(k), spec.colors.accent(k - 1));
        s.line_w = 1.5;
        out.push(s);
        let dot = d * 0.22;
        let (mx, my) = ((cx(k - 1) + cx(k)) / 2.0, (cy(k - 1) + cy(k)) / 2.0);
        out.push(plain("ellipse", [mx - dot / 2.0, my - dot / 2.0, dot, dot], graded(spec, k - 1, n)));
    }
    for (k, &i) in tops.iter().enumerate() {
        let p = graded(spec, k, n);
        let b = [cx(k) - d / 2.0, cy(k) - d / 2.0, d, d];
        let mut c = with_text(plain("ellipse", b, p), (k + 1).to_string(), "ctr", ink_on(p), 1.0).of_item(i).text_box(inset(b, ROUND_INSET));
        c.line = Paint::Light;
        c.line_w = 2.0;
        out.push(c);
    }
    for (k, &i) in tops.iter().enumerate() {
        let ty = cy(k) + d / 2.0 + h * 0.04;
        out.push(label(spec, i, [cx(k) - cw * 0.47, ty, cw * 0.94, (h * 0.98 - ty).max(0.0)], "ctr", "t"));
    }
    out
}

/// Small dots on a thin line near the top; under each dot the item's text, left-aligned from
/// the dot: its own line in the item's colour, the lines under it below.
pub(crate) fn timeline_line(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let d = (cw * 0.09).min(h * 0.07);
    // The dots and about half the height of text, centred top to bottom.
    let ly = ((h - d - h * 0.5) / 2.0).max(h * 0.04) + d / 2.0;
    let title_h = h * 0.16;
    let mut line = Sh::line(0.0, ly, w, ly, spec.colors.accent(0));
    line.line_w = 1.5;
    let mut out = vec![line];
    for (k, &i) in tops.iter().enumerate() {
        let x0 = cw * k as f32 + cw * 0.04;
        let tw = cw * 0.9;
        out.push(plain("ellipse", [x0, ly - d / 2.0, d, d], graded(spec, k, n)).of_item(i));
        let ty = ly + d / 2.0 + h * 0.06;
        let tb = [x0, ty, tw, title_h];
        let own = spec.items.get(i).map(|it| it.text.clone()).unwrap_or_default();
        let mut title = with_text(plain("rect", tb, Paint::None), own, "l", Ink::Accent(spec.colors.accent(k)), 1.15).of_item(i).text_box(tb);
        title.anchor = "t";
        out.push(title);
        let body = lines_of(spec, i, false, true);
        if !body.is_empty() {
            let by = ty + title_h;
            let b = [x0, by, tw, (h * 0.98 - by).max(0.0)];
            let mut s = plain("rect", b, Paint::None).of_item(i).text_box(b);
            s.text = body;
            s.algn = Some("l");
            s.anchor = "t";
            out.push(s);
        }
    }
    out
}

/// Tinted columns: the item's text and the lines under it on top; the last of those lines set
/// apart under a rule, large and in the item's colour (a figure, a short phrase).
pub(crate) fn card_emphasis(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let (y0, ch) = (h * 0.03, h * 0.94);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let (x0, card_w) = (cw * k as f32 + cw * 0.04, cw * 0.92);
        let (tx, tw) = (x0 + card_w * 0.08, card_w * 0.84);
        let mut lines = lines_of(spec, i, true, true);
        let last = if lines.len() > 1 { lines.pop() } else { None };
        let mut card = plain("rect", [x0, y0, card_w, ch], graded_light(spec, k, n, 18_000, 8_000));
        card.text = lines;
        card.item = Some(i);
        card.tb = [tx, y0 + ch * 0.06, tw, if last.is_some() { ch * 0.54 } else { ch * 0.88 }];
        card.algn = Some("l");
        card.anchor = "t";
        out.push(card);
        if let Some((text, _)) = last {
            let a = spec.colors.accent(k);
            let ry = y0 + ch * 0.64;
            let mut rule = Sh::line(tx, ry, tx + tw, ry, a);
            rule.line_w = 1.0;
            out.push(rule);
            let b = [tx, ry + ch * 0.04, tw, (y0 + ch * 0.96 - ry - ch * 0.04).max(0.0)];
            out.push(with_text(plain("rect", b, Paint::None), text, "l", Ink::Accent(a), 1.6).of_item(i).text_box(b));
        }
    }
    out
}

/// Rows of text with a small square in the item's colour at each row's right end and a thin rule
/// between rows.
pub(crate) fn card_marker_rows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let s = (rh * 0.3).min(w * 0.06);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        let pad = w * 0.02;
        out.push(label(spec, i, [pad, y, (w - s - pad * 4.0).max(0.0), rh], "l", "ctr"));
        out.push(plain("rect", [w - s - pad, y + (rh - s) / 2.0, s, s], graded(spec, k, n)).of_item(i));
        if k + 1 < n {
            let mut rule = Sh::line(pad, y + rh, w - pad, y + rh, spec.colors.accent(k));
            rule.line_w = 0.75;
            out.push(rule);
        }
    }
    out
}

/// Tinted squares, each with a large number in the item's colour over the item's text.
pub(crate) fn card_squares(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let side = (cw * 0.92).min(h * 0.94);
    let y0 = (h - side) / 2.0;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x0 = cw * k as f32 + (cw - side) / 2.0;
        let (tx, tw) = (x0 + side * 0.1, side * 0.8);
        let mut card = plain("rect", [x0, y0, side, side], graded_light(spec, k, n, 30_000, 12_000));
        card.text = lines_of(spec, i, true, true);
        card.item = Some(i);
        card.tb = [tx, y0 + side * 0.42, tw, side * 0.5];
        card.algn = Some("l");
        card.anchor = "t";
        out.push(card);
        let nb = [tx, y0 + side * 0.06, tw, side * 0.32];
        let num = with_text(plain("rect", nb, Paint::None), (k + 1).to_string(), "l", Ink::Accent(spec.colors.accent(k)), 2.0);
        let mut num = num.of_item(i).text_box(nb);
        num.anchor = "b";
        out.push(num);
    }
    out
}

// ---- fourth batch ----

/// A line across with a large circle per top item (its own line inside) and a small circle for
/// each item under it; a small circle's text under it. Every item is a shape of its own.
pub(crate) fn timeline_circle_accent(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let n = spec.items.len().max(1);
    let weight = |i: usize| if spec.items.get(i).is_some_and(|it| it.level == 0) { 1.7f32 } else { 1.0 };
    let total: f32 = (0..spec.items.len()).map(weight).sum::<f32>().max(1.0);
    let unit = w / total;
    let big = (unit * 1.7 * 0.94).min(h * 0.5);
    let small = (unit * 0.5).min(big * 0.4);
    // The line a little above the middle: room under it for the small circles' text.
    let ly = (h * 0.38).max(big / 2.0 + h * 0.01);
    let mut centres = Vec::with_capacity(n);
    let mut at = 0.0f32;
    for i in 0..spec.items.len() {
        let wi = weight(i);
        centres.push(at + wi * unit / 2.0);
        at += wi * unit;
    }
    let mut out = Vec::new();
    if let (Some(&a), Some(&b)) = (centres.first(), centres.last())
        && b - a > 0.5
    {
        let mut l = Sh::line(a, ly, b, ly, spec.colors.accent(0));
        l.line_w = 2.0;
        out.push(l);
    }
    let mut top = 0usize;
    let tops_n = tops(spec).len().max(1);
    let mut accent_k = 0usize;
    for (i, it) in spec.items.iter().enumerate() {
        let x = centres.get(i).copied().unwrap_or(0.0);
        if it.level == 0 {
            accent_k = top;
            let b = [x - big / 2.0, ly - big / 2.0, big, big];
            let mut c = tag("ellipse", b, graded(spec, top, tops_n), spec, i);
            c.tb = inset(b, ROUND_INSET);
            c.line = Paint::Light;
            c.line_w = 2.0;
            out.push(c);
            top += 1;
        } else {
            let mut c = plain("ellipse", [x - small / 2.0, ly - small / 2.0, small, small], tint(spec, accent_k, 70_000)).of_item(i);
            c.line = Paint::Light;
            c.line_w = 1.5;
            out.push(c);
            // Out to a large circle's middle beside it, or half way to a small one's.
            let side =
                |j: Option<usize>, edge: f32| match j.and_then(|j| spec.items.get(j).map(|o| (o.level, centres.get(j).copied().unwrap_or(edge)))) {
                    Some((0, c)) => c,
                    Some((_, c)) => (c + x) / 2.0,
                    None => edge,
                };
            let (l, r) = (side(i.checked_sub(1), 0.0), side(Some(i + 1).filter(|&j| j < spec.items.len()), w));
            let ty = ly + big / 2.0 + h * 0.03;
            let lb = [l.max(0.0), ty, (r.min(w) - l.max(0.0)).max(unit * 0.5), (h * 0.99 - ty).max(0.0)];
            out.push(line_box(lb, it.text.clone(), true, "ctr", i));
            if let Some(last) = out.last_mut() {
                last.anchor = "t";
            }
        }
    }
    out
}
