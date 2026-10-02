//! Named windows have their own settings; saved workspaces remain layout presets.

use crate::model::Settings;
use super::{AppState, Effect};

pub(super) fn ask_new(state: &AppState, workspace: Option<usize>) -> Vec<Effect> {
    let default = match workspace {
        Some(slot) => match state.workspace(slot) {
            Some(ws) => ws.name.clone().unwrap_or_else(|| "New window".into()),
            None => return vec![],
        },
        None => "New window".into(),
    };
    vec![Effect::PromptNewWindow { workspace, default }]
}

pub(super) fn create(state: &AppState, name: &str, workspace: Option<usize>) -> Vec<Effect> {
    let Some(name) = clean_name(name) else { return vec![] };
    // Copy appearance, not identity, position, bindings or other saved layouts.
    let mut settings = Settings {
        window_name: Some(name),
        theme_name: state.settings.theme_name.clone(),
        accent_color: state.settings.accent_color.clone(),
        window_margin: state.margin as i32,
        show_zone_headers: state.settings.show_zone_headers,
        ..Settings::default()
    };
    if let Some(slot) = workspace {
        let Some(ws) = state.workspace(slot) else { return vec![] };
        if crate::model::GridLayout::from_json(&ws.layout_json).is_err() { return vec![]; }
        let mut copy = ws.clone();
        copy.hotkey_chord = None;
        settings.last_layout_json = Some(copy.layout_json.clone());
        settings.workspaces[0] = Some(copy);
    }
    vec![Effect::LaunchInstance(settings)]
}

pub(super) fn rename(state: &mut AppState, name: &str) -> Vec<Effect> {
    let Some(name) = clean_name(name) else { return vec![] };
    state.settings.window_name = Some(name);
    vec![Effect::SaveSettings(state.current_settings()), Effect::Repaint]
}

pub(super) fn clean_name(name: &str) -> Option<String> {
    let name: String = name.trim().chars().filter(|c| !c.is_control()).take(100).collect();
    (!name.is_empty()).then_some(name)
}
