use fancy_window::model::*;

fn bounds() -> Rect {
    Rect::new(0.0, 0.0, 800.0, 600.0)
}

/// Rows[Cols[a,b], Cols[c,d]]
fn two_by_two() -> (GridLayout, [ZoneId; 4]) {
    let ids = [ZoneId::new(), ZoneId::new(), ZoneId::new(), ZoneId::new()];
    let row = |l: ZoneId, r: ZoneId| GridNode::split(Orientation::Columns, vec![GridNode::leaf(l), GridNode::leaf(r)]);
    let layout = GridLayout::new(GridNode::split(Orientation::Rows, vec![row(ids[0], ids[1]), row(ids[2], ids[3])]));
    (layout, ids)
}

fn rect_of(rects: &[ZoneRect], id: ZoneId) -> Rect {
    rects.iter().find(|r| r.id == id).expect("zone present").bounds
}

#[test]
fn join_up_in_two_by_two_makes_tall_right_column() {
    let (layout, [a, b, c, d]) = two_by_two();
    let rects = layout.join(d, JoinDirection::Up, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects.len(), 3);
    assert_eq!(rect_of(&rects, d), Rect::new(400.0, 0.0, 400.0, 600.0));
    assert!(rects.iter().all(|r| r.id != b));
    assert_eq!(rect_of(&rects, a).y, 0.0);
    assert_eq!(rect_of(&rects, c).y, 300.0);
}

#[test]
fn join_right_in_two_by_two_makes_wide_top_row() {
    let (layout, [a, ..]) = two_by_two();
    let rects = layout.join(a, JoinDirection::Right, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects.len(), 3);
    assert_eq!(rect_of(&rects, a), Rect::new(0.0, 0.0, 800.0, 300.0));
}

#[test]
fn join_down_and_left_work() {
    let (layout, [a, _, c, d]) = two_by_two();
    let down = layout.join(a, JoinDirection::Down, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rect_of(&down, a), Rect::new(0.0, 0.0, 400.0, 600.0));
    assert!(down.iter().all(|r| r.id != c));
    let left = layout.join(d, JoinDirection::Left, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rect_of(&left, d), Rect::new(0.0, 300.0, 800.0, 300.0));
}

#[test]
fn join_two_siblings_collapses_to_clicked_leaf() {
    let layout = GridLayout::equal_columns(2);
    let a = layout.leaves()[0];
    assert_eq!(layout.join(a, JoinDirection::Right, bounds()).unwrap().root().leaf_id(), Some(a));
}

#[test]
fn can_join_only_with_full_edge_neighbor() {
    let layout = GridLayout::equal_columns(2);
    let [a, b] = [layout.leaves()[0], layout.leaves()[1]];
    assert!(layout.can_join(a, JoinDirection::Right, bounds()));
    assert!(layout.can_join(b, JoinDirection::Left, bounds()));
    assert!(!layout.can_join(a, JoinDirection::Left, bounds()));
    assert!(!layout.can_join(a, JoinDirection::Up, bounds()));
    assert!(!layout.can_join(b, JoinDirection::Right, bounds()));

    // Cols[a, Rows[b, c]]: neither b nor c alone matches a's height.
    let (a, b, c) = (ZoneId::new(), ZoneId::new(), ZoneId::new());
    let inner = GridNode::split(Orientation::Rows, vec![GridNode::leaf(b), GridNode::leaf(c)]);
    let layout = GridLayout::new(GridNode::split(Orientation::Columns, vec![GridNode::leaf(a), inner]));
    assert!(!layout.can_join(a, JoinDirection::Right, bounds()));
    assert!(!layout.can_join(b, JoinDirection::Left, bounds()));
}

#[test]
fn join_errors() {
    let layout = GridLayout::equal_columns(2);
    let a = layout.leaves()[0];
    assert_eq!(layout.join(a, JoinDirection::Left, bounds()).unwrap_err(), LayoutError::NoNeighbor(JoinDirection::Left, a));
    let unknown = ZoneId::new();
    assert_eq!(layout.join(unknown, JoinDirection::Left, bounds()).unwrap_err(), LayoutError::ZoneNotFound(unknown));
}

#[test]
fn join_three_columns_keeps_proportions() {
    let layout = GridLayout::equal_columns(3);
    let a = layout.leaves()[0];
    let rects = layout.join(a, JoinDirection::Right, bounds()).unwrap().zone_rects(bounds());
    assert_eq!(rects.len(), 2);
    assert!((rect_of(&rects, a).width - 800.0 * 2.0 / 3.0).abs() < 1.0);
}

// JSON

#[test]
fn json_roundtrip_preserves_structure_and_rects() {
    let layout = GridLayout::uniform_grid(2, 3);
    let id = layout.root().as_split().unwrap().id;
    let layout = layout.set_weight(id, 0, 2.5).unwrap();
    let restored = GridLayout::from_json(&layout.to_json()).unwrap();
    assert_eq!(restored, layout);
}

#[test]
fn json_uses_dotnet_shape() {
    let json = GridLayout::equal_columns(2).to_json();
    assert!(json.contains("\"$type\": \"split\""));
    assert!(json.contains("\"Orientation\": \"Columns\""));
    assert!(json.contains("\"Children\""));
    assert!(!json.contains("-0000-"));
}

#[test]
fn json_reads_file_written_by_dotnet_app() {
    let json = r#"{
      "$type": "split",
      "Id": "5f2b6c1e9d8a4b7c8e1f2a3b4c5d6e7f",
      "Orientation": "Rows",
      "Children": [
        { "Node": { "$type": "leaf", "Id": "0a1b2c3d4e5f60718293a4b5c6d7e8f9" }, "Weight": 1 },
        { "Node": { "$type": "leaf", "Id": "1a1b2c3d-4e5f-6071-8293-a4b5c6d7e8f9" }, "Weight": 2.5 }
      ]
    }"#;
    let layout = GridLayout::from_json(json).unwrap();
    let rects = layout.zone_rects(Rect::new(0.0, 0.0, 100.0, 350.0));
    assert_eq!(rects[0].bounds.height, 100.0);
    assert_eq!(rects[1].bounds.height, 250.0);
    assert_eq!(rects[1].id.to_string(), "1a1b2c3d4e5f60718293a4b5c6d7e8f9");
}
