use std::collections::HashMap;

use crate::model::{GridLayout, Orientation, Point, Rect, Settings, SplitterHandle, ZoneRect};

use super::{Attachment, Frame, Modifiers, WindowId};

/// Thickness of the line that previews where a Ctrl/Shift+click will split.
pub const PREVIEW_THICKNESS: f64 = 2.0;

/// Everything the app knows. The view draws from this; `update` changes it.
#[derive(Clone, Debug)]
pub struct AppState {
    pub layout: GridLayout,
    pub frame: Frame,
    pub drag: Option<Drag>,
    /// Last mouse position over the canvas, if the mouse is over it.
    pub hover: Option<Point>,
    pub mods: Modifiers,
    /// Hosted windows, one per zone.
    pub attachments: Vec<Attachment>,
    /// Extra gap (DIPs) between a hosted window and its zone outline.
    pub margin: f64,
    /// Persisted settings as loaded; layout and margin above are the live copies.
    pub settings: Settings,
    /// `--profile` name; `None` for the default profile.
    pub profile: Option<String>,
    /// The hosted window the last Win+Alt+] / [ focused, or the user last activated.
    pub cycle: Option<WindowId>,
    /// The foreground window, if it is one of ours; its zone is drawn with the active glow.
    pub active: Option<WindowId>,
    /// Sticky Win+Alt+PageDown: stay at the back, even after attaching windows.
    pub stay_back: bool,
    /// Hosted windows' titles, as the platform last read them (for the zone headers).
    pub titles: HashMap<WindowId, String>,
}

/// A splitter being dragged, and where the mouse was last time it moved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drag {
    pub handle: SplitterHandle,
    pub last: Point,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CursorKind {
    Arrow,
    SizeWestEast,
    SizeNorthSouth,
    /// Over a zone header's ×.
    Hand,
}

impl AppState {
    pub fn new() -> Self {
        Self::from_settings(Settings::default(), None)
    }

    /// Restores the last layout and margin; falls back to a 2x2 grid if the layout is missing or bad.
    pub fn from_settings(settings: Settings, profile: Option<String>) -> Self {
        let layout = settings.last_layout_json.as_deref().and_then(|j| GridLayout::from_json(j).ok());
        Self {
            layout: layout.unwrap_or_else(|| GridLayout::uniform_grid(2, 2)),
            frame: Frame::default(),
            drag: None,
            hover: None,
            mods: Modifiers::default(),
            attachments: Vec::new(),
            margin: settings.window_margin as f64,
            settings,
            profile,
            cycle: None,
            active: None,
            stay_back: false,
            titles: HashMap::new(),
        }
    }

    pub fn zone_rects(&self) -> Vec<ZoneRect> {
        self.layout.zone_rects(self.frame.canvas)
    }

    pub fn splitters(&self) -> Vec<SplitterHandle> {
        self.layout.splitters(self.frame.canvas)
    }

    /// Topmost splitter under the point (splitters sit above zones).
    pub fn splitter_at(&self, p: Point) -> Option<SplitterHandle> {
        self.splitters().into_iter().rev().find(|s| s.bounds.contains(p))
    }

    /// The line showing where a Ctrl (vertical) or Shift (horizontal) click would split.
    pub fn split_preview(&self) -> Option<Rect> {
        let p = self.hover?;
        if self.drag.is_some() || self.splitter_at(p).is_some() {
            return None;
        }
        let zone = self.zone_rects().into_iter().find(|z| z.bounds.contains(p))?.bounds;
        let half = PREVIEW_THICKNESS / 2.0;
        if self.mods.ctrl {
            Some(Rect::new(zone.x + zone.width / 2.0 - half, zone.y, PREVIEW_THICKNESS, zone.height))
        } else if self.mods.shift {
            Some(Rect::new(zone.x, zone.y + zone.height / 2.0 - half, zone.width, PREVIEW_THICKNESS))
        } else {
            None
        }
    }

    pub fn cursor_at(&self, p: Point) -> CursorKind {
        let orientation = match (self.drag, self.splitter_at(p)) {
            (Some(d), _) => d.handle.orientation,
            (None, Some(s)) => s.orientation,
            (None, None) if matches!(self.header_at(p), Some(super::HeaderHit::Close(_))) => return CursorKind::Hand,
            (None, None) => return CursorKind::Arrow,
        };
        match orientation {
            Orientation::Columns => CursorKind::SizeWestEast,
            Orientation::Rows => CursorKind::SizeNorthSouth,
        }
    }
}

impl AppState {
    /// The settings to write now: what was loaded, plus the live layout and margin.
    pub fn current_settings(&self) -> Settings {
        let mut s = self.settings.clone();
        s.last_layout_json = Some(self.layout.to_json());
        s.window_margin = self.margin as i32;
        s
    }

    pub fn title(&self) -> String {
        match &self.profile {
            Some(p) => format!("Fancy Window [{p}]"),
            None => "Fancy Window".into(),
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
