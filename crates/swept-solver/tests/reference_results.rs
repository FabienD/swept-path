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
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::exact::{Approach, Grid, search};
use swept_solver::result::Outcome;
use swept_solver::solve::alternatives;

fn lbx() -> Vehicle {
    Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
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
        kerb_height: f64::INFINITY,
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
        kerb_height: f64::INFINITY,
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
        .map(|o| o.shape)
        .filter(|s| s.center.x.abs() < reach && s.center.y >= 0.0)
        .map(|s| s.center.y + s.half_height.max(s.half_width))
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

/// The measured gateway: 2.29 m clear, 1.30 m pavement, 5.90 m carriageway,
/// and leaves that swing back past square.
fn measured_gateway() -> Scene {
    Scene {
        left_post: Post {
            inner_edge_x: -2.29 / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        right_post: Post {
            inner_edge_x: 2.29 / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        wall_thickness: 0.30,
        pavement_width: 1.30,
        dropped_kerb_width: 3.20,
        road_width: 5.90,
        kerb_height: f64::INFINITY,
        gate: GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(118.0),
        },
    }
}

/// Criteria 1, 2 and 3 of the Dubins design, on the gateway that motivated it,
/// through the answer the interface actually receives.
///
/// Before this lot the exhaustive sweep returned nothing here, so the answer
/// came from the planner and was labelled heuristic — it proved nothing.
#[test]
fn the_measured_gateway_admits_a_proved_one_move_entry() {
    let vehicle =
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59).expect("valid vehicle");
    let scene = measured_gateway();

    let Outcome::Found(list) =
        alternatives(&vehicle, &scene, SearchBudget::default(), &mut Silent, None)
    else {
        panic!("this gateway admits an entry");
    };

    // Criterion 1: a one-move entry, and one the sweep proved rather than
    // stumbled on.
    let one = list
        .iter()
        .find(|m| m.moves == 1)
        .expect("a one-move entry exists");
    assert!(
        one.is_exact(),
        "a one-move entry must come from the exhaustive sweep, not the planner"
    );

    // Criterion 2, first half: the geometric ceiling. Clearance can never
    // exceed half the difference between the opening and the vehicle's widest
    // point, whatever the path. A figure above it would not be good news but
    // proof that the clearance field is lying.
    let ceiling = (scene.opening_width() - vehicle.mirror_width) / 2.0;
    assert!(
        one.min_clearance <= ceiling + 1e-9,
        "{:.1} cm claimed against a {:.1} cm ceiling",
        one.min_clearance * 100.0,
        ceiling * 100.0
    );
    assert!(
        one.min_clearance > 0.0,
        "an entry with no room is not an entry"
    );

    // Criterion 2, second half: the tightest point is in the gateway, not
    // against a kerb six metres short of it. A path whose worst moment is out
    // on the road has not been squeezed by the opening at all, and its figure
    // answers a different question than the one that was asked.
    let field = ClearanceField::new(&scene, &vehicle);
    let (_, where_it_is) = one
        .poses
        .iter()
        .filter_map(|step| match field.at(step.pose) {
            Clearance::Clear(margin) => Some((margin, step.pose)),
            Clearance::Collision => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("a collision-free manoeuvre has a tightest point");
    // The gateway runs from the outer face of the wall to behind the leaves,
    // and the approach across the pavement counts as part of threading it.
    let gateway_far_side = scene.left_post.depth.max(scene.right_post.depth)
        + match scene.gate {
            GateKind::Swinging { leaf_length, .. } => leaf_length,
            GateKind::Sliding => 0.0,
        };
    // What is recorded is the pose of the *rear axle*, while what actually
    // grazes is a mirror or a front corner, up to a wheelbase and an overhang
    // ahead of it. So the window reaches back by that much: an axle still out
    // on the road can perfectly well have its nose in the opening, and that
    // moment is part of threading it. What the window still excludes is the
    // failure this criterion is about — a tightest point out on the
    // carriageway, metres before the vehicle has anything to thread.
    let nose_reach = vehicle.wheelbase + vehicle.front_overhang;
    let window = (-scene.pavement_width - nose_reach)..=gateway_far_side;
    assert!(
        window.contains(&where_it_is.y),
        "tightest point at y={:.2} m, outside the gateway (which spans {:.2} to {:.2})",
        where_it_is.y,
        -scene.pavement_width - nose_reach,
        gateway_far_side
    );

    // Criterion 3: the vehicle finishes square to the opening, rather than
    // askew in the yard as the old depth-only arrival test allowed.
    let finish = one.poses.last().expect("a manoeuvre has poses");
    let off_square = (finish.pose.heading.get() - std::f64::consts::FRAC_PI_2).abs();
    assert!(
        off_square.to_degrees() <= 5.0 + 1e-9,
        "finished {:.1} degrees off square",
        off_square.to_degrees()
    );
}

/// What this batch was built for.
///
/// The pure 2D model treats a kerb as a wall of infinite height, so the only
/// candidates the exhaustive sweep refused on this gateway were those whose
/// front overhang swings over the pavement beside the dropped kerb. Declaring
/// the kerb for what it is can only help — never hinder, since every candidate
/// that was drivable before still is.
///
/// MEASURED, and worth knowing: on **this** gateway it buys nothing at all.
/// A wall, a 12 cm kerb and no pavement whatsoever all return 4.15 cm, with
/// the same tightest point and not one pose overhanging. What limits this
/// entry is the opening, not the footway. The batch makes the model right
/// where it was wrong; it does not make this gateway easier.
#[test]
fn a_low_kerb_never_costs_room_and_may_buy_some() {
    let vehicle =
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59).expect("valid vehicle");

    let walled = measured_gateway();
    let mut low = measured_gateway();
    // MEASURED — a standard French T2 kerb stands 12 cm above the gutter.
    low.kerb_height = 0.12;

    let walled_best = search(&vehicle, &walled, Approach::Forward, Grid::fine());
    let low_best = search(&vehicle, &low, Approach::Forward, Grid::fine());

    match (walled_best.best(), low_best.best()) {
        (Some(w), Some(l)) => assert!(
            l.min_clearance >= w.min_clearance - 1e-9,
            "a wall gave {:.1} cm, a kerb gave {:.1} cm",
            w.min_clearance * 100.0,
            l.min_clearance * 100.0
        ),
        (Some(_), None) => panic!("lowering the kerb removed an entry that existed"),
        (None, _) => { /* nothing to compare, and the batch is not at fault */ }
    }
}
