# Lot 2a — Courbes de Dubins — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner à `swept-core` la capacité de construire toutes les trajectoires
de Dubins entre deux poses, à rayon de braquage borné, sans rien changer au
comportement actuel du solveur.

**Architecture:** Un module `curves` dans `swept-core`, purement géométrique :
il ne connaît ni scène, ni obstacle, ni stratégie de recherche — seulement deux
poses et un rayon. Il produit des `CurvePath`, chaînes de segments à courbure
constante que la cinématique existante sait déjà intégrer. Aucun appelant n'est
modifié : le lot 2b s'en chargera.

**Tech Stack:** Rust 1.97.1 (édition 2024), aucune dépendance de production.
`proptest 1.11.0` en dépendance de développement seulement.

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**, épinglée par `rust-toolchain.toml`.
- `swept-core` garde **zéro dépendance de production**. Cette tâche ajoute
  `proptest` en `[dev-dependencies]` uniquement — invisible pour les
  consommateurs de la crate, sans effet sur sa licence `MIT OR Apache-2.0`.
  Toute entrée sous `[dependencies]` reste un signal d'erreur de conception.
- `#![deny(missing_docs)]` sur la crate. La documentation manquante casse le
  build.
- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms
  de tests, noms de branches, messages de commit. Seule la documentation projet
  (`docs/`) reste en français.
- Longueurs en **mètres** (`f64`), angles en **radians** (type `Radians`).
- **Aucune constante numérique nue.** Chaque valeur est une `const` nommée,
  documentée par sa justification et sa provenance.
- Clippy `pedantic` en warning, `missing_panics_doc` et `missing_errors_doc`
  actifs. Le CI échoue sur un warning.
- Une seule PR ouverte à la fois, branchée sur `main`. Ce lot entier est **une**
  PR ; chaque tâche est un commit.
- Repère : origine au milieu du passage, `y = 0` au nu extérieur du mur,
  `y > 0` vers la cour, `x` le long de la voie.

## Ce que ce lot ne fait pas

À ne pas déborder, même si la tentation est forte :

- Il ne touche **ni `swept-solver`, ni `swept-wasm`, ni l'interface**.
- Il n'implémente **pas** Reeds-Shepp — marche arrière, douze familles, lot 2c.
- Il ne teste **aucune collision** : les courbes ignorent les obstacles.
- Il ne choisit **pas** de trajectoire selon la marge — c'est le lot 2b.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-core/Cargo.toml` | Ajout de `proptest` en dev-dependency |
| `crates/swept-core/src/lib.rs` | Déclaration du module `curves` |
| `crates/swept-core/src/curves/mod.rs` | `Steering`, `Segment`, `CurvePath` — le vocabulaire commun, et l'intégration d'une chaîne de segments |
| `crates/swept-core/src/curves/dubins.rs` | Le repère normalisé, les six familles, `all` et `shortest` |
| `crates/swept-core/tests/dubins_properties.rs` | Tests de propriétés sous `proptest` |
| `docs/ALGORITHME.md` | Une section décrivant les courbes, en français |

Deux fichiers de code plutôt qu'un : le vocabulaire (`mod.rs`) servira tel quel
à Reeds-Shepp au lot 2c, alors que les six familles (`dubins.rs`) lui sont
propres. Les séparer maintenant évite d'avoir à démêler un fichier unique dans
deux lots.

---

## Le test qui arbitre tout

Une remarque avant les tâches, parce qu'elle décide de la conception des tests.

Les formules des six familles sont reprises de la littérature (Shkel &
Lumelsky, *Classification of the Dubins set*, 2001 ; LaValle, *Planning
Algorithms* §13.3.3). Elles sont denses, pleines de `atan2` et de conventions de
signe, et **elles se recopient mal**. Les versions publiées en ligne divergent
entre elles sur `LSR` et `LRL` en particulier.

La parade est de ne jamais tester une formule contre une autre formule, mais
contre la **cinématique** :

> Construis le chemin de `A` vers `B`, intègre-le avec `Pose::advance`, et tu
> dois arriver en `B`.

Ce test ne dépend d'aucune source. Il est vrai ou faux, sans discussion. Les
formules données dans ce plan sont un **point de départ** : si un test
d'atterrissage échoue, c'est la formule qu'on corrige, jamais le test.

---

### Task 1: Le vocabulaire des courbes

**Files:**
- Create: `crates/swept-core/src/curves/mod.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `kinematics::{Pose, Direction, sample_arc}`, `units::Radians`
- Produces: `curves::Steering`, `curves::Segment`, `curves::CurvePath`, avec
  `CurvePath::new(segments: Vec<Segment>, radius: f64) -> Self`,
  `CurvePath::length(&self) -> f64`,
  `CurvePath::end(&self, from: Pose) -> Pose`,
  `CurvePath::poses(&self, from: Pose, step: f64) -> Vec<Pose>`,
  `CurvePath::reversals(&self) -> usize`,
  `Segment::curvature(&self, radius: f64) -> f64`,
  `Segment::signed_length(&self) -> f64`

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-core/src/curves/mod.rs` avec, pour tout contenu, ce bloc de
tests. Le module ne compile pas encore : c'est voulu.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    const EPS: f64 = 1e-9;

    #[test]
    fn a_straight_segment_covers_its_length() {
        let path = CurvePath::new(
            vec![Segment::new(Steering::Straight, Direction::Forward, 3.0)],
            5.0,
        );
        assert!((path.length() - 3.0).abs() < EPS);
    }

    #[test]
    fn a_path_ends_where_the_kinematics_says_it_does() {
        // A quarter circle to the left at radius 5 takes the vehicle 5 m
        // forward and 5 m to its left, pointing ninety degrees round.
        let path = CurvePath::new(
            vec![Segment::new(
                Steering::Left,
                Direction::Forward,
                5.0 * FRAC_PI_2,
            )],
            5.0,
        );
        let end = path.end(Pose::default());
        assert!((end.x - 5.0).abs() < EPS);
        assert!((end.y - 5.0).abs() < EPS);
        assert!((end.heading.get() - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn turning_right_mirrors_turning_left() {
        let radius = 4.0;
        let quarter = radius * FRAC_PI_2;
        let left = CurvePath::new(
            vec![Segment::new(Steering::Left, Direction::Forward, quarter)],
            radius,
        )
        .end(Pose::default());
        let right = CurvePath::new(
            vec![Segment::new(Steering::Right, Direction::Forward, quarter)],
            radius,
        )
        .end(Pose::default());
        assert!((left.x - right.x).abs() < EPS);
        assert!((left.y + right.y).abs() < EPS);
        assert!((left.heading.get() + right.heading.get()).abs() < EPS);
    }

    #[test]
    fn sampling_ends_on_the_endpoint() {
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 3.0),
                Segment::new(Steering::Straight, Direction::Forward, 2.0),
            ],
            5.0,
        );
        let poses = path.poses(Pose::default(), 0.1);
        let last = *poses.last().expect("a sampled path is never empty");
        let end = path.end(Pose::default());
        assert!((last.x - end.x).abs() < EPS);
        assert!((last.y - end.y).abs() < EPS);
        assert!((last.heading.get() - end.heading.get()).abs() < EPS);
    }

    #[test]
    fn a_forward_only_path_has_no_reversals() {
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 1.0),
                Segment::new(Steering::Straight, Direction::Forward, 1.0),
            ],
            5.0,
        );
        assert_eq!(path.reversals(), 0);
    }

    #[test]
    fn changing_gear_counts_as_one_reversal() {
        // Reeds-Shepp will produce these; Dubins never does. Counting them
        // here means the solver can compare a Dubins path and a Reeds-Shepp
        // path on the same footing at lot 2c.
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 1.0),
                Segment::new(Steering::Right, Direction::Reverse, 1.0),
                Segment::new(Steering::Left, Direction::Forward, 1.0),
            ],
            5.0,
        );
        assert_eq!(path.reversals(), 2);
    }

    #[test]
    fn zero_length_segments_are_dropped() {
        // The closed forms routinely return a zero-length arc — LSL degenerates
        // to LS when the headings already agree. Keeping them would inflate the
        // reversal count and clutter the rendered path.
        let path = CurvePath::new(
            vec![
                Segment::new(Steering::Left, Direction::Forward, 0.0),
                Segment::new(Steering::Straight, Direction::Forward, 2.0),
            ],
            5.0,
        );
        assert_eq!(path.segments().len(), 1);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib curves`
Expected: FAIL — `error[E0433]: failed to resolve: use of undeclared crate or module `curves`` puis, une fois le module déclaré, `cannot find type `CurvePath``.

- [ ] **Step 3: Write minimal implementation**

Ajouter au-dessus du bloc de tests dans `crates/swept-core/src/curves/mod.rs` :

```rust
//! Optimal paths between two poses at bounded curvature.
//!
//! A vehicle that cannot turn tighter than some radius does not join two poses
//! by a straight line. The shortest way is a chain of at most three pieces,
//! each either a straight segment or an arc at exactly the minimum radius —
//! a result due to Dubins (1957) for forward-only motion, extended to reverse
//! by Reeds and Shepp (1990).
//!
//! This module holds what both families share: the alphabet of segments, and
//! the integration of a chain of them. The families themselves live in
//! [`dubins`].
//!
//! # Why the whole set and not just the shortest
//!
//! These curves minimise *length*. This project cares about *clearance* — the
//! room left between the vehicle and the posts. The shortest path grazes more,
//! not less. So the callers ask for every admissible curve, discard those that
//! collide, and keep the roomiest. Length only breaks ties.

use crate::kinematics::{Direction, Pose, sample_arc};

pub mod dubins;

/// Which way the steering is held over a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steering {
    /// Full lock to the left, at the minimum radius.
    Left,
    /// Wheels straight.
    Straight,
    /// Full lock to the right, at the minimum radius.
    Right,
}

/// Shortest segment worth keeping, in metres.
///
/// ARBITRARY — a millimetre is below anything this domain measures, and the
/// closed forms routinely return arcs of 1e-16 m where a family degenerates.
/// Keeping them would inflate the reversal count and clutter the drawing.
pub const NEGLIGIBLE_LENGTH_M: f64 = 0.001;

/// One piece of a path: constant steering, constant gear, a given length.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Where the steering is held.
    pub steering: Steering,
    /// Whether the vehicle is going forwards or backing up.
    pub direction: Direction,
    /// Arc length travelled, in metres. Always positive — the direction
    /// carries the sign.
    pub length: f64,
}

impl Segment {
    /// Builds a segment. `length` is a distance, so it must not be negative.
    #[must_use]
    pub fn new(steering: Steering, direction: Direction, length: f64) -> Self {
        Self { steering, direction, length: length.abs() }
    }

    /// The curvature to feed [`Pose::advance`], in reciprocal metres.
    ///
    /// Positive curvature turns left in this frame, because `y` grows to the
    /// left of a vehicle heading along `+x`.
    #[must_use]
    pub fn curvature(&self, radius: f64) -> f64 {
        match self.steering {
            Steering::Left => 1.0 / radius,
            Steering::Straight => 0.0,
            Steering::Right => -1.0 / radius,
        }
    }

    /// The distance to feed [`Pose::advance`], negative when reversing.
    #[must_use]
    pub fn signed_length(&self) -> f64 {
        match self.direction {
            Direction::Forward => self.length,
            Direction::Reverse => -self.length,
        }
    }
}

/// A chain of segments at one fixed turning radius.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePath {
    segments: Vec<Segment>,
    radius: f64,
}

impl CurvePath {
    /// Builds a path, dropping segments too short to matter.
    #[must_use]
    pub fn new(segments: Vec<Segment>, radius: f64) -> Self {
        let segments = segments
            .into_iter()
            .filter(|s| s.length > NEGLIGIBLE_LENGTH_M)
            .collect();
        Self { segments, radius }
    }

    /// The segments, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The turning radius every arc uses, in metres.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Total distance travelled, in metres, forwards and backwards alike.
    #[must_use]
    pub fn length(&self) -> f64 {
        self.segments.iter().map(|s| s.length).sum()
    }

    /// How many times the vehicle changes between forward and reverse.
    ///
    /// This is what the interface calls a manoeuvre. A Dubins path always
    /// scores zero; Reeds-Shepp paths will not.
    #[must_use]
    pub fn reversals(&self) -> usize {
        self.segments
            .windows(2)
            .filter(|pair| pair[0].direction != pair[1].direction)
            .count()
    }

    /// Where the path ends, starting from `from`.
    #[must_use]
    pub fn end(&self, from: Pose) -> Pose {
        self.segments.iter().fold(from, |pose, segment| {
            pose.advance(segment.curvature(self.radius), segment.signed_length())
        })
    }

    /// The path as successive poses, spaced by at most `step` metres.
    ///
    /// `from` itself is excluded, exactly as [`sample_arc`] excludes its own
    /// starting pose, so that chaining introduces no duplicate.
    ///
    /// # Panics
    ///
    /// Panics if `step` is not strictly positive.
    #[must_use]
    pub fn poses(&self, from: Pose, step: f64) -> Vec<Pose> {
        assert!(step > 0.0, "sampling step must be strictly positive");

        let mut pose = from;
        let mut out = Vec::new();
        for segment in &self.segments {
            let sampled = sample_arc(
                pose,
                segment.curvature(self.radius),
                segment.signed_length(),
                step,
            );
            if let Some(last) = sampled.last() {
                pose = *last;
            }
            out.extend(sampled);
        }
        out
    }
}
```

Puis déclarer le module dans `crates/swept-core/src/lib.rs`, en gardant l'ordre
alphabétique des `pub mod` :

```rust
pub mod clearance;
pub mod curves;
pub mod geometry;
pub mod kinematics;
pub mod scene;
pub mod units;
pub mod vehicle;
```

Créer aussi `crates/swept-core/src/curves/dubins.rs` avec pour seul contenu une
doc de module provisoire, sans quoi `pub mod dubins;` ne compile pas :

```rust
//! The six Dubins families. Filled in by the next tasks.
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib curves`
Expected: PASS — 7 tests.

Puis `cargo clippy -p swept-core --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves crates/swept-core/src/lib.rs
git commit -m "feat(core): add the segment vocabulary bounded-curvature paths share"
```

---

### Task 2: Le repère normalisé

**Files:**
- Modify: `crates/swept-core/src/curves/dubins.rs`

**Interfaces:**
- Consumes: `kinematics::Pose`, `curves::CurvePath`
- Produces: `dubins::mod_2pi(angle: f64) -> f64`,
  `dubins::Frame { d: f64, alpha: f64, beta: f64 }`,
  `dubins::Frame::between(from: Pose, to: Pose, radius: f64) -> Option<Frame>`

Toutes les formules des six familles s'expriment dans un repère où le départ est
à l'origine, cap nul, et où les longueurs sont divisées par le rayon. Cette
tâche construit ce changement de repère, seul et testé, parce qu'une erreur ici
se manifesterait six fois de suite et serait diagnostiquée six fois.

- [ ] **Step 1: Write the failing test**

Remplacer le contenu de `crates/swept-core/src/curves/dubins.rs` par ce bloc de
tests seul :

```rust
//! The six Dubins families.

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

        let turn = 0.7;
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: FAIL — `cannot find function `mod_2pi``.

- [ ] **Step 3: Write minimal implementation**

Insérer, entre la doc de module et le bloc de tests :

```rust
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
//! (2001), and cross-checked against LaValle, *Planning Algorithms* §13.3.3.
//! They are dense and they transcribe badly; published versions disagree on
//! `LSR` and `LRL`. Every family is therefore tested by integrating its result
//! through [`Pose::advance`] and checking it lands on the goal. That test
//! depends on no source and settles any disagreement.

use crate::kinematics::Pose;
use std::f64::consts::TAU;

/// Wraps an angle into `[0, 2π)`.
///
/// The closed forms subtract angles freely and rely on the result being taken
/// modulo a full turn; a negative arc length would otherwise be read as a
/// reverse segment, which Dubins never produces.
#[must_use]
pub fn mod_2pi(angle: f64) -> f64 {
    let wrapped = angle % TAU;
    if wrapped < 0.0 { wrapped + TAU } else { wrapped }
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: PASS — 5 tests.

Note : n'importer que `Pose` et `TAU` à ce stade. `CurvePath`, `Segment`,
`Steering` et `Direction` n'ont pas encore d'usage, et `#![deny(missing_docs)]`
mis à part, ce sont les warnings `unused_imports` qui feraient échouer
`clippy -D warnings`. La Task 3 les ajoute quand elle s'en sert.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/dubins.rs
git commit -m "feat(core): normalise a pose pair into the Dubins frame"
```

---

### Task 3: Les quatre familles arc-droite-arc

**Files:**
- Modify: `crates/swept-core/src/curves/dubins.rs`

**Interfaces:**
- Consumes: `dubins::{Frame, mod_2pi}`, `curves::{CurvePath, Segment, Steering}`
- Produces: `dubins::Word` (énumération des six mots),
  `dubins::solve(word: Word, frame: Frame) -> Option<[f64; 3]>` — les trois
  longueurs normalisées, ou `None` si la famille ne s'applique pas,
  `dubins::path(word: Word, from: Pose, to: Pose, radius: f64) -> Option<CurvePath>`

Cette tâche livre `LSL`, `RSR`, `LSR`, `RSL`. Les deux familles arc-arc-arc
suivent en Task 4, dans la même `solve`.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` existant de `dubins.rs` :

```rust
    /// Integrates a word's path and checks it lands on the goal.
    ///
    /// This is the arbiter for every family: it consults no published table,
    /// only the kinematics the rest of the crate already trusts.
    fn assert_lands_on(word: Word, from: Pose, to: Pose, radius: f64) {
        let path = path(word, from, to, radius)
            .unwrap_or_else(|| panic!("{word:?} should apply here"));
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: FAIL — `cannot find type `Word``.

- [ ] **Step 3: Write minimal implementation**

Étendre d'abord les imports en tête de fichier, maintenant qu'il y a de quoi
les employer :

```rust
use super::{CurvePath, Segment, Steering};
use crate::kinematics::{Direction, Pose};
use std::f64::consts::TAU;
```

Puis ajouter après `Frame` :

```rust
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
    pub const ALL: [Self; 6] =
        [Self::Lsl, Self::Rsr, Self::Lsr, Self::Rsl, Self::Rlr, Self::Lrl];

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
    let cos_ab = (alpha - beta).cos();

    match word {
        Word::Lsl => {
            let squared = 2.0 + d * d - 2.0 * cos_ab + 2.0 * d * (sin_a - sin_b);
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
            let squared = 2.0 + d * d - 2.0 * cos_ab + 2.0 * d * (sin_b - sin_a);
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
            let squared = -2.0 + d * d + 2.0 * cos_ab + 2.0 * d * (sin_a + sin_b);
            if squared < 0.0 {
                return None;
            }
            let straight = squared.sqrt();
            let tangent = (-cos_a - cos_b).atan2(d + sin_a + sin_b) - (-2.0f64).atan2(straight);
            Some([
                mod_2pi(tangent - alpha),
                straight,
                mod_2pi(tangent - beta),
            ])
        }
        Word::Rsl => {
            let squared = -2.0 + d * d + 2.0 * cos_ab - 2.0 * d * (sin_a + sin_b);
            if squared < 0.0 {
                return None;
            }
            let straight = squared.sqrt();
            let tangent = (cos_a + cos_b).atan2(d - sin_a - sin_b) - 2.0f64.atan2(straight);
            Some([
                mod_2pi(alpha - tangent),
                straight,
                mod_2pi(beta - tangent),
            ])
        }
        Word::Rlr | Word::Lrl => None, // Task 4.
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: PASS — 10 tests.

**Si `every_csc_family_lands_on_the_goal` échoue**, le message nomme la famille
et l'écart. Ne pas relâcher la tolérance. Vérifier dans cet ordre :

1. Le signe de la courbure dans `Segment::curvature` — `Left` doit produire une
   courbure positive, puisque `y` croît à gauche d'un véhicule orienté vers
   `+x`.
2. La convention de `alpha` et `beta` — mesurés depuis la ligne de visée, pas
   depuis l'axe `x`.
3. La formule elle-même, `atan2` par `atan2`. Un `atan2(y, x)` inversé en
   `atan2(x, y)` est l'erreur de transcription la plus fréquente.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/dubins.rs
git commit -m "feat(core): solve the four arc-straight-arc Dubins families"
```

---

### Task 4: Les deux familles arc-arc-arc

**Files:**
- Modify: `crates/swept-core/src/curves/dubins.rs`

**Interfaces:**
- Consumes: tout ce que la Task 3 produit
- Produces: `solve` répond désormais pour `Word::Rlr` et `Word::Lrl`

`RLR` et `LRL` n'existent que lorsque les poses sont **proches** — moins de
quatre rayons. Le véhicule fait alors un crochet à trois arcs plutôt qu'un
grand tour. Ce sont exactement les cas d'une entrée de cour serrée, donc ces
deux familles ne sont pas anecdotiques ici.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` :

```rust
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
        // Beyond four radii there is no third circle touching both, and the
        // closed form's arccos leaves its domain.
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib curves::dubins::tests::every_ccc_family_lands_on_the_goal`
Expected: FAIL — panic `Rlr should apply here`, puisque `solve` rend `None`.

- [ ] **Step 3: Write minimal implementation**

Remplacer le bras `Word::Rlr | Word::Lrl => None, // Task 4.` par :

```rust
        Word::Rlr => {
            // The middle arc's half-angle comes from the law of cosines on the
            // triangle joining the three circle centres. Outside [-1, 1] the
            // triangle does not close: no third circle touches both.
            let cosine = (6.0 - d * d + 2.0 * cos_ab + 2.0 * d * (sin_a - sin_b)) / 8.0;
            if cosine.abs() > 1.0 {
                return None;
            }
            let middle = mod_2pi(TAU - cosine.acos());
            let first = mod_2pi(
                alpha - (cos_a - cos_b).atan2(d - sin_a + sin_b) + middle / 2.0,
            );
            Some([first, middle, mod_2pi(alpha - beta - first + middle)])
        }
        Word::Lrl => {
            let cosine = (6.0 - d * d + 2.0 * cos_ab + 2.0 * d * (sin_b - sin_a)) / 8.0;
            if cosine.abs() > 1.0 {
                return None;
            }
            let middle = mod_2pi(TAU - cosine.acos());
            let first = mod_2pi(
                -alpha + (-cos_a + cos_b).atan2(d + sin_a - sin_b) + middle / 2.0,
            );
            Some([first, middle, mod_2pi(beta - alpha - first + middle)])
        }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: PASS — 13 tests.

**Si l'atterrissage échoue**, `LRL` est la famille dont les transcriptions
publiées divergent le plus. Le troisième terme est le suspect ; les variantes
rencontrées sont `mod_2pi(beta - alpha - first + middle)` et
`mod_2pi(mod_2pi(beta) - alpha + 2.0 * middle)`. Essayer la seconde avant de
remettre le reste en cause. Ne jamais relâcher la tolérance de `1e-9`.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/dubins.rs
git commit -m "feat(core): solve the two arc-arc-arc Dubins families"
```

---

### Task 5: Toutes les courbes, et la plus courte

**Files:**
- Modify: `crates/swept-core/src/curves/dubins.rs`

**Interfaces:**
- Consumes: `dubins::{Word, path}`
- Produces: `dubins::all(from: Pose, to: Pose, radius: f64) -> Vec<CurvePath>`,
  `dubins::shortest(from: Pose, to: Pose, radius: f64) -> Option<CurvePath>`

`all` est la fonction que le lot 2b consommera, et c'est la moins évidente des
deux : la spec l'établit en §3, ce projet trie par la marge, pas par la
longueur. `shortest` n'existe que pour les tests et pour un éventuel départage.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` :

```rust
    #[test]
    fn all_returns_every_applicable_family() {
        // Far apart: the four CSC families apply, the two CCC do not.
        let from = Pose::new(0.0, 0.0, Radians::new(0.4));
        let to = Pose::new(12.0, 7.0, Radians::new(2.1));
        let paths = all(from, to, 3.0);
        assert_eq!(paths.len(), 4);
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
        assert!(three_arc > 0, "no arc-arc-arc family applied at half a radius");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib curves::dubins`
Expected: FAIL — `cannot find function `all``.

- [ ] **Step 3: Write minimal implementation**

Ajouter à la fin de `dubins.rs`, avant le bloc de tests :

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib curves`
Expected: PASS — 27 tests au total sous `curves` : 7 pour le vocabulaire,
20 pour les familles, dont 7 ajoutés par cette tâche.

Puis `cargo clippy -p swept-core --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/dubins.rs
git commit -m "feat(core): enumerate every applicable Dubins path"
```

---

### Task 6: Les tests de propriétés

**Files:**
- Modify: `crates/swept-core/Cargo.toml`
- Create: `crates/swept-core/tests/dubins_properties.rs`

**Interfaces:**
- Consumes: `swept_core::curves::dubins::{all, shortest}`,
  `swept_core::curves::CurvePath`
- Produces: rien de nouveau — cette tâche ne fait qu'établir que ce qui précède
  tient sur autre chose que six paires de poses choisies à la main.

Les tests des tâches 3 à 5 valident des cas nommés. Ce sont les propriétés qui
attrapent les régimes qu'on n'a pas pensé à écrire : poses confondues, headings
opposés, distances au seuil exact des quatre rayons. C'est précisément là que
les `acos` et les divisions des formes closes sortent de leur domaine.

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-core/tests/dubins_properties.rs` :

```rust
//! Properties every Dubins path must satisfy, whatever the pose pair.
//!
//! The unit tests check named cases. These check the regimes nobody thought
//! to name — coincident poses, opposed headings, separations sitting exactly
//! on the four-radius threshold where the CCC families appear and disappear.
//! That is where closed forms fail, by leaving the domain of an `acos` or
//! dividing by a vanishing term.

use proptest::prelude::*;
use std::f64::consts::TAU;
use swept_core::curves::dubins::{all, shortest};
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// How far apart the generated poses may sit, in metres.
///
/// ARBITRARY — wide enough to straddle the four-radius threshold at every
/// radius generated below, narrow enough that the generator keeps producing
/// close pairs, which is where the CCC families live.
const SPREAD_M: f64 = 15.0;

prop_compose! {
    fn any_pose()(
        x in -SPREAD_M..SPREAD_M,
        y in -SPREAD_M..SPREAD_M,
        heading in 0.0..TAU,
    ) -> Pose {
        Pose::new(x, y, Radians::new(heading))
    }
}

proptest! {
    /// The contract the whole module rests on.
    #[test]
    fn every_path_lands_on_the_goal(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            let end = path.end(from);
            prop_assert!(
                (end.x - to.x).abs() < 1e-6,
                "x off by {} on a path of {} segments",
                end.x - to.x,
                path.segments().len(),
            );
            prop_assert!((end.y - to.y).abs() < 1e-6);
        }
    }

    /// A vehicle cannot turn tighter than its minimum radius. A path that
    /// asked it to would be geometrically valid and physically impossible.
    #[test]
    fn no_path_turns_tighter_than_the_radius(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            for segment in path.segments() {
                prop_assert!(segment.curvature(radius).abs() <= 1.0 / radius + 1e-12);
            }
        }
    }

    /// Dubins is forward-only by definition. A reversal here would mean the
    /// segment vocabulary is being misused.
    #[test]
    fn no_dubins_path_ever_reverses(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            prop_assert_eq!(path.reversals(), 0);
        }
    }

    /// A bounded-curvature path cannot beat the straight line between the two
    /// points. Anything shorter is a bug in the length accounting.
    #[test]
    fn no_path_is_shorter_than_the_separation(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let separation = (to.x - from.x).hypot(to.y - from.y);
        for path in all(from, to, radius) {
            prop_assert!(path.length() >= separation - 1e-9);
        }
    }

    /// `shortest` must actually be the shortest of `all`, and must exist
    /// whenever `all` is non-empty.
    #[test]
    fn the_shortest_is_the_minimum_of_all(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let paths = all(from, to, radius);
        match shortest(from, to, radius) {
            None => prop_assert!(paths.is_empty()),
            Some(best) => {
                prop_assert!(!paths.is_empty());
                for path in paths {
                    prop_assert!(best.length() <= path.length() + 1e-12);
                }
            }
        }
    }

    /// No length is ever a NaN or an infinity. The closed forms divide and
    /// take arc cosines; a degenerate frame must yield `None`, never a path
    /// carrying a poisoned number that would silently spread.
    #[test]
    fn no_length_is_ever_not_a_number(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        for path in all(from, to, radius) {
            prop_assert!(path.length().is_finite());
            for segment in path.segments() {
                prop_assert!(segment.length.is_finite());
            }
        }
    }
}

/// Coincident poses are the one degenerate case worth naming: the separation
/// is zero, the line of sight is undefined, and `atan2(0, 0)` returns zero
/// rather than failing. Nothing may panic.
#[test]
fn coincident_poses_do_not_panic() {
    let pose = Pose::new(3.0, -1.0, Radians::new(0.8));
    let paths = all(pose, pose, 4.0);
    for path in &paths {
        assert!(path.length().is_finite());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --test dubins_properties`
Expected: FAIL — `error[E0432]: unresolved import `proptest``.

- [ ] **Step 3: Write minimal implementation**

Ajouter à `crates/swept-core/Cargo.toml`, entre `[dependencies]` et `[lints]` :

```toml
[dependencies]

# Development only, so the published crate still has no dependency at all and
# keeps its MIT OR Apache-2.0 licensing. The closed forms below are exactly the
# kind of code where a named case passes and a generated one does not.
[dev-dependencies]
proptest = "1.11.0"

[lints]
workspace = true
```

Aucun code de production à écrire : si une propriété échoue, c'est une tâche
précédente qui est en cause, et c'est elle qu'on corrige.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --test dubins_properties`
Expected: PASS — 6 propriétés et 1 test nommé.

**En cas d'échec**, `proptest` imprime le cas minimal et l'écrit dans
`crates/swept-core/proptest-regressions/`. **Commiter ce fichier** : il devient
un test de non-régression permanent. Puis reprendre la formule fautive, jamais
la tolérance.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/Cargo.toml crates/swept-core/tests/dubins_properties.rs
git add crates/swept-core/proptest-regressions 2>/dev/null || true
git commit -m "test(core): check Dubins paths against generated pose pairs"
```

---

### Task 7: Documentation et intégration

**Files:**
- Modify: `docs/ALGORITHME.md`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: tout ce qui précède
- Produces: rien de nouveau. Cette tâche rend le lot lisible par quelqu'un qui
  n'a pas suivi, et vérifie que la crate entière tient toujours debout.

- [ ] **Step 1: Write the failing test**

Le test, ici, c'est la documentation elle-même : `#![deny(missing_docs)]` et les
`doctest` du module. Ajouter à `crates/swept-core/src/curves/mod.rs`, dans la
doc de module, un exemple exécutable — il échouera tant qu'il n'est pas juste :

````rust
//! # Example
//!
//! ```
//! use swept_core::curves::dubins;
//! use swept_core::kinematics::Pose;
//! use swept_core::units::Radians;
//!
//! // A vehicle facing along the road, asked to end up nine metres ahead
//! // facing the same way, cannot do better than driving straight.
//! let from = Pose::default();
//! let to = Pose::new(9.0, 0.0, Radians::default());
//! let best = dubins::shortest(from, to, 3.0).expect("some family applies");
//! assert!((best.length() - 9.0).abs() < 1e-9);
//!
//! // But the caller usually wants every option, to keep the roomiest rather
//! // than the shortest.
//! assert!(!dubins::all(from, to, 3.0).is_empty());
//! ```
````

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --doc`
Expected: PASS si l'exemple est juste du premier coup — c'est acceptable ici.
S'il échoue, le message donne l'assertion fautive.

- [ ] **Step 3: Write minimal implementation**

Ajouter à `docs/ALGORITHME.md`, en français, une section après celle qui décrit
la recherche exhaustive :

```markdown
## Les courbes de Dubins

Le solveur construisait ses trajectoires candidates à la main : une droite, un
quart de tour, une droite. Cette forme est arbitraire, et sur un passage serré
aucune de ses 7 410 variantes ne passe.

Dubins a montré en 1957 que le chemin le plus court entre deux poses, pour un
véhicule qui ne peut pas braquer plus court qu'un certain rayon et qui n'a pas
de marche arrière, est **toujours** l'un de six mots : quatre du type
arc-droite-arc (`LSL`, `RSR`, `LSR`, `RSL`) et deux du type arc-arc-arc
(`RLR`, `LRL`). Chacun a une forme close : aucune recherche, aucune itération.

Les quatre premiers s'appliquent quand les poses sont éloignées. Les deux
derniers n'existent que sous quatre rayons de distance, quand le véhicule fait
un crochet plutôt qu'un grand tour — c'est-à-dire exactement le régime d'une
entrée de cour.

**Une précision qui décide de tout l'usage.** Ces courbes minimisent la
*longueur*. Ce projet mesure la *marge*. La plus courte est celle qui rase le
plus. On n'utilise donc jamais `shortest`, mais `all` : on énumère les six,
on écarte celles qui touchent un obstacle, et on garde la plus dégagée. La
longueur ne sert qu'à départager.

**Comment on sait que les formules sont justes.** On ne les compare pas à
d'autres formules — les versions publiées divergent, sur `LSR` et `LRL` en
particulier. On construit la courbe, on l'intègre avec la cinématique du
véhicule, et on vérifie qu'elle arrive sur la pose visée à 1e-9 près. Ce test
ne dépend d'aucune source. Il tourne sur six paires de poses nommées et sur
des paires engendrées aléatoirement par `proptest`.
```

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout — `fmt`, `clippy`, les tests Rust, les tests web,
les vecteurs de référence, le build Wasm. Rien de ce lot ne touche au solveur,
donc **aucun test existant ne doit changer de résultat**. S'il y en a un qui
bouge, c'est que le module a été branché quelque part par mégarde, ce que ce
lot s'interdit.

Run: `cargo doc -p swept-core --no-deps`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add docs/ALGORITHME.md crates/swept-core/src/curves/mod.rs
git commit -m "docs: describe the Dubins families and why the shortest is not wanted"
```

---

## Ce que le lot 2b consommera

Récapitulé ici pour que le lot suivant n'ait pas à relire le code :

```rust
swept_core::curves::dubins::all(from: Pose, to: Pose, radius: f64) -> Vec<CurvePath>
swept_core::curves::dubins::shortest(from: Pose, to: Pose, radius: f64) -> Option<CurvePath>
swept_core::curves::CurvePath::poses(&self, from: Pose, step: f64) -> Vec<Pose>
swept_core::curves::CurvePath::length(&self) -> f64
swept_core::curves::CurvePath::reversals(&self) -> usize
```

Le lot 2b balaiera des poses de départ le long de la chaussée et des poses
d'arrivée dans l'axe du passage, appellera `all` pour chaque paire, échantillonnera
par `poses`, et testera chaque échantillon contre la scène avec le
`ClearanceField` existant. Il gardera la plus dégagée.

## Vérification finale du lot

Avant d'ouvrir la PR :

- [ ] `just ci` passe.
- [ ] `cargo tree -p swept-core --edges normal` ne montre **aucune** dépendance.
- [ ] `git diff main --stat` ne touche que `crates/swept-core/` et
      `docs/ALGORITHME.md`. Aucun fichier de `swept-solver`, `swept-wasm` ou
      `web/`.
- [ ] Les six familles ont chacune un test d'atterrissage qui les nomme.
- [ ] Aucune tolérance n'a été relâchée pour faire passer un test.
