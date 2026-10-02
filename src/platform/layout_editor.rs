//! Temporary input overlay above hosted apps. No permanent pane chrome, no
//! reparenting or hiding of hosted windows, and no input sent to their clients.
use std::cell::RefCell;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use crate::app::{AppState, MenuAction, WindowId, visible_rect, zone_menu};
use crate::model::{JoinDirection, Orientation, Point, Rect, ZoneId};
use crate::view::{Theme, theme_of};
use crate::view::gdi::{self, Align};
use super::menus;

#[derive(Clone, Copy)]
enum Action { Split(Orientation), Menu, Done }

struct Editor {
    state: AppState,
    selected: ZoneId,
    pending: Option<Action>,
    done: bool,
    refocus: bool,
}

thread_local! { static EDITOR: RefCell<Option<Editor>> = const { RefCell::new(None) }; }

/// The callback uses the normal update/effects path, keeping hosting and layout
/// invariants identical to the existing menus. `None` just refreshes the snapshot.
pub fn show(owner: HWND, state: AppState, mut apply: impl FnMut(Option<MenuAction>) -> AppState) -> Option<WindowId> {
    if EDITOR.with_borrow(|e| e.is_some()) {
        with(|e| { e.done = true; e.refocus = true; });
        return None;
    }
    let mut frame = state.frame;
    let selected = state.editing_zone();
    EDITOR.set(Some(Editor { state, selected, pending: None, done: false, refocus: false }));
    unsafe {
        let instance = GetModuleHandleW(None).expect("module handle");
        RegisterClassW(&WNDCLASSW {
            lpfnWndProc: Some(wndproc), hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("FancyWindowLayoutEditor"), ..Default::default()
        });
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_LAYERED,
            w!("FancyWindowLayoutEditor"), w!("Edit layout"), WS_POPUP,
            frame.origin.x as i32, frame.origin.y as i32,
            (frame.canvas.width * frame.scale) as i32,
            (frame.canvas.height * frame.scale) as i32,
            Some(owner), None, Some(instance.into()), None,
        ) else { EDITOR.set(None); return None; };
        // Keep the hosted app visible underneath while this window owns input.
        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 222, LWA_ALPHA);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(hwnd));
        let mut msg = MSG::default();
        while !with(|e| e.done) {
            if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                PostQuitMessage(msg.wParam.0 as i32);
                break;
            }
            // Our shortcuts are handled on key-down. Posting WM_CHAR for J here
            // would feed that same J into the context menu's mnemonic handling.
            if msg.hwnd != hwnd { let _ = TranslateMessage(&msg); }
            DispatchMessageW(&msg);
            let pending = with(|e| e.pending.take());
            let selected = with(|e| e.selected);
            let selected_window = with(|e| e.state.attachments.iter().find(|a| a.zone == selected).map(|a| a.window));
            let action = match pending {
                Some(Action::Split(orientation)) => Some(MenuAction::SplitZone(selected, orientation)),
                Some(Action::Menu) => {
                    let (items, at) = with(|e| {
                        let r = e.state.zone_rects().into_iter().find(|z| z.id == selected).map(|z| z.bounds).unwrap_or(e.state.frame.canvas);
                        let r = e.state.frame.to_screen(r);
                        (zone_menu(&e.state, selected), POINT { x: (r.x + r.width / 2.0) as i32, y: (r.y + r.height / 2.0) as i32 })
                    });
                    menus::pick(hwnd, &items, at)
                }
                Some(Action::Done) => { with(|e| { e.done = true; e.refocus = true; }); None }
                None => None,
            };
            // Owner destruction also destroys this owned popup. Do not access its shell.
            if !IsWindow(Some(owner)).as_bool() { break; }
            let state = apply(action);
            with(|e| {
                if !state.layout.leaves().contains(&e.selected) {
                    e.selected = selected_window.and_then(|w| state.zone_of(w)).unwrap_or_else(|| state.editing_zone());
                }
                e.state = state;
            });
            let next_frame = with(|e| e.state.frame);
            if next_frame != frame {
                frame = next_frame;
                let _ = SetWindowPos(hwnd, None, frame.origin.x as i32, frame.origin.y as i32,
                    (frame.canvas.width * frame.scale) as i32, (frame.canvas.height * frame.scale) as i32,
                    SWP_NOACTIVATE | SWP_NOZORDER);
            }
            if msg.message != WM_PAINT { let _ = InvalidateRect(Some(hwnd), None, false); }
        }
        let focus = with(|e| e.refocus.then(|| e.state.attachments.iter().find(|a| a.zone == e.selected).map(|a| a.window)).flatten());
        let _ = DestroyWindow(hwnd);
        EDITOR.set(None);
        focus
    }
}

fn with<R>(f: impl FnOnce(&mut Editor) -> R) -> R {
    EDITOR.with_borrow_mut(|e| f(e.as_mut().expect("layout editor is open")))
}

fn controls(canvas: Rect) -> [(Rect, &'static str, Action); 4] {
    let width = ((canvas.width - 40.0) / 4.0).max(1.0);
    let actions = [
        ("Columns · V", Action::Split(Orientation::Columns)),
        ("Rows · H", Action::Split(Orientation::Rows)),
        ("Split / join · J", Action::Menu),
        ("Done · Esc", Action::Done),
    ];
    std::array::from_fn(|i| (Rect::new(8.0 + i as f64 * (width + 8.0), canvas.height - 38.0, width, 30.0), actions[i].0, actions[i].1))
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => paint(hwnd),
            WM_ERASEBKGND => return LRESULT(1),
            WM_CLOSE => with(|e| { e.done = true; e.refocus = true; }),
            WM_ACTIVATE if (wparam.0 & 0xffff) as u32 == WA_INACTIVE => with(|e| e.done = true),
            WM_KEYDOWN => with(|e| match VIRTUAL_KEY(wparam.0 as u16) {
                VK_ESCAPE | VK_RETURN => e.pending = Some(Action::Done),
                VK_TAB => e.selected = e.state.next_editing_zone(e.selected, GetKeyState(VK_SHIFT.0 as i32) >= 0),
                VK_LEFT => e.selected = e.state.adjacent_editing_zone(e.selected, JoinDirection::Left),
                VK_RIGHT => e.selected = e.state.adjacent_editing_zone(e.selected, JoinDirection::Right),
                VK_UP => e.selected = e.state.adjacent_editing_zone(e.selected, JoinDirection::Up),
                VK_DOWN => e.selected = e.state.adjacent_editing_zone(e.selected, JoinDirection::Down),
                VK_V => e.pending = Some(Action::Split(Orientation::Columns)),
                VK_H => e.pending = Some(Action::Split(Orientation::Rows)),
                VK_J | VK_APPS => e.pending = Some(Action::Menu),
                _ => {},
            }),
            WM_LBUTTONDOWN | WM_RBUTTONDOWN => with(|e| {
                let at = Point::new((lparam.0 & 0xffff) as i16 as f64 / e.state.frame.scale, (lparam.0 >> 16) as i16 as f64 / e.state.frame.scale);
                if let Some((_, _, action)) = controls(e.state.frame.canvas).into_iter().find(|(r, _, _)| r.contains(at)) {
                    if message == WM_LBUTTONDOWN { e.pending = Some(action); }
                } else if let Some(zone) = e.state.layout.hit_test(e.state.frame.canvas, at) {
                    e.selected = zone;
                    if message == WM_RBUTTONDOWN { e.pending = Some(Action::Menu); }
                }
            }),
            _ => return DefWindowProcW(hwnd, message, wparam, lparam),
        }
        LRESULT(0)
    }
}

fn paint(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let dc = BeginPaint(hwnd, &mut ps);
        with(|e| {
            let scale = e.state.frame.scale;
            let px = |r: Rect| Rect::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale);
            let t: Theme = theme_of(&e.state);
            let canvas = e.state.frame.canvas;
            gdi::fill(dc, px(canvas), t.window_bg);
            for zone in e.state.zone_rects() {
                let selected = zone.id == e.selected;
                let r = visible_rect(zone.bounds, canvas);
                if selected {
                    gdi::fill(dc, px(r), t.active_glow);
                } else {
                    gdi::outline(dc, px(r), t.divider, 1, 4);
                }
                let label = e.state.editing_zone_label(zone.id);
                let label_rect = Rect::new(r.x + 8.0, r.y + 8.0, (r.width - 16.0).max(0.0), 30.0);
                gdi::text(dc, px(label_rect), &label, if selected { t.text } else { t.muted }, (14.0 * scale) as i32, Align::Left);
            }
            let footer = Rect::new(0.0, (canvas.height - 96.0).max(0.0), canvas.width, 96.0);
            gdi::fill(dc, px(footer), t.popup_bg);
            let label = format!("Editing {}", e.state.editing_zone_label(e.selected));
            gdi::text(dc, px(Rect::new(10.0, footer.y + 2.0, canvas.width - 20.0, 26.0)), &label, t.text, (14.0 * scale) as i32, Align::Left);
            gdi::text(dc, px(Rect::new(10.0, footer.y + 28.0, canvas.width - 20.0, 22.0)), "Click a pane or use arrows / Tab to select. Changes apply immediately.", t.muted, (12.0 * scale) as i32, Align::Left);
            for (r, text, action) in controls(canvas) {
                let primary = matches!(action, Action::Done);
                gdi::rounded(dc, px(r), if primary { t.accent } else { t.bar_bg }, t.divider, 4);
                gdi::text(dc, px(r), text, if primary { t.status_text } else { t.text }, (13.0 * scale) as i32, Align::Center);
            }
        });
        let _ = EndPaint(hwnd, &ps);
    }
}
