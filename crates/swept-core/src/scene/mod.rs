//! The scene the vehicle has to get through.
//!
//! A road, a pavement broken by a dropped kerb, a wall pierced by an opening
//! between two posts, and a free yard beyond. Everything is expressed in the
//! frame described at the crate root.
//!
//! Unlike the prototype, the two posts are placed independently: nothing here
//! assumes the opening is centred on `x = 0`.

pub mod gate;
pub mod obstacles;

use crate::geometry::Obb;
use crate::units::Radians;

/// One side of the opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Post {
    /// Where the post's inner face sits, along `x`. Negative on the left of
    /// the opening, positive on the right.
    pub inner_edge_x: f64,
    /// How wide the post is, along `x`, in metres.
    pub width: f64,
    /// How deep the post is, along `y`, in metres.
    pub depth: f64,
}

/// What closes the opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateKind {
    /// A sliding gate: it retracts alongside the wall and obstructs nothing.
    /// The usable corridor is then the depth of the posts alone.
    Sliding,
    /// A pair of swinging leaves, which stand in the opening once open.
    Swinging {
        /// Length of one leaf, in metres.
        leaf_length: f64,
        /// Thickness of a leaf, in metres.
        leaf_thickness: f64,
        /// Gap between the hinge axis and the post's inner face, in metres.
        /// Every centimetre here costs two centimetres of clear opening.
        hinge_offset: f64,
        /// Where the hinge sits through the post's depth, from `0.0` at the
        /// road face to `1.0` at the yard face.
        hinge_depth_ratio: f64,
        /// How far the leaves are opened.
        open_angle: Radians,
    },
}

/// A complete scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scene {
    /// The post on the negative `x` side.
    pub left_post: Post,
    /// The post on the positive `x` side.
    pub right_post: Post,
    /// Thickness of the wall running away from the posts, in metres.
    pub wall_thickness: f64,
    /// Width of the pavement between road and wall, in metres. Zero means no
    /// pavement.
    pub pavement_width: f64,
    /// Width of the dropped kerb across the pavement, in metres.
    pub dropped_kerb_width: f64,
    /// Width of the carriageway available to manoeuvre in, in metres.
    pub road_width: f64,
    /// What closes the opening.
    pub gate: GateKind,
}

impl Scene {
    /// Clear width between the two posts, in metres.
    ///
    /// ```
    /// use swept_core::scene::{GateKind, Post, Scene};
    ///
    /// let scene = Scene {
    ///     left_post: Post { inner_edge_x: -1.2, width: 0.55, depth: 0.55 },
    ///     right_post: Post { inner_edge_x: 1.2, width: 0.55, depth: 0.55 },
    ///     wall_thickness: 0.3,
    ///     pavement_width: 1.2,
    ///     dropped_kerb_width: 3.2,
    ///     road_width: 4.5,
    ///     gate: GateKind::Sliding,
    /// };
    /// assert!((scene.opening_width() - 2.4).abs() < 1e-12);
    /// ```
    #[must_use]
    pub fn opening_width(&self) -> f64 {
        self.right_post.inner_edge_x - self.left_post.inner_edge_x
    }

    /// Every obstacle in the scene, as oriented rectangles.
    #[must_use]
    pub fn obstacles(&self) -> Vec<Obb> {
        obstacles::build(self)
    }

    /// The widest angle these leaves can open to without fouling their posts.
    ///
    /// A sliding gate is unconstrained, so it reports the maximum.
    #[must_use]
    pub fn max_open_angle(&self) -> Radians {
        gate::max_open_angle(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    /// The prototype's default scene: a 2.40 m opening between two 0.55 m
    /// pillars, symmetric about the origin.
    fn symmetric() -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -1.20,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 1.20,
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

    #[test]
    fn measures_the_opening_between_the_posts() {
        assert!((symmetric().opening_width() - 2.40).abs() < EPS);
    }

    #[test]
    fn measures_an_off_centre_opening() {
        let mut scene = symmetric();
        scene.left_post.inner_edge_x = -0.80;
        assert!((scene.opening_width() - 2.00).abs() < EPS);
    }

    #[test]
    fn builds_the_expected_obstacles_for_a_sliding_gate() {
        // Two wall stretches, two pillars, the far kerb, and the pavement
        // split either side of the dropped kerb: seven rectangles.
        assert_eq!(symmetric().obstacles().len(), 7);
    }

    #[test]
    fn adds_two_leaves_for_a_swinging_gate() {
        let mut scene = symmetric();
        scene.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.10,
            hinge_offset: 0.05,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        };
        // Seven for a sliding gate, plus the two leaves.
        assert_eq!(scene.obstacles().len(), 9);
    }

    #[test]
    fn omits_the_pavement_when_there_is_none() {
        let mut scene = symmetric();
        scene.pavement_width = 0.0;
        assert_eq!(scene.obstacles().len(), 5);
    }

    #[test]
    fn places_the_pillars_against_the_opening() {
        let scene = symmetric();
        let obstacles = scene.obstacles();
        // The right pillar spans from the opening edge outwards by its width.
        let right = obstacles
            .iter()
            .find(|o| (o.center.x - (1.20 + 0.55 / 2.0)).abs() < EPS)
            .expect("right pillar");
        assert!((right.half_width - 0.55 / 2.0).abs() < EPS);
        assert!((right.half_height - 0.55 / 2.0).abs() < EPS);
    }
}
