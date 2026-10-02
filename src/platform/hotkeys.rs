//! Global hotkeys, shared between several running instances.
//!
//! Only one process can register a chord. Whichever instance owns it forwards the
//! press to the instance the user is looking at (the foreground one), so every
//! hotkey acts on the window in front. The protocol matches the .NET app, so old
//! and new instances cooperate.

use std::cell::RefCell;
use std::ffi::c_void;

use windows::Win32::Foundation::*;
use windows::Win32::System::DataExchange::COPYDATASTRUCT;
use windows::Win32::UI::Input::KeyboardAndMouse::{HOT_KEY_MODIFIERS, MOD_NOREPEAT, RegisterHotKey, UnregisterHotKey};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, w};

use crate::app::{Command, Msg};
use crate::model::{Chord, Rect};

/// Window property that marks a Fancy Window main window.
const MARKER: windows::core::PCWSTR = w!("FancyWindow.MainHwndMarker");
/// WM_COPYDATA tags (the .NET app's IntPtr constants, sign-extended).
const COPYDATA_HOTKEY: usize = 0xFFFF_FFFF_FC0D_EC0D;
const COPYDATA_BEGIN_CYCLE: usize = 0xFFFF_FFFF_FC0D_EC0E;

struct Binding {
    id: i32,
    chord: Chord,
    command: Command,
    /// Whether Windows gave this process the chord.
    owned: bool,
}

thread_local! {
    static BINDINGS: RefCell<Vec<Binding>> = const { RefCell::new(Vec::new()) };
}

/// Tags the window as a Fancy Window and registers the chords. A chord another
/// instance already owns still works here: that instance forwards it to us.
pub fn install(hwnd: HWND, bindings: Vec<(Chord, Command)>) {
    unsafe {
        let _ = SetPropW(hwnd, MARKER, Some(HANDLE(std::ptr::without_provenance_mut(1))));
    }
    for (chord, command) in bindings {
        register(hwnd, chord, command);
    }
}

/// Replaces whatever chord `command` had with `chord` (or none). Returns false if
/// another application already owns the chord; it is then not kept.
pub fn bind(hwnd: HWND, command: Command, chord: Option<Chord>) -> bool {
    let old = BINDINGS.with_borrow_mut(|b| {
        let i = b.iter().position(|x| x.command == command)?;
        Some(b.remove(i).id)
    });
    if let Some(id) = old {
        unsafe {
            let _ = UnregisterHotKey(Some(hwnd), id);
        }
    }
    let Some(chord) = chord else { return true };
    let (id, owned) = register(hwnd, chord, command);
    if !owned {
        BINDINGS.with_borrow_mut(|b| b.retain(|x| x.id != id));
    }
    owned
}

/// Registers with Windows and remembers the binding either way. Returns the id and
/// whether this process now owns the chord.
fn register(hwnd: HWND, chord: Chord, command: Command) -> (i32, bool) {
    let id = BINDINGS.with_borrow(|b| b.iter().map(|x| x.id).max().unwrap_or(0) + 1);
    let repeats = matches!(command, Command::MarginUp | Command::MarginDown | Command::CycleNext | Command::CyclePrevious);
    let modifiers = HOT_KEY_MODIFIERS(chord.modifiers) | if repeats { HOT_KEY_MODIFIERS(0) } else { MOD_NOREPEAT };
    let owned = unsafe { RegisterHotKey(Some(hwnd), id, modifiers, chord.key as u32) }.is_ok();
    BINDINGS.with_borrow_mut(|b| b.push(Binding { id, chord, command, owned }));
    (id, owned)
}

/// Whether Windows would give this process `chord` right now: registers it
/// briefly under a spare id, then lets it go.
pub fn is_free(hwnd: HWND, chord: Chord) -> bool {
    const PROBE: i32 = 0xBFFF;
    let free = unsafe { RegisterHotKey(Some(hwnd), PROBE, HOT_KEY_MODIFIERS(chord.modifiers) | MOD_NOREPEAT, chord.key as u32) }.is_ok();
    if free {
        unsafe {
            let _ = UnregisterHotKey(Some(hwnd), PROBE);
        }
    }
    free
}

pub fn uninstall(hwnd: HWND) {
    unsafe {
        let _ = RemovePropW(hwnd, MARKER);
    }
}

/// `Some(true)` if this process holds the command's chord, `Some(false)` if Windows
/// refused it, `None` if the command has no chord.
pub fn owned(command: Command) -> Option<bool> {
    BINDINGS.with_borrow(|b| b.iter().find(|x| x.command == command).map(|x| x.owned))
}

pub fn has_peers(hwnd: HWND) -> bool {
    !peers(hwnd).is_empty()
}

/// WM_HOTKEY: run it here, or forward it to the instance in front (then `None`).
pub fn on_hotkey(hwnd: HWND, id: i32) -> Option<Msg> {
    let (chord, command) = BINDINGS.with_borrow(|b| b.iter().find(|x| x.id == id).map(|x| (x.chord, x.command)))?;
    match foreground_instance() {
        Some(target) if target != hwnd => {
            send_copydata(target, COPYDATA_HOTKEY, &[chord.modifiers, chord.key as u32]);
            None
        }
        _ => Some(to_msg(hwnd, command)),
    }
}

/// WM_COPYDATA from another instance: a forwarded hotkey or a cycle hand-off.
pub fn on_copydata(hwnd: HWND, lparam: LPARAM) -> Option<Msg> {
    let data = unsafe { &*(lparam.0 as *const COPYDATASTRUCT) };
    if data.lpData.is_null() {
        return None;
    }
    let words = unsafe { std::slice::from_raw_parts(data.lpData as *const u32, data.cbData as usize / 4) };
    match (data.dwData, words) {
        (COPYDATA_HOTKEY, &[modifiers, key]) => {
            let chord = Chord::new(modifiers, key as u16);
            let command = BINDINGS.with_borrow(|b| b.iter().find(|x| x.chord == chord).map(|x| x.command))?;
            Some(to_msg(hwnd, command))
        }
        (COPYDATA_BEGIN_CYCLE, &[forward]) => Some(Msg::BeginCycleAtEdge { forward: forward != 0 }),
        _ => None,
    }
}

/// Passes the window cycle to the next instance along the screen.
pub fn hand_off_cycle(hwnd: HWND, forward: bool) {
    let mut instances = peers(hwnd);
    if instances.is_empty() {
        return;
    }
    instances.push((hwnd.0 as isize, window_rect(hwnd)));
    if let Some(next) = neighbour(&instances, hwnd.0 as isize, forward) {
        send_copydata(HWND(next as *mut c_void), COPYDATA_BEGIN_CYCLE, &[forward as u32]);
    }
}

fn to_msg(hwnd: HWND, command: Command) -> Msg {
    let has_peer = || !peers(hwnd).is_empty();
    match command {
        Command::CycleNext => Msg::Cycle { forward: true, has_peer: has_peer() },
        Command::CyclePrevious => Msg::Cycle { forward: false, has_peer: has_peer() },
        other => Msg::Command(other),
    }
}

/// The Fancy Window in front: the foreground window itself, or the owner of a hosted one.
fn foreground_instance() -> Option<HWND> {
    let fg = unsafe { GetForegroundWindow() };
    if is_instance(fg) {
        return Some(fg);
    }
    let owner = unsafe { GetWindow(fg, GW_OWNER) }.ok()?;
    is_instance(owner).then_some(owner)
}

fn is_instance(hwnd: HWND) -> bool {
    !hwnd.is_invalid() && !unsafe { GetPropW(hwnd, MARKER) }.is_invalid()
}

/// Every other Fancy Window main window, with its screen rect.
fn peers(me: HWND) -> Vec<(isize, Rect)> {
    let mut found: Vec<(isize, Rect)> = Vec::new();
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> BOOL {
        let (me, found) = unsafe { &mut *(data.0 as *mut (HWND, &mut Vec<(isize, Rect)>)) };
        if hwnd != *me && is_instance(hwnd) {
            found.push((hwnd.0 as isize, window_rect(hwnd)));
        }
        true.into()
    }
    let mut data = (me, &mut found);
    unsafe {
        let _ = EnumWindows(Some(visit), LPARAM(&mut data as *mut _ as isize));
    }
    found
}

/// The instance after (or before) `me`, ordered by centre x, then centre y, then handle; wraps.
pub fn neighbour(instances: &[(isize, Rect)], me: isize, forward: bool) -> Option<isize> {
    let mut ordered = instances.to_vec();
    let centre = |r: &Rect| (r.x + r.width / 2.0, r.y + r.height / 2.0);
    ordered.sort_by(|(ha, a), (hb, b)| {
        let ((ax, ay), (bx, by)) = (centre(a), centre(b));
        ax.total_cmp(&bx).then(ay.total_cmp(&by)).then(ha.cmp(hb))
    });
    let i = ordered.iter().position(|(h, _)| *h == me)?;
    let n = ordered.len();
    Some(ordered[if forward { (i + 1) % n } else { (i + n - 1) % n }].0)
}

fn window_rect(hwnd: HWND) -> Rect {
    let mut r = RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    Rect::new(r.left as f64, r.top as f64, (r.right - r.left) as f64, (r.bottom - r.top) as f64)
}

/// Synchronous, so the target has acted before the hotkey handler returns.
fn send_copydata(target: HWND, tag: usize, words: &[u32]) {
    let data = COPYDATASTRUCT { dwData: tag, cbData: (words.len() * 4) as u32, lpData: words.as_ptr() as *mut c_void };
    unsafe {
        SendMessageW(target, WM_COPYDATA, None, Some(LPARAM(&data as *const _ as isize)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighbours_are_ordered_left_to_right_and_wrap() {
        let at = |x: f64, y: f64| Rect::new(x, y, 100.0, 100.0);
        let instances = [(30, at(2000.0, 0.0)), (10, at(0.0, 0.0)), (20, at(1000.0, 0.0)), (40, at(1000.0, 900.0))];
        assert_eq!(neighbour(&instances, 10, true), Some(20));
        assert_eq!(neighbour(&instances, 20, true), Some(40));
        assert_eq!(neighbour(&instances, 30, true), Some(10));
        assert_eq!(neighbour(&instances, 10, false), Some(30));
        assert_eq!(neighbour(&instances, 99, true), None);
    }
}
