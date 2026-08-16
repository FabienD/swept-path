//! Properties every Reeds-Shepp path must satisfy, whatever the pose pair.
//!
//! The unit tests check named cases. These check the regimes nobody thought to
//! name — coincident poses, opposed headings, separations sitting exactly on
//! the thresholds where families appear and disappear. That is where closed
//! forms fail, by leaving the domain of an `acos` or dividing by a vanishing
//! term.

use proptest::prelude::*;
use std::f64::consts::TAU;
use swept_core::curves::reeds_shepp::{all, fewest_reversals, shortest};
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// How far apart the generated poses may sit, in metres.
///
/// ARBITRARY — wide enough to straddle every family threshold at the radii
/// generated below, narrow enough that close pairs keep coming up, which is
/// where the multi-arc families live.
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
                "x off by {} on {} segments",
                end.x - to.x,
                path.segments().len(),
            );
            prop_assert!((end.y - to.y).abs() < 1e-6, "y off by {}", end.y - to.y);
            let error = (end.heading.get() - to.heading.get()).rem_euclid(TAU);
            prop_assert!(error.min(TAU - error) < 1e-6, "heading off by {error}");
        }
    }

    /// A vehicle cannot turn tighter than its minimum radius.
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

    /// No length is ever a NaN or an infinity.
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

    /// A bounded-curvature path cannot beat the straight line between the two
    /// points — not even by reversing.
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

    /// Reeds-Shepp reaches every pose. Where Dubins can fail to apply, this
    /// must not: reversing is always available.
    #[test]
    fn some_path_always_exists(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        prop_assert!(!all(from, to, radius).is_empty());
    }

    /// `shortest` must be the minimum of `all`.
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
                for path in paths {
                    prop_assert!(best.length() <= path.length() + 1e-12);
                }
            }
        }
    }

    /// `fewest_reversals` must never be beaten on reversals.
    #[test]
    fn the_smoothest_has_the_fewest_reversals(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let paths = all(from, to, radius);
        if let Some(best) = fewest_reversals(from, to, radius) {
            for path in paths {
                prop_assert!(best.reversals() <= path.reversals());
            }
        }
    }
}

/// Coincident poses are the degenerate case worth naming: the separation is
/// zero and the line of sight is undefined. Nothing may panic.
#[test]
fn coincident_poses_do_not_panic() {
    let pose = Pose::new(3.0, -1.0, Radians::new(0.8));
    for path in all(pose, pose, 4.0) {
        assert!(path.length().is_finite());
    }
}
