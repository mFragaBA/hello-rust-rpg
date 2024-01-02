use serde::{Deserialize, Serialize};

#[derive(PartialEq, Copy, Clone, Serialize, Deserialize, Debug)]
pub struct Rect {
    pub x1: i32,
    pub x2: i32,
    pub y1: i32,
    pub y2: i32,
}

impl Rect {
    /// Creates a new Rect with its upper left corner at (x, y) and its lower right corner at (x +
    /// w, y + h)
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Rect {
        Rect {
            x1: x,
            y1: y,
            x2: x + w,
            y2: y + h,
        }
    }

    // Returns whether this rectangle overlaps with other
    #[inline]
    pub fn intersect(&self, other: &Rect) -> bool {
        let not_disjoint_in_x = self.x1 <= other.x2 && self.x2 >= other.x1;
        let not_disjoint_in_y = self.y1 <= other.y2 && self.y2 >= other.y1;

        not_disjoint_in_x && not_disjoint_in_y
    }

    pub fn center(&self) -> (i32, i32) {
        ((self.x1 + self.x2) / 2, (self.y1 + self.y2) / 2)
    }
}
