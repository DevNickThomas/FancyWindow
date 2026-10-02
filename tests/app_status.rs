use fancy_window::app::*;
use fancy_window::model::*;

const A: WindowId = WindowId(1);
const B: WindowId = WindowId(2);

/// Two columns on an 800x600 canvas at the screen origin.
fn two_columns() -> AppState {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::FrameChanged(Frame::new(800.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

fn save_workspace(state: &mut AppState, slot: usize, name: &str) {
    update(state, Msg::WorkspaceNamed { slot, name: name.into(), saved_at_utc: "2026-10-02T00:00:00Z".into() });
}

fn texts(state: &AppState) -> Vec<String> {
    state.status_bar().left.into_iter().map(|s| s.text).collect()
}

fn first(state: &AppState) -> Segment {
    state.status_bar().left.remove(0)
}

#[test]
fn workspace_zones_and_margin_with_their_clicks() {
    let mut state = two_columns();
    update(&mut state, Msg::WindowDropped { window: A, at: Point::new(200.0, 300.0), alt: true });
    let bar = state.status_bar();
    let shape: Vec<(SegmentKind, &str, Option<StatusClick>)> = bar.left.iter().map(|s| (s.kind, s.text.as_str(), s.click)).collect();
    assert_eq!(
        shape,
        vec![
            (SegmentKind::Strong, "No workspace", Some(StatusClick::WorkspacesMenu)),
            (SegmentKind::Plain, "2 zones \u{00B7} 1 hosted", Some(StatusClick::LayoutMenu)),
            (SegmentKind::Plain, "Margin 0", Some(StatusClick::Margin)),
        ]
    );
    // One window: nothing to cycle between, so no hint.
    assert_eq!(bar.hint, None);
}

#[test]
fn singular_zone() {
    let mut state = two_columns();
    state.layout = GridLayout::new(GridNode::new_leaf());
    assert_eq!(texts(&state)[1], "1 zone \u{00B7} 0 hosted");
}

#[test]
fn cycle_hint_uses_the_users_chord() {
    let mut state = two_columns();
    update(&mut state, Msg::WindowDropped { window: A, at: Point::new(200.0, 300.0), alt: true });
    update(&mut state, Msg::WindowDropped { window: B, at: Point::new(600.0, 300.0), alt: true });
    assert_eq!(state.status_bar().hint.as_deref(), Some("Win+Alt+] next window"));
    update(&mut state, Msg::SetHotkey { command: Command::CycleNext, chord: Chord::parse("Ctrl+Alt+OemOpenBrackets") });
    assert_eq!(state.status_bar().hint.as_deref(), Some("Ctrl+Alt+[ next window"));
}

#[test]
fn names_the_workspace_on_screen_until_the_layout_changes() {
    let mut state = two_columns();
    save_workspace(&mut state, 2, "Coding");
    assert_eq!(state.current_workspace(), Some(2));
    assert_eq!(first(&state).text, "Coding");
    // Splitting a zone means it is no longer the saved layout.
    update(&mut state, Msg::MouseDown { at: Point::new(200.0, 300.0), button: Button::Left, mods: Modifiers { ctrl: true, shift: false } });
    assert_eq!(state.current_workspace(), None);
    assert_eq!(first(&state).text, "No workspace");
    // Loading it again brings the name back.
    update(&mut state, Msg::Menu(MenuAction::LoadWorkspace(2)));
    assert_eq!(state.current_workspace(), Some(2));
}

#[test]
fn profile_and_workspace_share_the_strong_segment() {
    let mut state = AppState::from_settings(Settings::default(), Some("work".into()));
    state.layout = GridLayout::equal_columns(2);
    assert_eq!(first(&state).text, "Profile: work");
    save_workspace(&mut state, 0, "Coding");
    assert_eq!(first(&state).text, "Coding \u{00B7} Profile: work");
}

#[test]
fn warns_while_held_at_the_back() {
    let mut state = two_columns();
    update(&mut state, Msg::Command(Command::SendToBack));
    let warning = state.status_bar().left.remove(1);
    assert_eq!(warning.kind, SegmentKind::Warning);
    assert_eq!(warning.text, "Held at the back \u{00B7} Win+Alt+PageUp brings it forward");
    assert_eq!(warning.click, Some(StatusClick::BringForward));
    update(&mut state, Msg::Command(Command::BringToFront));
    assert!(!state.status_bar().left.iter().any(|s| s.kind == SegmentKind::Warning));
}
