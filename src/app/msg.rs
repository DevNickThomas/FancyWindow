use super::{Command, Frame, MenuAction, WindowId};
use crate::model::ZoneId;
use crate::model::{Chord, Point, Rect, Settings, WindowBounds};

/// Something that happened, already translated out of Win32 terms.
#[derive(Clone, Debug, PartialEq)]
pub enum Msg {
    /// The canvas moved, resized or changed DPI. Points in other messages are canvas DIPs.
    FrameChanged(Frame),
    MouseDown { at: Point, button: Button, mods: Modifiers },
    MouseMove { at: Point, mods: Modifiers },
    MouseUp,
    MouseLeft,
    CaptureLost,
    ModifiersChanged(Modifiers),
    /// A global hotkey fired.
    Command(Command),
    /// A menu item was chosen.
    Menu(MenuAction),
    /// The name prompt for saving a workspace was confirmed; `saved_at_utc` is ISO-8601.
    WorkspaceNamed { slot: usize, name: String, saved_at_utc: String },
    /// A chord was chosen for a command; `None` clears it.
    SetHotkey { command: Command, chord: Option<Chord> },
    /// Windows would not register the chord just assigned.
    HotkeyFailed { command: Command },
    /// Put every built-in hotkey back on its default.
    ResetHotkeys,
    WorkspaceDeleteConfirmed { slot: usize },
    /// A theme preset was picked in Preferences.
    SetTheme(String),
    /// An accent colour (#RRGGBB) was picked in Preferences.
    SetAccent(String),
    /// Win+Alt+] or [; `has_peer` says whether another instance could take the cycle over.
    Cycle { forward: bool, has_peer: bool },
    /// Another instance handed the cycle to this one.
    BeginCycleAtEdge { forward: bool },
    /// Another window finished a move; `at` is the cursor in screen pixels.
    WindowDropped { window: WindowId, at: Point, alt: bool },
    /// A hosted window no longer exists.
    WindowClosed(WindowId),
    /// A hosted window's title, read on hosting and whenever it changes.
    TitleChanged { window: WindowId, title: String },
    /// Fancy Window became the active window.
    Activated,
    /// Some window, ours or not, became the foreground window.
    ForegroundChanged(WindowId),
    /// Fancy Window is about to close; `bounds` is its normal (unmaximised) placement.
    Closing { bounds: WindowBounds },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub ctrl: bool,
    pub shift: bool,
}

/// Something the platform layer must do after an update. Rects are screen pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Repaint,
    CaptureMouse,
    ReleaseMouse,
    /// Take over a window: strip its frame, own it, move it to `rect`.
    Host { window: WindowId, rect: Rect },
    /// Move a hosted window.
    Place { window: WindowId, rect: Rect },
    /// Give a window back exactly as it was before hosting, including position.
    Release(WindowId),
    /// Give a window back where it is now.
    Forget(WindowId),
    /// Keep a hosted window above Fancy Window.
    Raise(WindowId),
    /// Write settings to disk.
    SaveSettings(Settings),
    /// Make a hosted window the foreground window.
    Focus(WindowId),
    /// Pass the cycle to the next instance to the right (forward) or left.
    HandOffCycle { forward: bool },
    /// Sink Fancy Window to the bottom of the z-order.
    SendToBack,
    /// Raise and activate Fancy Window.
    BringToFront,
    /// Show or hide the "Fancy Window is behind" reminder.
    StayBackIndicator(bool),
    /// Pop up the zone menu at a canvas point.
    ShowZoneMenu { zone: ZoneId, at: Point },
    OpenSettingsFolder,
    ShowShortcuts,
    ShowAbout,
    /// Close Fancy Window.
    Exit,
    /// Ask for a workspace name; reply with `Msg::WorkspaceNamed`.
    PromptWorkspaceName { slot: usize, default: String },
    /// Capture a chord; reply with `Msg::SetHotkey`.
    PromptHotkey { command: Command, current: Option<String> },
    /// Ask before deleting; reply with `Msg::WorkspaceDeleteConfirmed`.
    ConfirmDeleteWorkspace { slot: usize, name: String },
    /// Register (or with `None`, drop) a command's chord; on failure reply `Msg::HotkeyFailed`.
    BindHotkey { command: Command, chord: Option<Chord> },
    ShowWarning { title: String, text: String },
    ShowPreferences,
    /// The theme changed: recolour title bars, menus and the reminder.
    ApplyTheme,
}
