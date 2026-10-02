//! Restoring and remembering the main window's position and size.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::HiDpi::GetDpiForSystem;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::model::WindowBounds;

const DEFAULT_SIZE: (f64, f64) = (900.0, 600.0);
/// Pixels of the title bar that must stay on screen, or the saved spot is ignored.
const MIN_VISIBLE: i32 = 64;

/// Where to create the window, in pixels: `(x, y, width, height)`.
/// Saved positions that are now off every monitor fall back to the default spot.
pub fn initial(saved: Option<WindowBounds>) -> (i32, i32, i32, i32) {
    let px = |dip: f64| (dip * system_scale()).round() as i32;
    let Some(b) = saved else {
        return (CW_USEDEFAULT, CW_USEDEFAULT, px(DEFAULT_SIZE.0), px(DEFAULT_SIZE.1));
    };
    let (x, y, w, h) = (px(b.left), px(b.top), px(b.width), px(b.height));
    if on_screen(x, y, w, h) { (x, y, w, h) } else { (CW_USEDEFAULT, CW_USEDEFAULT, w, h) }
}

/// The window's normal (unmaximised) bounds in DIPs, and whether it is maximised.
pub fn current(hwnd: HWND) -> WindowBounds {
    let mut placement = WINDOWPLACEMENT { length: size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
    unsafe {
        let _ = GetWindowPlacement(hwnd, &mut placement);
    }
    let r: RECT = placement.rcNormalPosition;
    let dip = |px: i32| px as f64 / system_scale();
    WindowBounds {
        left: dip(r.left),
        top: dip(r.top),
        width: dip(r.right - r.left),
        height: dip(r.bottom - r.top),
        maximized: placement.showCmd == SW_SHOWMAXIMIZED.0 as u32,
    }
}

fn system_scale() -> f64 {
    unsafe { GetDpiForSystem() as f64 / 96.0 }
}

fn on_screen(x: i32, y: i32, w: i32, h: i32) -> bool {
    let metric = |m| unsafe { GetSystemMetrics(m) };
    let (left, top) = (metric(SM_XVIRTUALSCREEN), metric(SM_YVIRTUALSCREEN));
    let (right, bottom) = (left + metric(SM_CXVIRTUALSCREEN), top + metric(SM_CYVIRTUALSCREEN));
    x <= right - MIN_VISIBLE && y <= bottom - MIN_VISIBLE && x + w >= left + MIN_VISIBLE && y + h >= top + MIN_VISIBLE
}
