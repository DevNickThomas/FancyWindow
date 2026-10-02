use fancy_window::app::*;
use fancy_window::model::*;

const WIN: WindowId = WindowId(0x1234);
const OTHER: WindowId = WindowId(0x5678);
const BOUNDS: WindowBounds = WindowBounds { left: 1.0, top: 2.0, width: 3.0, height: 4.0, maximized: false };

/// Two columns, canvas 800x600 DIPs, at screen (100, 50), 100% scale. Zone headers
/// are off here so the numbers are about attaching; tests/app_headers.rs covers them.
fn two_columns() -> AppState {
    let mut state = AppState::new();
    state.settings.show_zone_headers = false;
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(100.0, 50.0), 1.0)));
    state
}

fn drop_at(state: &mut AppState, window: WindowId, x: f64, y: f64, alt: bool) -> Vec<Effect> {
    update(state, Msg::WindowDropped { window, at: Point::new(x, y), alt })
}

// host_rect

#[test]
fn host_rect_insets_outline_on_edges_and_half_splitter_inside() {
    let canvas = Rect::new(0.0, 0.0, 800.0, 600.0);
    // Left column: outer edges get 2, the splitter side gets 2 + 2.
    let left = host_rect(Rect::new(0.0, 0.0, 400.0, 600.0), canvas, 0.0, 0.0);
    assert_eq!(left, Rect::new(2.0, 2.0, 400.0 - 2.0 - 4.0, 600.0 - 4.0));
    // Margin adds on every side.
    let with_margin = host_rect(Rect::new(0.0, 0.0, 400.0, 600.0), canvas, 10.0, 0.0);
    assert_eq!(with_margin, Rect::new(12.0, 12.0, 400.0 - 12.0 - 14.0, 600.0 - 24.0));
}

#[test]
fn host_rect_sits_below_the_header() {
    let canvas = Rect::new(0.0, 0.0, 800.0, 600.0);
    let r = host_rect(Rect::new(0.0, 0.0, 400.0, 600.0), canvas, 0.0, 28.0);
    assert_eq!(r, Rect::new(2.0, 30.0, 394.0, 600.0 - 28.0 - 4.0));
}

#[test]
fn host_rect_never_goes_negative() {
    let r = host_rect(Rect::new(10.0, 10.0, 3.0, 3.0), Rect::new(0.0, 0.0, 800.0, 600.0), 0.0, 28.0);
    assert_eq!((r.width, r.height), (0.0, 0.0));
}

#[test]
fn frame_converts_between_screen_and_canvas() {
    let frame = Frame::new(800.0, 600.0, Point::new(100.0, 50.0), 1.5);
    assert_eq!(frame.to_screen(Rect::new(10.0, 20.0, 100.0, 40.0)), Rect::new(115.0, 80.0, 150.0, 60.0));
    assert_eq!(frame.from_screen(Point::new(115.0, 80.0)), Point::new(10.0, 20.0));
}

// attaching

#[test]
fn alt_drop_over_zone_hosts_window_in_that_zone() {
    let mut state = two_columns();
    let right = state.layout.leaves()[1];
    let effects = drop_at(&mut state, WIN, 700.0, 300.0, true);
    // Right zone is canvas (400,0,400,600) -> host (404,2,394,596) -> screen +(100,50).
    assert_eq!(effects, vec![Effect::Host { window: WIN, rect: Rect::new(504.0, 52.0, 394.0, 596.0) }, Effect::Repaint]);
    assert_eq!(state.zone_of(WIN), Some(right));
}

// active window highlight

#[test]
fn a_dropped_window_is_active_and_highlighted() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    assert_eq!(state.active, Some(WIN));
    let h = state.active_highlight().expect("highlight");
    // Glow covers the whole zone, splitter half included; the bevel hugs the window 2 DIPs out.
    assert_eq!(h.glow, Rect::new(400.0, 0.0, 400.0, 600.0));
    assert_eq!(h.bevel, Rect::new(402.0, 0.0, 398.0, 600.0));
}

#[test]
fn focus_moves_the_highlight_between_hosted_windows_only() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    drop_at(&mut state, OTHER, 200.0, 300.0, true);
    assert_eq!(update(&mut state, Msg::ForegroundChanged(WIN)), vec![Effect::Repaint]);
    assert_eq!(state.active, Some(WIN));
    // The same window again changes nothing.
    assert!(update(&mut state, Msg::ForegroundChanged(WIN)).is_empty());
    // Any other app (or Fancy Window itself) clears it.
    assert_eq!(update(&mut state, Msg::ForegroundChanged(WindowId(0x9999))), vec![Effect::Repaint]);
    assert_eq!(state.active_highlight(), None);
}

#[test]
fn bevel_follows_the_margin() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 200.0, 300.0, true);
    state.margin = 10.0;
    let h = state.active_highlight().expect("highlight");
    // Window at (12,12)-(386,588) with margin 10; bevel 2 DIPs outside it.
    assert_eq!(h.bevel, Rect::new(10.0, 10.0, 378.0, 580.0));
}

#[test]
fn a_released_window_loses_the_highlight() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    drop_at(&mut state, WIN, 900.0, 300.0, true); // Alt+drag it out
    assert_eq!(state.active_highlight(), None);
}

#[test]
fn activating_a_hosted_window_moves_the_cycle_there() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    drop_at(&mut state, OTHER, 200.0, 300.0, true);
    update(&mut state, Msg::ForegroundChanged(WIN));
    assert_eq!(state.cycle, Some(WIN));
}

#[test]
fn drop_without_alt_is_ignored() {
    let mut state = two_columns();
    assert!(drop_at(&mut state, WIN, 700.0, 300.0, false).is_empty());
    assert!(state.attachments.is_empty());
}

#[test]
fn alt_drop_outside_canvas_is_ignored() {
    let mut state = two_columns();
    assert!(drop_at(&mut state, WIN, 50.0, 300.0, true).is_empty());
    assert!(drop_at(&mut state, WIN, 500.0, 700.0, true).is_empty());
}

#[test]
fn hosting_into_occupied_zone_releases_previous_window() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = drop_at(&mut state, OTHER, 700.0, 300.0, true);
    assert_eq!(effects[0], Effect::Release(WIN));
    assert!(matches!(effects[1], Effect::Host { window: OTHER, .. }));
    assert_eq!(state.zone_of(WIN), None);
}

#[test]
fn moving_a_hosted_window_snaps_it_back() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = drop_at(&mut state, WIN, 200.0, 300.0, false);
    assert_eq!(effects, vec![Effect::Place { window: WIN, rect: Rect::new(504.0, 52.0, 394.0, 596.0) }]);
}

#[test]
fn alt_moving_a_hosted_window_lets_it_go_where_it_is() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    // Repaint: the status bar's hosted count and the highlight change.
    assert_eq!(drop_at(&mut state, WIN, 200.0, 300.0, true), vec![Effect::Forget(WIN), Effect::Repaint]);
    assert!(state.attachments.is_empty());
}

// following the layout

#[test]
fn moving_fancy_window_moves_hosted_windows() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    assert!(effects.contains(&Effect::Place { window: WIN, rect: Rect::new(404.0, 2.0, 394.0, 596.0) }));
}

#[test]
fn dragging_a_splitter_moves_hosted_windows() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    update(&mut state, Msg::MouseDown { at: Point::new(400.0, 300.0), button: Button::Left, mods: Modifiers::default() });
    let effects = update(&mut state, Msg::MouseMove { at: Point::new(500.0, 300.0), mods: Modifiers::default() });
    assert!(effects.contains(&Effect::Place { window: WIN, rect: Rect::new(604.0, 52.0, 294.0, 596.0) }));
}

#[test]
fn splitting_keeps_window_in_first_half() {
    let mut state = two_columns();
    let right = state.layout.leaves()[1];
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let ctrl = Modifiers { ctrl: true, shift: false };
    update(&mut state, Msg::MouseDown { at: Point::new(700.0, 300.0), button: Button::Left, mods: ctrl });
    assert_eq!(state.zone_of(WIN), Some(right));
    assert_eq!(state.host_screen_rect(right).unwrap().width, 200.0 - 4.0 - 4.0);
}

#[test]
fn merging_away_a_zone_releases_its_window() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = update(&mut state, Msg::MouseDown { at: Point::new(400.0, 300.0), button: Button::Right, mods: Modifiers::default() });
    assert!(effects.contains(&Effect::Release(WIN)));
    assert!(state.attachments.is_empty());
}

#[test]
fn reset_releases_all_windows() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    assert!(update(&mut state, Msg::Command(Command::ResetLayout)).contains(&Effect::Release(WIN)));
}

#[test]
fn unrelated_messages_do_not_move_hosted_windows() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = update(&mut state, Msg::MouseMove { at: Point::new(10.0, 10.0), mods: Modifiers::default() });
    assert!(effects.iter().all(|e| !matches!(e, Effect::Place { .. })));
}

// lifecycle

#[test]
fn closed_window_is_forgotten_silently() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    // Nothing is sent to the dead window; the status bar just repaints.
    assert_eq!(update(&mut state, Msg::WindowClosed(WIN)), vec![Effect::Repaint]);
    assert!(state.attachments.is_empty());
}

#[test]
fn activation_raises_every_hosted_window() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    drop_at(&mut state, OTHER, 200.0, 300.0, true);
    assert_eq!(update(&mut state, Msg::Activated), vec![Effect::Raise(WIN), Effect::Raise(OTHER)]);
}

#[test]
fn closing_forgets_every_hosted_window() {
    let mut state = two_columns();
    drop_at(&mut state, WIN, 700.0, 300.0, true);
    let effects = update(&mut state, Msg::Closing { bounds: BOUNDS });
    assert_eq!(effects[0], Effect::Forget(WIN));
    assert!(matches!(effects[1], Effect::SaveSettings(_)));
    assert!(state.attachments.is_empty());
}
