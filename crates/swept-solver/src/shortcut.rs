//! Removing what a plan does not need.
//!
//! A hybrid A\* cannot land exactly on its grid, so it bridges the gap with
//! small manoeuvres that buy nothing — the shunt that exists only because
//! 90 cm primitives and 6° headings do not line up on the goal.
//!
//! # What makes a stretch superfluous
//!
//! Not its length. A thirty-centimetre reverse to clear a post is exactly what
//! a driver does, and a threshold in centimetres would delete it. What matters
//! is whether it **buys** anything:
//!
//! > A stretch is superfluous when a Reeds-Shepp curve joins the pose before it
//! > to the pose after it without collision, with no more reversals, and
//! > leaving at least as much room.
//!
//! Directly checkable, because Reeds-Shepp gives that connection in closed
//! form. It is the same rule the alternatives filter already applies to whole
//! plans — *keep only what buys room* — applied to a stretch.

use crate::path::evaluate_at_least;
use crate::result::DirectedPose;
use swept_core::clearance::ClearanceField;
use swept_core::curves::{CurvePath, reeds_shepp};
use swept_core::kinematics::{Pose, sample_arc};
use swept_core::vehicle::Vehicle;

/// How many poses a shortcut must span to be worth trying.
///
/// ARBITRARY. Below this the stretch is shorter than the curve that would
/// replace it, so nothing can be gained and every attempt is wasted work.
const MIN_SPAN: usize = 4;

/// How many stretches one pass may try before giving up.
///
/// **This bound is what makes the reduction affordable.** Trying every pair of
/// poses is quadratic, and each pair costs up to forty-eight closed-form
/// solves: on a three-hundred-pose plan that is over two million, for a
/// post-processing step nobody is waiting on. Sampling a bounded number of
/// stretches, longest first, finds the replacements that matter — a shunt the
/// planner bridged is a long stretch, not a short one.
///
/// ARBITRARY in magnitude, and the figure to raise first if plans come back
/// still reducible.
const MAX_ATTEMPTS: usize = 2_000;

/// Sampling step along a replacement curve, in metres.
///
/// ARBITRARY — the step the rest of the solver uses, so that a clearance
/// measured here means what it means elsewhere.
const SAMPLE_STEP_M: f64 = 0.08;

/// How many gear changes a path makes.
#[must_use]
pub fn reversal_count(poses: &[DirectedPose]) -> usize {
    poses
        .windows(2)
        .filter(|pair| pair[0].direction != pair[1].direction)
        .count()
}

/// The tightest clearance along a path, or zero if it collides.
#[must_use]
pub fn tightest(poses: &[DirectedPose], field: &ClearanceField) -> f64 {
    let bare: Vec<Pose> = poses.iter().map(|p| p.pose).collect();
    evaluate_at_least(&bare, field, f64::NEG_INFINITY).unwrap_or(0.0)
}

/// Replaces every superfluous stretch it can find, until none is left.
///
/// Greedy and repeated: each pass takes the longest replacement it finds, and
/// passes run until one changes nothing.
#[must_use]
pub fn reduce(
    poses: &[DirectedPose],
    vehicle: &Vehicle,
    field: &ClearanceField,
) -> Vec<DirectedPose> {
    let mut current = poses.to_vec();
    while let Some(next) = one_pass(&current, vehicle, field) {
        current = next;
    }
    current
}

/// One replacement, or `None` when nothing more can be replaced.
fn one_pass(
    poses: &[DirectedPose],
    vehicle: &Vehicle,
    field: &ClearanceField,
) -> Option<Vec<DirectedPose>> {
    let reference_room = tightest(poses, field);
    let reference_shunts = reversal_count(poses);

    // Longest stretch first: replacing more at once converges faster, and a
    // long replacement is never worse than the short ones inside it. Bounded,
    // because trying every pair is quadratic.
    let mut attempts = 0usize;
    for span in (MIN_SPAN..poses.len()).rev() {
        for start in 0..poses.len().saturating_sub(span) {
            attempts += 1;
            if attempts > MAX_ATTEMPTS {
                return None;
            }
            let end = start + span;
            let (from, to) = (poses[start].pose, poses[end].pose);

            for curve in reeds_shepp::all(from, to, vehicle.min_turning_radius) {
                if curve.reversals() > reference_shunts {
                    continue;
                }
                let replacement = directed(&curve, from, SAMPLE_STEP_M);
                if replacement.len() >= span {
                    continue;
                }
                let mut candidate = poses[..start].to_vec();
                candidate.extend(replacement);
                candidate.extend_from_slice(&poses[end + 1..]);

                if reversal_count(&candidate) > reference_shunts {
                    continue;
                }
                if tightest(&candidate, field) + 1e-9 < reference_room {
                    continue;
                }
                return Some(candidate);
            }
        }
    }
    None
}

/// Samples a curve into poses that each carry the gear they are driven in.
fn directed(curve: &CurvePath, from: Pose, step: f64) -> Vec<DirectedPose> {
    let mut out = Vec::new();
    let mut at = from;
    for segment in curve.segments() {
        let sampled = sample_arc(
            at,
            segment.curvature(curve.radius()),
            segment.signed_length(),
            step,
        );
        if let Some(last) = sampled.last() {
            at = *last;
        }
        out.extend(sampled.into_iter().map(|pose| DirectedPose {
            pose,
            direction: segment.direction,
        }));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::kinematics::Direction;
    use swept_core::scene::{GateKind, Post, Scene};
    use swept_core::units::Radians;

    fn wide_scene() -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -2.50,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 2.50,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    fn straight(count: usize) -> Vec<DirectedPose> {
        (0..count)
            .map(|i| DirectedPose {
                #[allow(clippy::cast_precision_loss)]
                pose: Pose::new(-8.0 + i as f64 * 0.4, -3.0, Radians::default()),
                direction: Direction::Forward,
            })
            .collect()
    }

    #[test]
    fn reduction_keeps_the_endpoints() {
        // Whatever it removes in the middle, a reduced path still starts and
        // ends where the original did — otherwise it answers another question.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let reduced = reduce(&path, &vehicle, &field);
        let (a, b) = (
            path.first().expect("poses"),
            reduced.first().expect("poses"),
        );
        assert!((a.pose.x - b.pose.x).abs() < 1e-9);
        assert!((a.pose.y - b.pose.y).abs() < 1e-9);
        let (a, b) = (path.last().expect("poses"), reduced.last().expect("poses"));
        assert!((a.pose.x - b.pose.x).abs() < 1e-6);
        assert!((a.pose.y - b.pose.y).abs() < 1e-6);
    }

    #[test]
    fn reduction_never_adds_a_reversal() {
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let before = reversal_count(&path);
        let after = reversal_count(&reduce(&path, &vehicle, &field));
        assert!(after <= before);
    }

    #[test]
    fn reduction_never_gives_up_clearance() {
        // The criterion the spec states: a shortcut is taken only if it leaves
        // at least as much room. Anything else would trade the very thing this
        // tool measures for a shorter path nobody asked for.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let before = tightest(&path, &field);
        let after = tightest(&reduce(&path, &vehicle, &field), &field);
        assert!(after >= before - 1e-9, "{after} against {before}");
    }

    #[test]
    fn a_reduced_path_is_irreducible() {
        // Running it again must change nothing, which is the claim the module
        // makes and the only one worth testing here.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let once = reduce(&straight(20), &vehicle, &field);
        let twice = reduce(&once, &vehicle, &field);
        assert_eq!(once.len(), twice.len());
    }
}
