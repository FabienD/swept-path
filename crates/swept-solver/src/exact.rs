//! Exhaustive search for a one-move entry.
//!
//! Every combination on the grid is tried and the roomiest is kept. Because
//! the sweep is complete, a failure here means something: there is no one-move
//! entry *on this grid*. That is what makes this solver the reference the
//! planner is seeded from.

use crate::budget::Discretisation;
use crate::path::{evaluate, forward_path, reverse_path};
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use swept_core::clearance::ClearanceField;
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

/// How far either side of the opening centre the turn is aimed, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:372`).
pub const ENTRY_SPAN_M: f64 = 0.9;

/// Clearance kept between the vehicle's widest point and the lane edges when
/// choosing where the run-up is driven, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:362`).
pub const LANE_MARGIN_M: f64 = 0.02;

/// Increment between the approach angles tried when reversing, in degrees.
///
/// ARBITRARY — carried over from the prototype (`index.html:382`).
pub const REVERSE_ANGLE_STEP_DEGREES: f64 = 8.0;

/// Increment between the setbacks tried when reversing, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:384`).
pub const REVERSE_SETBACK_STEP_M: f64 = 0.5;

/// How many values of each parameter the sweep tries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Turning radii, from the vehicle's tightest upwards.
    pub radius_steps: u16,
    /// Lateral positions across the carriageway.
    pub lateral_steps: u16,
    /// Entry points along the opening.
    pub entry_steps: u16,
    /// Approach angles, reversing only.
    pub angle_steps: u16,
    /// Setbacks before the turn, reversing only.
    pub setback_steps: u16,
}

impl Grid {
    /// The full sweep, used whenever the answer is shown to a user.
    #[must_use]
    pub fn fine() -> Self {
        Self {
            radius_steps: 12,
            lateral_steps: 18,
            entry_steps: 29,
            angle_steps: 5,
            setback_steps: 4,
        }
    }

    /// A cheaper sweep, for callers that run the search many times over —
    /// the carriageway bisection in particular.
    #[must_use]
    pub fn coarse() -> Self {
        Self {
            radius_steps: 8,
            lateral_steps: 12,
            entry_steps: 17,
            angle_steps: 5,
            setback_steps: 4,
        }
    }

    /// How many candidates this grid produces, useful for reporting cost.
    #[must_use]
    pub fn candidate_count(self, approach: Approach) -> u64 {
        let base = u64::from(self.radius_steps + 1) * u64::from(self.entry_steps + 1);
        match approach {
            Approach::Forward => base * u64::from(self.lateral_steps + 1),
            Approach::Reverse => {
                base * u64::from(self.angle_steps + 1) * u64::from(self.setback_steps + 1)
            }
        }
    }
}

/// Keeps the roomiest candidate seen so far.
fn consider(best: &mut Option<(Vec<Pose>, f64)>, poses: Vec<Pose>, field: &ClearanceField) {
    if let Some(margin) = evaluate(&poses, field)
        && best.as_ref().is_none_or(|(_, m)| margin > *m)
    {
        *best = Some((poses, margin));
    }
}

/// Sweeps every one-move approach on `grid` and keeps the roomiest.
#[must_use]
pub fn search(vehicle: &Vehicle, scene: &Scene, approach: Approach, grid: Grid) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let step = Discretisation::default().sample_step;
    let half_width = vehicle.mirror_width / 2.0;

    // Where along the carriageway the run-up may be driven.
    let low = -scene.pavement_width - scene.road_width + half_width + LANE_MARGIN_M;
    let high = -half_width - LANE_MARGIN_M;
    if low > high {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    }

    let mut best: Option<(Vec<Pose>, f64)> = None;

    for i in 0..=grid.radius_steps {
        let radius = vehicle.min_turning_radius + f64::from(i) * RADIUS_STEP_M;
        for k in 0..=grid.entry_steps {
            let entry_x =
                -ENTRY_SPAN_M + 2.0 * ENTRY_SPAN_M * f64::from(k) / f64::from(grid.entry_steps);
            match approach {
                Approach::Forward => {
                    for j in 0..=grid.lateral_steps {
                        let lateral =
                            low + (high - low) * f64::from(j) / f64::from(grid.lateral_steps);
                        let poses = forward_path(vehicle, scene, radius, lateral, entry_x, step);
                        consider(&mut best, poses, &field);
                    }
                }
                Approach::Reverse => {
                    for a in 0..=grid.angle_steps {
                        let angle =
                            Radians::from_degrees(f64::from(a) * REVERSE_ANGLE_STEP_DEGREES);
                        for d in 0..=grid.setback_steps {
                            let setback = f64::from(d) * REVERSE_SETBACK_STEP_M;
                            if let Some(poses) =
                                reverse_path(vehicle, scene, radius, entry_x, setback, angle, step)
                            {
                                consider(&mut best, poses, &field);
                            }
                        }
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
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
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
    fn a_coarse_grid_visits_fewer_candidates_than_a_fine_one() {
        assert!(
            Grid::coarse().candidate_count(Approach::Forward)
                < Grid::fine().candidate_count(Approach::Forward)
        );
    }
}
