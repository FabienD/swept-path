//! The move that finishes an entry.
//!
//! From any state, try to swing onto the opening's axis and push through.
//! Every candidate is checked against collisions before being returned, which
//! is why a planner result is trustworthy even though the planner itself is
//! heuristic.

use crate::budget::Discretisation;
use crate::path::{entry_depth, evaluate};
use std::f64::consts::{FRAC_PI_2, PI};
use swept_core::clearance::ClearanceField;
use swept_core::kinematics::{Direction, Pose, sample_arc};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// How many turning radii the landing tries.
///
/// ARBITRARY — carried over from the prototype (`index.html:459`).
pub const LANDING_RADIUS_COUNT: usize = 6;

/// Spacing between those radii, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:459`).
pub const LANDING_RADIUS_SPREAD_M: f64 = 1.1;

/// Longest swing allowed when lining up, in metres of arc.
///
/// Beyond this the manoeuvre stops resembling anything a driver would do.
/// ARBITRARY — carried over from the prototype (`index.html:432`).
pub const MAX_LANDING_ARC_M: f64 = 22.0;

/// A completed entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    /// The poses of the landing move.
    pub poses: Vec<Pose>,
    /// Tightest clearance along it, in metres.
    pub min_clearance: f64,
    /// Which way the vehicle drives through.
    pub direction: Direction,
}

/// The turning radii a landing will try, tightest first.
#[must_use]
pub fn landing_radii(vehicle: &Vehicle) -> Vec<f64> {
    (0..LANDING_RADIUS_COUNT)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let offset = i as f64 * LANDING_RADIUS_SPREAD_M;
            vehicle.min_turning_radius + offset
        })
        .collect()
}

/// Normalises an angle into `(-π, π]`.
fn wrap(angle: f64) -> f64 {
    let mut a = angle;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a <= -PI {
        a += 2.0 * PI;
    }
    a
}

/// Every way of finishing the entry from `from`: at most one per direction.
///
/// Returning both matters. Landing in reverse is often roomier, but it costs
/// a move when the vehicle was going forwards — so the roomiest landing is
/// not always the best one, and only the caller knows the exchange rate.
/// Picking on clearance alone here made a one-move entry unreachable.
#[must_use]
pub fn landings(
    from: Pose,
    vehicle: &Vehicle,
    scene: &Scene,
    field: &ClearanceField,
    allowed: Option<Direction>,
) -> Vec<Landing> {
    let needed = entry_depth(scene, vehicle);
    let step = Discretisation::default().sample_step;
    let mut found: Vec<Landing> = Vec::with_capacity(2);

    for direction in [Direction::Forward, Direction::Reverse] {
        let mut best: Option<Landing> = None;
        if allowed.is_some_and(|only| only != direction) {
            continue;
        }
        let target = match direction {
            Direction::Forward => FRAC_PI_2,
            Direction::Reverse => -FRAC_PI_2,
        };
        let turn = wrap(target - from.heading.get());

        for radius in landing_radii(vehicle) {
            for sign in [1.0, -1.0] {
                let mut poses = Vec::new();
                let mut at = from;

                if turn.abs() > 1e-4 {
                    let curvature = sign / radius;
                    let arc = turn / curvature;
                    // Forwards must swing forwards, reverse must swing back.
                    match direction {
                        Direction::Forward if arc <= 0.0 => continue,
                        Direction::Reverse if arc >= 0.0 => continue,
                        _ => {}
                    }
                    if arc.abs() > MAX_LANDING_ARC_M {
                        continue;
                    }
                    poses.extend(sample_arc(at, curvature, arc, step));
                    at = at.advance(curvature, arc);
                } else if direction == Direction::Reverse {
                    continue;
                }

                if at.y >= needed {
                    continue;
                }
                let push = needed - at.y;
                let signed = match direction {
                    Direction::Forward => push,
                    Direction::Reverse => -push,
                };
                poses.extend(sample_arc(at, 0.0, signed, step));

                if let Some(min_clearance) = evaluate(&poses, field)
                    && best
                        .as_ref()
                        .is_none_or(|b| min_clearance > b.min_clearance)
                {
                    best = Some(Landing {
                        poses,
                        min_clearance,
                        direction,
                    });
                }
            }
        }
        found.extend(best);
    }
    found
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
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    #[test]
    fn radii_start_at_the_tightest_the_vehicle_can_hold() {
        let radii = landing_radii(&lbx());
        assert!(!radii.is_empty());
        let tightest = radii.iter().copied().fold(f64::MAX, f64::min);
        assert!((tightest - 5.2).abs() < 1e-9, "got {tightest}");
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
    fn a_landing_ends_past_the_entry_depth() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        let landing = landings(from, &vehicle, &sc, &field, None)
            .into_iter()
            .next()
            .expect("a clear run in");
        let last = landing.poses.last().expect("a landing has poses");
        assert!(
            last.y >= entry_depth(&sc, &vehicle) - 1e-6,
            "got y={}",
            last.y
        );
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
