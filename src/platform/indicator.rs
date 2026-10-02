//! The "Fancy Window is behind" reminder shown in stay-back mode. Always on top,
//! never takes focus; clicking it brings Fancy Window back.

use std::cell::{Cell, RefCell};

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use crate::model::Rect;
use crate::view::{Theme, draw_reminder};

/// Posted to the main window when the reminder is clicked.
pub const WM_BRING_BACK: u32 = WM_APP + 1;

/// Wide enough for a long chord, e.g. "Ctrl+Win+Alt+PageUp".
const SIZE: (f64, f64) = (440.0, 32.0);
const SCREEN_GAP: f64 = 16.0;

thread_local! {
    static INDICATOR: Cell<Option<HWND>> = const { Cell::new(None) };
    static OWNER: Cell<Option<HWND>> = const { Cell::new(None) };
    static THEME: Cell<Option<Theme>> = const { Cell::new(None) };
    static TEXT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// `bring_back` is the bring-forward chord as people read it, if it has one.
pub fn show(main: HWND, theme: Theme, bring_back: Option<String>) {
    TEXT.set(match bring_back {
        Some(chord) => format!("Fancy Window is behind  \u{00B7}  click or {chord} to bring back"),
        None => "Fancy Window is behind  \u{00B7}  click to bring back".into(),
    });
    THEME.set(Some(theme));
    if INDICATOR.get().is_some() {
        return;
    }
    OWNER.set(Some(main));
    let scale = unsafe { GetDpiForSystem() } as f64 / 96.0;
    let (w, h) = ((SIZE.0 * scale) as i32, (SIZE.1 * scale) as i32);
    let gap = (SCREEN_GAP * scale) as i32;
    let mut work = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut work as *mut _ as *mut _), Default::default());
        let instance = GetModuleHandleW(None).expect("module handle");
        let class = WNDCLASSW {
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_HAND).unwrap_or_default(),
            lpszClassName: w!("FancyWindowIndicator"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            w!("FancyWindowIndicator"),
            w!("Fancy Window - behind"),
            WS_POPUP,
            work.right - w - gap,
            work.bottom - h - gap,
            w,
            h,
            None,
            None,
            Some(instance.into()),
            None,
        )
        .ok();
        if let Some(hwnd) = hwnd {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
        INDICATOR.set(hwnd);
    }
}

pub fn hide() {
    if let Some(hwnd) = INDICATOR.take() {
        unsafe {
            let _ = DestroyWindow(hwnd);
        }
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
            WM_LBUTTONDOWN => {
                if let Some(main) = OWNER.get() {
                    let _ = PostMessageW(Some(main), WM_BRING_BACK, WPARAM(0), LPARAM(0));
                }
                LRESULT(0)
            }
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                let mut r = RECT::default();
                let _ = GetClientRect(hwnd, &mut r);
                let scale = GetDpiForSystem() as f64 / 96.0;
                let text = TEXT.with_borrow(String::clone);
                draw_reminder(hdc, Rect::new(0.0, 0.0, r.right as f64, r.bottom as f64), &text, &THEME.get().unwrap_or_else(Theme::dark), scale);
                let _ = EndPaint(hwnd, &ps);
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

/// Repaints in a new theme if the reminder is showing.
pub fn set_theme(theme: Theme) {
    THEME.set(Some(theme));
    if let Some(hwnd) = INDICATOR.get() {
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, true);
        }
    }
}
