//! How much carriageway a one-move entry needs.
//!
//! Answers the question a user actually asks when standing in front of their
//! gate: how much room do I need on the other side. Bisection on the road
//! width, with the exhaustive search as the predicate.

use crate::exact::{Approach, Grid, search};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// Narrowest carriageway considered, in metres.
pub const MIN_ROAD_SEARCH_LOW_M: f64 = 0.1;

/// Widest carriageway considered, in metres.
///
/// Past this, the answer is "more road will not help" — the opening itself is
/// the constraint. ARBITRARY — carried over from the prototype
/// (`index.html:588`).
pub const MIN_ROAD_SEARCH_HIGH_M: f64 = 16.0;

/// How many bisection steps are taken.
///
/// Twelve halvings of a 16 m span land within 4 mm, well under the centimetre
/// this tool reports. Carried over from the prototype (`index.html:589`).
pub const MIN_ROAD_BISECTIONS: u8 = 12;

/// The narrowest carriageway that still admits a one-move forward entry.
///
/// Returns `None` when no width up to [`MIN_ROAD_SEARCH_HIGH_M`] works, which
/// means the opening itself is blocking rather than the road.
///
/// Uses the coarse grid: the search runs a dozen times over, and the answer is
/// reported to the centimetre.
#[must_use]
pub fn minimum_road_width(vehicle: &Vehicle, scene: &Scene) -> Option<f64> {
    let mut low = MIN_ROAD_SEARCH_LOW_M;
    let mut high = MIN_ROAD_SEARCH_HIGH_M;
    let mut found = None;

    for _ in 0..MIN_ROAD_BISECTIONS {
        let middle = f64::midpoint(low, high);
        let mut probe = *scene;
        probe.road_width = middle;

        if search(vehicle, &probe, Approach::Forward, Grid::coarse())
            .best()
            .is_some()
        {
            found = Some(middle);
            high = middle;
        } else {
            low = middle;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::{GateKind, Post};

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
    fn a_generous_opening_needs_some_road_but_not_much() {
        let width = minimum_road_width(&lbx(), &scene(5.0)).expect("5 m admits an entry");
        assert!(
            (MIN_ROAD_SEARCH_LOW_M..MIN_ROAD_SEARCH_HIGH_M).contains(&width),
            "got {width}"
        );
    }

    #[test]
    fn the_answer_actually_admits_an_entry() {
        let (vehicle, base) = (lbx(), scene(4.0));
        let width = minimum_road_width(&vehicle, &base).expect("4 m admits an entry");

        let mut enough = base;
        enough.road_width = width + 0.05;
        assert!(
            search(&vehicle, &enough, Approach::Forward, Grid::coarse())
                .best()
                .is_some(),
            "the reported width plus a margin must work"
        );
    }

    #[test]
    fn a_blocked_opening_needs_no_amount_of_road() {
        // Narrower than the vehicle is wide over its mirrors.
        assert_eq!(minimum_road_width(&lbx(), &scene(1.6)), None);
    }

    #[test]
    fn a_narrower_opening_never_needs_less_road() {
        let vehicle = lbx();
        let wide = minimum_road_width(&vehicle, &scene(5.0));
        let narrow = minimum_road_width(&vehicle, &scene(3.5));
        if let (Some(w), Some(n)) = (wide, narrow) {
            assert!(n >= w - 0.05, "5 m needed {w}, 3.5 m needed {n}");
        }
    }
}
