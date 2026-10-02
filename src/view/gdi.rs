//! Thin wrappers over GDI drawing calls.

use windows::Win32::Foundation::{COLORREF, RECT, SIZE};
use windows::Win32::Graphics::Gdi::*;
use windows::core::w;

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
        DrawTextW(hdc, &mut wide, &mut rc, align | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
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

/// Runs `f` with Segoe UI at `px_height` selected into the DC.
fn with_font(hdc: HDC, px_height: i32, f: impl FnOnce()) {
    unsafe {
        let font = CreateFontW(
            -px_height, 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
            DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY,
            0, w!("Segoe UI"),
        );
        let old_font = SelectObject(hdc, font.into());
        f();
        SelectObject(hdc, old_font);
        let _ = DeleteObject(font.into());
    }
}
