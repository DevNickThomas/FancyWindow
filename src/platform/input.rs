//! Translates raw Win32 input messages into app `Msg`s. No decisions are made here.

use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT};
use windows::Win32::System::SystemServices::{MK_CONTROL, MK_SHIFT};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::app::{Button, Modifiers, Msg};
use crate::model::Point;

/// Returns the `Msg` for an input message, or `None` if it isn't one we care about.
/// `scale` and `top` (pixels above the canvas) convert client pixels to canvas DIPs.
pub fn translate(message: u32, wparam: WPARAM, lparam: LPARAM, scale: f64, top: f64) -> Option<Msg> {
    let at = point(lparam, scale, top);
    let msg = match message {
        WM_LBUTTONDOWN => Msg::MouseDown { at, button: Button::Left, mods: mouse_mods(wparam) },
        WM_RBUTTONDOWN => Msg::MouseDown { at, button: Button::Right, mods: mouse_mods(wparam) },
        WM_MOUSEMOVE => Msg::MouseMove { at, mods: mouse_mods(wparam) },
        WM_LBUTTONUP => Msg::MouseUp,
        WM_MOUSELEAVE => Msg::MouseLeft,
        WM_CAPTURECHANGED => Msg::CaptureLost,
        WM_KEYDOWN | WM_KEYUP if is_modifier_key(wparam) => Msg::ModifiersChanged(keyboard_mods()),
        _ => return None,
    };
    Some(msg)
}

/// Client coordinates packed in LPARAM, as canvas DIPs (signed: can be negative while captured).
fn point(lparam: LPARAM, scale: f64, top: f64) -> Point {
    let x = (lparam.0 & 0xFFFF) as i16;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16;
    Point::new(x as f64 / scale, (y as f64 - top) / scale)
}

fn mouse_mods(wparam: WPARAM) -> Modifiers {
    let keys = wparam.0 as u32;
    Modifiers { ctrl: keys & MK_CONTROL.0 != 0, shift: keys & MK_SHIFT.0 != 0 }
}

fn keyboard_mods() -> Modifiers {
    let down = |vk: i32| unsafe { GetKeyState(vk) } < 0;
    Modifiers { ctrl: down(VK_CONTROL.0 as i32), shift: down(VK_SHIFT.0 as i32) }
}

fn is_modifier_key(wparam: WPARAM) -> bool {
    let vk = wparam.0 as u16;
    vk == VK_CONTROL.0 || vk == VK_SHIFT.0
}
