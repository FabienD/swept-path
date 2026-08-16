//! How deep an entry must reach, and how a candidate path is scored.
//!
//! What shape a candidate takes is no longer decided here: the sweep asks
//! [`crate::poses`] where an entry may start and end, and Dubins for the
//! curves that join them. What is left is the two questions every candidate
//! is judged on — is it in far enough, and how much room did it leave.

use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::Pose;
use swept_core::scene::{GateKind, Scene};
use swept_core::vehicle::Vehicle;

/// Extra depth required beyond the vehicle itself before an entry counts as
/// complete, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:326`).
pub const ENTRY_CLEARANCE_M: f64 = 0.6;

/// How many poses [`evaluate_at_least`] probes before committing to a full
/// walk of the path.
///
/// ARBITRARY. Few enough that a surviving path pays under three percent extra,
/// spread widely enough that one probe always lands near the opening — which
/// is where a candidate dies, and which walking in order only reaches after
/// paying for the whole approach. MEASURED: eight probes take one fine sweep
/// from 1.6 s to well under a second.
const RECONNAISSANCE_PROBES: usize = 8;

/// How deep into the yard the vehicle must be for the entry to be complete.
///
/// Behind the posts, behind the gate leaves when they swing into the way, and
/// far enough that the whole vehicle is through.
#[must_use]
pub fn entry_depth(scene: &Scene, vehicle: &Vehicle) -> f64 {
    let gate_depth = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth)
        + gate_depth
        + vehicle.wheelbase
        + vehicle.front_overhang
        + ENTRY_CLEARANCE_M
}

/// Scores a path: its tightest clearance, or `None` if it collides anywhere.
#[must_use]
pub fn evaluate(poses: &[Pose], field: &ClearanceField) -> Option<f64> {
    evaluate_at_least(poses, field, f64::NEG_INFINITY)
}

/// Scores a path, giving up as soon as it can no longer beat `floor`.
///
/// Returns the same answer as [`evaluate`] whenever it returns `Some`. The
/// difference is what it does with a bad candidate: a sweep looking for the
/// roomiest path does not need to know *how* bad a worse one is, only that it
/// is worse. Passing the best clearance found so far as `floor` rejects most
/// candidates within a few poses instead of walking all two hundred.
///
/// Pass `f64::NEG_INFINITY` to score unconditionally.
#[must_use]
pub fn evaluate_at_least(poses: &[Pose], field: &ClearanceField, floor: f64) -> Option<f64> {
    // A reconnaissance pass first, spread over the whole path. Walking the
    // poses in order means paying for the entire drive up the road before
    // reaching the opening, which is where a candidate almost always dies —
    // so most of that walk is spent proving something already doomed. Probing
    // a handful of poses spread end to end reaches the opening immediately.
    //
    // This costs nothing in correctness: a collision at any pose refuses the
    // path wherever it is found, and a margin at or below `floor` refuses it
    // just the same. Only the order of discovery changes.
    if poses.len() > RECONNAISSANCE_PROBES {
        let last = poses.len() - 1;
        for i in 0..RECONNAISSANCE_PROBES {
            let probe = i * last / (RECONNAISSANCE_PROBES - 1);
            match field.at(poses[probe]) {
                Clearance::Collision => return None,
                Clearance::Clear(margin) if margin <= floor => return None,
                Clearance::Clear(_) => {}
            }
        }
    }

    let mut smallest = f64::MAX;
    for pose in poses {
        match field.at(*pose) {
            Clearance::Collision => return None,
            Clearance::Clear(margin) => {
                if margin <= floor {
                    return None;
                }
                smallest = smallest.min(margin);
            }
        }
    }
    (smallest < f64::MAX).then_some(smallest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;
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
            sidewalk_width: 1.20,
            curb_cut_width: 3.20,
            road_width: 4.50,
            curb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    #[test]
    fn the_entry_depth_clears_the_posts_and_the_whole_vehicle() {
        let depth = entry_depth(&wide_scene(), &lbx());
        // Post depth, plus the vehicle ahead of its rear axle, plus a margin.
        assert!(depth > 0.55 + 2.580 + 0.850, "got {depth}");
    }

    #[test]
    fn a_swinging_gate_pushes_the_entry_depth_back_by_a_leaf() {
        let mut scene = wide_scene();
        let sliding = entry_depth(&scene, &lbx());
        scene.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.10,
            hinge_offset: 0.05,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        };
        assert!((entry_depth(&scene, &lbx()) - sliding - 1.15).abs() < 1e-9);
    }

    #[test]
    fn evaluating_a_clear_path_returns_its_tightest_point() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        // A straight run down the middle of the carriageway, clear of
        // everything. Whether any given approach is drivable is the sweep's
        // problem, not this function's — here we only check that a clear path
        // reports its tightest point.
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, -3.5, Radians::default()))
            .collect();
        match evaluate(&poses, &field) {
            Some(margin) => assert!(margin >= 0.0 && margin.is_finite(), "got {margin}"),
            None => panic!("a 5 m opening admits this approach"),
        }
    }

    #[test]
    fn evaluating_a_path_through_a_wall_returns_nothing() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        // Drive straight along the wall line rather than through the opening.
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, 0.15, Radians::default()))
            .collect();
        assert_eq!(evaluate(&poses, &field), None);
    }

    #[test]
    fn a_floor_below_everything_gives_the_same_answer_as_evaluating() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, -3.5, Radians::default()))
            .collect();
        let plain = evaluate(&poses, &field).expect("a clear path");
        let floored = evaluate_at_least(&poses, &field, f64::NEG_INFINITY).expect("a clear path");
        assert!((plain - floored).abs() < 1e-12);
    }

    #[test]
    fn a_path_that_cannot_beat_the_floor_is_abandoned() {
        // The sweep only wants to know whether a candidate beats the best so
        // far. Once a pose falls below that, how much worse it gets is of no
        // interest, and finishing the walk would be wasted work.
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, -3.5, Radians::default()))
            .collect();
        let reachable = evaluate(&poses, &field).expect("a clear path");
        assert_eq!(evaluate_at_least(&poses, &field, reachable + 0.01), None);
    }

    #[test]
    fn a_colliding_path_is_refused_whatever_the_floor() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, 0.15, Radians::default()))
            .collect();
        assert_eq!(evaluate_at_least(&poses, &field, f64::NEG_INFINITY), None);
    }

    #[test]
    fn an_empty_path_scores_nothing() {
        // Guards the `smallest < f64::MAX` sentinel: an empty path must not
        // come back as infinitely roomy and win the sweep.
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        assert_eq!(evaluate_at_least(&[], &field, f64::NEG_INFINITY), None);
    }
}
