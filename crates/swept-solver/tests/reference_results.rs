//! The reference results recorded in `CLAUDE.md`, as regression tests.
//!
//! These are measurements, not derivations. If one of them starts failing,
//! either the port drifted or the recorded value was wrong — decide which
//! before touching a tolerance.

use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::Pose;
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use swept_solver::exact::{Approach, Grid, search};

fn lbx() -> Vehicle {
    Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
}

fn scene(opening: f64, gate: GateKind) -> Scene {
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
        gate,
    }
}

/// The same scene with no pavement, so that only the corridor constrains the
/// crossing. Measuring the corridor means measuring the corridor, not the kerb
/// the vehicle happens to clip on its way in.
fn corridor_scene(opening: f64, gate: GateKind) -> Scene {
    Scene {
        pavement_width: 0.0,
        road_width: 12.0,
        ..scene(opening, gate)
    }
}

fn swinging(open_degrees: f64) -> GateKind {
    GateKind::Swinging {
        leaf_length: 1.15,
        leaf_thickness: 0.10,
        hinge_offset: 0.05,
        hinge_depth_ratio: 0.5,
        open_angle: Radians::from_degrees(open_degrees),
    }
}

/// How deep the constrained corridor runs, measured from the obstacles
/// themselves rather than assumed.
///
/// Keeps only the obstacles bordering the opening — the posts, and the gate
/// leaves when they swing into the way — and reports how far into the yard
/// they reach.
fn corridor_depth(scene: &Scene) -> f64 {
    let reach = scene.opening_width() / 2.0 + 1.0;
    scene
        .obstacles()
        .iter()
        .filter(|o| o.center.x.abs() < reach && o.center.y >= 0.0)
        .map(|o| o.center.y + o.half_height.max(o.half_width))
        .fold(0.0_f64, f64::max)
}

/// Whether the vehicle can cross the whole corridor holding `degrees`.
///
/// Rotating the vehicle on the spot measures nothing: it pivots about its rear
/// axle and its nose swings into the open yard, so the corridor never
/// constrains it. What matters is *getting through* — the vehicle is therefore
/// slid along its own heading, from fully outside to fully inside, and the
/// angle passes if some lateral offset clears every step of that crossing.
fn crosses_at_angle(
    scene: &Scene,
    vehicle: &Vehicle,
    field: &ClearanceField,
    degrees: f64,
) -> bool {
    let heading = Radians::from_degrees(degrees);
    let (sin, cos) = heading.sin_cos();
    let corridor = corridor_depth(scene);

    let mut offset = -1.5;
    while offset <= 1.5 {
        let mut clear = true;
        let mut along = -vehicle.wheelbase - vehicle.front_overhang - 0.5;
        while along <= corridor + vehicle.rear_overhang + 0.5 {
            let pose = Pose::new(
                along * cos - offset * sin,
                along * sin + offset * cos,
                heading,
            );
            if field.at(pose) == Clearance::Collision {
                clear = false;
                break;
            }
            along += 0.05;
        }
        if clear {
            return true;
        }
        offset += 0.02;
    }
    false
}

/// The widest unbroken span of approach angles that crosses the corridor, in
/// degrees.
fn angular_tolerance(scene: &Scene, vehicle: &Vehicle) -> f64 {
    let field = ClearanceField::new(scene, vehicle);
    let mut widest = 0.0_f64;
    let mut run = 0.0_f64;
    let mut degrees = 60.0;
    while degrees <= 120.0 {
        if crosses_at_angle(scene, vehicle, &field, degrees) {
            run += 0.5;
            widest = widest.max(run);
        } else {
            run = 0.0;
        }
        degrees += 0.5;
    }
    widest
}

/// Widest angle off the perpendicular that still fits, derived from geometry
/// alone, in degrees.
///
/// A vehicle of width `w` crossing a corridor of depth `L` at an angle `a` off
/// the perpendicular spans `w / cos a + L * tan a` across the opening. Solving
/// that against the opening width gives the tolerance without consulting the
/// code under test — which is the point: it is an independent oracle.
fn predicted_half_tolerance(opening: f64, width: f64, corridor: f64) -> f64 {
    let footprint = |a: f64| width / a.cos() + corridor * a.tan();
    if footprint(0.0) > opening {
        return 0.0;
    }
    let (mut lo, mut hi) = (0.0_f64, std::f64::consts::FRAC_PI_2 - 1e-6);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if footprint(mid) <= opening {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lo.to_degrees()
}

/// The measured angular tolerance must match what the geometry predicts.
///
/// This replaces the "critical width" claim recorded in `CLAUDE.md`, which
/// does not hold: since the footprint grows without bound as the approach
/// flattens, no opening width admits *every* angle. What is true is the
/// formula above, and the core agrees with it.
#[test]
fn angular_tolerance_matches_the_geometry() {
    let vehicle = lbx();
    let corridor = 0.55_f64; // sliding gate: the posts alone

    for opening in [2.30, 2.40, 2.60, 3.00] {
        let scene = corridor_scene(opening, GateKind::Sliding);
        let measured = angular_tolerance(&scene, &vehicle);
        // The sweep spans 60° to 120°, so it can never report more than 60.
        let predicted =
            (2.0 * predicted_half_tolerance(opening, vehicle.mirror_width, corridor)).min(60.0);
        assert!(
            (measured - predicted).abs() <= 3.0,
            "{opening} m opening: measured {measured}°, geometry predicts {predicted}°"
        );
    }
}

/// A narrower opening always leaves less angular room.
#[test]
fn a_narrower_opening_squeezes_the_tolerance() {
    let vehicle = lbx();
    let wide = angular_tolerance(&corridor_scene(3.00, GateKind::Sliding), &vehicle);
    let narrow = angular_tolerance(&corridor_scene(2.30, GateKind::Sliding), &vehicle);
    assert!(narrow < wide, "3.00 m gave {wide}°, 2.30 m gave {narrow}°");
}

/// Reference 3: with a sliding gate the corridor is the post depth alone.
#[test]
fn a_sliding_gate_leaves_only_the_post_depth() {
    let depth = corridor_depth(&scene(2.40, GateKind::Sliding));
    assert!(
        (depth - 0.55).abs() < 0.01,
        "expected the 0.55 m post depth, got {depth}"
    );
}

/// Reference 2, restated: swinging leaves open to 90° stretch the corridor to
/// about one leaf length, and that alone squeezes the angular tolerance hard.
///
/// `CLAUDE.md` records "about 4°" for this case. Measurement and geometry both
/// give roughly 14°, and they agree with each other to under a degree, so the
/// recorded figure is what is wrong. The qualitative claim survives intact:
/// leaves are what kills the tolerance.
#[test]
fn swinging_leaves_stretch_the_corridor_and_squeeze_the_tolerance() {
    let vehicle = lbx();
    let open = corridor_scene(2.40, swinging(90.0));

    let depth = corridor_depth(&open);
    assert!(
        depth >= 1.15,
        "the corridor should reach past a leaf length (1.15 m), got {depth}"
    );

    let with_leaves = angular_tolerance(&open, &vehicle);
    let without = angular_tolerance(&corridor_scene(2.40, GateKind::Sliding), &vehicle);
    assert!(
        with_leaves < without / 2.0,
        "leaves must more than halve the tolerance: {with_leaves}° with, {without}° without"
    );

    // The leaves also narrow the gap itself: each hinge sits 5 cm inside its
    // post, so the clear width drops from 2.40 m to about 2.20 m.
    let predicted = 2.0 * predicted_half_tolerance(2.40 - 2.0 * 0.10, vehicle.mirror_width, depth);
    assert!(
        (with_leaves - predicted).abs() <= 4.0,
        "measured {with_leaves}°, geometry predicts {predicted}°"
    );
}

/// The headline conclusion: extra moves buy no room, because the ceiling is
/// `(W - w) / 2` whatever the path.
#[test]
fn clearance_never_exceeds_the_geometric_ceiling() {
    let vehicle = lbx();
    for opening in [2.4, 3.0, 4.0, 5.0] {
        let ceiling = (opening - vehicle.mirror_width) / 2.0;
        let outcome = search(
            &vehicle,
            &scene(opening, GateKind::Sliding),
            Approach::Forward,
            Grid::fine(),
        );
        if let Some(best) = outcome.best() {
            assert!(
                best.min_clearance <= ceiling + 1e-6,
                "{opening} m opening: clearance {} exceeds the ceiling {ceiling}",
                best.min_clearance
            );
        }
    }
}
