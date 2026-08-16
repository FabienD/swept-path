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
    /// Most of a scene is walls and curbs, which are axis-aligned; this is the
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

/// How far a point lies from a rectangle.
///
/// The prototype folded both cases into a single number, returning `-1` for a
/// point inside the rectangle. That sentinel is easy to forget to check, and
/// forgetting it turns a collision into a very small clearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointDistance {
    /// The point lies within the rectangle.
    Inside,
    /// The point lies outside, at this distance from the nearest edge, in
    /// metres.
    Outside(f64),
}

impl Obb {
    /// Distance from `point` to this rectangle.
    ///
    /// The point is taken into the rectangle's local frame, where the distance
    /// reduces to the length of the componentwise overshoot beyond the
    /// half-sizes.
    ///
    /// ```
    /// use swept_core::geometry::{Obb, Point, PointDistance};
    ///
    /// let pillar = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
    /// assert_eq!(pillar.distance_to(Point::new(0.5, 0.5)), PointDistance::Inside);
    /// assert_eq!(pillar.distance_to(Point::new(3.0, 0.5)), PointDistance::Outside(2.0));
    /// ```
    #[must_use]
    pub fn distance_to(&self, point: Point) -> PointDistance {
        let (sin, cos) = self.angle.sin_cos();
        let (dx, dy) = (point.x - self.center.x, point.y - self.center.y);
        let local_x = dx * cos + dy * sin;
        let local_y = -dx * sin + dy * cos;

        let overshoot_x = (local_x.abs() - self.half_width).max(0.0);
        let overshoot_y = (local_y.abs() - self.half_height).max(0.0);

        if overshoot_x == 0.0 && overshoot_y == 0.0 {
            PointDistance::Inside
        } else {
            PointDistance::Outside(overshoot_x.hypot(overshoot_y))
        }
    }
}

/// Overlap below which two rectangles are still considered disjoint, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:258`), where it
/// absorbs the floating-point noise of gate leaves that come to rest exactly
/// against a pillar. No measurement backs the specific value; it should be
/// revalidated against real tolerances.
pub const OVERLAP_TOLERANCE_M: f64 = 0.006;

impl Obb {
    /// Whether two rectangles overlap, by the separating axis theorem.
    ///
    /// Two convex shapes are disjoint if and only if some axis exists on which
    /// their projections do not meet. For rectangles it is enough to test the
    /// four edge normals — two per rectangle.
    #[must_use]
    pub fn overlaps(&self, other: &Obb) -> bool {
        let (a_sin, a_cos) = self.angle.sin_cos();
        let (b_sin, b_cos) = other.angle.sin_cos();
        let axes = [
            (a_cos, a_sin),
            (-a_sin, a_cos),
            (b_cos, b_sin),
            (-b_sin, b_cos),
        ];

        let mine = self.corners();
        let theirs = other.corners();

        for (ux, uy) in axes {
            let project = |corners: &[Point; 4]| {
                corners.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                    let v = p.x * ux + p.y * uy;
                    (lo.min(v), hi.max(v))
                })
            };
            let (a_lo, a_hi) = project(&mine);
            let (b_lo, b_hi) = project(&theirs);

            if a_hi < b_lo + OVERLAP_TOLERANCE_M || b_hi < a_lo + OVERLAP_TOLERANCE_M {
                return false;
            }
        }
        true
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
    fn reports_a_point_inside_the_rectangle() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        assert_eq!(obb.distance_to(Point::new(1.0, 1.0)), PointDistance::Inside);
    }

    #[test]
    fn measures_perpendicular_distance_to_an_edge() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        match obb.distance_to(Point::new(3.5, 1.0)) {
            PointDistance::Outside(d) => assert!((d - 1.5).abs() < EPS),
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }

    #[test]
    fn measures_diagonal_distance_to_a_corner() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        match obb.distance_to(Point::new(5.0, 6.0)) {
            PointDistance::Outside(d) => assert!((d - 5.0).abs() < EPS), // 3-4-5
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }

    #[test]
    fn accounts_for_rotation() {
        // A 2x1 rectangle turned a quarter turn is 1 wide and 2 tall.
        let obb = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(90.0), 1.0, 0.5);
        match obb.distance_to(Point::new(2.0, 0.0)) {
            PointDistance::Outside(d) => assert!((d - 1.5).abs() < EPS),
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }

    #[test]
    fn detects_plainly_overlapping_rectangles() {
        let a = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        let b = Obb::from_bounds(1.0, 3.0, 1.0, 3.0);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn separates_disjoint_rectangles() {
        let a = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
        let b = Obb::from_bounds(2.0, 3.0, 0.0, 1.0);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn tolerates_an_overlap_below_the_threshold() {
        // Carried over from the prototype: an overlap of less than 6 mm does
        // not count as contact.
        let a = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
        let barely = Obb::from_bounds(1.0 - 0.005, 2.0, 0.0, 1.0);
        assert!(!a.overlaps(&barely));

        let clearly = Obb::from_bounds(1.0 - 0.02, 2.0, 0.0, 1.0);
        assert!(a.overlaps(&clearly));
    }

    #[test]
    fn separates_rotated_rectangles_that_axis_aligned_bounds_would_not() {
        // Two squares turned a half-quarter turn. Each spans ±1.414 on both
        // axes, so their axis-aligned bounds overlap on [1.086, 1.414] — but
        // the shapes themselves are 3.54 apart, against a combined reach of
        // 2.83. Only a proper separating-axis test gets this right.
        let a = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(45.0), 1.0, 1.0);
        let b = Obb::new(Point::new(2.5, 2.5), Radians::from_degrees(45.0), 1.0, 1.0);
        assert!(!a.overlaps(&b));
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
