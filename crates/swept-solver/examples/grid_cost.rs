//! Measures what refining the planning grid costs.
//!
//! Run with `cargo run -p swept-solver --release --example grid_cost`.
//!
//! The prototype planned on 90 cm primitives with a 6° heading step.
//! `CLAUDE.md` calls for 20 cm and 1°. This reports whether the node budget
//! still suffices at that resolution, on scenes tight enough to make the
//! planner work.

use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{Discretisation, Progress, SearchBudget};
use swept_solver::multi::plan;
use swept_solver::result::Outcome;

/// Records the last progress report, which is the node count reached.
#[derive(Default)]
struct Counter(u32);
impl Progress for Counter {
    fn nodes_expanded(&mut self, _moves: u8, expanded: u32) {
        self.0 = expanded;
    }
}

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

fn main() {
    let vehicle =
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle");

    println!(
        "{:<10} {:>8} {:>10} {:>11} {:>7}  note",
        "grid", "opening", "nodes", "clearance", "moves"
    );
    for (name, discretisation) in [
        ("default", Discretisation::default()),
        ("fine", Discretisation::fine()),
    ] {
        for opening in [2.2, 2.6, 3.0, 4.0] {
            let budget = SearchBudget {
                discretisation,
                ..SearchBudget::default()
            };
            let mut counter = Counter::default();
            let outcome = plan(&vehicle, &scene(opening), 4, budget, &mut counter, None);
            let (clearance, moves) = match outcome.best() {
                Some(m) => (format!("{:.3}", m.min_clearance), m.moves.to_string()),
                None => (String::from("none"), String::from("-")),
            };
            let note = if matches!(
                outcome,
                Outcome::NotFound {
                    budget_exhausted: true
                }
            ) {
                "budget exhausted"
            } else {
                ""
            };
            println!(
                "{name:<10} {opening:>8.2} {:>10} {clearance:>11} {moves:>7}  {note}",
                counter.0
            );
        }
    }
}
