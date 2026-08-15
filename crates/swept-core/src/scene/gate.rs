//! Swinging gate leaves, and how far they can open.
//!
//! A leaf pivots about a hinge set a little inside the post's face. That
//! offset is what lets the leaf swing past 90° without fouling the post — but
//! every centimetre of offset costs two centimetres of clear opening, one on
//! each side, so past roughly 120° the trade stops paying.

use super::{GateKind, Post, Scene};
use crate::geometry::{Obb, Point};
use crate::units::Radians;

/// Narrowest opening angle considered, in degrees.
///
/// Below this a gate is barely open and the question does not arise.
/// ARBITRARY — carried over from the prototype (`index.html:269`).
const MIN_OPEN_DEGREES: f64 = 70.0;

/// Widest opening angle considered, in degrees.
///
/// A leaf folded back flat against the wall. ARBITRARY — carried over from the
/// prototype (`index.html:270`).
const MAX_OPEN_DEGREES: f64 = 180.0;

/// Step used when searching for the widest workable angle, in degrees.
const OPEN_ANGLE_STEP_DEGREES: f64 = 1.0;

/// Past this angle, the first fouling ends the search.
///
/// Below it, a leaf may foul and then clear again as it swings; above it the
/// geometry is monotonic. ARBITRARY — carried over from the prototype
/// (`index.html:271`).
const MONOTONIC_ABOVE_DEGREES: f64 = 85.0;

/// The rectangle occupied by one leaf at rest, or `None` for a sliding gate.
fn leaf(scene: &Scene, post: &Post, side: f64) -> Option<Obb> {
    let GateKind::Swinging {
        leaf_length,
        leaf_thickness,
        hinge_offset,
        hinge_depth_ratio,
        open_angle,
    } = scene.gate
    else {
        return None;
    };

    // The hinge sits `hinge_offset` back from the post's inner face, and
    // `hinge_depth_ratio` of the way through its depth.
    let hinge = Point::new(
        post.inner_edge_x - side * hinge_offset,
        hinge_depth_ratio * post.depth,
    );

    let (sin, cos) = open_angle.sin_cos();
    let (dx, dy) = (-side * cos, sin);

    Some(Obb::new(
        Point::new(
            hinge.x + dx * leaf_length / 2.0,
            hinge.y + dy * leaf_length / 2.0,
        ),
        Radians::new(dy.atan2(dx)),
        leaf_length / 2.0,
        leaf_thickness / 2.0,
    ))
}

/// Both leaves of a swinging gate; empty for a sliding one.
pub(super) fn leaves(scene: &Scene) -> Vec<Obb> {
    [
        leaf(scene, &scene.right_post, 1.0),
        leaf(scene, &scene.left_post, -1.0),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Whether either leaf currently overlaps its own post.
pub(super) fn hits_a_post(scene: &Scene) -> bool {
    let posts = [
        (
            &scene.right_post,
            Obb::from_bounds(
                scene.right_post.inner_edge_x,
                scene.right_post.inner_edge_x + scene.right_post.width,
                0.0,
                scene.right_post.depth,
            ),
            1.0,
        ),
        (
            &scene.left_post,
            Obb::from_bounds(
                scene.left_post.inner_edge_x - scene.left_post.width,
                scene.left_post.inner_edge_x,
                0.0,
                scene.left_post.depth,
            ),
            -1.0,
        ),
    ];

    posts
        .iter()
        .any(|(post, body, side)| leaf(scene, post, *side).is_some_and(|leaf| leaf.overlaps(body)))
}

/// Searches for the widest angle the leaves can open to.
///
/// The search walks degree by degree rather than solving in closed form: the
/// leaf-versus-post test already exists, and the answer is only ever shown to
/// a degree of precision.
pub(super) fn max_open_angle(scene: &Scene) -> Radians {
    if matches!(scene.gate, GateKind::Sliding) {
        return Radians::from_degrees(MAX_OPEN_DEGREES);
    }

    let mut best = MIN_OPEN_DEGREES;
    let mut degrees = MIN_OPEN_DEGREES;
    while degrees <= MAX_OPEN_DEGREES {
        let mut probe = *scene;
        if let GateKind::Swinging {
            ref mut open_angle, ..
        } = probe.gate
        {
            *open_angle = Radians::from_degrees(degrees);
        }

        if hits_a_post(&probe) {
            if degrees > MONOTONIC_ABOVE_DEGREES {
                break;
            }
        } else {
            best = degrees;
        }
        degrees += OPEN_ANGLE_STEP_DEGREES;
    }
    Radians::from_degrees(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference scene from CLAUDE.md: 0.55 m pillars, hinge halfway
    /// through their depth, 5 cm of offset.
    fn swinging(hinge_offset: f64, hinge_depth_ratio: f64, open_degrees: f64) -> Scene {
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
            kerb_height: f64::INFINITY,
            gate: GateKind::Swinging {
                leaf_length: 1.15,
                leaf_thickness: 0.10,
                hinge_offset,
                hinge_depth_ratio,
                open_angle: Radians::from_degrees(open_degrees),
            },
        }
    }

    #[test]
    fn a_swinging_gate_contributes_two_leaves() {
        assert_eq!(leaves(&swinging(0.05, 0.5, 90.0)).len(), 2);
    }

    #[test]
    fn a_sliding_gate_contributes_none() {
        let mut scene = swinging(0.05, 0.5, 90.0);
        scene.gate = GateKind::Sliding;
        assert!(leaves(&scene).is_empty());
    }

    #[test]
    fn leaves_stand_clear_of_the_pillars_at_ninety_degrees() {
        assert!(!hits_a_post(&swinging(0.05, 0.5, 90.0)));
    }

    #[test]
    fn leaves_foul_the_pillars_when_opened_too_far() {
        assert!(hits_a_post(&swinging(0.05, 0.5, 150.0)));
    }

    /// Fourth reference result from CLAUDE.md: with the hinge halfway through
    /// a 0.55 m pillar and 5 cm of offset, the leaf clears up to about 91°;
    /// moving the hinge to the yard face buys about 118°.
    #[test]
    fn reproduces_the_reference_opening_angles() {
        let halfway = swinging(0.05, 0.5, 90.0).max_open_angle().to_degrees();
        assert!(
            (halfway - 91.0).abs() <= 1.0,
            "hinge halfway: expected about 91 degrees, got {halfway}"
        );

        let yard_face = swinging(0.05, 1.0, 90.0).max_open_angle().to_degrees();
        assert!(
            (yard_face - 118.0).abs() <= 2.0,
            "hinge on the yard face: expected about 118 degrees, got {yard_face}"
        );
    }
}
