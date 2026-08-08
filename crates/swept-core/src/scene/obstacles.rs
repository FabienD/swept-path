//! Turns a scene into the list of rectangles a vehicle can hit.

use super::Scene;
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

/// Below this, a pavement is treated as absent, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:243`), where it
/// guards against a zero-height rectangle.
pub const PAVEMENT_EPSILON_M: f64 = 0.001;

/// Builds the obstacle list for a scene.
pub(super) fn build(scene: &Scene) -> Vec<Obb> {
    let left = scene.left_post.inner_edge_x;
    let right = scene.right_post.inner_edge_x;
    let left_outer = left - scene.left_post.width;
    let right_outer = right + scene.right_post.width;

    let mut obstacles = vec![
        // The wall either side of the posts.
        Obb::from_bounds(-SCENE_HALF_EXTENT_M, left_outer, 0.0, scene.wall_thickness),
        Obb::from_bounds(right_outer, SCENE_HALF_EXTENT_M, 0.0, scene.wall_thickness),
        // The posts themselves.
        Obb::from_bounds(left_outer, left, 0.0, scene.left_post.depth),
        Obb::from_bounds(right, right_outer, 0.0, scene.right_post.depth),
        // Whatever stands across the road.
        Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            SCENE_HALF_EXTENT_M,
            -(scene.pavement_width + scene.road_width) - FAR_SIDE_THICKNESS_M,
            -(scene.pavement_width + scene.road_width),
        ),
    ];

    // The pavement, split either side of the dropped kerb.
    if scene.pavement_width > PAVEMENT_EPSILON_M {
        let half_kerb = scene.dropped_kerb_width / 2.0;
        let centre = f64::midpoint(left, right);
        obstacles.push(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            centre - half_kerb,
            -scene.pavement_width,
            0.0,
        ));
        obstacles.push(Obb::from_bounds(
            centre + half_kerb,
            SCENE_HALF_EXTENT_M,
            -scene.pavement_width,
            0.0,
        ));
    }

    obstacles
}
