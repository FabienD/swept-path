//! Properties that must hold whatever the scene.
//!
//! These cover the solvers, whose behaviour is allowed to change as the grid
//! is refined — unlike the geometry primitives, which are pinned to golden
//! vectors recorded from the frozen prototype.

use proptest::prelude::*;
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::exact::{Approach, Grid, search};
use swept_solver::solve::alternatives;

fn scene(opening: f64, post_depth: f64, road: f64) -> Scene {
    Scene {
        left_post: Post {
            inner_edge_x: -opening / 2.0,
            width: 0.55,
            depth: post_depth,
        },
        right_post: Post {
            inner_edge_x: opening / 2.0,
            width: 0.55,
            depth: post_depth,
        },
        wall_thickness: 0.30,
        sidewalk_width: 1.20,
        curb_cut_width: opening + 0.80,
        road_width: road,
        curb_height: f64::INFINITY,
        gate: GateKind::Sliding,
    }
}

fn lbx() -> Vehicle {
    Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
}

/// A small node ceiling: these properties test invariants that hold whatever
/// the budget, so exploring sixty thousand nodes per case buys nothing but
/// minutes of CI.
fn thrifty() -> SearchBudget {
    SearchBudget {
        max_nodes: 6_000,
        ..SearchBudget::default()
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    /// Whatever comes back is drivable: no pose along it touches anything.
    #[test]
    fn returned_paths_never_collide(
        opening in 2.0_f64..6.0,
        post_depth in 0.2_f64..0.9,
        road in 3.0_f64..8.0,
    ) {
        let (vehicle, sc) = (lbx(), scene(opening, post_depth, road));
        if let Some(best) = search(&vehicle, &sc, Approach::Forward, Grid::coarse()).best() {
            let field = ClearanceField::new(&sc, &vehicle);
            for step in &best.poses {
                prop_assert_ne!(field.at(step.pose), Clearance::Collision);
            }
        }
    }

    /// The reported clearance is the one the path actually has.
    #[test]
    fn reported_clearance_matches_the_path(
        opening in 2.5_f64..6.0,
        road in 3.5_f64..8.0,
    ) {
        let (vehicle, sc) = (lbx(), scene(opening, 0.55, road));
        if let Some(best) = search(&vehicle, &sc, Approach::Forward, Grid::coarse()).best() {
            let field = ClearanceField::new(&sc, &vehicle);
            let actual = best.poses.iter()
                .filter_map(|s| match field.at(s.pose) {
                    Clearance::Clear(m) => Some(m),
                    Clearance::Collision => None,
                })
                .fold(f64::MAX, f64::min);
            prop_assert!((actual - best.min_clearance).abs() < 1e-9);
        }
    }

    /// Clearance can never exceed the geometric ceiling, whatever the path.
    /// This is the headline conclusion of the project: extra moves buy no room.
    #[test]
    fn clearance_stays_under_the_ceiling(opening in 2.2_f64..6.0) {
        let vehicle = lbx();
        let sc = scene(opening, 0.55, 4.5);
        let ceiling = (opening - vehicle.mirror_width) / 2.0;
        if let Some(best) = alternatives(&vehicle, &sc, thrifty(), &mut Silent, None).best() {
            prop_assert!(best.min_clearance <= ceiling + 1e-6);
        }
    }

    /// A wider opening never admits less room than a narrower one.
    #[test]
    fn wider_is_never_tighter(opening in 2.5_f64..5.0, extra in 0.1_f64..1.0) {
        let vehicle = lbx();
        let narrow = search(&vehicle, &scene(opening, 0.55, 4.5), Approach::Forward, Grid::coarse());
        let wide = search(&vehicle, &scene(opening + extra, 0.55, 4.5), Approach::Forward, Grid::coarse());
        if let (Some(n), Some(w)) = (narrow.best(), wide.best()) {
            prop_assert!(w.min_clearance >= n.min_clearance - 1e-9);
        }
    }

    /// Multi never worse than simple, across arbitrary scenes.
    #[test]
    fn multi_is_never_worse_than_simple(opening in 2.4_f64..5.0, road in 3.5_f64..7.0) {
        let (vehicle, sc) = (lbx(), scene(opening, 0.55, road));
        if let swept_solver::result::Outcome::Found(list) =
            alternatives(&vehicle, &sc, thrifty(), &mut Silent, None)
            && let Some(one) = list.iter().find(|m| m.moves == 1)
        {
            for other in list.iter().filter(|m| m.moves > 1) {
                prop_assert!(other.min_clearance >= one.min_clearance - 1e-9);
            }
        }
    }
}
