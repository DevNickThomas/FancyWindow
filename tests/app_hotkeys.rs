use fancy_window::app::*;
use fancy_window::model::*;

fn chord(text: &str) -> Option<Chord> {
    Chord::parse(text)
}

fn set(state: &mut AppState, command: Command, text: Option<&str>) -> Vec<Effect> {
    update(state, Msg::SetHotkey { command, chord: text.and_then(Chord::parse) })
}

fn saved(effects: &[Effect]) -> &Settings {
    effects.iter().find_map(|e| if let Effect::SaveSettings(s) = e { Some(s) } else { None }).expect("save effect")
}

#[test]
fn defaults_use_win_alt_and_leave_ctrl_win_free() {
    let state = AppState::new();
    let labels: Vec<(Command, String)> = CONFIGURABLE.iter().map(|&c| (c, state.chord_label(c).unwrap())).collect();
    assert_eq!(
        labels,
        vec![
            (Command::CycleNext, "Win+Alt+]".into()),
            (Command::CyclePrevious, "Win+Alt+[".into()),
            (Command::SendToBack, "Win+Alt+PageDown".into()),
            (Command::BringToFront, "Win+Alt+PageUp".into()),
            (Command::MarginUp, "Win+Alt+=".into()),
            (Command::MarginDown, "Win+Alt+-".into()),
            (Command::ResetLayout, "Win+Alt+Home".into()),
        ]
    );
    assert_eq!(state.hotkey_bindings().len(), 7);
}

#[test]
fn reassigning_binds_the_new_chord_and_saves_it_by_name() {
    let mut state = AppState::new();
    let effects = set(&mut state, Command::CycleNext, Some("Ctrl+Alt+Right"));
    assert_eq!(effects[0], Effect::BindHotkey { command: Command::CycleNext, chord: chord("Ctrl+Alt+Right") });
    assert_eq!(saved(&effects).hotkeys.get("cycleNext").map(String::as_str), Some("Ctrl+Alt+Right"));
    assert_eq!(state.chord_for(Command::CycleNext), chord("Ctrl+Alt+Right"));
}

#[test]
fn clearing_leaves_the_command_without_a_chord() {
    let mut state = AppState::new();
    set(&mut state, Command::ResetLayout, None);
    assert_eq!(state.chord_for(Command::ResetLayout), None);
    assert!(!state.hotkey_bindings().iter().any(|(_, c)| *c == Command::ResetLayout));
    assert_eq!(state.settings.hotkeys.get("resetLayout").map(String::as_str), Some(""));
}

#[test]
fn choosing_the_default_again_removes_the_override() {
    let mut state = AppState::new();
    set(&mut state, Command::SendToBack, Some("Ctrl+Alt+B"));
    set(&mut state, Command::SendToBack, Some("Alt+Win+PageDown"));
    assert!(state.settings.hotkeys.is_empty());
}

#[test]
fn a_chord_used_elsewhere_is_refused() {
    let mut state = AppState::new();
    let effects = set(&mut state, Command::CycleNext, Some("Alt+Win+PageDown"));
    assert!(matches!(&effects[..], [Effect::ShowWarning { text, .. }] if text.contains("Send Fancy Window behind")));
    assert_eq!(state.chord_label(Command::CycleNext).as_deref(), Some("Win+Alt+]"));
}

#[test]
fn chord_user_names_the_other_command_but_not_itself() {
    let state = AppState::new();
    let b = chord("Alt+Win+PageDown").unwrap();
    assert_eq!(state.chord_user(b, Command::CycleNext), Some(Command::SendToBack));
    assert_eq!(state.chord_user(b, Command::SendToBack), None);
    assert_eq!(state.chord_user(chord("Ctrl+Alt+K").unwrap(), Command::CycleNext), None);
}

#[test]
fn workspace_chords_count_as_used() {
    let mut state = AppState::new();
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Code".into(), saved_at_utc: "now".into() });
    set(&mut state, Command::LoadWorkspace(0), Some("Ctrl+Win+D1"));
    let effects = set(&mut state, Command::MarginUp, Some("Ctrl+Win+D1"));
    assert!(matches!(&effects[..], [Effect::ShowWarning { text, .. }] if text.contains("Load workspace 1")));
}

#[test]
fn failure_clears_and_warns() {
    let mut state = AppState::new();
    set(&mut state, Command::CycleNext, Some("Ctrl+Alt+Right"));
    let effects = update(&mut state, Msg::HotkeyFailed { command: Command::CycleNext });
    assert_eq!(state.chord_for(Command::CycleNext), None);
    assert!(effects.iter().any(|e| matches!(e, Effect::ShowWarning { text, .. } if text.contains("Ctrl+Alt+Right"))));
}

#[test]
fn reset_restores_every_default() {
    let mut state = AppState::new();
    set(&mut state, Command::CycleNext, Some("Ctrl+Alt+Right"));
    set(&mut state, Command::ResetLayout, None);
    let effects = update(&mut state, Msg::ResetHotkeys);
    assert!(state.settings.hotkeys.is_empty());
    assert!(effects.contains(&Effect::BindHotkey { command: Command::CycleNext, chord: chord("Alt+Win+OemCloseBrackets") }));
    assert!(effects.contains(&Effect::BindHotkey { command: Command::ResetLayout, chord: chord("Alt+Win+Home") }));
}

#[test]
fn overrides_survive_a_restart() {
    let mut state = AppState::new();
    set(&mut state, Command::CycleNext, Some("Ctrl+Alt+Right"));
    let json = state.current_settings().to_json();
    let restored = AppState::from_settings(Settings::from_json(&json).unwrap(), None);
    assert_eq!(restored.chord_for(Command::CycleNext), chord("Ctrl+Alt+Right"));
}

#[test]
fn menus_show_the_current_chord() {
    let mut state = AppState::new();
    set(&mut state, Command::MarginUp, Some("Ctrl+Alt+Up"));
    let layout = &menu_bar(&state)[1];
    let shortcut = layout.items.iter().find_map(|i| match i {
        MenuItem::Item { label, shortcut, .. } if label == "Increase margin" => shortcut.clone(),
        _ => None,
    });
    assert_eq!(shortcut.as_deref(), Some("Ctrl+Alt+Up"));
}

#[test]
fn chords_display_with_symbols() {
    assert_eq!(chord("Ctrl+Win+OemMinus").unwrap().display(), "Ctrl+Win+-");
    assert_eq!(chord("Ctrl+Win+D7").unwrap().display(), "Ctrl+Win+7");
    assert_eq!(chord("Ctrl+Alt+Delete").unwrap().display(), "Ctrl+Alt+Delete");
}

#[test]
fn no_default_takes_ctrl_win() {
    // Ctrl+Win chords are left to other apps.
    let state = AppState::new();
    for (chord, command) in state.hotkey_bindings() {
        assert!(!(chord.modifiers & MOD_CONTROL != 0 && chord.modifiers & MOD_WIN != 0), "{command:?} uses Ctrl+Win");
    }
}
