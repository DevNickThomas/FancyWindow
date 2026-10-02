//! The main window: creation, the message loop, and running effects.

use std::cell::Cell;
use std::collections::HashMap;
use std::ffi::c_void;

use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::{ClientToScreen, InvalidateRect, ScreenToClient};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemInformation::GetSystemTime;
use windows::Win32::UI::Accessibility::{HWINEVENTHOOK, SetWinEventHook, UnhookWinEvent};
use windows::Win32::UI::Controls::WM_MOUSELEAVE;
use windows::Win32::UI::HiDpi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{HSTRING, PCWSTR, Result, w};

use std::path::PathBuf;

use super::preferences::{self, Change};
use super::shortcuts::{self, Request, Row, Status};
use super::{chrome, dialogs, host, hotkeys, indicator, input, menus, placement, storage};
use crate::app::{AppState, CONFIGURABLE, Command, CursorKind, Effect, Frame, MenuAction, Msg, WindowId, menu_bar, update, zone_menu};
use crate::model::{Chord, Point, Rect, Settings, ZoneId, crash_log_name};
use crate::view::{self, Theme, theme_of};

const PURGE_TIMER: usize = 1;
const PURGE_INTERVAL_MS: u32 = 2000;

thread_local! {
    /// The main window, for the WinEvent callback (which gets no user data).
    static MAIN: Cell<HWND> = const { Cell::new(HWND(std::ptr::null_mut())) };
}

/// Everything the window procedure needs, stored in the window's user data.
struct Shell {
    state: AppState,
    tracking_leave: bool,
    /// Menu-bar title whose popup is open, drawn pressed.
    open_menu: Option<usize>,
    settings_path: PathBuf,
    /// Hosted windows' small icons for the zone headers (handles owned by those windows).
    icons: HashMap<WindowId, isize>,
}

/// Creates the main window and runs the message loop until it closes.
pub fn run(state: AppState, settings_path: PathBuf) -> Result<()> {
    unsafe {
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)?;
        let maximized = state.settings.window_maximized;
        let hwnd = create_window(state, settings_path)?;
        MAIN.set(hwnd);
        // Fires when any other process's window starts and finishes a move: the Alt+drag drop.
        let move_hook = SetWinEventHook(
            EVENT_SYSTEM_MOVESIZESTART,
            EVENT_SYSTEM_MOVESIZEEND,
            None,
            Some(on_move_size),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        // Any window coming to the front, our own included (which clears the highlight).
        let focus_hook = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            None,
            Some(on_foreground),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        // Hosted windows renaming themselves, for the zone headers.
        let name_hook = SetWinEventHook(
            EVENT_OBJECT_NAMECHANGE,
            EVENT_OBJECT_NAMECHANGE,
            None,
            Some(on_name_change),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        SetTimer(Some(hwnd), PURGE_TIMER, PURGE_INTERVAL_MS, None);
        hotkeys::install(hwnd, with_shell(hwnd, |s| s.state.hotkey_bindings()));
        apply_theme(hwnd);
        let _ = ShowWindow(hwnd, if maximized { SW_SHOWMAXIMIZED } else { SW_SHOW });

        let mut message = MSG::default();
        while GetMessageW(&mut message, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        let _ = UnhookWinEvent(move_hook);
        let _ = UnhookWinEvent(focus_hook);
        let _ = UnhookWinEvent(name_hook);
    }
    Ok(())
}

unsafe fn create_window(state: AppState, settings_path: PathBuf) -> Result<HWND> {
    unsafe {
        let instance = GetModuleHandleW(None)?;
        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wndproc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hIcon: LoadIconW(Some(instance.into()), PCWSTR(std::ptr::without_provenance(1)))?,
            lpszClassName: w!("FancyWindow"),
            ..Default::default()
        };
        RegisterClassW(&class);
        let (x, y, width, height) = placement::initial(state.settings.bounds());
        let title = HSTRING::from(state.title());
        let shell = Box::new(Shell { state, tracking_leave: false, open_menu: None, settings_path, icons: HashMap::new() });
        CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("FancyWindow"),
            &title,
            WS_OVERLAPPEDWINDOW,
            x,
            y,
            width,
            height,
            None,
            None,
            Some(instance.into()),
            Some(Box::into_raw(shell) as *const c_void),
        )
    }
}

extern "system" fn wndproc(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        match message {
            WM_NCCREATE => {
                let create = &*(lparam.0 as *const CREATESTRUCTW);
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            WM_NCDESTROY => drop(Box::from_raw(SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut Shell)),
            WM_CLOSE => {
                // Let go of hosted windows first: owned windows die with their owner.
                dispatch(hwnd, Msg::Closing { bounds: placement::current(hwnd) });
                let _ = DestroyWindow(hwnd);
            }
            WM_DESTROY => {
                hotkeys::uninstall(hwnd);
                indicator::hide();
                PostQuitMessage(0);
            }
            WM_HOTKEY => {
                if let Some(msg) = hotkeys::on_hotkey(hwnd, wparam.0 as i32) {
                    dispatch(hwnd, msg);
                }
            }
            WM_COPYDATA => match hotkeys::on_copydata(hwnd, lparam) {
                Some(msg) => {
                    dispatch(hwnd, msg);
                    return LRESULT(1);
                }
                None => return LRESULT(0),
            },
            indicator::WM_BRING_BACK => dispatch(hwnd, Msg::Command(Command::BringToFront)),
            WM_SIZE | WM_MOVE if !IsIconic(hwnd).as_bool() => dispatch(hwnd, Msg::FrameChanged(frame(hwnd))),
            WM_DPICHANGED => {
                let r = &*(lparam.0 as *const RECT);
                let _ = SetWindowPos(hwnd, None, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
                dispatch(hwnd, Msg::FrameChanged(frame(hwnd)));
            }
            WM_ACTIVATE => {
                if (wparam.0 & 0xFFFF) as u32 != WA_INACTIVE {
                    dispatch(hwnd, Msg::Activated);
                }
                return DefWindowProcW(hwnd, message, wparam, lparam);
            }
            WM_TIMER if wparam.0 == PURGE_TIMER => purge_closed_windows(hwnd),
            WM_PAINT => with_shell(hwnd, |s| {
                let titles: Vec<&str> = menu_bar(&s.state).iter().map(|m| m.title).collect();
                view::paint(hwnd, &s.state, &titles, s.open_menu, &s.icons);
            }),
            WM_LBUTTONDOWN if (lparam.0 >> 16) as i16 as f64 <= view::canvas_top_px(scale(hwnd)) => {
                open_bar_menu(hwnd, (lparam.0 & 0xFFFF) as i16 as f64);
            }
            WM_LBUTTONDOWN if on_status_help(hwnd, lparam) => dispatch(hwnd, Msg::Menu(MenuAction::ShowShortcuts)),
            WM_ERASEBKGND => return LRESULT(1),
            WM_SETCURSOR if (lparam.0 & 0xFFFF) as u32 == HTCLIENT => {
                set_cursor(hwnd);
                return LRESULT(1);
            }
            WM_MOUSELEAVE => {
                with_shell(hwnd, |s| s.tracking_leave = false);
                dispatch(hwnd, Msg::MouseLeft);
            }
            _ => {
                if message == WM_MOUSEMOVE {
                    track_mouse_leave(hwnd);
                }
                match input::translate(message, wparam, lparam, scale(hwnd), view::canvas_top_px(scale(hwnd))) {
                    Some(msg) => dispatch(hwnd, msg),
                    None => return DefWindowProcW(hwnd, message, wparam, lparam),
                }
            }
        }
        LRESULT(0)
    }
}

/// The MVU step: update the state, then carry out the resulting effects.
fn dispatch(hwnd: HWND, msg: Msg) {
    let effects = with_shell(hwnd, |s| update(&mut s.state, msg));
    for effect in effects {
        run_effect(hwnd, effect);
    }
}

fn run_effect(hwnd: HWND, effect: Effect) {
    unsafe {
        match effect {
            Effect::Repaint => {
                let _ = InvalidateRect(Some(hwnd), None, false);
            }
            Effect::CaptureMouse => {
                SetCapture(hwnd);
            }
            Effect::ReleaseMouse => {
                let _ = ReleaseCapture();
            }
            Effect::Host { window, rect } => {
                host::host(hwnd, window, rect);
                refresh_window_info(hwnd, window);
            }
            Effect::Place { window, rect } => host::place(hwnd, window, rect),
            Effect::Release(window) => host::release(window),
            Effect::Forget(window) => host::forget(window),
            Effect::Raise(window) => host::raise(hwnd, window),
            Effect::SaveSettings(settings) => save_settings(hwnd, &settings),
            Effect::Focus(window) => {
                let _ = SetForegroundWindow(HWND(window.0 as *mut c_void));
            }
            Effect::HandOffCycle { forward } => hotkeys::hand_off_cycle(hwnd, forward),
            Effect::SendToBack => {
                let _ = SetWindowPos(hwnd, Some(HWND_BOTTOM), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            }
            Effect::BringToFront => {
                if IsIconic(hwnd).as_bool() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                }
                let _ = SetForegroundWindow(hwnd);
            }
            Effect::StayBackIndicator(true) => {
                let chord = with_shell(hwnd, |s| s.state.chord_label(Command::BringToFront));
                indicator::show(hwnd, theme(hwnd), chord);
            }
            Effect::StayBackIndicator(false) => indicator::hide(),
            Effect::ShowZoneMenu { zone, at } => show_zone_menu(hwnd, zone, at),
            Effect::OpenSettingsFolder => menus::open_folder(hwnd, &storage::exe_dir()),
            Effect::ShowShortcuts => show_shortcuts(hwnd),
            Effect::ShowAbout => {
                let (settings, profile) = with_shell(hwnd, |s| (s.settings_path.clone(), s.state.profile.clone()));
                menus::show_about(hwnd, &settings, &storage::exe_dir().join(crash_log_name(profile.as_deref())));
            }
            Effect::PromptWorkspaceName { slot, default } => {
                if let Some(name) = dialogs::prompt_text(hwnd, theme(hwnd), "Save workspace", "Workspace name:", &default) {
                    dispatch(hwnd, Msg::WorkspaceNamed { slot, name, saved_at_utc: utc_now() });
                }
            }
            Effect::PromptHotkey { command, current } => {
                let title = format!("Hotkey: {}", command.label());
                let current = current.as_deref().and_then(Chord::parse);
                if let Some(chord) = dialogs::capture_hotkey(hwnd, theme(hwnd), &title, current, |c| audition(hwnd, command, c)) {
                    dispatch(hwnd, Msg::SetHotkey { command, chord });
                }
            }
            Effect::ConfirmDeleteWorkspace { slot, name } => {
                let text = format!("Delete saved workspace \"{name}\" from slot {}?
This cannot be undone.", slot + 1);
                if dialogs::confirm(hwnd, "Delete workspace", &text) {
                    dispatch(hwnd, Msg::WorkspaceDeleteConfirmed { slot });
                }
            }
            Effect::BindHotkey { command, chord } => {
                if !hotkeys::bind(hwnd, command, chord) {
                    dispatch(hwnd, Msg::HotkeyFailed { command });
                }
            }
            Effect::ShowWarning { title, text } => dialogs::warn(hwnd, &title, &text),
            Effect::ShowPreferences => show_preferences(hwnd),
            Effect::ApplyTheme => apply_theme(hwnd),
            Effect::Exit => {
                let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
            }
        }
    }
}

/// Best effort, like the original: failing to save must not stop the app closing.
fn save_settings(hwnd: HWND, settings: &Settings) {
    let path = with_shell(hwnd, |s| s.settings_path.clone());
    let _ = storage::save(&path, settings);
}

/// Runs `f` against the shell. Keep it short and free of Win32 calls that can
/// re-enter the window procedure; effects run after the borrow ends.
fn with_shell<R>(hwnd: HWND, f: impl FnOnce(&mut Shell) -> R) -> R {
    unsafe {
        let shell = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Shell;
        f(&mut *shell)
    }
}

fn scale(hwnd: HWND) -> f64 {
    unsafe { GetDpiForWindow(hwnd) as f64 / 96.0 }
}

/// Current canvas size, screen position and DPI scale.
fn frame(hwnd: HWND) -> Frame {
    let mut client = RECT::default();
    let mut origin = POINT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
        let _ = ClientToScreen(hwnd, &mut origin);
    }
    let scale = scale(hwnd);
    let (w, h) = view::canvas_size(client.right as f64, client.bottom as f64, scale);
    let top = view::canvas_top_px(scale);
    Frame::new(w, h, Point::new(origin.x as f64, origin.y as f64 + top), scale)
}

/// Hosted apps can close at any time; tell the app about any that are gone.
fn purge_closed_windows(hwnd: HWND) {
    let windows: Vec<WindowId> = with_shell(hwnd, |s| s.state.attachments.iter().map(|a| a.window).collect());
    for window in windows.into_iter().filter(|w| !host::is_alive(*w)) {
        dispatch(hwnd, Msg::WindowClosed(window));
    }
    with_shell(hwnd, |s| {
        let hosted: Vec<WindowId> = s.state.attachments.iter().map(|a| a.window).collect();
        s.icons.retain(|w, _| hosted.contains(w));
    });
}

/// Reads a hosted window's title and icon for its zone header.
fn refresh_window_info(hwnd: HWND, window: WindowId) {
    let icon = host::icon(window);
    with_shell(hwnd, |s| match icon {
        Some(icon) => s.icons.insert(window, icon),
        None => s.icons.remove(&window),
    });
    dispatch(hwnd, Msg::TitleChanged { window, title: host::title(window) });
}

unsafe extern "system" fn on_name_change(_: HWINEVENTHOOK, _: u32, window: HWND, id_object: i32, id_child: i32, _: u32, _: u32) {
    const CHILDID_SELF: i32 = 0;
    if id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF || window.is_invalid() {
        return;
    }
    let main = MAIN.get();
    let id = WindowId(window.0 as isize);
    // Every window in the system renames itself now and then; only ours matter.
    if with_shell(main, |s| s.state.zone_of(id).is_some()) {
        refresh_window_info(main, id);
    }
}

unsafe extern "system" fn on_move_size(_: HWINEVENTHOOK, event: u32, window: HWND, id_object: i32, _: i32, _: u32, _: u32) {
    let main = MAIN.get();
    if id_object != OBJID_WINDOW.0 || window.is_invalid() || window == main {
        return;
    }
    let id = WindowId(window.0 as isize);
    if event == EVENT_SYSTEM_MOVESIZESTART {
        host::move_started(id);
        return;
    }
    let mut cursor = POINT::default();
    let alt = unsafe {
        let _ = GetCursorPos(&mut cursor);
        GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0
    };
    let at = Point::new(cursor.x as f64, cursor.y as f64);
    dispatch(main, Msg::WindowDropped { window: id, at, alt });
    host::move_ended();
}

unsafe extern "system" fn on_foreground(_: HWINEVENTHOOK, _: u32, window: HWND, id_object: i32, _: i32, _: u32, _: u32) {
    if id_object != OBJID_WINDOW.0 || window.is_invalid() {
        return;
    }
    dispatch(MAIN.get(), Msg::ForegroundChanged(WindowId(window.0 as isize)));
}

fn set_cursor(hwnd: HWND) {
    let mut p = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut p);
        let _ = ScreenToClient(hwnd, &mut p);
    }
    let s = scale(hwnd);
    let top = view::canvas_top_px(s);
    let kind = with_shell(hwnd, |sh| sh.state.cursor_at(Point::new(p.x as f64 / s, (p.y as f64 - top) / s)));
    let id = match kind {
        CursorKind::Arrow => IDC_ARROW,
        CursorKind::SizeWestEast => IDC_SIZEWE,
        CursorKind::SizeNorthSouth => IDC_SIZENS,
        CursorKind::Hand => IDC_HAND,
    };
    unsafe {
        SetCursor(LoadCursorW(None, id).ok());
    }
}

/// Asks Windows to send WM_MOUSELEAVE once the mouse leaves the client area.
fn track_mouse_leave(hwnd: HWND) {
    if with_shell(hwnd, |s| std::mem::replace(&mut s.tracking_leave, true)) {
        return;
    }
    let mut tme = TRACKMOUSEEVENT { cbSize: size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: hwnd, dwHoverTime: 0 };
    unsafe {
        let _ = TrackMouseEvent(&mut tme);
    }
}


/// Opens the menu-bar popup under client x (pixels), if a title is there.
fn open_bar_menu(hwnd: HWND, x: f64) {
    let (menus, scale) = with_shell(hwnd, |s| (menu_bar(&s.state), s.state.frame.scale));
    let titles: Vec<&str> = menus.iter().map(|m| m.title).collect();
    let rects = view::menu_title_rects(hwnd, &titles, scale);
    let Some(i) = rects.iter().position(|r| x >= r.x && x < r.right()) else { return };
    let mut at = POINT { x: rects[i].x as i32, y: rects[i].bottom() as i32 };
    unsafe {
        let _ = ClientToScreen(hwnd, &mut at);
    }
    with_shell(hwnd, |s| s.open_menu = Some(i));
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    let chosen = menus::pick(hwnd, &menus[i].items, at);
    with_shell(hwnd, |s| s.open_menu = None);
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
    if let Some(action) = chosen {
        dispatch(hwnd, Msg::Menu(action));
    }
}

/// Pops up the zone menu at a canvas point (DIPs).
fn show_zone_menu(hwnd: HWND, zone: ZoneId, at: Point) {
    let (items, frame) = with_shell(hwnd, |s| (zone_menu(&s.state, zone), s.state.frame));
    let screen = POINT { x: (frame.origin.x + at.x * frame.scale) as i32, y: (frame.origin.y + at.y * frame.scale) as i32 };
    if let Some(action) = menus::pick(hwnd, &items, screen) {
        dispatch(hwnd, Msg::Menu(action));
    }
}

/// Current UTC time as ISO-8601, e.g. "2026-10-01T09:30:00Z".
fn utc_now() -> String {
    let t = unsafe { GetSystemTime() };
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", t.wYear, t.wMonth, t.wDay, t.wHour, t.wMinute, t.wSecond)
}

fn theme(hwnd: HWND) -> Theme {
    with_shell(hwnd, |s| theme_of(&s.state))
}

/// Recolours the OS-drawn parts (title bar, popup menus, reminder) to match the theme.
fn apply_theme(hwnd: HWND) {
    let theme = theme(hwnd);
    chrome::title_bar(hwnd, &theme);
    chrome::menus(&theme);
    indicator::set_theme(theme);
}

fn show_preferences(hwnd: HWND) {
    let (name, accent) = with_shell(hwnd, |s| (s.state.settings.theme_name.clone(), s.state.settings.accent_color.clone()));
    preferences::show(hwnd, theme(hwnd), &name, &accent, |change| {
        let msg = match change {
            Change::Theme(name) => Msg::SetTheme(name.to_string()),
            Change::Accent(hex) => Msg::SetAccent(hex),
        };
        dispatch(hwnd, msg);
        theme(hwnd)
    });
}

/// Whether a click (client pixels in `lparam`) hit the status bar's "?".
fn on_status_help(hwnd: HWND, lparam: LPARAM) -> bool {
    let (x, y) = ((lparam.0 & 0xFFFF) as i16 as f64, (lparam.0 >> 16) as i16 as f64);
    let mut client = RECT::default();
    unsafe {
        let _ = GetClientRect(hwnd, &mut client);
    }
    let client = Rect::new(0.0, 0.0, client.right as f64, client.bottom as f64);
    view::status_help_rect(hwnd, client, scale(hwnd)).contains(Point::new(x, y))
}

/// Whether `chord` can be given to `command`: not used by another command here,
/// and not held by another application.
fn audition(hwnd: HWND, command: Command, chord: Chord) -> std::result::Result<(), String> {
    if let Some(other) = with_shell(hwnd, |s| s.state.chord_user(chord, command)) {
        return Err(format!("Already used for \"{}\"", other.label()));
    }
    let ours = with_shell(hwnd, |s| s.state.chord_for(command)) == Some(chord) && hotkeys::owned(command) == Some(true);
    if ours || hotkeys::is_free(hwnd, chord) {
        Ok(())
    } else if hotkeys::has_peers(hwnd) {
        Err("In use by another application or Fancy Window".into())
    } else {
        Err("In use by another application".into())
    }
}

fn show_shortcuts(hwnd: HWND) {
    let rows = || {
        let shared = hotkeys::has_peers(hwnd);
        CONFIGURABLE
            .iter()
            .map(|&command| Row {
                command,
                current: with_shell(hwnd, |s| s.state.chord_for(command)),
                status: match hotkeys::owned(command) {
                    Some(false) if shared => Status::Shared,
                    Some(false) => Status::Taken,
                    _ => Status::Active,
                },
            })
            .collect()
    };
    shortcuts::show(hwnd, theme(hwnd), rows, |command, chord| audition(hwnd, command, chord), |request| {
        let msg = match request {
            Request::Change(command, chord) => Msg::SetHotkey { command, chord },
            Request::ResetAll => Msg::ResetHotkeys,
        };
        dispatch(hwnd, msg);
    });
}
