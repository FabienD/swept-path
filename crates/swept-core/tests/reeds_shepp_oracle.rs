//! Our Reeds-Shepp against somebody else's.
//!
//! The closed forms are transcribed from the literature, and the part hardest
//! to be sure of is the sign conditions that decide whether a family applies.
//! Getting one wrong does not produce a wrong path — it produces a **missing**
//! one, which neither the landing tests nor the property tests can see: both
//! only examine the words that were returned.
//!
//! An independent transcription can see it. If it finds a shorter path than
//! ours, we discarded a family we should have kept.
//!
//! `reeds_shepp` is a development dependency and must never become anything
//! else: `swept-core` claims zero production dependencies, and that claim is
//! what lets it be published under `MIT OR Apache-2.0` beside an AGPL
//! application.

use proptest::prelude::*;
// The package is `reeds_shepp`; its library target is `reeds_shepp_lib`.
use reeds_shepp_lib::utils::Pose as OraclePose;
use reeds_shepp_lib::{get_optimal_path, path_length};
use std::f64::consts::TAU;
use swept_core::curves::reeds_shepp::shortest;
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// How far apart the generated poses may sit, in metres.
///
/// ARBITRARY, and the same spread the property tests use, so that a failure
/// here can be reproduced there.
const SPREAD_M: f64 = 15.0;

/// How much longer than the oracle our answer may be, as a fraction.
///
/// ARBITRARY, and deliberately tight: both solve the same problem in closed
/// form, so they should agree to floating-point noise. A tolerance is kept
/// only because the two normalise differently and a whole path accumulates a
/// few ulps.
const TOLERANCE: f64 = 1e-6;

prop_compose! {
    fn any_pose()(
        x in -SPREAD_M..SPREAD_M,
        y in -SPREAD_M..SPREAD_M,
        heading in 0.0..TAU,
    ) -> Pose {
        Pose::new(x, y, Radians::new(heading))
    }
}

/// The same problem in the oracle's terms: unit radius, and degrees.
fn as_oracle(pose: Pose, radius: f64) -> OraclePose {
    OraclePose {
        x: pose.x / radius,
        y: pose.y / radius,
        theta_degree: pose.heading.get().to_degrees(),
    }
}

proptest! {
    /// We must never be longer than the oracle.
    ///
    /// Being longer means we discarded a family that applies. Being *shorter*
    /// would be worse — it would mean returning a path that does not land —
    /// but the property tests already cover that, so this one guards only the
    /// direction they cannot see.
    #[test]
    fn our_shortest_is_never_longer_than_the_oracle(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let Some(theirs) =
            get_optimal_path(as_oracle(from, radius), as_oracle(to, radius))
        else {
            // The oracle found nothing, so it has no opinion on us.
            return Ok(());
        };
        let theirs = path_length(&theirs) * radius;

        let ours = shortest(from, to, radius).map(|p| p.length());
        prop_assert!(ours.is_some(), "we found nothing where the oracle found {theirs}");
        let ours = ours.unwrap_or(f64::INFINITY);

        prop_assert!(
            ours <= theirs.mul_add(TOLERANCE, theirs) + TOLERANCE,
            "ours {ours} against the oracle's {theirs} — a family was discarded",
        );
    }
}
