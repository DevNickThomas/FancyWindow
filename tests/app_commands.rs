use fancy_window::app::*;
use fancy_window::model::*;

const A: WindowId = WindowId(1);
const B: WindowId = WindowId(2);
const C: WindowId = WindowId(3);

/// Three columns on an 900x600 canvas at the screen origin.
fn three_columns() -> AppState {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(3);
    update(&mut state, Msg::FrameChanged(Frame::new(900.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

/// Hosts A, B, C in the three columns, dropped right to left to prove order comes from zones.
fn with_three_windows() -> AppState {
    let mut state = three_columns();
    for (window, x) in [(C, 750.0), (B, 450.0), (A, 150.0)] {
        update(&mut state, Msg::WindowDropped { window, at: Point::new(x, 300.0), alt: true });
    }
    state
}

fn cycle(state: &mut AppState, forward: bool, has_peer: bool) -> Vec<Effect> {
    update(state, Msg::Cycle { forward, has_peer })
}

fn command(state: &mut AppState, c: Command) -> Vec<Effect> {
    update(state, Msg::Command(c))
}

// margin

#[test]
fn margin_hotkeys_step_by_two_within_limits_and_move_windows() {
    let mut state = with_three_windows();
    let effects = command(&mut state, Command::MarginUp);
    assert_eq!(state.margin, 2.0);
    assert!(effects.iter().any(|e| matches!(e, Effect::Place { window: A, .. })));
    command(&mut state, Command::MarginDown);
    command(&mut state, Command::MarginDown);
    assert_eq!(state.margin, 0.0);
    state.margin = MAX_MARGIN as f64;
    assert!(command(&mut state, Command::MarginUp).is_empty());
    assert_eq!(state.margin, MAX_MARGIN as f64);
}

// cycling

#[test]
fn cycle_follows_zone_reading_order_and_wraps_without_peers() {
    let mut state = with_three_windows();
    assert_eq!(state.ordered_windows(), vec![A, B, C]);
    let focused: Vec<WindowId> = (0..4)
        .map(|_| match cycle(&mut state, true, false)[0] {
            Effect::Focus(w) => w,
            ref other => panic!("expected focus, got {other:?}"),
        })
        .collect();
    assert_eq!(focused, vec![A, B, C, A]);
}

#[test]
fn cycle_backwards_starts_at_last() {
    let mut state = with_three_windows();
    assert_eq!(cycle(&mut state, false, false)[0], Effect::Focus(C));
    assert_eq!(cycle(&mut state, false, false)[0], Effect::Focus(B));
}

#[test]
fn cycle_hands_off_at_edge_when_a_peer_exists() {
    let mut state = with_three_windows();
    for _ in 0..3 {
        cycle(&mut state, true, true);
    }
    assert_eq!(state.cycle, Some(C));
    assert_eq!(cycle(&mut state, true, true)[0], Effect::HandOffCycle { forward: true });
    assert_eq!(state.cycle, None);
}

#[test]
fn empty_instance_hands_off_or_does_nothing() {
    let mut state = three_columns();
    assert_eq!(cycle(&mut state, true, true), vec![Effect::HandOffCycle { forward: true }, Effect::Repaint]);
    assert!(cycle(&mut state, true, false).is_empty());
}

#[test]
fn receiving_a_hand_off_lands_on_first_or_last() {
    let mut state = with_three_windows();
    assert_eq!(update(&mut state, Msg::BeginCycleAtEdge { forward: true })[0], Effect::Focus(A));
    assert_eq!(update(&mut state, Msg::BeginCycleAtEdge { forward: false })[0], Effect::Focus(C));
    let mut empty = three_columns();
    assert_eq!(update(&mut empty, Msg::BeginCycleAtEdge { forward: true }), vec![Effect::HandOffCycle { forward: true }]);
}

#[test]
fn cycle_highlight_outlines_the_focused_windows_zone() {
    let mut state = with_three_windows();
    assert_eq!(state.cycle_highlight(), None);
    cycle(&mut state, true, false);
    cycle(&mut state, true, false);
    assert_eq!(state.cycle_highlight(), Some(Rect::new(300.0, 0.0, 300.0, 600.0)));
}

#[test]
fn cycle_resumes_from_the_right_place_after_a_window_closes() {
    let mut state = with_three_windows();
    cycle(&mut state, true, false); // A
    update(&mut state, Msg::WindowClosed(A));
    assert_eq!(cycle(&mut state, true, false)[0], Effect::Focus(B));
}

// stay-back

#[test]
fn send_to_back_arms_stay_back_and_shows_reminder() {
    let mut state = with_three_windows();
    cycle(&mut state, true, false);
    let effects = command(&mut state, Command::SendToBack);
    assert_eq!(effects, vec![Effect::SendToBack, Effect::StayBackIndicator(true), Effect::Repaint]);
    assert!(state.stay_back);
    assert_eq!(state.cycle, None);
}

#[test]
fn attaching_in_stay_back_sinks_again() {
    let mut state = three_columns();
    command(&mut state, Command::SendToBack);
    let effects = update(&mut state, Msg::WindowDropped { window: A, at: Point::new(100.0, 100.0), alt: true });
    assert_eq!(effects.last(), Some(&Effect::SendToBack));
    let snap_back = update(&mut state, Msg::WindowDropped { window: A, at: Point::new(500.0, 100.0), alt: false });
    assert!(!snap_back.contains(&Effect::SendToBack));
}

#[test]
fn bring_to_front_or_clicking_leaves_stay_back() {
    let mut state = three_columns();
    command(&mut state, Command::SendToBack);
    let effects = command(&mut state, Command::BringToFront);
    assert_eq!(effects, vec![Effect::StayBackIndicator(false), Effect::BringToFront, Effect::Repaint]);
    assert!(!state.stay_back);

    command(&mut state, Command::SendToBack);
    let click = update(&mut state, Msg::MouseDown { at: Point::new(10.0, 10.0), button: Button::Left, mods: Modifiers::default() });
    assert_eq!(click, vec![Effect::StayBackIndicator(false)]);
    assert!(!state.stay_back);
}

