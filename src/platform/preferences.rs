//! The Preferences dialog: theme presets, accent swatches and a custom accent.
//! Every click applies straight away; Close just closes.

use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetFocus, VK_RETURN};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use super::dialogs::{CANCEL, Dialog, Look, OK, set_text, window_text};
use crate::view::{ACCENT_SWATCHES, Color, PRESETS, Theme};

/// What the user picked.
pub enum Change {
    Theme(&'static str),
    Accent(String),
}

const WIDTH: f64 = 460.0;
const ROW_ID: usize = 100;
const SWATCH_ID: usize = 200;
const APPLY_HEX: usize = 300;
const ROW_HEIGHT: f64 = 36.0;
const SWATCH: f64 = 44.0;

/// Shows Preferences. `apply` carries each change out and returns the resulting theme.
pub fn show(owner: HWND, theme: Theme, theme_name: &str, accent: &str, mut apply: impl FnMut(Change) -> Theme) {
    // Tall enough for every preset row, the swatches and the custom colour.
    let height = 36.0 + PRESETS.len() as f64 * ROW_HEIGHT + 8.0 + 24.0 + SWATCH + 10.0 + 26.0 + 60.0;
    let dialog = Dialog::new(owner, theme, "Preferences", WIDTH, height);
    let mut selected_theme = PRESETS.iter().position(|p| p.name.eq_ignore_ascii_case(theme_name)).unwrap_or(0);
    let mut accent = Color::parse(accent).unwrap_or(theme.accent);

    dialog.label("Theme", (16.0, 10.0, 200.0, 22.0));
    for (i, preset) in PRESETS.iter().enumerate() {
        let y = 36.0 + i as f64 * ROW_HEIGHT;
        dialog.button(preset.name, ROW_ID + i, (16.0, y), (WIDTH - 32.0, ROW_HEIGHT - 4.0), Look::ThemeRow { preset: i, selected: i == selected_theme });
    }
    let accent_top = 36.0 + PRESETS.len() as f64 * ROW_HEIGHT + 8.0;
    dialog.label("Accent", (16.0, accent_top, 200.0, 22.0));
    for (i, color) in ACCENT_SWATCHES.iter().enumerate() {
        let x = 16.0 + i as f64 * SWATCH;
        dialog.button("", SWATCH_ID + i, (x, accent_top + 24.0), (SWATCH - 4.0, SWATCH - 4.0), Look::Swatch { color: *color, selected: *color == accent });
    }
    let custom_top = accent_top + 24.0 + SWATCH + 10.0;
    dialog.label("Custom", (16.0, custom_top + 4.0, 60.0, 22.0));
    let hex = dialog.control(w!("EDIT"), &accent.hex(), WS_BORDER | WS_TABSTOP, (80.0, custom_top, 110.0, 26.0), 0);
    dialog.button("Apply", APPLY_HEX, (198.0, custom_top - 1.0), (86.0, 28.0), Look::Push { primary: false });
    dialog.button("Close", OK, (WIDTH - 102.0, height - 44.0), (86.0, 28.0), Look::Push { primary: true });

    loop {
        let pressed = dialog.run(|msg| {
            // Enter in the hex box applies it rather than closing the dialog.
            if msg.message == WM_KEYDOWN && msg.wParam.0 as u16 == VK_RETURN.0 && unsafe { GetFocus() } == hex {
                super::dialogs::press(APPLY_HEX);
                return true;
            }
            unsafe { IsDialogMessageW(dialog.hwnd, msg).as_bool() }
        });
        let change = match pressed {
            OK | CANCEL => break,
            id if (ROW_ID..ROW_ID + PRESETS.len()).contains(&id) => {
                selected_theme = id - ROW_ID;
                Change::Theme(PRESETS[selected_theme].name)
            }
            id if (SWATCH_ID..SWATCH_ID + ACCENT_SWATCHES.len()).contains(&id) => {
                accent = ACCENT_SWATCHES[id - SWATCH_ID];
                Change::Accent(accent.hex())
            }
            APPLY_HEX => match Color::parse(&window_text(hex)) {
                Some(c) => {
                    accent = c;
                    Change::Accent(c.hex())
                }
                None => {
                    set_text(hex, &accent.hex());
                    continue;
                }
            },
            _ => continue,
        };
        let theme = apply(change);
        set_text(hex, &accent.hex());
        for i in 0..PRESETS.len() {
            dialog.set_look(ROW_ID + i, Look::ThemeRow { preset: i, selected: i == selected_theme });
        }
        for (i, color) in ACCENT_SWATCHES.iter().enumerate() {
            dialog.set_look(SWATCH_ID + i, Look::Swatch { color: *color, selected: *color == accent });
        }
        dialog.set_theme(theme);
    }
    dialog.finish(());
}
