//! Times the pure work done per mouse move and per repaint, on a busy state: a 3x3
//! grid, nine hosted windows with titles, nine saved workspaces.
//!
//!   cargo run --release --example perf
//!
//! A 60 Hz frame is 16 667 µs; these should be a tiny fraction of it.

use std::hint::black_box;
use std::time::Instant;

use fancy_window::app::*;
use fancy_window::model::*;

fn busy_state() -> AppState {
    let mut state = AppState::new();
    state.layout = GridLayout::uniform_grid(3, 3);
    update(&mut state, Msg::FrameChanged(Frame::new(1800.0, 1100.0, Point::new(0.0, 0.0), 1.0)));
    for slot in 0..WORKSPACE_SLOTS {
        update(&mut state, Msg::WorkspaceNamed { slot, name: format!("Workspace {slot}"), saved_at_utc: "2026-10-02T00:00:00Z".into() });
    }
    // A different layout on screen, so the workspace lookup has to check all nine.
    state.layout = GridLayout::uniform_grid(3, 3);
    let zones = state.zone_rects();
    for (i, zone) in zones.iter().enumerate() {
        let window = WindowId(100 + i as isize);
        let centre = Point::new(zone.bounds.x + zone.bounds.width / 2.0, zone.bounds.y + 60.0);
        update(&mut state, Msg::WindowDropped { window, at: centre, alt: true });
        update(&mut state, Msg::TitleChanged { window, title: format!("Some application window title {i} - App") });
    }
    state
}

fn time(name: &str, runs: u32, mut f: impl FnMut()) {
    for _ in 0..runs / 10 {
        f();
    }
    let start = Instant::now();
    for _ in 0..runs {
        f();
    }
    let per = start.elapsed().as_secs_f64() * 1e6 / runs as f64;
    println!("{name:<42} {per:>9.2} µs");
}

fn main() {
    let mut state = busy_state();
    assert_eq!(state.attachments.len(), 9);
    let entries = state.palette_entries();
    println!("{} hosted windows, {} palette entries\n", state.attachments.len(), entries.len());

    let mut x = 0.0;
    time("update(MouseMove) over the canvas", 100_000, || {
        x = (x + 7.0) % 1800.0;
        black_box(update(&mut state, Msg::MouseMove { at: Point::new(x, 500.0), mods: Modifiers::default() }));
    });
    time("cursor_at (every WM_SETCURSOR)", 100_000, || {
        black_box(state.cursor_at(Point::new(300.0, 14.0)));
    });
    time("status_bar (every repaint)", 20_000, || {
        black_box(state.status_bar());
    });
    time("zone_headers (every repaint)", 20_000, || {
        black_box(state.zone_headers());
    });
    time("active_highlight + empty_zones (repaint)", 20_000, || {
        black_box((state.active_highlight(), state.empty_zones()));
    });
    time("palette_entries (opening the palette)", 5_000, || {
        black_box(state.palette_entries());
    });
    time("filter_palette(\"lay big\") (per keystroke)", 5_000, || {
        black_box(filter_palette(&entries, "lay big"));
    });
    let mut y = 0.0;
    time("splitter drag step (MouseMove while dragging)", 20_000, || {
        if state.drag.is_none() {
            let splitter = state.splitters()[0];
            let at = Point::new(splitter.bounds.x + 1.0, splitter.bounds.y + 50.0);
            update(&mut state, Msg::MouseDown { at, button: Button::Left, mods: Modifiers::default() });
        }
        y = if y > 0.0 { -1.0 } else { 1.0 };
        let at = Point::new(state.drag.unwrap().last.x + y, 500.0);
        black_box(update(&mut state, Msg::MouseMove { at, mods: Modifiers::default() }));
    });
}
