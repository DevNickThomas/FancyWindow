//! Which external window lives in which zone, and where it should be on screen.

use crate::model::{Point, Rect, SPLITTER_THICKNESS, ZoneId};

use super::{AppState, Effect};

/// Gap between a hosted window and its zone outline, before the user's margin.
const OUTLINE_BUFFER: f64 = 2.0;

/// Another application's top-level window, by its OS handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WindowId(pub isize);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub zone: ZoneId,
    pub window: WindowId,
}

impl AppState {
    pub fn zone_of(&self, window: WindowId) -> Option<ZoneId> {
        self.attachments.iter().find(|a| a.window == window).map(|a| a.zone)
    }

    /// Screen pixels a window hosted in `zone` should occupy.
    pub fn host_screen_rect(&self, zone: ZoneId) -> Option<Rect> {
        let bounds = self.zone_rects().into_iter().find(|z| z.id == zone)?.bounds;
        Some(self.frame.to_screen(host_rect(bounds, self.frame.canvas, self.margin)))
    }

    /// Where to draw the active window's highlight (canvas DIPs), if a hosted window has focus.
    pub fn active_highlight(&self) -> Option<ActiveHighlight> {
        let zone = self.zone_of(self.active?)?;
        let bounds = self.zone_rects().into_iter().find(|z| z.id == zone)?.bounds;
        let window = host_rect(bounds, self.frame.canvas, self.margin);
        Some(ActiveHighlight { glow: bounds, bevel: window.inflate(OUTLINE_BUFFER) })
    }
}

/// The active window's zone: a glow over the whole zone, including the splitter
/// halves beside it, and a bevel that hugs the hosted window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActiveHighlight {
    pub glow: Rect,
    pub bevel: Rect,
}

/// Tracks focus: only our own hosted windows light up. Activating one also moves
/// the cycle there, so Win+Alt+] carries on from the window the user picked.
pub(super) fn foreground_changed(state: &mut AppState, window: WindowId) -> Vec<Effect> {
    let active = state.zone_of(window).map(|_| window);
    if active == state.active {
        return vec![];
    }
    state.active = active;
    if active.is_some() {
        state.cycle = active;
    }
    vec![Effect::Repaint]
}

/// Insets a zone so every side shows the same gap (`OUTLINE_BUFFER + margin`) to the
/// visible edge. Sides next to a splitter lose an extra half splitter, which covers them.
pub fn host_rect(zone: Rect, canvas: Rect, margin: f64) -> Rect {
    let base = OUTLINE_BUFFER + margin;
    let half = SPLITTER_THICKNESS / 2.0;
    let inner = |at_edge: bool| if at_edge { base } else { base + half };
    let left = inner(zone.x <= canvas.x + 0.5);
    let top = inner(zone.y <= canvas.y + 0.5);
    let right = inner(zone.right() >= canvas.right() - 0.5);
    let bottom = inner(zone.bottom() >= canvas.bottom() - 0.5);
    Rect::new(
        zone.x + left,
        zone.y + top,
        (zone.width - left - right).max(0.0),
        (zone.height - top - bottom).max(0.0),
    )
}

/// A window finished moving. Alt+drop over a zone attaches it; an attached window
/// snaps back to its zone, or is let go if Alt is held.
pub(super) fn window_dropped(state: &mut AppState, window: WindowId, at: Point, alt: bool) -> Vec<Effect> {
    if let Some(zone) = state.zone_of(window) {
        if alt {
            state.attachments.retain(|a| a.window != window);
            return vec![Effect::Forget(window)];
        }
        return state.host_screen_rect(zone).map(|rect| vec![Effect::Place { window, rect }]).unwrap_or_default();
    }
    if !alt {
        return vec![];
    }
    let local = state.frame.from_screen(at);
    match state.layout.hit_test(state.frame.canvas, local) {
        Some(zone) => attach(state, zone, window),
        None => vec![],
    }
}

/// Puts `window` in `zone`, releasing whatever was there.
fn attach(state: &mut AppState, zone: ZoneId, window: WindowId) -> Vec<Effect> {
    let mut effects: Vec<Effect> = state
        .attachments
        .iter()
        .filter(|a| a.zone == zone)
        .map(|a| Effect::Release(a.window))
        .collect();
    state.attachments.retain(|a| a.zone != zone);
    state.attachments.push(Attachment { zone, window });
    // It was just dragged, so it is the foreground window; its focus event came before it was ours.
    state.active = Some(window);
    let rect = state.host_screen_rect(zone).expect("zone was just hit-tested");
    effects.push(Effect::Host { window, rect });
    effects.push(Effect::Repaint);
    effects
}

/// Moves every hosted window to its zone's current rect; windows whose zone
/// disappeared are released back to where they came from.
pub(super) fn reflow(state: &mut AppState) -> Vec<Effect> {
    let mut effects = Vec::new();
    let mut kept = Vec::new();
    for a in std::mem::take(&mut state.attachments) {
        match state.host_screen_rect(a.zone) {
            Some(rect) => {
                effects.push(Effect::Place { window: a.window, rect });
                kept.push(a);
            }
            None => effects.push(Effect::Release(a.window)),
        }
    }
    state.attachments = kept;
    effects
}

pub(super) fn raise_all(state: &AppState) -> Vec<Effect> {
    state.attachments.iter().map(|a| Effect::Raise(a.window)).collect()
}

/// Lets go of every window without moving it (used on exit).
pub(super) fn forget_all(state: &mut AppState) -> Vec<Effect> {
    std::mem::take(&mut state.attachments).into_iter().map(|a| Effect::Forget(a.window)).collect()
}
