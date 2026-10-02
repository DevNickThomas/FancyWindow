//! Turns `AppState` into pixels. Reads state; never changes it.
//!
//! Client area, top to bottom: title bar (icon, menus, caption buttons), zone
//! canvas, status bar. The native caption is gone; see `titlebar`.

pub(crate) mod gdi;
mod paint;
mod theme;
mod statusbar;
mod titlebar;

pub use paint::{TitleChrome, draw_reminder, measure_status_bar, measure_title_bar, paint, render};
pub use statusbar::{STATUS_BAR_HEIGHT, StatusHit, StatusLayout, layout as status_layout};
pub use theme::{ACCENT_SWATCHES, Color, PRESETS, Theme};
pub use titlebar::{CaptionButton, TITLE_BAR_HEIGHT, TitleHit, TitleLayout, layout as title_layout};

use crate::app::AppState;

/// The theme the settings ask for.
pub fn theme_of(state: &AppState) -> Theme {
    Theme::new(&state.settings.theme_name, &state.settings.accent_color)
}

use crate::model::Rect;

/// Canvas size in DIPs for a client area in pixels.
pub fn canvas_size(client_width_px: f64, client_height_px: f64, scale: f64) -> (f64, f64) {
    let height = (client_height_px - canvas_top_px(scale)) / scale - STATUS_BAR_HEIGHT;
    (client_width_px / scale, height.max(0.0))
}

/// Pixels from the top of the client area to the top of the canvas.
pub fn canvas_top_px(scale: f64) -> f64 {
    (TITLE_BAR_HEIGHT * scale).round()
}

/// A DIP rect to pixels relative to the canvas.
fn to_px(r: Rect, scale: f64) -> Rect {
    Rect::new(r.x * scale, r.y * scale, r.width * scale, r.height * scale)
}
