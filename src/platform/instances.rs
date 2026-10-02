//! Starting and reopening named host windows without touching another profile.

use std::hash::{Hash, Hasher};
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, POINT};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, HSTRING};

use crate::app::{MenuAction, MenuItem};
use crate::model::Settings;
use super::{dialogs, menus, placement, storage};

fn marker(path: &Path) -> HSTRING {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.to_string_lossy().to_lowercase().hash(&mut hash);
    HSTRING::from(format!("FancyWindow.Settings.{:016x}", hash.finish()))
}

/// Keep the kernel object alive for the process lifetime. This closes the gap
/// between launching a profile and its main window becoming discoverable.
pub struct ProfileGuard(HANDLE);

impl Drop for ProfileGuard {
    fn drop(&mut self) { unsafe { let _ = CloseHandle(self.0); } }
}

pub fn claim(path: &Path) -> windows::core::Result<Option<ProfileGuard>> {
    let name = HSTRING::from(format!("Local\\{}", marker(path)));
    unsafe {
        let handle = CreateMutexW(None, false, &name)?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let _ = CloseHandle(handle);
            focus_existing(path);
            return Ok(None);
        }
        Ok(Some(ProfileGuard(handle)))
    }
}

pub fn mark(hwnd: HWND, path: &Path) {
    unsafe { let _ = SetPropW(hwnd, &marker(path), Some(HANDLE(std::ptr::without_provenance_mut(1)))); }
}

pub fn unmark(hwnd: HWND, path: &Path) {
    unsafe { let _ = RemovePropW(hwnd, &marker(path)); }
}

/// Reopening an already-running profile brings it forward instead of starting a
/// second writer for the same settings file.
pub fn focus_existing(path: &Path) -> bool {
    struct Search { marker: HSTRING, found: HWND }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
        unsafe {
            let search = &mut *(data.0 as *mut Search);
            if !GetPropW(hwnd, &search.marker).is_invalid() {
                search.found = hwnd;
                return false.into();
            }
        }
        true.into()
    }
    let mut search = Search { marker: marker(path), found: HWND::default() };
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut search as *mut Search as isize));
        if search.found.is_invalid() { return false; }
        if IsIconic(search.found).as_bool() { let _ = ShowWindow(search.found, SW_RESTORE); }
        let _ = SetForegroundWindow(search.found);
    }
    true
}

fn launch(profile: Option<&str>) -> std::io::Result<()> {
    let mut child = std::process::Command::new(std::env::current_exe()?);
    if let Some(profile) = profile { child.arg("--profile").arg(profile); }
    child.spawn()?;
    Ok(())
}

pub fn create(hwnd: HWND, mut settings: Settings) {
    settings.set_bounds(placement::for_new_window(hwnd));
    let result = storage::create_window_profile(&storage::exe_dir(), &settings)
        .and_then(|profile| launch(Some(&profile)));
    if let Err(error) = result { dialogs::warn(hwnd, "Could not open window", &error.to_string()); }
}

pub fn pick(hwnd: HWND) {
    let windows = match storage::saved_windows(&storage::exe_dir()) {
        Ok(windows) => windows,
        Err(error) => { dialogs::warn(hwnd, "Could not read saved windows", &error.to_string()); return; }
    };
    let mut items: Vec<MenuItem> = windows.iter().enumerate().map(|(i, window)| {
        // Names are user text, so escape native menu mnemonic markers.
        let duplicates = windows.iter().filter(|w| w.name.eq_ignore_ascii_case(&window.name)).count() > 1;
        let label = if duplicates {
            format!("{} ({})", window.name, window.profile.as_deref().unwrap_or("default"))
        } else { window.name.clone() };
        MenuItem::Item { label: label.replace('&', "&&"), shortcut: None, enabled: true, action: MenuAction::OpenSavedInstance(i) }
    }).collect();
    if items.is_empty() { items.push(MenuItem::Note("No saved windows yet".into())); }
    let mut at = POINT::default();
    unsafe { let _ = GetCursorPos(&mut at); }
    if let Some(MenuAction::OpenSavedInstance(i)) = menus::pick(hwnd, &items, at) {
        let window = &windows[i];
        let path = storage::settings_path(window.profile.as_deref());
        if !focus_existing(&path) {
            if let Err(error) = launch(window.profile.as_deref()) {
                dialogs::warn(hwnd, "Could not open window", &error.to_string());
            }
        }
    }
}
