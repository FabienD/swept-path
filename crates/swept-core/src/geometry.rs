//! Plane geometry: points and oriented bounding boxes.
//!
//! Every obstacle in a scene is an oriented rectangle, which keeps collision
//! detection to two primitives: the distance from a point to a rectangle, and
//! the overlap of two rectangles.

use crate::units::Radians;

/// A point in the plane, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Along the road.
    pub x: f64,
    /// Away from the road; `0` is the outer face of the wall.
    pub y: f64,
}

impl Point {
    /// Builds a point from its coordinates, in metres.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An oriented bounding box: a rectangle free to rotate about its centre.
///
/// ```
/// use swept_core::geometry::Obb;
///
/// let wall = Obb::from_bounds(-3.0, 3.0, 0.0, 0.3);
/// assert_eq!(wall.corners().len(), 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    /// Centre of the rectangle.
    pub center: Point,
    /// Rotation of the local axes about the centre.
    pub angle: Radians,
    /// Half-size along the local `x` axis.
    pub half_width: f64,
    /// Half-size along the local `y` axis.
    pub half_height: f64,
}

impl Obb {
    /// Builds a rectangle from its centre, rotation and half-sizes.
    #[must_use]
    pub const fn new(center: Point, angle: Radians, half_width: f64, half_height: f64) -> Self {
        Self {
            center,
            angle,
            half_width,
            half_height,
        }
    }

    /// Builds an axis-aligned rectangle from its bounds.
    ///
    /// Most of a scene is walls and kerbs, which are axis-aligned; this is the
    /// constructor they use.
    #[must_use]
    pub fn from_bounds(x0: f64, x1: f64, y0: f64, y1: f64) -> Self {
        Self::new(
            Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1)),
            Radians::default(),
            (x1 - x0) / 2.0,
            (y1 - y0) / 2.0,
        )
    }

    /// The four corners, counter-clockwise from the local `(-1, -1)` corner.
    ///
    /// The order matters: it is the order the prototype used, and the golden
    /// vectors are recorded in it.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        let (sin, cos) = self.angle.sin_cos();
        let signs: [(f64, f64); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        signs.map(|(sx, sy)| {
            let (dx, dy) = (sx * self.half_width, sy * self.half_height);
            Point::new(
                self.center.x + dx * cos - dy * sin,
                self.center.y + dx * sin + dy * cos,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    fn assert_point_near(actual: Point, x: f64, y: f64) {
        assert!(
            (actual.x - x).abs() < EPS && (actual.y - y).abs() < EPS,
            "expected ({x}, {y}), got ({}, {})",
            actual.x,
            actual.y
        );
    }

    #[test]
    fn builds_an_axis_aligned_box_from_bounds() {
        let obb = Obb::from_bounds(-1.0, 3.0, 0.0, 2.0);
        assert_point_near(obb.center, 1.0, 1.0);
        assert!((obb.half_width - 2.0).abs() < EPS);
        assert!((obb.half_height - 1.0).abs() < EPS);
        assert!(obb.angle.get().abs() < EPS);
    }

    #[test]
    fn lists_corners_counterclockwise_from_the_near_left() {
        // Same order as the prototype: (-1,-1), (1,-1), (1,1), (-1,1) in local
        // coordinates.
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 4.0);
        let corners = obb.corners();
        assert_point_near(corners[0], 0.0, 0.0);
        assert_point_near(corners[1], 2.0, 0.0);
        assert_point_near(corners[2], 2.0, 4.0);
        assert_point_near(corners[3], 0.0, 4.0);
    }

    #[test]
    fn rotates_corners_about_the_centre() {
        // A 2x2 square centred on the origin, turned a quarter turn, maps each
        // corner onto the next one.
        let obb = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(90.0), 1.0, 1.0);
        let corners = obb.corners();
        assert_point_near(corners[0], 1.0, -1.0);
        assert_point_near(corners[1], 1.0, 1.0);
    }
}
