use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

use super::gdi::{self, Align};
use super::{MENU_BAR_HEIGHT, STATUS_BAR_HEIGHT, Theme, canvas_top_px, theme_of, to_px};
use crate::app::AppState;
use crate::model::Rect;

const ZONE_RADIUS: f64 = 4.0;
const BAR_FONT: f64 = 15.0;
const STATUS_PADDING: f64 = 10.0;
const MENU_TITLE_PADDING: f64 = 9.0;
const HIGHLIGHT_WIDTH: f64 = 2.0;
const REMINDER_FONT: f64 = 14.0;
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Paints the whole client area, double-buffered to avoid flicker.
/// `open_menu` is the menu-bar title to show pressed.
pub fn paint(hwnd: HWND, state: &AppState, titles: &[&str], open_menu: Option<usize>) {
    let theme = &theme_of(state);
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = Default::default();
        let _ = GetClientRect(hwnd, &mut client);
        let (w, h) = (client.right, client.bottom);

        let buffer = CreateCompatibleDC(Some(hdc));
        let bitmap = CreateCompatibleBitmap(hdc, w, h);
        let old = SelectObject(buffer, bitmap.into());

        let client = Rect::new(0.0, 0.0, w as f64, h as f64);
        gdi::fill(buffer, client, theme.window_bg);
        draw_canvas(buffer, state, theme);
        draw_menu_bar(buffer, client, state.frame.scale, theme, titles, open_menu);
        draw_status_bar(buffer, client, state, theme);

        let _ = BitBlt(hdc, 0, 0, w, h, Some(buffer), 0, 0, SRCCOPY);
        SelectObject(buffer, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(buffer);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Zones, splitters, split preview and cycle highlight, shifted below the menu bar.
fn draw_canvas(hdc: HDC, state: &AppState, theme: &Theme) {
    let scale = state.frame.scale;
    let mut previous = POINT::default();
    unsafe {
        let _ = SetViewportOrgEx(hdc, 0, canvas_top_px(scale) as i32, Some(&mut previous));
    }
    let radius = (ZONE_RADIUS * scale).round() as i32;
    for zone in state.zone_rects() {
        gdi::rounded(hdc, to_px(zone.bounds, scale), theme.zone_fill, theme.zone_border, radius);
    }
    for splitter in state.splitters() {
        gdi::fill(hdc, to_px(splitter.bounds, scale), theme.splitter);
    }
    if let Some(line) = state.split_preview() {
        gdi::fill(hdc, to_px(line, scale), theme.accent);
    }
    if let Some(zone) = state.cycle_highlight() {
        gdi::outline(hdc, to_px(zone, scale), theme.accent, (HIGHLIGHT_WIDTH * scale).round() as i32, radius);
    }
    unsafe {
        let _ = SetViewportOrgEx(hdc, previous.x, previous.y, None);
    }
}

fn draw_menu_bar(hdc: HDC, client: Rect, scale: f64, theme: &Theme, titles: &[&str], open: Option<usize>) {
    let bar = Rect::new(0.0, 0.0, client.width, MENU_BAR_HEIGHT * scale);
    gdi::fill(hdc, bar, theme.bar_bg);
    let font = (BAR_FONT * scale).round() as i32;
    for (i, (title, rect)) in titles.iter().zip(title_rects(hdc, titles, scale)).enumerate() {
        let pressed = open == Some(i);
        if pressed {
            gdi::fill(hdc, rect, theme.active_bg);
        }
        let color = if pressed { theme.active_text } else { theme.text };
        gdi::text(hdc, rect, title, color, font, Align::Center);
    }
}

/// Where each menu-bar title sits, in client pixels.
pub fn menu_title_rects(hwnd: HWND, titles: &[&str], scale: f64) -> Vec<Rect> {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let rects = title_rects(hdc, titles, scale);
        ReleaseDC(Some(hwnd), hdc);
        rects
    }
}

fn title_rects(hdc: HDC, titles: &[&str], scale: f64) -> Vec<Rect> {
    let font = (BAR_FONT * scale).round() as i32;
    let pad = MENU_TITLE_PADDING * scale;
    let mut x = (STATUS_PADDING * scale) / 2.0;
    titles
        .iter()
        .map(|t| {
            let width = gdi::text_width(hdc, t, font) as f64 + 2.0 * pad;
            let rect = Rect::new(x, 0.0, width, MENU_BAR_HEIGHT * scale);
            x += width;
            rect
        })
        .collect()
}

fn draw_status_bar(hdc: HDC, client: Rect, state: &AppState, theme: &Theme) {
    let scale = state.frame.scale;
    let height = STATUS_BAR_HEIGHT * scale;
    let bar = Rect::new(0.0, client.height - height, client.width, height);
    gdi::fill(hdc, bar, theme.bar_bg);
    gdi::fill(hdc, Rect::new(bar.x, bar.y, bar.width, 1.0), theme.divider);
    let pad = STATUS_PADDING * scale;
    let inner = Rect::new(bar.x + pad, bar.y, bar.width - 2.0 * pad, bar.height);
    let font = (BAR_FONT * scale).round() as i32;
    if let Some(profile) = &state.profile {
        gdi::text(hdc, inner, &format!("Profile: {profile}"), theme.muted, font, Align::Left);
    }
    gdi::text(hdc, inner, VERSION, theme.muted, font, Align::Right);
    gdi::text(hdc, help_rect(hdc, client, scale), "?", theme.muted, font, Align::Center);
}

/// The "?" in the status bar that opens the keyboard shortcuts, in client pixels.
pub fn status_help_rect(hwnd: HWND, client: Rect, scale: f64) -> Rect {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let rect = help_rect(hdc, client, scale);
        ReleaseDC(Some(hwnd), hdc);
        rect
    }
}

fn help_rect(hdc: HDC, client: Rect, scale: f64) -> Rect {
    let font = (BAR_FONT * scale).round() as i32;
    let height = STATUS_BAR_HEIGHT * scale;
    let width = 24.0 * scale;
    let right = client.width - STATUS_PADDING * scale - gdi::text_width(hdc, VERSION, font) as f64 - 6.0 * scale;
    Rect::new(right - width, client.height - height, width, height)
}

/// The stay-back reminder: accent-bordered pill with centred text.
pub fn draw_reminder(hdc: HDC, bounds: Rect, text: &str, theme: &Theme, scale: f64) {
    gdi::fill(hdc, bounds, theme.active_bg);
    gdi::outline(hdc, bounds, theme.accent, 1, (ZONE_RADIUS * scale).round() as i32);
    gdi::text(hdc, bounds, text, theme.active_text, (REMINDER_FONT * scale).round() as i32, Align::Center);
}
