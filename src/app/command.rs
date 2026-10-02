//! What the global hotkeys do.

use crate::model::{GridLayout, MAX_MARGIN};

use super::{AppState, Effect, WindowId};

const MARGIN_STEP: f64 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    MarginUp,
    MarginDown,
    /// Win+Alt+PageUp: bring Fancy Window forward and leave stay-back mode.
    BringToFront,
    /// Win+Alt+PageDown: sink behind everything and stay there across Alt+drag attaches.
    SendToBack,
    ResetLayout,
    CycleNext,
    CyclePrevious,
    /// Open the command palette.
    OpenPalette,
    /// A workspace hotkey.
    LoadWorkspace(usize),
}

impl AppState {
    /// Hosted windows in zone reading order (left to right, top to bottom).
    pub fn ordered_windows(&self) -> Vec<WindowId> {
        self.layout
            .leaves()
            .into_iter()
            .filter_map(|zone| self.attachments.iter().find(|a| a.zone == zone).map(|a| a.window))
            .collect()
    }
}

pub(super) fn run(state: &mut AppState, command: Command) -> Vec<Effect> {
    match command {
        Command::MarginUp => adjust_margin(state, MARGIN_STEP),
        Command::MarginDown => adjust_margin(state, -MARGIN_STEP),
        Command::BringToFront => {
            state.cycle = None;
            state.stay_back = false;
            vec![Effect::StayBackIndicator(false), Effect::BringToFront, Effect::Repaint]
        }
        Command::SendToBack => {
            state.cycle = None;
            state.stay_back = true;
            vec![Effect::SendToBack, Effect::StayBackIndicator(true), Effect::Repaint]
        }
        Command::ResetLayout => {
            state.layout = GridLayout::uniform_grid(2, 2);
            state.cycle = None;
            vec![Effect::Repaint]
        }
        // Cycling needs to know about other instances; the platform asks with `Msg::Cycle`.
        Command::CycleNext | Command::CyclePrevious => vec![],
        Command::OpenPalette => vec![Effect::ShowPalette],
        Command::LoadWorkspace(slot) => super::workspace::load(state, slot),
    }
}

fn adjust_margin(state: &mut AppState, delta: f64) -> Vec<Effect> {
    state.margin = (state.margin + delta).clamp(0.0, MAX_MARGIN as f64);
    vec![]
}

/// One step of Win+Alt+] / [. At the last (or first) window, hands over to the
/// neighbouring instance if there is one; otherwise wraps around.
pub(super) fn cycle(state: &mut AppState, forward: bool, has_peer: bool) -> Vec<Effect> {
    let windows = state.ordered_windows();
    let current = state.cycle.and_then(|w| windows.iter().position(|x| *x == w));
    match next_index(current, windows.len(), forward, has_peer) {
        Step::HandOff => {
            state.cycle = None;
            vec![Effect::HandOffCycle { forward }, Effect::Repaint]
        }
        Step::Stay => vec![],
        Step::Focus(i) => focus(state, windows[i]),
    }
}

/// A neighbouring instance handed the cycle to us: start from our first (or last) window.
pub(super) fn begin_cycle_at_edge(state: &mut AppState, forward: bool) -> Vec<Effect> {
    let windows = state.ordered_windows();
    match if forward { windows.first() } else { windows.last() } {
        Some(&window) => focus(state, window),
        // Nothing here to focus: pass it on so an empty instance doesn't stall the cycle.
        None => vec![Effect::HandOffCycle { forward }],
    }
}

/// Highlights straight away rather than waiting for the foreground event to come back.
fn focus(state: &mut AppState, window: WindowId) -> Vec<Effect> {
    state.cycle = Some(window);
    state.active = Some(window);
    vec![Effect::Focus(window), Effect::Repaint]
}

enum Step {
    Focus(usize),
    HandOff,
    Stay,
}

fn next_index(current: Option<usize>, count: usize, forward: bool, has_peer: bool) -> Step {
    if count == 0 {
        return if has_peer { Step::HandOff } else { Step::Stay };
    }
    let Some(i) = current else {
        return Step::Focus(if forward { 0 } else { count - 1 });
    };
    let at_edge = if forward { i == count - 1 } else { i == 0 };
    if at_edge && has_peer {
        return Step::HandOff;
    }
    Step::Focus(if forward { (i + 1) % count } else { (i + count - 1) % count })
}
