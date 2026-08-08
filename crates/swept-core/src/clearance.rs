//! How much room a given pose leaves.
//!
//! Two tests run, and both are needed. The forward test walks the sampled
//! points of the vehicle outline against every obstacle. The reverse test
//! walks the obstacle corners against the vehicle's body rectangle — because a
//! pillar corner can sit inside the body without any sampled point falling
//! inside the pillar, and the forward test alone would call that clear.

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
    obstacles: Vec<Obb>,
    corners: Vec<Point>,
    envelope: Vec<Point>,
    half_width: f64,
    rear: f64,
    front: f64,
}

impl ClearanceField {
    /// Prepares the field for one scene and one vehicle.
    #[must_use]
    pub fn new(scene: &Scene, vehicle: &Vehicle) -> Self {
        let obstacles = scene.obstacles();
        let corners = obstacles
            .iter()
            .filter(|o| {
                o.half_width <= CORNER_TEST_MAX_HALF_SIZE_M
                    && o.half_height <= CORNER_TEST_MAX_HALF_SIZE_M
            })
            .flat_map(Obb::corners)
            .collect();

        Self {
            obstacles,
            corners,
            envelope: vehicle.envelope(),
            half_width: vehicle.width / 2.0,
            rear: -vehicle.rear_overhang,
            front: vehicle.wheelbase + vehicle.front_overhang,
        }
    }

    /// The clearance left by one pose.
    #[must_use]
    pub fn at(&self, pose: Pose) -> Clearance {
        let (sin, cos) = pose.heading.sin_cos();

        let mut smallest = f64::MAX;
        for local in &self.envelope {
            let point = Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            );
            for obstacle in &self.obstacles {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // Reverse test: an obstacle corner inside the vehicle body.
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
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
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
