//! Multi-move planning by hybrid A\*.
//!
//! Search runs over `(x, y, heading, direction)`. The dominant cost is the
//! number of direction changes — reversing is what a driver counts — with
//! distance as a tie-breaker.
//!
//! Unlike the prototype, nothing here consults a clock: the search stops when
//! it runs out of nodes, not when it runs out of time, so the same inputs
//! always yield the same plan.

use crate::budget::{Discretisation, Progress, SearchBudget};
use crate::landing::{Landing, landings};
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::f64::consts::FRAC_PI_2;
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::{Direction, Pose, sample_arc};
use swept_core::scene::{GateKind, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Cost charged for each direction change.
///
/// Dominant by design: a driver counts reverses, not metres. ARBITRARY in
/// magnitude — carried over from the prototype (`index.html:513`).
pub const MOVE_COST: f64 = 5.0;

/// Cost charged per metre travelled, as a tie-breaker.
///
/// ARBITRARY — carried over from the prototype (`index.html:513`).
pub const LENGTH_COST_PER_M: f64 = 0.18;

/// Below this clearance, a plan starts paying for being tight, in metres.
///
/// ARBITRARY in magnitude, but not in choice: it is the wider of the two alert
/// bands the interface already draws, so the planner charges for exactly what
/// the interface calls "vigilance". Above it, room is free — a plan with 30 cm
/// to spare has nothing to gain from 40.
pub const TIGHTNESS_THRESHOLD_M: f64 = 0.25;

/// Cost charged per metre of shortfall below [`TIGHTNESS_THRESHOLD_M`].
///
/// **Chosen so that room can never outweigh a manoeuvre.** The shortfall is
/// bounded by the threshold itself — a zero margin falls short by the whole of
/// it — so the worst a plan can pay is `0.25 * 16 = 4.0`, against
/// [`MOVE_COST`] of 5.0 for one more shunt. The property therefore holds on
/// every scene by arithmetic rather than by calibration, and a test pins it.
pub const TIGHTNESS_COST: f64 = 16.0;

/// Room can never outweigh a manoeuvre, and the compiler enforces it.
///
/// Stronger than a test: change either constant so that a plan could buy its
/// way past a shunt, and the crate stops building.
const _: () = assert!(TIGHTNESS_THRESHOLD_M * TIGHTNESS_COST < MOVE_COST);

/// Weight given to heading error in the heuristic.
///
/// ARBITRARY — carried over from the prototype (`index.html:471`).
pub const HEADING_ERROR_WEIGHT: f64 = 2.2;

/// Where along the road the planner starts, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:474`).
pub const START_X_M: f64 = -6.5;

/// How many lateral start positions are seeded.
///
/// ARBITRARY — carried over from the prototype (`index.html:472`).
pub const START_POSITIONS: u8 = 10;

/// How often an analytic landing is attempted, in expanded nodes.
///
/// **Without this the planner is unusable.** A landing now enumerates
/// closed-form curves to a grid of goal poses, samples each and walks it
/// against every obstacle. That is the right answer per attempt and far too
/// expensive to make at every node: the suite ran past ten minutes before this
/// bound existed, where it took seconds.
///
/// Spacing the attempts is the standard remedy for hybrid A\* — the analytic
/// expansion is tried periodically rather than always, because a landing that
/// exists is still found a few nodes later, and a landing that does not exist
/// costs the same either way.
///
/// ARBITRARY in magnitude, and the figure to lower first if plans come back
/// worse than the batch before.
pub const ANALYTIC_EVERY: u32 = 20;

/// Distance from the opening centre within which a landing is attempted.
///
/// ARBITRARY — carried over from the prototype (`index.html:490`).
pub const LANDING_TRIGGER_X_M: f64 = 2.4;

/// Heading error within which a landing is attempted, in radians.
///
/// ARBITRARY — carried over from the prototype (`index.html:490`).
pub const LANDING_TRIGGER_HEADING_RAD: f64 = 1.0;

/// Fractions of the tightest turning radius used as motion primitives.
///
/// Hard left, gentle left, straight, gentle right, hard right. The 1.6 divisor
/// is ARBITRARY — carried over from the prototype (`index.html:461`).
pub const CURVATURE_FRACTIONS: [f64; 5] = [-1.0, -1.0 / 1.6, 0.0, 1.0 / 1.6, 1.0];

/// How deep the vehicle must reach for the goal heuristic, in metres.
///
/// Shallower than the full entry depth: the heuristic only has to point the
/// search the right way. ARBITRARY — carried over from the prototype
/// (`index.html:413`).
const GOAL_MARGIN_M: f64 = 0.45;

/// How often progress is reported, in expanded nodes.
const PROGRESS_EVERY: u32 = 500;

/// Clearance kept from the lane edges when seeding, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:463`).
const SEED_MARGIN_LOW_M: f64 = 0.03;
const SEED_MARGIN_HIGH_M: f64 = 0.05;

/// A node in the search, held in an arena and referred to by index.
#[derive(Debug, Clone)]
struct Node {
    pose: Pose,
    direction: Direction,
    moves: u8,
    travelled: f64,
    /// Worst shortfall below [`TIGHTNESS_THRESHOLD_M`] anywhere on the way
    /// here, in metres.
    ///
    /// The **worst**, not the sum: a plan is as tight as its tightest moment,
    /// and summing would let a long roomy plan out-cost a short grazing one.
    /// It can only grow along a path, so the cost stays monotonic and the A\*
    /// stays correct.
    worst_shortfall: f64,
    parent: Option<usize>,
    segment: Vec<Pose>,
}

/// A heap entry ordered so that `BinaryHeap` pops the cheapest score first.
#[derive(Debug, Clone, Copy)]
struct Ranked {
    score: f64,
    index: usize,
}

impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed: BinaryHeap is a max-heap, we want the smallest score.
        // `total_cmp` gives a total order over f64 without the pitfalls of
        // comparing floats directly.
        other.score.total_cmp(&self.score)
    }
}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Ranked {}

/// The visited-state cell a pose falls into.
fn cell(pose: &Pose, direction: Direction, grid: Discretisation) -> (i64, i64, i64, bool) {
    #[allow(clippy::cast_possible_truncation)]
    let x = (pose.x / grid.position_step).round() as i64;
    #[allow(clippy::cast_possible_truncation)]
    let y = (pose.y / grid.position_step).round() as i64;
    #[allow(clippy::cast_possible_truncation)]
    let h = (pose.heading.get() / grid.heading_step.get()).round() as i64;
    (x, y, h, direction == Direction::Forward)
}

/// Optimistic remaining cost: distance to the goal plus heading error.
fn heuristic(pose: &Pose, goal: f64) -> f64 {
    pose.x.hypot((goal - pose.y).max(0.0))
        + HEADING_ERROR_WEIGHT * (FRAC_PI_2 - pose.heading.get()).abs()
}

/// Turns one finished search branch back into a manoeuvre.
fn assemble(
    arena: &[Node],
    goal_index: usize,
    landing: &Landing,
    vehicle: &Vehicle,
    field: &ClearanceField,
    exhausted: bool,
    moves: u8,
) -> Maneuver {
    let mut chain: Vec<usize> = Vec::new();
    let mut cursor = Some(goal_index);
    while let Some(i) = cursor {
        chain.push(i);
        cursor = arena[i].parent;
    }
    chain.reverse();

    let mut poses: Vec<DirectedPose> = Vec::new();
    for i in chain {
        let node = &arena[i];
        for pose in &node.segment {
            poses.push(DirectedPose {
                pose: *pose,
                direction: node.direction,
            });
        }
    }
    for pose in &landing.poses {
        poses.push(DirectedPose {
            pose: *pose,
            direction: landing.direction,
        });
    }

    let min_clearance = poses
        .iter()
        .filter_map(|p| match field.at(p.pose) {
            Clearance::Clear(margin) => Some(margin),
            Clearance::Collision => None,
        })
        .fold(None, |acc: Option<f64>, m| {
            Some(acc.map_or(m, |a| a.min(m)))
        })
        .unwrap_or(landing.min_clearance);

    // Reduce, then **re-measure**. The reduction can only improve the room —
    // it takes a shortcut solely when one leaves at least as much — but
    // reporting the figure from before would describe a path that is no longer
    // the one returned, which is exactly the failure this project corrects
    // elsewhere. The move count is recounted for the same reason.
    let poses = crate::shortcut::reduce(&poses, vehicle, field);
    let min_clearance = crate::shortcut::tightest(&poses, field).max(min_clearance);
    let moves = u8::try_from(crate::shortcut::reversal_count(&poses) + 1)
        .unwrap_or(u8::MAX)
        .min(moves);

    Maneuver {
        poses,
        min_clearance,
        moves,
        confidence: Confidence::Heuristic {
            budget_exhausted: exhausted,
        },
    }
}

/// The search frontier: every visited node, the open set, and the cells
/// already claimed.
struct Frontier {
    arena: Vec<Node>,
    heap: BinaryHeap<Ranked>,
    seen: HashSet<(i64, i64, i64, bool)>,
}

/// Spreads start poses across the carriageway.
///
/// Returns `None` when the vehicle is wider than the lane it would have to
/// start in, which no amount of searching can fix.
fn seed(
    vehicle: &Vehicle,
    scene: &Scene,
    field: &ClearanceField,
    grid: Discretisation,
    goal: f64,
) -> Option<Frontier> {
    let half_width = vehicle.mirror_width / 2.0;
    let low = -scene.pavement_width - scene.road_width + half_width + SEED_MARGIN_LOW_M;
    let high = -half_width - SEED_MARGIN_HIGH_M;
    if low > high {
        return None;
    }

    let mut frontier = Frontier {
        arena: Vec::new(),
        heap: BinaryHeap::new(),
        seen: HashSet::new(),
    };

    for j in 0..=START_POSITIONS {
        let y = low + (high - low) * f64::from(j) / f64::from(START_POSITIONS);
        let pose = Pose::new(START_X_M, y, Radians::default());
        if field.at(pose) == Clearance::Collision {
            continue;
        }
        let score = MOVE_COST + heuristic(&pose, goal);
        frontier.seen.insert(cell(&pose, Direction::Forward, grid));
        frontier.arena.push(Node {
            pose,
            direction: Direction::Forward,
            moves: 1,
            travelled: 0.0,
            worst_shortfall: 0.0,
            parent: None,
            segment: Vec::new(),
        });
        frontier.heap.push(Ranked {
            score,
            index: frontier.arena.len() - 1,
        });
    }
    Some(frontier)
}

/// How far a clearance falls below the threshold, in metres. Zero above it.
fn shortfall(margin: f64) -> f64 {
    (TIGHTNESS_THRESHOLD_M - margin).max(0.0)
}

/// What a node is ranked on: shunts, then distance, then how tight it got.
fn score_of(moves: u8, travelled: f64, worst_shortfall: f64, end: &Pose, goal: f64) -> f64 {
    f64::from(moves) * MOVE_COST
        + travelled * LENGTH_COST_PER_M
        + worst_shortfall * TIGHTNESS_COST
        + heuristic(end, goal)
}

/// The smallest clearance along a segment, or `None` if it collides.
///
/// The collision test already asks the field for every margin here and used to
/// throw them away. Keeping the smallest costs nothing, and it is what lets a
/// plan value room.
fn tightest_margin(segment: &[Pose], field: &ClearanceField) -> Option<f64> {
    let mut tightest = f64::INFINITY;
    for probe in segment {
        match field.at(*probe) {
            Clearance::Collision => return None,
            Clearance::Clear(margin) => tightest = tightest.min(margin),
        }
    }
    Some(tightest)
}

fn goal_depth(scene: &Scene) -> f64 {
    let gate = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth) + gate + GOAL_MARGIN_M
}

/// Plans an entry in at most `max_moves` moves.
#[must_use]
pub fn plan(
    vehicle: &Vehicle,
    scene: &Scene,
    max_moves: u8,
    budget: SearchBudget,
    progress: &mut impl Progress,
    allowed: Option<Direction>,
) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let grid = budget.discretisation;
    let goal = goal_depth(scene);

    let Some(Frontier {
        mut arena,
        mut heap,
        mut seen,
    }) = seed(vehicle, scene, &field, grid, goal)
    else {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    };

    let mut expanded: u32 = 0;
    // The roomiest landing found for each total move count. One search now
    // answers every depth, instead of one search per depth re-exploring the
    // same space — which cost three times over for nothing.
    let mut best: Vec<Option<(usize, Landing)>> = vec![None; usize::from(max_moves) + 2];
    // Counted per move count, not overall: a single quota would fill up with
    // landings at one depth and stop the search before it ever reached the
    // others.
    let mut solutions: Vec<u16> = vec![0; usize::from(max_moves) + 2];
    let mut exhausted = false;

    while let Some(Ranked { index, .. }) = heap.pop() {
        if expanded >= budget.max_nodes {
            exhausted = true;
            break;
        }
        expanded += 1;
        if expanded.is_multiple_of(PROGRESS_EVERY) {
            progress.nodes_expanded(max_moves, expanded);
        }

        let (pose, direction, moves, travelled, worst_shortfall) = {
            let node = &arena[index];
            (
                node.pose,
                node.direction,
                node.moves,
                node.travelled,
                node.worst_shortfall,
            )
        };

        let heading_error = (FRAC_PI_2 - pose.heading.get())
            .abs()
            .min((-FRAC_PI_2 - pose.heading.get()).abs());
        if pose.x.abs() < LANDING_TRIGGER_X_M && heading_error < LANDING_TRIGGER_HEADING_RAD {
            // The shaped landing runs every time; only the closed-form one is
            // spaced out, because only it is expensive.
            let with_curves = expanded.is_multiple_of(ANALYTIC_EVERY);
            // Each landing is filed under what it actually costs: turning
            // round to back in is a move, and a roomier landing that spends
            // one is not the same answer as a tighter one that does not.
            for landing in landings(pose, vehicle, scene, &field, allowed, with_curves) {
                // Every shunt the curve makes counts, not just its final
                // gear. The old landing was one arc and a straight run and
                // could not shunt at all; a closed-form curve can, and a plan
                // claiming two moves while driving four is worse than useless.
                let total = moves + landing.moves(direction);
                let Some(slot) = best.get_mut(usize::from(total)) else {
                    continue;
                };
                if slot
                    .as_ref()
                    .is_none_or(|(_, b)| landing.min_clearance > b.min_clearance)
                {
                    *slot = Some((index, landing));
                }
                solutions[usize::from(total)] += 1;
            }
            // Nothing left to improve anywhere: stop.
            if solutions.iter().all(|&n| n >= budget.max_solutions) {
                break;
            }
        }

        if moves > max_moves {
            continue;
        }

        for fraction in CURVATURE_FRACTIONS {
            for step_direction in [Direction::Forward, Direction::Reverse] {
                let next_moves = moves + u8::from(step_direction != direction);
                if next_moves > max_moves {
                    continue;
                }

                let curvature = fraction / vehicle.min_turning_radius;
                let signed = match step_direction {
                    Direction::Forward => grid.primitive_length,
                    Direction::Reverse => -grid.primitive_length,
                };
                let segment = sample_arc(pose, curvature, signed, grid.sample_step);
                let Some(tightest) = tightest_margin(&segment, &field) else {
                    continue;
                };
                let end = pose.advance(curvature, signed);

                if !seen.insert(cell(&end, step_direction, grid)) {
                    continue;
                }

                let next_travelled = travelled + grid.primitive_length;
                let next_worst = worst_shortfall.max(shortfall(tightest));
                let score = score_of(next_moves, next_travelled, next_worst, &end, goal);
                arena.push(Node {
                    pose: end,
                    direction: step_direction,
                    moves: next_moves,
                    travelled: next_travelled,
                    worst_shortfall: next_worst,
                    parent: Some(index),
                    segment,
                });
                heap.push(Ranked {
                    score,
                    index: arena.len() - 1,
                });
            }
        }
    }

    collect(&arena, &best, vehicle, &field, exhausted)
}

/// Turns the best landing of each move count into an outcome.
fn collect(
    arena: &[Node],
    best: &[Option<(usize, Landing)>],
    vehicle: &Vehicle,
    field: &ClearanceField,
    exhausted: bool,
) -> Outcome {
    let found: Vec<Maneuver> = best
        .iter()
        .enumerate()
        .filter_map(|(total, slot)| {
            let (index, landing) = slot.as_ref()?;
            #[allow(clippy::cast_possible_truncation)]
            Some(assemble(
                arena,
                *index,
                landing,
                vehicle,
                field,
                exhausted,
                total as u8,
            ))
        })
        .collect();

    // Reduction can drop a shunt, so two plans filed under different depths
    // may come back with the same move count. Keep the roomiest of each, which
    // is the same rule the alternatives filter applies downstream: only what
    // buys room survives.
    let mut found = found;
    found.sort_by(|a, b| {
        a.moves
            .cmp(&b.moves)
            .then_with(|| b.min_clearance.total_cmp(&a.min_clearance))
    });
    found.dedup_by_key(|m| m.moves);

    if found.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: exhausted,
        };
    }
    Outcome::Found(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Silent;
    use swept_core::scene::Post;

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
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_generous_opening_is_planned_within_a_few_moves() {
        let outcome = plan(
            &lbx(),
            &scene(4.0),
            3,
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        let best = outcome.best().expect("4 m should be plannable");
        assert!(
            best.moves >= 1 && best.moves <= 3,
            "got {} moves",
            best.moves
        );
    }

    /// One search, every depth. Exploring once and reading off each move
    /// count is what removes the threefold cost of planning depth by depth.
    #[test]
    fn a_single_search_yields_one_alternative_per_move_count() {
        let outcome = plan(
            &lbx(),
            &scene(2.6),
            4,
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        if let Outcome::Found(list) = outcome {
            let counts: Vec<u8> = list.iter().map(|m| m.moves).collect();
            let mut unique = counts.clone();
            unique.sort_unstable();
            unique.dedup();
            assert_eq!(counts.len(), unique.len(), "one per move count: {counts:?}");
            assert!(
                list.len() > 1,
                "a 2.6 m opening admits several depths, got {counts:?}"
            );
        }
    }

    #[test]
    fn planner_results_never_claim_to_be_exact() {
        let outcome = plan(
            &lbx(),
            &scene(4.0),
            3,
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        assert!(!outcome.best().expect("a plan").is_exact());
    }

    #[test]
    fn the_planned_path_is_actually_collision_free() {
        let (vehicle, sc) = (lbx(), scene(3.5));
        let outcome = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        if let Some(best) = outcome.best() {
            let field = ClearanceField::new(&sc, &vehicle);
            for step in &best.poses {
                assert_ne!(field.at(step.pose), Clearance::Collision);
            }
        }
    }

    #[test]
    fn the_same_inputs_always_give_the_same_result() {
        // The whole point of counting nodes instead of milliseconds.
        let (vehicle, sc) = (lbx(), scene(3.5));
        let once = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        let twice = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        assert_eq!(once, twice);
    }

    #[test]
    fn a_starved_budget_reports_that_it_ran_out() {
        let budget = SearchBudget {
            max_nodes: 5,
            ..SearchBudget::default()
        };
        let outcome = plan(&lbx(), &scene(2.2), 4, budget, &mut Silent, None);
        match outcome {
            Outcome::NotFound { budget_exhausted } => assert!(budget_exhausted),
            Outcome::Found(list) => assert!(list.iter().all(|m| !m.is_exact())),
        }
    }

    /// The default grid must be affordable under the default budget.
    ///
    /// This is the pairing that ships, and the one that broke: 18 000 nodes —
    /// the prototype's ceiling — returned nothing at all once the grid was
    /// refined to 20 cm and 1°.
    #[test]
    fn the_default_grid_fits_within_the_default_budget() {
        let outcome = plan(
            &lbx(),
            &scene(4.0),
            3,
            SearchBudget::default(),
            &mut Silent,
            None,
        );
        assert!(
            outcome.best().is_some(),
            "the default budget must suffice for the default grid"
        );
    }

    #[test]
    fn progress_is_reported_while_searching() {
        #[derive(Default)]
        struct Spy(u32);
        impl Progress for Spy {
            fn nodes_expanded(&mut self, _moves: u8, _expanded: u32) {
                self.0 += 1;
            }
        }
        let mut spy = Spy::default();
        let _ = plan(
            &lbx(),
            &scene(2.6),
            3,
            SearchBudget::default(),
            &mut spy,
            None,
        );
        assert!(spy.0 > 0, "the planner never reported progress");
    }

    #[test]
    fn the_planner_prefers_the_roomier_of_two_plans() {
        // A wide opening leaves the planner a choice: it can graze a post or
        // stay off it for the same number of moves. Before this batch nothing
        // in the cost told the two apart, so it took whichever was shorter.
        //
        // MEASURED by `examples/grid_cost`: on a 4 m opening the default grid
        // returned 2.6 cm before and 4.9 cm after. The floor below is set well
        // under that, since the point is to catch a planner that has gone back
        // to grazing, not to freeze a figure that will drift with the grid.
        let (vehicle, sc) = (lbx(), scene(4.0));
        let outcome = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        let best = outcome.best().expect("4 m should be plannable");
        let field = ClearanceField::new(&sc, &vehicle);
        let tightest = best
            .poses
            .iter()
            .filter_map(|step| match field.at(step.pose) {
                Clearance::Clear(margin) => Some(margin),
                Clearance::Collision => None,
            })
            .fold(f64::INFINITY, f64::min);
        assert!(
            tightest > 0.04,
            "the planner still grazes: {:.1} cm at its tightest",
            tightest * 100.0
        );
    }
}
