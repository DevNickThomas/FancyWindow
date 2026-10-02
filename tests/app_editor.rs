use fancy_window::app::*;
use fancy_window::model::*;

fn grid() -> AppState {
    let mut s = AppState::new();
    s.frame = Frame::new(1000.0, 800.0, Point::new(20.0, 40.0), 1.5);
    s
}

#[test]
fn toolbar_retains_the_context_of_the_last_focused_hosted_app() {
    let mut s = grid();
    let zone = s.layout.leaves()[3];
    s.attachments.push(Attachment { zone, window: WindowId(1) });
    update(&mut s, Msg::ForegroundChanged(WindowId(1)));
    update(&mut s, Msg::ForegroundChanged(WindowId(999))); // toolbar owns focus
    assert_eq!(s.active, None);
    assert_eq!(s.editing_zone(), zone);
    assert_eq!(update(&mut s, Msg::Command(Command::EditLayout)), vec![Effect::ShowLayoutEditor]);
}

#[test]
fn spatial_navigation_and_tab_select_panes_without_changing_layout() {
    let s = grid();
    let z = s.layout.leaves();
    assert_eq!(s.adjacent_editing_zone(z[0], JoinDirection::Right), z[1]);
    assert_eq!(s.adjacent_editing_zone(z[1], JoinDirection::Down), z[3]);
    assert_eq!(s.adjacent_editing_zone(z[3], JoinDirection::Left), z[2]);
    assert_eq!(s.adjacent_editing_zone(z[2], JoinDirection::Up), z[0]);
    assert_eq!(s.adjacent_editing_zone(z[0], JoinDirection::Left), z[0]);
    assert_eq!(s.next_editing_zone(z[3], true), z[0]);
    assert_eq!(s.next_editing_zone(z[0], false), z[3]);
}

#[test]
fn splitting_selected_header_free_pane_preserves_both_apps_and_focus_context() {
    for layout in [GridLayout::equal_columns(2), GridLayout::uniform_grid(2, 2)] {
        for orientation in [Orientation::Rows, Orientation::Columns] {
            let mut s = grid();
            s.layout = layout.clone();
            let zones = s.layout.leaves();
            let selected = zones[0];
            let other = *zones.last().unwrap();
            s.attachments = vec![Attachment { zone: selected, window: WindowId(1) }, Attachment { zone: other, window: WindowId(2) }];
            update(&mut s, Msg::ForegroundChanged(WindowId(1)));
            let unchanged = s.host_screen_rect(other);
            assert_eq!(s.header_height(), 0.0);
            assert_eq!(s.editing_zone(), selected);
            let effects = update(&mut s, Msg::Menu(MenuAction::SplitZone(selected, orientation)));
            assert_eq!(s.layout.leaves().len(), zones.len() + 1);
            assert_eq!(s.zone_of(WindowId(1)), Some(selected));
            assert_eq!(s.zone_of(WindowId(2)), Some(other));
            assert_eq!(s.host_screen_rect(other), unchanged);
            assert_eq!(s.editing_zone(), selected);
            assert!(!effects.iter().any(|e| matches!(e, Effect::Release(_) | Effect::Forget(_))));
        }
    }
}

#[test]
fn closing_the_selected_app_falls_back_to_an_existing_pane() {
    let mut s = grid();
    let zone = s.layout.leaves()[2];
    s.attachments.push(Attachment { zone, window: WindowId(1) });
    update(&mut s, Msg::ForegroundChanged(WindowId(1)));
    update(&mut s, Msg::WindowClosed(WindowId(1)));
    assert!(s.layout.leaves().contains(&s.editing_zone()));
}

#[test]
fn a_newly_attached_app_is_the_edit_target_before_another_focus_event() {
    let mut s = grid();
    let at = s.frame.to_screen(Rect::new(700.0, 600.0, 1.0, 1.0));
    update(&mut s, Msg::WindowDropped { window: WindowId(1), at: Point::new(at.x, at.y), alt: true });
    let zone = s.zone_of(WindowId(1)).unwrap();
    update(&mut s, Msg::ForegroundChanged(WindowId(999)));
    assert_eq!(s.editing_zone(), zone);
}
