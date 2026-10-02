use fancy_window::app::*;
use fancy_window::model::*;

const CTRL: Modifiers = Modifiers { ctrl: true, shift: false };
const SHIFT: Modifiers = Modifiers { ctrl: false, shift: true };
const NONE: Modifiers = Modifiers { ctrl: false, shift: false };

/// Two equal columns on an 800x600 canvas: splitter at x=400.
fn two_columns() -> AppState {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn down(state: &mut AppState, at: Point, button: Button, mods: Modifiers) -> Vec<Effect> {
    update(state, Msg::MouseDown { at, button, mods })
}

#[test]
fn starts_with_two_by_two_grid() {
    assert_eq!(AppState::new().layout.leaves().len(), 4);
}

#[test]
fn ctrl_click_splits_zone_into_columns() {
    let mut state = two_columns();
    let effects = down(&mut state, p(100.0, 100.0), Button::Left, CTRL);
    assert_eq!(effects, vec![Effect::Repaint]);
    let rects = state.zone_rects();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].bounds.width, 200.0);
}

#[test]
fn shift_click_splits_zone_into_rows() {
    let mut state = two_columns();
    down(&mut state, p(100.0, 100.0), Button::Left, SHIFT);
    let rects = state.zone_rects();
    assert_eq!(rects.len(), 3);
    assert_eq!(rects[0].bounds.height, 300.0);
}

#[test]
fn plain_click_on_zone_does_nothing() {
    let mut state = two_columns();
    assert!(down(&mut state, p(100.0, 100.0), Button::Left, NONE).is_empty());
    assert!(down(&mut state, p(100.0, 100.0), Button::Right, NONE).is_empty());
    assert_eq!(state.layout.leaves().len(), 2);
}

#[test]
fn dragging_a_splitter_resizes_zones() {
    let mut state = two_columns();
    assert_eq!(down(&mut state, p(400.0, 300.0), Button::Left, NONE), vec![Effect::CaptureMouse]);
    assert_eq!(update(&mut state, Msg::MouseMove { at: p(480.0, 300.0), mods: NONE }), vec![Effect::Repaint]);
    assert_eq!(update(&mut state, Msg::MouseMove { at: p(500.0, 310.0), mods: NONE }), vec![Effect::Repaint]);
    assert_eq!(update(&mut state, Msg::MouseUp), vec![Effect::ReleaseMouse, Effect::Repaint]);
    assert_eq!(state.zone_rects()[0].bounds.width, 500.0);
    assert!(state.drag.is_none());
}

#[test]
fn tiny_drag_steps_are_ignored() {
    let mut state = two_columns();
    down(&mut state, p(400.0, 300.0), Button::Left, NONE);
    assert!(update(&mut state, Msg::MouseMove { at: p(400.2, 300.0), mods: NONE }).is_empty());
}

#[test]
fn losing_capture_ends_drag() {
    let mut state = two_columns();
    down(&mut state, p(400.0, 300.0), Button::Left, NONE);
    update(&mut state, Msg::CaptureLost);
    assert!(state.drag.is_none());
}

#[test]
fn right_click_on_splitter_merges_into_left() {
    let mut state = two_columns();
    let left = state.layout.leaves()[0];
    assert_eq!(down(&mut state, p(400.0, 300.0), Button::Right, NONE), vec![Effect::Repaint]);
    assert_eq!(state.layout.leaves(), vec![left]);
}

#[test]
fn cursor_shows_resize_over_splitters() {
    let mut state = two_columns();
    assert_eq!(state.cursor_at(p(400.0, 10.0)), CursorKind::SizeWestEast);
    assert_eq!(state.cursor_at(p(100.0, 10.0)), CursorKind::Arrow);
    state.layout = GridLayout::equal_rows(2);
    assert_eq!(state.cursor_at(p(100.0, 300.0)), CursorKind::SizeNorthSouth);
}

#[test]
fn holding_ctrl_previews_vertical_split_line() {
    let mut state = two_columns();
    update(&mut state, Msg::MouseMove { at: p(100.0, 100.0), mods: NONE });
    assert_eq!(state.split_preview(), None);
    assert_eq!(update(&mut state, Msg::ModifiersChanged(CTRL)), vec![Effect::Repaint]);
    assert_eq!(state.split_preview(), Some(Rect::new(199.0, 0.0, 2.0, 600.0)));
}

#[test]
fn holding_shift_previews_horizontal_split_line() {
    let mut state = two_columns();
    update(&mut state, Msg::MouseMove { at: p(500.0, 100.0), mods: SHIFT });
    assert_eq!(state.split_preview(), Some(Rect::new(400.0, 299.0, 400.0, 2.0)));
}

#[test]
fn preview_hides_when_mouse_leaves() {
    let mut state = two_columns();
    update(&mut state, Msg::MouseMove { at: p(100.0, 100.0), mods: CTRL });
    assert_eq!(update(&mut state, Msg::MouseLeft), vec![Effect::Repaint]);
    assert_eq!(state.split_preview(), None);
}

#[test]
fn moving_within_same_zone_does_not_repaint() {
    let mut state = two_columns();
    update(&mut state, Msg::MouseMove { at: p(100.0, 100.0), mods: CTRL });
    assert!(update(&mut state, Msg::MouseMove { at: p(150.0, 120.0), mods: CTRL }).is_empty());
}

#[test]
fn reset_restores_two_by_two() {
    let mut state = two_columns();
    update(&mut state, Msg::Command(Command::ResetLayout));
    assert_eq!(state.layout.leaves().len(), 4);
}
