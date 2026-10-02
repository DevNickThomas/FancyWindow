use crate::model::{Point, Rect};

/// Where the zone canvas is. The app works in DIPs (1/96 inch) like WPF did;
/// `origin` and `scale` convert to and from physical screen pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// Canvas size in DIPs; always at (0, 0).
    pub canvas: Rect,
    /// Screen position of the canvas's top-left corner, in pixels.
    pub origin: Point,
    /// Pixels per DIP (DPI / 96).
    pub scale: f64,
}

impl Frame {
    pub fn new(width: f64, height: f64, origin: Point, scale: f64) -> Self {
        Self { canvas: Rect::new(0.0, 0.0, width, height), origin, scale }
    }

    /// A canvas rect in DIPs to screen pixels.
    pub fn to_screen(&self, r: Rect) -> Rect {
        Rect::new(
            self.origin.x + r.x * self.scale,
            self.origin.y + r.y * self.scale,
            r.width * self.scale,
            r.height * self.scale,
        )
    }

    /// A screen pixel position to canvas DIPs.
    pub fn from_screen(&self, p: Point) -> Point {
        Point::new((p.x - self.origin.x) / self.scale, (p.y - self.origin.y) / self.scale)
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new(0.0, 0.0, Point::new(0.0, 0.0), 1.0)
    }
}
