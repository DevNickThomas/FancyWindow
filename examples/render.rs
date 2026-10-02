//! Paints Fancy Window into off-screen bitmaps and saves them, to compare against
//! docs/design/restyle-report.html without opening a window or touching the desktop.
//!
//!   cargo run --release --example render -- <out-dir>
//!
//! Writes dark.bmp and light.bmp: the report's hero scene (1100 x 660, "Big left,
//! stacked right", workspace "Coding", two hosted windows and one empty zone).

use std::collections::HashMap;
use std::ffi::c_void;
use std::path::PathBuf;

use fancy_window::app::*;
use fancy_window::model::*;
use fancy_window::view::{TitleChrome, render};
use windows::Win32::Graphics::Gdi::*;

const SIZE: (i32, i32) = (1100, 660);

fn scene(theme: &str) -> AppState {
    let mut state = AppState::new();
    state.settings.theme_name = theme.into();
    state.layout = (PRESETS.iter().find(|(name, _)| *name == "Big left, stacked right").expect("preset").1)();
    // Canvas = client minus the 36 DIP title bar and 24 DIP status bar.
    update(&mut state, Msg::FrameChanged(Frame::new(SIZE.0 as f64, SIZE.1 as f64 - 60.0, Point::new(0.0, 36.0), 1.0)));
    update(&mut state, Msg::WorkspaceNamed { slot: 0, name: "Coding".into(), saved_at_utc: "2026-10-02T00:00:00Z".into() });
    let zones = state.zone_rects();
    let hosted = [(WindowId(1), "PowerShell \u{2014} D:\\projects\\fancy-window-rs\\source"), (WindowId(2), "notes.txt \u{2014} Notepad")];
    for ((window, title), zone) in hosted.into_iter().zip(&zones) {
        let at = Point::new(zone.bounds.x + zone.bounds.width / 2.0, 36.0 + zone.bounds.y + zone.bounds.height / 2.0);
        update(&mut state, Msg::WindowDropped { window, at, alt: true });
        update(&mut state, Msg::TitleChanged { window, title: title.into() });
    }
    update(&mut state, Msg::ForegroundChanged(WindowId(1)));
    state
}

fn main() {
    let out = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| ".".into()));
    std::fs::create_dir_all(&out).expect("output folder");
    for (theme, file) in [("Dark", "dark.bmp"), ("Light", "light.bmp")] {
        let state = scene(theme);
        let titles: Vec<&str> = menu_bar(&state).iter().map(|m| m.title).collect();
        let chrome = TitleChrome { active: true, ..Default::default() };
        let pixels = draw(|hdc| render(hdc, SIZE.0 as f64, SIZE.1 as f64, &state, &titles, &chrome, &HashMap::new()));
        write_bmp(&out.join(file), &pixels);
        println!("wrote {}", out.join(file).display());
    }
}

/// Runs `paint` on a 32-bit off-screen bitmap and returns its pixels, top row first.
fn draw(paint: impl FnOnce(HDC)) -> Vec<u8> {
    unsafe {
        let screen = GetDC(None);
        let dc = CreateCompatibleDC(Some(screen));
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: SIZE.0,
                biHeight: -SIZE.1,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits: *mut c_void = std::ptr::null_mut();
        let bitmap = CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0).expect("DIB section");
        let old = SelectObject(dc, bitmap.into());
        paint(dc);
        let _ = GdiFlush();
        let pixels = std::slice::from_raw_parts(bits as *const u8, (SIZE.0 * SIZE.1 * 4) as usize).to_vec();
        SelectObject(dc, old);
        let _ = DeleteObject(bitmap.into());
        let _ = DeleteDC(dc);
        ReleaseDC(None, screen);
        pixels
    }
}

/// A plain top-down 32-bit BMP.
fn write_bmp(path: &std::path::Path, pixels: &[u8]) {
    let mut file = Vec::with_capacity(54 + pixels.len());
    let size = 54 + pixels.len() as u32;
    file.extend_from_slice(b"BM");
    file.extend_from_slice(&size.to_le_bytes());
    file.extend_from_slice(&[0; 4]);
    file.extend_from_slice(&54u32.to_le_bytes());
    file.extend_from_slice(&40u32.to_le_bytes());
    file.extend_from_slice(&SIZE.0.to_le_bytes());
    file.extend_from_slice(&(-SIZE.1).to_le_bytes());
    file.extend_from_slice(&1u16.to_le_bytes());
    file.extend_from_slice(&32u16.to_le_bytes());
    file.extend_from_slice(&[0; 24]);
    file.extend_from_slice(pixels);
    std::fs::write(path, file).expect("write bmp");
}
