use fancy_window::app::*;
use fancy_window::model::*;

const A: WindowId = WindowId(1);
const B: WindowId = WindowId(2);
const C: WindowId = WindowId(3);

fn state_with(layout: GridLayout) -> AppState {
    let mut state = AppState::new();
    state.layout = layout;
    update(&mut state, Msg::FrameChanged(Frame::new(900.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

fn host(state: &mut AppState, window: WindowId, x: f64, y: f64) {
    update(state, Msg::WindowDropped { window, at: Point::new(x, y), alt: true });
}

fn menu(state: &mut AppState, action: MenuAction) -> Vec<Effect> {
    update(state, Msg::Menu(action))
}

fn find_item<'a>(items: &'a [MenuItem], wanted: &str) -> &'a MenuItem {
    items.iter().find(|i| matches!(i, MenuItem::Item { label, .. } if label == wanted)).expect(wanted)
}

fn enabled(item: &MenuItem) -> bool {
    matches!(item, MenuItem::Item { enabled: true, .. })
}

#[test]
fn menu_bar_has_file_layout_workspaces_help() {
    let titles: Vec<&str> = menu_bar(&AppState::new()).iter().map(|m| m.title).collect();
    assert_eq!(titles, vec!["File", "Layout", "Workspaces", "Help"]);
}

#[test]
fn presets_build_expected_zone_counts() {
    let counts: Vec<usize> = PRESETS.iter().map(|(_, make)| make().leaves().len()).collect();
    assert_eq!(counts, vec![1, 2, 2, 4, 3, 3, 3, 3, 3]);
}

#[test]
fn big_left_preset_gives_two_thirds_to_the_big_zone() {
    let (name, make) = PRESETS[6];
    assert_eq!(name, "Big left, stacked right");
    let rects = make().zone_rects(Rect::new(0.0, 0.0, 900.0, 600.0));
    assert_eq!(rects[0].bounds, Rect::new(0.0, 0.0, 600.0, 600.0));
    assert_eq!(rects[1].bounds, Rect::new(600.0, 0.0, 300.0, 300.0));
}

#[test]
fn applying_a_preset_carries_windows_in_reading_order() {
    let mut state = state_with(GridLayout::equal_columns(3));
    host(&mut state, A, 150.0, 300.0);
    host(&mut state, B, 450.0, 300.0);
    host(&mut state, C, 750.0, 300.0);
    // "Two stacked": A goes top, B bottom, C has no zone left.
    let effects = menu(&mut state, MenuAction::ApplyPreset(1));
    assert!(effects.contains(&Effect::Release(C)));
    assert!(!effects.contains(&Effect::Release(A)));
    let zones = state.layout.leaves();
    assert_eq!(state.zone_of(A), Some(zones[0]));
    assert_eq!(state.zone_of(B), Some(zones[1]));
    assert!(effects.iter().any(|e| matches!(e, Effect::Place { window: A, rect } if rect.width > 800.0)));
}

#[test]
fn add_outer_adds_a_zone() {
    let mut state = state_with(GridLayout::equal_columns(2));
    menu(&mut state, MenuAction::AddOuter { orientation: Orientation::Rows, at_end: true });
    assert_eq!(state.layout.leaves().len(), 3);
}

#[test]
fn ctrl_right_click_on_zone_opens_its_menu() {
    let mut state = state_with(GridLayout::equal_columns(2));
    let ctrl = Modifiers { ctrl: true, shift: false };
    let effects = update(&mut state, Msg::MouseDown { at: Point::new(600.0, 100.0), button: Button::Right, mods: ctrl });
    assert_eq!(effects, vec![Effect::ShowZoneMenu { zone: state.layout.leaves()[1], at: Point::new(600.0, 100.0) }]);
}

#[test]
fn zone_menu_enables_only_possible_joins() {
    let state = state_with(GridLayout::equal_columns(2));
    let left = state.layout.leaves()[0];
    let items = zone_menu(&state, left);
    assert!(enabled(find_item(&items, "Join right")));
    assert!(!enabled(find_item(&items, "Join left")));
    assert!(!enabled(find_item(&items, "Join up")));
    assert!(enabled(find_item(&items, "Remove zone")));
    let single = state_with(GridLayout::new(GridNode::new_leaf()));
    assert!(!enabled(find_item(&zone_menu(&single, single.layout.leaves()[0]), "Remove zone")));
}

#[test]
fn zone_menu_actions_edit_the_layout() {
    let mut state = state_with(GridLayout::equal_columns(2));
    let [left, right] = [state.layout.leaves()[0], state.layout.leaves()[1]];
    menu(&mut state, MenuAction::SplitZone(left, Orientation::Rows));
    assert_eq!(state.layout.leaves().len(), 3);
    menu(&mut state, MenuAction::RemoveZone(right));
    assert_eq!(state.layout.leaves().len(), 2);
}

#[test]
fn joining_releases_the_window_in_the_absorbed_zone() {
    let mut state = state_with(GridLayout::equal_columns(2));
    host(&mut state, B, 600.0, 300.0);
    let left = state.layout.leaves()[0];
    let effects = menu(&mut state, MenuAction::Join(left, JoinDirection::Right));
    assert!(effects.contains(&Effect::Release(B)));
    assert_eq!(state.layout.leaves(), vec![left]);
}

#[test]
fn stale_menu_action_is_ignored() {
    let mut state = state_with(GridLayout::equal_columns(2));
    let gone = ZoneId::new();
    assert!(menu(&mut state, MenuAction::RemoveZone(gone)).is_empty());
    assert!(menu(&mut state, MenuAction::Join(gone, JoinDirection::Left)).is_empty());
}

#[test]
fn file_and_help_items_become_effects() {
    let mut state = AppState::new();
    assert_eq!(menu(&mut state, MenuAction::Exit), vec![Effect::Exit]);
    assert_eq!(menu(&mut state, MenuAction::OpenSettingsFolder), vec![Effect::OpenSettingsFolder]);
    assert_eq!(menu(&mut state, MenuAction::ShowShortcuts), vec![Effect::ShowShortcuts]);
    assert_eq!(menu(&mut state, MenuAction::Run(Command::MarginUp)), vec![]);
    assert_eq!(state.margin, 2.0);
}
