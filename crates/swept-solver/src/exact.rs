//! Exhaustive search for a one-move entry.
//!
//! Every pair of poses on the grid is tried, joined by every Dubins curve that
//! applies at every radius, and the roomiest collision-free result is kept.
//! Because the sweep is complete, a failure here means something: there is no
//! one-move entry *on this grid*. That is what makes this solver the reference
//! the planner is seeded from.
//!
//! # Why every curve and not the shortest
//!
//! Dubins curves minimise length. This search maximises clearance, and the
//! shortest path is the one that grazes most — so it asks for all of them and
//! sorts by room. Length never enters the comparison.
//!
//! # Reverse
//!
//! Backing along a path covers exactly the ground that driving forward along
//! it in the other direction covers. A reverse entry is therefore a Dubins
//! problem too: the same curves, read from the goal back to the start with
//! both headings turned about.
//!
//! # What a failure here does and does not mean
//!
//! It means no one-move entry exists on this grid — not that the vehicle
//! cannot get in. It may still get in over several moves, which is what
//! `multi` is for. And the sweep only knows the geometry it is given: a
//! gateway this search refuses may well be one a driver threads daily, if the
//! model charges for something reality does not. The pavement is the standing
//! example — modelled as a wall of infinite height, when a mirror a metre up
//! clears a fifteen-centimetre kerb without noticing it.

use crate::budget::Discretisation;
use crate::path::evaluate_at_least;
use crate::poses::{goal_poses, start_poses};
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use std::f64::consts::{PI, TAU};
use swept_core::clearance::ClearanceField;
use swept_core::curves::CurvePath;
use swept_core::curves::dubins;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Which way the vehicle drives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approach {
    /// Driving in nose first.
    Forward,
    /// Backing in.
    Reverse,
}

/// Increment between the turning radii tried, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:368`).
pub const RADIUS_STEP_M: f64 = 0.5;

/// How many values of each parameter the sweep tries.
///
/// Every count is a number of *intervals*, so a count of `n` yields `n + 1`
/// values, and zero yields the midpoint alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Turning radii, from the vehicle's tightest upwards.
    pub radius_steps: u16,
    /// Positions along the road the approach may start from.
    pub start_x_steps: u16,
    /// Positions across the carriageway the approach may start from.
    pub lateral_steps: u16,
    /// Points along the opening the entry is aimed at.
    pub entry_steps: u16,
    /// Final headings tried, spread about square to the opening.
    pub heading_steps: u16,
}

impl Grid {
    /// The full sweep, used whenever the answer is shown to a user.
    ///
    /// Coarser per axis than the shape-based sweep it replaces, and richer
    /// overall: one pair of poses yields up to six curves where the old
    /// parameters yielded one path.
    ///
    /// MEASURED by `examples/bench.rs`: 38 025 pose pairs, 766 ms for a
    /// forward sweep and 1.1 s for a reverse one, against 29 and 44 ms for
    /// [`Grid::coarse`]. The budget is the second or so beyond which a tool
    /// stops feeling like it answers.
    ///
    /// The counts are not uniform because the axes are not worth the same.
    /// Measured on a 2.29 m opening, doubling `entry_steps`, `heading_steps`
    /// or `radius_steps` bought **no clearance at all**, while `start_x_steps`
    /// took it from 0.1 cm to 4.2 cm. That axis decides *where the turn
    /// begins*, and the window where a turn lands square in a narrow opening
    /// is a few centimetres wide — so it gets the budget the other three do
    /// not need. Starving it is also what made the carriageway bisection
    /// return nothing at all.
    ///
    /// Every count is even, which matters: the grids are inclusive of their
    /// bounds, so an odd count would straddle the centre value rather than
    /// land on it — and dead centre is exactly where an entry is aimed.
    #[must_use]
    pub fn fine() -> Self {
        Self {
            radius_steps: 2,
            start_x_steps: 64,
            lateral_steps: 12,
            entry_steps: 4,
            heading_steps: 2,
        }
    }

    /// A cheaper sweep, for callers that run the search many times over —
    /// the carriageway bisection in particular.
    ///
    /// **Every count divides its counterpart in [`Grid::fine`], on purpose.**
    /// The pose grids place value `i` of `n` at `low + (high - low) * i / n`,
    /// so halving the count keeps exactly every other value — and keeps it
    /// *bit for bit*, since IEEE division is correctly rounded and `2i / 2n`
    /// and `i / n` are the same rational. The coarse sweep therefore
    /// tries a strict subset of what the fine one tries, which makes "finer is
    /// never worse" a property rather than a hope. The radii need no such care:
    /// they step by a fixed increment from the vehicle's tightest, so a smaller
    /// count is simply a prefix.
    #[must_use]
    pub fn coarse() -> Self {
        Self {
            radius_steps: 1,
            start_x_steps: 16,
            lateral_steps: 4,
            entry_steps: 2,
            heading_steps: 2,
        }
    }

    /// How many pose pairs this grid produces, useful for reporting cost.
    ///
    /// Each pair yields up to six Dubins curves, so the number of paths
    /// actually evaluated is at most six times this.
    #[must_use]
    pub fn candidate_count(self) -> u64 {
        let starts = u64::from(self.start_x_steps + 1) * u64::from(self.lateral_steps + 1);
        let goals = u64::from(self.entry_steps + 1) * u64::from(self.heading_steps + 1);
        u64::from(self.radius_steps + 1) * starts * goals
    }
}

/// Sweeps every one-move approach on `grid` and keeps the roomiest.
#[must_use]
pub fn search(vehicle: &Vehicle, scene: &Scene, approach: Approach, grid: Grid) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let step = Discretisation::default().sample_step;

    let starts = start_poses(vehicle, scene, grid.start_x_steps, grid.lateral_steps);
    let goals = goal_poses(vehicle, scene, grid.entry_steps, grid.heading_steps);
    if starts.is_empty() || goals.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    }

    let mut best: Option<(Vec<Pose>, f64)> = None;

    for i in 0..=grid.radius_steps {
        let radius = vehicle.min_turning_radius + f64::from(i) * RADIUS_STEP_M;
        for &start in &starts {
            for &goal in &goals {
                for curve in curves_between(approach, start, goal, radius) {
                    // The path is sampled from `start`, which the curve
                    // excludes, so the starting pose is prepended by hand.
                    let mut path = vec![start];
                    path.extend(curve_poses(approach, &curve, start, goal, step));

                    // Only a strictly roomier candidate is worth walking to
                    // the end. `floor` is the best clearance so far, so most
                    // candidates die within a few poses.
                    let floor = best.as_ref().map_or(f64::NEG_INFINITY, |(_, m)| *m);
                    if let Some(margin) = evaluate_at_least(&path, &field, floor) {
                        best = Some((path, margin));
                    }
                }
            }
        }
    }

    match best {
        None => Outcome::NotFound {
            // An exhaustive sweep has no budget to exhaust.
            budget_exhausted: false,
        },
        Some((poses, min_clearance)) => {
            let direction = match approach {
                Approach::Forward => Direction::Forward,
                Approach::Reverse => Direction::Reverse,
            };
            Outcome::Found(vec![Maneuver {
                poses: poses
                    .into_iter()
                    .map(|pose| DirectedPose { pose, direction })
                    .collect(),
                min_clearance,
                moves: 1,
                confidence: Confidence::Exact,
            }])
        }
    }
}

/// The same pose, turned about.
///
/// A vehicle backing along a path covers exactly the ground a vehicle driving
/// forward along the same path in the other direction covers. Turning both
/// poses about is what turns a reverse problem into a Dubins one.
///
/// The heading is wrapped, because this is applied twice — once to state the
/// problem, once to read the answer back — and [`Radians`] does not wrap on
/// its own. Without it a reverse entry would come out a full turn off, and
/// carry that all the way to the interface, which would report a vehicle
/// finishing 361 degrees from square.
fn turned_about(pose: Pose) -> Pose {
    Pose::new(
        pose.x,
        pose.y,
        Radians::new((pose.heading.get() + PI).rem_euclid(TAU)),
    )
}

/// Every curve joining two poses for the given approach.
///
/// Forward is Dubins directly. Reverse is Dubins on the turned-about problem,
/// read from the goal back to the start — which is why the arguments are
/// swapped here and the samples reversed in [`curve_poses`].
fn curves_between(approach: Approach, start: Pose, goal: Pose, radius: f64) -> Vec<CurvePath> {
    match approach {
        Approach::Forward => dubins::all(start, goal, radius),
        Approach::Reverse => dubins::all(turned_about(goal), turned_about(start), radius),
    }
}

/// Samples a curve into the poses the vehicle actually occupies.
///
/// For a reverse approach the curve runs from goal to start with the headings
/// turned about, so the samples are turned back and read in reverse. The
/// vehicle's own pose is what the caller wants, not the direction it happens
/// to be travelling in.
fn curve_poses(
    approach: Approach,
    curve: &CurvePath,
    start: Pose,
    goal: Pose,
    step: f64,
) -> Vec<Pose> {
    match approach {
        Approach::Forward => curve.poses(start, step),
        Approach::Reverse => {
            // `CurvePath::poses` excludes its starting pose and includes its
            // ending one. Read backwards, the curve's *excluded* beginning is
            // the journey's goal — so putting it back is what makes the
            // journey land on the goal exactly, rather than one sampling step
            // short of it. Its end, conversely, is the journey's start, which
            // the caller prepends itself, so that one is dropped.
            let entry = turned_about(goal);
            let mut sampled = vec![entry];
            sampled.extend(curve.poses(entry, step));
            sampled.pop();
            sampled.reverse();
            // Turned back in place: this runs on every candidate of every
            // sweep, and collecting into a second vector cost the reverse
            // approach a third of its time for nothing.
            for pose in &mut sampled {
                *pose = turned_about(*pose);
            }
            sampled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::clearance::Clearance;
    use swept_core::scene::{GateKind, Post};

    /// A scene whose dropped kerb is wider than the opening, as a real one is:
    /// the kerb has to be dropped at least across the gateway.
    fn scene_with_opening(width: f64) -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -width / 2.0,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: width / 2.0,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: width + 0.80,
            road_width: 4.50,
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_generous_opening_admits_a_forward_entry() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(5.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome.best().expect("5 m is plenty");
        assert_eq!(best.moves, 1);
        assert!(best.min_clearance > 0.0);
    }

    #[test]
    fn every_exact_result_says_it_is_exact() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(5.0),
            Approach::Forward,
            Grid::fine(),
        );
        assert!(outcome.best().expect("a solution").is_exact());
    }

    #[test]
    fn an_opening_narrower_than_the_mirrors_admits_nothing() {
        // The LBX measures 2.029 m over its mirrors.
        let outcome = search(
            &lbx(),
            &scene_with_opening(1.6),
            Approach::Forward,
            Grid::fine(),
        );
        match outcome {
            Outcome::NotFound { budget_exhausted } => assert!(
                !budget_exhausted,
                "an exhaustive sweep never runs out of budget"
            ),
            Outcome::Found(_) => panic!("the vehicle cannot fit through 1.6 m"),
        }
    }

    #[test]
    fn a_wider_opening_is_never_tighter_than_a_narrower_one() {
        // Fails if the sweep misses candidates in a scene-dependent way.
        let vehicle = lbx();
        let narrow = search(
            &vehicle,
            &scene_with_opening(3.0),
            Approach::Forward,
            Grid::fine(),
        );
        let wide = search(
            &vehicle,
            &scene_with_opening(4.0),
            Approach::Forward,
            Grid::fine(),
        );
        if let (Some(n), Some(w)) = (narrow.best(), wide.best()) {
            assert!(
                w.min_clearance >= n.min_clearance - 1e-9,
                "3 m gave {}, 4 m gave {}",
                n.min_clearance,
                w.min_clearance
            );
        }
    }

    #[test]
    fn the_returned_path_is_actually_collision_free() {
        let (vehicle, scene) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &scene, Approach::Forward, Grid::fine());
        let best = outcome.best().expect("a solution");
        let field = ClearanceField::new(&scene, &vehicle);
        for step in &best.poses {
            assert_ne!(field.at(step.pose), Clearance::Collision);
        }
    }

    #[test]
    fn a_coarse_grid_visits_fewer_pairs_than_a_fine_one() {
        assert!(Grid::coarse().candidate_count() < Grid::fine().candidate_count());
    }

    #[test]
    fn the_fine_grid_stays_within_a_workable_number_of_pairs() {
        // ARBITRARY ceiling, and deliberately generous: this is not a
        // performance target but a tripwire. A grid that quietly grew by an
        // order of magnitude would still return correct answers, just far too
        // slowly for a worker the interface waits on — and nothing else in the
        // suite would notice.
        assert!(
            Grid::fine().candidate_count() <= 60_000,
            "the fine grid now visits {} pairs",
            Grid::fine().candidate_count()
        );
    }

    /// The measured gateway: 2.29 m clear, 1.30 m pavement, 5.90 m
    /// carriageway, and the pivot radius rather than the kerb radius.
    fn measured_gateway(open_degrees: f64) -> Scene {
        let mut sc = scene_with_opening(2.29);
        sc.pavement_width = 1.30;
        sc.road_width = 5.90;
        sc.dropped_kerb_width = 3.20;
        sc.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(open_degrees),
        };
        sc
    }

    fn lbx_pivot() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59).expect("valid vehicle")
    }

    #[test]
    fn a_narrow_opening_that_defeated_the_old_shape_now_admits_an_entry() {
        // The old fixed shape — straight, quarter turn, straight — found
        // nothing at all on this gateway, at any leaf angle, out of 7 410
        // candidates. Lifting its two gratuitous constraints is what buys the
        // entry: the turn no longer has to be exactly ninety degrees, and the
        // approach no longer has to be exactly five metres.
        let outcome = search(
            &lbx_pivot(),
            &measured_gateway(118.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome
            .best()
            .expect("Dubins finds what the fixed shape could not");
        assert_eq!(best.moves, 1);
        assert!(best.is_exact(), "an exhaustive sweep says so");
        assert!(best.min_clearance > 0.0, "got {}", best.min_clearance);
    }

    #[test]
    fn leaves_square_to_the_wall_defeat_this_gateway_whatever_the_curve() {
        // Not a shortcoming of the sweep, and worth pinning so nobody spends a
        // day widening the grid over it. A leaf held at ninety degrees makes
        // the passage a 1.70 m corridor, which tolerates almost no angle,
        // while the vehicle needs its full turning radius of clear depth
        // before its nose reaches the wall — and this street leaves 1.44 m.
        // The entry appears as soon as either constraint eases: opening the
        // leaves further, or a sliding gate.
        let vehicle = lbx_pivot();
        let square = search(
            &vehicle,
            &measured_gateway(90.0),
            Approach::Forward,
            Grid::fine(),
        );
        assert!(square.best().is_none(), "geometry, not grid coverage");

        let mut sliding = measured_gateway(90.0);
        sliding.gate = GateKind::Sliding;
        assert!(
            search(&vehicle, &sliding, Approach::Forward, Grid::fine())
                .best()
                .is_some(),
            "without the leaves the same street admits the same vehicle"
        );
    }

    #[test]
    fn the_vehicle_finishes_square_to_the_opening() {
        // Criterion 3 of the design. The old arrival test only demanded depth,
        // so the vehicle could end up askew in the yard.
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits an entry");
        let last = best.poses.last().expect("a manoeuvre has poses");
        let off_square = (last.pose.heading.get() - std::f64::consts::FRAC_PI_2).abs();
        assert!(
            off_square.to_degrees() <= 5.0 + 1e-9,
            "finished {} degrees off square",
            off_square.to_degrees()
        );
    }

    #[test]
    fn a_forward_sweep_never_reverses() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits an entry");
        for step in &best.poses {
            assert_eq!(step.direction, Direction::Forward);
        }
    }

    #[test]
    fn the_path_starts_on_the_road_and_ends_in_the_yard() {
        let (vehicle, sc) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &sc, Approach::Forward, Grid::fine());
        let best = outcome.best().expect("4 m admits an entry");
        let first = best.poses.first().expect("a manoeuvre has poses");
        let last = best.poses.last().expect("a manoeuvre has poses");
        assert!(
            first.pose.y < 0.0,
            "starts on the road, got y={}",
            first.pose.y
        );
        assert!(
            last.pose.y >= crate::path::entry_depth(&sc, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.pose.y
        );
    }

    #[test]
    fn the_coarse_grid_divides_the_fine_one_on_every_axis() {
        // This is what makes the next test a property instead of a hope. Two
        // sweeps at unrelated step counts share almost no values, so a coarse
        // grid could legitimately beat a fine one. Making each coarse count
        // divide its fine counterpart makes the coarse sweep try a strict
        // subset, bit for bit.
        let (fine, coarse) = (Grid::fine(), Grid::coarse());
        for (f, c, axis) in [
            (fine.start_x_steps, coarse.start_x_steps, "start_x"),
            (fine.lateral_steps, coarse.lateral_steps, "lateral"),
            (fine.entry_steps, coarse.entry_steps, "entry"),
            (fine.heading_steps, coarse.heading_steps, "heading"),
        ] {
            assert!(
                c > 0,
                "{axis}: a coarse count of zero cannot divide anything"
            );
            assert_eq!(f % c, 0, "{axis}: {c} does not divide {f}");
        }
        // Radii step by a fixed increment, so a smaller count is a prefix.
        assert!(coarse.radius_steps <= fine.radius_steps);
    }

    #[test]
    fn a_reverse_entry_is_driven_backwards_from_the_road_into_the_yard() {
        let (vehicle, sc) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &sc, Approach::Reverse, Grid::fine());
        let best = outcome.best().expect("4 m admits a reverse entry");
        let first = best.poses.first().expect("a manoeuvre has poses");
        let last = best.poses.last().expect("a manoeuvre has poses");

        assert!(
            first.pose.y < 0.0,
            "starts on the road, got y={}",
            first.pose.y
        );
        assert!(
            last.pose.y >= crate::path::entry_depth(&sc, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.pose.y
        );
        for step in &best.poses {
            assert_eq!(step.direction, Direction::Reverse);
        }
    }

    #[test]
    fn a_reverse_entry_also_finishes_square_to_the_opening() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Reverse,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits a reverse entry");
        let last = best.poses.last().expect("a manoeuvre has poses");
        let off_square = (last.pose.heading.get() - std::f64::consts::FRAC_PI_2).abs();
        assert!(
            off_square.to_degrees() <= 5.0 + 1e-9,
            "finished {} degrees off square",
            off_square.to_degrees()
        );
    }

    #[test]
    fn a_reverse_path_is_the_forward_path_of_the_turned_about_problem() {
        // The symmetry the whole task rests on, checked on its own rather than
        // through a sweep: backing from A to B covers the same ground as
        // driving forward from B to A with both headings turned about.
        let start = Pose::new(-6.0, -2.5, Radians::default());
        let goal = Pose::new(0.3, 5.0, Radians::new(std::f64::consts::FRAC_PI_2));
        let radius = 4.0;

        let curves = curves_between(Approach::Reverse, start, goal, radius);
        assert!(!curves.is_empty(), "some family applies here");

        for curve in &curves {
            let poses = curve_poses(Approach::Reverse, curve, start, goal, 0.05);
            let last = *poses.last().expect("a sampled path is never empty");
            assert!(
                (last.x - goal.x).abs() < 1e-6,
                "x off by {}",
                last.x - goal.x
            );
            assert!(
                (last.y - goal.y).abs() < 1e-6,
                "y off by {}",
                last.y - goal.y
            );
            let error = (last.heading.get() - goal.heading.get()).rem_euclid(2.0 * PI);
            assert!(error.min(2.0 * PI - error) < 1e-6, "heading off by {error}");
        }
    }

    #[test]
    fn a_finer_grid_never_finds_less_room_than_a_coarser_one() {
        let (vehicle, sc) = (lbx(), scene_with_opening(3.0));
        let coarse = search(&vehicle, &sc, Approach::Forward, Grid::coarse());
        let fine = search(&vehicle, &sc, Approach::Forward, Grid::fine());
        if let (Some(c), Some(f)) = (coarse.best(), fine.best()) {
            assert!(
                f.min_clearance >= c.min_clearance - 1e-9,
                "coarse gave {}, fine gave {}",
                c.min_clearance,
                f.min_clearance
            );
        }
    }
}
