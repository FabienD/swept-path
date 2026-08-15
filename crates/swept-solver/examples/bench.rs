//! What the exhaustive sweep costs, measured rather than supposed.
//!
//! Run with `cargo run -p swept-solver --release --example bench`.
//!
//! The figure that matters is the wall time of one fine sweep: the interface
//! waits on it inside a worker, and beyond a second or so the tool stops
//! feeling like it answers. Reported here, not asserted anywhere — timings
//! belong in a report, never in a test, or the suite starts depending on the
//! machine it runs on.

use std::time::Instant;
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use swept_solver::exact::{Approach, Grid, search};

/// The measured gateway, with the opening left free to vary.
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
        pavement_width: 1.30,
        dropped_kerb_width: 3.20,
        road_width: 5.90,
        gate: GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(118.0),
        },
    }
}

fn main() {
    // The measured vehicle, with the pivot radius rather than the kerb radius.
    let vehicle =
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59).expect("valid vehicle");

    println!(
        "fine grid:   {} pose pairs, up to {} curves",
        Grid::fine().candidate_count(),
        Grid::fine().candidate_count() * 6
    );
    println!(
        "coarse grid: {} pose pairs, up to {} curves",
        Grid::coarse().candidate_count(),
        Grid::coarse().candidate_count() * 6
    );

    for opening in [2.29_f64, 2.60, 3.00, 4.00] {
        let sc = scene(opening);
        for (label, grid) in [("fine", Grid::fine()), ("coarse", Grid::coarse())] {
            for approach in [Approach::Forward, Approach::Reverse] {
                let started = Instant::now();
                let outcome = search(&vehicle, &sc, approach, grid);
                let elapsed = started.elapsed();
                let found = outcome.best().map_or_else(
                    || "nothing".to_string(),
                    |m| format!("{:.1} cm", m.min_clearance * 100.0),
                );
                println!("{opening:.2} m  {label:6}  {approach:?}  {elapsed:>8.1?}  {found}");
            }
        }
    }
}
