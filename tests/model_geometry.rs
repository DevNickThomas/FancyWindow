use fancy_window::model::*;

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 800.0, 600.0)
}

fn two_columns() -> (SplitId, ZoneId, ZoneId, GridLayout) {
    let (a, b) = (ZoneId::new(), ZoneId::new());
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, vec![GridNode::leaf(a), GridNode::leaf(b)]));
    let id = layout.root().as_split().unwrap().id;
    (id, a, b, layout)
}

#[test]
fn rect_contains_is_half_open() {
    let r = Rect::new(10.0, 20.0, 100.0, 50.0);
    assert!(r.contains(Point::new(50.0, 30.0)));
    assert!(r.contains(Point::new(10.0, 20.0)));
    assert!(!r.contains(Point::new(110.0, 30.0)));
    assert!(!r.contains(Point::new(50.0, 70.0)));
    assert!(!r.contains(Point::new(5.0, 5.0)));
}

#[test]
fn new_ids_are_distinct() {
    assert_ne!(ZoneId::new(), ZoneId::new());
    assert_ne!(SplitId::new(), SplitId::new());
}

#[test]
fn equal_columns_split_bounds_evenly() {
    let rects = GridLayout::equal_columns(3).zone_rects(Rect::new(0.0, 0.0, 900.0, 600.0));
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].bounds, Rect::new(0.0, 0.0, 300.0, 600.0));
    assert_eq!(rects[1].bounds, Rect::new(300.0, 0.0, 300.0, 600.0));
    assert_eq!(rects[2].bounds, Rect::new(600.0, 0.0, 300.0, 600.0));
}

#[test]
fn equal_columns_respect_bounds_offset() {
    let rects = GridLayout::equal_columns(2).zone_rects(Rect::new(100.0, 50.0, 400.0, 200.0));
    assert_eq!(rects[0].bounds, Rect::new(100.0, 50.0, 200.0, 200.0));
    assert_eq!(rects[1].bounds, Rect::new(300.0, 50.0, 200.0, 200.0));
}

#[test]
fn equal_rows_stack_evenly() {
    let rects = GridLayout::equal_rows(2).zone_rects(bounds());
    assert_eq!(rects[0].bounds, Rect::new(0.0, 0.0, 800.0, 300.0));
    assert_eq!(rects[1].bounds, Rect::new(0.0, 300.0, 800.0, 300.0));
}

#[test]
fn uniform_grid_two_by_two_gives_quadrants_in_leaf_order() {
    let layout = GridLayout::uniform_grid(2, 2);
    let rects = layout.zone_rects(bounds());
    assert_eq!(rects.len(), 4);
    assert_eq!(rects[0].bounds, Rect::new(0.0, 0.0, 400.0, 300.0));
    assert_eq!(rects[1].bounds, Rect::new(400.0, 0.0, 400.0, 300.0));
    assert_eq!(rects[2].bounds, Rect::new(0.0, 300.0, 400.0, 300.0));
    assert_eq!(rects[3].bounds, Rect::new(400.0, 300.0, 400.0, 300.0));
    let ids: Vec<ZoneId> = rects.iter().map(|r| r.id).collect();
    assert_eq!(layout.leaves(), ids);
}

#[test]
fn weights_control_share() {
    let mut root = GridNode::split(Orientation::Columns, vec![GridNode::new_leaf(), GridNode::new_leaf()]);
    if let GridNode::Split(s) = &mut root {
        s.children[0].weight = 2.0;
    }
    let rects = root.compute_rects(Rect::new(0.0, 0.0, 900.0, 600.0));
    assert_eq!(rects[0].bounds, Rect::new(0.0, 0.0, 600.0, 600.0));
    assert_eq!(rects[1].bounds, Rect::new(600.0, 0.0, 300.0, 600.0));
}

#[test]
fn hit_test_finds_zone_and_favours_later_on_boundary() {
    let layout = GridLayout::equal_columns(3);
    let b = Rect::new(0.0, 0.0, 900.0, 600.0);
    assert_eq!(layout.hit_test(b, Point::new(450.0, 300.0)), Some(layout.leaves()[1]));
    assert_eq!(layout.hit_test(b, Point::new(-50.0, -50.0)), None);
    assert_eq!(layout.hit_test(b, Point::new(300.0, 10.0)), Some(layout.leaves()[1]));
}

#[test]
fn single_leaf_has_no_splitters() {
    assert!(GridLayout::new(GridNode::new_leaf()).splitters(bounds()).is_empty());
}

#[test]
fn two_columns_have_one_vertical_splitter_at_midline() {
    let (id, _, _, layout) = two_columns();
    let s = layout.splitters(bounds());
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].split_id, id);
    assert_eq!(s[0].left_child_index, 0);
    assert_eq!(s[0].orientation, Orientation::Columns);
    assert_eq!(s[0].bounds, Rect::new(400.0 - SPLITTER_THICKNESS / 2.0, 0.0, SPLITTER_THICKNESS, 600.0));
}

#[test]
fn three_columns_have_two_splitters() {
    let s = GridLayout::equal_columns(3).splitters(Rect::new(0.0, 0.0, 900.0, 600.0));
    assert_eq!(s.len(), 2);
    assert_eq!(s[1].left_child_index, 1);
    assert_eq!(s[1].bounds, Rect::new(600.0 - SPLITTER_THICKNESS / 2.0, 0.0, SPLITTER_THICKNESS, 600.0));
}

#[test]
fn two_rows_have_one_horizontal_splitter() {
    let s = GridLayout::equal_rows(2).splitters(bounds());
    assert_eq!(s[0].orientation, Orientation::Rows);
    assert_eq!(s[0].bounds, Rect::new(0.0, 300.0 - SPLITTER_THICKNESS / 2.0, 800.0, SPLITTER_THICKNESS));
}

#[test]
fn two_by_two_has_one_outer_and_two_inner_splitters() {
    let s = GridLayout::uniform_grid(2, 2).splitters(bounds());
    assert_eq!(s.len(), 3);
    assert_eq!(s.iter().filter(|h| h.orientation == Orientation::Rows).count(), 1);
}
