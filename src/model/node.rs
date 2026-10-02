use serde::{Deserialize, Serialize};

use super::{Rect, SplitId, ZoneId, ZoneRect};

pub const SPLITTER_THICKNESS: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Orientation {
    /// Children sit side by side.
    Columns,
    /// Children stack top to bottom.
    Rows,
}

/// A node in the layout tree: either a zone (leaf) or a split into weighted children.
///
/// JSON shape matches the .NET app: `{"$type":"leaf","Id":"..."}` / `{"$type":"split",...}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "$type")]
pub enum GridNode {
    #[serde(rename = "leaf")]
    Leaf {
        #[serde(rename = "Id")]
        id: ZoneId,
    },
    #[serde(rename = "split")]
    Split(SplitNode),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SplitNode {
    pub id: SplitId,
    pub orientation: Orientation,
    pub children: Vec<SplitChild>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SplitChild {
    pub node: GridNode,
    pub weight: f64,
}

/// The draggable bar between child `left_child_index` and the one after it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplitterHandle {
    pub split_id: SplitId,
    pub left_child_index: usize,
    pub orientation: Orientation,
    pub bounds: Rect,
}

impl GridNode {
    pub fn leaf(id: ZoneId) -> Self {
        GridNode::Leaf { id }
    }

    pub fn new_leaf() -> Self {
        Self::leaf(ZoneId::new())
    }

    /// A new split whose children all have weight 1.
    pub fn split(orientation: Orientation, nodes: Vec<GridNode>) -> Self {
        let children = nodes.into_iter().map(|node| SplitChild { node, weight: 1.0 }).collect();
        GridNode::Split(SplitNode { id: SplitId::new(), orientation, children })
    }

    pub fn as_split(&self) -> Option<&SplitNode> {
        match self {
            GridNode::Split(s) => Some(s),
            GridNode::Leaf { .. } => None,
        }
    }

    pub fn leaf_id(&self) -> Option<ZoneId> {
        match self {
            GridNode::Leaf { id } => Some(*id),
            GridNode::Split(_) => None,
        }
    }

    /// Zone ids in reading order (same order as `compute_rects`).
    pub fn leaves(&self) -> Vec<ZoneId> {
        match self {
            GridNode::Leaf { id } => vec![*id],
            GridNode::Split(s) => s.children.iter().flat_map(|c| c.node.leaves()).collect(),
        }
    }

    pub fn compute_rects(&self, bounds: Rect) -> Vec<ZoneRect> {
        match self {
            GridNode::Leaf { id } => vec![ZoneRect { id: *id, bounds }],
            GridNode::Split(s) => s
                .children
                .iter()
                .zip(s.child_bounds(bounds))
                .flat_map(|(c, b)| c.node.compute_rects(b))
                .collect(),
        }
    }

    /// Splitters for nested splits come first, then this split's own.
    pub fn splitters(&self, bounds: Rect) -> Vec<SplitterHandle> {
        let GridNode::Split(s) = self else { return Vec::new() };
        let child_bounds = s.child_bounds(bounds);
        let mut out: Vec<SplitterHandle> = s
            .children
            .iter()
            .zip(&child_bounds)
            .flat_map(|(c, b)| c.node.splitters(*b))
            .collect();
        for (i, left) in child_bounds.iter().take(child_bounds.len() - 1).enumerate() {
            let half = SPLITTER_THICKNESS / 2.0;
            let rect = match s.orientation {
                Orientation::Columns => Rect::new(left.right() - half, bounds.y, SPLITTER_THICKNESS, bounds.height),
                Orientation::Rows => Rect::new(bounds.x, left.bottom() - half, bounds.width, SPLITTER_THICKNESS),
            };
            out.push(SplitterHandle { split_id: s.id, left_child_index: i, orientation: s.orientation, bounds: rect });
        }
        out
    }
}

impl SplitNode {
    pub fn total_weight(&self) -> f64 {
        self.children.iter().map(|c| c.weight).sum()
    }

    /// Extent along the split axis.
    pub fn extent(&self, bounds: Rect) -> f64 {
        match self.orientation {
            Orientation::Columns => bounds.width,
            Orientation::Rows => bounds.height,
        }
    }

    /// Each child's share of `bounds`, proportional to its weight.
    pub fn child_bounds(&self, bounds: Rect) -> Vec<Rect> {
        let total_weight = self.total_weight();
        let extent = self.extent(bounds);
        let mut offset = 0.0;
        self.children
            .iter()
            .map(|c| {
                let share = c.weight / total_weight * extent;
                let rect = match self.orientation {
                    Orientation::Columns => Rect::new(bounds.x + offset, bounds.y, share, bounds.height),
                    Orientation::Rows => Rect::new(bounds.x, bounds.y + offset, bounds.width, share),
                };
                offset += share;
                rect
            })
            .collect()
    }
}
