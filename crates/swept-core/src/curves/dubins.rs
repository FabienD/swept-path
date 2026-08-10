//! The six Dubins families.
//!
//! Dubins (1957) proved that the shortest forward-only path between two poses
//! at bounded curvature is always one of six words: four of the form
//! arc-straight-arc (`LSL`, `RSR`, `LSR`, `RSL`) and two of the form
//! arc-arc-arc (`RLR`, `LRL`). Each has a closed form — no search, no
//! iteration.
//!
//! # The normalised frame
//!
//! Every formula below is written in a frame that removes the rigid motion:
//! the start sits at the origin pointing along `+x`, and lengths are divided
//! by the turning radius. What is left is three numbers — the normalised
//! distance `d`, and the two headings `α` and `β` measured against the line of
//! sight. Two problems with the same triple have the same solution up to that
//! rigid motion, which is why the closed forms can exist at all.
//!
//! # On the formulas
//!
//! They are taken from Shkel & Lumelsky, *Classification of the Dubins set*
//! (2001), and cross-checked against `LaValle`, *Planning Algorithms* §13.3.3.
//! They are dense and they transcribe badly; published versions disagree on
//! `LSR` and `LRL`. Every family is therefore tested by integrating its result
//! through [`Pose::advance`] and checking it lands on the goal. That test
//! depends on no source and settles any disagreement.

use super::{CurvePath, Segment, Steering};
use crate::kinematics::{Direction, Pose};
use std::f64::consts::TAU;

/// Wraps an angle into `[0, 2π)`.
///
/// The closed forms subtract angles freely and rely on the result being taken
/// modulo a full turn; a negative arc length would otherwise be read as a
/// reverse segment, which Dubins never produces.
#[must_use]
pub fn mod_2pi(angle: f64) -> f64 {
    let wrapped = angle % TAU;
    if wrapped < 0.0 {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// The problem stripped of its rigid motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Distance between the poses, divided by the turning radius.
    pub d: f64,
    /// Start heading, measured from the line of sight, in `[0, 2π)`.
    pub alpha: f64,
    /// Goal heading, measured from the same line, in `[0, 2π)`.
    pub beta: f64,
}

impl Frame {
    /// Normalises a start and goal pose against a turning radius.
    ///
    /// Returns `None` if the radius is not a usable positive length — the
    /// caller has a vehicle that cannot turn, and no family applies.
    #[must_use]
    pub fn between(from: Pose, to: Pose, radius: f64) -> Option<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let separation = dx.hypot(dy);
        if !separation.is_finite() {
            return None;
        }
        let line_of_sight = dy.atan2(dx);
        Some(Self {
            d: separation / radius,
            alpha: mod_2pi(from.heading.get() - line_of_sight),
            beta: mod_2pi(to.heading.get() - line_of_sight),
        })
    }
}

/// One of the six Dubins words.
///
/// `L` and `R` are arcs at the minimum radius, `S` a straight run. The letters
/// read in order of travel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Word {
    /// Left, straight, left.
    Lsl,
    /// Right, straight, right.
    Rsr,
    /// Left, straight, right.
    Lsr,
    /// Right, straight, left.
    Rsl,
    /// Right, left, right.
    Rlr,
    /// Left, right, left.
    Lrl,
}

impl Word {
    /// Every word, in a fixed order so that results are reproducible.
    pub const ALL: [Self; 6] = [
        Self::Lsl,
        Self::Rsr,
        Self::Lsr,
        Self::Rsl,
        Self::Rlr,
        Self::Lrl,
    ];

    /// The steering held over each of the three pieces.
    #[must_use]
    pub const fn steerings(self) -> [Steering; 3] {
        use Steering::{Left, Right, Straight};
        match self {
            Self::Lsl => [Left, Straight, Left],
            Self::Rsr => [Right, Straight, Right],
            Self::Lsr => [Left, Straight, Right],
            Self::Rsl => [Right, Straight, Left],
            Self::Rlr => [Right, Left, Right],
            Self::Lrl => [Left, Right, Left],
        }
    }
}

/// Solves one family in the normalised frame.
///
/// Returns the three normalised lengths — radians for the arcs, radii for the
/// straight run — or `None` when the family does not apply to this frame.
/// A family failing is ordinary: `LSR` needs the two circles far enough apart
/// to admit a common tangent, and `RLR` needs them close enough to admit a
/// third circle touching both.
#[must_use]
pub fn solve(word: Word, frame: Frame) -> Option<[f64; 3]> {
    let Frame { d, alpha, beta } = frame;
    let (sin_a, cos_a) = alpha.sin_cos();
    let (sin_b, cos_b) = beta.sin_cos();
    // The cosine of the angle between the two headings, which every family
    // needs and which is cheaper to take once than four times.
    let cos_turn = (alpha - beta).cos();

    match word {
        Word::Lsl => {
            let squared = 2.0 + d * d - 2.0 * cos_turn + 2.0 * d * (sin_a - sin_b);
            if squared < 0.0 {
                return None;
            }
            let tangent = (cos_b - cos_a).atan2(d + sin_a - sin_b);
            Some([
                mod_2pi(tangent - alpha),
                squared.sqrt(),
                mod_2pi(beta - tangent),
            ])
        }
        Word::Rsr => {
            let squared = 2.0 + d * d - 2.0 * cos_turn + 2.0 * d * (sin_b - sin_a);
            if squared < 0.0 {
                return None;
            }
            let tangent = (cos_a - cos_b).atan2(d - sin_a + sin_b);
            Some([
                mod_2pi(alpha - tangent),
                squared.sqrt(),
                mod_2pi(tangent - beta),
            ])
        }
        Word::Lsr => {
            let squared = -2.0 + d * d + 2.0 * cos_turn + 2.0 * d * (sin_a + sin_b);
            if squared < 0.0 {
                return None;
            }
            let straight = squared.sqrt();
            let tangent = (-cos_a - cos_b).atan2(d + sin_a + sin_b) - (-2.0f64).atan2(straight);
            Some([mod_2pi(tangent - alpha), straight, mod_2pi(tangent - beta)])
        }
        Word::Rsl => {
            let squared = -2.0 + d * d + 2.0 * cos_turn - 2.0 * d * (sin_a + sin_b);
            if squared < 0.0 {
                return None;
            }
            let straight = squared.sqrt();
            let tangent = (cos_a + cos_b).atan2(d - sin_a - sin_b) - 2.0f64.atan2(straight);
            Some([mod_2pi(alpha - tangent), straight, mod_2pi(beta - tangent)])
        }
        Word::Rlr => {
            // The middle arc's half-angle comes from the law of cosines on the
            // triangle joining the three circle centres. Outside [-1, 1] the
            // triangle does not close: no third circle touches both.
            let cosine = (6.0 - d * d + 2.0 * cos_turn + 2.0 * d * (sin_a - sin_b)) / 8.0;
            if cosine.abs() > 1.0 {
                return None;
            }
            let middle = mod_2pi(TAU - cosine.acos());
            let first = mod_2pi(alpha - (cos_a - cos_b).atan2(d - sin_a + sin_b) + middle / 2.0);
            Some([first, middle, mod_2pi(alpha - beta - first + middle)])
        }
        Word::Lrl => {
            let cosine = (6.0 - d * d + 2.0 * cos_turn + 2.0 * d * (sin_b - sin_a)) / 8.0;
            if cosine.abs() > 1.0 {
                return None;
            }
            let middle = mod_2pi(TAU - cosine.acos());
            let first = mod_2pi(-alpha + (-cos_a + cos_b).atan2(d + sin_a - sin_b) + middle / 2.0);
            Some([first, middle, mod_2pi(beta - alpha - first + middle)])
        }
    }
}

/// Builds one family's path between two poses, in world coordinates.
///
/// Returns `None` when the family does not apply, or when the radius is not a
/// usable positive length.
#[must_use]
pub fn path(word: Word, from: Pose, to: Pose, radius: f64) -> Option<CurvePath> {
    let frame = Frame::between(from, to, radius)?;
    let lengths = solve(word, frame)?;
    if lengths.iter().any(|l| !l.is_finite()) {
        return None;
    }
    let steerings = word.steerings();
    let segments = (0..3)
        .map(|i| {
            // Normalised lengths are radians for arcs and radii for the
            // straight run. Multiplying by the radius turns both into metres.
            Segment::new(steerings[i], Direction::Forward, lengths[i] * radius)
        })
        .collect();
    Some(CurvePath::new(segments, radius))
}

/// Every Dubins path that applies between two poses.
///
/// The order is [`Word::ALL`], so the result is reproducible. Families that do
/// not apply are simply absent; an empty vector means no forward-only path
/// exists at this radius, which happens only when the radius is unusable.
///
/// This — and not [`shortest`] — is what a clearance-seeking caller wants.
/// The shortest path is the one that grazes most.
#[must_use]
pub fn all(from: Pose, to: Pose, radius: f64) -> Vec<CurvePath> {
    Word::ALL
        .iter()
        .filter_map(|&word| path(word, from, to, radius))
        .collect()
}

/// The shortest Dubins path, or `None` if none applies.
///
/// Ties break on [`Word::ALL`] order, so the answer is deterministic.
#[must_use]
pub fn shortest(from: Pose, to: Pose, radius: f64) -> Option<CurvePath> {
    all(from, to, radius)
        .into_iter()
        .min_by(|a, b| a.length().total_cmp(&b.length()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Radians;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    const EPS: f64 = 1e-12;

    #[test]
    fn wraps_angles_into_zero_to_two_pi() {
        assert!((mod_2pi(0.0) - 0.0).abs() < EPS);
        assert!((mod_2pi(TAU) - 0.0).abs() < EPS);
        assert!((mod_2pi(-FRAC_PI_2) - (TAU - FRAC_PI_2)).abs() < EPS);
        assert!((mod_2pi(3.0 * TAU + PI) - PI).abs() < EPS);
    }

    #[test]
    fn normalises_the_distance_by_the_radius() {
        let from = Pose::default();
        let to = Pose::new(10.0, 0.0, Radians::default());
        let frame = Frame::between(from, to, 5.0).expect("a valid frame");
        assert!((frame.d - 2.0).abs() < EPS);
        assert!(frame.alpha.abs() < EPS);
        assert!(frame.beta.abs() < EPS);
    }

    #[test]
    fn measures_headings_against_the_line_of_sight() {
        // Start pointing along +y, goal 10 m along +x also pointing along +y.
        // The line of sight runs along +x, so both headings sit ninety degrees
        // off it.
        let from = Pose::new(0.0, 0.0, Radians::new(FRAC_PI_2));
        let to = Pose::new(10.0, 0.0, Radians::new(FRAC_PI_2));
        let frame = Frame::between(from, to, 5.0).expect("a valid frame");
        assert!((frame.alpha - FRAC_PI_2).abs() < EPS);
        assert!((frame.beta - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn is_invariant_under_rotation_and_translation() {
        // The frame is the whole point: two pose pairs that differ only by a
        // rigid motion must normalise identically, or the closed forms would
        // have to know about the world frame.
        let from = Pose::new(1.0, 2.0, Radians::new(0.3));
        let to = Pose::new(6.0, 5.0, Radians::new(1.1));
        let plain = Frame::between(from, to, 4.0).expect("a valid frame");

        let turn: f64 = 0.7;
        let (sin, cos) = turn.sin_cos();
        let rotate = |p: Pose| {
            Pose::new(
                p.x * cos - p.y * sin + 13.0,
                p.x * sin + p.y * cos - 8.0,
                p.heading + Radians::new(turn),
            )
        };
        let moved = Frame::between(rotate(from), rotate(to), 4.0).expect("a valid frame");

        assert!((plain.d - moved.d).abs() < 1e-9);
        assert!((plain.alpha - moved.alpha).abs() < 1e-9);
        assert!((plain.beta - moved.beta).abs() < 1e-9);
    }

    #[test]
    fn refuses_a_radius_that_is_not_positive() {
        let from = Pose::default();
        let to = Pose::new(10.0, 0.0, Radians::default());
        assert!(Frame::between(from, to, 0.0).is_none());
        assert!(Frame::between(from, to, -1.0).is_none());
        assert!(Frame::between(from, to, f64::NAN).is_none());
    }

    /// Integrates a word's path and checks it lands on the goal.
    ///
    /// This is the arbiter for every family: it consults no published table,
    /// only the kinematics the rest of the crate already trusts.
    fn assert_lands_on(word: Word, from: Pose, to: Pose, radius: f64) {
        let path =
            path(word, from, to, radius).unwrap_or_else(|| panic!("{word:?} should apply here"));
        let end = path.end(from);
        assert!(
            (end.x - to.x).abs() < 1e-9,
            "{word:?}: x off by {}",
            end.x - to.x
        );
        assert!(
            (end.y - to.y).abs() < 1e-9,
            "{word:?}: y off by {}",
            end.y - to.y
        );
        let heading_error = mod_2pi(end.heading.get() - to.heading.get());
        let heading_error = heading_error.min(TAU - heading_error);
        assert!(
            heading_error < 1e-9,
            "{word:?}: heading off by {heading_error}"
        );
    }

    #[test]
    fn every_csc_family_lands_on_the_goal() {
        // Far apart, so all four apply: the two poses are further than four
        // radii, which no CSC family can fail to join.
        let from = Pose::new(0.0, 0.0, Radians::new(0.4));
        let to = Pose::new(12.0, 7.0, Radians::new(2.1));
        for word in [Word::Lsl, Word::Rsr, Word::Lsr, Word::Rsl] {
            assert_lands_on(word, from, to, 3.0);
        }
    }

    #[test]
    fn a_csc_path_never_reverses() {
        let from = Pose::new(0.0, 0.0, Radians::new(0.4));
        let to = Pose::new(12.0, 7.0, Radians::new(2.1));
        for word in [Word::Lsl, Word::Rsr, Word::Lsr, Word::Rsl] {
            let path = path(word, from, to, 3.0).expect("applies");
            assert_eq!(path.reversals(), 0, "{word:?} reversed");
            for segment in path.segments() {
                assert_eq!(segment.direction, Direction::Forward);
            }
        }
    }

    #[test]
    fn lsl_is_a_straight_line_when_the_poses_are_aligned() {
        // Same heading, goal straight ahead: both arcs vanish and only the
        // straight run survives, so the path is exactly the separation.
        let from = Pose::default();
        let to = Pose::new(9.0, 0.0, Radians::default());
        let path = path(Word::Lsl, from, to, 3.0).expect("applies");
        assert!((path.length() - 9.0).abs() < 1e-9);
        assert_eq!(path.segments().len(), 1);
        assert_eq!(path.segments()[0].steering, Steering::Straight);
    }

    #[test]
    fn a_crossing_family_does_not_apply_when_the_poses_are_too_close() {
        // LSR and RSL need room for the straight run that crosses between the
        // two circles. Inside two radii there is none.
        let from = Pose::default();
        let to = Pose::new(0.2, 0.0, Radians::new(PI));
        assert!(path(Word::Lsr, from, to, 5.0).is_none());
    }

    #[test]
    fn every_ccc_family_lands_on_the_goal() {
        // Close together and nearly reversed: the regime where three arcs beat
        // any arc-straight-arc, and the regime an entry manoeuvre lives in.
        let from = Pose::new(0.0, 0.0, Radians::new(0.2));
        let to = Pose::new(2.0, 1.0, Radians::new(2.6));
        for word in [Word::Rlr, Word::Lrl] {
            assert_lands_on(word, from, to, 3.0);
        }
    }

    #[test]
    fn a_ccc_family_does_not_apply_when_the_poses_are_far_apart() {
        // Same heading and thirteen radii apart, so the two turning circles
        // sit thirteen radii from each other too. A third circle of the same
        // radius cannot touch both past four, and the closed form's arccos
        // leaves its domain. (Separation alone would not be enough to say
        // this: opposed headings pull the centres two radii closer.)
        let from = Pose::default();
        let to = Pose::new(40.0, 0.0, Radians::default());
        assert!(path(Word::Rlr, from, to, 3.0).is_none());
        assert!(path(Word::Lrl, from, to, 3.0).is_none());
    }

    #[test]
    fn a_ccc_path_never_reverses() {
        let from = Pose::new(0.0, 0.0, Radians::new(0.2));
        let to = Pose::new(2.0, 1.0, Radians::new(2.6));
        for word in [Word::Rlr, Word::Lrl] {
            let path = path(word, from, to, 3.0).expect("applies");
            assert_eq!(path.reversals(), 0, "{word:?} reversed");
        }
    }

    #[test]
    fn every_word_keeps_to_the_turning_radius() {
        let from = Pose::new(0.0, 0.0, Radians::new(0.4));
        let to = Pose::new(12.0, 7.0, Radians::new(2.1));
        let radius = 3.0;
        for word in [Word::Lsl, Word::Rsr, Word::Lsr, Word::Rsl] {
            let path = path(word, from, to, radius).expect("applies");
            for segment in path.segments() {
                let curvature = segment.curvature(radius).abs();
                assert!(
                    curvature < 1.0 / radius + 1e-12,
                    "{word:?} turns tighter than the vehicle can"
                );
            }
        }
    }

    #[test]
    fn all_returns_every_applicable_family() {
        // Far apart, so the four arc-straight-arc families all apply. Whether
        // an arc-arc-arc family joins them is *not* settled by the separation
        // alone: `d` measures pose to pose, while the three-arc condition is
        // about the distance between turning-circle centres, and the headings
        // displace each centre by a radius. Four and a half radii apart, `LRL`
        // does apply here — and lands. So this pins what actually holds:
        // everything `path` accepts is present, and nothing else is.
        let from = Pose::new(0.0, 0.0, Radians::new(0.4));
        let to = Pose::new(12.0, 7.0, Radians::new(2.1));
        let applicable = Word::ALL
            .into_iter()
            .filter(|&word| path(word, from, to, 3.0).is_some())
            .count();
        let paths = all(from, to, 3.0);
        assert_eq!(paths.len(), applicable);
        assert!(
            paths.len() >= 4,
            "the four arc-straight-arc families always apply this far apart"
        );
        for path in paths {
            let end = path.end(from);
            assert!((end.x - to.x).abs() < 1e-9);
            assert!((end.y - to.y).abs() < 1e-9);
        }
    }

    #[test]
    fn a_very_short_leading_arc_is_not_dropped() {
        // Found by the property tests, and worth naming because the cause is
        // not obvious. `RSL` opens here with an arc of 0.86 mm, which the
        // original millimetre threshold on [`NEGLIGIBLE_LENGTH_M`] discarded.
        // The arc is tiny but it turns the vehicle by 0.63 mrad, and the 20.7 m
        // straight run that follows amplifies that into 13 mm at the landing
        // point. Dropping a segment costs its heading, not merely its length.
        let from = Pose::new(
            -11.817_686_772_629_195,
            2.818_970_478_859_454_5,
            Radians::new(0.438_459_390_871_496_5),
        );
        let to = Pose::new(
            5.942_933_673_257_208,
            11.555_281_195_304_69,
            Radians::new(5.954_130_075_693_988),
        );
        assert_lands_on(Word::Rsl, from, to, 1.365_349_745_873_029_3);
    }

    #[test]
    fn all_returns_nothing_when_the_radius_is_unusable() {
        let from = Pose::default();
        let to = Pose::new(10.0, 0.0, Radians::default());
        assert!(all(from, to, 0.0).is_empty());
    }

    #[test]
    fn every_returned_path_lands_on_the_goal() {
        // The contract the callers rely on: whatever comes back is usable
        // without rechecking. Lot 2b will test these for collision and keep
        // the roomiest, and it must not have to filter out broken ones first.
        let from = Pose::new(1.0, -2.0, Radians::new(1.3));
        let to = Pose::new(6.0, 4.0, Radians::new(0.2));
        for path in all(from, to, 2.5) {
            let end = path.end(from);
            assert!((end.x - to.x).abs() < 1e-9);
            assert!((end.y - to.y).abs() < 1e-9);
        }
    }

    #[test]
    fn the_shortest_is_no_longer_than_any_other() {
        let from = Pose::new(1.0, -2.0, Radians::new(1.3));
        let to = Pose::new(6.0, 4.0, Radians::new(0.2));
        let best = shortest(from, to, 2.5).expect("some family applies");
        for path in all(from, to, 2.5) {
            assert!(best.length() <= path.length() + 1e-12);
        }
    }

    #[test]
    fn the_shortest_beats_the_straight_line_only_by_matching_it() {
        // Aligned poses: no curve can be shorter than the separation, and one
        // achieves it. A shortest path below that would mean the geometry is
        // wrong, not that it found a clever route.
        let from = Pose::default();
        let to = Pose::new(9.0, 0.0, Radians::default());
        let best = shortest(from, to, 3.0).expect("some family applies");
        assert!((best.length() - 9.0).abs() < 1e-9);
    }

    #[test]
    fn a_close_reversed_goal_is_reached_at_all() {
        // Half a radius apart, pointing the other way — the regime a courtyard
        // entry actually poses, and the one the old hand-built candidates
        // could not represent. The assertion is deliberately not "the answer
        // comes from RLR": several families may apply, and which one wins is
        // not a property worth freezing. What matters is that an answer exists
        // and that it lands.
        let from = Pose::default();
        let to = Pose::new(1.5, 0.5, Radians::new(PI));
        let best = shortest(from, to, 3.0).expect("some family applies");
        let end = best.end(from);
        assert!((end.x - to.x).abs() < 1e-9);
        assert!((end.y - to.y).abs() < 1e-9);
        let heading_error = mod_2pi(end.heading.get() - to.heading.get());
        assert!(heading_error.min(TAU - heading_error) < 1e-9);
    }

    #[test]
    fn the_three_arc_families_appear_when_the_poses_are_close() {
        // The CCC words exist for under four radii of separation. Losing them
        // would go unnoticed — the CSC words still answer — so this pins their
        // presence explicitly.
        let from = Pose::default();
        let to = Pose::new(1.5, 0.5, Radians::new(PI));
        let three_arc = [Word::Rlr, Word::Lrl]
            .into_iter()
            .filter(|&word| path(word, from, to, 3.0).is_some())
            .count();
        assert!(
            three_arc > 0,
            "no arc-arc-arc family applied at half a radius"
        );
    }
}
