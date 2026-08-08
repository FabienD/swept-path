//! Vehicle motion under the bicycle model.
//!
//! The vehicle's state reduces to the pose of its rear axle: two coordinates
//! and a heading. A manoeuvre is a chain of constant-curvature segments, each
//! integrated in closed form:
//!
//! ```text
//! θ₁ = θ + κ·ds
//! x += R·(sin θ₁ − sin θ)      with R = 1/κ
//! y −= R·(cos θ₁ − cos θ)
//! ```
//!
//! A negative `ds` is reverse motion. A curvature of zero degenerates to a
//! straight segment, handled separately because `R` is then infinite.

use crate::geometry::Point;
use crate::units::Radians;

/// Which way the vehicle is moving along its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Moving forward.
    Forward,
    /// Backing up.
    Reverse,
}

/// The pose of the vehicle's rear axle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pose {
    /// Along the road, in metres.
    pub x: f64,
    /// Away from the road, in metres.
    pub y: f64,
    /// Direction the vehicle points in.
    pub heading: Radians,
}

impl Pose {
    /// Builds a pose from its coordinates and heading.
    #[must_use]
    pub const fn new(x: f64, y: f64, heading: Radians) -> Self {
        Self { x, y, heading }
    }

    /// The position alone, dropping the heading.
    #[must_use]
    pub const fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Advances the pose along a constant-curvature segment.
    ///
    /// `curvature` is the inverse of the turning radius, in reciprocal metres;
    /// zero means a straight line. `distance` is the arc length travelled,
    /// negative when reversing.
    ///
    /// ```
    /// use swept_core::kinematics::Pose;
    /// use swept_core::units::Radians;
    ///
    /// let start = Pose::new(0.0, 0.0, Radians::default());
    /// let end = start.advance(0.0, 2.5);
    /// assert!((end.x - 2.5).abs() < 1e-12);
    /// ```
    #[must_use]
    pub fn advance(&self, curvature: f64, distance: f64) -> Self {
        if curvature == 0.0 {
            let (sin, cos) = self.heading.sin_cos();
            return Self::new(
                self.x + cos * distance,
                self.y + sin * distance,
                self.heading,
            );
        }

        let radius = 1.0 / curvature;
        let end_heading = self.heading + Radians::new(curvature * distance);
        let (sin_0, cos_0) = self.heading.sin_cos();
        let (sin_1, cos_1) = end_heading.sin_cos();

        Self::new(
            self.x + radius * (sin_1 - sin_0),
            self.y - radius * (cos_1 - cos_0),
            end_heading,
        )
    }
}

/// Largest number of samples produced for a single segment.
///
/// ARBITRARY — carried over from the prototype (`index.html:321`), which caps
/// a straight segment at 400 samples. The cap exists to bound memory on long
/// approach runs, not for any geometric reason.
pub const MAX_SAMPLES_PER_SEGMENT: usize = 400;

/// Samples a constant-curvature segment into successive poses.
///
/// The returned poses exclude `from` and always end exactly on the segment's
/// endpoint, so that chaining segments introduces no drift. `step` is an upper
/// bound on the spacing, in metres.
///
/// # Panics
///
/// Panics if `step` is not strictly positive.
#[must_use]
pub fn sample_arc(from: Pose, curvature: f64, distance: f64, step: f64) -> Vec<Pose> {
    assert!(step > 0.0, "sampling step must be strictly positive");

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((distance.abs() / step).ceil() as usize).clamp(1, MAX_SAMPLES_PER_SEGMENT);

    (1..=count)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let fraction = i as f64 / count as f64;
            from.advance(curvature, distance * fraction)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    const EPS: f64 = 1e-12;

    #[test]
    fn a_straight_segment_keeps_the_heading() {
        let start = Pose::new(1.0, 2.0, Radians::default());
        let end = start.advance(0.0, 3.0);
        assert!((end.x - 4.0).abs() < EPS);
        assert!((end.y - 2.0).abs() < EPS);
        assert!(end.heading.get().abs() < EPS);
    }

    #[test]
    fn a_straight_segment_backwards_moves_the_other_way() {
        let start = Pose::new(1.0, 2.0, Radians::default());
        let end = start.advance(0.0, -3.0);
        assert!((end.x + 2.0).abs() < EPS);
        assert!((end.y - 2.0).abs() < EPS);
    }

    #[test]
    fn a_quarter_circle_turns_ninety_degrees() {
        // Radius 5, heading east, turning left: the rear axle ends up 5 m
        // ahead and 5 m to the left, pointing north.
        let start = Pose::new(0.0, 0.0, Radians::default());
        let radius = 5.0;
        let end = start.advance(1.0 / radius, radius * FRAC_PI_2);
        assert!((end.x - 5.0).abs() < 1e-9);
        assert!((end.y - 5.0).abs() < 1e-9);
        assert!((end.heading.get() - FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn a_full_circle_returns_to_the_start() {
        let start = Pose::new(2.0, -1.0, Radians::from_degrees(30.0));
        let radius = 4.0;
        let end = start.advance(1.0 / radius, 2.0 * std::f64::consts::PI * radius);
        assert!((end.x - start.x).abs() < 1e-9);
        assert!((end.y - start.y).abs() < 1e-9);
    }

    #[test]
    fn sampling_lands_exactly_on_the_endpoint() {
        let start = Pose::new(0.0, 0.0, Radians::default());
        let poses = sample_arc(start, 1.0 / 5.0, 5.0 * FRAC_PI_2, 0.06);
        let last = *poses.last().expect("at least one sample");
        let direct = start.advance(1.0 / 5.0, 5.0 * FRAC_PI_2);
        assert!((last.x - direct.x).abs() < 1e-9);
        assert!((last.y - direct.y).abs() < 1e-9);
        assert!((last.heading.get() - direct.heading.get()).abs() < 1e-9);
    }

    #[test]
    fn sampling_respects_the_requested_step() {
        let start = Pose::new(0.0, 0.0, Radians::default());
        let poses = sample_arc(start, 0.0, 1.0, 0.25);
        assert_eq!(poses.len(), 4);
    }
}
