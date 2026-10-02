//! Where the status bar's segments, hint, "?" and version sit, and what a click hits.
//! Pure (client pixels in, rects out) so it can be tested; `paint` measures the text.

use crate::model::{Point, Rect};

/// Status bar height in DIPs.
pub const STATUS_BAR_HEIGHT: f64 = 24.0;
/// Space either side of a segment's content.
const SEGMENT_PADDING: f64 = 9.0;
/// Gap from the right edge to the version.
const EDGE_PADDING: f64 = 10.0;
const HELP_WIDTH: f64 = 24.0;
const HELP_GAP: f64 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusHit {
    Segment(usize),
    /// The "?": keyboard shortcuts.
    Help,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StatusLayout {
    pub bar: Rect,
    pub segments: Vec<Rect>,
    /// Left out when it would run into the segments.
    pub hint: Option<Rect>,
    pub help: Rect,
    pub version: Rect,
}

/// Lays out a status bar along the bottom of a `width` x `height` client area.
/// `contents` are each segment's content width (icon and text) in pixels.
pub fn layout(width: f64, height: f64, scale: f64, contents: &[f64], hint: Option<f64>, version: f64) -> StatusLayout {
    let px = |dip: f64| (dip * scale).round();
    let bar_height = px(STATUS_BAR_HEIGHT);
    let bar = Rect::new(0.0, height - bar_height, width, bar_height);
    let mut x = 0.0;
    let segments: Vec<Rect> = contents
        .iter()
        .map(|w| {
            let r = Rect::new(x, bar.y, w + 2.0 * px(SEGMENT_PADDING), bar_height);
            x = r.right();
            r
        })
        .collect();
    let version = Rect::new(width - px(EDGE_PADDING) - version, bar.y, version, bar_height);
    let help = Rect::new(version.x - px(HELP_GAP) - px(HELP_WIDTH), bar.y, px(HELP_WIDTH), bar_height);
    let hint = hint.map(|w| w + 2.0 * px(SEGMENT_PADDING)).filter(|w| help.x - w >= x).map(|w| Rect::new(help.x - w, bar.y, w, bar_height));
    StatusLayout { bar, segments, hint, help, version }
}

impl StatusLayout {
    pub fn hit(&self, p: Point) -> Option<StatusHit> {
        if !self.bar.contains(p) {
            return None;
        }
        if self.help.contains(p) {
            return Some(StatusHit::Help);
        }
        self.segments.iter().position(|r| r.contains(p)).map(StatusHit::Segment)
    }
}
