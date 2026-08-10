//! Optimal paths between two poses at bounded curvature.
//!
//! A vehicle that cannot turn tighter than some radius does not join two poses
//! by a straight line. The shortest way is a chain of at most three pieces,
//! each either a straight segment or an arc at exactly the minimum radius —
//! a result due to Dubins (1957) for forward-only motion, extended to reverse
//! by Reeds and Shepp (1990).
//!
//! This module holds what both families share: the alphabet of segments, and
//! the integration of a chain of them. The families themselves live in
//! [`dubins`].
//!
//! # Why the whole set and not just the shortest
//!
//! These curves minimise *length*. This project cares about *clearance* — the
//! room left between the vehicle and the posts. The shortest path grazes more,
//! not less. So the callers ask for every admissible curve, discard those that
//! collide, and keep the roomiest. Length only breaks ties.

use crate::kinematics::{Direction, Pose, sample_arc};

pub mod dubins;

/// Which way the steering is held over a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steering {
    /// Full lock to the left, at the minimum radius.
    Left,
    /// Wheels straight.
    Straight,
    /// Full lock to the right, at the minimum radius.
    Right,
}

/// Shortest segment worth keeping, in metres.
///
/// A nanometre — deliberately far below what this domain measures, which is
/// not the reasoning one would expect. "Below what anyone measures" argues for
/// a millimetre, and a millimetre is wrong: dropping an arc does not only lose
/// its length, it loses the heading it was turning through. An arc of length
/// `l` at radius `R` sweeps `l / R`, and every metre travelled afterwards
/// amplifies that error.
///
/// The property tests found the case: an `RSL` opening with a 0.86 mm arc at a
/// 1.37 m radius, followed by a 20.7 m straight run. The arc turns the vehicle
/// by 0.63 mrad; discarding it moves the landing point by 13 mm. On a gateway
/// where the entire available margin is 13 cm, that is not negligible.
///
/// At a nanometre the amplified error stays under a micrometre for any radius
/// and path length this domain produces. The constant then does only what it
/// was meant to do: drop the exactly degenerate arcs the closed forms return,
/// such as the leading arc of an `LSL` whose headings already agree.
pub const NEGLIGIBLE_LENGTH_M: f64 = 1e-9;

/// One piece of a path: constant steering, constant gear, a given length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Where the steering is held.
    pub steering: Steering,
    /// Whether the vehicle is going forwards or backing up.
    pub direction: Direction,
    /// Arc length travelled, in metres. Always positive — the direction
    /// carries the sign.
    pub length: f64,
}

impl Segment {
    /// Builds a segment. `length` is a distance, so it must not be negative.
    #[must_use]
    pub fn new(steering: Steering, direction: Direction, length: f64) -> Self {
        Self {
            steering,
            direction,
            length: length.abs(),
        }
    }

    /// The curvature to feed [`Pose::advance`], in reciprocal metres.
    ///
    /// Positive curvature turns left in this frame, because `y` grows to the
    /// left of a vehicle heading along `+x`.
    #[must_use]
    pub fn curvature(&self, radius: f64) -> f64 {
        match self.steering {
            Steering::Left => 1.0 / radius,
            Steering::Straight => 0.0,
            Steering::Right => -1.0 / radius,
        }
    }

    /// The distance to feed [`Pose::advance`], negative when reversing.
    #[must_use]
    pub fn signed_length(&self) -> f64 {
        match self.direction {
            Direction::Forward => self.length,
            Direction::Reverse => -self.length,
        }
    }
}

/// A chain of segments at one fixed turning radius.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePath {
    segments: Vec<Segment>,
    radius: f64,
}

impl CurvePath {
    /// Builds a path, dropping segments too short to matter.
    #[must_use]
    pub fn new(segments: Vec<Segment>, radius: f64) -> Self {
        let segments = segments
            .into_iter()
            .filter(|s| s.length > NEGLIGIBLE_LENGTH_M)
            .collect();
        Self { segments, radius }
    }

    /// The segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The turning radius every arc uses, in metres.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Total distance travelled, in metres, forwards and backwards alike.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.segments.iter().map(|s| s.length).sum()
    }

    /// How many times the vehicle changes between forward and reverse.
    ///
    /// This is what the interface calls a manoeuvre. A Dubins path always
    /// scores zero; Reeds-Shepp paths will not.
    #[must_use]
    pub fn reversals(&self) -> usize {
        self.segments
            .windows(2)
            .filter(|pair| pair[0].direction != pair[1].direction)
            .count()
    }

    /// Where the path ends, starting from `from`.
    #[must_use]
    pub fn end(&self, from: Pose) -> Pose {
        self.segments.iter().fold(from, |pose, segment| {
            pose.advance(segment.curvature(self.radius), segment.signed_length())
        })
    }

    /// The path as successive poses, spaced by at most `step` metres.
    ///
    /// `from` itself is excluded, exactly as [`sample_arc`] excludes its own
    /// starting pose, so that chaining introduces no duplicate.
    ///
    /// # Panics
    ///
    /// Panics if `step` is not strictly positive.
    #[must_use]
    pub fn poses(&self, from: Pose, step: f64) -> Vec<Pose> {
        assert!(step > 0.0, "sampling step must be strictly positive");

        let mut pose = from;
        let mut out = Vec::new();
        for segment in &self.segments {
            let sampled = sample_arc(
                pose,
                segment.curvature(self.radius),
                segment.signed_length(),
                step,
            );
            if let Some(last) = sampled.last() {
                pose = *last;
            }
            out.extend(sampled);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    const EPS: f64 = 1e-9;

    #[test]
    fn a_straight_segment_covers_its_length() {
        let path = CurvePath::new(
            vec![Segment::new(Steering::Straight, Direction::Forward, 3.0)],
            5.0,
        );
        assert!((path.length() - 3.0).abs() < EPS);
    }

    #[test]
    fn a_path_ends_where_the_kinematics_says_it_does() {
        // A quarter circle to the left at radius 5 takes the vehicle 5 m
        // forward and 5 m to its left, pointing ninety degrees round.
        let path = CurvePath::new(
            vec![Segment::new(
                Steering::Left,
                Direction::Forward,
                5.0 * FRAC_PI_2,
            )],
            5.0,
        );
        let end = path.end(Pose::default());
        assert!((end.x - 5.0).abs() < EPS);
        assert!((end.y - 5.0).abs() < EPS);
        assert!((end.heading.get() - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn turning_right_mirrors_turning_left() {
        let radius = 4.0;
        let quarter = radius * FRAC_PI_2;
        let left = CurvePath::new(
            vec![Segment::new(Steering::Left, Direction::Forward, quarter)],
            radius,
        )
        .end(Pose::default());
        let right = CurvePath::new(
            vec![Segment::new(Steering::Right, Direction::Forward, quarter)],
            radius,
        )
        .end(Pose::default());
        assert!((left.x - right.x).abs() < EPS);
        assert!((left.y + right.y).abs() < EPS);
        assert!((left.heading.get() + right.heading.get()).abs() < EPS);
    }

    #[test]
    fn sampling_ends_on_the_endpoint() {
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 3.0),
                Segment::new(Steering::Straight, Direction::Forward, 2.0),
            ],
            5.0,
        );
        let poses = path.poses(Pose::default(), 0.1);
        let last = *poses.last().expect("a sampled path is never empty");
        let end = path.end(Pose::default());
        assert!((last.x - end.x).abs() < EPS);
        assert!((last.y - end.y).abs() < EPS);
        assert!((last.heading.get() - end.heading.get()).abs() < EPS);
    }

    #[test]
    fn a_forward_only_path_has_no_reversals() {
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 1.0),
                Segment::new(Steering::Straight, Direction::Forward, 1.0),
            ],
            5.0,
        );
        assert_eq!(path.reversals(), 0);
    }

    #[test]
    fn changing_gear_counts_as_one_reversal() {
        // Reeds-Shepp will produce these; Dubins never does. Counting them
        // here means the solver can compare a Dubins path and a Reeds-Shepp
        // path on the same footing at lot 2c.
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 1.0),
                Segment::new(Steering::Right, Direction::Reverse, 1.0),
                Segment::new(Steering::Left, Direction::Forward, 1.0),
            ],
            5.0,
        );
        assert_eq!(path.reversals(), 2);
    }

    #[test]
    fn zero_length_segments_are_dropped() {
        // The closed forms routinely return a zero-length arc — LSL degenerates
        // to LS when the headings already agree. Keeping them would inflate the
        // reversal count and clutter the rendered path.
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 0.0),
                Segment::new(Steering::Straight, Direction::Forward, 2.0),
            ],
            5.0,
        );
        assert_eq!(path.segments().len(), 1);
    }
}
