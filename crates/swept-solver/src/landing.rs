//! The last move: getting from wherever the planner is to a pose in the yard.
//!
//! Reeds-Shepp joins two **poses** in closed form, so a landing arrives
//! exactly where it was aimed rather than wherever its shape happened to end.
//! That is the same correction batch 2b made to the exhaustive search, applied
//! to the planner's last move.
//!
//! Every curve that applies is tried and the roomiest kept. Length never
//! enters it: the shortest path is the one that grazes most.

use crate::path::evaluate_at_least;
use crate::poses::goal_poses;
use swept_core::clearance::ClearanceField;
use swept_core::curves::reeds_shepp;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// How many points along the opening a landing aims at.
///
/// ARBITRARY, and deliberately smaller than the exhaustive sweep's grid: this
/// runs at every expanded node rather than once, so a pose that costs nothing
/// there costs tens of thousands of times as much here.
pub const LANDING_ENTRY_STEPS: u16 = 2;

/// Final headings tried, spread about square to the opening.
///
/// ARBITRARY, same reasoning as [`LANDING_ENTRY_STEPS`].
pub const LANDING_HEADING_STEPS: u16 = 2;

/// How many curves per goal pose are actually sampled.
///
/// **This bound is what makes analytic expansion affordable.** Reeds-Shepp
/// returns up to forty-eight words per pose pair, and the planner asks for a
/// landing at every node near the opening. Sampling them all costs nineteen
/// million point-obstacle tests per node — measured, and enough to hang the
/// test suite past ten minutes.
///
/// Sorting by length and keeping the shortest few is a **cost filter, not a
/// choice of criterion**: among those kept, the roomiest still wins. It is a
/// compromise all the same, and the honest way to state it is that a long
/// contorted curve that happened to be roomier will not be found.
///
/// ARBITRARY in magnitude. The figure to raise first if plans come back worse
/// than the batch before.
pub const LANDING_CURVES_TRIED: usize = 48;

/// Longest a landing may be, in metres.
///
/// **Without this the solver reports entries through walls it cannot pass.**
/// The scene's wall is finite — it ends at `SCENE_HALF_EXTENT_M`, eighteen
/// metres either side — and a closed-form curve is happy to drive round it. On
/// a 1.60 m opening that no vehicle fits through, the planner came back with a
/// 175 m "landing" that went round the end of the wall and reported 27 cm of
/// clearance. It was not wrong about the geometry it was given; the geometry
/// is a model, and a model has edges.
///
/// The old landing could not do this because it was an arc and a straight run,
/// short by construction. A bound restores that property without giving up the
/// closed form.
///
/// ARBITRARY in magnitude: generous enough for any manoeuvre in front of a
/// gateway, far short of the distance needed to reach the end of the wall.
pub const LANDING_MAX_LENGTH_M: f64 = 25.0;

/// Sampling step along a landing curve, in metres.
///
/// ARBITRARY — the step the rest of the solver samples at, so that a clearance
/// measured here means what it means everywhere else.
pub const LANDING_SAMPLE_STEP_M: f64 = 0.08;

/// One way of finishing the entry.
#[derive(Debug, Clone)]
pub struct Landing {
    /// The poses of the landing move.
    pub poses: Vec<Pose>,
    /// Tightest clearance along it, in metres.
    pub min_clearance: f64,
    /// Which way the vehicle drives through, at the end.
    pub direction: Direction,
    /// How many times the curve changes gear on its way in.
    ///
    /// The old landing was a single arc and a straight run, so this was always
    /// zero and the caller could count a landing as one move or none. A
    /// closed-form curve may shunt, and a plan that claims two moves while
    /// driving four is worse than useless.
    pub reversals: u8,
}

impl Landing {
    /// What this landing costs, in moves, when reached driving `arriving_in`.
    ///
    /// Every gear change inside the curve counts, plus one more if the planner
    /// has to change gear to begin it.
    #[must_use]
    pub fn moves(&self, arriving_in: Direction) -> u8 {
        self.reversals + u8::from(self.starts_in() != arriving_in)
    }

    /// The gear the landing begins in.
    ///
    /// With an even number of gear changes the curve ends as it began.
    fn starts_in(&self) -> Direction {
        if self.reversals % 2 == 0 {
            self.direction
        } else {
            match self.direction {
                Direction::Forward => Direction::Reverse,
                Direction::Reverse => Direction::Forward,
            }
        }
    }
}

/// Where a landing may finish: in the yard, square to the opening, and no
/// further off centre than the opening can hold.
///
/// [`goal_poses`] spreads its aim over ±90 cm, which is right for a wide
/// gateway and absurd for a narrow one: a 2.20 m opening holds a 2.03 m
/// vehicle with 8.5 cm to spare, so a goal 90 cm off centre is inside a post.
/// Aiming there wastes every attempt, and the planner came back empty on the
/// tightest opening it used to solve.
fn landing_goals(vehicle: &Vehicle, scene: &Scene) -> Vec<Pose> {
    let room = (scene.opening_width() - vehicle.mirror_width) / 2.0;
    if room <= 0.0 {
        return Vec::new();
    }
    let mut goals = goal_poses(vehicle, scene, LANDING_ENTRY_STEPS, LANDING_HEADING_STEPS);
    goals.retain(|g| g.x.abs() <= room);
    if goals.is_empty() {
        // Nothing within reach off centre: aim dead centre, which is where a
        // tight opening is threaded anyway.
        goals = goal_poses(vehicle, scene, 0, LANDING_HEADING_STEPS);
    }
    goals
}

/// Every collision-free landing from `from`, roomiest first per direction.
///
/// At most two are returned — one finishing forwards, one finishing in
/// reverse — because the caller files a landing under what it costs in moves,
/// and only the best of each gear can win that comparison.
///
/// `allowed` restricts the gear when the interface asked for one.
#[must_use]
pub fn landings(
    from: Pose,
    vehicle: &Vehicle,
    scene: &Scene,
    field: &ClearanceField,
    allowed: Option<Direction>,
) -> Vec<Landing> {
    let goals = landing_goals(vehicle, scene);
    let mut best: [Option<Landing>; 2] = [None, None];

    for goal in goals {
        let mut curves = reeds_shepp::all(from, goal, vehicle.min_turning_radius);
        curves.retain(|c| c.length() <= LANDING_MAX_LENGTH_M);
        curves.sort_by(|a, b| a.length().total_cmp(&b.length()));
        curves.truncate(LANDING_CURVES_TRIED);

        for curve in curves {
            // A landing's gear is the gear it finishes in: that is what the
            // driver is doing while threading the opening.
            let Some(last) = curve.segments().last() else {
                continue;
            };
            let direction = last.direction;
            if allowed.is_some_and(|only| only != direction) {
                continue;
            }

            let mut poses = vec![from];
            poses.extend(curve.poses(from, LANDING_SAMPLE_STEP_M));

            let slot = &mut best[usize::from(direction == Direction::Reverse)];
            let floor = slot
                .as_ref()
                .map_or(f64::NEG_INFINITY, |l: &Landing| l.min_clearance);
            if let Some(min_clearance) = evaluate_at_least(&poses, field, floor) {
                *slot = Some(Landing {
                    poses,
                    min_clearance,
                    direction,
                    reversals: u8::try_from(curve.reversals()).unwrap_or(u8::MAX),
                });
            }
        }
    }
    best.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::{GateKind, Post};
    use swept_core::units::Radians;

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
    fn a_landing_ends_on_a_goal_pose_exactly() {
        // The point of the batch. The old shape aimed at a depth and arrived
        // wherever its arc happened to end; a curve arrives on a pose.
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        let goals =
            crate::poses::goal_poses(&vehicle, &sc, LANDING_ENTRY_STEPS, LANDING_HEADING_STEPS);

        for landing in landings(from, &vehicle, &sc, &field, None) {
            let last = *landing.poses.last().expect("a landing has poses");
            let matched = goals.iter().any(|g| {
                (g.x - last.x).abs() < 1e-6
                    && (g.y - last.y).abs() < 1e-6
                    && (g.heading.get() - last.heading.get()).abs() < 1e-6
            });
            assert!(matched, "landed at {last:?}, which is no goal pose");
        }
    }

    #[test]
    fn a_landing_starts_where_it_was_asked_to() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        for landing in landings(from, &vehicle, &sc, &field, None) {
            let first = *landing.poses.first().expect("a landing has poses");
            assert!((first.x - from.x).abs() < 1e-9);
            assert!((first.y - from.y).abs() < 1e-9);
        }
    }

    #[test]
    fn a_landing_is_collision_free_all_the_way() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        for landing in landings(from, &vehicle, &sc, &field, None) {
            for pose in &landing.poses {
                assert_ne!(field.at(*pose), swept_core::clearance::Clearance::Collision);
            }
            assert!(landing.min_clearance > 0.0);
        }
    }

    #[test]
    fn a_landing_that_never_changes_gear_costs_nothing_extra() {
        let landing = Landing {
            poses: vec![Pose::default(); 3],
            min_clearance: 0.1,
            direction: Direction::Forward,
            reversals: 0,
        };
        assert_eq!(landing.moves(Direction::Forward), 0);
        assert_eq!(landing.moves(Direction::Reverse), 1);
    }

    #[test]
    fn a_landing_that_shunts_costs_every_shunt() {
        // The case the old landing could not produce and the new one can: a
        // curve that changes gear twice on the way in is not one move.
        let landing = Landing {
            poses: vec![Pose::default(); 3],
            min_clearance: 0.1,
            direction: Direction::Forward,
            reversals: 2,
        };
        assert_eq!(landing.moves(Direction::Forward), 2);
        assert_eq!(landing.moves(Direction::Reverse), 3);
    }

    #[test]
    fn a_vehicle_squared_up_in_the_opening_lands_forwards() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        // Already pointing into the yard, just short of the wall.
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        let landing = landings(from, &vehicle, &sc, &field, None)
            .into_iter()
            .next()
            .expect("a clear run in");
        assert_eq!(landing.direction, Direction::Forward);
        assert!(landing.min_clearance > 0.0);
    }

    #[test]
    fn restricting_the_direction_is_honoured() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        if let Some(landing) = landings(from, &vehicle, &sc, &field, Some(Direction::Reverse))
            .into_iter()
            .next()
        {
            assert_eq!(landing.direction, Direction::Reverse);
        }
    }

    #[test]
    fn a_vehicle_facing_away_from_a_narrow_opening_cannot_land() {
        let (vehicle, sc) = (lbx(), scene(2.0));
        let field = ClearanceField::new(&sc, &vehicle);
        // Pointing along the road, far off to the side.
        let from = Pose::new(-6.0, -3.0, Radians::default());
        assert!(landings(from, &vehicle, &sc, &field, None).is_empty());
    }
}
