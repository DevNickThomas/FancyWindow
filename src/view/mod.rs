//! Turns `AppState` into pixels. Reads state; never changes it.
//!
//! Client area, top to bottom: menu bar, zone canvas, status bar.

pub(crate) mod gdi;
mod paint;
mod theme;

pub use paint::{draw_reminder, menu_title_rects, paint, status_help_rect};
pub use theme::{ACCENT_SWATCHES, Color, PRESETS, Theme};

use crate::app::AppState;

/// The theme the settings ask for.
pub fn theme_of(state: &AppState) -> Theme {
    Theme::new(&state.settings.theme_name, &state.settings.accent_color)
}

use crate::model::Rect;

/// Menu bar and status bar heights in DIPs.
pub const MENU_BAR_HEIGHT: f64 = 26.0;
pub const STATUS_BAR_HEIGHT: f64 = 24.0;

/// Canvas size in DIPs for a client area in pixels.
pub fn canvas_size(client_width_px: f64, client_height_px: f64, scale: f64) -> (f64, f64) {
    let height = client_height_px / scale - MENU_BAR_HEIGHT - STATUS_BAR_HEIGHT;
    (client_width_px / scale, height.max(0.0))
}

/// Pixels from the top of the client area to the top of the canvas.
pub fn canvas_top_px(scale: f64) -> f64 {
    (MENU_BAR_HEIGHT * scale).round()
}

/// A DIP rect to pixels relative to the canvas.
fn to_px(r: Rect, scale: f64) -> Rect {
    Rect::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale)
}
