use thiserror::Error;

use super::{GridNode, Orientation, Point, Rect, SplitChild, SplitId, SplitNode, SplitterHandle, ZoneId, ZoneRect};

/// Smallest weight a child can be squeezed to by dragging a splitter.
pub const MIN_WEIGHT: f64 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Error, PartialEq)]
pub enum LayoutError {
    #[error("zone {0} was not found in the layout")]
    ZoneNotFound(ZoneId),
    #[error("split {0} was not found in the layout")]
    SplitNotFound(SplitId),
    #[error("child index {0} is out of range")]
    IndexOutOfRange(usize),
    #[error("cannot remove the only zone in the layout")]
    LastZone,
    #[error("no matching-extent neighbor {0:?} of zone {1}")]
    NoNeighbor(JoinDirection, ZoneId),
    #[error("zone rectangles do not form a guillotine partition")]
    NotGuillotine,
    #[error("invalid layout json: {0}")]
    Json(String),
}

pub type LayoutResult = Result<GridLayout, LayoutError>;

/// An immutable layout tree. Every edit returns a new layout.
#[derive(Clone, Debug, PartialEq)]
pub struct GridLayout {
    root: GridNode,
}

impl GridLayout {
    pub fn new(root: GridNode) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &GridNode {
        &self.root
    }

    pub fn equal_columns(count: usize) -> Self {
        Self::new(equal_split(Orientation::Columns, count))
    }

    pub fn equal_rows(count: usize) -> Self {
        Self::new(equal_split(Orientation::Rows, count))
    }

    pub fn uniform_grid(rows: usize, columns: usize) -> Self {
        let row_nodes = (0..rows).map(|_| equal_split(Orientation::Columns, columns)).collect();
        Self::new(GridNode::split(Orientation::Rows, row_nodes))
    }

    pub fn leaves(&self) -> Vec<ZoneId> {
        self.root.leaves()
    }

    pub fn zone_rects(&self, bounds: Rect) -> Vec<ZoneRect> {
        self.root.compute_rects(bounds)
    }

    pub fn splitters(&self, bounds: Rect) -> Vec<SplitterHandle> {
        self.root.splitters(bounds)
    }

    pub fn hit_test(&self, bounds: Rect, point: Point) -> Option<ZoneId> {
        self.zone_rects(bounds).into_iter().find(|z| z.bounds.contains(point)).map(|z| z.id)
    }

    /// Splits a zone in two. The original id stays in the first child so anything
    /// attached to it (a hosted window) survives the split.
    pub fn split_zone(&self, zone: ZoneId, orientation: Orientation) -> LayoutResult {
        let mut root = self.root.clone();
        let target = find_mut(&mut root, &|n| n.leaf_id() == Some(zone)).ok_or(LayoutError::ZoneNotFound(zone))?;
        *target = GridNode::split(orientation, vec![GridNode::leaf(zone), GridNode::new_leaf()]);
        Ok(Self::new(root))
    }

    /// Removes a zone; its siblings absorb the space and single-child splits collapse.
    pub fn remove_zone(&self, zone: ZoneId) -> LayoutResult {
        if self.root.leaf_id() == Some(zone) {
            return Err(LayoutError::LastZone);
        }
        let mut root = self.root.clone();
        if !remove_leaf(&mut root, zone) {
            return Err(LayoutError::ZoneNotFound(zone));
        }
        Ok(Self::new(root))
    }

    /// Removes the splitter after `left` in a split; the left child absorbs the right one.
    pub fn remove_splitter(&self, split: SplitId, left: usize) -> LayoutResult {
        let mut root = self.root.clone();
        let node = find_split_mut(&mut root, split)?;
        let GridNode::Split(s) = node else { unreachable!() };
        if left + 1 >= s.children.len() {
            return Err(LayoutError::IndexOutOfRange(left));
        }
        let right = s.children.remove(left + 1);
        s.children[left].weight += right.weight;
        collapse_if_single(node);
        Ok(Self::new(root))
    }

    pub fn set_weight(&self, split: SplitId, child: usize, weight: f64) -> LayoutResult {
        let mut root = self.root.clone();
        let GridNode::Split(s) = find_split_mut(&mut root, split)? else { unreachable!() };
        s.children.get_mut(child).ok_or(LayoutError::IndexOutOfRange(child))?.weight = weight;
        Ok(Self::new(root))
    }

    /// Drags a splitter by `pixel_delta`, keeping both neighbours at least `MIN_WEIGHT`.
    pub fn move_splitter(&self, split: SplitId, left: usize, pixel_delta: f64, bounds: Rect) -> LayoutResult {
        let (s, split_bounds) = find_split_bounds(&self.root, bounds, split).ok_or(LayoutError::SplitNotFound(split))?;
        if left + 1 >= s.children.len() {
            return Err(LayoutError::IndexOutOfRange(left));
        }
        let left_weight = s.children[left].weight;
        let right_weight = s.children[left + 1].weight;
        let delta = (pixel_delta / s.extent(split_bounds) * s.total_weight())
            .clamp(-(left_weight - MIN_WEIGHT), right_weight - MIN_WEIGHT);
        self.set_weight(split, left, left_weight + delta)?.set_weight(split, left + 1, right_weight - delta)
    }

    /// Adds an empty zone on the outside edge. Appends to the root split when its
    /// orientation already matches, otherwise wraps the root.
    pub fn add_outer(&self, orientation: Orientation, at_end: bool) -> Self {
        let new_child = SplitChild { node: GridNode::new_leaf(), weight: 1.0 };
        if let GridNode::Split(s) = &self.root
            && s.orientation == orientation
        {
            let mut s = s.clone();
            let index = if at_end { s.children.len() } else { 0 };
            s.children.insert(index, new_child);
            return Self::new(GridNode::Split(s));
        }
        let existing = SplitChild { node: self.root.clone(), weight: 1.0 };
        let children = if at_end { vec![existing, new_child] } else { vec![new_child, existing] };
        Self::new(GridNode::Split(SplitNode { id: SplitId::new(), orientation, children }))
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&self.root).expect("layout always serializes")
    }

    pub fn from_json(json: &str) -> LayoutResult {
        serde_json::from_str(json).map(Self::new).map_err(|e| LayoutError::Json(e.to_string()))
    }
}

fn equal_split(orientation: Orientation, count: usize) -> GridNode {
    assert!(count >= 1, "count must be at least 1");
    if count == 1 {
        return GridNode::new_leaf();
    }
    GridNode::split(orientation, (0..count).map(|_| GridNode::new_leaf()).collect())
}

/// Depth-first search for the first node matching `pred`.
fn find_mut<'a>(node: &'a mut GridNode, pred: &dyn Fn(&GridNode) -> bool) -> Option<&'a mut GridNode> {
    if pred(node) {
        return Some(node);
    }
    match node {
        GridNode::Split(s) => s.children.iter_mut().find_map(|c| find_mut(&mut c.node, pred)),
        GridNode::Leaf { .. } => None,
    }
}

fn find_split_mut(root: &mut GridNode, split: SplitId) -> Result<&mut GridNode, LayoutError> {
    find_mut(root, &|n| n.as_split().is_some_and(|s| s.id == split)).ok_or(LayoutError::SplitNotFound(split))
}

fn find_split_bounds(node: &GridNode, bounds: Rect, split: SplitId) -> Option<(&SplitNode, Rect)> {
    let s = node.as_split()?;
    if s.id == split {
        return Some((s, bounds));
    }
    s.children.iter().zip(s.child_bounds(bounds)).find_map(|(c, b)| find_split_bounds(&c.node, b, split))
}

fn remove_leaf(node: &mut GridNode, zone: ZoneId) -> bool {
    let GridNode::Split(s) = node else { return false };
    let mut removed = false;
    s.children.retain_mut(|c| {
        if c.node.leaf_id() == Some(zone) {
            removed = true;
            return false;
        }
        removed |= remove_leaf(&mut c.node, zone);
        true
    });
    collapse_if_single(node);
    removed
}

/// Replaces a split that has a single child with that child.
fn collapse_if_single(node: &mut GridNode) {
    if let GridNode::Split(s) = node
        && s.children.len() == 1
    {
        *node = s.children.pop().unwrap().node;
    }
}
