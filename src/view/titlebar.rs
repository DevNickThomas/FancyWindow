//! The one-row title bar, VS Code style: app icon, menus, a centre box and the
//! caption buttons, drawn by us in place of the native caption. Layout and
//! hit-testing are pure (client pixels in, rects out) so they can be tested.

use crate::model::{Point, Rect};

/// Title-bar height in DIPs.
pub const TITLE_BAR_HEIGHT: f64 = 32.0;
const ICON_LEFT: f64 = 12.0;
const ICON_SIZE: f64 = 16.0;
const ICON_GAP: f64 = 6.0;
/// Space either side of a menu title.
const MENU_PADDING: f64 = 8.0;
/// Caption buttons are Windows' standard 46 DIP wide.
const BUTTON_WIDTH: f64 = 46.0;
const CENTRE_WIDTH: f64 = 360.0;
const CENTRE_MIN_WIDTH: f64 = 160.0;
const CENTRE_HEIGHT: f64 = 22.0;
/// Room kept between the centre box and the menus or buttons.
const CENTRE_GAP: f64 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptionButton {
    Minimize,
    Maximize,
    Close,
}

/// What is under a point in the title bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleHit {
    /// The app icon: the window menu.
    SystemMenu,
    Menu(usize),
    /// The centre box: opens the command palette.
    Centre,
    Button(CaptionButton),
    /// Drag to move, double-click to maximise.
    Caption,
    /// The strip along the top edge that resizes the window.
    ResizeTop,
    ResizeTopLeft,
    ResizeTopRight,
}

/// Where everything in the title bar sits, in client pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct TitleLayout {
    pub bar: Rect,
    pub icon: Rect,
    pub menus: Vec<Rect>,
    /// Shows the window and workspace name; left out when the window is too narrow.
    pub centre: Option<Rect>,
    /// Minimise, maximise, close, left to right.
    pub buttons: [(CaptionButton, Rect); 3],
}

/// Lays out a title bar `width` pixels wide at `scale`; `menu_text_widths` are the
/// menu titles' text widths in pixels.
pub fn layout(width: f64, scale: f64, menu_text_widths: &[f64]) -> TitleLayout {
    let px = |dip: f64| (dip * scale).round();
    let height = px(TITLE_BAR_HEIGHT);
    let bar = Rect::new(0.0, 0.0, width, height);
    let icon_size = px(ICON_SIZE);
    let icon = Rect::new(px(ICON_LEFT), ((height - icon_size) / 2.0).round(), icon_size, icon_size);

    let mut x = icon.right() + px(ICON_GAP);
    let menus: Vec<Rect> = menu_text_widths
        .iter()
        .map(|w| {
            let r = Rect::new(x, 0.0, w + 2.0 * px(MENU_PADDING), height);
            x = r.right();
            r
        })
        .collect();

    let button = px(BUTTON_WIDTH);
    let buttons = [
        (CaptionButton::Minimize, Rect::new(width - 3.0 * button, 0.0, button, height)),
        (CaptionButton::Maximize, Rect::new(width - 2.0 * button, 0.0, button, height)),
        (CaptionButton::Close, Rect::new(width - button, 0.0, button, height)),
    ];

    // Centred on the window, but never over the menus or the buttons.
    let (left, right) = (x + px(CENTRE_GAP), buttons[0].1.x - px(CENTRE_GAP));
    let centre_width = px(CENTRE_WIDTH).min(right - left);
    let centre = (centre_width >= px(CENTRE_MIN_WIDTH)).then(|| {
        let cx = ((width - centre_width) / 2.0).clamp(left, right - centre_width);
        let h = px(CENTRE_HEIGHT);
        Rect::new(cx.round(), ((height - h) / 2.0).round(), centre_width, h)
    });

    TitleLayout { bar, icon, menus, centre, buttons }
}

impl TitleLayout {
    /// `None` below the title bar. `resize` is the top resize strip's height in pixels;
    /// a maximised window has none.
    pub fn hit(&self, p: Point, resize: f64, maximized: bool) -> Option<TitleHit> {
        if !self.bar.contains(p) {
            return None;
        }
        if !maximized && p.y < resize {
            return Some(if p.x < resize * 2.0 {
                TitleHit::ResizeTopLeft
            } else if p.x >= self.bar.right() - resize * 2.0 {
                TitleHit::ResizeTopRight
            } else {
                TitleHit::ResizeTop
            });
        }
        if let Some((button, _)) = self.buttons.iter().find(|(_, r)| r.contains(p)) {
            return Some(TitleHit::Button(*button));
        }
        if let Some(i) = self.menus.iter().position(|r| r.contains(p)) {
            return Some(TitleHit::Menu(i));
        }
        if self.centre.is_some_and(|c| c.contains(p)) {
            return Some(TitleHit::Centre);
        }
        // The whole strip left of the menus, not just the 16 px glyph.
        if p.x < self.menus.first().map_or(self.icon.right(), |m| m.x) {
            return Some(TitleHit::SystemMenu);
        }
        Some(TitleHit::Caption)
    }
}
