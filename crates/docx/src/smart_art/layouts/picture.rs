//! Picture layouts: a picture placeholder per item ([`picture`]) with the item's text.

use wordcraft_doc::smart_art::SmartArtSpec;

use crate::smart_art::shapes::{Paint, Sh, best_cols, graded, graded_light, grid_cell, ink_on, inset, label, lines_of, picture, plain, tops};

/// Pictures in a row, landscape, each item's text under its picture.
pub(crate) fn picture_captions(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    // A cell is a picture (4:3) over about half as much again of text; in a narrow frame the
    // pictures wrap onto more rows.
    let cols = best_cols(n, w, h, 0.8);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cell = grid_cell(k, n, cols, [0.0, 0.0, w, h]);
        let pw = (cell[2] * 0.88).min(cell[3] * 0.62 / 0.75);
        let ph = pw * 0.75;
        let x = cell[0] + (cell[2] - pw) / 2.0;
        let y0 = cell[1] + ((cell[3] - ph - cell[3] * 0.34) / 2.0).max(cell[3] * 0.02);
        let ty = y0 + ph + cell[3] * 0.04;
        out.extend(picture([x, y0, pw, ph], "roundRect", spec.colors.accent(k), i));
        let tw = cell[2] * 0.94;
        out.push(label(spec, i, [cell[0] + (cell[2] - tw) / 2.0, ty, tw, (cell[1] + cell[3] * 0.98 - ty).max(0.0)], "ctr", "t"));
    }
    out
}

/// Cards in a grid: a picture over a band in the item's colour that holds its text.
pub(crate) fn picture_grid(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cols = best_cols(n, w, h, 1.25);
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let cell = grid_cell(k, n, cols, [0.0, 0.0, w, h]);
        let (cw, ch) = ((cell[2] * 0.92).min(cell[3] * 0.92 * 1.25), (cell[3] * 0.92).min(cell[2] * 0.92 / 1.25));
        let (x, y) = (cell[0] + (cell[2] - cw) / 2.0, cell[1] + (cell[3] - ch) / 2.0);
        let band = ch * 0.34;
        out.extend(picture([x, y, cw, ch - band], "rect", spec.colors.accent(k), i));
        let p = graded(spec, k, n);
        let b = [x, y + ch - band, cw, band];
        let mut cap = plain("rect", b, p).of_item(i);
        cap.text = lines_of(spec, i, true, true);
        cap.tb = [x + cw * 0.06, b[1], cw * 0.88, band];
        cap.algn = Some("l");
        out.push(cap);
    }
    out
}

/// Tinted cards, each with a round picture over its top edge and the item's text inside.
pub(crate) fn picture_circles(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let card_w = cw * 0.86;
    let d = (card_w * 0.62).min(h * 0.42);
    let y0 = h * 0.02;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + (cw - card_w) / 2.0;
        let top = y0 + d / 2.0;
        let mut card = plain("roundRect", [x, top, card_w, (h * 0.98 - top).max(0.0)], graded_light(spec, k, n, 30_000, 14_000));
        card.adj = vec![("adj", 8_000)];
        card.text = lines_of(spec, i, true, true);
        card.item = Some(i);
        let ty = y0 + d + h * 0.03;
        card.tb = [x + card_w * 0.08, ty, card_w * 0.84, (h * 0.95 - ty).max(0.0)];
        card.anchor = "t";
        card.algn = Some("ctr");
        out.push(card);
        let mut pic = picture([x + (card_w - d) / 2.0, y0, d, d], "ellipse", spec.colors.accent(k), i);
        if let Some(frame) = pic.first_mut() {
            frame.line = Paint::Light;
            frame.line_w = 2.0;
        }
        out.extend(pic);
    }
    out
}

/// Rows, each with a picture on the left and the item's text beside it, a thin rule between rows.
pub(crate) fn picture_rows(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let rh = h / n as f32;
    let ph = rh * 0.8;
    let pw = (ph * 1.4).min(w * 0.32);
    let tx = pw + w * 0.04;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let y = rh * k as f32;
        out.extend(picture([0.0, y + (rh - ph) / 2.0, pw, ph], "roundRect", spec.colors.accent(k), i));
        out.push(label(spec, i, [tx, y, (w - tx).max(0.0), rh], "l", "ctr"));
        if k + 1 < n {
            let mut rule = Sh::line(tx, y + rh, w, y + rh, spec.colors.accent(k));
            rule.line_w = 0.75;
            out.push(rule);
        }
    }
    out
}

/// Tall picture strips side by side, each ending in a caption band in the item's colour.
pub(crate) fn picture_strips(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let sw = cw * 0.9;
    let (y0, sh) = (h * 0.02, h * 0.96);
    let band = sh * 0.3;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + (cw - sw) / 2.0;
        out.extend(picture([x, y0, sw, sh - band], "rect", spec.colors.accent(k), i));
        let p = graded(spec, k, n);
        let b = [x, y0 + sh - band, sw, band];
        let mut cap = plain("rect", b, p).of_item(i);
        cap.text = lines_of(spec, i, true, true);
        cap.tb = inset(b, 0.08);
        cap.ink = ink_on(p);
        out.push(cap);
    }
    out
}

/// Columns with the picture high and the text low, then the other way round, in turn.
pub(crate) fn picture_staggered(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<Sh> {
    let tops = tops(spec);
    let n = tops.len().max(1);
    let cw = w / n as f32;
    let pw = cw * 0.9;
    let half = h * 0.48;
    let mut out = Vec::new();
    for (k, &i) in tops.iter().enumerate() {
        let x = cw * k as f32 + (cw - pw) / 2.0;
        let high = k.is_multiple_of(2);
        let (py, ty) = if high { (h * 0.02, h * 0.52) } else { (h * 0.5, h * 0.02) };
        out.extend(picture([x, py, pw, half], "roundRect", spec.colors.accent(k), i));
        out.push(label(spec, i, [x, ty, pw, half * 0.96], "ctr", if high { "t" } else { "b" }));
        // A short bar in the item's colour between the picture and its text.
        let bar_y = if high { h * 0.5 + h * 0.005 } else { h * 0.48 + h * 0.005 };
        out.push(plain("rect", [x + pw * 0.35, bar_y, pw * 0.3, h * 0.012], graded(spec, k, n)));
    }
    out
}
