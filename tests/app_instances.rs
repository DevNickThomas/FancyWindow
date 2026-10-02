use fancy_window::app::*;
use fancy_window::model::*;

fn saved(effects: &[Effect]) -> &Settings {
    effects.iter().find_map(|e| if let Effect::SaveSettings(s) = e { Some(s) } else { None }).expect("settings saved")
}

#[test]
fn window_identity_survives_layout_changes_and_restart() {
    let mut state = AppState::from_settings(Settings::default(), Some("stable-profile".into()));
    let effects = update(&mut state, Msg::WindowRenamed("  Claude · Project A  ".into()));
    assert_eq!(state.title(), "Claude · Project A — Fancy Window");
    assert_eq!(state.centre_title(), "Claude · Project A");
    assert_eq!(state.profile.as_deref(), Some("stable-profile"));
    let settings = Settings::from_json(&saved(&effects).to_json()).unwrap();
    let restored = AppState::from_settings(settings, state.profile.clone());
    assert_eq!(restored.title(), state.title());
    update(&mut state, Msg::Menu(MenuAction::ApplyPreset(2)));
    assert_eq!(state.centre_title(), "Claude · Project A");
    assert_eq!(state.status_bar().left[0].text, "Claude · Project A");
    assert!(update(&mut state, Msg::WindowRenamed(" \n ".into())).is_empty());
    assert_eq!(state.title(), restored.title());
}

#[test]
fn new_window_inherits_appearance_without_sharing_identity_or_saved_workspaces() {
    let mut state = AppState::new();
    state.settings.theme_name = "Light Modern".into();
    state.settings.accent_color = "#7EE787".into();
    state.margin = 12.0;
    state.settings.window_name = Some("Existing".into());
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Layout".into(), saved_at_utc: "now".into() });
    let before = state.current_settings();
    let effects = update(&mut state, Msg::NewWindowNamed { name: "Codex · Project B".into(), workspace: None });
    let [Effect::LaunchInstance(settings)] = effects.as_slice() else { panic!("launch expected") };
    assert_eq!(settings.window_name.as_deref(), Some("Codex · Project B"));
    assert_eq!(settings.theme_name, "Light Modern");
    assert_eq!(settings.accent_color, "#7EE787");
    assert_eq!(settings.window_margin, 12);
    assert!(settings.workspaces.iter().all(Option::is_none));
    assert_eq!(settings.bounds(), None);
    assert_eq!(state.current_settings(), before);
}

#[test]
fn opening_saved_layout_in_another_window_uses_the_saved_layout_not_current_edits() {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::WorkspaceNamed { slot: 3, name: "Agent pair".into(), saved_at_utc: "now".into() });
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(3), chord: Chord::parse("Ctrl+Alt+F9") });
    let original = state.workspace(3).unwrap().clone();
    state.layout = GridLayout::equal_rows(3);
    let effects = update(&mut state, Msg::NewWindowNamed { name: "Claude".into(), workspace: Some(3) });
    let [Effect::LaunchInstance(settings)] = effects.as_slice() else { panic!("launch expected") };
    assert_eq!(settings.last_layout_json.as_ref(), Some(&original.layout_json));
    assert_eq!(settings.workspaces[0].as_ref().unwrap().hotkey_chord, None);
    assert_eq!(state.workspace(3), Some(&original));
    assert_eq!(state.layout.leaves().len(), 3);
    assert!(update(&mut state, Msg::NewWindowNamed { name: "Missing".into(), workspace: Some(99) }).is_empty());
    assert!(update(&mut state, Msg::NewWindowNamed { name: " ".into(), workspace: None }).is_empty());
}

#[test]
fn renaming_a_saved_workspace_preserves_its_layout_hotkey_and_timestamp() {
    let mut state = AppState::new();
    state.layout = GridLayout::equal_columns(2);
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Old".into(), saved_at_utc: "original time".into() });
    update(&mut state, Msg::SetHotkey { command: Command::LoadWorkspace(0), chord: Chord::parse("Ctrl+Alt+F9") });
    let original = state.workspace(0).unwrap().clone();
    state.layout = GridLayout::equal_rows(3);
    update(&mut state, Msg::WorkspaceRenamed { slot: 0, name: "  Pair  ".into() });
    let renamed = state.workspace(0).unwrap();
    assert_eq!(renamed.name.as_deref(), Some("Pair"));
    assert_eq!(renamed.layout_json, original.layout_json);
    assert_eq!(renamed.hotkey_chord, original.hotkey_chord);
    assert_eq!(renamed.saved_at_utc, original.saved_at_utc);
    assert_eq!(state.layout.leaves().len(), 3);
    update(&mut state, Msg::Menu(MenuAction::LoadWorkspace(0)));
    assert_eq!(state.layout.leaves().len(), 2);
    assert!(state.centre_title().contains("Pair"));
}

#[test]
fn organisation_commands_are_discoverable_in_the_keyboard_palette() {
    let mut state = AppState::new();
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Agents".into(), saved_at_utc: "now".into() });
    let entries = state.palette_entries();
    for action in [MenuAction::NewInstance, MenuAction::OpenWindowPicker, MenuAction::RenameInstance,
                   MenuAction::RenameWorkspace(0), MenuAction::OpenWorkspaceInNewWindow(0)] {
        assert!(entries.iter().any(|entry| entry.action == action), "missing {action:?}");
    }
}
