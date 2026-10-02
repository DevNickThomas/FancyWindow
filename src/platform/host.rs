//! Takes over other applications' windows and gives them back unchanged.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::Shell::{ITaskbarList, TaskbarList};
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::app::WindowId;
use crate::model::Rect;

/// Removed while hosted: resize border and min/max buttons, so the window
/// can't be resized or maximised out of its zone.
const STRIPPED_STYLE: isize = (WS_THICKFRAME.0 | WS_MINIMIZEBOX.0 | WS_MAXIMIZEBOX.0) as isize;

/// How a window looked before we hosted it.
#[derive(Clone, Copy)]
struct Original {
    rect: RECT,
    style: isize,
    ex_style: isize,
}

thread_local! {
    static ORIGINALS: RefCell<HashMap<WindowId, Original>> = RefCell::new(HashMap::new());
    static TASKBAR: OnceCell<Option<ITaskbarList>> = const { OnceCell::new() };
}

fn hwnd(window: WindowId) -> HWND {
    HWND(window.0 as *mut c_void)
}

pub fn is_alive(window: WindowId) -> bool {
    unsafe { IsWindow(Some(hwnd(window))).as_bool() }
}

/// Strips the frame, moves the window into `rect`, and makes `anchor` its owner so it
/// stays above Fancy Window. The window keeps its taskbar button.
pub fn host(anchor: HWND, window: WindowId, rect: Rect) {
    if !is_alive(window) {
        return;
    }
    let h = hwnd(window);
    let known = ORIGINALS.with_borrow(|o| o.contains_key(&window));
    if !known {
        let original = unsafe {
            let mut r = RECT::default();
            let _ = GetWindowRect(h, &mut r);
            Original { rect: r, style: GetWindowLongPtrW(h, GWL_STYLE), ex_style: GetWindowLongPtrW(h, GWL_EXSTYLE) }
        };
        ORIGINALS.with_borrow_mut(|o| o.insert(window, original));
        unsafe { SetWindowLongPtrW(h, GWL_STYLE, original.style & !STRIPPED_STYLE) };
    }
    set_pos(h, rect, SWP_FRAMECHANGED);
    unsafe {
        // Owned windows leave the taskbar; APPWINDOW (and no TOOLWINDOW) asks to stay.
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
        SetWindowLongPtrW(h, GWL_EXSTYLE, (ex | WS_EX_APPWINDOW.0 as isize) & !(WS_EX_TOOLWINDOW.0 as isize));
        SetWindowLongPtrW(h, GWLP_HWNDPARENT, anchor.0 as isize);
    }
    raise(anchor, window);
    add_taskbar_tab(h);
}

/// Moves a hosted window without waiting for it to respond (keeps splitter drags smooth).
pub fn place(anchor: HWND, window: WindowId, rect: Rect) {
    set_pos(hwnd(window), rect, SWP_ASYNCWINDOWPOS);
    raise(anchor, window);
}

/// Restores style, owner and original position.
pub fn release(window: WindowId) {
    if let Some(original) = restore(window) {
        let r = original.rect;
        let rect = Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64);
        set_pos(hwnd(window), rect, SWP_FRAMECHANGED);
    }
}

/// Restores style and owner but leaves the window where it is.
pub fn forget(window: WindowId) {
    if restore(window).is_some() {
        unsafe {
            let _ = SetWindowPos(hwnd(window), None, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED);
        }
    }
}

/// Puts the hosted window just above the anchor in the z-order.
pub fn raise(anchor: HWND, window: WindowId) {
    unsafe {
        let _ = SetWindowPos(hwnd(window), Some(anchor), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_ASYNCWINDOWPOS);
    }
}

fn restore(window: WindowId) -> Option<Original> {
    let original = ORIGINALS.with_borrow_mut(|o| o.remove(&window))?;
    if !is_alive(window) {
        return None;
    }
    let h = hwnd(window);
    unsafe {
        SetWindowLongPtrW(h, GWLP_HWNDPARENT, 0);
        SetWindowLongPtrW(h, GWL_STYLE, original.style);
        SetWindowLongPtrW(h, GWL_EXSTYLE, original.ex_style);
    }
    // Explorer doesn't always re-add an un-owned window by itself.
    add_taskbar_tab(h);
    Some(original)
}

/// SWP_NOSENDCHANGING stops consoles snapping our size to their character grid.
fn set_pos(h: HWND, r: Rect, extra: SET_WINDOW_POS_FLAGS) {
    let (x, y) = (r.x.round() as i32, r.y.round() as i32);
    let (w, hgt) = ((r.right().round() as i32) - x, (r.bottom().round() as i32) - y);
    unsafe {
        let _ = SetWindowPos(h, None, x, y, w, hgt, SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOSENDCHANGING | extra);
    }
}

/// Best effort: the shell can be mid-restart, and a missing tab is not worth failing over.
fn add_taskbar_tab(h: HWND) {
    TASKBAR.with(|cell| {
        let taskbar = cell.get_or_init(|| unsafe {
            let list: ITaskbarList = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER).ok()?;
            list.HrInit().ok()?;
            Some(list)
        });
        if let Some(list) = taskbar {
            let _ = unsafe { list.AddTab(h) };
        }
    });
}
