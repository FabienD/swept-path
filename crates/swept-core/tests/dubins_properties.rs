//! Properties every Dubins path must satisfy, whatever the pose pair.
//!
//! The unit tests check named cases. These check the regimes nobody thought
//! to name — coincident poses, opposed headings, separations sitting exactly
//! on the four-radius threshold where the CCC families appear and disappear.
//! That is where closed forms fail, by leaving the domain of an `acos` or
//! dividing by a vanishing term.

use proptest::prelude::*;
use std::f64::consts::TAU;
use swept_core::curves::dubins::{all, shortest};
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// How far apart the generated poses may sit, in metres.
///
/// ARBITRARY — wide enough to straddle the four-radius threshold at every
/// radius generated below, narrow enough that the generator keeps producing
/// close pairs, which is where the CCC families live.
const SPREAD_M: f64 = 15.0;

prop_compose! {
    fn any_pose()(
        x in -SPREAD_M..SPREAD_M,
        y in -SPREAD_M..SPREAD_M,
        heading in 0.0..TAU,
    ) -> Pose {
        Pose::new(x, y, Radians::new(heading))
    }
}

proptest! {
    /// The contract the whole module rests on.
    #[test]
    fn every_path_lands_on_the_goal(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            let end = path.end(from);
            prop_assert!(
                (end.x - to.x).abs() < 1e-6,
                "x off by {} on a path of {} segments",
                end.x - to.x,
                path.segments().len(),
            );
            prop_assert!((end.y - to.y).abs() < 1e-6);
            // Landing on the point but facing the wrong way is not landing.
            // Compared modulo a full turn, since the closed forms are free to
            // wind the arcs round.
            let heading_error = (end.heading.get() - to.heading.get()).rem_euclid(TAU);
            prop_assert!(
                heading_error.min(TAU - heading_error) < 1e-6,
                "heading off by {heading_error}",
            );
        }
    }

    /// A vehicle cannot turn tighter than its minimum radius. A path that
    /// asked it to would be geometrically valid and physically impossible.
    #[test]
    fn no_path_turns_tighter_than_the_radius(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            for segment in path.segments() {
                prop_assert!(segment.curvature(radius).abs() <= 1.0 / radius + 1e-12);
            }
        }
    }

    /// Dubins is forward-only by definition. A reversal here would mean the
    /// segment vocabulary is being misused.
    #[test]
    fn no_dubins_path_ever_reverses(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            prop_assert_eq!(path.reversals(), 0);
        }
    }

    /// A bounded-curvature path cannot beat the straight line between the two
    /// points. Anything shorter is a bug in the length accounting.
    #[test]
    fn no_path_is_shorter_than_the_separation(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let separation = (to.x - from.x).hypot(to.y - from.y);
        for path in all(from, to, radius) {
            prop_assert!(path.length() >= separation - 1e-9);
        }
    }

    /// `shortest` must actually be the shortest of `all`, and must exist
    /// whenever `all` is non-empty.
    #[test]
    fn the_shortest_is_the_minimum_of_all(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let paths = all(from, to, radius);
        match shortest(from, to, radius) {
            None => prop_assert!(paths.is_empty()),
            Some(best) => {
                prop_assert!(!paths.is_empty());
                for path in paths {
                    prop_assert!(best.length() <= path.length() + 1e-12);
                }
            }
        }
    }

    /// No length is ever a NaN or an infinity. The closed forms divide and
    /// take arc cosines; a degenerate frame must yield `None`, never a path
    /// carrying a poisoned number that would silently spread.
    #[test]
    fn no_length_is_ever_not_a_number(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            prop_assert!(path.length().is_finite());
            for segment in path.segments() {
                prop_assert!(segment.length.is_finite());
            }
        }
    }
}

/// Coincident poses are the one degenerate case worth naming: the separation
/// is zero, the line of sight is undefined, and `atan2(0, 0)` returns zero
/// rather than failing. Nothing may panic.
#[test]
fn coincident_poses_do_not_panic() {
    let pose = Pose::new(3.0, -1.0, Radians::new(0.8));
    let paths = all(pose, pose, 4.0);
    for path in &paths {
        assert!(path.length().is_finite());
    }
}
