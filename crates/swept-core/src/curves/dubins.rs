//! The six Dubins families.
//!
//! Dubins (1957) proved that the shortest forward-only path between two poses
//! at bounded curvature is always one of six words: four of the form
//! arc-straight-arc (`LSL`, `RSR`, `LSR`, `RSL`) and two of the form
//! arc-arc-arc (`RLR`, `LRL`). Each has a closed form — no search, no
//! iteration.
//!
//! # The normalised frame
//!
//! Every formula below is written in a frame that removes the rigid motion:
//! the start sits at the origin pointing along `+x`, and lengths are divided
//! by the turning radius. What is left is three numbers — the normalised
//! distance `d`, and the two headings `α` and `β` measured against the line of
//! sight. Two problems with the same triple have the same solution up to that
//! rigid motion, which is why the closed forms can exist at all.
//!
//! # On the formulas
//!
//! They are taken from Shkel & Lumelsky, *Classification of the Dubins set*
//! (2001), and cross-checked against `LaValle`, *Planning Algorithms* §13.3.3.
//! They are dense and they transcribe badly; published versions disagree on
//! `LSR` and `LRL`. Every family is therefore tested by integrating its result
//! through [`Pose::advance`] and checking it lands on the goal. That test
//! depends on no source and settles any disagreement.

use crate::kinematics::Pose;
use std::f64::consts::TAU;

/// Wraps an angle into `[0, 2π)`.
///
/// The closed forms subtract angles freely and rely on the result being taken
/// modulo a full turn; a negative arc length would otherwise be read as a
/// reverse segment, which Dubins never produces.
#[must_use]
pub fn mod_2pi(angle: f64) -> f64 {
    let wrapped = angle % TAU;
    if wrapped < 0.0 {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// The problem stripped of its rigid motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Distance between the poses, divided by the turning radius.
    pub d: f64,
    /// Start heading, measured from the line of sight, in `[0, 2π)`.
    pub alpha: f64,
    /// Goal heading, measured from the same line, in `[0, 2π)`.
    pub beta: f64,
}

impl Frame {
    /// Normalises a start and goal pose against a turning radius.
    ///
    /// Returns `None` if the radius is not a usable positive length — the
    /// caller has a vehicle that cannot turn, and no family applies.
    #[must_use]
    pub fn between(from: Pose, to: Pose, radius: f64) -> Option<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let separation = dx.hypot(dy);
        if !separation.is_finite() {
            return None;
        }
        let line_of_sight = dy.atan2(dx);
        Some(Self {
            d: separation / radius,
            alpha: mod_2pi(from.heading.get() - line_of_sight),
            beta: mod_2pi(to.heading.get() - line_of_sight),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Radians;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    const EPS: f64 = 1e-12;

    #[test]
    fn wraps_angles_into_zero_to_two_pi() {
        assert!((mod_2pi(0.0) - 0.0).abs() < EPS);
        assert!((mod_2pi(TAU) - 0.0).abs() < EPS);
        assert!((mod_2pi(-FRAC_PI_2) - (TAU - FRAC_PI_2)).abs() < EPS);
        assert!((mod_2pi(3.0 * TAU + PI) - PI).abs() < EPS);
    }

    #[test]
    fn normalises_the_distance_by_the_radius() {
        let from = Pose::default();
        let to = Pose::new(10.0, 0.0, Radians::default());
        let frame = Frame::between(from, to, 5.0).expect("a valid frame");
        assert!((frame.d - 2.0).abs() < EPS);
        assert!(frame.alpha.abs() < EPS);
        assert!(frame.beta.abs() < EPS);
    }

    #[test]
    fn measures_headings_against_the_line_of_sight() {
        // Start pointing along +y, goal 10 m along +x also pointing along +y.
        // The line of sight runs along +x, so both headings sit ninety degrees
        // off it.
        let from = Pose::new(0.0, 0.0, Radians::new(FRAC_PI_2));
        let to = Pose::new(10.0, 0.0, Radians::new(FRAC_PI_2));
        let frame = Frame::between(from, to, 5.0).expect("a valid frame");
        assert!((frame.alpha - FRAC_PI_2).abs() < EPS);
        assert!((frame.beta - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn is_invariant_under_rotation_and_translation() {
        // The frame is the whole point: two pose pairs that differ only by a
        // rigid motion must normalise identically, or the closed forms would
        // have to know about the world frame.
        let from = Pose::new(1.0, 2.0, Radians::new(0.3));
        let to = Pose::new(6.0, 5.0, Radians::new(1.1));
        let plain = Frame::between(from, to, 4.0).expect("a valid frame");

        let turn: f64 = 0.7;
        let (sin, cos) = turn.sin_cos();
        let rotate = |p: Pose| {
            Pose::new(
                p.x * cos - p.y * sin + 13.0,
                p.x * sin + p.y * cos - 8.0,
                p.heading + Radians::new(turn),
            )
        };
        let moved = Frame::between(rotate(from), rotate(to), 4.0).expect("a valid frame");

        assert!((plain.d - moved.d).abs() < 1e-9);
        assert!((plain.alpha - moved.alpha).abs() < 1e-9);
        assert!((plain.beta - moved.beta).abs() < 1e-9);
    }

    #[test]
    fn refuses_a_radius_that_is_not_positive() {
        let from = Pose::default();
        let to = Pose::new(10.0, 0.0, Radians::default());
        assert!(Frame::between(from, to, 0.0).is_none());
        assert!(Frame::between(from, to, -1.0).is_none());
        assert!(Frame::between(from, to, f64::NAN).is_none());
    }
}
