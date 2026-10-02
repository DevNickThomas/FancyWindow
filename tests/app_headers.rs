use fancy_window::app::*;
use fancy_window::model::*;

const A: WindowId = WindowId(1);
const B: WindowId = WindowId(2);

/// Two columns, canvas 800x600 DIPs at the screen origin, headers on (the default).
fn two_columns() -> AppState {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

/// A in the left column, B in the right.
fn with_two_windows() -> AppState {
    let mut state = two_columns();
    update(&mut state, Msg::WindowDropped { window: B, at: Point::new(600.0, 300.0), alt: true });
    update(&mut state, Msg::WindowDropped { window: A, at: Point::new(200.0, 300.0), alt: true });
    state
}

fn click(state: &mut AppState, x: f64, y: f64) -> Vec<Effect> {
    update(state, Msg::MouseDown { at: Point::new(x, y), button: Button::Left, mods: Modifiers::default() })
}

#[test]
fn headers_are_on_by_default_and_push_windows_down() {
    let mut state = two_columns();
    assert_eq!(state.header_height(), HEADER_HEIGHT);
    let effects = update(&mut state, Msg::WindowDropped { window: A, at: Point::new(200.0, 300.0), alt: true });
    // Left column: header 0..28, then the usual 2 DIP gap.
    assert_eq!(effects[0], Effect::Host { window: A, rect: Rect::new(2.0, 30.0, 394.0, 568.0) });
}

#[test]
fn one_header_per_occupied_zone_with_title_number_and_close() {
    let mut state = with_two_windows();
    update(&mut state, Msg::TitleChanged { window: A, title: "notes.txt - Notepad".into() });
    let headers = state.zone_headers();
    assert_eq!(headers.len(), 2);
    let a = &headers[0];
    assert_eq!(a.window, A);
    assert_eq!(a.title, "notes.txt - Notepad");
    assert_eq!(a.number, 1);
    // Left column's visible area stops at the splitter half (398).
    assert_eq!(a.bounds, Rect::new(0.0, 0.0, 398.0, 28.0));
    assert_eq!(a.close, Rect::new(398.0 - 5.0 - 18.0, 5.0, 18.0, 18.0));
    // B has no title yet, and is second in reading order.
    assert_eq!((headers[1].title.as_str(), headers[1].number), ("", 2));
}

#[test]
fn active_flag_follows_focus() {
    let mut state = with_two_windows();
    update(&mut state, Msg::ForegroundChanged(B));
    let active: Vec<WindowId> = state.zone_headers().iter().filter(|h| h.active).map(|h| h.window).collect();
    assert_eq!(active, vec![B]);
}

#[test]
fn clicking_a_header_focuses_its_window() {
    let mut state = with_two_windows();
    update(&mut state, Msg::ForegroundChanged(WindowId(99)));
    assert_eq!(click(&mut state, 600.0, 14.0), vec![Effect::Focus(B), Effect::Repaint]);
    assert_eq!(state.active, Some(B));
    assert_eq!(state.cycle, Some(B));
}

#[test]
fn clicking_close_releases_the_window() {
    let mut state = with_two_windows();
    let close = state.zone_headers()[1].close;
    let effects = click(&mut state, close.x + 9.0, close.y + 9.0);
    assert_eq!(effects[0], Effect::Release(B));
    assert_eq!(state.zone_of(B), None);
    assert_eq!(state.zone_headers().len(), 1);
}

#[test]
fn ctrl_click_on_a_header_still_splits_the_zone() {
    let mut state = with_two_windows();
    let mods = Modifiers { ctrl: true, shift: false };
    update(&mut state, Msg::MouseDown { at: Point::new(200.0, 14.0), button: Button::Left, mods });
    assert_eq!(state.layout.leaves().len(), 3);
}

#[test]
fn titles_of_windows_we_do_not_host_are_ignored() {
    let mut state = with_two_windows();
    assert!(update(&mut state, Msg::TitleChanged { window: WindowId(99), title: "Other".into() }).is_empty());
    // The same title twice repaints once.
    assert_eq!(update(&mut state, Msg::TitleChanged { window: A, title: "One".into() }), vec![Effect::Repaint]);
    assert!(update(&mut state, Msg::TitleChanged { window: A, title: "One".into() }).is_empty());
}

#[test]
fn a_released_windows_title_is_dropped() {
    let mut state = with_two_windows();
    update(&mut state, Msg::TitleChanged { window: A, title: "One".into() });
    update(&mut state, Msg::WindowClosed(A));
    assert!(!state.titles.contains_key(&A));
}

#[test]
fn empty_zones_get_hints_and_no_header() {
    let mut state = two_columns();
    update(&mut state, Msg::WindowDropped { window: A, at: Point::new(200.0, 300.0), alt: true });
    assert_eq!(state.empty_zones(), vec![Rect::new(402.0, 0.0, 398.0, 600.0)]);
    assert_eq!(state.zone_headers().len(), 1);
}

#[test]
fn toggling_headers_moves_windows_and_saves() {
    let mut state = with_two_windows();
    let effects = update(&mut state, Msg::Menu(MenuAction::ToggleZoneHeaders));
    assert!(!state.settings.show_zone_headers);
    assert!(state.zone_headers().is_empty());
    assert!(effects.iter().any(|e| matches!(e, Effect::SaveSettings(s) if !s.show_zone_headers)));
    assert!(effects.contains(&Effect::Place { window: A, rect: Rect::new(2.0, 2.0, 394.0, 596.0) }));
}

#[test]
fn layout_menu_offers_the_opposite_of_the_current_state() {
    let mut state = two_columns();
    let label = |s: &AppState| {
        menu_bar(s)[1].items.iter().find_map(|i| match i {
            MenuItem::Item { label, action: MenuAction::ToggleZoneHeaders, .. } => Some(label.clone()),
            _ => None,
        })
    };
    assert_eq!(label(&state).as_deref(), Some("Hide zone headers"));
    update(&mut state, Msg::Menu(MenuAction::ToggleZoneHeaders));
    assert_eq!(label(&state).as_deref(), Some("Show zone headers"));
}

#[test]
fn active_bevel_wraps_header_and_window() {
    let mut state = with_two_windows();
    update(&mut state, Msg::ForegroundChanged(A));
    let h = state.active_highlight().expect("highlight");
    assert_eq!(h.bevel, Rect::new(0.0, 0.0, 398.0, 600.0));
}

#[test]
fn old_settings_without_the_field_turn_headers_on() {
    let s = Settings::from_json(r#"{ "themeName": "Dark" }"#).expect("settings");
    assert!(s.show_zone_headers);
}

#[test]
fn hand_cursor_over_close_only() {
    let state = with_two_windows();
    let close = state.zone_headers()[0].close;
    assert_eq!(state.cursor_at(Point::new(close.x + 9.0, close.y + 9.0)), CursorKind::Hand);
    assert_eq!(state.cursor_at(Point::new(100.0, 14.0)), CursorKind::Arrow);
}
