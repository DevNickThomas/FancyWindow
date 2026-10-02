//! Workspaces: nine slots, each a named layout with an optional global hotkey.

use crate::model::{GridLayout, WORKSPACE_SLOTS, Workspace};

use super::menu::apply_layout;
use super::{AppState, Command, Effect, MenuAction, MenuItem};

impl AppState {
    pub fn workspace(&self, slot: usize) -> Option<&Workspace> {
        self.settings.workspaces.get(slot)?.as_ref()
    }

    fn slot_label(&self, slot: usize) -> String {
        match self.workspace(slot) {
            Some(ws) => format!("Slot {} \u{2014} {}", slot + 1, display_name(ws)),
            None => format!("Slot {}  (empty)", slot + 1),
        }
    }
}

pub(super) fn display_name(ws: &Workspace) -> &str {
    ws.name.as_deref().unwrap_or("(unnamed)")
}

pub fn workspace_menu(state: &AppState) -> Vec<MenuItem> {
    let saved: Vec<usize> = (0..WORKSPACE_SLOTS).filter(|&s| state.workspace(s).is_some()).collect();
    let mut items: Vec<MenuItem> = saved
        .iter()
        .map(|&slot| {
            let ws = state.workspace(slot).expect("saved slot");
            MenuItem::Item {
                label: format!("{}  {}", slot + 1, display_name(ws)),
                shortcut: ws.hotkey_chord.clone(),
                enabled: true,
                action: MenuAction::LoadWorkspace(slot),
            }
        })
        .collect();
    if saved.is_empty() {
        items.push(MenuItem::Note("No saved workspaces".into()));
    }
    let slots = |action: fn(usize) -> MenuAction, saved_only: bool| -> Vec<MenuItem> {
        (0..WORKSPACE_SLOTS)
            .map(|slot| MenuItem::Item {
                label: state.slot_label(slot),
                shortcut: state.workspace(slot).and_then(|ws| ws.hotkey_chord.clone()),
                enabled: !saved_only || state.workspace(slot).is_some(),
                action: action(slot),
            })
            .collect()
    };
    items.push(MenuItem::Separator);
    items.push(MenuItem::Submenu { label: "Save current as".into(), items: slots(MenuAction::SaveWorkspace, false) });
    items.push(MenuItem::Submenu { label: "Rename".into(), items: slots(MenuAction::RenameWorkspace, true) });
    items.push(MenuItem::Submenu { label: "Open in new window".into(), items: slots(MenuAction::OpenWorkspaceInNewWindow, true) });
    items.push(MenuItem::Submenu { label: "Set hotkey".into(), items: slots(MenuAction::SetWorkspaceHotkey, true) });
    items.push(MenuItem::Submenu { label: "Delete".into(), items: slots(MenuAction::DeleteWorkspace, true) });
    items
}

/// Loads a saved layout, carrying hosted windows across like a preset.
pub(super) fn load(state: &mut AppState, slot: usize) -> Vec<Effect> {
    let Some(layout) = state.workspace(slot).and_then(|ws| GridLayout::from_json(&ws.layout_json).ok()) else {
        return vec![];
    };
    apply_layout(state, layout)
}

pub(super) fn ask_name(state: &AppState, slot: usize) -> Vec<Effect> {
    let default = state.workspace(slot).and_then(|ws| ws.name.clone()).unwrap_or_else(|| format!("Workspace {}", slot + 1));
    vec![Effect::PromptWorkspaceName { slot, default }]
}

/// Stores the current layout in a slot, keeping the slot's hotkey.
pub(super) fn save(state: &mut AppState, slot: usize, name: &str, saved_at_utc: String) -> Vec<Effect> {
    let hotkey_chord = state.workspace(slot).and_then(|ws| ws.hotkey_chord.clone());
    let name = Some(name.trim().to_string()).filter(|n| !n.is_empty());
    state.settings.workspaces[slot] = Some(Workspace { name, layout_json: state.layout.to_json(), saved_at_utc, hotkey_chord });
    vec![Effect::SaveSettings(state.current_settings()), Effect::Repaint]
}

pub(super) fn ask_hotkey(state: &AppState, slot: usize) -> Vec<Effect> {
    match state.workspace(slot) {
        Some(ws) => vec![Effect::PromptHotkey { command: Command::LoadWorkspace(slot), current: ws.hotkey_chord.clone() }],
        None => vec![],
    }
}

pub(super) fn ask_rename(state: &AppState, slot: usize) -> Vec<Effect> {
    state.workspace(slot).map(|ws| vec![Effect::PromptWorkspaceRename {
        slot, default: display_name(ws).into(),
    }]).unwrap_or_default()
}

/// Renaming must not replace a saved layout with the currently edited one.
pub(super) fn rename(state: &mut AppState, slot: usize, name: &str) -> Vec<Effect> {
    let Some(ws) = state.settings.workspaces.get_mut(slot).and_then(Option::as_mut) else { return vec![] };
    let Some(name) = super::instance::clean_name(name) else { return vec![] };
    ws.name = Some(name);
    vec![Effect::SaveSettings(state.current_settings()), Effect::Repaint]
}

pub(super) fn ask_delete(state: &AppState, slot: usize) -> Vec<Effect> {
    match state.workspace(slot) {
        Some(ws) => vec![Effect::ConfirmDeleteWorkspace { slot, name: display_name(ws).to_string() }],
        None => vec![],
    }
}

pub(super) fn delete(state: &mut AppState, slot: usize) -> Vec<Effect> {
    if state.settings.workspaces[slot].take().is_none() {
        return vec![];
    }
    vec![Effect::BindHotkey { command: Command::LoadWorkspace(slot), chord: None }, Effect::SaveSettings(state.current_settings()), Effect::Repaint]
}
