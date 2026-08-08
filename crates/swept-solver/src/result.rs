//! What a search returns, and how much it can be trusted.

use swept_core::kinematics::{Direction, Pose};

/// How much a result can be trusted.
///
/// The `CLAUDE.md` rule is that no clearance is ever shown without saying
/// where it came from. Carrying it in the type makes that impossible to
/// forget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Produced by an exhaustive sweep of a grid. A failure proves there is
    /// no solution *on that grid*.
    Exact,
    /// Produced by a heuristic search. A success is verified against
    /// collisions; a failure proves nothing.
    Heuristic {
        /// Whether the search stopped because it ran out of budget rather
        /// than because it ran out of states to visit.
        budget_exhausted: bool,
    },
}

/// A pose, plus the direction the vehicle is travelling in to reach it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DirectedPose {
    /// Where the rear axle is.
    pub pose: Pose,
    /// Which way the vehicle is moving.
    pub direction: Direction,
}

/// One way of getting in.
#[derive(Debug, Clone, PartialEq)]
pub struct Maneuver {
    /// The path, sampled from the start of the approach to the final pose.
    pub poses: Vec<DirectedPose>,
    /// Smallest clearance anywhere along the path, in metres.
    pub min_clearance: f64,
    /// How many times the vehicle changes direction, counted as moves.
    pub moves: u8,
    /// Where this result came from.
    pub confidence: Confidence,
}

impl Maneuver {
    /// Whether this came from an exhaustive sweep.
    #[must_use]
    pub fn is_exact(&self) -> bool {
        self.confidence == Confidence::Exact
    }
}

/// The outcome of a search.
///
/// Finding nothing is a result, not an error: it is exactly the distinction
/// the interface has to preserve between *no solution was found* and *no
/// solution exists*.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// At least one way in, ordered by increasing number of moves.
    Found(Vec<Maneuver>),
    /// No way in was found.
    NotFound {
        /// Whether the search ran out of budget. When false, the search space
        /// was exhausted.
        budget_exhausted: bool,
    },
}

impl Outcome {
    /// The manoeuvre with the fewest moves, if any.
    #[must_use]
    pub fn best(&self) -> Option<&Maneuver> {
        match self {
            Self::Found(list) => list.iter().min_by_key(|m| m.moves),
            Self::NotFound { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::units::Radians;

    fn maneuver(moves: u8, min_clearance: f64, confidence: Confidence) -> Maneuver {
        Maneuver {
            poses: vec![DirectedPose {
                pose: Pose::new(0.0, 0.0, Radians::default()),
                direction: Direction::Forward,
            }],
            min_clearance,
            moves,
            confidence,
        }
    }

    #[test]
    fn an_exact_manoeuvre_says_so() {
        assert!(maneuver(1, 0.3, Confidence::Exact).is_exact());
        assert!(
            !maneuver(
                3,
                0.3,
                Confidence::Heuristic {
                    budget_exhausted: false
                }
            )
            .is_exact()
        );
    }

    #[test]
    fn the_best_outcome_is_the_one_with_the_fewest_moves() {
        let found = Outcome::Found(vec![
            maneuver(1, 0.10, Confidence::Exact),
            maneuver(
                3,
                0.40,
                Confidence::Heuristic {
                    budget_exhausted: false,
                },
            ),
        ]);
        // Fewer moves wins even when a longer manoeuvre has more room: the
        // caller ranks the alternatives, this only names the default.
        assert_eq!(found.best().map(|m| m.moves), Some(1));
    }

    #[test]
    fn a_failed_search_has_no_best() {
        let none = Outcome::NotFound {
            budget_exhausted: true,
        };
        assert!(none.best().is_none());
    }
}
