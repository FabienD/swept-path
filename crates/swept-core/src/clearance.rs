//! How much room a given pose leaves.
//!
//! Three tests run, and each exists for a case the others miss. The body walks
//! the sampled outline against what it cannot pass over. The wheels walk four
//! contact points against everything, kerbs included — a body may overhang a
//! kerb, a tyre may not leave what it can roll on. The reverse test walks
//! obstacle corners against the body rectangle, because a pillar corner can
//! sit inside the body without any sampled point falling inside the pillar.
//!
//! # Heights are compared once
//!
//! [`ClearanceField::at`] is the hot path of the whole project — a fine sweep
//! calls it hundreds of thousands of times. So no height is ever compared
//! there. They are compared once, here, when the field is built, and the
//! obstacles filed into two disjoint lists: what the body hits, and what it
//! flies over.

use crate::geometry::{Obb, Point, PointDistance};
use crate::kinematics::Pose;
use crate::scene::Scene;
use crate::vehicle::Vehicle;

/// How much room a pose leaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clearance {
    /// The vehicle is touching something.
    Collision,
    /// Smallest distance from the vehicle outline to any obstacle, in metres.
    Clear(f64),
}

/// Above this half-size, an obstacle's corners are ignored by the reverse
/// test, in metres.
///
/// The scene's outer walls and the far side of the road are modelled as very
/// large rectangles whose corners sit far outside the scene; feeding them to
/// the reverse test would only waste work. ARBITRARY in magnitude — carried
/// over from the prototype (`index.html:285`).
pub const CORNER_TEST_MAX_HALF_SIZE_M: f64 = 12.0;

/// Everything about a scene and a vehicle that does not depend on the pose.
///
/// A single search evaluates hundreds of thousands of poses against one
/// unchanging scene, so the obstacle list, the retained corners and the
/// sampled outline are all built once.
#[derive(Debug, Clone)]
pub struct ClearanceField {
    /// What the body hits: taller than the vehicle's ground clearance.
    blocking: Vec<Obb>,
    /// What the body flies over, and only the wheels hit.
    overhung: Vec<Obb>,
    corners: Vec<Point>,
    envelope: Vec<Point>,
    wheels: [Point; 4],
    half_width: f64,
    rear: f64,
    front: f64,
}

impl ClearanceField {
    /// Prepares the field for one scene and one vehicle.
    #[must_use]
    pub fn new(scene: &Scene, vehicle: &Vehicle) -> Self {
        // Strictly taller blocks: a kerb exactly at the ground clearance is
        // overhung. See the boundary test in this module.
        let (blocking, overhung): (Vec<_>, Vec<_>) = scene
            .obstacles()
            .into_iter()
            .partition(|o| o.height > vehicle.ground_clearance);
        let blocking: Vec<Obb> = blocking.into_iter().map(|o| o.shape).collect();
        let overhung: Vec<Obb> = overhung.into_iter().map(|o| o.shape).collect();

        // Only blocking obstacles need the corner test: a kerb corner inside
        // the body is an overhang, not a collision.
        let corners = blocking
            .iter()
            .filter(|o| {
                o.half_width <= CORNER_TEST_MAX_HALF_SIZE_M
                    && o.half_height <= CORNER_TEST_MAX_HALF_SIZE_M
            })
            .flat_map(Obb::corners)
            .collect();

        Self {
            blocking,
            overhung,
            corners,
            envelope: vehicle.envelope(),
            wheels: vehicle.wheels(),
            half_width: vehicle.width / 2.0,
            rear: -vehicle.rear_overhang,
            front: vehicle.wheelbase + vehicle.front_overhang,
        }
    }

    /// The clearance left by one pose.
    #[must_use]
    pub fn at(&self, pose: Pose) -> Clearance {
        let (sin, cos) = pose.heading.sin_cos();
        let place = |local: &Point| {
            Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            )
        };

        let mut smallest = f64::MAX;

        // The body, against what it cannot pass over. An overhung obstacle is
        // ignored outright — neither collision nor distance.
        for local in &self.envelope {
            let point = place(local);
            for obstacle in &self.blocking {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // The wheels, against everything.
        for local in &self.wheels {
            let point = place(local);
            for obstacle in self.blocking.iter().chain(&self.overhung) {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // Reverse test: a blocking obstacle's corner inside the vehicle body.
        for corner in &self.corners {
            let (dx, dy) = (corner.x - pose.x, corner.y - pose.y);
            let local_x = dx * cos + dy * sin;
            let local_y = -dx * sin + dy * cos;
            if local_x > self.rear
                && local_x < self.front
                && local_y > -self.half_width
                && local_y < self.half_width
            {
                return Clearance::Collision;
            }
        }

        Clearance::Clear(smallest)
    }

    /// Does any part of the body sit over an obstacle it is passing above?
    ///
    /// Reported rather than penalised. A bumper crossing a pavement is legal
    /// geometry and worth knowing about all the same: the model is flat, and
    /// knows nothing of the bollard, sign or post that so often stands there.
    ///
    /// Measured on a finished trajectory, never inside a search — like the
    /// alert distances, and for the same reason.
    #[must_use]
    pub fn overhangs(&self, pose: Pose) -> bool {
        if self.overhung.is_empty() {
            return false;
        }
        let (sin, cos) = pose.heading.sin_cos();
        self.envelope.iter().any(|local| {
            let point = Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            );
            self.overhung
                .iter()
                .any(|o| matches!(o.distance_to(point), PointDistance::Inside))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{GateKind, Post};
    use crate::units::Radians;

    fn wide_scene() -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -2.50,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 2.50,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    /// A scene with a pavement the body can pass over.
    fn low_kerb_scene() -> Scene {
        let mut scene = wide_scene();
        scene.kerb_height = 0.12;
        scene
    }

    /// A pose with the nose over the pavement and every wheel off it.
    ///
    /// **Only an overhang can overhang.** The wheels sit at the corners of the
    /// body, so the flank is over a kerb exactly when a tyre is — which is
    /// physically right, a wheel arch following its tyre to within a couple of
    /// centimetres. What can pass over a kerb is therefore what sticks out
    /// beyond an axle: the front overhang, or the rear.
    ///
    /// Here the vehicle points into the yard on `wide_scene`, whose pavement
    /// runs from y = -1.20 to 0 and whose carriageway runs from -5.70 to
    /// -1.20. Rear axle at -4.20, front axle at -1.62 — both on the
    /// carriageway — and the nose 3.43 m ahead of the rear axle, reaching
    /// -0.77, which is over the pavement. `x = -6` is clear of the dropped
    /// kerb, which spans -1.60 to 1.60.
    fn overhanging_pose() -> Pose {
        Pose::new(-6.0, -4.2, Radians::from_degrees(90.0))
    }

    #[test]
    fn a_kerb_lower_than_the_ground_clearance_does_not_stop_the_body() {
        let vehicle = lbx();
        let over = ClearanceField::new(&low_kerb_scene(), &vehicle);
        assert_ne!(
            over.at(overhanging_pose()),
            Clearance::Collision,
            "a 12 cm kerb passes under an 18 cm ground clearance"
        );
    }

    #[test]
    fn the_same_pose_is_a_collision_when_the_kerb_is_a_wall() {
        // The other half of the previous test: without the height, this is
        // exactly the refusal the batch exists to remove.
        let field = ClearanceField::new(&wide_scene(), &lbx());
        assert_eq!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn a_low_kerb_still_stops_a_wheel() {
        // Straddling the kerb line, along the road. The near-side wheels land
        // at y = -0.09, up on the pavement, while the off-side pair stay at
        // -1.91 on the carriageway. The body may fly over a kerb; a tyre may
        // not leave what it can roll on, and that alone must refuse this.
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        let pose = Pose::new(-6.0, -1.0, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn a_wall_taller_than_the_ground_clearance_stops_everything() {
        let mut scene = wide_scene();
        scene.kerb_height = 0.40;
        let field = ClearanceField::new(&scene, &lbx());
        assert_eq!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn a_kerb_exactly_at_the_ground_clearance_is_overhung() {
        // The boundary, pinned deliberately: blocking is `height > clearance`,
        // so equality passes. A model that hesitated on the millimetre would
        // serve nobody.
        let mut scene = wide_scene();
        scene.kerb_height = 0.18;
        let field = ClearanceField::new(&scene, &lbx());
        assert_ne!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn a_pose_over_the_pavement_is_reported_as_overhanging() {
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        assert!(field.overhangs(overhanging_pose()));
    }

    #[test]
    fn a_pose_out_on_the_road_overhangs_nothing() {
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        assert!(!field.overhangs(Pose::new(-6.0, -3.5, Radians::default())));
    }

    #[test]
    fn nothing_overhangs_a_scene_whose_kerb_is_a_wall() {
        // With no overhung obstacle there is nothing to overhang, so the
        // reference tests have no new quantity to account for.
        let field = ClearanceField::new(&wide_scene(), &lbx());
        assert!(!field.overhangs(overhanging_pose()));
    }

    #[test]
    fn an_overhung_obstacle_contributes_no_distance() {
        // The subtle half of the rule. If a kerb the body flies over still
        // counted towards the margin, the margin would collapse to zero the
        // moment a bumper crossed the line — which is the very refusal this
        // batch removes, wearing a different mask.
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        match field.at(overhanging_pose()) {
            Clearance::Clear(margin) => assert!(margin > 0.05, "got {margin}"),
            Clearance::Collision => panic!("the body overhangs this kerb"),
        }
    }

    #[test]
    fn reports_clearance_in_the_open_yard() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        // Well past the wall, pointing into the yard.
        let pose = Pose::new(0.0, 6.0, Radians::from_degrees(90.0));
        match field.at(pose) {
            Clearance::Clear(margin) => assert!(margin > 1.0, "got {margin}"),
            Clearance::Collision => panic!("the yard is empty"),
        }
    }

    #[test]
    fn reports_a_collision_through_the_wall() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        // Straddling the wall, across the opening rather than through it.
        let pose = Pose::new(6.0, 0.15, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn catches_an_obstacle_corner_inside_the_body() {
        // The case the reverse test exists for: the body swallows a corner
        // whole, while every sampled point sits in clear air.
        //
        // The vehicle lies broadside at y = 0.275, so its flanks pass at
        // y = 1.1875 and y = -0.6375 — above every obstacle and short of the
        // pavement. Its bumper centres pass at y = 0.275, clear of a wall only
        // 0.10 m thick. Yet the body spans x from -0.26 to 3.93, which
        // encloses the whole right-hand post.
        let scene = Scene {
            left_post: Post {
                inner_edge_x: -0.30,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 0.30,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.10,
            pavement_width: 1.20,
            dropped_kerb_width: 12.0,
            road_width: 4.50,
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        };
        let field = ClearanceField::new(&scene, &lbx());
        let pose = Pose::new(0.5, 0.275, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn margin_shrinks_as_the_vehicle_approaches_a_post() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        let far = Pose::new(0.0, 6.0, Radians::from_degrees(90.0));
        let near = Pose::new(1.2, 6.0, Radians::from_degrees(90.0));
        match (field.at(far), field.at(near)) {
            (Clearance::Clear(a), Clearance::Clear(b)) => {
                assert!(b < a, "{b} should be under {a}");
            }
            _ => panic!("both poses are clear of the obstacles"),
        }
    }
}
