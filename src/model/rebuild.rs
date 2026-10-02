//! Joining zones: merge two rectangles, then rebuild a tree from the flat list.

use super::{GridLayout, GridNode, JoinDirection, LayoutError, Orientation, Rect, SplitChild, SplitId, SplitNode, ZoneId, ZoneRect};
use super::layout::LayoutResult;

const TOLERANCE: f64 = 0.5;

impl GridLayout {
    pub fn can_join(&self, zone: ZoneId, direction: JoinDirection, bounds: Rect) -> bool {
        let rects = self.zone_rects(bounds);
        matching_neighbor(&rects, zone, direction).is_some()
    }

    /// Merges a zone with the neighbour that shares its full edge. The clicked zone keeps its id.
    pub fn join(&self, zone: ZoneId, direction: JoinDirection, bounds: Rect) -> LayoutResult {
        let rects = self.zone_rects(bounds);
        let subject = rects.iter().find(|r| r.id == zone).ok_or(LayoutError::ZoneNotFound(zone))?;
        let neighbor = matching_neighbor(&rects, zone, direction).ok_or(LayoutError::NoNeighbor(direction, zone))?;
        let merged = ZoneRect { id: zone, bounds: subject.bounds.union(&neighbor.bounds) };
        let mut remaining: Vec<ZoneRect> =
            rects.iter().filter(|r| r.id != zone && r.id != neighbor.id).copied().collect();
        remaining.push(merged);
        build_from_rects(&remaining).map(GridLayout::new)
    }
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < TOLERANCE
}

fn matching_neighbor(rects: &[ZoneRect], zone: ZoneId, direction: JoinDirection) -> Option<ZoneRect> {
    let s = rects.iter().find(|r| r.id == zone)?.bounds;
    rects.iter().filter(|r| r.id != zone).copied().find(|other| {
        let n = other.bounds;
        match direction {
            JoinDirection::Left => near(n.right(), s.x) && near(n.y, s.y) && near(n.height, s.height),
            JoinDirection::Right => near(s.right(), n.x) && near(n.y, s.y) && near(n.height, s.height),
            JoinDirection::Up => near(n.bottom(), s.y) && near(n.x, s.x) && near(n.width, s.width),
            JoinDirection::Down => near(s.bottom(), n.y) && near(n.x, s.x) && near(n.width, s.width),
        }
    })
}

/// Rebuilds a guillotine tree from rectangles that tile some area: find a clean
/// horizontal cut (then vertical), recurse on each side. Weights are the real
/// extents so geometry is preserved.
fn build_from_rects(rects: &[ZoneRect]) -> Result<GridNode, LayoutError> {
    if let [only] = rects {
        return Ok(GridNode::leaf(only.id));
    }
    let row_cut = try_cut(rects, Orientation::Rows, |r| (r.y, r.bottom()))?;
    if let Some(node) = row_cut {
        return Ok(node);
    }
    try_cut(rects, Orientation::Columns, |r| (r.x, r.right()))?.ok_or(LayoutError::NotGuillotine)
}

/// Tries every edge on one axis as a cut line. `span` gives a rect's (start, end) on that axis.
fn try_cut(
    rects: &[ZoneRect],
    orientation: Orientation,
    span: fn(&Rect) -> (f64, f64),
) -> Result<Option<GridNode>, LayoutError> {
    let mut edges: Vec<f64> = rects.iter().flat_map(|r| { let (a, b) = span(&r.bounds); [a, b] }).collect();
    edges.sort_by(f64::total_cmp);
    for cut in edges {
        let before: Vec<ZoneRect> = rects.iter().filter(|r| span(&r.bounds).1 <= cut + TOLERANCE).copied().collect();
        let after: Vec<ZoneRect> = rects.iter().filter(|r| span(&r.bounds).0 >= cut - TOLERANCE).copied().collect();
        if before.is_empty() || after.is_empty() || before.len() + after.len() != rects.len() {
            continue;
        }
        let children = vec![
            SplitChild { node: build_from_rects(&before)?, weight: extent(&before, span) },
            SplitChild { node: build_from_rects(&after)?, weight: extent(&after, span) },
        ];
        return Ok(Some(GridNode::Split(SplitNode { id: SplitId::new(), orientation, children })));
    }
    Ok(None)
}

fn extent(rects: &[ZoneRect], span: fn(&Rect) -> (f64, f64)) -> f64 {
    let start = rects.iter().map(|r| span(&r.bounds).0).fold(f64::INFINITY, f64::min);
    let end = rects.iter().map(|r| span(&r.bounds).1).fold(f64::NEG_INFINITY, f64::max);
    end - start
}
