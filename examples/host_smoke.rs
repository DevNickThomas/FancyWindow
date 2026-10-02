//! Manual smoke test for `platform::host` against two windows you launch yourself.
//!
//! Usage: host_smoke <anchor-hwnd> <target-hwnd>   (decimal handles)
//! Only those two windows are touched; no mouse or keyboard input is sent.

use std::thread::sleep;
use std::time::Duration;

use fancy_window::app::WindowId;
use fancy_window::model::Rect;
use fancy_window::platform::host;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() {
    let args: Vec<isize> = std::env::args().skip(1).map(|a| a.parse().expect("decimal hwnd")).collect();
    let [anchor, target] = args[..] else { panic!("usage: host_smoke <anchor-hwnd> <target-hwnd>") };
    let anchor = HWND(anchor as *mut _);
    let target = WindowId(target);
    unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };

    report("before");
    host::host(anchor, target, Rect::new(300.0, 300.0, 400.0, 250.0));
    settle();
    report("hosted at 300,300 400x250");
    host::place(anchor, target, Rect::new(350.0, 320.0, 300.0, 200.0));
    settle();
    report("placed at 350,320 300x200");
    host::release(target);
    settle();
    report("released (expect original rect)");
    host::host(anchor, target, Rect::new(300.0, 300.0, 400.0, 250.0));
    host::forget(target);
    settle();
    report("hosted then forgotten (expect 300,300 400x250, frame back)");
}

/// `place` and `raise` are asynchronous; give the target's thread a moment.
fn settle() {
    sleep(Duration::from_millis(300));
}

fn report(label: &str) {
    let h = HWND(std::env::args().nth(2).unwrap().parse::<isize>().unwrap() as *mut _);
    unsafe {
        let mut r = RECT::default();
        let _ = GetWindowRect(h, &mut r);
        let style = GetWindowLongPtrW(h, GWL_STYLE) as u32;
        let owner = GetWindow(h, GW_OWNER).map(|o| o.0 as isize).unwrap_or(0);
        println!(
            "{label:<55} rect={},{} {}x{}  owner={owner}  thickframe={} maxbox={}",
            r.left,
            r.top,
            r.right - r.left,
            r.bottom - r.top,
            style & WS_THICKFRAME.0 != 0,
            style & WS_MAXIMIZEBOX.0 != 0,
        );
    }
}
