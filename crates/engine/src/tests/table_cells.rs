//! Cell borders (`table.cellBorders`), alignment of the selected cells (`table.cellAlign` with
//! `h` or `v` alone, `para.align` over whole cells).

use serde_json::{Value, json};
use wordcraft_doc::props::{Align, Border, BorderStyle, Borders, VAlign};
use wordcraft_doc::{Path, StoryRef, Table};

use crate::Session;

fn run(s: &mut Session, id: &str, v: Value) -> Value {
    s.run(id, &v).unwrap_or_else(|e| panic!("{id}: {e}"))
}

/// A 3 × 3 table at body block 0, caret in its first cell.
fn table_doc() -> Session {
    let mut s = Session::new(wordcraft_doc::Document::new());
    run(&mut s, "insert.table", json!({"rows": 3, "cols": 3}));
    s
}

fn tp(s: &Session) -> Path {
    s.sel.focus.path.cell().unwrap().0
}

fn table(s: &Session) -> &Table {
    let tp = tp(s);
    s.doc.table(StoryRef::Body, &tp).unwrap()
}

/// Select whole cells: rows and grid columns, both inclusive.
fn pick(s: &mut Session, rows: (usize, usize), cols: (usize, usize)) {
    let tp = tp(s);
    crate::cmd::table::select_range(s, StoryRef::Body, &tp, rows, cols).unwrap();
    assert_eq!(s.cell_range().map(|r| (r.rows, r.cols)), Some((rows, cols)));
}

fn cb(s: &Session, r: usize, c: usize) -> Borders {
    table(s).rows[r].cells[c].props.borders.unwrap_or_default()
}

fn vis(x: Option<Border>) -> Option<bool> {
    x.map(|b| b.is_visible())
}

fn state(s: &Session) -> Value {
    crate::cmd::table::cell_borders_state(s)
}

#[test]
fn cell_border_state_reads_the_grid_and_none_clears_in_one_undo_step() {
    let mut s = table_doc();
    // A new table (Table Grid): every line shows. A caret: its cell alone (no inside lines).
    let st = state(&s);
    for k in ["top", "bottom", "left", "right"] {
        assert_eq!(st[k], json!(true), "{k}: {st}");
    }
    assert_eq!((st["insideH"].clone(), st["insideV"].clone(), st["span"].clone()), (Value::Null, Value::Null, json!([1, 1])));
    assert_eq!((st["style"].clone(), st["width"].clone()), (json!("single"), json!(0.5)));
    run(&mut s, "table.selectTable", json!({}));
    let st = state(&s);
    assert_eq!((st["insideH"].clone(), st["insideV"].clone(), st["span"].clone()), (json!(true), json!(true), json!([3, 3])), "{st}");
    let undo = s.undo_depth();
    let st = run(&mut s, "table.cellBorders", json!({"kind": "none"}));
    assert_eq!(s.undo_depth(), undo + 1, "one undo step");
    for k in ["top", "bottom", "left", "right", "insideH", "insideV"] {
        assert_eq!(st[k], json!(false), "{k}: {st}");
    }
    assert!(st["color"].is_null() && st["width"].is_null() && st["style"].is_null());
    run(&mut s, "edit.undo", json!({}));
    assert_eq!(state(&s)["insideH"], json!(true), "undo brings the lines back");
}

#[test]
fn cell_borders_touch_only_the_selected_cells_and_their_shared_edges() {
    let mut s = table_doc();
    pick(&mut s, (0, 1), (0, 1));
    assert_eq!(state(&s)["span"], json!([2, 2]));
    // The 2 × 2 block's inside lines go: its outer sides and the other cells stay.
    let st = run(&mut s, "table.cellBorders", json!({"kind": "inside", "on": false}));
    assert_eq!(
        (st["insideH"].clone(), st["insideV"].clone(), st["top"].clone(), st["right"].clone()),
        (json!(false), json!(false), json!(true), json!(true)),
        "{st}"
    );
    assert_eq!(vis(cb(&s, 0, 0).right), Some(false));
    assert_eq!(vis(cb(&s, 0, 1).left), Some(false));
    assert_eq!(cb(&s, 0, 1).right, None, "an outer side is untouched");
    assert_eq!(cb(&s, 2, 2), Borders::default(), "a cell outside the block is untouched");
    // Outside in a thick red line: the cells beside the block (row 2's top, column 2's left) too.
    let st = run(&mut s, "table.cellBorders", json!({"kind": "outside", "width": 1.5, "color": "FF0000"}));
    assert_eq!((st["top"].clone(), st["bottom"].clone(), st["insideH"].clone()), (json!(true), json!(true), json!(false)), "{st}");
    assert_eq!((st["color"].clone(), st["width"].clone()), (json!("FF0000"), json!(1.5)));
    let below = cb(&s, 2, 0).top.unwrap();
    assert_eq!((below.width, below.color.map(|c| c.hex())), (1.5, Some("FF0000".to_string())));
    assert_eq!(cb(&s, 1, 2).left.unwrap().width, 1.5);
    assert_eq!(cb(&s, 2, 2).top, None, "a cell not beside the block is untouched");
    // One side off.
    let st = run(&mut s, "table.cellBorders", json!({"kind": "top", "on": false}));
    assert_eq!((st["top"].clone(), st["left"].clone()), (json!(false), json!(true)));
    // All borders: every line, one colour (automatic), the default 0.5 pt.
    let st = run(&mut s, "table.cellBorders", json!({"kind": "all"}));
    assert_eq!(
        (st["insideV"].clone(), st["top"].clone(), st["color"].clone(), st["width"].clone()),
        (json!(true), json!(true), json!("auto"), json!(0.5)),
        "{st}"
    );
    // Restyle changes the lines that show only: the top stays off.
    run(&mut s, "table.cellBorders", json!({"kind": "top", "on": false}));
    let st = run(&mut s, "table.cellBorders", json!({"kind": "restyle", "width": 3.0, "color": "00B050"}));
    assert_eq!(
        (st["top"].clone(), st["left"].clone(), st["color"].clone(), st["width"].clone()),
        (json!(false), json!(true), json!("00B050"), json!(3.0)),
        "{st}"
    );
    assert!(s.run("table.cellBorders", &json!({"kind": "diagonal"})).is_err());
    assert!(s.run("table.cellBorders", &json!({"kind": "all", "style": "zigzag"})).is_err());
}

#[test]
fn cell_border_line_styles_restyle_and_survive_docx() {
    let mut s = table_doc();
    pick(&mut s, (0, 1), (0, 1));
    let st = run(&mut s, "table.cellBorders", json!({"kind": "outside", "style": "dotted", "width": 1.0}));
    assert_eq!(st["style"], Value::Null, "dotted outside, single inside: {st}");
    assert_eq!(cb(&s, 0, 0).top.unwrap().style, BorderStyle::Dotted);
    // Restyle with just a style keeps each line's width and colour.
    let st = run(&mut s, "table.cellBorders", json!({"kind": "restyle", "style": "double"}));
    assert_eq!(st["width"], Value::Null, "the inside lines are still 0.5 pt, the outside 1 pt: {st}");
    assert_eq!(st["style"], json!("double"));
    assert_eq!(cb(&s, 0, 0).top.unwrap().width, 1.0);
    run(&mut s, "caret.set", json!({"pos": {"story": "body", "path": [0, 2, 2, 0], "off": 0}}));
    run(&mut s, "table.cellBorders", json!({"kind": "bottom", "style": "dashed", "width": 2.25, "color": "0070C0"}));
    let bytes = crate::io::save_bytes("t.docx", &s.doc).unwrap();
    let doc = crate::io::open_bytes("t.docx", &bytes).unwrap();
    let t = doc.body.iter().find_map(|b| b.as_table()).unwrap();
    let side = |r: usize, c: usize| t.rows[r].cells[c].props.borders.unwrap_or_default();
    assert_eq!(side(0, 0).top.unwrap().style, BorderStyle::Double);
    assert_eq!(side(1, 1).bottom.unwrap().style, BorderStyle::Double);
    let b = side(2, 2).bottom.unwrap();
    assert_eq!((b.style, b.width, b.color.map(|c| c.hex())), (BorderStyle::Dashed, 2.25, Some("0070C0".to_string())));
    // Every style the command takes reads back as itself.
    for style in ["single", "dotted", "dashed", "dotDash", "double", "triple", "thick", "wave"] {
        let mut s = table_doc();
        run(&mut s, "table.cellBorders", json!({"kind": "all", "style": style}));
        let bytes = crate::io::save_bytes("t.docx", &s.doc).unwrap();
        let doc = crate::io::open_bytes("t.docx", &bytes).unwrap();
        let t = doc.body.iter().find_map(|b| b.as_table()).unwrap();
        assert_eq!(t.rows[0].cells[0].props.borders.unwrap().left.unwrap().style.ooxml(), style);
    }
    // The whole-table command takes a style too.
    let mut s = table_doc();
    run(&mut s, "table.borders", json!({"kind": "all", "style": "double"}));
    assert_eq!(table(&s).props.borders.unwrap().top.unwrap().style, BorderStyle::Double);
}

#[test]
fn cell_borders_are_tracked_and_respect_restrictions() {
    let mut s = table_doc();
    run(&mut s, "review.trackChanges", json!({"value": true}));
    run(&mut s, "review.trackingOptions", json!({"trackFormatting": true}));
    run(&mut s, "table.cellBorders", json!({"kind": "bottom", "style": "double"}));
    assert!(table(&s).rows[0].cells[0].props.fmt_change.is_some(), "a tracked cell formatting change");
    run(&mut s, "review.rejectAll", json!({}));
    assert_eq!(cb(&s, 0, 0).bottom, None, "Reject All takes it back");
    let mut s = table_doc();
    run(&mut s, "review.restrict", json!({"mode": "none", "formatting": true}));
    let e = s.run("table.cellBorders", &json!({"kind": "all"})).unwrap_err().to_string();
    assert!(e.contains("formatting is limited"), "{e}");
    let mut s = table_doc();
    run(&mut s, "review.restrict", json!({"mode": "readOnly"}));
    assert!(s.run("table.cellBorders", &json!({"kind": "all"})).is_err(), "a read-only document");
    assert!(s.run("table.cellAlign", &json!({"v": "center"})).is_err(), "a read-only document");
}

#[test]
fn cell_align_sets_horizontal_or_vertical_alone() {
    let mut s = table_doc();
    // Mixed vertical alignment: row 0 top, row 1 bottom.
    pick(&mut s, (1, 1), (0, 2));
    run(&mut s, "table.cellAlign", json!({"v": "bottom"}));
    pick(&mut s, (0, 1), (0, 2));
    let undo = s.undo_depth();
    run(&mut s, "table.cellAlign", json!({"h": "center"}));
    assert_eq!(s.undo_depth(), undo + 1);
    let t = table(&s);
    assert_eq!(t.rows[0].cells[0].props.valign, None, "h alone keeps row 0's vertical alignment");
    assert_eq!(t.rows[1].cells[2].props.valign, Some(VAlign::Bottom), "and row 1's");
    for (r, c) in [(0, 0), (0, 2), (1, 1), (1, 2)] {
        assert_eq!(t.rows[r].cells[c].blocks[0].as_para().unwrap().props.align, Some(Align::Center), "({r}, {c})");
    }
    assert_eq!(t.rows[2].cells[0].blocks[0].as_para().unwrap().props.align, None, "row 2 untouched");
    run(&mut s, "table.cellAlign", json!({"v": "center"}));
    let t = table(&s);
    assert_eq!(t.rows[1].cells[1].props.valign, Some(VAlign::Center));
    assert_eq!(t.rows[1].cells[1].blocks[0].as_para().unwrap().props.align, Some(Align::Center), "v alone keeps h");
    assert!(s.run("table.cellAlign", &json!({"h": "middle"})).is_err());
    assert!(s.run("table.cellAlign", &json!({"v": "middle"})).is_err());
    // The nine-way value still sets both.
    run(&mut s, "table.cellAlign", json!({"value": "topRight"}));
    let t = table(&s);
    assert_eq!(
        (t.rows[0].cells[0].props.valign, t.rows[0].cells[0].blocks[0].as_para().unwrap().props.align),
        (Some(VAlign::Top), Some(Align::Right))
    );
}

/// Regression: whole cells ending in an empty cell (whole rows, or cells of one row, are one
/// run of the text) — `para.align` left the last cell out, as the selection ends at the start
/// of its paragraph.
#[test]
fn para_align_reaches_a_trailing_empty_cell() {
    let mut s = table_doc();
    run(&mut s, "text.insert", json!({"text": "A1"}));
    for (sel, label) in [(((0, 0), (0, 2)), "cells of one row"), (((0, 1), (0, 2)), "whole rows"), (((0, 1), (0, 1)), "a block")] {
        let ((r0, r1), (c0, c1)) = sel;
        pick(&mut s, (r0, r1), (c0, c1));
        run(&mut s, "para.align", json!({"value": "center"}));
        let t = table(&s);
        for r in r0..=r1 {
            for c in c0..=c1 {
                assert_eq!(t.rows[r].cells[c].blocks[0].as_para().unwrap().props.align, Some(Align::Center), "{label}: ({r}, {c})");
            }
        }
        assert_eq!(t.rows[2].cells[2].blocks[0].as_para().unwrap().props.align, None, "{label}: outside untouched");
        run(&mut s, "edit.undo", json!({}));
    }
}
