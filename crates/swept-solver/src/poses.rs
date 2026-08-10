//! Where a one-move entry may start, and where it must end.
//!
//! A Dubins curve joins two *poses*, not two positions. That is a stricter
//! requirement than the search used to work under — its arrival test was
//! merely "past the entry depth", which constrained neither where along the
//! opening the vehicle ended up nor which way it pointed, hence the vehicle
//! finishing askew in the yard. Naming the arrival pose fixes that by
//! construction: this module only ever produces goals square to the opening,
//! within the few degrees the design allows.
//!
//! Both grids are inclusive of their bounds and never empty when the geometry
//! admits anything at all — a zero step count still yields the centre pose,
//! because the carriageway bisection runs the coarse grid a dozen times over
//! and must not silently start returning nothing.

use crate::path::entry_depth;
use std::f64::consts::FRAC_PI_2;
use swept_core::kinematics::Pose;
use swept_core::scene::Scene;
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Clearance kept between the vehicle's widest point and the lane edges when
/// choosing where the approach is driven, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:362`).
pub const LANE_MARGIN_M: f64 = 0.02;

/// How far either side of the opening centre a goal is aimed, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:372`).
pub const ENTRY_SPAN_M: f64 = 0.9;

/// How far back along the road the earliest start pose sits, in metres.
///
/// ARBITRARY. The old search drove a fixed 5 m run-up and started the turn at
/// `entry_x - radius - 5`, which for the radii tried put the first pose
/// between 8 and 12 m short of the opening. Fourteen metres covers that and
/// leaves room for a wider turn; anything further back is straight road that
/// buys no clearance.
pub const APPROACH_REACH_M: f64 = 14.0;

/// How far the final heading may sit from square to the opening, in degrees.
///
/// Criterion 3 of the design: the vehicle must end within five degrees of the
/// perpendicular. Enforced here, in the generator, so that no later stage has
/// to re-check it.
pub const GOAL_HEADING_SPAN_DEGREES: f64 = 5.0;

/// Spreads `steps + 1` values evenly across `low..=high`.
///
/// A step count of zero yields the midpoint alone rather than nothing, which
/// is what keeps a coarse grid usable.
fn spread(low: f64, high: f64, steps: u16) -> Vec<f64> {
    if steps == 0 {
        return vec![f64::midpoint(low, high)];
    }
    (0..=steps)
        .map(|i| low + (high - low) * f64::from(i) / f64::from(steps))
        .collect()
}

/// Every pose an approach may start from, on the carriageway facing the
/// opening.
///
/// Returns an empty vector when the carriageway is too narrow for the vehicle
/// to sit in at all — a result, not an error.
#[must_use]
pub fn start_poses(vehicle: &Vehicle, scene: &Scene, x_steps: u16, lateral_steps: u16) -> Vec<Pose> {
    let half_width = vehicle.mirror_width / 2.0;
    let low = -scene.pavement_width - scene.road_width + half_width + LANE_MARGIN_M;
    let high = -half_width - LANE_MARGIN_M;
    if low > high {
        return Vec::new();
    }

    let mut out = Vec::new();
    for x in spread(-APPROACH_REACH_M, -ENTRY_SPAN_M, x_steps) {
        for y in spread(low, high, lateral_steps) {
            out.push(Pose::new(x, y, Radians::default()));
        }
    }
    out
}

/// Every pose an entry may finish on: in the yard, square to the opening.
#[must_use]
pub fn goal_poses(
    vehicle: &Vehicle,
    scene: &Scene,
    entry_steps: u16,
    heading_steps: u16,
) -> Vec<Pose> {
    let depth = entry_depth(scene, vehicle);
    let span = GOAL_HEADING_SPAN_DEGREES.to_radians();

    let mut out = Vec::new();
    for x in spread(-ENTRY_SPAN_M, ENTRY_SPAN_M, entry_steps) {
        for heading in spread(FRAC_PI_2 - span, FRAC_PI_2 + span, heading_steps) {
            out.push(Pose::new(x, depth, Radians::new(heading)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;
    use swept_core::scene::{GateKind, Post};

    fn scene(opening: f64) -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -opening / 2.0,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: opening / 2.0,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: opening + 0.80,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn every_start_sits_on_the_carriageway_facing_along_it() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        let starts = start_poses(&vehicle, &sc, 4, 6);
        assert!(!starts.is_empty());
        for pose in &starts {
            assert!(pose.y < 0.0, "a start belongs on the road, got y={}", pose.y);
            assert!(
                pose.y > -sc.pavement_width - sc.road_width,
                "a start belongs on the carriageway, got y={}",
                pose.y
            );
            assert!(
                pose.heading.get().abs() < 1e-12,
                "a start faces along the road"
            );
            assert!(
                pose.x < 0.0,
                "a start is short of the opening, got x={}",
                pose.x
            );
        }
    }

    #[test]
    fn the_lateral_sweep_keeps_the_mirrors_inside_the_lane() {
        // A start pose whose mirrors already overhang the kerb is not a start
        // at all: the sweep would spend its budget on candidates that collide
        // before they move.
        let (vehicle, sc) = (lbx(), scene(3.0));
        let half = vehicle.mirror_width / 2.0;
        for pose in start_poses(&vehicle, &sc, 4, 6) {
            assert!(pose.y + half <= -LANE_MARGIN_M + 1e-12, "got y={}", pose.y);
            assert!(
                pose.y - half >= -sc.pavement_width - sc.road_width + LANE_MARGIN_M - 1e-12,
                "got y={}",
                pose.y
            );
        }
    }

    #[test]
    fn every_goal_sits_in_the_yard_facing_into_it() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        let goals = goal_poses(&vehicle, &sc, 8, 2);
        assert!(!goals.is_empty());
        let depth = crate::path::entry_depth(&sc, &vehicle);
        for pose in &goals {
            assert!(
                (pose.y - depth).abs() < 1e-12,
                "a goal sits at the entry depth"
            );
            assert!(pose.x.abs() <= ENTRY_SPAN_M + 1e-12, "got x={}", pose.x);
        }
    }

    #[test]
    fn no_goal_heading_strays_further_than_the_design_allows() {
        // Criterion 3 of the design: the vehicle ends within five degrees of
        // square to the opening. Enforcing it in the generator means no later
        // stage has to check it.
        let (vehicle, sc) = (lbx(), scene(3.0));
        for pose in goal_poses(&vehicle, &sc, 8, 4) {
            let off_square = (pose.heading.get() - FRAC_PI_2).abs();
            assert!(
                off_square <= GOAL_HEADING_SPAN_DEGREES.to_radians() + 1e-12,
                "got {} degrees off square",
                off_square.to_degrees()
            );
        }
    }

    #[test]
    fn a_single_step_still_yields_the_square_centred_goal() {
        // Zero steps must not mean zero poses. The bisection in `min_road`
        // runs the coarse grid a dozen times over and would otherwise start
        // returning nothing at all.
        let (vehicle, sc) = (lbx(), scene(3.0));
        let goals = goal_poses(&vehicle, &sc, 0, 0);
        assert_eq!(goals.len(), 1);
        assert!(goals[0].x.abs() < 1e-12);
        assert!((goals[0].heading.get() - FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn a_carriageway_narrower_than_the_vehicle_yields_no_start() {
        // Not an error — a result. The sweep reports NotFound rather than
        // pretending a lane it cannot sit in is drivable.
        let mut sc = scene(3.0);
        sc.road_width = 0.5;
        assert!(start_poses(&lbx(), &sc, 4, 6).is_empty());
    }

    #[test]
    fn more_steps_never_yield_fewer_poses() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        assert!(start_poses(&vehicle, &sc, 8, 12).len() >= start_poses(&vehicle, &sc, 4, 6).len());
        assert!(goal_poses(&vehicle, &sc, 16, 4).len() >= goal_poses(&vehicle, &sc, 8, 2).len());
    }
}
