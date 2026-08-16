//! The twelve Reeds-Shepp families.
//!
//! Reeds and Shepp (1990) extended Dubins to a vehicle that may reverse: the
//! shortest path between two poses at bounded curvature is then one of
//! forty-eight words, built from twelve fundamental families. Like Dubins,
//! every one has a closed form.
//!
//! What matters here beyond the length: Reeds-Shepp also minimises the number
//! of **direction changes**, which is precisely what this project counts as a
//! manoeuvre.
//!
//! # The normalised frame
//!
//! Formulas are written with the start at the origin facing `+x` and lengths
//! divided by the turning radius, leaving the goal as `(x, y, φ)`. This is not
//! the `(d, α, β)` triple [`super::dubins`] uses: Reeds-Shepp's involutions
//! act simply on `(x, y, φ)` and awkwardly on the other.
//!
//! # Forty-eight words from eight functions
//!
//! Two involutions generate the rest. **Time flip** drives the path backwards,
//! which negates `x` and `φ` and turns every forward segment into a reverse
//! one. **Reflection** swaps left for right, which negates `y` and `φ`. Applied
//! to the *input* rather than the output, they let eight base functions cover
//! everything — the alternative being forty-eight transcriptions, each its own
//! chance of a sign error.
//!
//! # On the formulas
//!
//! Taken from Reeds & Shepp, *Optimal paths for a car that goes both forwards
//! and backwards* (1990), cross-checked against `LaValle`, *Planning
//! Algorithms* §15.3. They transcribe badly and published versions disagree.
//! **Every family is therefore tested by integrating its result through
//! [`Pose::advance`] and checking where it lands** — a test that depends on no
//! source and settles any disagreement.

use super::{CurvePath, Segment, Steering};
use crate::kinematics::{Direction, Pose};
use std::f64::consts::{PI, TAU};

/// Wraps an angle into `(-π, π]`.
///
/// Centred on zero, unlike [`super::dubins::mod_2pi`], because Reeds-Shepp
/// tests angles against zero to decide whether a family applies: a value just
/// below a full turn must read as a small negative, not as a large positive.
#[must_use]
pub fn wrap_pi(angle: f64) -> f64 {
    let wrapped = angle % TAU;
    if wrapped > PI {
        wrapped - TAU
    } else if wrapped <= -PI {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// Cartesian to polar, as the formulas write it.
#[must_use]
pub fn polar(x: f64, y: f64) -> (f64, f64) {
    (x.hypot(y), y.atan2(x))
}

/// The goal pose, in the start's frame, divided by the turning radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Ahead of the start, in radii.
    pub x: f64,
    /// To the left of the start, in radii.
    pub y: f64,
    /// Change of heading, in radians.
    pub phi: f64,
}

impl Frame {
    /// Normalises a start and goal pose against a turning radius.
    ///
    /// Returns `None` when the radius is not a usable positive length.
    #[must_use]
    pub fn between(from: Pose, to: Pose, radius: f64) -> Option<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let (sin, cos) = from.heading.sin_cos();
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        Some(Self {
            x: dx.mul_add(cos, dy * sin) / radius,
            y: dy.mul_add(cos, -(dx * sin)) / radius,
            phi: wrap_pi(to.heading.get() - from.heading.get()),
        })
    }

    /// The same problem driven backwards.
    ///
    /// Time symmetry: a path traversed in reverse covers the same ground, so
    /// solving the flipped problem and negating every length gives a word
    /// whose gears are all swapped.
    #[must_use]
    pub fn time_flipped(self) -> Self {
        Self {
            x: -self.x,
            y: self.y,
            phi: -self.phi,
        }
    }

    /// The same problem with left and right exchanged.
    #[must_use]
    pub fn reflected(self) -> Self {
        Self {
            x: self.x,
            y: -self.y,
            phi: -self.phi,
        }
    }
}

/// One piece of a Reeds-Shepp word: a steering, and a **signed** length.
///
/// The sign is the gear — negative means reversing. Lengths are normalised:
/// radians for an arc, radii for a straight run, which the radius converts to
/// metres in one place, [`Word::path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    /// Where the steering is held.
    pub steering: Steering,
    /// Signed, normalised length.
    pub length: f64,
}

/// A Reeds-Shepp word: two to five elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Word(pub Vec<Element>);

impl Word {
    /// Whether this word can be driven at all.
    ///
    /// The closed forms divide and take arc cosines, so a family that does not
    /// apply can yield a NaN rather than nothing. Catching it here keeps a
    /// poisoned number out of every path built downstream.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty() && self.0.iter().all(|e| e.length.is_finite())
    }

    /// The path this word describes, in metres.
    ///
    /// **The only place a sign becomes a gear.** Anywhere else would be a
    /// second chance to lose one.
    #[must_use]
    pub fn path(&self, radius: f64) -> CurvePath {
        let segments = self
            .0
            .iter()
            .map(|e| {
                let direction = if e.length < 0.0 {
                    Direction::Reverse
                } else {
                    Direction::Forward
                };
                Segment::new(e.steering, direction, e.length.abs() * radius)
            })
            .collect();
        CurvePath::new(segments, radius)
    }

    /// Total normalised length, ignoring gear.
    #[must_use]
    pub fn cost(&self) -> f64 {
        self.0.iter().map(|e| e.length.abs()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Radians;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    const EPS: f64 = 1e-12;

    #[test]
    fn a_negative_length_becomes_a_reverse_segment() {
        // The one thing this type exists to get right.
        let word = Word(vec![
            Element {
                steering: Steering::Left,
                length: 1.0,
            },
            Element {
                steering: Steering::Right,
                length: -0.5,
            },
        ]);
        let path = word.path(2.0);
        let segments = path.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].direction, Direction::Forward);
        assert!(
            (segments[0].length - 2.0).abs() < EPS,
            "one radian at radius 2"
        );
        assert_eq!(segments[1].direction, Direction::Reverse);
        assert!((segments[1].length - 1.0).abs() < EPS);
        assert_eq!(path.reversals(), 1);
    }

    #[test]
    fn a_word_carrying_a_non_finite_length_is_refused() {
        // The closed forms divide and take arc cosines. A poisoned number must
        // be caught here rather than spread into a path nobody can drive.
        let word = Word(vec![Element {
            steering: Steering::Left,
            length: f64::NAN,
        }]);
        assert!(!word.is_valid());
    }

    #[test]
    fn an_empty_word_is_not_valid() {
        assert!(!Word(Vec::new()).is_valid());
    }

    #[test]
    fn a_word_of_finite_lengths_is_valid() {
        let word = Word(vec![Element {
            steering: Steering::Straight,
            length: -3.0,
        }]);
        assert!(word.is_valid());
        assert!((word.cost() - 3.0).abs() < EPS);
    }

    #[test]
    fn the_frame_puts_the_start_at_the_origin_facing_along_x() {
        // Nine metres ahead at a three-metre radius is three radii along x,
        // nothing across, and no change of heading.
        let from = Pose::new(4.0, -2.0, Radians::new(FRAC_PI_2));
        let to = Pose::new(4.0, 7.0, Radians::new(FRAC_PI_2));
        let frame = Frame::between(from, to, 3.0).expect("a usable radius");
        assert!((frame.x - 3.0).abs() < EPS, "got x={}", frame.x);
        assert!(frame.y.abs() < EPS, "got y={}", frame.y);
        assert!(frame.phi.abs() < EPS, "got phi={}", frame.phi);
    }

    #[test]
    fn an_unusable_radius_yields_no_frame() {
        let pose = Pose::default();
        assert!(Frame::between(pose, pose, 0.0).is_none());
        assert!(Frame::between(pose, pose, -1.0).is_none());
        assert!(Frame::between(pose, pose, f64::NAN).is_none());
    }

    #[test]
    fn turning_time_about_mirrors_the_problem_along_x() {
        // Driving the path backwards is the same problem with x and the
        // heading negated. Applying it twice must return the original.
        let frame = Frame {
            x: 1.5,
            y: -0.4,
            phi: 0.8,
        };
        let there = frame.time_flipped();
        assert!((there.x + 1.5).abs() < EPS);
        assert!((there.y + 0.4).abs() < EPS);
        assert!((there.phi + 0.8).abs() < EPS);
        let back = there.time_flipped();
        assert!((back.x - frame.x).abs() < EPS);
        assert!((back.phi - frame.phi).abs() < EPS);
    }

    #[test]
    fn reflecting_mirrors_the_problem_along_y() {
        // Swapping left for right is the same problem with y and the heading
        // negated. Also an involution.
        let frame = Frame {
            x: 1.5,
            y: -0.4,
            phi: 0.8,
        };
        let there = frame.reflected();
        assert!((there.x - 1.5).abs() < EPS);
        assert!((there.y - 0.4).abs() < EPS);
        assert!((there.phi + 0.8).abs() < EPS);
        let back = there.reflected();
        assert!((back.y - frame.y).abs() < EPS);
    }

    #[test]
    fn angles_wrap_into_a_half_turn_either_side() {
        // Reeds-Shepp compares angles against zero to decide whether a family
        // applies, so its wrap must be centred on zero — unlike the Dubins
        // one, which runs from zero to a full turn.
        assert!(wrap_pi(0.0).abs() < EPS);
        assert!((wrap_pi(TAU + 0.3) - 0.3).abs() < EPS);
        assert!((wrap_pi(-0.3) + 0.3).abs() < EPS);
        for angle in [3.0 * PI, -3.0 * PI, 7.5, -7.5, 0.0] {
            let wrapped = wrap_pi(angle);
            assert!(
                wrapped > -PI && wrapped <= PI,
                "{angle} wrapped to {wrapped}"
            );
        }
    }

    #[test]
    fn polar_coordinates_round_trip() {
        let (r, theta) = polar(3.0, 4.0);
        assert!((r - 5.0).abs() < EPS);
        assert!((r * theta.cos() - 3.0).abs() < EPS);
        assert!((r * theta.sin() - 4.0).abs() < EPS);
    }
}
