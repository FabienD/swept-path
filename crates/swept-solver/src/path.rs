//! The one-move approaches a search tries, and how they are scored.
//!
//! Each candidate is a fixed shape: a run-up along the road, a quarter turn
//! at a chosen radius, and a straight push into the yard. Sweeping the radius,
//! the lateral start position and the entry point covers the useful space of
//! single-move approaches.

use std::f64::consts::FRAC_PI_2;
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::{Pose, sample_arc};
use swept_core::scene::{GateKind, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Extra depth required beyond the vehicle itself before an entry counts as
/// complete, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:326`).
pub const ENTRY_CLEARANCE_M: f64 = 0.6;

/// Length of the straight run-up before the turn, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:331`). Long
/// enough that the approach starts well clear of the opening.
pub const RUN_UP_M: f64 = 5.0;

/// How far a reverse path must reach beyond the pavement to count as having
/// rejoined the road, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:343`).
pub const REVERSE_EXIT_MARGIN_M: f64 = 0.2;

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

/// Builds a forward approach: run up along the road, turn, push in.
///
/// `radius` is the turning radius held through the quarter turn, `lateral_y`
/// the distance out from the kerb the run-up is driven at, and `entry_x` where
/// the turn is aimed along the opening.
#[must_use]
pub fn forward_path(
    vehicle: &Vehicle,
    scene: &Scene,
    radius: f64,
    lateral_y: f64,
    entry_x: f64,
    step: f64,
) -> Vec<Pose> {
    let start = Pose::new(entry_x - radius - RUN_UP_M, lateral_y, Radians::default());
    let mut poses = vec![start];

    // `sample_arc` lands exactly on the endpoint, so `advance` gives the same
    // pose without having to unwrap the last sample.
    let before_turn = start.advance(0.0, RUN_UP_M);
    poses.extend(sample_arc(start, 0.0, RUN_UP_M, step));

    let curvature = 1.0 / radius;
    let sweep = radius * FRAC_PI_2;
    let after_turn = before_turn.advance(curvature, sweep);
    poses.extend(sample_arc(before_turn, curvature, sweep, step));

    let needed = entry_depth(scene, vehicle);
    if after_turn.y < needed {
        poses.extend(sample_arc(after_turn, 0.0, needed - after_turn.y, step));
    }
    poses
}

/// Builds a reverse approach, read forwards.
///
/// The path is generated from the finished position outwards — that is the
/// only end whose pose is known — then reversed, so the caller always sees a
/// path that starts on the road.
///
/// Returns `None` when the swing never reaches back out to the road, which
/// means this combination of radius and setback cannot be driven.
#[must_use]
pub fn reverse_path(
    vehicle: &Vehicle,
    scene: &Scene,
    radius: f64,
    entry_x: f64,
    setback: f64,
    approach_angle: Radians,
    step: f64,
) -> Option<Vec<Pose>> {
    let top = entry_depth(scene, vehicle);
    let start = Pose::new(entry_x, top, Radians::new(-FRAC_PI_2));
    let mut poses = vec![start];

    let before_turn = if setback > 0.0 {
        poses.extend(sample_arc(start, 0.0, setback, step));
        start.advance(0.0, setback)
    } else {
        start
    };

    let curvature = 1.0 / radius;
    let sweep = (-approach_angle.get() + FRAC_PI_2) / curvature;
    let after_turn = before_turn.advance(curvature, sweep);
    poses.extend(sample_arc(before_turn, curvature, sweep, step));

    if after_turn.y > -scene.pavement_width - REVERSE_EXIT_MARGIN_M {
        return None;
    }

    poses.extend(sample_arc(after_turn, 0.0, RUN_UP_M, step));
    poses.reverse();
    Some(poses)
}

/// Scores a path: its tightest clearance, or `None` if it collides anywhere.
#[must_use]
pub fn evaluate(poses: &[Pose], field: &ClearanceField) -> Option<f64> {
    let mut smallest = f64::MAX;
    for pose in poses {
        match field.at(*pose) {
            Clearance::Collision => return None,
            Clearance::Clear(margin) => smallest = smallest.min(margin),
        }
    }
    (smallest < f64::MAX).then_some(smallest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;

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
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
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
    fn a_forward_path_starts_on_the_road_and_ends_in_the_yard() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let poses = forward_path(&vehicle, &scene, 5.2, -2.0, 0.0, 0.1);
        let first = poses.first().expect("a path has poses");
        let last = poses.last().expect("a path has poses");

        assert!(first.y < 0.0, "starts on the road, got y={}", first.y);
        assert!(first.heading.get().abs() < 1e-9, "starts along the road");
        assert!(
            last.y >= entry_depth(&scene, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.y
        );
        assert!(
            (last.heading.to_degrees() - 90.0).abs() < 1e-6,
            "ends pointing into the yard"
        );
    }

    #[test]
    fn a_reverse_path_that_never_reaches_the_road_is_rejected() {
        let (scene, vehicle) = (wide_scene(), lbx());
        // A very tight radius with no setback cannot swing far enough out.
        assert!(reverse_path(&vehicle, &scene, 5.2, 0.0, 0.0, Radians::default(), 0.1).is_none());
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
}
