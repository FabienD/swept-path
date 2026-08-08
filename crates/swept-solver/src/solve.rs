//! The entry point callers actually use: every alternative, best first.
//!
//! Planning is always seeded by the exhaustive one-move search. That ordering
//! is not an optimisation — it is what guarantees the property `CLAUDE.md`
//! records as acquired: a multi-move answer is never worse than the one-move
//! answer, because the one-move answer is always among the candidates.

use crate::budget::{Progress, SearchBudget};
use crate::exact::{Approach, Grid, search};
use crate::multi::plan;
use crate::result::{Maneuver, Outcome};
use swept_core::kinematics::Direction;
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// Deepest plan offered.
///
/// Past four moves the answer stops being useful advice. ARBITRARY — carried
/// over from the prototype (`index.html:806`).
pub const MAX_MOVES: u8 = 4;

/// Every way in, one alternative per move count.
///
/// The one-move sweep runs first, in both directions unless `allowed`
/// restricts it. Deeper plans follow, and any that fails to beat the one-move
/// clearance is dropped rather than shown: a driver told to make three moves
/// deserves more room than the single-move answer, not less.
#[must_use]
pub fn alternatives(
    vehicle: &Vehicle,
    scene: &Scene,
    budget: SearchBudget,
    progress: &mut impl Progress,
    allowed: Option<Direction>,
) -> Outcome {
    let mut found: Vec<Maneuver> = Vec::new();
    let mut exhausted = false;

    for (approach, direction) in [
        (Approach::Forward, Direction::Forward),
        (Approach::Reverse, Direction::Reverse),
    ] {
        if allowed.is_some_and(|only| only != direction) {
            continue;
        }
        if let Outcome::Found(list) = search(vehicle, scene, approach, Grid::fine()) {
            found.extend(list);
            break;
        }
    }

    let one_move_clearance = found
        .iter()
        .filter(|m| m.moves == 1)
        .map(|m| m.min_clearance)
        .fold(f64::MIN, f64::max);
    let has_one_move = one_move_clearance > f64::MIN;

    for depth in 2..=MAX_MOVES {
        match plan(vehicle, scene, depth, budget, progress, allowed) {
            Outcome::Found(list) => {
                for candidate in list {
                    // Never present a deeper plan that is worse than the exact
                    // one-move answer.
                    if has_one_move && candidate.min_clearance < one_move_clearance {
                        continue;
                    }
                    match found.iter_mut().find(|m| m.moves == candidate.moves) {
                        Some(existing) if candidate.min_clearance > existing.min_clearance => {
                            *existing = candidate;
                        }
                        Some(_) => {}
                        None => found.push(candidate),
                    }
                }
            }
            Outcome::NotFound { budget_exhausted } => exhausted |= budget_exhausted,
        }
    }

    if found.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: exhausted,
        };
    }
    found.sort_by_key(|m| m.moves);
    Outcome::Found(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Silent;
    use swept_core::scene::{GateKind, Post};

    /// The seeding logic under test does not depend on how deep the planner
    /// gets to dig, so the tests do not pay for a full-depth search.
    fn thrifty() -> SearchBudget {
        SearchBudget {
            max_nodes: 6_000,
            ..SearchBudget::default()
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

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_one_move_entry_is_returned_exactly() {
        let outcome = alternatives(
            &lbx(),
            &scene(5.0),
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        let best = outcome.best().expect("5 m admits a one-move entry");
        assert_eq!(best.moves, 1);
        assert!(
            best.is_exact(),
            "a one-move entry comes from the exact sweep"
        );
    }

    #[test]
    fn alternatives_are_ordered_by_move_count_without_duplicates() {
        let outcome = alternatives(
            &lbx(),
            &scene(3.0),
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        if let Outcome::Found(list) = outcome {
            let moves: Vec<u8> = list.iter().map(|m| m.moves).collect();
            let mut sorted = moves.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(moves, sorted, "got {moves:?}");
        }
    }

    #[test]
    fn more_moves_never_yield_less_room_than_the_one_move_answer() {
        // The invariant CLAUDE.md calls out as acquired and not to be lost.
        let (vehicle, sc) = (lbx(), scene(3.2));
        let outcome = alternatives(&vehicle, &sc, thrifty(), &mut Silent, None);
        if let Outcome::Found(list) = outcome
            && let Some(one) = list.iter().find(|m| m.moves == 1)
        {
            for other in list.iter().filter(|m| m.moves > 1) {
                assert!(
                    other.min_clearance >= one.min_clearance - 1e-9,
                    "{} moves gave {} against {} for one move",
                    other.moves,
                    other.min_clearance,
                    one.min_clearance
                );
            }
        }
    }

    #[test]
    fn a_blocked_opening_yields_nothing_at_any_depth() {
        let outcome = alternatives(
            &lbx(),
            &scene(1.6),
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        assert!(outcome.best().is_none());
    }
}
