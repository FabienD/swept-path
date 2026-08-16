//! Turns a scene into the list of rectangles a vehicle can hit.

use super::{GateKind, Scene};
use crate::geometry::Obb;

/// How far the scene extends either side of the opening, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:239`). Walls have
/// to end somewhere; 18 m is far enough that no manoeuvre reaches the edge.
pub const SCENE_HALF_EXTENT_M: f64 = 18.0;

/// Thickness given to the wall across the road, in metres.
///
/// It only has to be thicker than any vehicle is long, so that no search can
/// tunnel through it. ARBITRARY in magnitude, deliberate in intent.
pub const FAR_SIDE_THICKNESS_M: f64 = 1000.0;

/// Below this, a sidewalk is treated as absent, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:243`), where it
/// guards against a zero-height rectangle.
pub const SIDEWALK_EPSILON_M: f64 = 0.001;

/// A rectangle a vehicle can hit, and how tall it stands.
///
/// Height is what lets a body pass over a curb it would otherwise be stopped
/// by. It lives here rather than on [`Obb`] because it is a fact about the
/// scene, not about geometry: an [`Obb`] is a rectangle and should stay one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    /// Where it stands.
    pub shape: Obb,
    /// How tall it stands, in metres. Infinite for anything unclimbable.
    pub height: f64,
}

impl Obstacle {
    /// Something nothing can pass over: a wall, a post, a gate leaf.
    #[must_use]
    pub fn wall(shape: Obb) -> Self {
        Self {
            shape,
            height: f64::INFINITY,
        }
    }

    /// Something a high enough body can overhang.
    #[must_use]
    pub fn low(shape: Obb, height: f64) -> Self {
        Self { shape, height }
    }
}

/// Builds the obstacle list for a scene.
pub(super) fn build(scene: &Scene) -> Vec<Obstacle> {
    let left = scene.left_post.inner_edge_x;
    let right = scene.right_post.inner_edge_x;
    let left_outer = left - scene.left_post.width;
    let right_outer = right + scene.right_post.width;

    let mut obstacles = vec![
        // The wall either side of the posts.
        Obstacle::wall(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            left_outer,
            0.0,
            scene.wall_thickness,
        )),
        Obstacle::wall(Obb::from_bounds(
            right_outer,
            SCENE_HALF_EXTENT_M,
            0.0,
            scene.wall_thickness,
        )),
        // The posts themselves.
        Obstacle::wall(Obb::from_bounds(
            left_outer,
            left,
            0.0,
            scene.left_post.depth,
        )),
        Obstacle::wall(Obb::from_bounds(
            right,
            right_outer,
            0.0,
            scene.right_post.depth,
        )),
        // Whatever stands across the road.
        Obstacle::wall(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            SCENE_HALF_EXTENT_M,
            -(scene.sidewalk_width + scene.road_width) - FAR_SIDE_THICKNESS_M,
            -(scene.sidewalk_width + scene.road_width),
        )),
    ];

    // The sidewalk, split either side of the curb cut. The only thing in
    // a scene a vehicle can overhang.
    if scene.sidewalk_width > SIDEWALK_EPSILON_M {
        let half_curb = scene.curb_cut_width / 2.0;
        let centre = f64::midpoint(left, right);
        obstacles.push(Obstacle::low(
            Obb::from_bounds(
                -SCENE_HALF_EXTENT_M,
                centre - half_curb,
                -scene.sidewalk_width,
                0.0,
            ),
            scene.curb_height,
        ));
        obstacles.push(Obstacle::low(
            Obb::from_bounds(
                centre + half_curb,
                SCENE_HALF_EXTENT_M,
                -scene.sidewalk_width,
                0.0,
            ),
            scene.curb_height,
        ));
    }

    if matches!(scene.gate, GateKind::Swinging { .. }) {
        obstacles.extend(super::gate::leaves(scene).into_iter().map(Obstacle::wall));
    }

    obstacles
}
