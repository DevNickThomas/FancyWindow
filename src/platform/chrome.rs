//! Light or dark window chrome (title bars and popup menus) to match the theme.

use std::ffi::c_void;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dwm::{DWMWA_USE_IMMERSIVE_DARK_MODE, DwmSetWindowAttribute};
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::core::{BOOL, PCSTR, w};

use crate::view::Theme;

pub fn title_bar(hwnd: HWND, theme: &Theme) {
    let dark = BOOL::from(!theme.is_light());
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as *const c_void, size_of::<BOOL>() as u32);
    }
}

/// Light or dark popup menus. Uses uxtheme's SetPreferredAppMode (ordinal 135)
/// and FlushMenuThemes (136): undocumented, but stable since Windows 10 1903 and
/// the only way to theme Win32 menus. Missing exports are simply skipped.
pub fn menus(theme: &Theme) {
    const FORCE_DARK: i32 = 2;
    const FORCE_LIGHT: i32 = 3;
    let mode = if theme.is_light() { FORCE_LIGHT } else { FORCE_DARK };
    unsafe {
        let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) else { return };
        if let Some(set_mode) = GetProcAddress(uxtheme, PCSTR(135 as *const u8)) {
            let set_mode: extern "system" fn(i32) -> i32 = std::mem::transmute(set_mode);
            set_mode(mode);
        }
        if let Some(flush) = GetProcAddress(uxtheme, PCSTR(136 as *const u8)) {
            let flush: extern "system" fn() = std::mem::transmute(flush);
            flush();
        }
    }
}
