//! Takes over other applications' windows and gives them back unchanged.

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, RECT, WPARAM};
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
    /// Where the window being dragged was when the drag began. Windows are hosted on
    /// the drop, by when they already sit over Fancy Window; this is where they came from.
    static MOVE_START: RefCell<Option<(WindowId, RECT)>> = const { RefCell::new(None) };
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
        let before_drag = MOVE_START.with_borrow(|s| s.filter(|(w, _)| *w == window).map(|(_, r)| r));
        let original = unsafe {
            let mut r = RECT::default();
            let _ = GetWindowRect(h, &mut r);
            Original { rect: before_drag.unwrap_or(r), style: GetWindowLongPtrW(h, GWL_STYLE), ex_style: GetWindowLongPtrW(h, GWL_EXSTYLE) }
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

/// The window's title as the system last saw it. Unlike GetWindowText this sends the
/// window no message, so a hung app can't stall Fancy Window.
pub fn title(window: WindowId) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { InternalGetWindowText(hwnd(window), &mut buffer) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

/// The window's small icon: asked of the window (giving up after 100 ms), then its class.
/// The handle belongs to the window or its class; it is never destroyed here.
pub fn icon(window: WindowId) -> Option<isize> {
    const ICON_SMALL: usize = 0;
    const ICON_BIG: usize = 1;
    const ICON_SMALL2: usize = 2;
    let h = hwnd(window);
    for kind in [ICON_SMALL2, ICON_SMALL, ICON_BIG] {
        let mut result = 0usize;
        let answered = unsafe {
            SendMessageTimeoutW(h, WM_GETICON, WPARAM(kind), LPARAM(0), SMTO_ABORTIFHUNG | SMTO_BLOCK, 100, Some(&mut result))
        };
        if answered.0 != 0 && result != 0 {
            return Some(result as isize);
        }
    }
    [GCLP_HICONSM, GCLP_HICON].into_iter().map(|i| unsafe { GetClassLongPtrW(h, i) }).find(|&v| v != 0).map(|v| v as isize)
}

/// A window started moving: remember where it was (see `MOVE_START`).
pub fn move_started(window: WindowId) {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd(window), &mut r);
    }
    MOVE_START.set(Some((window, r)));
}

/// The move is over, hosted or not.
pub fn move_ended() {
    MOVE_START.set(None);
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
