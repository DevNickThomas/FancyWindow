//! What the status bar says, as data. The view lays it out and paints it; clicks
//! come back as the segment's `StatusClick`.

use crate::model::{GridLayout, WORKSPACE_SLOTS};

use super::workspace::display_name;
use super::{AppState, Command};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentKind {
    /// The workspace and profile, on a stronger background.
    Strong,
    Plain,
    /// Something to notice, like being held at the back.
    Warning,
}

/// What clicking a segment does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusClick {
    /// Pop up the Workspaces menu.
    WorkspacesMenu,
    /// Pop up the Layout menu.
    LayoutMenu,
    /// Left click widens the margin, right click narrows it.
    Margin,
    /// Bring Fancy Window forward.
    BringForward,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Segment {
    pub kind: SegmentKind,
    pub text: String,
    pub click: Option<StatusClick>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusBar {
    /// Segments from the left edge.
    pub left: Vec<Segment>,
    /// A shortcut hint shown right-aligned, before the "?" and the version.
    pub hint: Option<String>,
}

impl AppState {
    /// The saved workspace whose layout is on screen now. Any edit to the layout
    /// (even a splitter drag) means it no longer is.
    pub fn current_workspace(&self) -> Option<usize> {
        let mut cache = self.parsed_workspaces.borrow_mut();
        cache.resize(WORKSPACE_SLOTS, None);
        (0..WORKSPACE_SLOTS).find(|&slot| {
            let Some(ws) = self.workspace(slot) else { return false };
            // Re-parse only when the slot's JSON changed since last time.
            if !cache[slot].as_ref().is_some_and(|(json, _)| *json == ws.layout_json) {
                cache[slot] = Some((ws.layout_json.clone(), GridLayout::from_json(&ws.layout_json).ok()));
            }
            cache[slot].as_ref().and_then(|(_, layout)| layout.as_ref()) == Some(&self.layout)
        })
    }

    pub fn status_bar(&self) -> StatusBar {
        let segment = |kind, text: String, click| Segment { kind, text, click: Some(click) };
        let workspace = self.current_workspace().and_then(|slot| self.workspace(slot)).map(|ws| display_name(ws).to_string());
        let profile = self.settings.window_name.clone().or_else(|| self.profile.as_ref().map(|p| format!("Profile: {p}")));
        // Always shown, so the Workspaces menu is one click away even before the first save.
        let strong = match (workspace, profile) {
            (Some(w), Some(p)) if w != p => if self.settings.window_name.is_some() {
                format!("{p} \u{00B7} {w}")
            } else { format!("{w} \u{00B7} {p}") },
            (Some(w), Some(_)) => w,
            (Some(w), None) => w,
            (None, Some(p)) => p,
            (None, None) => "No workspace".into(),
        };
        let mut left = vec![segment(SegmentKind::Strong, strong, StatusClick::WorkspacesMenu)];
        if self.stay_back {
            let back = match self.chord_label(Command::BringToFront) {
                Some(chord) => format!("Held at the back \u{00B7} {chord} brings it forward"),
                None => "Held at the back".into(),
            };
            left.push(segment(SegmentKind::Warning, back, StatusClick::BringForward));
        }
        let zones = self.layout.leaves().len();
        let hosted = self.attachments.len();
        left.push(segment(SegmentKind::Plain, format!("{} \u{00B7} {hosted} hosted", plural(zones, "zone")), StatusClick::LayoutMenu));
        left.push(segment(SegmentKind::Plain, format!("Margin {}", self.margin), StatusClick::Margin));
        let hint = (hosted > 1).then(|| self.chord_label(Command::CycleNext)).flatten().map(|chord| format!("{chord} next window"));
        StatusBar { left, hint }
    }
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 { format!("1 {noun}") } else { format!("{n} {noun}s") }
}
