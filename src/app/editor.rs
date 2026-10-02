//! Contextual pane selection shared by the mouse and keyboard layout editor.
use super::AppState;
use crate::model::{JoinDirection, ZoneId};

impl AppState {
    /// Activating our toolbar clears `active`, so retain the last hosted focus.
    pub fn editing_zone(&self) -> ZoneId {
        self.active.and_then(|w| self.zone_of(w))
            .or_else(|| self.last_focused.and_then(|w| self.zone_of(w)))
            .or_else(|| self.cycle.and_then(|w| self.zone_of(w)))
            .unwrap_or(self.layout.leaves()[0])
    }

    pub fn next_editing_zone(&self, selected: ZoneId, forward: bool) -> ZoneId {
        let zones = self.layout.leaves();
        let i = zones.iter().position(|z| *z == selected).unwrap_or(0);
        zones[(i + if forward { 1 } else { zones.len() - 1 }) % zones.len()]
    }

    /// Prefer a neighbour in the same row/column, then the closest centre.
    pub fn adjacent_editing_zone(&self, selected: ZoneId, direction: JoinDirection) -> ZoneId {
        let zones = self.zone_rects();
        let Some(from) = zones.iter().find(|z| z.id == selected) else { return self.editing_zone() };
        let centre = |r: crate::model::Rect| (r.x + r.width / 2.0, r.y + r.height / 2.0);
        let (x, y) = centre(from.bounds);
        zones.iter().filter(|z| z.id != selected).filter_map(|z| {
            let (nx, ny) = centre(z.bounds);
            let (along, across) = match direction {
                JoinDirection::Left => (x - nx, (ny - y).abs()),
                JoinDirection::Right => (nx - x, (ny - y).abs()),
                JoinDirection::Up => (y - ny, (nx - x).abs()),
                JoinDirection::Down => (ny - y, (nx - x).abs()),
            };
            (along > 0.5).then_some((z.id, across * 4.0 + along))
        }).min_by(|a, b| a.1.total_cmp(&b.1)).map_or(selected, |(id, _)| id)
    }

    pub fn editing_zone_label(&self, selected: ZoneId) -> String {
        let number = self.layout.leaves().iter().position(|z| *z == selected).unwrap_or(0) + 1;
        let title = self.attachments.iter().find(|a| a.zone == selected)
            .map(|a| self.titles.get(&a.window).map(String::as_str).unwrap_or("Hosted window"))
            .unwrap_or("Empty pane");
        format!("Pane {number}: {title}")
    }
}
