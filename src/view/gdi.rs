//! Thin wrappers over GDI drawing calls.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::OnceLock;

use windows::Win32::Foundation::{COLORREF, LPARAM, RECT, SIZE};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DrawIconEx, HICON};
use windows::core::{PCWSTR, w};

use super::theme::Color;
use crate::model::Rect;

pub fn colorref(c: Color) -> COLORREF {
    COLORREF(c.0 as u32 | (c.1 as u32) << 8 | (c.2 as u32) << 16)
}

pub fn to_rect(r: Rect) -> RECT {
    RECT {
        left: r.x.round() as i32,
        top: r.y.round() as i32,
        right: r.right().round() as i32,
        bottom: r.bottom().round() as i32,
    }
}

pub fn fill(hdc: HDC, r: Rect, color: Color) {
    unsafe {
        let brush = CreateSolidBrush(colorref(color));
        FillRect(hdc, &to_rect(r), brush);
        let _ = DeleteObject(brush.into());
    }
}

/// An unfilled rounded rectangle outline `width` pixels thick.
pub fn outline(hdc: HDC, r: Rect, color: Color, width: i32, radius: i32) {
    let rc = to_rect(r);
    unsafe {
        let pen = CreatePen(PS_INSIDEFRAME, width, colorref(color));
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let old_pen = SelectObject(hdc, pen.into());
        let _ = RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius * 2, radius * 2);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen.into());
    }
}

/// A filled rounded rectangle with a 1px border.
pub fn rounded(hdc: HDC, r: Rect, fill: Color, border: Color, radius: i32) {
    let rc = to_rect(r);
    unsafe {
        let brush = CreateSolidBrush(colorref(fill));
        let pen = CreatePen(PS_SOLID, 1, colorref(border));
        let old_brush = SelectObject(hdc, brush.into());
        let old_pen = SelectObject(hdc, pen.into());
        let _ = RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius * 2, radius * 2);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(brush.into());
        let _ = DeleteObject(pen.into());
    }
}

/// A straight line `width` pixels thick.
pub fn line(hdc: HDC, from: (i32, i32), to: (i32, i32), color: Color, width: i32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, width, colorref(color));
        let old_pen = SelectObject(hdc, pen.into());
        let _ = MoveToEx(hdc, from.0, from.1, None);
        let _ = LineTo(hdc, to.0, to.1);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen.into());
    }
}

/// Draws an icon handle (owned by someone else) scaled to `size` pixels.
pub fn icon(hdc: HDC, x: i32, y: i32, size: i32, handle: isize) {
    unsafe {
        let _ = DrawIconEx(hdc, x, y, HICON(handle as *mut c_void), size, size, 0, None, DI_NORMAL);
    }
}

#[derive(Clone, Copy)]
pub enum Align {
    Left,
    Center,
    Right,
}

/// Single-line, vertically centred text.
pub fn text(hdc: HDC, r: Rect, s: &str, color: Color, px_height: i32, align: Align) {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    let mut rc = to_rect(r);
    let align = match align {
        Align::Left => DT_LEFT,
        Align::Center => DT_CENTER,
        Align::Right => DT_RIGHT,
    };
    with_font(hdc, px_height, || unsafe {
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, colorref(color));
        DrawTextW(hdc, &mut wide, &mut rc, align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS);
    });
}

/// Width in pixels of `s` in the UI font.
pub fn text_width(hdc: HDC, s: &str, px_height: i32) -> i32 {
    let wide: Vec<u16> = s.encode_utf16().collect();
    let mut size = SIZE::default();
    with_font(hdc, px_height, || unsafe {
        let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
    });
    size.cx
}

/// The UI font: Segoe UI Variable (Windows 11) where installed, else Segoe UI.
/// Asking GDI for a missing face silently gives a different font, so check once.
pub fn ui_face() -> PCWSTR {
    static VARIABLE: OnceLock<bool> = OnceLock::new();
    if *VARIABLE.get_or_init(|| font_installed("Segoe UI Variable Text")) { w!("Segoe UI Variable Text") } else { w!("Segoe UI") }
}

fn font_installed(face: &str) -> bool {
    unsafe extern "system" fn found(_: *const LOGFONTW, _: *const TEXTMETRICW, _: u32, seen: LPARAM) -> i32 {
        unsafe { *(seen.0 as *mut bool) = true };
        0
    }
    let mut query = LOGFONTW { lfCharSet: DEFAULT_CHARSET, ..Default::default() };
    for (dst, src) in query.lfFaceName.iter_mut().zip(face.encode_utf16().take(31)) {
        *dst = src;
    }
    let mut seen = false;
    unsafe {
        let hdc = GetDC(None);
        EnumFontFamiliesExW(hdc, &query, Some(found), LPARAM(&mut seen as *mut bool as isize), 0);
        ReleaseDC(None, hdc);
    }
    seen
}

/// Runs `f` with the UI font at `px_height` selected into the DC.
fn with_font(hdc: HDC, px_height: i32, f: impl FnOnce()) {
    with_face(hdc, px_height, ui_face(), f)
}

/// Runs `f` with `face` at `px_height` selected into the DC.
/// Fonts are cached by face and size: a repaint draws dozens of strings, and only a
/// handful of sizes exist (a few more after a DPI change), so they are never freed.
fn with_face(hdc: HDC, px_height: i32, face: PCWSTR, f: impl FnOnce()) {
    thread_local! {
        static FONTS: RefCell<HashMap<(usize, i32), HFONT>> = RefCell::new(HashMap::new());
    }
    // Faces are static w!() literals, so their address identifies them.
    let key = (face.0 as usize, px_height);
    let font = FONTS.with_borrow_mut(|fonts| {
        *fonts.entry(key).or_insert_with(|| unsafe {
            CreateFontW(
                -px_height, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
                DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
                0, face,
            )
        })
    });
    unsafe {
        let old_font = SelectObject(hdc, font.into());
        f();
        SelectObject(hdc, old_font);
    }
}

/// The symbol font for caption-button glyphs: Segoe Fluent Icons (Windows 11), else
/// Segoe MDL2 Assets (Windows 10). Both use the same code points.
fn icon_face() -> PCWSTR {
    static FLUENT: OnceLock<bool> = OnceLock::new();
    if *FLUENT.get_or_init(|| font_installed("Segoe Fluent Icons")) { w!("Segoe Fluent Icons") } else { w!("Segoe MDL2 Assets") }
}

/// One symbol-font glyph centred in `r`.
pub fn glyph(hdc: HDC, r: Rect, glyph: char, color: Color, px_height: i32) {
    let mut wide: Vec<u16> = glyph.to_string().encode_utf16().collect();
    let mut rc = to_rect(r);
    with_face(hdc, px_height, icon_face(), || unsafe {
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, colorref(color));
        DrawTextW(hdc, &mut wide, &mut rc, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    });
}

/// The face for keys and chords: Cascadia Mono (Windows 11, Terminal), else Consolas.
fn mono_face() -> PCWSTR {
    static CASCADIA: OnceLock<bool> = OnceLock::new();
    if *CASCADIA.get_or_init(|| font_installed("Cascadia Mono")) { w!("Cascadia Mono") } else { w!("Consolas") }
}

/// Single-line text in the mono face.
pub fn mono_text(hdc: HDC, r: Rect, s: &str, color: Color, px_height: i32, align: Align) {
    let mut wide: Vec<u16> = s.encode_utf16().collect();
    let mut rc = to_rect(r);
    let align = match align {
        Align::Left => DT_LEFT,
        Align::Center => DT_CENTER,
        Align::Right => DT_RIGHT,
    };
    with_face(hdc, px_height, mono_face(), || unsafe {
        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, colorref(color));
        DrawTextW(hdc, &mut wide, &mut rc, align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
    });
}

pub fn mono_text_width(hdc: HDC, s: &str, px_height: i32) -> i32 {
    let wide: Vec<u16> = s.encode_utf16().collect();
    let mut size = SIZE::default();
    with_face(hdc, px_height, mono_face(), || unsafe {
        let _ = GetTextExtentPoint32W(hdc, &wide, &mut size);
    });
    size.cx
}

/// A dashed rounded outline `width` pixels thick.
pub fn dashed(hdc: HDC, r: Rect, color: Color, width: i32, radius: i32) {
    let rc = to_rect(r);
    let brush = LOGBRUSH { lbStyle: BS_SOLID, lbColor: colorref(color), lbHatch: 0 };
    unsafe {
        let pen = ExtCreatePen(PEN_STYLE(PS_GEOMETRIC.0 | PS_DASH.0 | PS_ENDCAP_FLAT.0), width as u32, &brush, None);
        let old_brush = SelectObject(hdc, GetStockObject(NULL_BRUSH));
        let old_pen = SelectObject(hdc, pen.into());
        let _ = RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, radius * 2, radius * 2);
        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(pen.into());
    }
}
