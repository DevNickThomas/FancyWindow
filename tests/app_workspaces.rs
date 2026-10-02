use fancy_window::app::*;
use fancy_window::model::*;

const NOW: &str = "2026-10-01T09:30:00Z";
const A: WindowId = WindowId(1);

fn state() -> AppState {
    let mut state = AppState::new();
    update(&mut state, Msg::FrameChanged(Frame::new(900.0, 600.0, Point::new(0.0, 0.0), 1.0)));
    state
}

fn save_as(state: &mut AppState, slot: usize, name: &str) -> Vec<Effect> {
    update(state, Msg::WorkspaceNamed { slot, name: name.into(), saved_at_utc: NOW.into() })
}

fn saved_settings(effects: &[Effect]) -> &Settings {
    effects.iter().find_map(|e| if let Effect::SaveSettings(s) = e { Some(s) } else { None }).expect("save effect")
}

#[test]
fn save_asks_for_a_name_then_stores_layout_and_saves_settings() {
    let mut state = state();
    assert_eq!(update(&mut state, Msg::Menu(MenuAction::SaveWorkspace(2))), vec![Effect::PromptWorkspaceName { slot: 2, default: "Workspace 3".into() }]);
    let effects = save_as(&mut state, 2, "  Code  ");
    let ws = saved_settings(&effects).workspaces[2].clone().expect("slot filled");
    assert_eq!(ws.name.as_deref(), Some("Code"));
    assert_eq!(ws.saved_at_utc, NOW);
    assert_eq!(GridLayout::from_json(&ws.layout_json).unwrap(), state.layout);
}

#[test]
fn overwriting_keeps_the_hotkey_and_offers_the_old_name() {
    let mut state = state();
    save_as(&mut state, 0, "Code");
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord: Chord::parse("Ctrl+Win+D1") });
    assert_eq!(update(&mut state, Msg::Menu(MenuAction::SaveWorkspace(0))), vec![Effect::PromptWorkspaceName { slot: 0, default: "Code".into() }]);
    save_as(&mut state, 0, "Code v2");
    assert_eq!(state.workspace(0).unwrap().hotkey_chord.as_deref(), Some("Ctrl+Win+D1"));
}

#[test]
fn blank_name_is_stored_as_unnamed() {
    let mut state = state();
    save_as(&mut state, 1, "   ");
    assert_eq!(state.workspace(1).unwrap().name, None);
}

#[test]
fn loading_restores_layout_and_carries_windows() {
    let mut state = state();
    state.layout = GridLayout::equal_rows(3);
    save_as(&mut state, 4, "Rows");
    let saved = state.layout.clone();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::WindowDropped { window: A, at: Point::new(100.0, 100.0), alt: true });
    update(&mut state, Msg::Menu(MenuAction::LoadWorkspace(4)));
    assert_eq!(state.layout, saved);
    assert_eq!(state.zone_of(A), Some(saved.leaves()[0]));
}

#[test]
fn workspace_hotkey_loads_it() {
    let mut state = state();
    state.layout = GridLayout::equal_rows(3);
    save_as(&mut state, 0, "Rows");
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::Command(Command::LoadWorkspace(0)));
    assert_eq!(state.layout.leaves().len(), 3);
}

#[test]
fn loading_an_empty_slot_does_nothing() {
    let mut state = state();
    let before = state.layout.clone();
    assert!(update(&mut state, Msg::Menu(MenuAction::LoadWorkspace(5))).is_empty());
    assert_eq!(state.layout, before);
}

#[test]
fn setting_a_hotkey_binds_and_saves() {
    let mut state = state();
    save_as(&mut state, 0, "Code");
    assert_eq!(update(&mut state, Msg::Menu(MenuAction::SetWorkspaceHotkey(0))), vec![Effect::PromptHotkey { command: Command::LoadWorkspace(0), current: None }]);
    let chord = Chord::parse("Ctrl+Win+W");
    let effects = update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord });
    assert_eq!(effects[0], Effect::BindHotkey { command: Command::LoadWorkspace(0), chord });
    assert_eq!(saved_settings(&effects).workspaces[0].as_ref().unwrap().hotkey_chord.as_deref(), Some("Ctrl+Win+W"));
    assert!(state.hotkey_bindings().contains(&(chord.unwrap(), Command::LoadWorkspace(0))));
}

#[test]
fn clearing_a_hotkey_unbinds_it() {
    let mut state = state();
    save_as(&mut state, 0, "Code");
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord: Chord::parse("Ctrl+Win+W") });
    let effects = update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord: None });
    assert_eq!(effects[0], Effect::BindHotkey { command: Command::LoadWorkspace(0), chord: None });
    assert_eq!(state.chord_for(Command::LoadWorkspace(0)), None);
}

#[test]
fn failed_registration_clears_the_chord_and_warns() {
    let mut state = state();
    save_as(&mut state, 0, "Code");
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord: Chord::parse("Ctrl+Win+W") });
    let effects = update(&mut state, Msg::HotkeyFailed { command: Command::LoadWorkspace(0) });
    assert_eq!(state.workspace(0).unwrap().hotkey_chord, None);
    assert!(effects.iter().any(|e| matches!(e, Effect::ShowWarning { text, .. } if text.contains("Ctrl+Win+W"))));
}

#[test]
fn delete_confirms_then_clears_slot_and_hotkey() {
    let mut state = state();
    save_as(&mut state, 3, "Docs");
    assert_eq!(update(&mut state, Msg::Menu(MenuAction::DeleteWorkspace(3))), vec![Effect::ConfirmDeleteWorkspace { slot: 3, name: "Docs".into() }]);
    let effects = update(&mut state, Msg::WorkspaceDeleteConfirmed { slot: 3 });
    assert_eq!(effects[0], Effect::BindHotkey { command: Command::LoadWorkspace(3), chord: None });
    assert!(state.workspace(3).is_none());
}

#[test]
fn menu_lists_saved_workspaces_with_hotkeys() {
    let mut state = state();
    assert!(matches!(&workspace_menu(&state)[0], MenuItem::Note(text) if text == "No saved workspaces"));
    save_as(&mut state, 1, "Code");
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(1), chord: Chord::parse("Ctrl+Win+D2") });
    match &workspace_menu(&state)[0] {
        MenuItem::Item { label, shortcut, action, .. } => {
            assert_eq!(label, "2  Code");
            assert_eq!(shortcut.as_deref(), Some("Ctrl+Win+D2"));
            assert_eq!(*action, MenuAction::LoadWorkspace(1));
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn reads_workspaces_saved_by_dotnet_app() {
    let json = r#"{ "workspaces": [ { "name": "Old", "layoutJson": "{\"$type\":\"leaf\",\"Id\":\"0a1b2c3d4e5f60718293a4b5c6d7e8f9\"}", "savedAtUtc": "2025-01-02T03:04:05.1234567Z", "hotkeyChord": "Ctrl+Win+D9" }, null ] }"#;
    let state = AppState::from_settings(Settings::from_json(json).unwrap(), None);
    assert_eq!(state.workspace(0).unwrap().name.as_deref(), Some("Old"));
    assert_eq!(state.chord_for(Command::LoadWorkspace(0)), Chord::parse("Ctrl+Win+D9"));
}
