//! Menus as plain data. The platform turns them into popups and sends back the
//! chosen `MenuAction` as `Msg::Menu`.

use crate::model::{GridLayout, GridNode, JoinDirection, Orientation, SplitChild, SplitId, SplitNode, ZoneId};

use super::{AppState, Command, Effect, WindowId, workspace};

#[derive(Clone, Debug, PartialEq)]
pub enum MenuItem {
    Item { label: String, shortcut: Option<String>, enabled: bool, action: MenuAction },
    /// Greyed-out text that does nothing.
    Note(String),
    Submenu { label: String, items: Vec<MenuItem> },
    Separator,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    pub title: &'static str,
    pub items: Vec<MenuItem>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuAction {
    OpenSettingsFolder,
    Preferences,
    Exit,
    ApplyPreset(usize),
    AddOuter { orientation: Orientation, at_end: bool },
    Run(Command),
    SplitZone(ZoneId, Orientation),
    Join(ZoneId, JoinDirection),
    RemoveZone(ZoneId),
    ShowShortcuts,
    ShowAbout,
    LoadWorkspace(usize),
    SaveWorkspace(usize),
    SetWorkspaceHotkey(usize),
    DeleteWorkspace(usize),
    ToggleZoneHeaders,
    /// The title bar's centre box: open the command palette.
    OpenPalette,
}

/// Builds a preset layout with fresh zone ids.
pub type PresetFn = fn() -> GridLayout;

/// Preset layouts in the Layout menu.
pub const PRESETS: &[(&str, PresetFn)] = &[
    ("Single", || GridLayout::new(GridNode::new_leaf())),
    ("Two stacked", || GridLayout::equal_rows(2)),
    ("Two side-by-side", || GridLayout::equal_columns(2)),
    ("2\u{00D7}2 grid", || GridLayout::uniform_grid(2, 2)),
    ("Three rows", || GridLayout::equal_rows(3)),
    ("Three columns", || GridLayout::equal_columns(3)),
    ("Big left, stacked right", || big_and_pair(Orientation::Columns, true)),
    ("Stacked left, big right", || big_and_pair(Orientation::Columns, false)),
    ("Big top, split bottom", || big_and_pair(Orientation::Rows, true)),
];

/// A weight-2 zone next to a pair split the other way; `big_first` puts the big one first.
fn big_and_pair(orientation: Orientation, big_first: bool) -> GridLayout {
    let other = match orientation {
        Orientation::Columns => Orientation::Rows,
        Orientation::Rows => Orientation::Columns,
    };
    let big = SplitChild { node: GridNode::new_leaf(), weight: 2.0 };
    let pair = SplitChild { node: GridNode::split(other, vec![GridNode::new_leaf(), GridNode::new_leaf()]), weight: 1.0 };
    let children = if big_first { vec![big, pair] } else { vec![pair, big] };
    GridLayout::new(GridNode::Split(SplitNode { id: SplitId::new(), orientation, children }))
}

fn item(label: &str, action: MenuAction) -> MenuItem {
    MenuItem::Item { label: label.into(), shortcut: None, enabled: true, action }
}

/// A menu item that runs a command, showing the command's current chord.
fn command_item(state: &AppState, label: &str, command: Command) -> MenuItem {
    MenuItem::Item { label: label.into(), shortcut: state.chord_label(command), enabled: true, action: MenuAction::Run(command) }
}

pub fn menu_bar(state: &AppState) -> Vec<Menu> {
    vec![
        Menu {
            title: "File",
            items: vec![
                command_item(state, "Command palette...", Command::OpenPalette),
                MenuItem::Separator,
                item("Open settings folder", MenuAction::OpenSettingsFolder),
                MenuItem::Separator,
                item("Preferences...", MenuAction::Preferences),
                item("Keyboard shortcuts...", MenuAction::ShowShortcuts),
                MenuItem::Separator,
                item("Exit", MenuAction::Exit),
            ],
        },
        Menu { title: "Layout", items: layout_items(state) },
        Menu { title: "Workspaces", items: super::workspace::workspace_menu(state) },
        Menu {
            title: "Help",
            items: vec![item("Keyboard shortcuts...", MenuAction::ShowShortcuts), MenuItem::Separator, item("About", MenuAction::ShowAbout)],
        },
    ]
}

fn layout_items(state: &AppState) -> Vec<MenuItem> {
    let mut items: Vec<MenuItem> = PRESETS.iter().enumerate().map(|(i, (name, _))| item(name, MenuAction::ApplyPreset(i))).collect();
    items.push(MenuItem::Separator);
    let add = |label: &str, orientation, at_end| item(label, MenuAction::AddOuter { orientation, at_end });
    items.push(MenuItem::Submenu {
        label: "Add".into(),
        items: vec![
            add("Row above", Orientation::Rows, false),
            add("Row below", Orientation::Rows, true),
            add("Column left", Orientation::Columns, false),
            add("Column right", Orientation::Columns, true),
        ],
    });
    items.push(MenuItem::Separator);
    items.push(command_item(state, "Increase margin", Command::MarginUp));
    items.push(command_item(state, "Decrease margin", Command::MarginDown));
    items.push(command_item(state, "Reset to 2\u{00D7}2", Command::ResetLayout));
    items.push(MenuItem::Separator);
    let headers = if state.settings.show_zone_headers { "Hide zone headers" } else { "Show zone headers" };
    items.push(item(headers, MenuAction::ToggleZoneHeaders));
    items
}

/// The Ctrl+right-click menu for one zone. Joins are enabled only where a
/// same-sized neighbour exists.
pub fn zone_menu(state: &AppState, zone: ZoneId) -> Vec<MenuItem> {
    let join = |label: &str, direction| MenuItem::Item {
        label: label.into(),
        shortcut: None,
        enabled: state.layout.can_join(zone, direction, state.frame.canvas),
        action: MenuAction::Join(zone, direction),
    };
    vec![
        item("Split horizontal", MenuAction::SplitZone(zone, Orientation::Rows)),
        item("Split vertical", MenuAction::SplitZone(zone, Orientation::Columns)),
        MenuItem::Separator,
        join("Join up", JoinDirection::Up),
        join("Join down", JoinDirection::Down),
        join("Join left", JoinDirection::Left),
        join("Join right", JoinDirection::Right),
        MenuItem::Separator,
        MenuItem::Item {
            label: "Remove zone".into(),
            shortcut: None,
            enabled: state.layout.leaves().len() > 1,
            action: MenuAction::RemoveZone(zone),
        },
    ]
}

/// Mouse gestures for the shortcuts help: (gesture, what it does). Hotkeys are listed
/// from the live bindings instead, because they can be reassigned.
pub const MOUSE_GESTURES: &[(&str, &str)] = &[
    ("Alt + drag a window", "Attach it to the zone under the cursor"),
    ("Alt + drag a hosted window out", "Let it go"),
    ("Ctrl + click a zone", "Split it into columns"),
    ("Shift + click a zone", "Split it into rows"),
    ("Ctrl + right-click a zone", "Zone menu: split, join, remove"),
    ("Drag a splitter", "Resize"),
    ("Right-click a splitter", "Merge the zones beside it"),
    ("Click a zone header", "Focus its window"),
    ("Click \u{00D7} on a zone header", "Let the window go"),
];

pub(super) fn run(state: &mut AppState, action: MenuAction) -> Vec<Effect> {
    match action {
        MenuAction::Run(command) => super::command::run(state, command),
        MenuAction::ApplyPreset(i) => apply_layout(state, (PRESETS[i].1)()),
        MenuAction::AddOuter { orientation, at_end } => {
            state.layout = state.layout.add_outer(orientation, at_end);
            vec![Effect::Repaint]
        }
        MenuAction::SplitZone(zone, orientation) => edit_layout(state, |l| l.split_zone(zone, orientation)),
        MenuAction::Join(zone, direction) => {
            let canvas = state.frame.canvas;
            edit_layout(state, |l| l.join(zone, direction, canvas))
        }
        MenuAction::RemoveZone(zone) => edit_layout(state, |l| l.remove_zone(zone)),
        MenuAction::OpenSettingsFolder => vec![Effect::OpenSettingsFolder],
        MenuAction::Exit => vec![Effect::Exit],
        MenuAction::Preferences => vec![Effect::ShowPreferences],
        MenuAction::ShowShortcuts => vec![Effect::ShowShortcuts],
        MenuAction::ShowAbout => vec![Effect::ShowAbout],
        MenuAction::LoadWorkspace(slot) => workspace::load(state, slot),
        MenuAction::SaveWorkspace(slot) => workspace::ask_name(state, slot),
        MenuAction::SetWorkspaceHotkey(slot) => workspace::ask_hotkey(state, slot),
        MenuAction::DeleteWorkspace(slot) => workspace::ask_delete(state, slot),
        MenuAction::ToggleZoneHeaders => super::headers::toggle(state),
        MenuAction::OpenPalette => vec![Effect::ShowPalette],
    }
}

/// Menu actions act on the layout the menu was built from, but the user may
/// have changed it while the menu was open; a stale action is ignored.
fn edit_layout<E>(state: &mut AppState, edit: impl FnOnce(&GridLayout) -> Result<GridLayout, E>) -> Vec<Effect> {
    match edit(&state.layout) {
        Ok(layout) => {
            state.layout = layout;
            vec![Effect::Repaint]
        }
        Err(_) => vec![],
    }
}

/// Switches layout, keeping hosted windows in reading order: the first window
/// goes to the first new zone, and so on. Windows without a zone are released.
pub(super) fn apply_layout(state: &mut AppState, layout: GridLayout) -> Vec<Effect> {
    let windows: Vec<WindowId> = state.ordered_windows();
    let zones = layout.leaves();
    state.layout = layout;
    state.cycle = None;
    state.attachments = zones.iter().zip(&windows).map(|(&zone, &window)| super::Attachment { zone, window }).collect();
    let mut effects: Vec<Effect> = windows.iter().skip(zones.len()).map(|&w| Effect::Release(w)).collect();
    effects.push(Effect::Repaint);
    effects
}
