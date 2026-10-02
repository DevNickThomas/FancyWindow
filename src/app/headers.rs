//! Zone headers: a strip over each occupied zone with the hosted window's icon,
//! title, cycle number and a × that lets it go. Empty zones get usage hints instead.

use crate::model::{Point, Rect};

use super::attach::visible_rect;
use super::{AppState, Effect, WindowId};

/// Header height in DIPs, like an editor tab.
pub const HEADER_HEIGHT: f64 = 28.0;
/// The × button: size, and gap to the header's right edge.
const CLOSE_SIZE: f64 = 18.0;
const CLOSE_INSET: f64 = 5.0;

/// One occupied zone's header, in canvas DIPs.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoneHeader {
    pub window: WindowId,
    pub bounds: Rect,
    /// The × that releases the window.
    pub close: Rect,
    /// Empty until the platform has read the window's title.
    pub title: String,
    /// Position in the Win+Alt+] / [ cycle, from 1.
    pub number: usize,
    pub active: bool,
}

/// What a click on a header hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeaderHit {
    Title(WindowId),
    Close(WindowId),
}

impl AppState {
    /// Height the headers take from each zone (0 when they are turned off).
    pub fn header_height(&self) -> f64 {
        if self.settings.show_zone_headers { HEADER_HEIGHT } else { 0.0 }
    }

    pub fn zone_headers(&self) -> Vec<ZoneHeader> {
        let height = self.header_height();
        if height == 0.0 {
            return Vec::new();
        }
        let order = self.ordered_windows();
        self.zone_rects()
            .into_iter()
            .filter_map(|z| {
                let window = self.attachments.iter().find(|a| a.zone == z.id)?.window;
                let v = visible_rect(z.bounds, self.frame.canvas);
                let bounds = Rect::new(v.x, v.y, v.width, height.min(v.height));
                let close = Rect::new(
                    bounds.right() - CLOSE_INSET - CLOSE_SIZE,
                    bounds.y + (bounds.height - CLOSE_SIZE) / 2.0,
                    CLOSE_SIZE,
                    CLOSE_SIZE,
                );
                Some(ZoneHeader {
                    window,
                    bounds,
                    close,
                    title: self.titles.get(&window).cloned().unwrap_or_default(),
                    number: order.iter().position(|w| *w == window).map_or(0, |i| i + 1),
                    active: self.active == Some(window),
                })
            })
            .collect()
    }

    /// The visible area of every zone without a window, for the "drag a window here" hints.
    pub fn empty_zones(&self) -> Vec<Rect> {
        self.zone_rects()
            .into_iter()
            .filter(|z| !self.attachments.iter().any(|a| a.zone == z.id))
            .map(|z| visible_rect(z.bounds, self.frame.canvas))
            .collect()
    }

    pub fn header_at(&self, p: Point) -> Option<HeaderHit> {
        let header = self.zone_headers().into_iter().find(|h| h.bounds.contains(p))?;
        Some(if header.close.contains(p) { HeaderHit::Close(header.window) } else { HeaderHit::Title(header.window) })
    }
}

/// A plain click on a header: the title focuses the window, the × releases it.
pub(super) fn clicked(state: &mut AppState, hit: HeaderHit) -> Vec<Effect> {
    match hit {
        HeaderHit::Title(window) => {
            state.active = Some(window);
            state.cycle = Some(window);
            vec![Effect::Focus(window), Effect::Repaint]
        }
        HeaderHit::Close(window) => {
            state.attachments.retain(|a| a.window != window);
            vec![Effect::Release(window), Effect::Repaint]
        }
    }
}

/// The platform read a window's title (on hosting, and whenever it changes).
pub(super) fn title_changed(state: &mut AppState, window: WindowId, title: String) -> Vec<Effect> {
    if state.zone_of(window).is_none() || state.titles.get(&window) == Some(&title) {
        return vec![];
    }
    state.titles.insert(window, title);
    vec![Effect::Repaint]
}

pub(super) fn toggle(state: &mut AppState) -> Vec<Effect> {
    state.settings.show_zone_headers = !state.settings.show_zone_headers;
    vec![Effect::SaveSettings(state.current_settings())]
}
