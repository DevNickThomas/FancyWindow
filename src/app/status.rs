//! What the status bar says, as data. The view lays it out and paints it.

use crate::model::{GridLayout, WORKSPACE_SLOTS};

use super::workspace::display_name;
use super::{AppState, Command};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Segment {
    /// The workspace or profile in use, on a stronger background.
    Strong(String),
    Plain(String),
    /// Something to notice, like being held at the back.
    Warning(String),
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
        (0..WORKSPACE_SLOTS).find(|&slot| {
            self.workspace(slot).and_then(|ws| GridLayout::from_json(&ws.layout_json).ok()).is_some_and(|l| l == self.layout)
        })
    }

    pub fn status_bar(&self) -> StatusBar {
        let mut left = Vec::new();
        let workspace = self.current_workspace().and_then(|slot| self.workspace(slot)).map(|ws| display_name(ws).to_string());
        let profile = self.profile.as_ref().map(|p| format!("Profile: {p}"));
        let strong: Vec<String> = workspace.into_iter().chain(profile).collect();
        if !strong.is_empty() {
            left.push(Segment::Strong(strong.join(" \u{00B7} ")));
        }
        if self.stay_back {
            let back = match self.chord_label(Command::BringToFront) {
                Some(chord) => format!("Held at the back \u{00B7} {chord} brings it forward"),
                None => "Held at the back".into(),
            };
            left.push(Segment::Warning(back));
        }
        let zones = self.layout.leaves().len();
        let hosted = self.attachments.len();
        left.push(Segment::Plain(format!("{} \u{00B7} {hosted} hosted", plural(zones, "zone"))));
        left.push(Segment::Plain(format!("Margin {}", self.margin)));
        let hint = (hosted > 1).then(|| self.chord_label(Command::CycleNext)).flatten().map(|chord| format!("{chord} next window"));
        StatusBar { left, hint }
    }
}

fn plural(n: usize, noun: &str) -> String {
    if n == 1 { format!("1 {noun}") } else { format!("{n} {noun}s") }
}
