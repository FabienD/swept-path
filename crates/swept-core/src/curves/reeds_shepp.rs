//! The twelve Reeds-Shepp families.
//!
//! Reeds and Shepp (1990) extended Dubins to a vehicle that may reverse: the
//! shortest path between two poses at bounded curvature is then one of
//! forty-eight words, built from twelve fundamental families. Like Dubins,
//! every one has a closed form.
//!
//! What matters here beyond the length: Reeds-Shepp also minimises the number
//! of **direction changes**, which is precisely what this project counts as a
//! manoeuvre.
//!
//! # The normalised frame
//!
//! Formulas are written with the start at the origin facing `+x` and lengths
//! divided by the turning radius, leaving the goal as `(x, y, φ)`. This is not
//! the `(d, α, β)` triple [`super::dubins`] uses: Reeds-Shepp's involutions
//! act simply on `(x, y, φ)` and awkwardly on the other.
//!
//! # Forty-eight words from eight functions
//!
//! Two involutions generate the rest. **Time flip** drives the path backwards,
//! which negates `x` and `φ` and turns every forward segment into a reverse
//! one. **Reflection** swaps left for right, which negates `y` and `φ`. Applied
//! to the *input* rather than the output, they let eight base functions cover
//! everything — the alternative being forty-eight transcriptions, each its own
//! chance of a sign error.
//!
//! # On the formulas
//!
//! Taken from Reeds & Shepp, *Optimal paths for a car that goes both forwards
//! and backwards* (1990), cross-checked against `LaValle`, *Planning
//! Algorithms* §15.3. They transcribe badly and published versions disagree.
//! **Every family is therefore tested by integrating its result through
//! [`Pose::advance`] and checking where it lands** — a test that depends on no
//! source and settles any disagreement.

use super::{CurvePath, Segment, Steering};
use crate::kinematics::{Direction, Pose};
use std::f64::consts::{PI, TAU};

/// Wraps an angle into `(-π, π]`.
///
/// Centred on zero, unlike [`super::dubins::mod_2pi`], because Reeds-Shepp
/// tests angles against zero to decide whether a family applies: a value just
/// below a full turn must read as a small negative, not as a large positive.
#[must_use]
pub fn wrap_pi(angle: f64) -> f64 {
    let wrapped = angle % TAU;
    if wrapped > PI {
        wrapped - TAU
    } else if wrapped <= -PI {
        wrapped + TAU
    } else {
        wrapped
    }
}

/// Cartesian to polar, as the formulas write it.
#[must_use]
pub fn polar(x: f64, y: f64) -> (f64, f64) {
    (x.hypot(y), y.atan2(x))
}

/// The goal pose, in the start's frame, divided by the turning radius.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    /// Ahead of the start, in radii.
    pub x: f64,
    /// To the left of the start, in radii.
    pub y: f64,
    /// Change of heading, in radians.
    pub phi: f64,
}

impl Frame {
    /// Normalises a start and goal pose against a turning radius.
    ///
    /// Returns `None` when the radius is not a usable positive length.
    #[must_use]
    pub fn between(from: Pose, to: Pose, radius: f64) -> Option<Self> {
        if !radius.is_finite() || radius <= 0.0 {
            return None;
        }
        let (sin, cos) = from.heading.sin_cos();
        let (dx, dy) = (to.x - from.x, to.y - from.y);
        Some(Self {
            x: dx.mul_add(cos, dy * sin) / radius,
            y: dy.mul_add(cos, -(dx * sin)) / radius,
            phi: wrap_pi(to.heading.get() - from.heading.get()),
        })
    }

    /// The same problem driven backwards.
    ///
    /// Time symmetry: a path traversed in reverse covers the same ground, so
    /// solving the flipped problem and negating every length gives a word
    /// whose gears are all swapped.
    #[must_use]
    pub fn time_flipped(self) -> Self {
        Self {
            x: -self.x,
            y: self.y,
            phi: -self.phi,
        }
    }

    /// The same problem with left and right exchanged.
    #[must_use]
    pub fn reflected(self) -> Self {
        Self {
            x: self.x,
            y: -self.y,
            phi: -self.phi,
        }
    }
}

/// One piece of a Reeds-Shepp word: a steering, and a **signed** length.
///
/// The sign is the gear — negative means reversing. Lengths are normalised:
/// radians for an arc, radii for a straight run, which the radius converts to
/// metres in one place, [`Word::path`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Element {
    /// Where the steering is held.
    pub steering: Steering,
    /// Signed, normalised length.
    pub length: f64,
}

/// A Reeds-Shepp word: two to five elements.
#[derive(Debug, Clone, PartialEq)]
pub struct Word(pub Vec<Element>);

impl Word {
    /// Whether this word can be driven at all.
    ///
    /// The closed forms divide and take arc cosines, so a family that does not
    /// apply can yield a NaN rather than nothing. Catching it here keeps a
    /// poisoned number out of every path built downstream.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.0.is_empty() && self.0.iter().all(|e| e.length.is_finite())
    }

    /// The path this word describes, in metres.
    ///
    /// **The only place a sign becomes a gear.** Anywhere else would be a
    /// second chance to lose one.
    #[must_use]
    pub fn path(&self, radius: f64) -> CurvePath {
        let segments = self
            .0
            .iter()
            .map(|e| {
                let direction = if e.length < 0.0 {
                    Direction::Reverse
                } else {
                    Direction::Forward
                };
                Segment::new(e.steering, direction, e.length.abs() * radius)
            })
            .collect();
        CurvePath::new(segments, radius)
    }

    /// Total normalised length, ignoring gear.
    #[must_use]
    pub fn cost(&self) -> f64 {
        self.0.iter().map(|e| e.length.abs()).sum()
    }
}

/// One reading of a base family: how to transform the frame going in, whether
/// the lengths come back negated, and whether left and right swap.
type Variant = (fn(Frame) -> Frame, bool, bool);

/// The four ways to read a base family: as written, driven backwards,
/// mirrored, and both.
const VARIANTS: [Variant; 4] = [
    (|f| f, false, false),
    (Frame::time_flipped, true, false),
    (Frame::reflected, false, true),
    (|f| f.time_flipped().reflected(), true, true),
];

/// `L⁺S⁺L⁺` — two arcs the same way, joined by a straight run.
///
/// Reading it: the goal's turning centre sits at `(x − sin φ, y − 1 + cos φ)`
/// relative to the start's own centre. The polar form of that offset gives the
/// straight run directly, and its bearing gives the first arc.
fn lp_sp_lp(f: Frame) -> Option<(f64, f64, f64)> {
    let (u, t) = polar(f.x - f.phi.sin(), f.y - 1.0 + f.phi.cos());
    let v = wrap_pi(f.phi - t);
    (t >= 0.0 && v >= 0.0).then_some((t, u, v))
}

/// `L⁺S⁺R⁺` — two arcs opposite ways, joined by a straight run.
///
/// The two turning centres are two radii apart across the straight run, which
/// is where the `− 4` comes from: below that separation the run cannot exist
/// and the family does not apply.
fn lp_sp_rp(f: Frame) -> Option<(f64, f64, f64)> {
    let (u1, t1) = polar(f.x + f.phi.sin(), f.y - 1.0 - f.phi.cos());
    let squared = u1.mul_add(u1, -4.0);
    if squared < 0.0 {
        return None;
    }
    let u = squared.sqrt();
    let t = wrap_pi(t1 + 2.0_f64.atan2(u));
    let v = wrap_pi(t - f.phi);
    (t >= 0.0 && v >= 0.0).then_some((t, u, v))
}

/// Every arc-straight-arc word between the frame's two poses.
///
/// The four variants come from the two involutions: as written the arcs turn
/// left first and every segment is forward; time-flipping gives the reverse
/// gears, reflecting gives the right-first mirror.
#[must_use]
pub fn csc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right, Straight};
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };

        let same = if mirror { Right } else { Left };
        if let Some((t, u, v)) = lp_sp_lp(f) {
            out.push(Word(vec![
                Element {
                    steering: same,
                    length: sign * t,
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: same,
                    length: sign * v,
                },
            ]));
        }

        let (first, last) = if mirror { (Right, Left) } else { (Left, Right) };
        if let Some((t, u, v)) = lp_sp_rp(f) {
            out.push(Word(vec![
                Element {
                    steering: first,
                    length: sign * t,
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: last,
                    length: sign * v,
                },
            ]));
        }
    }
    out.retain(Word::is_valid);
    out
}

/// `L⁺R⁻L⁻` — three arcs, no straight run.
///
/// The two turning centres are `u1` apart. Three arcs can bridge them only
/// while that stays within four radii, which is what the `acos(u1 / 4)` says:
/// beyond it the argument leaves `[-1, 1]` and the family does not apply.
fn lp_rm_lm(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x - f.phi.sin();
    let eta = f.y - 1.0 + f.phi.cos();
    let (separation, theta) = polar(xi, eta);
    if separation > 4.0 {
        return None;
    }
    let half_middle = (separation / 4.0).acos();
    let t = wrap_pi(theta + PI / 2.0 + half_middle);
    let u = wrap_pi(2.0f64.mul_add(-half_middle, PI));
    let v = wrap_pi(f.phi - t - u);
    // `(t, −u, v)`, not `(t, −u, −v)`. The heading a word turns through is the
    // sum of its arcs with `L` positive and `R` negative, so this triplet
    // turns through `t + u + v`, which the last line makes equal to φ. Negating
    // `v` as well turns through `t + u − v` and lands a full turn out — an
    // error the landing test caught at 2.96 radians.
    Some((t, -u, v))
}

/// Every three-arc word between the frame's two poses.
#[must_use]
pub fn ccc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right};
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };
        let (a, b) = if mirror { (Right, Left) } else { (Left, Right) };

        if let Some((t, u, v)) = lp_rm_lm(f) {
            out.push(Word(vec![
                Element {
                    steering: a,
                    length: sign * t,
                },
                Element {
                    steering: b,
                    length: sign * u,
                },
                Element {
                    steering: a,
                    length: sign * v,
                },
            ]));
        }

        // The same three arcs read from the goal backwards, which is a
        // distinct word rather than the same one: the outer arcs swap places
        // while the middle keeps its gear.
        let (turn_sin, turn_cos) = f.phi.sin_cos();
        let backwards = Frame {
            x: f.x.mul_add(turn_cos, f.y * turn_sin),
            y: f.x.mul_add(turn_sin, -(f.y * turn_cos)),
            phi: f.phi,
        };
        if let Some((t, u, v)) = lp_rm_lm(backwards) {
            out.push(Word(vec![
                Element {
                    steering: a,
                    length: sign * v,
                },
                Element {
                    steering: b,
                    length: sign * u,
                },
                Element {
                    steering: a,
                    length: sign * t,
                },
            ]));
        }
    }
    out.retain(Word::is_valid);
    out
}

/// Solves the two outer arcs once the two inner ones are known.
///
/// Shared by both four-arc families, which differ only in how they choose the
/// inner pair.
fn tau_omega(u: f64, v: f64, xi: f64, eta: f64, phi: f64) -> (f64, f64) {
    let delta = wrap_pi(u - v);
    let a = u.sin() - delta.sin();
    let b = u.cos() - delta.cos() - 1.0;
    let t1 = eta.mul_add(a, -(xi * b)).atan2(xi.mul_add(a, eta * b));
    let t2 = 2.0f64.mul_add(delta.cos() - v.cos() - u.cos(), 3.0);
    let tau = if t2 < 0.0 {
        wrap_pi(t1 + PI)
    } else {
        wrap_pi(t1)
    };
    let omega = wrap_pi(tau - u + v - phi);
    (tau, omega)
}

/// `L⁺R⁺L⁻R⁻` — the inner arcs turn the same way for the same length.
fn lp_rup_lum_rm(f: Frame) -> Option<(f64, f64, f64, f64)> {
    let xi = f.x + f.phi.sin();
    let eta = f.y - 1.0 - f.phi.cos();
    let rho = 0.25 * (2.0 + xi.hypot(eta));
    if rho > 1.0 {
        return None;
    }
    let u = rho.acos();
    let (t, v) = tau_omega(u, -u, xi, eta, f.phi);
    Some((t, u, -u, v))
}

/// `L⁺R⁻L⁻R⁺` — the inner arcs turn opposite ways for the same length.
fn lp_rum_lum_rp(f: Frame) -> Option<(f64, f64, f64, f64)> {
    let xi = f.x + f.phi.sin();
    let eta = f.y - 1.0 - f.phi.cos();
    let rho = (20.0 - xi.mul_add(xi, eta * eta)) / 16.0;
    if !(0.0..=1.0).contains(&rho) {
        return None;
    }
    let u = -rho.acos();
    if u < -PI / 2.0 {
        return None;
    }
    let (t, v) = tau_omega(u, u, xi, eta, f.phi);
    Some((t, u, u, v))
}

/// Every four-arc word between the frame's two poses.
///
/// These are the families that manoeuvre on the spot: they appear when the
/// goal is close but badly oriented, which is exactly a narrow gateway.
#[must_use]
pub fn cccc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right};
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };
        let (a, b) = if mirror { (Right, Left) } else { (Left, Right) };
        for (t, u, w, v) in [lp_rup_lum_rm(f), lp_rum_lum_rp(f)].into_iter().flatten() {
            {
                out.push(Word(vec![
                    Element {
                        steering: a,
                        length: sign * t,
                    },
                    Element {
                        steering: b,
                        length: sign * u,
                    },
                    Element {
                        steering: a,
                        length: sign * w,
                    },
                    Element {
                        steering: b,
                        length: sign * v,
                    },
                ]));
            }
        }
    }
    out.retain(Word::is_valid);
    out
}

/// `L⁺R⁻S⁻L⁻` — the straight run leaves the second arc on the same side.
fn lp_rm_sm_lm(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x - f.phi.sin();
    let eta = f.y - 1.0 + f.phi.cos();
    let (rho, theta) = polar(xi, eta);
    if rho < 2.0 {
        return None;
    }
    let leg = rho.mul_add(rho, -4.0).sqrt();
    let u = 2.0 - leg;
    let t = wrap_pi(theta + leg.atan2(-2.0));
    let v = wrap_pi(f.phi - PI / 2.0 - t);
    (t >= 0.0 && u <= 0.0 && v <= 0.0).then_some((t, u, v))
}

/// `L⁺R⁻S⁻R⁻` — the straight run leaves the second arc on the other side.
fn lp_rm_sm_rm(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x + f.phi.sin();
    let eta = f.y - 1.0 - f.phi.cos();
    let (rho, theta) = polar(-eta, xi);
    if rho < 2.0 {
        return None;
    }
    let t = theta;
    let u = 2.0 - rho;
    let v = wrap_pi(t + PI / 2.0 - f.phi);
    (t >= 0.0 && u <= 0.0 && v <= 0.0).then_some((t, u, v))
}

/// Every arc-arc-straight-arc word between the frame's two poses.
///
/// The middle arc turns exactly a quarter, which is what these families are
/// defined by and what makes their closed form so short.
#[must_use]
pub fn ccsc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right, Straight};
    let quarter = -PI / 2.0;
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };
        let (a, b) = if mirror { (Right, Left) } else { (Left, Right) };

        if let Some((t, u, v)) = lp_rm_sm_lm(f) {
            out.push(Word(vec![
                Element {
                    steering: a,
                    length: sign * t,
                },
                Element {
                    steering: b,
                    length: sign * quarter,
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: a,
                    length: sign * v,
                },
            ]));
        }
        if let Some((t, u, v)) = lp_rm_sm_rm(f) {
            out.push(Word(vec![
                Element {
                    steering: a,
                    length: sign * t,
                },
                Element {
                    steering: b,
                    length: sign * quarter,
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: b,
                    length: sign * v,
                },
            ]));
        }
    }
    out.retain(Word::is_valid);
    out
}

/// `L⁺R⁻S⁻L⁻R⁺` — the only five-element family.
fn lp_rm_sm_lm_rp(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x + f.phi.sin();
    let eta = f.y - 1.0 - f.phi.cos();
    let (rho, _) = polar(xi, eta);
    if rho < 2.0 {
        return None;
    }
    let u = 4.0 - rho.mul_add(rho, -4.0).sqrt();
    if u > 0.0 {
        return None;
    }
    let t = wrap_pi(
        (4.0 - u)
            .mul_add(xi, -(2.0 * eta))
            .atan2((-2.0f64).mul_add(xi, (u - 4.0) * eta)),
    );
    let v = wrap_pi(t - f.phi);
    (t >= 0.0 && v >= 0.0).then_some((t, u, v))
}

/// Every five-element word between the frame's two poses.
#[must_use]
pub fn ccscc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right, Straight};
    let quarter = -PI / 2.0;
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };
        let (a, b) = if mirror { (Right, Left) } else { (Left, Right) };
        if let Some((t, u, v)) = lp_rm_sm_lm_rp(f) {
            out.push(Word(vec![
                Element {
                    steering: a,
                    length: sign * t,
                },
                Element {
                    steering: b,
                    length: sign * quarter,
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: a,
                    length: sign * quarter,
                },
                Element {
                    steering: b,
                    length: sign * v,
                },
            ]));
        }
    }
    out.retain(Word::is_valid);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Radians;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    const EPS: f64 = 1e-12;

    /// Integrates a word and returns how far its end misses the frame's goal.
    ///
    /// This is the arbiter of the whole module: it depends on no publication,
    /// only on [`Pose::advance`]. A family that transcribes wrongly lands
    /// somewhere else, and this says by how much.
    fn landing_error(word: &Word, frame: Frame) -> f64 {
        let end = word.path(1.0).end(Pose::default());
        let heading = (end.heading.get() - frame.phi).rem_euclid(TAU);
        let heading = heading.min(TAU - heading);
        (end.x - frame.x)
            .abs()
            .max((end.y - frame.y).abs())
            .max(heading)
    }

    /// Every word a family returns must land on the goal.
    fn assert_all_land(words: &[Word], frame: Frame) {
        assert!(!words.is_empty(), "no word for {frame:?}");
        for word in words {
            assert!(word.is_valid(), "invalid word {word:?}");
            let error = landing_error(word, frame);
            assert!(error < 1e-9, "{word:?} misses by {error} on {frame:?}");
        }
    }

    #[test]
    fn the_five_element_family_lands_on_its_goal() {
        let frame = Frame {
            x: 3.5,
            y: 3.0,
            phi: 0.4,
        };
        assert_all_land(&ccscc(frame), frame);
    }

    #[test]
    fn the_five_element_family_is_absent_when_the_goal_is_close() {
        let frame = Frame {
            x: 0.1,
            y: 0.1,
            phi: 0.0,
        };
        assert!(ccscc(frame).is_empty());
    }

    #[test]
    fn arc_arc_straight_arc_lands_on_a_turned_goal() {
        let frame = Frame {
            x: 2.5,
            y: 2.0,
            phi: 2.2,
        };
        assert_all_land(&ccsc(frame), frame);
    }

    #[test]
    fn arc_arc_straight_arc_is_absent_when_the_goal_is_too_close() {
        let frame = Frame {
            x: 0.05,
            y: 0.02,
            phi: 0.01,
        };
        assert!(ccsc(frame).is_empty());
    }

    #[test]
    fn four_arcs_land_on_a_near_goal() {
        let frame = Frame {
            x: 0.4,
            y: 1.1,
            phi: 0.3,
        };
        // `assert_all_land` rather than a bare loop: a landing test over an
        // empty vector passes without proving anything, which is precisely the
        // failure mode of a family whose sign condition is too strict.
        assert_all_land(&cccc(frame), frame);
    }

    #[test]
    fn four_arcs_are_absent_when_the_goal_is_far() {
        let frame = Frame {
            x: 9.0,
            y: 0.0,
            phi: 0.0,
        };
        assert!(cccc(frame).is_empty());
    }

    #[test]
    fn three_arcs_land_on_a_near_goal() {
        // Close together and turned: exactly where the three-arc families
        // live, and where the straight-run ones give long detours.
        let frame = Frame {
            x: 0.6,
            y: 0.9,
            phi: 1.4,
        };
        assert_all_land(&ccc(frame), frame);
    }

    #[test]
    fn three_arcs_are_absent_when_the_circles_cannot_meet() {
        // Far apart, no three-arc word applies. Returning an empty vector is
        // the answer, not a failure.
        let frame = Frame {
            x: 12.0,
            y: 0.0,
            phi: 0.0,
        };
        assert!(ccc(frame).is_empty());
    }

    #[test]
    fn three_arcs_land_when_the_goal_sits_behind() {
        let frame = Frame {
            x: -0.8,
            y: 0.4,
            phi: -0.9,
        };
        assert_all_land(&ccc(frame), frame);
    }

    #[test]
    fn arc_straight_arc_lands_on_the_goal() {
        // A goal ahead, offset and turned: the bread-and-butter case where
        // both same-side and opposed families apply.
        let frame = Frame {
            x: 3.0,
            y: 1.2,
            phi: 0.7,
        };
        assert_all_land(&csc(frame), frame);
    }

    #[test]
    fn arc_straight_arc_handles_a_goal_straight_ahead() {
        let frame = Frame {
            x: 4.0,
            y: 0.0,
            phi: 0.0,
        };
        assert_all_land(&csc(frame), frame);
    }

    #[test]
    fn arc_straight_arc_handles_a_goal_behind() {
        // Reverse is the whole point of Reeds-Shepp: a goal behind the start
        // must still be reachable, which Dubins could only manage by driving
        // right round.
        let frame = Frame {
            x: -3.0,
            y: 0.5,
            phi: 0.2,
        };
        assert_all_land(&csc(frame), frame);
    }

    #[test]
    fn a_negative_length_becomes_a_reverse_segment() {
        // The one thing this type exists to get right.
        let word = Word(vec![
            Element {
                steering: Steering::Left,
                length: 1.0,
            },
            Element {
                steering: Steering::Right,
                length: -0.5,
            },
        ]);
        let path = word.path(2.0);
        let segments = path.segments();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].direction, Direction::Forward);
        assert!(
            (segments[0].length - 2.0).abs() < EPS,
            "one radian at radius 2"
        );
        assert_eq!(segments[1].direction, Direction::Reverse);
        assert!((segments[1].length - 1.0).abs() < EPS);
        assert_eq!(path.reversals(), 1);
    }

    #[test]
    fn a_word_carrying_a_non_finite_length_is_refused() {
        // The closed forms divide and take arc cosines. A poisoned number must
        // be caught here rather than spread into a path nobody can drive.
        let word = Word(vec![Element {
            steering: Steering::Left,
            length: f64::NAN,
        }]);
        assert!(!word.is_valid());
    }

    #[test]
    fn an_empty_word_is_not_valid() {
        assert!(!Word(Vec::new()).is_valid());
    }

    #[test]
    fn a_word_of_finite_lengths_is_valid() {
        let word = Word(vec![Element {
            steering: Steering::Straight,
            length: -3.0,
        }]);
        assert!(word.is_valid());
        assert!((word.cost() - 3.0).abs() < EPS);
    }

    #[test]
    fn the_frame_puts_the_start_at_the_origin_facing_along_x() {
        // Nine metres ahead at a three-metre radius is three radii along x,
        // nothing across, and no change of heading.
        let from = Pose::new(4.0, -2.0, Radians::new(FRAC_PI_2));
        let to = Pose::new(4.0, 7.0, Radians::new(FRAC_PI_2));
        let frame = Frame::between(from, to, 3.0).expect("a usable radius");
        assert!((frame.x - 3.0).abs() < EPS, "got x={}", frame.x);
        assert!(frame.y.abs() < EPS, "got y={}", frame.y);
        assert!(frame.phi.abs() < EPS, "got phi={}", frame.phi);
    }

    #[test]
    fn an_unusable_radius_yields_no_frame() {
        let pose = Pose::default();
        assert!(Frame::between(pose, pose, 0.0).is_none());
        assert!(Frame::between(pose, pose, -1.0).is_none());
        assert!(Frame::between(pose, pose, f64::NAN).is_none());
    }

    #[test]
    fn turning_time_about_mirrors_the_problem_along_x() {
        // Driving the path backwards is the same problem with x and the
        // heading negated. Applying it twice must return the original.
        let frame = Frame {
            x: 1.5,
            y: -0.4,
            phi: 0.8,
        };
        let there = frame.time_flipped();
        assert!((there.x + 1.5).abs() < EPS);
        assert!((there.y + 0.4).abs() < EPS);
        assert!((there.phi + 0.8).abs() < EPS);
        let back = there.time_flipped();
        assert!((back.x - frame.x).abs() < EPS);
        assert!((back.phi - frame.phi).abs() < EPS);
    }

    #[test]
    fn reflecting_mirrors_the_problem_along_y() {
        // Swapping left for right is the same problem with y and the heading
        // negated. Also an involution.
        let frame = Frame {
            x: 1.5,
            y: -0.4,
            phi: 0.8,
        };
        let there = frame.reflected();
        assert!((there.x - 1.5).abs() < EPS);
        assert!((there.y - 0.4).abs() < EPS);
        assert!((there.phi + 0.8).abs() < EPS);
        let back = there.reflected();
        assert!((back.y - frame.y).abs() < EPS);
    }

    #[test]
    fn angles_wrap_into_a_half_turn_either_side() {
        // Reeds-Shepp compares angles against zero to decide whether a family
        // applies, so its wrap must be centred on zero — unlike the Dubins
        // one, which runs from zero to a full turn.
        assert!(wrap_pi(0.0).abs() < EPS);
        assert!((wrap_pi(TAU + 0.3) - 0.3).abs() < EPS);
        assert!((wrap_pi(-0.3) + 0.3).abs() < EPS);
        for angle in [3.0 * PI, -3.0 * PI, 7.5, -7.5, 0.0] {
            let wrapped = wrap_pi(angle);
            assert!(
                wrapped > -PI && wrapped <= PI,
                "{angle} wrapped to {wrapped}"
            );
        }
    }

    #[test]
    fn polar_coordinates_round_trip() {
        let (r, theta) = polar(3.0, 4.0);
        assert!((r - 5.0).abs() < EPS);
        assert!((r * theta.cos() - 3.0).abs() < EPS);
        assert!((r * theta.sin() - 4.0).abs() < EPS);
    }
}
