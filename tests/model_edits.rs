use fancy_window::model::*;

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 800.0, 600.0)
}

fn leaves(ids: &[ZoneId]) -> Vec<GridNode> {
    ids.iter().map(|id| GridNode::leaf(*id)).collect()
}

fn split_of(layout: &GridLayout) -> &SplitNode {
    layout.root().as_split().expect("root is a split")
}

fn child_leaf(s: &SplitNode, i: usize) -> ZoneId {
    s.children[i].node.leaf_id().expect("child is a leaf")
}

// split_zone

#[test]
fn split_zone_keeps_original_id_in_first_child() {
    let id = ZoneId::new();
    let result = GridLayout::new(GridNode::leaf(id)).split_zone(id, Orientation::Columns).unwrap();
    assert_eq!(result.leaves().len(), 2);
    assert_eq!(child_leaf(split_of(&result), 0), id);
}

#[test]
fn split_zone_on_nested_leaf_leaves_siblings_and_original_untouched() {
    let layout = GridLayout::equal_columns(3);
    let ids = layout.leaves();
    let result = layout.split_zone(ids[1], Orientation::Rows).unwrap();
    let new_ids = result.leaves();
    assert_eq!(new_ids.len(), 4);
    assert!(ids.iter().all(|id| new_ids.contains(id)));
    assert_eq!(layout.leaves().len(), 3);
}

#[test]
fn split_zone_unknown_id_errors() {
    let unknown = ZoneId::new();
    let err = GridLayout::equal_columns(2).split_zone(unknown, Orientation::Columns).unwrap_err();
    assert_eq!(err, LayoutError::ZoneNotFound(unknown));
}

// remove_zone

#[test]
fn remove_zone_siblings_absorb_space() {
    let layout = GridLayout::equal_columns(3);
    let middle = layout.leaves()[1];
    let result = layout.remove_zone(middle).unwrap();
    let rects = result.zone_rects(Rect::new(0.0, 0.0, 900.0, 600.0));
    assert_eq!(rects.len(), 2);
    assert!(!result.leaves().contains(&middle));
    assert_eq!(rects[0].bounds.width, 450.0);
    assert_eq!(rects[1].bounds.width, 450.0);
}

#[test]
fn remove_zone_from_two_collapses_to_leaf() {
    let (a, b) = (ZoneId::new(), ZoneId::new());
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, leaves(&[a, b])));
    assert_eq!(layout.remove_zone(a).unwrap().root().leaf_id(), Some(b));
}

#[test]
fn remove_zone_cascades_nested_collapse() {
    let (a, b, c) = (ZoneId::new(), ZoneId::new(), ZoneId::new());
    let inner = GridNode::split(Orientation::Rows, leaves(&[a, b]));
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, vec![inner, GridNode::leaf(c)]));
    let result = layout.remove_zone(a).unwrap();
    let s = split_of(&result);
    assert_eq!(s.children.len(), 2);
    assert_eq!(child_leaf(s, 0), b);
    assert_eq!(child_leaf(s, 1), c);
}

#[test]
fn remove_zone_errors() {
    let id = ZoneId::new();
    assert_eq!(GridLayout::new(GridNode::leaf(id)).remove_zone(id).unwrap_err(), LayoutError::LastZone);
    let unknown = ZoneId::new();
    assert_eq!(GridLayout::equal_columns(2).remove_zone(unknown).unwrap_err(), LayoutError::ZoneNotFound(unknown));
}

// remove_splitter

#[test]
fn remove_splitter_on_two_children_collapses_to_left() {
    let (a, b) = (ZoneId::new(), ZoneId::new());
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, leaves(&[a, b])));
    let result = layout.remove_splitter(split_of(&layout).id, 0).unwrap();
    assert_eq!(result.root().leaf_id(), Some(a));
}

#[test]
fn remove_splitter_left_absorbs_right_weight() {
    let (a, b, c) = (ZoneId::new(), ZoneId::new(), ZoneId::new());
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, leaves(&[a, b, c])));
    let id = split_of(&layout).id;
    let layout = layout.set_weight(id, 1, 2.0).unwrap();
    let result = layout.remove_splitter(id, 0).unwrap();
    let s = split_of(&result);
    assert_eq!((child_leaf(s, 0), child_leaf(s, 1)), (a, c));
    assert_eq!((s.children[0].weight, s.children[1].weight), (3.0, 1.0));
}

#[test]
fn remove_splitter_on_nested_keeps_outer() {
    let (a, b, c) = (ZoneId::new(), ZoneId::new(), ZoneId::new());
    let inner = GridNode::split(Orientation::Columns, leaves(&[a, b]));
    let inner_id = inner.as_split().unwrap().id;
    let layout = GridLayout::new(GridNode::split(Orientation::Rows, vec![inner, GridNode::leaf(c)]));
    let result = layout.remove_splitter(inner_id, 0).unwrap();
    let s = split_of(&result);
    assert_eq!((child_leaf(s, 0), child_leaf(s, 1)), (a, c));
}

#[test]
fn remove_splitter_errors() {
    let layout = GridLayout::equal_columns(2);
    let unknown = SplitId::new();
    assert_eq!(layout.remove_splitter(unknown, 0).unwrap_err(), LayoutError::SplitNotFound(unknown));
    assert_eq!(layout.remove_splitter(split_of(&layout).id, 5).unwrap_err(), LayoutError::IndexOutOfRange(5));
}

// set_weight / move_splitter

#[test]
fn set_weight_changes_target_only_and_is_immutable() {
    let layout = GridLayout::equal_columns(2);
    let id = split_of(&layout).id;
    let result = layout.set_weight(id, 0, 3.0).unwrap();
    let rects = result.zone_rects(bounds());
    assert_eq!(rects[0].bounds.width, 600.0);
    assert_eq!(rects[1].bounds.width, 200.0);
    assert_eq!(split_of(&layout).children[0].weight, 1.0);
}

#[test]
fn set_weight_errors() {
    let layout = GridLayout::equal_columns(2);
    assert_eq!(layout.set_weight(split_of(&layout).id, 5, 2.0).unwrap_err(), LayoutError::IndexOutOfRange(5));
    let unknown = SplitId::new();
    assert_eq!(layout.set_weight(unknown, 0, 2.0).unwrap_err(), LayoutError::SplitNotFound(unknown));
}

#[test]
fn move_splitter_grows_left_shrinks_right() {
    let layout = GridLayout::equal_columns(2);
    let rects = layout.move_splitter(split_of(&layout).id, 0, 80.0, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects[0].bounds.width, 480.0);
    assert_eq!(rects[1].bounds.width, 320.0);
}

#[test]
fn move_splitter_on_rows() {
    let layout = GridLayout::equal_rows(2);
    let rects = layout.move_splitter(split_of(&layout).id, 0, 60.0, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects[0].bounds.height, 360.0);
    assert_eq!(rects[1].bounds.height, 240.0);
}

#[test]
fn move_splitter_nested_uses_own_bounds() {
    let inner = GridNode::split(Orientation::Columns, vec![GridNode::new_leaf(), GridNode::new_leaf()]);
    let inner_id = inner.as_split().unwrap().id;
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, vec![inner, GridNode::new_leaf()]));
    let rects = layout.move_splitter(inner_id, 0, 40.0, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects[0].bounds.width, 240.0);
    assert_eq!(rects[1].bounds.width, 160.0);
    assert_eq!(rects[2].bounds.width, 400.0);
}

#[test]
fn move_splitter_clamps_to_minimum_weight() {
    let layout = GridLayout::equal_columns(2);
    let rects = layout.move_splitter(split_of(&layout).id, 0, 100_000.0, bounds()).unwrap().zone_rects(bounds());
    assert!((rects[1].bounds.width - 0.1 / 2.0 * 800.0).abs() < 0.001);
}

#[test]
fn move_splitter_errors() {
    let layout = GridLayout::equal_columns(2);
    assert_eq!(layout.move_splitter(split_of(&layout).id, 1, 50.0, bounds()).unwrap_err(), LayoutError::IndexOutOfRange(1));
    let unknown = SplitId::new();
    assert_eq!(layout.move_splitter(unknown, 0, 50.0, bounds()).unwrap_err(), LayoutError::SplitNotFound(unknown));
}

// add_outer

#[test]
fn add_outer_wraps_leaf_and_keeps_id() {
    let id = ZoneId::new();
    let result = GridLayout::new(GridNode::leaf(id)).add_outer(Orientation::Columns, true);
    assert_eq!(split_of(&result).orientation, Orientation::Columns);
    assert_eq!(result.leaves().len(), 2);
    assert!(result.leaves().contains(&id));
}

#[test]
fn add_outer_appends_or_prepends_to_matching_root() {
    let layout = GridLayout::equal_columns(2);
    let first = layout.leaves()[0];
    assert_eq!(split_of(&layout.add_outer(Orientation::Columns, true)).children.len(), 3);
    let prepended = layout.add_outer(Orientation::Columns, false);
    assert_eq!(child_leaf(split_of(&prepended), 1), first);
    assert_eq!(split_of(&layout).children.len(), 2);
}

#[test]
fn add_outer_wraps_root_with_other_orientation() {
    let result = GridLayout::equal_columns(2).add_outer(Orientation::Rows, true);
    let s = split_of(&result);
    assert_eq!(s.orientation, Orientation::Rows);
    assert!(s.children[0].node.as_split().is_some());
    assert!(s.children[1].node.leaf_id().is_some());
}
