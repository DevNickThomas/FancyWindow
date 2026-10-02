//! The Keyboard shortcuts dialog: every global hotkey with its chord and whether
//! Windows actually gave it to us, a Change button to reassign it, and the mouse
//! gestures for reference.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::IsDialogMessageW;

use super::dialogs::{self, CANCEL, Dialog, Look, OK, set_text};
use crate::app::{CONFIGURABLE, Command, MOUSE_GESTURES};
use crate::model::Chord;
use crate::view::Theme;

/// One hotkey as the dialog shows it.
pub struct Row {
    pub command: Command,
    pub current: Option<Chord>,
    pub status: Status,
}

pub enum Status {
    Active,
    /// Another Fancy Window holds it and forwards presses to whichever is in front.
    Shared,
    /// Another application holds it, so pressing it never reaches Fancy Window.
    Taken,
}

/// What the user asked for.
pub enum Request {
    Change(Command, Option<Chord>),
    ResetAll,
}

const WIDTH: f64 = 680.0;
const ROW: f64 = 34.0;
const CHANGE_ID: usize = 400;
const RESET_ID: usize = 500;

/// `rows` reads the current bindings; `audition` says whether a chord can be used
/// for a command; `on_request` carries out a change. The dialog refreshes from
/// `rows` after every request.
pub fn show(
    owner: HWND,
    theme: Theme,
    rows: impl Fn() -> Vec<Row>,
    audition: impl Fn(Command, Chord) -> Result<(), String>,
    mut on_request: impl FnMut(Request),
) {
    let gestures_top = 40.0 + CONFIGURABLE.len() as f64 * ROW + 16.0;
    let height = gestures_top + 26.0 + MOUSE_GESTURES.len() as f64 * 22.0 + 64.0;
    let dialog = Dialog::new(owner, theme, "Keyboard shortcuts", WIDTH, height);

    dialog.label("Global hotkeys (work from any application)", (16.0, 12.0, WIDTH - 32.0, 22.0));
    let chord_labels: Vec<HWND> = CONFIGURABLE
        .iter()
        .enumerate()
        .map(|(i, command)| {
            let y = 40.0 + i as f64 * ROW;
            dialog.label(&command.label(), (16.0, y + 5.0, 250.0, 22.0));
            dialog.button("Change", CHANGE_ID + i, (WIDTH - 102.0, y), (86.0, 28.0), Look::Push { primary: false });
            dialog.label("", (272.0, y + 5.0, WIDTH - 272.0 - 110.0, 22.0))
        })
        .collect();

    dialog.label("Mouse", (16.0, gestures_top, 200.0, 22.0));
    for (i, (gesture, effect)) in MOUSE_GESTURES.iter().enumerate() {
        let y = gestures_top + 26.0 + i as f64 * 22.0;
        dialog.label(gesture, (16.0, y, 250.0, 20.0));
        dialog.label(effect, (272.0, y, WIDTH - 288.0, 20.0));
    }
    dialog.button("Reset to defaults", RESET_ID, (16.0, height - 44.0), (150.0, 28.0), Look::Push { primary: false });
    dialog.button("Close", OK, (WIDTH - 102.0, height - 44.0), (86.0, 28.0), Look::Push { primary: true });

    let refresh = |rows: Vec<Row>| {
        for (row, label) in rows.iter().zip(&chord_labels) {
            set_text(*label, &describe(row));
        }
    };
    refresh(rows());
    loop {
        let pressed = dialog.run(|msg| unsafe { IsDialogMessageW(dialog.hwnd, msg).as_bool() });
        match pressed {
            OK | CANCEL => break,
            RESET_ID => on_request(Request::ResetAll),
            id if (CHANGE_ID..CHANGE_ID + CONFIGURABLE.len()).contains(&id) => {
                let command = CONFIGURABLE[id - CHANGE_ID];
                let current = rows().into_iter().find(|r| r.command == command).and_then(|r| r.current);
                let title = format!("Hotkey: {}", command.label());
                if let Some(chord) = dialogs::capture_hotkey(dialog.hwnd, theme, &title, current, |c| audition(command, c)) {
                    on_request(Request::Change(command, chord));
                }
            }
            _ => {}
        }
        refresh(rows());
    }
    dialog.finish(());
}

fn describe(row: &Row) -> String {
    let Some(chord) = row.current.map(|c| c.display()) else { return "(none)".into() };
    match row.status {
        Status::Active => chord,
        Status::Shared => format!("{chord}  (via another Fancy Window)"),
        Status::Taken => format!("{chord}  (in use by another app)"),
    }
}
