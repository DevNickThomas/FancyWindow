//! Small modal dialogs drawn in the current theme: a text prompt, a hotkey
//! capture, message boxes, and the building blocks Preferences uses.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::{SS_CENTER, SS_CENTERIMAGE};
use windows::Win32::UI::Controls::{DRAWITEMSTRUCT, EM_SETSEL, ODS_SELECTED};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, PCWSTR, w};

use super::chrome;
use crate::model::{Chord, MOD_ALT, MOD_CONTROL, MOD_SHIFT, MOD_WIN, Rect};
use crate::view::gdi::{self, Align, colorref};
use crate::view::{Color, PRESETS, Theme};

pub const OK: usize = 1;
pub const CANCEL: usize = 2;
const CLEAR: usize = 3;
const WIDTH: f64 = 420.0;
const BUTTON: (f64, f64) = (86.0, 28.0);
const FONT: f64 = 14.0;

/// How an owner-drawn button looks.
#[derive(Clone, Copy)]
pub enum Look {
    /// A push button; `primary` ones use the accent.
    Push { primary: bool },
    /// A theme preset row: three colour tiles and the name.
    ThemeRow { preset: usize, selected: bool },
    /// An accent colour swatch.
    Swatch { color: Color, selected: bool },
}

thread_local! {
    /// The button that closed the open dialog's modal loop.
    static PRESSED: Cell<Option<usize>> = const { Cell::new(None) };
    static THEME: Cell<Option<Theme>> = const { Cell::new(None) };
    /// Owner-drawn button looks, by button handle (so nested dialogs don't collide).
    static LOOKS: RefCell<HashMap<isize, Look>> = RefCell::new(HashMap::new());
}

fn theme() -> Theme {
    THEME.get().unwrap_or_else(Theme::dark)
}

/// Asks for a line of text. `None` if cancelled.
pub fn prompt_text(owner: HWND, theme: Theme, title: &str, label: &str, default: &str) -> Option<String> {
    let dialog = Dialog::new(owner, theme, title, WIDTH, 126.0);
    dialog.label(label, (16.0, 14.0, WIDTH - 32.0, 20.0));
    let edit = dialog.control(w!("EDIT"), default, WS_BORDER | WINDOW_STYLE(ES_AUTOHSCROLL as u32) | WS_TABSTOP, (16.0, 40.0, WIDTH - 32.0, 26.0), 0);
    dialog.button("OK", OK, (WIDTH - 196.0, 82.0), BUTTON, Look::Push { primary: true });
    dialog.button("Cancel", CANCEL, (WIDTH - 102.0, 82.0), BUTTON, Look::Push { primary: false });
    unsafe {
        let _ = SetFocus(Some(edit));
        SendMessageW(edit, EM_SETSEL, Some(WPARAM(0)), Some(LPARAM(-1)));
    }
    let pressed = dialog.run(|msg| unsafe { IsDialogMessageW(dialog.hwnd, msg).as_bool() });
    let text = window_text(edit);
    dialog.finish((pressed == OK).then_some(text))
}

/// Captures a key chord, auditioning each one with `check` (an error says why it
/// can't be used). `Some(None)` means Clear; `None` means cancelled.
pub fn capture_hotkey(
    owner: HWND,
    theme: Theme,
    title: &str,
    current: Option<Chord>,
    check: impl Fn(Chord) -> Result<(), String>,
) -> Option<Option<Chord>> {
    let dialog = Dialog::new(owner, theme, title, WIDTH, 196.0);
    dialog.label("Press a key combination with at least one modifier (Ctrl, Alt, Shift, Win).", (16.0, 14.0, WIDTH - 32.0, 36.0));
    let preview_style = WINDOW_STYLE(SS_CENTER.0 | SS_CENTERIMAGE.0) | WS_BORDER;
    let shown = current.map(|c| c.display()).unwrap_or_else(|| "(no chord yet)".into());
    let preview = dialog.control(w!("STATIC"), &shown, preview_style, (16.0, 56.0, WIDTH - 32.0, 32.0), 0);
    let verdict = dialog.control(w!("STATIC"), "", WINDOW_STYLE(SS_CENTER.0), (16.0, 96.0, WIDTH - 32.0, 20.0), 0);
    dialog.label("Esc cancels. Save stores the chord. Clear removes the hotkey.", (16.0, 124.0, WIDTH - 32.0, 20.0));
    dialog.button("Save", OK, (WIDTH - 290.0, 152.0), BUTTON, Look::Push { primary: true });
    dialog.button("Clear", CLEAR, (WIDTH - 196.0, 152.0), BUTTON, Look::Push { primary: false });
    dialog.button("Cancel", CANCEL, (WIDTH - 102.0, 152.0), BUTTON, Look::Push { primary: false });
    let audition = |chord: Chord| {
        let result = check(chord);
        set_text(verdict, result.as_ref().map_or_else(|why| why.as_str(), |_| "Available"));
        result.is_ok()
    };
    let mut chord = current;
    let mut usable = current.is_some_and(audition);
    loop {
        let pressed = dialog.run(|msg| {
            if msg.message != WM_KEYDOWN && msg.message != WM_SYSKEYDOWN {
                return false;
            }
            match captured_chord(msg.wParam.0 as u16) {
                Capture::Cancel => PRESSED.set(Some(CANCEL)),
                Capture::Ignore => {}
                Capture::Invalid(why) => {
                    set_text(preview, why);
                    set_text(verdict, "");
                    chord = None;
                    usable = false;
                }
                Capture::Chord(c) => {
                    chord = Some(c);
                    set_text(preview, &c.display());
                    usable = audition(c);
                }
            }
            true
        });
        match (pressed, chord) {
            (OK, None) => set_text(preview, "(press a chord first)"),
            (OK, Some(_)) if !usable => {}
            (OK, Some(c)) => break dialog.finish(Some(Some(c))),
            (CLEAR, _) => break dialog.finish(Some(None)),
            _ => break dialog.finish(None),
        }
    }
}

enum Capture {
    Chord(Chord),
    Cancel,
    Ignore,
    Invalid(&'static str),
}

fn captured_chord(vk: u16) -> Capture {
    const MODIFIER_KEYS: [VIRTUAL_KEY; 11] =
        [VK_CONTROL, VK_LCONTROL, VK_RCONTROL, VK_SHIFT, VK_LSHIFT, VK_RSHIFT, VK_MENU, VK_LMENU, VK_RMENU, VK_LWIN, VK_RWIN];
    if vk == VK_ESCAPE.0 {
        return Capture::Cancel;
    }
    if MODIFIER_KEYS.iter().any(|k| k.0 == vk) {
        return Capture::Ignore;
    }
    let down = |k: VIRTUAL_KEY| unsafe { GetKeyState(k.0 as i32) } < 0;
    let modifiers = [(VK_CONTROL, MOD_CONTROL), (VK_MENU, MOD_ALT), (VK_SHIFT, MOD_SHIFT), (VK_LWIN, MOD_WIN), (VK_RWIN, MOD_WIN)]
        .iter()
        .filter(|(k, _)| down(*k))
        .fold(0, |acc, (_, m)| acc | m);
    if modifiers == 0 {
        return Capture::Invalid("(needs a modifier)");
    }
    let chord = Chord::new(modifiers, vk);
    // Only keys we can save and load again.
    match Chord::parse(&chord.format()) {
        Some(c) if c == chord => Capture::Chord(chord),
        _ => Capture::Invalid("(that key isn't supported)"),
    }
}

pub fn confirm(owner: HWND, title: &str, text: &str) -> bool {
    unsafe { MessageBoxW(Some(owner), &HSTRING::from(text), &HSTRING::from(title), MB_OKCANCEL | MB_ICONWARNING) == IDOK }
}

pub fn warn(owner: HWND, title: &str, text: &str) {
    unsafe {
        MessageBoxW(Some(owner), &HSTRING::from(text), &HSTRING::from(title), MB_OK | MB_ICONWARNING);
    }
}

/// A themed, fixed-size window centred on its owner, laid out in DIPs.
pub struct Dialog {
    owner: HWND,
    pub hwnd: HWND,
    scale: f64,
    font: HFONT,
}

impl Dialog {
    pub fn new(owner: HWND, theme: Theme, title: &str, width: f64, height: f64) -> Self {
        THEME.set(Some(theme));
        unsafe {
            let instance = GetModuleHandleW(None).expect("module handle");
            let class = WNDCLASSW {
                lpfnWndProc: Some(wndproc),
                hInstance: instance.into(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                lpszClassName: w!("FancyWindowDialog"),
                ..Default::default()
            };
            RegisterClassW(&class);
            let scale = GetDpiForWindow(owner) as f64 / 96.0;
            let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
            let mut frame = RECT { left: 0, top: 0, right: (width * scale) as i32, bottom: (height * scale) as i32 };
            let _ = AdjustWindowRectEx(&mut frame, style, false, WS_EX_DLGMODALFRAME);
            let (w, h) = (frame.right - frame.left, frame.bottom - frame.top);
            let mut owner_rect = RECT::default();
            let _ = GetWindowRect(owner, &mut owner_rect);
            let x = (owner_rect.left + owner_rect.right - w) / 2;
            let y = (owner_rect.top + owner_rect.bottom - h) / 2;
            let hwnd = CreateWindowExW(WS_EX_DLGMODALFRAME | WS_EX_CONTROLPARENT, w!("FancyWindowDialog"), &HSTRING::from(title), style, x, y, w, h, Some(owner), None, Some(instance.into()), None)
                .expect("dialog window");
            chrome::title_bar(hwnd, &theme);
            let font = CreateFontW(-((FONT * scale) as i32), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, DEFAULT_CHARSET, OUT_DEFAULT_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0, gdi::ui_face());
            Self { owner, hwnd, scale, font }
        }
    }

    pub fn control(&self, class: PCWSTR, text: &str, style: WINDOW_STYLE, (x, y, w, h): (f64, f64, f64, f64), id: usize) -> HWND {
        let s = self.scale;
        unsafe {
            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                class,
                &HSTRING::from(text),
                WS_CHILD | WS_VISIBLE | style,
                (x * s) as i32,
                (y * s) as i32,
                (w * s) as i32,
                (h * s) as i32,
                Some(self.hwnd),
                Some(HMENU(id as *mut c_void)),
                None,
                None,
            )
            .expect("dialog control");
            SendMessageW(hwnd, WM_SETFONT, Some(WPARAM(self.font.0 as usize)), Some(LPARAM(1)));
            hwnd
        }
    }

    pub fn label(&self, text: &str, bounds: (f64, f64, f64, f64)) -> HWND {
        self.control(w!("STATIC"), text, WINDOW_STYLE(0), bounds, 0)
    }

    /// An owner-drawn button, so it follows the theme.
    pub fn button(&self, text: &str, id: usize, (x, y): (f64, f64), (w, h): (f64, f64), look: Look) -> HWND {
        let button = self.control(w!("BUTTON"), text, WINDOW_STYLE(BS_OWNERDRAW as u32) | WS_TABSTOP, (x, y, w, h), id);
        LOOKS.with_borrow_mut(|l| l.insert(button.0 as isize, look));
        button
    }

    pub fn set_look(&self, id: usize, look: Look) {
        let button = unsafe { GetDlgItem(Some(self.hwnd), id as i32) }.expect("button exists");
        LOOKS.with_borrow_mut(|l| l.insert(button.0 as isize, look));
    }

    /// Switches to a new theme and repaints everything.
    pub fn set_theme(&self, theme: Theme) {
        THEME.set(Some(theme));
        chrome::title_bar(self.hwnd, &theme);
        unsafe {
            let _ = RedrawWindow(Some(self.hwnd), None, None, RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN);
        }
    }

    /// Runs modally until a button is pressed and returns its id. `filter` sees
    /// each message first and returns true if it handled it.
    pub fn run(&self, mut filter: impl FnMut(&MSG) -> bool) -> usize {
        PRESSED.set(None);
        unsafe {
            let _ = EnableWindow(self.owner, false);
            let _ = ShowWindow(self.hwnd, SW_SHOW);
            let mut msg = MSG::default();
            while PRESSED.get().is_none() && GetMessageW(&mut msg, None, 0, 0).as_bool() {
                if !filter(&msg) {
                    let _ = TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let _ = EnableWindow(self.owner, true);
        }
        PRESSED.take().unwrap_or(CANCEL)
    }

    pub fn finish<T>(self, result: T) -> T {
        unsafe {
            let _ = SetForegroundWindow(self.owner);
            let _ = DestroyWindow(self.hwnd);
            let _ = DeleteObject(self.font.into());
        }
        result
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_COMMAND if ((wparam.0 >> 16) & 0xFFFF) as u32 == BN_CLICKED => {
                PRESSED.set(Some(wparam.0 & 0xFFFF));
                LRESULT(0)
            }
            WM_CLOSE => {
                PRESSED.set(Some(CANCEL));
                LRESULT(0)
            }
            WM_ERASEBKGND => {
                let mut r = RECT::default();
                let _ = GetClientRect(hwnd, &mut r);
                let bounds = Rect::new(0.0, 0.0, r.right as f64, r.bottom as f64);
                gdi::fill(HDC(wparam.0 as *mut c_void), bounds, theme().popup_bg);
                LRESULT(1)
            }
            WM_DRAWITEM => {
                draw_button(&*(lparam.0 as *const DRAWITEMSTRUCT));
                LRESULT(1)
            }
            WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT => {
                let t = theme();
                let hdc = HDC(wparam.0 as *mut c_void);
                let bg = if message == WM_CTLCOLOREDIT { t.bar_bg } else { t.popup_bg };
                SetTextColor(hdc, colorref(t.text));
                SetBkColor(hdc, colorref(bg));
                SetDCBrushColor(hdc, colorref(bg));
                LRESULT(GetStockObject(DC_BRUSH).0 as isize)
            }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }
}

fn draw_button(item: &DRAWITEMSTRUCT) {
    let t = theme();
    let r = item.rcItem;
    let bounds = Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64);
    let scale = unsafe { GetDpiForWindow(item.hwndItem) } as f64 / 96.0;
    let font = (FONT * scale) as i32;
    let pressed = item.itemState.0 & ODS_SELECTED.0 != 0;
    let look = LOOKS.with_borrow(|l| l.get(&(item.hwndItem.0 as isize)).copied()).unwrap_or(Look::Push { primary: false });
    let hdc = item.hDC;
    match look {
        Look::Push { primary } => {
            let fill = if pressed { t.active_bg } else if primary { t.accent } else { t.splitter };
            gdi::fill(hdc, bounds, fill);
            let text = if primary || pressed { t.active_text } else { t.text };
            gdi::text(hdc, bounds, &window_text(item.hwndItem), text, font, Align::Center);
        }
        Look::ThemeRow { preset, selected } => {
            let p = &PRESETS[preset];
            gdi::fill(hdc, bounds, if pressed { t.bar_bg } else { t.popup_bg });
            let tile = 18.0 * scale;
            let top = bounds.y + (bounds.height - tile) / 2.0;
            for (i, color) in [p.window_bg, p.popup_bg, p.bar_bg].into_iter().enumerate() {
                let tile_rect = Rect::new(bounds.x + 10.0 * scale + i as f64 * (tile + 3.0 * scale), top, tile, tile);
                gdi::rounded(hdc, tile_rect, color, t.divider, (2.0 * scale) as i32);
            }
            let text_left = 10.0 * scale + 3.0 * (tile + 3.0 * scale) + 8.0 * scale;
            gdi::text(hdc, Rect::new(bounds.x + text_left, bounds.y, bounds.width - text_left, bounds.height), p.name, t.text, font, Align::Left);
            selection_border(hdc, bounds, selected, &t, scale);
        }
        Look::Swatch { color, selected } => {
            gdi::fill(hdc, bounds, t.popup_bg);
            let inset = 4.0 * scale;
            let inner = Rect::new(bounds.x + inset, bounds.y + inset, bounds.width - 2.0 * inset, bounds.height - 2.0 * inset);
            gdi::rounded(hdc, inner, color, color, (3.0 * scale) as i32);
            selection_border(hdc, bounds, selected, &t, scale);
        }
    }
}

fn selection_border(hdc: HDC, bounds: Rect, selected: bool, t: &Theme, scale: f64) {
    let (color, width) = if selected { (t.accent, 2.0) } else { (t.divider, 1.0) };
    gdi::outline(hdc, bounds, color, (width * scale).round() as i32, (4.0 * scale) as i32);
}

pub fn window_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) } as usize;
    String::from_utf16_lossy(&buffer[..len])
}

pub fn set_text(hwnd: HWND, text: &str) {
    unsafe {
        let _ = SetWindowTextW(hwnd, &HSTRING::from(text));
    }
}

/// Ends the current modal loop as if button `id` had been clicked.
pub fn press(id: usize) {
    PRESSED.set(Some(id));
}
