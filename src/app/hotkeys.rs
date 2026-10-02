//! Which chord runs which command, including the user's reassignments.
//!
//! Built-in chords can be changed or cleared; changes are stored by command
//! name in `settings.hotkeys`. Workspace chords live on their workspace.

use crate::model::{Chord, MOD_ALT, MOD_WIN, WORKSPACE_SLOTS};

use super::{AppState, Command, Effect};

/// The built-in commands, in the order the shortcuts dialog lists them.
pub const CONFIGURABLE: [Command; 8] = [
    Command::CycleNext,
    Command::CyclePrevious,
    Command::SendToBack,
    Command::BringToFront,
    Command::MarginUp,
    Command::MarginDown,
    Command::ResetLayout,
    Command::OpenPalette,
];

impl Command {
    /// Stable name used in settings.json.
    pub fn key(self) -> &'static str {
        match self {
            Command::MarginUp => "marginUp",
            Command::MarginDown => "marginDown",
            Command::BringToFront => "bringToFront",
            Command::SendToBack => "sendToBack",
            Command::ResetLayout => "resetLayout",
            Command::CycleNext => "cycleNext",
            Command::CyclePrevious => "cyclePrevious",
            Command::OpenPalette => "openPalette",
            Command::LoadWorkspace(_) => "loadWorkspace",
        }
    }

    pub fn label(self) -> String {
        match self {
            Command::MarginUp => "Increase margin".into(),
            Command::MarginDown => "Decrease margin".into(),
            Command::BringToFront => "Bring Fancy Window forward".into(),
            Command::SendToBack => "Send Fancy Window behind".into(),
            Command::ResetLayout => "Reset to 2\u{00D7}2".into(),
            Command::CycleNext => "Focus next hosted window".into(),
            Command::CyclePrevious => "Focus previous hosted window".into(),
            Command::OpenPalette => "Open command palette".into(),
            Command::LoadWorkspace(slot) => format!("Load workspace {}", slot + 1),
        }
    }

    /// Win+Alt, leaving Ctrl+Win to other apps. Win+Alt+B (HDR) and Win+Alt+R (Game Bar
    /// recording) belong to Windows, hence PageUp/PageDown and Home for those commands.
    pub fn default_chord(self) -> Option<Chord> {
        let win_alt = |key| Some(Chord::new(MOD_WIN | MOD_ALT, key));
        match self {
            Command::MarginUp => win_alt(0xBB),      // =
            Command::MarginDown => win_alt(0xBD),    // -
            Command::BringToFront => win_alt(0x21),  // PageUp
            Command::SendToBack => win_alt(0x22),    // PageDown
            Command::ResetLayout => win_alt(0x24),   // Home
            Command::CycleNext => win_alt(0xDD),     // ]
            Command::CyclePrevious => win_alt(0xDB), // [
            Command::OpenPalette => win_alt(0x20),   // Space
            Command::LoadWorkspace(_) => None,
        }
    }
}

impl AppState {
    /// The chord currently assigned to a command, if any.
    pub fn chord_for(&self, command: Command) -> Option<Chord> {
        match command {
            Command::LoadWorkspace(slot) => Chord::parse(self.workspace(slot)?.hotkey_chord.as_deref()?),
            _ => match self.settings.hotkeys.get(command.key()) {
                Some(text) => Chord::parse(text),
                None => command.default_chord(),
            },
        }
    }

    /// Every chord to register: built-ins, then workspaces.
    pub fn hotkey_bindings(&self) -> Vec<(Chord, Command)> {
        let workspaces = (0..WORKSPACE_SLOTS).map(Command::LoadWorkspace);
        CONFIGURABLE.into_iter().chain(workspaces).filter_map(|c| Some((self.chord_for(c)?, c))).collect()
    }

    /// The other command already using `chord`, if any.
    pub fn chord_user(&self, chord: Chord, except: Command) -> Option<Command> {
        self.hotkey_bindings().into_iter().find(|&(used, other)| used == chord && other != except).map(|(_, other)| other)
    }

    /// The current chord as people read it, e.g. "Win+Alt+]".
    pub fn chord_label(&self, command: Command) -> Option<String> {
        self.chord_for(command).map(|c| c.display())
    }
}

/// Assigns (or with `None`, clears) a command's chord. Refuses a chord another
/// command already uses.
pub(super) fn set(state: &mut AppState, command: Command, chord: Option<Chord>) -> Vec<Effect> {
    if let Some(c) = chord
        && let Some(other) = state.chord_user(c, command)
    {
        return vec![Effect::ShowWarning {
            title: "Hotkey already used".into(),
            text: format!("{} is already used for \"{}\".", c.display(), other.label()),
        }];
    }
    store(state, command, chord);
    vec![Effect::BindHotkey { command, chord }, Effect::SaveSettings(state.current_settings())]
}

/// Windows refused the chord (another application owns it): clear it and say so.
pub(super) fn failed(state: &mut AppState, command: Command) -> Vec<Effect> {
    let chord = state.chord_label(command).unwrap_or_default();
    store(state, command, None);
    vec![
        Effect::SaveSettings(state.current_settings()),
        Effect::ShowWarning {
            title: "Hotkey unavailable".into(),
            text: format!("Could not register {chord}. It may already be in use by another application."),
        },
    ]
}

/// Puts every built-in command back on its default chord.
pub(super) fn reset(state: &mut AppState) -> Vec<Effect> {
    state.settings.hotkeys.clear();
    let mut effects: Vec<Effect> = CONFIGURABLE.iter().map(|&command| Effect::BindHotkey { command, chord: command.default_chord() }).collect();
    effects.push(Effect::SaveSettings(state.current_settings()));
    effects
}

fn store(state: &mut AppState, command: Command, chord: Option<Chord>) {
    match command {
        Command::LoadWorkspace(slot) => {
            if let Some(ws) = state.settings.workspaces[slot].as_mut() {
                ws.hotkey_chord = chord.map(|c| c.format());
            }
        }
        _ if chord == command.default_chord() => {
            state.settings.hotkeys.remove(command.key());
        }
        _ => {
            state.settings.hotkeys.insert(command.key().into(), chord.map(|c| c.format()).unwrap_or_default());
        }
    }
}
