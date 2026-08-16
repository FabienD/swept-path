//! What a plan costs in distance, and whether it wanders.
//!
//! Run with `cargo run -p swept-solver --release --example wander`.

use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::result::Outcome;
use swept_solver::solve::alternatives;

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
        sidewalk_width: 1.30,
        curb_cut_width: opening + 0.80,
        road_width: 5.90,
        curb_height: 0.12,
        gate: GateKind::Sliding,
    }
}

fn main() {
    let ev3 = Vehicle::new(2.680, 4.300, 0.815, 1.850, 2.040, 0.14, 3.496).expect("valid");

    println!(
        "{:>8} {:>7} {:>8} {:>9} {:>8}",
        "opening", "moves", "margin", "walked", "ratio"
    );
    for opening in [2.20, 2.40, 2.60, 3.00, 4.00] {
        let outcome = alternatives(
            &ev3,
            &scene(opening),
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        let Outcome::Found(list) = outcome else {
            println!("{opening:>8.2} {:>7} {:>8} {:>9} {:>8}", "-", "-", "-", "-");
            continue;
        };
        for m in &list {
            let walked: f64 = m
                .poses
                .windows(2)
                .map(|w| (w[1].pose.x - w[0].pose.x).hypot(w[1].pose.y - w[0].pose.y))
                .sum();
            let first = m.poses.first().expect("poses").pose;
            let arrival = m.poses.last().expect("poses").pose;
            let direct = (arrival.x - first.x).hypot(arrival.y - first.y);
            println!(
                "{opening:>8.2} {:>7} {:>7.1}cm {:>8.1}m {:>8.1}",
                m.moves,
                m.min_clearance * 100.0,
                walked,
                walked / direct
            );
        }
    }
}
