//! Table Design › Borders on the selected cells (Word's Borders menu with cells selected): All,
//! Outside, Inside, Top, Bottom, Left, Right and No Border change the cells' own borders
//! (`w:tcBorders`) rather than the whole table's, in a line style, width and colour. Changing a
//! side also changes the facing side of the cell beside it, as the eraser does (`table_draw`):
//! both cells draw that one line.
//!
//! [`cell_borders_state`] reads what the selected cells show (cell > row exception > table and
//! table style, the order `wordcraft_layout` draws them in), for a Borders menu's checked items.
//! It doesn't read a table style's conditional parts (header row, banding…).

use super::cells::{cell, selected_cells};
use super::layout::touch_cells;
use super::{Border, BorderStyle, Borders, CmdError, CmdResult, Rgb, Session, Table, Value, json, p};
use wordcraft_doc::{Path, StoryRef};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Side {
    Top,
    Bottom,
    Left,
    Right,
}
const SIDES: [Side; 4] = [Side::Top, Side::Bottom, Side::Left, Side::Right];

/// The kinds `table.cellBorders` takes.
const KINDS: [&str; 9] = ["all", "outside", "inside", "top", "bottom", "left", "right", "none", "restyle"];

/// The selected cells: the table, the rows and grid columns they cover (both inclusive) and the
/// cells (row, cell index).
struct Target {
    story: StoryRef,
    table: Path,
    rows: (usize, usize),
    cols: (usize, usize),
    cells: Vec<(usize, usize)>,
}

fn target(s: &Session) -> Option<Target> {
    let (tp, _, _) = s.sel.focus.path.cell()?;
    let story = s.sel.focus.story;
    let t = s.doc.table(story, &tp)?;
    if let Some(cr) = s.cell_range()
        && cr.table == tp
        && cr.story == story
    {
        return Some(Target { story, table: tp, rows: cr.rows, cols: cr.cols, cells: cr.cells(t) });
    }
    // A caret, or text selected in a cell or across cells: the cells the selection touches.
    let cells: Vec<(usize, usize)> = selected_cells(s).into_iter().filter(|&(r, c)| t.rows.get(r).is_some_and(|row| c < row.cells.len())).collect();
    let mut bounds: Option<(usize, usize, usize, usize)> = None;
    for &(r, c) in &cells {
        let (g0, g1) = super::grid_cols(t, r, c);
        bounds = Some(bounds.map_or((r, r, g0, g1), |(r0, r1, a, b)| (r0.min(r), r1.max(r), a.min(g0), b.max(g1))));
    }
    let (r0, r1, g0, g1) = bounds?;
    Some(Target { story, table: tp, rows: (r0, r1), cols: (g0, g1), cells })
}

/// Whether side `side` of cell (r, c) is on the outside of the selected block.
fn outer(t: &Table, tg: &Target, r: usize, c: usize, side: Side) -> bool {
    let (g0, g1) = super::grid_cols(t, r, c);
    match side {
        Side::Top => r <= tg.rows.0,
        Side::Bottom => r >= tg.rows.1,
        Side::Left => g0 <= tg.cols.0,
        Side::Right => g1 >= tg.cols.1,
    }
}

/// The line drawn on side `side` of cell (r, c): the cell's own, else the row exception's, else
/// the table's (`tb`, the table style's under the table's own) — `layout_table`'s order.
fn effective(t: &Table, tb: &Borders, r: usize, c: usize, side: Side) -> Option<Border> {
    let row = t.rows.get(r)?;
    let cl = row.cells.get(c)?;
    let cb = cl.props.borders.unwrap_or_default();
    let rb = row.props.exceptions.as_ref().and_then(|e| e.borders).unwrap_or_default();
    let n = t.rows.len();
    let g = t.grid_col(r, c);
    let first_g = g == row.first_grid();
    let last_g = g.saturating_add(cl.span()) >= t.cols().saturating_sub(row.grid_after());
    match side {
        Side::Top => cb.top.or(if r == 0 { rb.top } else { rb.between }).or(if r == 0 { tb.top } else { tb.between }),
        Side::Bottom => cb.bottom.or(if r + 1 == n { rb.bottom } else { rb.between }).or(if r + 1 == n { tb.bottom } else { tb.between }),
        Side::Left => cb.left.or(if first_g { rb.left } else { rb.inside_v }).or(if first_g { tb.left } else { tb.inside_v }),
        Side::Right => cb.right.or(if last_g { rb.right } else { rb.inside_v }).or(if last_g { tb.right } else { tb.inside_v }),
    }
}

/// The table's borders: its style's, with its own on top (`layout`'s table frame).
fn table_borders(s: &Session, t: &Table) -> Borders {
    let mut tb = t.props.style.as_deref().and_then(|id| s.doc.styles.table_style(id)).and_then(|st| st.parts.borders).unwrap_or_default();
    if let Some(own) = t.props.borders {
        tb.overlay(&own);
    }
    tb
}

/// One value when they all agree, else null (also when there are none).
fn agree(v: &[bool]) -> Value {
    match v.first() {
        Some(&f) if v.iter().all(|&x| x == f) => json!(f),
        _ => Value::Null,
    }
}

fn same<T: PartialEq + Clone>(v: &[T]) -> Option<T> {
    v.first().filter(|f| v.iter().all(|x| x == *f)).cloned()
}

/// The line styles the command takes (their OOXML names).
pub(super) fn parse_style(x: &str) -> Result<BorderStyle, CmdError> {
    Ok(match x {
        "single" => BorderStyle::Single,
        "dotted" => BorderStyle::Dotted,
        "dashed" => BorderStyle::Dashed,
        "dotDash" => BorderStyle::DotDash,
        "double" => BorderStyle::Double,
        "triple" => BorderStyle::Triple,
        "thick" => BorderStyle::Thick,
        "wave" => BorderStyle::Wave,
        _ => return Err(CmdError::Params(format!("unknown border style `{x}`"))),
    })
}

/// The selected cells' borders (a caret: its cell's): `{top, bottom, left, right, insideH,
/// insideV, style, color, width, span: [rows, grid columns]}`. Each side is true (every line
/// shows), false (none does) or null (mixed; the inside lines are null with one row or column).
/// `style` (OOXML name), `color` ("RRGGBB" or "auto") and `width` (pt) are those of the lines
/// that show when they all agree, else null. Null outside a table.
pub fn cell_borders_state(s: &Session) -> Value {
    let Some(tg) = target(s) else { return Value::Null };
    let Some(t) = s.doc.table(tg.story, &tg.table) else { return Value::Null };
    let tb = table_borders(s, t);
    let mut groups: [Vec<bool>; 6] = Default::default();
    let (mut styles, mut colors, mut widths) = (Vec::new(), Vec::new(), Vec::new());
    for &(r, c) in &tg.cells {
        for side in SIDES {
            let b = effective(t, &tb, r, c, side).filter(Border::is_visible);
            let i = if outer(t, &tg, r, c, side) {
                side as usize
            } else if matches!(side, Side::Top | Side::Bottom) {
                4
            } else {
                5
            };
            if let Some(g) = groups.get_mut(i) {
                g.push(b.is_some());
            }
            if let Some(b) = b {
                styles.push(b.style.ooxml());
                colors.push(b.color.map_or_else(|| "auto".to_string(), Rgb::hex));
                widths.push((b.width * 100.0).round() as i32);
            }
        }
    }
    let [top, bottom, left, right, inside_h, inside_v] = groups;
    json!({
        "top": agree(&top), "bottom": agree(&bottom), "left": agree(&left), "right": agree(&right),
        "insideH": agree(&inside_h), "insideV": agree(&inside_v),
        "style": same(&styles), "color": same(&colors), "width": same(&widths).map(|w| w as f32 / 100.0),
        "span": [tg.rows.1.saturating_sub(tg.rows.0) + 1, tg.cols.1.saturating_sub(tg.cols.0) + 1],
    })
}

fn set_side(cl: &mut wordcraft_doc::Cell, side: Side, b: Border) {
    let bs = cl.props.borders.get_or_insert_with(Default::default);
    match side {
        Side::Top => bs.top = Some(b),
        Side::Bottom => bs.bottom = Some(b),
        Side::Left => bs.left = Some(b),
        Side::Right => bs.right = Some(b),
    }
}

/// `table.cellBorders` — see the module docs. Returns [`cell_borders_state`] after the change.
pub(super) fn cell_borders(s: &mut Session, v: &Value) -> CmdResult {
    let kind = p::req_str(v, "kind")?;
    if !KINDS.contains(&kind) {
        return Err(CmdError::Params(format!("unknown border kind `{kind}`")));
    }
    let on = p::bool(v, "on").unwrap_or(true) && kind != "none";
    let style = p::str(v, "style").map(parse_style).transpose()?;
    let width = p::f32(v, "width").filter(|w| w.is_finite()).map(|w| w.clamp(0.25, 6.0));
    // `color`: "RRGGBB"; null is automatic (black). Restyle keeps each line's colour without one.
    let color: Option<Option<Rgb>> = match v.get("color") {
        None => None,
        Some(Value::Null) => Some(None),
        Some(x) => Some(Some(x.as_str().and_then(Rgb::parse).ok_or_else(|| CmdError::Params("`color` is \"RRGGBB\" or null".into()))?)),
    };
    let (tp, _, _) = cell(s)?;
    let tg = target(s).ok_or_else(|| CmdError::Disabled("the cursor isn't in a table".into()))?;
    let pen = Border { style: style.unwrap_or(BorderStyle::Single), width: width.unwrap_or(0.5), color: color.flatten(), space: 0.0 };
    let nil = Border { style: BorderStyle::None, width: 0.0, color: None, space: 0.0 };
    // The sides to set and their new lines; the table is only read while choosing.
    let edits: Vec<(usize, usize, Side, Border)> = {
        let t = s.doc.table(tg.story, &tg.table).ok_or_else(|| CmdError::Disabled("the cursor isn't in a table".into()))?;
        let tb = table_borders(s, t);
        let mut out = Vec::new();
        for &(r, c) in &tg.cells {
            for side in SIDES {
                let o = outer(t, &tg, r, c, side);
                let line = if kind == "restyle" {
                    // Only the lines that show, keeping what isn't given.
                    let Some(cur) = effective(t, &tb, r, c, side).filter(Border::is_visible) else { continue };
                    Border {
                        style: style.unwrap_or(cur.style),
                        width: width.unwrap_or(cur.width),
                        color: color.unwrap_or(cur.color),
                        space: cur.space,
                    }
                } else {
                    let hit = match kind {
                        "all" | "none" => true,
                        "outside" => o,
                        "inside" => !o,
                        "top" => o && side == Side::Top,
                        "bottom" => o && side == Side::Bottom,
                        "left" => o && side == Side::Left,
                        _ => o && side == Side::Right,
                    };
                    if !hit {
                        continue;
                    }
                    if on { pen } else { nil }
                };
                out.push((r, c, side, line));
                // The facing side of the cell beside it.
                let (g0, g1) = super::grid_cols(t, r, c);
                match side {
                    Side::Top | Side::Bottom => {
                        let (nr, opp) =
                            if side == Side::Top { (r.checked_sub(1), Side::Bottom) } else { (Some(r + 1).filter(|&x| x < t.rows.len()), Side::Top) };
                        if let Some(nr) = nr {
                            let mut g = g0;
                            while g <= g1 {
                                let Some(nc) = t.cell_at_grid(nr, g) else { break };
                                out.push((nr, nc, opp, line));
                                g = super::grid_cols(t, nr, nc).1.saturating_add(1);
                            }
                        }
                    }
                    Side::Left => {
                        if let Some(nc) = g0.checked_sub(1).and_then(|x| t.cell_at_grid(r, x)) {
                            out.push((r, nc, Side::Right, line));
                        }
                    }
                    Side::Right => {
                        if let Some(nc) = t.cell_at_grid(r, g1.saturating_add(1)) {
                            out.push((r, nc, Side::Left, line));
                        }
                    }
                }
            }
        }
        out
    };
    if !edits.is_empty() {
        let t = s.doc.table_mut(tg.story, &tg.table)?;
        for (r, c, side, line) in edits {
            if let Some(cl) = t.rows.get_mut(r).and_then(|row| row.cells.get_mut(c)) {
                set_side(cl, side, line);
            }
        }
        touch_cells(s, &tp)?;
    }
    Ok(cell_borders_state(s))
}
