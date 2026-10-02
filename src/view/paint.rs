use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW};
use windows::core::PCWSTR;

use super::gdi::{self, Align};
use super::titlebar::{CaptionButton, TitleLayout, layout as title_layout};
use super::{Color, STATUS_BAR_HEIGHT, Theme, canvas_top_px, theme_of, to_px};
use crate::app::{AppState, Segment, WindowId, ZoneHeader};
use crate::model::Rect;

const ZONE_RADIUS: f64 = 4.0;
/// Zone headers: text size, icon size, left padding and gap between parts (DIPs).
const HEADER_FONT: f64 = 12.0;
const HEADER_ICON: f64 = 16.0;
const HEADER_PADDING: f64 = 8.0;
const HEADER_GAP: f64 = 7.0;
/// Menu-bar titles, at VS Code's 13px UI size.
const BAR_FONT: f64 = 13.0;
const STATUS_FONT: f64 = 12.0;
const STATUS_PADDING: f64 = 10.0;
/// Space either side of each status-bar segment's text.
const SEGMENT_PADDING: f64 = 9.0;
/// Caption-button glyph size, as Windows draws them.
const CAPTION_GLYPH: f64 = 10.0;
const HIGHLIGHT_WIDTH: f64 = 2.0;
const REMINDER_FONT: f64 = 14.0;
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Paints the whole client area, double-buffered to avoid flicker.
/// `chrome` is the title bar's window state; `icons` are hosted windows' small icons.
pub fn paint(hwnd: HWND, state: &AppState, titles: &[&str], chrome: &TitleChrome, icons: &HashMap<WindowId, isize>) {
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
        draw_canvas(buffer, state, theme, icons);
        draw_title_bar(buffer, client, state, theme, titles, chrome);
        draw_status_bar(buffer, client, state, theme);

        let _ = BitBlt(hdc, 0, 0, w, h, Some(buffer), 0, 0, SRCCOPY);
        SelectObject(buffer, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(buffer);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Zones, splitters, the active-window highlight, zone headers, empty-zone hints and
/// the split preview, shifted below the menu bar.
fn draw_canvas(hdc: HDC, state: &AppState, theme: &Theme, icons: &HashMap<WindowId, isize>) {
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
    // After the splitters, so the glow spills onto the splitter halves beside the zone.
    let active = state.active_highlight();
    if let Some(active) = active {
        gdi::rounded(hdc, to_px(active.glow, scale), theme.active_glow, theme.active_glow, radius);
    }
    for header in state.zone_headers() {
        draw_header(hdc, &header, theme, icons.get(&header.window).copied(), scale);
    }
    // Over the headers: the active header's top edge becomes its accent line.
    if let Some(active) = active {
        gdi::outline(hdc, to_px(active.bevel, scale), theme.active_bevel, (HIGHLIGHT_WIDTH * scale).round() as i32, radius);
    }
    for zone in state.empty_zones() {
        draw_hint(hdc, to_px(zone, scale), theme, scale);
    }
    if let Some(line) = state.split_preview() {
        gdi::fill(hdc, to_px(line, scale), theme.accent);
    }
    unsafe {
        let _ = SetViewportOrgEx(hdc, previous.x, previous.y, None);
    }
}

/// Icon, title, cycle number and ×, like an editor tab. The active one is lifted to
/// the window background with full-strength text; the rest sit on the bar colour.
fn draw_header(hdc: HDC, header: &ZoneHeader, theme: &Theme, icon: Option<isize>, scale: f64) {
    let px = |dip: f64| (dip * scale).round();
    let bounds = to_px(header.bounds, scale);
    let (background, text) = if header.active { (theme.window_bg, theme.text) } else { (theme.bar_bg, theme.muted) };
    gdi::fill(hdc, bounds, background);
    gdi::fill(hdc, Rect::new(bounds.x, bounds.bottom() - 1.0, bounds.width, 1.0), theme.divider);

    let mut x = bounds.x + px(HEADER_PADDING);
    if let Some(icon) = icon {
        let size = px(HEADER_ICON);
        gdi::icon(hdc, x as i32, (bounds.y + (bounds.height - size) / 2.0) as i32, size as i32, icon);
        x += size + px(HEADER_GAP);
    }

    let close = to_px(header.close, scale);
    let number = header.number.to_string();
    let font = px(HEADER_FONT) as i32;
    let badge_width = gdi::text_width(hdc, &number, font) as f64 + px(10.0);
    let badge = Rect::new(close.x - px(HEADER_GAP) - badge_width, bounds.y + (bounds.height - px(16.0)) / 2.0, badge_width, px(16.0));
    gdi::rounded(hdc, badge, theme.splitter, theme.splitter, px(3.0) as i32);
    gdi::text(hdc, badge, &number, text, font, Align::Center);

    let title = if header.title.is_empty() { "(untitled)" } else { header.title.as_str() };
    let title_rect = Rect::new(x, bounds.y, (badge.x - px(HEADER_GAP) - x).max(0.0), bounds.height);
    gdi::text(hdc, title_rect, title, text, font, Align::Left);

    // ×: two strokes across the middle of the close box.
    let (cx, cy, arm) = (close.x + close.width / 2.0, close.y + close.height / 2.0, px(4.0));
    let width = px(1.2).max(1.0) as i32;
    gdi::line(hdc, ((cx - arm) as i32, (cy - arm) as i32), ((cx + arm) as i32 + 1, (cy + arm) as i32 + 1), text, width);
    gdi::line(hdc, ((cx - arm) as i32, (cy + arm) as i32), ((cx + arm) as i32 + 1, (cy - arm) as i32 - 1), text, width);
}

/// "Alt + drag a window here" in the middle of an empty zone, when there is room.
fn draw_hint(hdc: HDC, zone: Rect, theme: &Theme, scale: f64) {
    if zone.width < 230.0 * scale || zone.height < 70.0 * scale {
        return;
    }
    let font = (HEADER_FONT * scale).round() as i32;
    let line = 20.0 * scale;
    let middle = zone.y + zone.height / 2.0;
    gdi::text(hdc, Rect::new(zone.x, middle - line, zone.width, line), "Alt + drag a window here", theme.text, font, Align::Center);
    gdi::text(hdc, Rect::new(zone.x, middle, zone.width, line), "Ctrl click splits \u{00B7} Shift click rows", theme.muted, font, Align::Center);
}

/// Title-bar state only the platform knows: which menu is open, whether the window is
/// active or maximised, and which caption button the mouse is over or pressing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TitleChrome {
    pub open_menu: Option<usize>,
    pub active: bool,
    pub maximized: bool,
    pub hover: Option<CaptionButton>,
    pub pressed: Option<CaptionButton>,
}

/// The title bar's layout for a client area `width` pixels wide, measuring the menu titles.
pub fn measure_title_bar(hwnd: HWND, titles: &[&str], width: f64, scale: f64) -> TitleLayout {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let layout = title_layout_in(hdc, titles, width, scale);
        ReleaseDC(Some(hwnd), hdc);
        layout
    }
}

fn title_layout_in(hdc: HDC, titles: &[&str], width: f64, scale: f64) -> TitleLayout {
    let font = (BAR_FONT * scale).round() as i32;
    let widths: Vec<f64> = titles.iter().map(|t| gdi::text_width(hdc, t, font) as f64).collect();
    title_layout(width, scale, &widths)
}

/// Icon, menus, the centre box with the window and workspace name, and caption buttons.
/// An inactive window's title bar is muted, as Windows does.
fn draw_title_bar(hdc: HDC, client: Rect, state: &AppState, theme: &Theme, titles: &[&str], chrome: &TitleChrome) {
    let scale = state.frame.scale;
    let layout = title_layout_in(hdc, titles, client.width, scale);
    let text = if chrome.active { theme.text } else { theme.muted };
    gdi::fill(hdc, layout.bar, theme.bar_bg);
    gdi::fill(hdc, Rect::new(0.0, layout.bar.bottom() - 1.0, client.width, 1.0), theme.divider);

    if let Some(icon) = app_icon(layout.icon.width as i32) {
        gdi::icon(hdc, layout.icon.x as i32, layout.icon.y as i32, layout.icon.width as i32, icon);
    }

    let font = (BAR_FONT * scale).round() as i32;
    for (i, (title, rect)) in titles.iter().zip(&layout.menus).enumerate() {
        let open = chrome.open_menu == Some(i);
        if open {
            gdi::fill(hdc, *rect, theme.active_bg);
        }
        gdi::text(hdc, *rect, title, if open { theme.active_text } else { text }, font, Align::Center);
    }

    if let Some(centre) = layout.centre {
        let workspace = state.current_workspace().and_then(|slot| state.workspace(slot)).and_then(|ws| ws.name.clone());
        let label = match workspace {
            Some(name) => format!("{} \u{2014} {name}", state.title()),
            None => state.title(),
        };
        gdi::rounded(hdc, centre, theme.window_bg, theme.divider, (6.0 * scale).round() as i32);
        gdi::text(hdc, centre, &label, theme.muted, (STATUS_FONT * scale).round() as i32, Align::Center);
    }

    let glyph_size = (CAPTION_GLYPH * scale).round() as i32;
    for (button, rect) in layout.buttons {
        let close = button == CaptionButton::Close;
        let background = match (chrome.pressed == Some(button), chrome.hover == Some(button), close) {
            (true, _, true) => Some(Color(0xB2, 0x27, 0x1A)),
            (_, true, true) => Some(Color(0xC4, 0x2B, 0x1C)),
            (true, _, false) => Some(theme.text.over(theme.bar_bg, 0.18)),
            (_, true, false) => Some(theme.text.over(theme.bar_bg, 0.10)),
            _ => None,
        };
        if let Some(background) = background {
            gdi::fill(hdc, rect, background);
        }
        let glyph = match button {
            CaptionButton::Minimize => '\u{E921}',
            CaptionButton::Maximize if chrome.maximized => '\u{E923}',
            CaptionButton::Maximize => '\u{E922}',
            CaptionButton::Close => '\u{E8BB}',
        };
        let color = if close && background.is_some() { Color(0xFF, 0xFF, 0xFF) } else { text };
        gdi::glyph(hdc, rect, glyph, color, glyph_size);
    }
}

/// Our own icon at exactly `size` pixels, picked from the .ico's sizes so it stays crisp.
fn app_icon(size: i32) -> Option<isize> {
    thread_local! {
        static ICONS: RefCell<HashMap<i32, isize>> = RefCell::new(HashMap::new());
    }
    if let Some(icon) = ICONS.with_borrow(|i| i.get(&size).copied()) {
        return Some(icon);
    }
    let icon = unsafe {
        let module = GetModuleHandleW(None).ok()?;
        LoadImageW(Some(module.into()), PCWSTR(std::ptr::without_provenance(1)), IMAGE_ICON, size, size, LR_DEFAULTCOLOR).ok()?
    };
    ICONS.with_borrow_mut(|i| i.insert(size, icon.0 as isize));
    Some(icon.0 as isize)
}

/// Accent-coloured bar: state segments from the left; hint, "?" and version on the right.
fn draw_status_bar(hdc: HDC, client: Rect, state: &AppState, theme: &Theme) {
    let scale = state.frame.scale;
    let height = STATUS_BAR_HEIGHT * scale;
    let bar = Rect::new(0.0, client.height - height, client.width, height);
    gdi::fill(hdc, bar, theme.status_bg);
    let font = (STATUS_FONT * scale).round() as i32;
    let pad = SEGMENT_PADDING * scale;
    let status = state.status_bar();
    let mut x = bar.x;
    for segment in &status.left {
        let (text, background, color) = match segment {
            Segment::Strong(t) => (t, Some(theme.status_strong), theme.status_text),
            Segment::Plain(t) => (t, None, theme.status_text),
            Segment::Warning(t) => (t, Some(theme.status_warn), Color(0xFF, 0xFF, 0xFF)),
        };
        let rect = Rect::new(x, bar.y, gdi::text_width(hdc, text, font) as f64 + 2.0 * pad, bar.height);
        if let Some(background) = background {
            gdi::fill(hdc, rect, background);
        }
        gdi::text(hdc, rect, text, color, font, Align::Center);
        x = rect.right();
    }
    let inner = Rect::new(bar.x + STATUS_PADDING * scale, bar.y, bar.width - 2.0 * STATUS_PADDING * scale, bar.height);
    gdi::text(hdc, inner, VERSION, theme.status_text, font, Align::Right);
    let help = help_rect(hdc, client, scale);
    gdi::text(hdc, help, "?", theme.status_text, font, Align::Center);
    if let Some(hint) = &status.hint {
        let width = gdi::text_width(hdc, hint, font) as f64 + 2.0 * pad;
        // Only when it fits beside the segments; a narrow window drops it.
        if help.x - width >= x {
            gdi::text(hdc, Rect::new(help.x - width, bar.y, width, bar.height), hint, theme.status_text, font, Align::Center);
        }
    }
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
    let font = (STATUS_FONT * scale).round() as i32;
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
