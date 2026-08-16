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

    // One search, every depth. plan() explores the space once and reports the
    // roomiest landing for each move count, so calling it per depth would
    // re-explore the same states three times over for nothing.
    match plan(vehicle, scene, MAX_MOVES, budget, progress, allowed) {
        Outcome::Found(list) => {
            for candidate in list {
                // Never present a deeper plan that is worse than the exact
                // one-move answer.
                if has_one_move && candidate.min_clearance < one_move_clearance {
                    continue;
                }
                match found.iter_mut().find(|m| m.moves == candidate.moves) {
                    // A heuristic result never displaces an exact one. The
                    // sweep is exhaustive on its grid and says so; letting a
                    // planner overwrite it would raise the one-move figure
                    // that every deeper plan is measured against, and deeper
                    // plans already filtered against the old one would end up
                    // below it — breaking multi-never-worse-than-simple.
                    Some(existing) if existing.is_exact() => {}
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

    if found.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: exhausted,
        };
    }

    // Drop dominated alternatives. Extra manoeuvres have to buy room: an
    // answer in three moves offering no more than the one in a single move is
    // not an option, it is noise. The earlier guard only compared against the
    // *exact* one-move answer, so on a scene where the exhaustive sweep finds
    // nothing — most tight ones — there was no reference and everything got
    // through.
    found.sort_by_key(|m| m.moves);
    let mut roomiest = f64::MIN;
    found.retain(|m| {
        let keep = m.min_clearance > roomiest;
        if keep {
            roomiest = m.min_clearance;
        }
        keep
    });

    Outcome::Found(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Silent;
    use swept_core::scene::{GateKind, Post};
    use swept_core::vehicle::pivot_radius_from_curb;

    /// The seeding logic under test does not depend on how deep the planner
    /// gets to dig, so the tests do not pay for a full-depth search.
    fn thrifty() -> SearchBudget {
        SearchBudget {
            max_nodes: 6_000,
            ..SearchBudget::default()
        }
    }

    pub(super) fn scene(opening: f64) -> Scene {
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
            sidewalk_width: 1.20,
            curb_cut_width: opening + 0.80,
            road_width: 4.50,
            curb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    pub(super) fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    /// Which way the vehicle is going as it crosses into the yard.
    ///
    /// The last pose, not the first: asking to enter in reverse is a claim
    /// about how the gateway is crossed, not about how the street is driven.
    fn arrives_in(maneuver: &Maneuver) -> Direction {
        maneuver.poses.last().expect("a path has poses").direction
    }

    /// Reported from production, with a screenshot: the vehicle is through
    /// the gateway, straight and centred, and the plan goes on manoeuvring.
    ///
    /// Measured on that scene rather than judged from the picture. The
    /// two-move answer drove straight past the opening to `y = 5.64` while
    /// turning away from square, then reversed two metres back to `y = 4.60`
    /// — a shunt three metres clear of anything, made when the vehicle had
    /// been entirely in the yard for four metres. What a driver is told to do
    /// after they are in is not part of getting in.
    #[test]
    fn no_alternative_shunts_once_the_vehicle_is_through() {
        let pivot = pivot_radius_from_curb(5.61, 2.45, 1.591).expect("plausible");
        let porsche = Vehicle::new(2.45, 4.542, 1.0, 1.852, 2.033, 0.11, pivot).expect("valid");
        let scene = Scene {
            left_post: Post {
                inner_edge_x: -1.225,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 1.225,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            sidewalk_width: 1.30,
            curb_cut_width: 3.60,
            road_width: 5.90,
            curb_height: 0.12,
            gate: GateKind::Sliding,
        };

        let Outcome::Found(list) =
            alternatives(&porsche, &scene, SearchBudget::default(), &mut Silent, None)
        else {
            panic!("this gateway admits an entry");
        };

        // Behind the posts, every corner of the vehicle: that is what being
        // through means, and it does not depend on which way the vehicle
        // faces or on the depth the goal happens to be set at.
        let behind = scene.left_post.depth.max(scene.right_post.depth);
        let envelope = porsche.envelope();
        let through = |pose: &swept_core::kinematics::Pose| {
            let (sin, cos) = pose.heading.sin_cos();
            envelope
                .iter()
                .map(|p| pose.y + p.x * sin + p.y * cos)
                .fold(f64::MAX, f64::min)
                > behind
        };

        for m in &list {
            let Some(in_at) = m.poses.iter().position(|p| through(&p.pose)) else {
                continue;
            };
            let shunts = m.poses[in_at..]
                .windows(2)
                .filter(|w| w[0].direction != w[1].direction)
                .count();
            assert_eq!(
                shunts,
                0,
                "{} moves: {shunts} gear change(s) after the vehicle was already through, at pose {in_at} of {}",
                m.moves,
                m.poses.len()
            );
        }
    }

    /// Reported from production: the trace wanders once through the gateway.
    ///
    /// Checked on `alternatives`, which is what the interface calls, and on
    /// every depth it returns — not just the one that happens to be selected.
    #[test]
    fn no_alternative_wanders_around_the_yard() {
        let ev3 = Vehicle::new(2.680, 4.300, 0.815, 1.850, 2.040, 0.14, 3.496).expect("valid");
        let mut sc = scene(2.40);
        sc.road_width = 5.90;
        sc.sidewalk_width = 1.30;
        sc.curb_height = 0.12;

        let Outcome::Found(list) =
            alternatives(&ev3, &sc, SearchBudget::default(), &mut Silent, None)
        else {
            panic!("this gateway admits an entry");
        };

        for m in &list {
            let travelled: f64 = m
                .poses
                .windows(2)
                .map(|w| (w[1].pose.x - w[0].pose.x).hypot(w[1].pose.y - w[0].pose.y))
                .sum();
            let first = m.poses.first().expect("poses").pose;
            let arrival = m.poses.last().expect("poses").pose;
            let direct = (arrival.x - first.x).hypot(arrival.y - first.y);
            assert!(
                travelled < direct * 3.0,
                "{} moves: walked {travelled:.1} m to cover {direct:.1} m",
                m.moves
            );
        }
    }

    #[test]
    fn asking_to_enter_in_reverse_never_answers_with_a_forward_entry() {
        // Reported: a run restricted to reverse came back with a one-move
        // forward entry. A wide opening is the case that catches it, because
        // both directions succeed there and the forward sweep runs first.
        let outcome = alternatives(
            &lbx(),
            &scene(5.0),
            SearchBudget::default(),
            &mut Silent,
            Some(Direction::Reverse),
        );
        let Outcome::Found(list) = outcome else {
            panic!("5 m admits a reverse entry");
        };
        for maneuver in &list {
            assert_eq!(
                arrives_in(maneuver),
                Direction::Reverse,
                "a {}-move answer entered forwards",
                maneuver.moves
            );
        }
    }

    #[test]
    fn asking_to_enter_forwards_never_answers_with_a_reverse_entry() {
        let outcome = alternatives(
            &lbx(),
            &scene(5.0),
            SearchBudget::default(),
            &mut Silent,
            Some(Direction::Forward),
        );
        let Outcome::Found(list) = outcome else {
            panic!("5 m admits a forward entry");
        };
        for maneuver in &list {
            assert_eq!(arrives_in(maneuver), Direction::Forward);
        }
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

    /// Regression, found by proptest.
    ///
    /// Once `plan()` started reporting a one-move alternative of its own, a
    /// heuristic result with more clearance than the exhaustive sweep would
    /// replace it. That raised the figure every deeper plan is measured
    /// against, after those plans had already been filtered against the old
    /// one — so multi came out worse than simple on this exact scene.
    #[test]
    fn a_heuristic_plan_never_displaces_the_exact_one_move_answer() {
        let vehicle = lbx();
        let mut sc = scene(2.701_696_162_945_853);
        sc.road_width = 5.795_297_516_544_246;

        let Outcome::Found(list) = alternatives(&vehicle, &sc, thrifty(), &mut Silent, None) else {
            panic!("this scene admits an entry");
        };

        let one = list
            .iter()
            .find(|m| m.moves == 1)
            .expect("a one-move answer exists here");
        assert!(
            one.is_exact(),
            "the one-move answer must stay the exact one"
        );
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

    /// Regression: three manoeuvres offered 0.3 cm where one offered 4.5.
    ///
    /// The guard only compared against the *exact* one-move answer, so on a
    /// scene where the exhaustive sweep finds nothing — which is most tight
    /// ones — there was no reference at all and everything got through.
    #[test]
    fn more_moves_must_buy_more_room_than_every_shorter_answer() {
        // Fabien's gateway, with the pivot radius rather than the published
        // one: swinging leaves at 90 degrees, 1.25 m sidewalk, 6.20 m road.
        let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59).expect("valid");
        for opening in [2.30_f64, 2.40, 2.60, 3.00] {
            let mut sc = scene(opening);
            sc.sidewalk_width = 1.25;
            sc.road_width = 6.20;
            sc.curb_cut_width = 3.20;
            sc.gate = GateKind::Swinging {
                leaf_length: 1.15,
                leaf_thickness: 0.04,
                hinge_offset: 0.035,
                hinge_depth_ratio: 0.5,
                open_angle: swept_core::units::Radians::from_degrees(90.0),
            };
            let Outcome::Found(list) =
                alternatives(&vehicle, &sc, SearchBudget::default(), &mut Silent, None)
            else {
                continue;
            };
            for (i, deeper) in list.iter().enumerate() {
                for shorter in &list[..i] {
                    assert!(
                        deeper.min_clearance > shorter.min_clearance,
                        "{opening} m: {} moves gave {} against {} for {} moves",
                        deeper.moves,
                        deeper.min_clearance,
                        shorter.min_clearance,
                        shorter.moves
                    );
                }
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
