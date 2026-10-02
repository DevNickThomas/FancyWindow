//! The command palette: a themed popup under the title bar. Type to filter, arrows
//! to move, Enter to apply, Esc or a click anywhere else to close. Filtering and the
//! entries themselves are pure (`app::palette`); this is only the window.

use std::cell::RefCell;
use std::ffi::c_void;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::{DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND, DwmSetWindowAttribute};
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::w;

use crate::app::{MenuAction, PaletteEntry, PaletteMatch, filter_palette};
use crate::model::Rect;
use crate::view::gdi::{self, Align};
use crate::view::{Color, TITLE_BAR_HEIGHT, Theme};

/// Sizes in DIPs.
const WIDTH: f64 = 560.0;
const INPUT: f64 = 44.0;
const ROW: f64 = 28.0;
const FOOTER: f64 = 26.0;
const VISIBLE_ROWS: usize = 12;
const PADDING: f64 = 12.0;
const FONT: f64 = 13.0;
const SMALL_FONT: f64 = 12.0;
const KEY_FONT: f64 = 11.0;
/// Layout thumbnails, 16:10.
const THUMB: (f64, f64) = (26.0, 16.0);

struct Palette {
    entries: Vec<PaletteEntry>,
    query: String,
    matches: Vec<PaletteMatch>,
    selected: usize,
    /// First visible row.
    scroll: usize,
    theme: Theme,
    scale: f64,
    /// Set once the palette should close; `Some(action)` applies one.
    done: Option<Option<MenuAction>>,
}

thread_local! {
    static PALETTE: RefCell<Option<Palette>> = const { RefCell::new(None) };
}

/// Shows the palette over `owner` and waits; returns the chosen action, if any.
pub fn show(owner: HWND, theme: Theme, entries: Vec<PaletteEntry>) -> Option<MenuAction> {
    unsafe {
        if IsIconic(owner).as_bool() {
            let _ = ShowWindow(owner, SW_RESTORE);
        }
        let scale = GetDpiForWindow(owner) as f64 / 96.0;
        let matches = filter_palette(&entries, "");
        PALETTE.set(Some(Palette { entries, query: String::new(), matches, selected: 0, scroll: 0, theme, scale, done: None }));

        let mut origin = POINT::default();
        let _ = windows::Win32::Graphics::Gdi::ClientToScreen(owner, &mut origin);
        let mut client = RECT::default();
        let _ = GetClientRect(owner, &mut client);
        let width = (WIDTH * scale).min(client.right as f64 - 32.0 * scale).max(240.0 * scale);
        let height = (INPUT + ROW * VISIBLE_ROWS as f64 + FOOTER) * scale;
        let x = origin.x as f64 + (client.right as f64 - width) / 2.0;
        let y = origin.y as f64 + ((TITLE_BAR_HEIGHT + 6.0) * scale).round();

        let instance = GetModuleHandleW(None).expect("module handle");
        let class = WNDCLASSW {
            style: CS_DROPSHADOW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            lpszClassName: w!("FancyWindowPalette"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let Ok(hwnd) = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            w!("FancyWindowPalette"),
            w!("Command palette"),
            WS_POPUP,
            x as i32,
            y as i32,
            width as i32,
            height as i32,
            Some(owner),
            None,
            Some(instance.into()),
            None,
        ) else {
            PALETTE.set(None);
            return None;
        };
        let round = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &round as *const _ as *const c_void, size_of_val(&round) as u32);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetFocus(Some(hwnd));

        let mut msg = MSG::default();
        while PALETTE.with_borrow(|p| p.as_ref().is_some_and(|p| p.done.is_none())) {
            if !GetMessageW(&mut msg, None, 0, 0).as_bool() {
                // The app is quitting: hand WM_QUIT back to the main loop.
                PostQuitMessage(msg.wParam.0 as i32);
                break;
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        let _ = DestroyWindow(hwnd);
        PALETTE.take().and_then(|p| p.done.flatten())
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_PAINT => paint(hwnd),
            WM_ERASEBKGND => return LRESULT(1),
            WM_CHAR => {
                if let Some(c) = char::from_u32(wparam.0 as u32).filter(|c| !c.is_control()) {
                    edit(hwnd, |q| q.push(c));
                }
            }
            WM_KEYDOWN => key(hwnd, VIRTUAL_KEY(wparam.0 as u16)),
            WM_LBUTTONDOWN => {
                if let Some(row) = row_at(lparam) {
                    with(|p| {
                        p.selected = row;
                        p.done = Some(p.matches.get(row).map(|m| p.entries[m.entry].action));
                    });
                }
            }
            WM_MOUSEMOVE => {
                if let Some(row) = row_at(lparam) {
                    if with(|p| std::mem::replace(&mut p.selected, row)) != row {
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                }
            }
            WM_MOUSEWHEEL => {
                let notches = (wparam.0 >> 16) as i16 as i32 / WHEEL_DELTA as i32;
                with(|p| {
                    let max = p.matches.len().saturating_sub(VISIBLE_ROWS);
                    p.scroll = (p.scroll as i32 - 3 * notches).clamp(0, max as i32) as usize;
                });
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            // Clicking anywhere else closes it, like a menu.
            WM_ACTIVATE if (wparam.0 & 0xFFFF) as u32 == WA_INACTIVE => with(|p| {
                p.done.get_or_insert(None);
            }),
            _ => return DefWindowProcW(hwnd, message, wparam, lparam),
        }
        LRESULT(0)
    }
}

fn with<R>(f: impl FnOnce(&mut Palette) -> R) -> R {
    PALETTE.with_borrow_mut(|p| f(p.as_mut().expect("palette is open")))
}

/// Changes the query, then filters again from the top.
fn edit(hwnd: HWND, change: impl FnOnce(&mut String)) {
    with(|p| {
        change(&mut p.query);
        p.matches = filter_palette(&p.entries, &p.query);
        p.selected = 0;
        p.scroll = 0;
    });
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

fn key(hwnd: HWND, vk: VIRTUAL_KEY) {
    let ctrl = unsafe { GetKeyState(VK_CONTROL.0 as i32) } < 0;
    let step = |delta: i32| {
        with(|p| {
            if p.matches.is_empty() {
                return;
            }
            p.selected = (p.selected as i32 + delta).clamp(0, p.matches.len() as i32 - 1) as usize;
            if p.selected < p.scroll {
                p.scroll = p.selected;
            } else if p.selected >= p.scroll + VISIBLE_ROWS {
                p.scroll = p.selected + 1 - VISIBLE_ROWS;
            }
        });
        unsafe {
            let _ = InvalidateRect(Some(hwnd), None, false);
        }
    };
    match vk {
        VK_ESCAPE => with(|p| p.done = Some(None)),
        VK_RETURN => with(|p| p.done = Some(p.matches.get(p.selected).map(|m| p.entries[m.entry].action))),
        VK_UP => step(-1),
        VK_DOWN => step(1),
        VK_PRIOR => step(-(VISIBLE_ROWS as i32)),
        VK_NEXT => step(VISIBLE_ROWS as i32),
        // Ctrl+Backspace drops the last word, as in most text boxes.
        VK_BACK if ctrl => edit(hwnd, |q| {
            let kept = q.trim_end().rfind(' ').map_or(0, |i| i + 1);
            q.truncate(kept);
        }),
        VK_BACK => edit(hwnd, |q| {
            q.pop();
        }),
        _ => {}
    }
}

/// The row under a client point in `lparam`, if it shows an entry.
fn row_at(lparam: LPARAM) -> Option<usize> {
    let y = (lparam.0 >> 16) as i16 as f64;
    with(|p| {
        let top = INPUT * p.scale;
        let i = ((y - top) / (ROW * p.scale)).floor();
        (y >= top && i < VISIBLE_ROWS as f64).then(|| p.scroll + i as usize).filter(|&row| row < p.matches.len())
    })
}

unsafe fn paint(hwnd: HWND) {
    unsafe {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);
        let mut r = RECT::default();
        let _ = GetClientRect(hwnd, &mut r);
        let buffer = CreateCompatibleDC(Some(hdc));
        let bitmap = CreateCompatibleBitmap(hdc, r.right, r.bottom);
        let old = SelectObject(buffer, bitmap.into());
        PALETTE.with_borrow(|p| {
            if let Some(p) = p {
                draw(buffer, Rect::new(0.0, 0.0, r.right as f64, r.bottom as f64), p);
            }
        });
        let _ = BitBlt(hdc, 0, 0, r.right, r.bottom, Some(buffer), 0, 0, SRCCOPY);
        SelectObject(buffer, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(buffer);
        let _ = EndPaint(hwnd, &ps);
    }
}

/// Input box, matching rows with the typed letters highlighted, and a key-hint footer.
fn draw(hdc: HDC, bounds: Rect, p: &Palette) {
    let t = &p.theme;
    let px = |dip: f64| (dip * p.scale).round();
    let font = px(FONT) as i32;
    let small = px(SMALL_FONT) as i32;
    gdi::fill(hdc, bounds, t.popup_bg);
    gdi::outline(hdc, bounds, t.divider, 1, 0);

    let input = Rect::new(px(PADDING), px(8.0), bounds.width - 2.0 * px(PADDING), px(INPUT) - px(16.0));
    gdi::rounded(hdc, input, t.window_bg, t.accent, px(4.0) as i32);
    let text = Rect::new(input.x + px(10.0), input.y, input.width - px(20.0), input.height);
    if p.query.is_empty() {
        gdi::text(hdc, text, "Type a command, layout or workspace", t.muted, font, Align::Left);
    } else {
        gdi::text(hdc, text, &p.query, t.text, font, Align::Left);
    }
    let caret = (text.x + gdi::text_width(hdc, &p.query, font) as f64 + 1.0).min(text.right()) as i32;
    let (top, bottom) = ((input.y + input.height * 0.25) as i32, (input.bottom() - input.height * 0.25) as i32);
    gdi::line(hdc, (caret, top), (caret, bottom), t.text, 1);

    let rows_top = px(INPUT);
    if p.matches.is_empty() {
        let area = Rect::new(0.0, rows_top, bounds.width, px(ROW) * 2.0);
        gdi::text(hdc, area, "No matching commands", t.muted, font, Align::Center);
    }
    for (i, m) in p.matches.iter().enumerate().skip(p.scroll).take(VISIBLE_ROWS) {
        let entry = &p.entries[m.entry];
        let row = Rect::new(px(4.0), rows_top + (i - p.scroll) as f64 * px(ROW), bounds.width - px(8.0), px(ROW));
        let selected = i == p.selected;
        if selected {
            gdi::rounded(hdc, row, t.active_bg, t.active_bg, px(4.0) as i32);
        }
        let color = if selected { t.active_text } else { t.text };
        let mut right = row.right() - px(PADDING);
        if let Some(chord) = &entry.chord {
            right = draw_chord(hdc, right, row, chord, t, p.scale) - px(PADDING);
        }
        let mut left = row.x + px(PADDING) - px(4.0);
        if let Some(zones) = &entry.preview {
            let thumb = Rect::new(left, row.y + (row.height - px(THUMB.1)) / 2.0, px(THUMB.0), px(THUMB.1));
            draw_thumbnail(hdc, thumb, zones, if selected { t.active_text } else { t.muted }, p.scale);
            left = thumb.right() + px(10.0);
        }
        let label = Rect::new(left, row.y, (right - left).max(0.0), row.height);
        draw_highlighted(hdc, label, &entry.label, &m.matched, color, t.active_bevel, font);
    }

    let footer = Rect::new(0.0, bounds.height - px(FOOTER), bounds.width, px(FOOTER));
    gdi::fill(hdc, Rect::new(0.0, footer.y, bounds.width, 1.0), t.divider);
    let inner = Rect::new(px(PADDING), footer.y, footer.width - 2.0 * px(PADDING), footer.height);
    gdi::text(hdc, inner, "\u{2191}\u{2193} move     Enter apply     Esc close", t.muted, small, Align::Left);
    gdi::text(hdc, inner, &format!("{} of {}", p.matches.len(), p.entries.len()), t.muted, small, Align::Right);
}

/// A chord as one key chip per key ("Win" "Alt" "Home"), right-aligned at `right`.
/// Returns where the chips start.
fn draw_chord(hdc: HDC, right: f64, row: Rect, chord: &str, t: &Theme, scale: f64) -> f64 {
    let px = |dip: f64| (dip * scale).round();
    let mono = px(KEY_FONT) as i32;
    let keys: Vec<&str> = chord.split('+').collect();
    let widths: Vec<f64> = keys.iter().map(|k| gdi::mono_text_width(hdc, k, mono) as f64 + px(10.0)).collect();
    let mut x = right - widths.iter().sum::<f64>() - px(3.0) * (keys.len() as f64 - 1.0);
    let start = x;
    let height = px(18.0);
    for (key, w) in keys.iter().zip(widths) {
        let chip = Rect::new(x, row.y + (row.height - height) / 2.0, w, height);
        gdi::rounded(hdc, chip, t.kbd_bg, t.divider, px(3.0) as i32);
        gdi::mono_text(hdc, chip, key, t.text, mono, Align::Center);
        x += w + px(3.0);
    }
    start
}

/// A layout thumbnail: the zones of `zones` (unit square) as small filled tiles.
fn draw_thumbnail(hdc: HDC, r: Rect, zones: &[Rect], color: Color, scale: f64) {
    let gap = (1.0 * scale).round().max(1.0);
    gdi::outline(hdc, r, color, 1, (2.0 * scale).round() as i32);
    let inner = r.inflate(-gap - 1.0);
    for z in zones {
        let tile = Rect::new(inner.x + z.x * inner.width, inner.y + z.y * inner.height, z.width * inner.width, z.height * inner.height).inflate(-gap / 2.0);
        gdi::fill(hdc, tile, color);
    }
}

/// A label with the matched characters in `highlight`, drawn run by run.
fn draw_highlighted(hdc: HDC, r: Rect, label: &str, matched: &[usize], color: Color, highlight: Color, font: i32) {
    let chars: Vec<char> = label.chars().collect();
    let mut x = r.x;
    let mut start = 0;
    while start < chars.len() && x < r.right() {
        let lit = matched.contains(&start);
        let end = (start..chars.len()).find(|i| matched.contains(i) != lit).unwrap_or(chars.len());
        let run: String = chars[start..end].iter().collect();
        let width = gdi::text_width(hdc, &run, font) as f64;
        gdi::text(hdc, Rect::new(x, r.y, (r.right() - x).min(width + 1.0), r.height), &run, if lit { highlight } else { color }, font, Align::Left);
        x += width;
        start = end;
    }
}
