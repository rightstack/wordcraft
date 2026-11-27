use super::*;
use wordcraft_doc::graphic::PathSeg;

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
            assert_eq!(spec_of_xml(&data_xml(&spec, "rId9")).as_ref(), Some(&spec), "{layout:?} from bytes");
            let items = smart_art_items(&spec, &THEME_COLORS, 400.0, 250.0);
            assert!(items.iter().any(|i| matches!(i, GraphicItem::Text { .. })), "{layout:?} has text");
            for it in &items {
                if let GraphicItem::Shape { rect, .. } = it {
                    assert!(rect[0] >= -0.5 && rect[1] >= -0.5 && rect[0] + rect[2] <= 400.5 && rect[1] + rect[3] <= 250.5, "{layout:?}: {rect:?}");
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
            // Trees, radials and groups draw every item as its own shape; the others draw sub-items as lines.
            let own = it.level == 0 || layout.children_are_shapes();
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
    let mut bad = Vec::new();
    for layout in SmartArtLayout::ALL {
        for n in [1usize, 3, 7] {
            let mut spec = SmartArtSpec::sample(layout);
            spec.items = (0..n).flat_map(|k| [SmartArtItem::new(format!("Item {k}"), 0), SmartArtItem::new("line", 1)]).collect();
            let spec = spec.sanitized();
            for (w, h) in [(432.0, 252.0), (120.0, 300.0), (600.0, 80.0)] {
                for sh in super::layout(&spec, w, h) {
                    let [x, y, bw, bh] = sh.drawn.unwrap_or(sh.b);
                    if !(x >= -0.5 && y >= -0.5 && x + bw <= w + 0.5 && y + bh <= h + 0.5) {
                        bad.push(format!("{layout:?} {n} {w}x{h}: {:?} {:?}", sh.prst, sh.b));
                    }
                    if !sh.text.is_empty()
                        && let Paint::Accent(_, Some(t)) = sh.fill
                        && t < 90_000
                        && sh.ink == Ink::Light
                    {
                        bad.push(format!("{layout:?}: light text on a light fill"));
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

/// A light theme whose first accent is a pale yellow (white text on it reads 1.5:1), and a
/// theme of mid-strength accents.
const YELLOW_THEME: [Rgb; 12] = [
    Rgb(0x17, 0x13, 0x1A),
    Rgb(0xFF, 0xFF, 0xFF),
    Rgb(0x3A, 0x32, 0x3D),
    Rgb(0xFD, 0xF4, 0xEA),
    Rgb(0xFF, 0xD0, 0x2F),
    Rgb(0x5A, 0xA5, 0xFF),
    Rgb(0xF9, 0x7A, 0x3A),
    Rgb(0x62, 0xD2, 0x73),
    Rgb(0x8E, 0x63, 0xE8),
    Rgb(0xFF, 0xA8, 0xCC),
    Rgb(0x00, 0x62, 0xD8),
    Rgb(0x8E, 0x21, 0xFA),
];
const BLUE_THEME: [Rgb; 12] = [
    Rgb(0, 0, 0),
    Rgb(0xFF, 0xFF, 0xFF),
    Rgb(0x44, 0x54, 0x6A),
    Rgb(0xE7, 0xE6, 0xE6),
    Rgb(0x44, 0x72, 0xC4),
    Rgb(0xED, 0x7D, 0x31),
    Rgb(0xA5, 0xA5, 0xA5),
    Rgb(0xFF, 0xC0, 0x00),
    Rgb(0x5B, 0x9B, 0xD5),
    Rgb(0x70, 0xAD, 0x47),
    Rgb(0x05, 0x63, 0xC1),
    Rgb(0x95, 0x4F, 0x72),
];

/// The box of a drawn shape or path, and its fill.
fn filled_box(it: &GraphicItem) -> Option<([f32; 4], Rgb)> {
    match it {
        GraphicItem::Shape { rect, fill: Some(f), .. } => Some((*rect, *f)),
        GraphicItem::Path { segs, fill: Some(f), .. } => {
            let pts: Vec<(f32, f32)> = segs
                .iter()
                .filter_map(|s| match *s {
                    PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(_, _, _, _, x, y) => Some((x, y)),
                    PathSeg::Close => None,
                })
                .collect();
            let (x0, y0) = pts.iter().fold((f32::MAX, f32::MAX), |(a, b), p| (a.min(p.0), b.min(p.1)));
            let (x1, y1) = pts.iter().fold((f32::MIN, f32::MIN), |(a, b), p| (a.max(p.0), b.max(p.1)));
            Some(([x0, y0, x1 - x0, y1 - y0], *f))
        }
        _ => None,
    }
}

#[test]
fn every_layouts_text_reads_on_what_is_behind_it_in_any_theme() {
    // Drawn as the app draws it: each text line against the topmost filled shape drawn before
    // it under the line's middle, else the white page.
    let mut bad = Vec::new();
    for theme in [&YELLOW_THEME[..], &BLUE_THEME[..], &THEME_COLORS[..]] {
        for layout in SmartArtLayout::ALL {
            for colors in SmartArtColors::ALL {
                for n in [3usize, 6] {
                    let mut spec = SmartArtSpec::sample(layout);
                    spec.colors = colors;
                    spec.items = (0..n).flat_map(|k| [SmartArtItem::new(format!("Item {k}"), 0), SmartArtItem::new("line", 1)]).collect();
                    let spec = spec.sanitized();
                    let items = smart_art_items(&spec, theme, 432.0, 252.0);
                    for (k, it) in items.iter().enumerate() {
                        let GraphicItem::Text { rect, color, text, .. } = it else { continue };
                        let (cx, cy) = (rect[0] + rect[2] / 2.0, rect[1] + rect[3] / 2.0);
                        let bg = items[..k]
                            .iter()
                            .rev()
                            .filter_map(filled_box)
                            .find(|(b, _)| b[0] <= cx && cx <= b[0] + b[2] && b[1] <= cy && cy <= b[1] + b[3])
                            .map_or(Rgb::WHITE, |(_, f)| f);
                        let c = contrast(*color, bg);
                        if c < TEXT_CONTRAST {
                            bad.push(format!("{layout:?} {colors:?} n={n} theme accent1 {:?}: {text:?} {color:?} on {bg:?} = {c:.2}", theme[4]));
                        }
                    }
                }
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn accent_lines_stand_out_from_the_page() {
    for theme in [&YELLOW_THEME[..], &BLUE_THEME[..]] {
        for layout in SmartArtLayout::ALL {
            for colors in SmartArtColors::ALL {
                let mut spec = SmartArtSpec::sample(layout);
                spec.colors = colors;
                let mut shapes = super::layout(&spec.sanitized(), 432.0, 252.0);
                fit_colours(&mut shapes, theme);
                for (k, sh) in shapes.iter().enumerate() {
                    if sh.prst == "line" && sh.line != Paint::None {
                        let c = paint_rgb(sh.line, theme).unwrap();
                        let bg = backdrop(&shapes, k, theme);
                        assert!(contrast(c, bg) >= LINE_CONTRAST, "{layout:?} {colors:?}: {:?} {c:?} on {bg:?}", sh.line);
                    }
                }
            }
        }
    }
}

#[test]
fn a_pale_accent_gets_dark_text_and_a_strong_one_light_text_in_the_saved_drawing() {
    let spec = SmartArtSpec { colors: SmartArtColors::Accent1, ..SmartArtSpec::sample(SmartArtLayout::Process) }.sanitized();
    // The first step is the full accent: dark text on pale yellow, light text on mid blue.
    let first_run = |theme: &[Rgb]| {
        let d = parse(&drawing_xml(&spec, theme, 400.0, 200.0));
        let sp = d.child("dsp:spTree").unwrap().children("dsp:sp").find(|sp| sp.find("a:t").is_some()).unwrap();
        sp.find("a:rPr").unwrap().find("a:schemeClr").unwrap().attr("val").unwrap().to_string()
    };
    assert_eq!(first_run(&YELLOW_THEME), "tx1");
    assert_eq!(first_run(&BLUE_THEME), "lt1");
    // Accent numbers on white in a pale accent are written darker (lumMod).
    let spec = SmartArtSpec { colors: SmartArtColors::Accent1, ..SmartArtSpec::sample(SmartArtLayout::CardBigNumbers) }.sanitized();
    let x = drawing_xml(&spec, &YELLOW_THEME, 400.0, 200.0);
    assert!(x.contains("<a:lumMod val="), "{x}");
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
    let drawing = parse(&drawing_xml(&spec, &THEME_COLORS, 300.0, 200.0));
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
    let spec = SmartArtSpec { items: (0..MAX_ITEMS).map(|i| SmartArtItem::new("word ".repeat(50), (i % 9) as u8)).collect(), ..Default::default() }
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

fn tops_spec(layout: SmartArtLayout, n: usize) -> SmartArtSpec {
    SmartArtSpec { layout, items: (0..n).map(|k| SmartArtItem::new(format!("Item {k}"), 0)).collect(), ..Default::default() }.sanitized()
}

fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    a[0] < b[0] + b[2] - 0.01 && b[0] < a[0] + a[2] - 0.01 && a[1] < b[1] + b[3] - 0.01 && b[1] < a[1] + a[3] - 0.01
}

#[test]
fn wedges_go_round_the_circle_in_turn_and_are_picked_by_their_own_text() {
    for layout in [SmartArtLayout::MatrixCycle, SmartArtLayout::CycleSegmented, SmartArtLayout::CyclePie] {
        for n in 2..=7 {
            let spec = tops_spec(layout, n);
            let shapes = super::layout(&spec, 432.0, 252.0);
            let wedges: Vec<&Sh> = shapes.iter().filter(|s| s.prst == "pie").collect();
            assert_eq!(wedges.len(), n, "{layout:?} {n}");
            let adj = |s: &Sh, k: &str| s.adj.iter().find(|(name, _)| *name == k).map(|(_, v)| *v).unwrap();
            // Each wedge starts where the one before ends, and together they make a full turn.
            let mut sweep = 0;
            for (k, w) in wedges.iter().enumerate() {
                let (a, b) = (adj(w, "adj1"), adj(w, "adj2"));
                sweep += (b - a).rem_euclid(21_600_000);
                if k > 0 {
                    assert_eq!(adj(wedges[k - 1], "adj2"), a, "{layout:?} {n}: wedge {k}");
                }
                assert_eq!(w.b, wedges[0].b, "{layout:?}: every wedge shares the circle's box");
            }
            assert!((sweep - 21_600_000).abs() <= n as i64, "{layout:?} {n}: {sweep}");
            // Picking: each item's node is its text box inside the circle, none overlapping.
            let nodes = smart_art_nodes(&spec, 432.0, 252.0);
            let c = wedges[0].b;
            for (k, a) in nodes.iter().enumerate() {
                assert!(
                    a.b[0] >= c[0] && a.b[1] >= c[1] && a.b[0] + a.b[2] <= c[0] + c[2] && a.b[1] + a.b[3] <= c[1] + c[3],
                    "{layout:?} {n}: {a:?}"
                );
                for b in &nodes[k + 1..] {
                    assert!(!overlap(a.b, b.b), "{layout:?} {n}: {a:?} {b:?}");
                }
            }
        }
    }
}

#[test]
fn gears_mesh_without_their_bodies_overlapping() {
    for n in 1..=6 {
        let spec = tops_spec(SmartArtLayout::Gears, n);
        let shapes = super::layout(&spec, 432.0, 252.0);
        let bodies: Vec<&Sh> = shapes.iter().filter(|s| s.prst == "ellipse").collect();
        assert_eq!(bodies.len(), n);
        assert!(bodies.iter().enumerate().all(|(k, b)| b.item == Some(k)), "each gear is its item");
        // Ten teeth on the large gears, eight on the small, turned to point outwards.
        assert_eq!(shapes.iter().filter(|s| s.prst == "rect").count(), (0..n).map(|k| if k % 2 == 0 { 10 } else { 8 }).sum::<usize>());
        let centre = |s: &Sh| (s.b[0] + s.b[2] / 2.0, s.b[1] + s.b[3] / 2.0, s.b[2] / 2.0);
        for k in 1..n {
            let ((x0, y0, r0), (x1, y1, r1)) = (centre(bodies[k - 1]), centre(bodies[k]));
            let d = (x1 - x0).hypot(y1 - y0);
            assert!(d > r0 + r1, "gears {k}: bodies overlap");
            // Close enough for the teeth (out to 1/0.82 of the body) to mesh.
            assert!(d < (r0 + r1) / 0.82, "gears {k}: teeth don't meet");
        }
    }
}

#[test]
fn a_funnel_takes_the_items_in_at_the_top_and_gives_the_last_out_below() {
    let spec = tops_spec(SmartArtLayout::Funnel, 4);
    let shapes = super::layout(&spec, 432.0, 252.0);
    let funnel = shapes.iter().find(|s| s.prst == "trapezoid").unwrap();
    assert_eq!(funnel.rot, 180.0, "wide side up");
    for s in shapes.iter().filter(|s| s.item.is_some()) {
        if s.item == Some(3) {
            assert!(s.b[1] >= funnel.b[1] + funnel.b[3], "the result is below the funnel");
            assert_eq!(s.prst, "roundRect");
        } else {
            assert!(s.b[1] + s.b[3] <= funnel.b[1] + 0.01, "the inputs are above it");
            assert_eq!(s.prst, "ellipse");
        }
    }
}

#[test]
fn target_rings_are_picked_by_their_own_band() {
    for n in 1..=6 {
        let spec = tops_spec(SmartArtLayout::Target, n);
        let nodes = smart_art_nodes(&spec, 432.0, 252.0);
        assert_eq!(nodes.len(), n);
        for (k, a) in nodes.iter().enumerate() {
            for b in &nodes[k + 1..] {
                assert!(!overlap(a.b, b.b), "{n}: {a:?} {b:?}");
            }
            let (cx, cy) = (a.b[0] + a.b[2] / 2.0, a.b[1] + a.b[3] / 2.0);
            assert_eq!(smart_art_item_at(&spec, 432.0, 252.0, cx, cy), Some(a.item));
        }
    }
}

#[test]
fn later_layouts_carry_our_identifier_category_and_walk() {
    for l in SmartArtLayout::ALL.into_iter().skip(8) {
        let (id, cat) = layout_id(l);
        assert_eq!(id, format!("urn:lilyoffice.com/smartart/2026/layout/{}", l.id()));
        let x = layout_xml(l);
        assert!(x.contains(&format!("<dgm:cat type=\"{cat}\"")), "{l:?}");
        if l.category() == "Picture" {
            assert_eq!(cat, "picture");
        }
        // Trees walk the outline as a tree; a centre with items round it names its centre.
        let tree = x.contains("hierRoot");
        let centre = x.contains("ctrShpMap");
        assert_eq!(tree || centre, l.children_are_shapes(), "{l:?}");
    }
}

// ---- fourth batch ----

#[test]
fn a_labeled_hierarchy_names_its_bands_with_the_later_childless_top_items() {
    for layout in [SmartArtLayout::HierarchyLabeled, SmartArtLayout::HierarchyLabeledHorizontal] {
        let spec = SmartArtSpec::sample(layout).sanitized();
        // Lead … Tests make the tree; Leadership, Teams, Work (items 6–8) name the three levels.
        let shapes = super::layout(&spec, 432.0, 252.0);
        let bands: Vec<&Sh> = shapes.iter().filter(|s| s.prst == "rect" && s.item.is_some()).collect();
        assert_eq!(bands.iter().map(|s| s.item).collect::<Vec<_>>(), [Some(6), Some(7), Some(8)], "{layout:?}");
        for (k, b) in bands.iter().enumerate() {
            let pick = b.pick.unwrap();
            // Each band's name sits at its band's start (left, or top when the tree runs across).
            if layout == SmartArtLayout::HierarchyLabeled {
                assert!(pick[0] < 432.0 * 0.05 && (pick[1] - b.b[1]).abs() < 2.0, "{layout:?} band {k}: {pick:?}");
            } else {
                assert!(pick[1] < 252.0 * 0.05 && (pick[0] - b.b[0]).abs() < 2.0, "{layout:?} band {k}: {pick:?}");
            }
        }
        // Every tree item is a node over its level's band.
        let nodes = smart_art_nodes(&spec, 432.0, 252.0);
        for i in 0..6 {
            let n = nodes.iter().find(|n| n.item == i).unwrap();
            let level = usize::from(spec.items[i].level);
            let band = bands[level].b;
            let (cx, cy) = (n.b[0] + n.b[2] / 2.0, n.b[1] + n.b[3] / 2.0);
            assert!(band[0] <= cx && cx <= band[0] + band[2] && band[1] <= cy && cy <= band[1] + band[3], "{layout:?} item {i}");
        }
    }
    // With no names (every top item has items under it) the bands are plain.
    let spec = SmartArtSpec { layout: SmartArtLayout::HierarchyLabeled, ..SmartArtSpec::sample(SmartArtLayout::OrgChart) }.sanitized();
    assert!(super::layout(&spec, 432.0, 252.0).iter().filter(|s| s.prst == "rect").all(|s| s.item.is_none()));
}

/// The points of the drawn paths of `spec` laid out `w` × `h`, with each path's fill.
fn paths(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<(Vec<(f32, f32)>, Option<Rgb>)> {
    smart_art_items(spec, &THEME_COLORS, w, h)
        .into_iter()
        .filter_map(|it| match it {
            GraphicItem::Path { segs, fill, .. } => Some((
                segs.iter()
                    .filter_map(|s| match *s {
                        PathSeg::Move(x, y) | PathSeg::Line(x, y) | PathSeg::Cubic(_, _, _, _, x, y) => Some((x, y)),
                        PathSeg::Close => None,
                    })
                    .collect(),
                fill,
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn arrows_point_their_own_way_and_the_reader_follows_their_shaft_and_head() {
    let spec = SmartArtSpec::sample(SmartArtLayout::ArrowsConverging).sanitized();
    let arrows: Vec<&'static str> = super::layout(&spec, 432.0, 252.0).iter().filter(|s| s.prst.ends_with("Arrow")).map(|s| s.prst).collect();
    // Skills on the left points right (in), Needs on the right points left (in).
    assert_eq!(arrows, ["rightArrow", "leftArrow"]);
    let spec = SmartArtSpec { layout: SmartArtLayout::ArrowsDiverging, ..spec };
    let arrows: Vec<&'static str> = super::layout(&spec, 432.0, 252.0).iter().filter(|s| s.prst.ends_with("Arrow")).map(|s| s.prst).collect();
    assert_eq!(arrows, ["leftArrow", "rightArrow"]);
    // No arrow is a turned-round left arrow any more (its text would be turned in Word).
    for layout in SmartArtLayout::ALL {
        let spec = SmartArtSpec::sample(layout).sanitized();
        assert!(super::layout(&spec, 432.0, 252.0).iter().all(|s| !(s.prst == "leftArrow" && s.rot == 180.0)), "{layout:?}");
    }
    // The continuous block process: its band is a right arrow whose shaft is 70% of its height
    // and whose head is 40% of its height long, as the gallery shows (not the reader's default).
    let spec = SmartArtSpec::sample(SmartArtLayout::ProcessContinuous).sanitized();
    let band = super::layout(&spec, 432.0, 252.0).into_iter().next().unwrap();
    assert_eq!(band.prst, "rightArrow");
    let (pts, _) = paths(&spec, 432.0, 252.0).into_iter().next().expect("the band is drawn as its outline");
    let ys: Vec<f32> = pts.iter().filter(|p| p.0 < 1.0).map(|p| p.1).collect();
    let shaft = ys.iter().fold(f32::MIN, |a, &b| a.max(b)) - ys.iter().fold(f32::MAX, |a, &b| a.min(b));
    assert!((shaft / band.b[3] - 0.7).abs() < 0.02, "shaft {shaft} of {}", band.b[3]);
    let neck = pts.iter().filter(|p| (p.1 - band.b[1]).abs() < 0.5).map(|p| p.0).fold(f32::MAX, f32::min);
    assert!(((band.b[2] - neck) / band.b[3] - 0.4).abs() < 0.02, "head from {neck}");
}

#[test]
fn pills_are_drawn_with_round_ends() {
    // A team pill is a `roundRect` with its full radius (adj 50000): the reader draws it from its
    // outline, so its top is a half circle (the middle of the top edge only touches the box).
    let spec = SmartArtSpec::sample(SmartArtLayout::TeamPills).sanitized();
    let pill = super::layout(&spec, 432.0, 252.0).into_iter().find(|s| s.prst == "roundRect").unwrap();
    assert_eq!(pill.adj, [("adj", 50_000)]);
    let (pts, _) = paths(&spec, 432.0, 252.0).into_iter().next().expect("the pill is drawn as its outline");
    let [x, y, w, _] = pill.b;
    let r = w / 2.0;
    for (px, py) in pts.iter().filter(|p| p.1 < y + r - 0.5) {
        let d = ((px - (x + r)).powi(2) + (py - (y + r)).powi(2)).sqrt();
        assert!((d - r).abs() < 0.5, "({px}, {py}) on the round top, {d} from its centre");
    }
}

#[test]
fn the_target_list_is_a_quarter_target_in_the_lower_left_corner() {
    for n in 1..=6 {
        let spec = tops_spec(SmartArtLayout::ListTarget, n);
        let shapes = super::layout(&spec, 432.0, 252.0);
        let pies: Vec<&Sh> = shapes.iter().filter(|s| s.prst == "pie").collect();
        assert_eq!(pies.len(), n, "{n}");
        let corner = (pies[0].b[0] + pies[0].b[2] / 2.0, pies[0].b[1] + pies[0].b[3] / 2.0);
        for (k, p) in pies.iter().enumerate() {
            // Upper right quarter of a circle centred on the same corner, smaller each time.
            assert_eq!(p.adj, [("adj1", 16_200_000), ("adj2", 0)]);
            assert!(((p.b[0] + p.b[2] / 2.0) - corner.0).abs() < 0.01 && ((p.b[1] + p.b[3] / 2.0) - corner.1).abs() < 0.01);
            assert!(corner.0 < 432.0 * 0.05 && corner.1 > 252.0 * 0.95, "{corner:?}");
            if k > 0 {
                assert!(p.b[2] < pies[k - 1].b[2]);
            }
            let d = p.drawn.unwrap();
            let want = [corner.0, corner.1 - p.b[2] / 2.0, p.b[2] / 2.0, p.b[2] / 2.0];
            assert!(d.iter().zip(want).all(|(a, b)| (a - b).abs() < 0.01), "{d:?} {want:?}");
        }
        // A line from each ring to its text, the texts to the right of the target.
        assert_eq!(shapes.iter().filter(|s| s.prst == "line").count(), n);
        let right = pies[0].b[0] + pies[0].b[2];
        assert!(shapes.iter().filter(|s| !s.text.is_empty()).all(|s| s.tb[0] >= right), "{n}");
    }
    // The drawn wedges stay in the frame.
    let spec = tops_spec(SmartArtLayout::ListTarget, 4);
    // …and the drawing is not shrunk for the circles' boxes reaching past it: the outer wedge
    // is drawn where the layout put it.
    let top = paths(&spec, 432.0, 252.0).first().map(|(pts, _)| pts.iter().map(|p| p.1).fold(f32::MAX, f32::min)).unwrap();
    let want = super::layout(&spec, 432.0, 252.0).iter().find_map(|s| s.drawn).unwrap()[1];
    assert!((top - want).abs() < 0.5, "{top} {want}");
    for (pts, _) in paths(&spec, 432.0, 252.0) {
        assert!(pts.iter().all(|p| p.0 >= -0.5 && p.1 >= -0.5 && p.0 <= 432.5 && p.1 <= 252.5), "{pts:?}");
    }
}

/// The Korean text-card samples the app's gallery inserts (`smartart-samples.ts`), whose sub-lines
/// used to break in two.
fn ko_cards() -> SmartArtSpec {
    let items = [("아이디어", 0), ("왜 중요한가요", 1), ("계획", 0), ("어떻게 해 나갈까요", 1), ("결과", 0), ("무엇이 달라지나요", 1)];
    SmartArtSpec { items: items.iter().map(|(t, l)| SmartArtItem::new(*t, *l)).collect(), ..Default::default() }
}

/// The text lines of `spec` drawn `w` × `h`: (text, size).
fn drawn_lines(spec: &SmartArtSpec, w: f32, h: f32) -> Vec<(String, f32)> {
    smart_art_items(spec, &THEME_COLORS, w, h)
        .into_iter()
        .filter_map(|it| match it {
            GraphicItem::Text { text, size, .. } => Some((text, size)),
            _ => None,
        })
        .collect()
}

#[test]
fn text_is_capped_sub_lines_are_smaller_and_no_line_breaks_inside_an_eojeol() {
    let mut bad = Vec::new();
    // (Layouts whose sub-items are shapes of their own lay these samples out differently.)
    for layout in SmartArtLayout::ALL.into_iter().filter(|l| !l.children_are_shapes()) {
        let mut spec = ko_cards();
        spec.layout = layout;
        let spec = spec.sanitized();
        let shapes = super::layout(&spec, 432.0, 252.0);
        let sz = text_sizes(&shapes, 432.0, 252.0);
        if sz.item > MAX_SIZE || sz.child >= sz.item || sz.child > sz.item * CHILD + 0.25 {
            bad.push(format!("{layout:?}: {sz:?}"));
        }
        // Every drawn line is whole eojeols: each word of a line is a word of the item's text.
        let words: Vec<&str> = spec.items.iter().flat_map(|it| it.text.split_whitespace()).collect();
        for (line, size) in drawn_lines(&spec, 432.0, 252.0) {
            // (Numbers and bullets the layout adds are not the item's words.)
            let hangul = |w: &str| w.chars().any(|c| ('\u{AC00}'..='\u{D7A3}').contains(&c));
            if line.split_whitespace().any(|w| hangul(w) && !words.contains(&w)) {
                bad.push(format!("{layout:?}: {line:?} breaks a word"));
            }
            if size > MAX_SIZE * 3.0 {
                bad.push(format!("{layout:?}: {line:?} at {size}"));
            }
        }
    }
    assert!(bad.is_empty(), "{}", bad.join("\n"));
}

#[test]
fn the_header_cards_set_their_sub_lines_on_one_line_in_a_smaller_size() {
    // 영일: "기본 값이 이렇게 줄바꿈이 되면 못 생겼잖아? 밑에는 글꼴이 더 작아야 예쁠텐데."
    let mut spec = ko_cards();
    spec.layout = SmartArtLayout::CardHeader;
    let spec = spec.sanitized();
    let lines = drawn_lines(&spec, 432.0, 252.0);
    let texts: Vec<&str> = lines.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(texts, ["왜 중요한가요", "아이디어", "어떻게 해 나갈까요", "계획", "무엇이 달라지나요", "결과"]);
    let size = |t: &str| lines.iter().find(|(l, _)| l == t).map(|l| l.1).unwrap();
    assert!(size("아이디어") <= MAX_SIZE);
    assert!(size("어떻게 해 나갈까요") <= size("아이디어") * CHILD + 0.25, "{lines:?}");
    // The text sits inside its band with a margin: padded text boxes.
    let shapes = super::layout(&spec, 432.0, 252.0);
    for s in shapes.iter().filter(|s| !s.text.is_empty() && s.fill != Paint::None) {
        assert!(s.tb[0] > s.b[0] + 1.9 && s.tb[0] + s.tb[2] < s.b[0] + s.b[2] - 1.9, "{s:?}");
    }
}

#[test]
fn a_small_graphic_gets_smaller_text_and_sizes_stay_in_range() {
    let spec = SmartArtSpec::sample(SmartArtLayout::BasicList).sanitized();
    let big = text_sizes(&super::layout(&spec, 432.0, 252.0), 432.0, 252.0);
    let small = text_sizes(&super::layout(&spec, 250.0, 140.0), 250.0, 140.0);
    assert!(small.item < big.item && big.item <= MAX_SIZE, "{small:?} {big:?}");
    for (w, h) in [(f32::NAN, 10.0), (0.0, 0.0), (1.0e30, 5.0), (5.0, 1.0e30)] {
        let sz = text_sizes(&super::layout(&spec, w, h), w, h);
        assert!((MIN_SIZE..=MAX_SIZE).contains(&sz.item) && sz.child >= MIN_SIZE && sz.child <= sz.item, "{w} {h}: {sz:?}");
    }
}

#[test]
fn winding_processes_turn_back_at_each_line_end() {
    let centres = |layout: SmartArtLayout| -> Vec<(f32, f32)> {
        let spec = tops_spec(layout, 6);
        super::layout(&spec, 432.0, 252.0).iter().filter(|s| s.prst == "roundRect").map(|s| (s.b[0] + s.b[2] / 2.0, s.b[1] + s.b[3] / 2.0)).collect()
    };
    // Snaking: three a row, the second row right to left, the turn straight down.
    let c = centres(SmartArtLayout::ProcessSnake);
    assert_eq!(c.len(), 6);
    assert!(c[0].0 < c[1].0 && c[1].0 < c[2].0 && c[3].0 > c[4].0 && c[4].0 > c[5].0, "{c:?}");
    assert!((c[2].0 - c[3].0).abs() < 0.5 && c[3].1 > c[2].1);
    // Vertical bending: down the first column, up the second.
    let c = centres(SmartArtLayout::ProcessBendingVertical);
    let per = c.iter().take_while(|p| (p.0 - c[0].0).abs() < 0.5).count();
    assert!((2..6).contains(&per), "{c:?}");
    assert!(c[per].1 > c[per + 1].1 || per + 1 >= c.len(), "the second column goes up: {c:?}");
    assert!((c[per - 1].1 - c[per].1).abs() < 0.5, "the turn is straight across");
}

#[test]
fn a_circle_accent_timeline_gives_every_item_a_circle_on_one_line() {
    let spec = SmartArtSpec::sample(SmartArtLayout::TimelineCircleAccent).sanitized();
    let shapes = super::layout(&spec, 432.0, 252.0);
    let circles: Vec<&Sh> = shapes.iter().filter(|s| s.prst == "ellipse" && s.item.is_some()).collect();
    assert_eq!(circles.iter().map(|s| s.item.unwrap()).collect::<Vec<_>>(), (0..spec.items.len()).collect::<Vec<_>>());
    let mid = |s: &Sh| s.b[1] + s.b[3] / 2.0;
    assert!(circles.iter().all(|s| (mid(s) - mid(circles[0])).abs() < 0.5), "all on the line");
    for (k, s) in circles.iter().enumerate() {
        let big = spec.items[k].level == 0;
        assert_eq!(s.b[2] > circles[1].b[2] * 1.5, big, "top items are the large circles");
        if k > 0 {
            assert!(s.b[0] >= circles[k - 1].b[0] + circles[k - 1].b[2] - 0.01, "in order, apart");
        }
    }
}

#[test]
fn a_stacked_venn_and_a_nested_target_pick_each_item_by_its_own_band() {
    for layout in [SmartArtLayout::VennStacked, SmartArtLayout::TargetNested, SmartArtLayout::ListTarget] {
        for n in 1..=6 {
            let spec = tops_spec(layout, n);
            let nodes = smart_art_nodes(&spec, 432.0, 252.0);
            assert_eq!(nodes.len(), n, "{layout:?} {n}");
            for (k, a) in nodes.iter().enumerate() {
                for b in &nodes[k + 1..] {
                    assert!(!overlap(a.b, b.b), "{layout:?} {n}: {a:?} {b:?}");
                }
                let (cx, cy) = (a.b[0] + a.b[2] / 2.0, a.b[1] + a.b[3] / 2.0);
                assert_eq!(smart_art_item_at(&spec, 432.0, 252.0, cx, cy), Some(a.item), "{layout:?} {n}");
            }
        }
    }
}
