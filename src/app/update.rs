use crate::model::{Orientation, Point};

use super::attach::{foreground_changed, forget_all, raise_all, reflow, window_dropped};
use super::command::{self, begin_cycle_at_edge, cycle};
use super::{headers, hotkeys, menu, workspace};
use super::{AppState, Button, Drag, Effect, Modifiers, Msg, WindowId};

/// Ignore drags smaller than this many DIPs.
const MIN_DRAG_STEP: f64 = 0.5;

/// The single place where state changes. Returns what the platform must do next.
///
/// Whenever the layout, frame, margin or header height changes, hosted windows are
/// moved to match. Whenever anything the canvas or status bar shows changes, it is repainted.
pub fn update(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    let placement = |s: &AppState| (s.layout.clone(), s.frame, s.margin, s.header_height());
    let before = placement(state);
    let shown = |s: &AppState| (s.layout.clone(), s.margin, s.attachments.clone(), s.stay_back, s.active, s.header_height());
    let shown_before = shown(state);
    // A window about to be destroyed needs no repaint, and saving stays the last effect.
    let closing = matches!(msg, Msg::Closing { .. });
    let mut effects = handle(state, msg);
    if placement(state) != before {
        effects.extend(reflow(state));
    }
    if !closing && shown(state) != shown_before && !effects.contains(&Effect::Repaint) {
        effects.push(Effect::Repaint);
    }
    // Titles are only kept for windows still hosted.
    let hosted: Vec<WindowId> = state.attachments.iter().map(|a| a.window).collect();
    state.titles.retain(|w, _| hosted.contains(w));
    effects
}

fn handle(state: &mut AppState, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::WindowDropped { window, at, alt } => {
            let mut effects = window_dropped(state, window, at, alt);
            // Attaching activates Fancy Window; in stay-back mode, sink it again.
            if state.stay_back && effects.iter().any(|e| matches!(e, Effect::Host { .. })) {
                effects.push(Effect::SendToBack);
            }
            effects
        }
        Msg::WindowClosed(window) => {
            state.attachments.retain(|a| a.window != window);
            vec![]
        }
        Msg::Activated => raise_all(state),
        Msg::ForegroundChanged(window) => foreground_changed(state, window),
        Msg::TitleChanged { window, title } => headers::title_changed(state, window, title),
        Msg::Closing { bounds } => {
            state.settings = state.current_settings();
            state.settings.set_bounds(bounds);
            let mut effects = forget_all(state);
            effects.push(Effect::SaveSettings(state.settings.clone()));
            effects
        }
        Msg::FrameChanged(frame) => {
            state.frame = frame;
            vec![Effect::Repaint]
        }
        Msg::MouseDown { at, button, mods } => {
            // Clicking Fancy Window is a clear sign the user wants it back: leave stay-back.
            let mut effects = if std::mem::take(&mut state.stay_back) { vec![Effect::StayBackIndicator(false)] } else { vec![] };
            effects.extend(mouse_down(state, at, button, mods));
            effects
        }
        Msg::MouseMove { at, mods } => mouse_move(state, at, mods),
        Msg::MouseUp => match state.drag.take() {
            Some(_) => vec![Effect::ReleaseMouse, Effect::Repaint],
            None => vec![],
        },
        Msg::CaptureLost => {
            state.drag = None;
            vec![Effect::Repaint]
        }
        Msg::MouseLeft => repaint_if_preview_changed(state, |s| s.hover = None),
        Msg::ModifiersChanged(mods) => repaint_if_preview_changed(state, |s| s.mods = mods),
        Msg::Command(c) => command::run(state, c),
        Msg::Menu(action) => menu::run(state, action),
        Msg::WorkspaceNamed { slot, name, saved_at_utc } => workspace::save(state, slot, &name, saved_at_utc),
        Msg::SetHotkey { command, chord } => hotkeys::set(state, command, chord),
        Msg::HotkeyFailed { command } => hotkeys::failed(state, command),
        Msg::ResetHotkeys => hotkeys::reset(state),
        Msg::WorkspaceDeleteConfirmed { slot } => workspace::delete(state, slot),
        Msg::SetTheme(name) => {
            state.settings.theme_name = name;
            vec![Effect::ApplyTheme, Effect::Repaint, Effect::SaveSettings(state.current_settings())]
        }
        Msg::SetAccent(hex) => {
            state.settings.accent_color = hex;
            vec![Effect::ApplyTheme, Effect::Repaint, Effect::SaveSettings(state.current_settings())]
        }
        Msg::Cycle { forward, has_peer } => cycle(state, forward, has_peer),
        Msg::BeginCycleAtEdge { forward } => begin_cycle_at_edge(state, forward),
    }
}

/// Splitter: left drags, right merges. Zone: Ctrl+left splits into columns, Shift+left into rows.
fn mouse_down(state: &mut AppState, at: Point, button: Button, mods: Modifiers) -> Vec<Effect> {
    state.mods = mods;
    if let Some(handle) = state.splitter_at(at) {
        return match button {
            Button::Left => {
                state.drag = Some(Drag { handle, last: at });
                vec![Effect::CaptureMouse]
            }
            Button::Right => {
                state.layout = state
                    .layout
                    .remove_splitter(handle.split_id, handle.left_child_index)
                    .expect("splitter comes from the current layout");
                vec![Effect::Repaint]
            }
        };
    }
    // A plain click on a header; with Ctrl or Shift it splits the zone as anywhere else.
    if button == Button::Left && mods == Modifiers::default() {
        if let Some(hit) = state.header_at(at) {
            return headers::clicked(state, hit);
        }
    }
    if button == Button::Right && mods.ctrl {
        return match state.layout.hit_test(state.frame.canvas, at) {
            Some(zone) => vec![Effect::ShowZoneMenu { zone, at }],
            None => vec![],
        };
    }
    let orientation = match (button, mods.ctrl, mods.shift) {
        (Button::Left, true, _) => Orientation::Columns,
        (Button::Left, false, true) => Orientation::Rows,
        _ => return vec![],
    };
    let Some(zone) = state.layout.hit_test(state.frame.canvas, at) else { return vec![] };
    state.layout = state.layout.split_zone(zone, orientation).expect("zone comes from the current layout");
    vec![Effect::Repaint]
}

fn mouse_move(state: &mut AppState, at: Point, mods: Modifiers) -> Vec<Effect> {
    let Some(drag) = state.drag else {
        return repaint_if_preview_changed(state, |s| {
            s.hover = Some(at);
            s.mods = mods;
        });
    };
    let delta = match drag.handle.orientation {
        Orientation::Columns => at.x - drag.last.x,
        Orientation::Rows => at.y - drag.last.y,
    };
    if delta.abs() < MIN_DRAG_STEP {
        return vec![];
    }
    let handle = drag.handle;
    state.layout = state
        .layout
        .move_splitter(handle.split_id, handle.left_child_index, delta, state.frame.canvas)
        .expect("dragged splitter comes from the current layout");
    state.drag = Some(Drag { handle, last: at });
    vec![Effect::Repaint]
}

fn repaint_if_preview_changed(state: &mut AppState, change: impl FnOnce(&mut AppState)) -> Vec<Effect> {
    let before = state.split_preview();
    change(state);
    if state.split_preview() != before { vec![Effect::Repaint] } else { vec![] }
}
