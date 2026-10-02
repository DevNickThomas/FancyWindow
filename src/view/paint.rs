use std::cell::RefCell;
use std::collections::HashMap;

use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IMAGE_ICON, LR_DEFAULTCOLOR, LoadImageW};
use windows::core::PCWSTR;

use super::gdi::{self, Align};
use super::titlebar::{CaptionButton, TitleLayout, layout as title_layout};
use super::statusbar::{StatusLayout, layout as status_layout};
use super::{Color, Theme, canvas_top_px, theme_of, to_px};
use crate::app::{AppState, SegmentKind, StatusBar, StatusClick, WindowId, ZoneHeader};
use crate::model::Rect;

/// Zone corners, as in the mockup.
const ZONE_RADIUS: f64 = 6.0;
/// Key chips and the cycle-number badge, in the mono face.
const KEY_FONT: f64 = 11.0;
/// Zone headers: text size, icon size, left padding and gap between parts (DIPs).
const HEADER_FONT: f64 = 12.0;
const HEADER_ICON: f64 = 16.0;
const HEADER_PADDING: f64 = 8.0;
const HEADER_GAP: f64 = 7.0;
/// Menu-bar titles, at VS Code's 13px UI size.
const BAR_FONT: f64 = 13.0;
const STATUS_FONT: f64 = 12.0;
/// Space either side of each status-bar segment's content (as in view::statusbar).
const SEGMENT_PADDING: f64 = 9.0;
/// Status-bar segment icons and the gap after them.
const STATUS_ICON: f64 = 12.0;
const STATUS_ICON_GAP: f64 = 5.0;
/// Caption-button glyph size, as Windows draws them.
const CAPTION_GLYPH: f64 = 10.0;
const HIGHLIGHT_WIDTH: f64 = 2.0;
const REMINDER_FONT: f64 = 14.0;
const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));

/// Paints the whole client area, double-buffered to avoid flicker.
/// `chrome` is the title bar's window state; `icons` are hosted windows' small icons.
pub fn paint(hwnd: HWND, state: &AppState, titles: &[&str], chrome: &TitleChrome, icons: &HashMap<WindowId, isize>) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut client = Default::default();
        let _ = GetClientRect(hwnd, &mut client);
        let (w, h) = (client.right, client.bottom);

        let buffer = CreateCompatibleDC(Some(hdc));
        let bitmap = CreateCompatibleBitmap(hdc, w, h);
        let old = SelectObject(buffer, bitmap.into());

        render(buffer, w as f64, h as f64, state, titles, chrome, icons);

        let _ = BitBlt(hdc, 0, 0, w, h, Some(buffer), 0, 0, SRCCOPY);
        SelectObject(buffer, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(buffer);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Draws the whole client area (`width` x `height` pixels) into any DC: the window's
/// back buffer, or an off-screen bitmap (see examples/render.rs).
pub fn render(hdc: HDC, width: f64, height: f64, state: &AppState, titles: &[&str], chrome: &TitleChrome, icons: &HashMap<WindowId, isize>) {
    let theme = &theme_of(state);
    let client = Rect::new(0.0, 0.0, width, height);
    gdi::fill(hdc, client, theme.window_bg);
    draw_canvas(hdc, state, theme, icons);
    draw_title_bar(hdc, client, state, theme, titles, chrome);
    draw_status_bar(hdc, client, state, theme);
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
    // Occupied zones are tinted; empty ones are a dashed outline on the canvas (drawn below).
    for zone in state.zone_rects().into_iter().filter(|z| state.attachments.iter().any(|a| a.zone == z.id)) {
        gdi::rounded(hdc, to_px(zone.bounds, scale), theme.zone_fill, theme.zone_border, radius);
    }
    // Splitters are gaps, as in the mockup; only the one under the mouse or being dragged
    // shows, in the accent, like VS Code's sashes.
    for splitter in state.splitters() {
        gdi::fill(hdc, to_px(splitter.bounds, scale), theme.window_bg);
    }
    if let Some(lit) = state.highlighted_splitter() {
        gdi::fill(hdc, to_px(lit, scale), theme.accent);
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
    let mono = px(KEY_FONT) as i32;
    let badge_width = gdi::mono_text_width(hdc, &number, mono) as f64 + px(10.0);
    let badge = Rect::new(close.x - px(HEADER_GAP) - badge_width, bounds.y + (bounds.height - px(16.0)) / 2.0, badge_width, px(16.0));
    gdi::rounded(hdc, badge, theme.kbd_bg, theme.kbd_bg, px(3.0) as i32);
    gdi::mono_text(hdc, badge, &number, text, mono, Align::Center);

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
/// An empty zone, as in the mockup: a dashed outline, "Empty zone", and the two
/// gestures with their keys drawn as chips. The text is dropped when there's no room.
fn draw_hint(hdc: HDC, zone: Rect, theme: &Theme, scale: f64) {
    let px = |dip: f64| (dip * scale).round();
    let inset = zone.inflate(-px(1.0));
    gdi::dashed(hdc, inset, theme.zone_border, px(1.5).max(1.0) as i32, (ZONE_RADIUS * scale).round() as i32);
    if zone.width < 240.0 * scale || zone.height < 100.0 * scale {
        return;
    }
    let line = px(24.0);
    let top = zone.y + zone.height / 2.0 - 1.5 * line;
    let centre = zone.x + zone.width / 2.0;
    gdi::text(hdc, Rect::new(zone.x, top, zone.width, line), "Empty zone", theme.text, px(14.0) as i32, Align::Center);
    let hint = |y: f64, parts: &[Part]| draw_parts(hdc, centre, y, line, parts, theme, scale);
    hint(top + line, &[Part::Key("Alt"), Part::Text(" + drag a window here")]);
    hint(top + 2.0 * line, &[Part::Key("Ctrl"), Part::Text(" click splits \u{00B7} "), Part::Key("Shift"), Part::Text(" click rows")]);
}

/// A piece of a hint line: plain text, or a key drawn as a chip.
enum Part<'a> {
    Text(&'a str),
    Key(&'a str),
}

/// Draws parts side by side, centred on `centre`.
fn draw_parts(hdc: HDC, centre: f64, y: f64, height: f64, parts: &[Part], theme: &Theme, scale: f64) {
    let font = (HEADER_FONT * scale).round() as i32;
    let mono = (KEY_FONT * scale).round() as i32;
    let pad = (5.0 * scale).round();
    let width = |part: &Part| match part {
        Part::Text(t) => gdi::text_width(hdc, t, font) as f64,
        Part::Key(k) => gdi::mono_text_width(hdc, k, mono) as f64 + 2.0 * pad,
    };
    let mut x = centre - parts.iter().map(width).sum::<f64>() / 2.0;
    for part in parts {
        let w = width(part);
        match part {
            Part::Text(t) => gdi::text(hdc, Rect::new(x, y, w + 1.0, height), t, theme.muted, font, Align::Left),
            Part::Key(k) => {
                let chip = Rect::new(x, y + (height - (18.0 * scale).round()) / 2.0, w, (18.0 * scale).round());
                gdi::rounded(hdc, chip, theme.kbd_bg, theme.divider, (4.0 * scale).round() as i32);
                gdi::mono_text(hdc, chip, k, theme.text, mono, Align::Center);
            }
        }
        x += w;
    }
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
        // A search glyph says "click to search", as VS Code's command center does.
        let font = (STATUS_FONT * scale).round() as i32;
        let gap = (6.0 * scale).round();
        let glyph = (12.0 * scale).round();
        let width = gdi::text_width(hdc, &label, font) as f64 + glyph + gap;
        let x = (centre.x + (centre.width - width) / 2.0).max(centre.x + gap);
        gdi::glyph(hdc, Rect::new(x, centre.y, glyph, centre.height), '\u{E721}', theme.muted, (11.0 * scale).round() as i32);
        let text = Rect::new(x + glyph + gap, centre.y, (centre.right() - gap - x - glyph - gap).max(0.0), centre.height);
        gdi::text(hdc, text, &label, theme.muted, font, Align::Left);
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


/// The stay-back reminder: accent-bordered pill with centred text.
pub fn draw_reminder(hdc: HDC, bounds: Rect, text: &str, theme: &Theme, scale: f64) {
    gdi::fill(hdc, bounds, theme.active_bg);
    gdi::outline(hdc, bounds, theme.accent, 1, (ZONE_RADIUS * scale).round() as i32);
    gdi::text(hdc, bounds, text, theme.active_text, (REMINDER_FONT * scale).round() as i32, Align::Center);
}

/// The icon a status-bar segment shows, by what clicking it does (Segoe Fluent Icons).
fn segment_icon(click: Option<StatusClick>) -> Option<char> {
    match click? {
        StatusClick::WorkspacesMenu => Some('\u{E8A4}'),
        StatusClick::LayoutMenu => Some('\u{ECA5}'),
        StatusClick::BringForward => Some('\u{E896}'),
        StatusClick::Margin => None,
    }
}

/// The status bar's contents and where they go, measured with `hdc`.
fn status_layout_in(hdc: HDC, state: &AppState, width: f64, height: f64) -> (StatusBar, StatusLayout) {
    let scale = state.frame.scale;
    let font = (STATUS_FONT * scale).round() as i32;
    let icon = (STATUS_ICON + STATUS_ICON_GAP) * scale;
    let status = state.status_bar();
    let contents: Vec<f64> = status
        .left
        .iter()
        .map(|s| gdi::text_width(hdc, &s.text, font) as f64 + if segment_icon(s.click).is_some() { icon } else { 0.0 })
        .collect();
    let hint = status.hint.as_ref().map(|h| gdi::text_width(hdc, h, font) as f64);
    let layout = status_layout(width, height, scale, &contents, hint, gdi::text_width(hdc, VERSION, font) as f64);
    (status, layout)
}

/// The status bar's contents and layout for a client area, for hit-testing clicks.
pub fn measure_status_bar(hwnd: HWND, state: &AppState, width: f64, height: f64) -> (StatusBar, StatusLayout) {
    unsafe {
        let hdc = GetDC(Some(hwnd));
        let result = status_layout_in(hdc, state, width, height);
        ReleaseDC(Some(hwnd), hdc);
        result
    }
}

/// Accent-coloured bar: icon-led segments from the left; hint, "?" and version on the right.
fn draw_status_bar(hdc: HDC, client: Rect, state: &AppState, theme: &Theme) {
    let scale = state.frame.scale;
    let (status, layout) = status_layout_in(hdc, state, client.width, client.height);
    gdi::fill(hdc, layout.bar, theme.status_bg);
    let font = (STATUS_FONT * scale).round() as i32;
    let pad = (SEGMENT_PADDING * scale).round();
    for (segment, rect) in status.left.iter().zip(&layout.segments) {
        let (background, color) = match segment.kind {
            SegmentKind::Strong => (Some(theme.status_strong), theme.status_text),
            SegmentKind::Plain => (None, theme.status_text),
            SegmentKind::Warning => (Some(theme.status_warn), Color(0xFF, 0xFF, 0xFF)),
        };
        if let Some(background) = background {
            gdi::fill(hdc, *rect, background);
        }
        let mut x = rect.x + pad;
        if let Some(icon) = segment_icon(segment.click) {
            let size = (STATUS_ICON * scale).round();
            gdi::glyph(hdc, Rect::new(x, rect.y, size, rect.height), icon, color, size as i32);
            x += size + (STATUS_ICON_GAP * scale).round();
        }
        gdi::text(hdc, Rect::new(x, rect.y, rect.right() - x, rect.height), &segment.text, color, font, Align::Left);
    }
    if let (Some(hint), Some(rect)) = (&status.hint, layout.hint) {
        gdi::text(hdc, rect, hint, theme.status_text, font, Align::Center);
    }
    gdi::text(hdc, layout.help, "?", theme.status_text, font, Align::Center);
    gdi::text(hdc, layout.version, VERSION, theme.status_text, font, Align::Right);
}
