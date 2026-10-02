//! Native popup menus built from `app::MenuItem` data, plus the small dialogs.

use std::path::Path;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, w};

use crate::app::{MenuAction, MenuItem};

/// Shows `items` at a screen point; returns the chosen action, or `None` if dismissed.
pub fn pick(hwnd: HWND, items: &[MenuItem], at: POINT) -> Option<MenuAction> {
    let mut actions = Vec::new();
    unsafe {
        let menu = CreatePopupMenu().ok()?;
        build(menu, items, &mut actions);
        let flags = TPM_RETURNCMD | TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RIGHTBUTTON;
        let chosen = TrackPopupMenuEx(menu, flags.0, at.x, at.y, hwnd, None).0 as usize;
        let _ = DestroyMenu(menu);
        chosen.checked_sub(1).map(|i| actions[i])
    }
}

/// Appends items; each action's command id is its index in `actions` plus one.
fn build(menu: HMENU, items: &[MenuItem], actions: &mut Vec<MenuAction>) {
    for item in items {
        unsafe {
            let _ = match item {
                MenuItem::Separator => AppendMenuW(menu, MF_SEPARATOR, 0, None),
                MenuItem::Note(text) => AppendMenuW(menu, MF_STRING | MF_GRAYED, 0, &HSTRING::from(text.as_str())),
                MenuItem::Item { label, shortcut, enabled, action } => {
                    actions.push(*action);
                    let text = match shortcut {
                        Some(keys) => format!("{label}\t{keys}"),
                        None => label.clone(),
                    };
                    let flags = if *enabled { MF_STRING } else { MF_STRING | MF_GRAYED };
                    AppendMenuW(menu, flags, actions.len(), &HSTRING::from(text))
                }
                MenuItem::Submenu { label, items } => {
                    let Ok(sub) = CreatePopupMenu() else { continue };
                    build(sub, items, actions);
                    AppendMenuW(menu, MF_POPUP, sub.0 as usize, &HSTRING::from(label.as_str()))
                }
            };
        }
    }
}

pub fn open_folder(hwnd: HWND, folder: &Path) {
    unsafe {
        ShellExecuteW(Some(hwnd), w!("open"), &HSTRING::from(folder.as_os_str()), None, None, SW_SHOWNORMAL);
    }
}

pub fn show_about(hwnd: HWND, settings: &Path, crash_log: &Path) {
    let text = format!(
        "Fancy Window v{}\nA lightweight tiling host for Windows.\n\nSettings:\n{}\n\nCrash log:\n{}",
        env!("CARGO_PKG_VERSION"),
        settings.display(),
        crash_log.display(),
    );
    message(hwnd, &text, "About Fancy Window");
}

fn message(hwnd: HWND, text: &str, caption: &str) {
    unsafe {
        MessageBoxW(Some(hwnd), &HSTRING::from(text), &HSTRING::from(caption), MB_OK | MB_ICONINFORMATION);
    }
}
