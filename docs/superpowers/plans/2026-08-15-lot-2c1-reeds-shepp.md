# Lot 2c-1 — Les douze familles de Reeds-Shepp — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner à `swept-core` les courbes de Reeds-Shepp — les chemins les
plus courts entre deux poses à courbure bornée **avec marche arrière** — sans
rien brancher, exactement comme le lot 2a l'a fait pour Dubins.

**Architecture:** Huit fonctions de base couvrent les douze familles ; les
quarante-huit mots s'en déduisent par deux involutions, le retournement du
temps et la réflexion, appliquées **à l'entrée** plutôt qu'à la sortie. Chaque
famille est validée dès son écriture par un test d'atterrissage : on intègre le
résultat et on vérifie qu'il tombe sur le but.

**Tech Stack:** Rust 1.97.1 (édition 2024), zéro dépendance de production.
`proptest` en dépendance de développement, déjà présente depuis le lot 2a.

**Spec:** `docs/superpowers/specs/2026-08-10-dubins-reeds-shepp-design.md`

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**, épinglée par `rust-toolchain.toml`.
- `swept-core` garde **zéro dépendance de production**. C'est ce qui lui permet
  de rester `MIT OR Apache-2.0` à côté d'une application AGPL, et c'est
  pourquoi la crate `reeds_shepp` de crates.io est écartée : sa licence
  contaminerait le noyau.
- `#![deny(missing_docs)]`. La documentation manquante casse le build.
- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms
  de tests, noms de branches, messages de commit et descriptions de PR. Seuls
  `docs/` et l'interface restent en français.
- Longueurs en **mètres** (`f64`), angles en **radians**.
- **Aucune constante numérique nue.** Chaque valeur est une `const` nommée et
  documentée par sa provenance (`ARBITRARY` ou `MEASURED`).
- Clippy `pedantic` en warning, le CI échoue sur un warning.
- Une PR, branchée sur `main`, une tâche par commit. Branche :
  `feat/lot-2c1-reeds-shepp`.

## L'avertissement qui porte ce lot

**Ces formules se transcrivent mal.** La doc de `dubins.rs` le dit déjà pour
six familles : « published versions disagree on `LSR` and `LRL` ». Reeds-Shepp
en a douze, plus longues, avec des conditions de signe qui décident si une
solution existe. Une erreur de signe ne produit pas un plantage mais une courbe
qui atterrit ailleurs.

D'où la règle de ce plan : **chaque tâche de famille contient son test
d'atterrissage**, et ce test ne dépend d'aucune source. On intègre le résultat
par `Pose::advance` et on regarde où il tombe. C'est ce qui a permis au lot 2a
de trancher entre des publications qui se contredisaient, et c'est ce qui
attrapera les erreurs de transcription ici.

Le lot 2a a par ailleurs montré que les tests de propriétés trouvent ce que les
cas nommés manquent : ils ont exhibé un arc de 0,86 mm dont l'omission
déplaçait l'arrivée de 13 mm. La Task 9 les rétablit pour Reeds-Shepp.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-core/src/curves/reeds_shepp.rs` | **Nouveau.** Le repère normalisé, les huit fonctions de base, les involutions, `all` et `shortest`. |
| `crates/swept-core/src/curves/mod.rs` | Déclaration du module, et le peu de vocabulaire qui manque. |
| `crates/swept-core/tests/reeds_shepp_properties.rs` | **Nouveau.** Les propriétés que toute courbe doit satisfaire. |
| `docs/ALGORITHME.md` | La section sur Reeds-Shepp. |

Un seul fichier de production, comme `dubins.rs` : les huit fonctions partagent
le repère et les involutions, et les séparer obligerait à exporter des
intermédiaires qui n'ont de sens qu'ensemble.

---

### Task 1: Le repère normalisé et les involutions

**Files:**
- Create: `crates/swept-core/src/curves/reeds_shepp.rs`
- Modify: `crates/swept-core/src/curves/mod.rs`

**Interfaces:**
- Consumes: `super::{CurvePath, Segment, Steering}`, `crate::kinematics::{Direction, Pose}`
- Produces: `reeds_shepp::Frame { x, y, phi }`, `Frame::between(from, to, radius) -> Option<Self>`,
  `Frame::time_flipped(self) -> Self`, `Frame::reflected(self) -> Self`,
  `reeds_shepp::wrap_pi(angle: f64) -> f64`, `reeds_shepp::polar(x: f64, y: f64) -> (f64, f64)`

**Le repère n'est pas celui de Dubins.** `dubins::Frame` porte `(d, α, β)`, ce
qui suffit quand le départ et l'arrivée jouent des rôles symétriques. Les
formules de Reeds-Shepp sont écrites en `(x, y, φ)` — la pose d'arrivée
exprimée dans le repère du départ, normalisée par le rayon. C'est cette forme
que les involutions transforment simplement, et la traduire en `(d, α, β)`
n'apporterait rien.

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-core/src/curves/reeds_shepp.rs` avec pour tout contenu :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::Radians;
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    const EPS: f64 = 1e-12;

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
            assert!(wrapped > -PI && wrapped <= PI, "{angle} wrapped to {wrapped}");
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
```

- [ ] **Step 2: Run test to verify it fails**

Il faut d'abord déclarer le module. Dans
`crates/swept-core/src/curves/mod.rs`, sous `pub mod dubins;` :

```rust
pub mod reeds_shepp;
```

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find type `Frame` in this scope``.

- [ ] **Step 3: Write minimal implementation**

Insérer au-dessus du bloc de tests :

```rust
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
            x: (dx * cos + dy * sin) / radius,
            y: (-dx * sin + dy * cos) / radius,
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 6 tests.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs crates/swept-core/src/curves/mod.rs
git commit -m "feat(core): give Reeds-Shepp its frame and its two involutions"
```

---

### Task 2: Le mot, et comment il devient un chemin

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: la Task 1
- Produces: `reeds_shepp::Element { steering: Steering, length: f64 }`,
  `reeds_shepp::Word(Vec<Element>)`, `Word::path(&self, radius: f64) -> CurvePath`,
  `Word::is_valid(&self) -> bool`

**Un mot Reeds-Shepp porte des longueurs signées.** C'est la différence avec
Dubins : un nombre négatif ne veut pas dire « impossible » mais « en marche
arrière ». [`Segment`] du projet sépare déjà le signe (`Direction`) de la
grandeur (`length`), donc la conversion est directe — mais elle doit être faite
à un seul endroit, sous peine de perdre un signe quelque part.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` :

```rust
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
        assert!((segments[0].length - 2.0).abs() < EPS, "one radian at radius 2");
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
        assert!(
            Word(vec![Element {
                steering: Steering::Straight,
                length: -3.0,
            }])
            .is_valid()
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find struct `Word` ``.

- [ ] **Step 3: Write minimal implementation**

Ajouter avant le bloc de tests :

```rust
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 10 tests.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): let a signed Reeds-Shepp length become a gear, once"
```

---

### Task 3: Les familles arc-droite-arc

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 et 2
- Produces: `reeds_shepp::csc(frame: Frame) -> Vec<Word>`

Deux fonctions de base : `L⁺S⁺L⁺`, où les deux arcs tournent du même côté, et
`L⁺S⁺R⁺`, où ils s'opposent. Ce sont les analogues directs de `LSL` et `LSR`
chez Dubins, et ils se lisent en tournant la position du but autour du centre
du premier arc.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests`, ainsi que le helper que toutes les tâches
suivantes réemploient :

```rust
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `csc` ``.

- [ ] **Step 3: Write minimal implementation**

Ajouter avant le bloc de tests :

```rust
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
        let (first, last) = if mirror {
            (Right, Right)
        } else {
            (Left, Left)
        };
        if let Some((t, u, v)) = lp_sp_lp(f) {
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

/// The four ways to read a base family: as written, driven backwards,
/// mirrored, and both.
///
/// Each entry is the transform to apply to the frame, whether the resulting
/// lengths are negated, and whether left and right swap.
const VARIANTS: [(fn(Frame) -> Frame, bool, bool); 4] = [
    (|f| f, false, false),
    (Frame::time_flipped, true, false),
    (Frame::reflected, false, true),
    (|f| f.time_flipped().reflected(), true, true),
];
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 13 tests.

**Si un test d'atterrissage échoue**, l'erreur est dans une formule ou dans
l'application d'une involution, et le message dit de combien. Un écart proche
de 2 sur `y` désigne le signe de `y` dans une involution ; un écart sur le cap
seul désigne `wrap_pi`. Ne jamais relâcher le seuil de `1e-9` : il est atteint
par la géométrie exacte, et l'assouplir ne ferait que cacher le défaut.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): add the Reeds-Shepp arc-straight-arc families"
```

---

### Task 4: Les familles à trois arcs

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 à 3, `VARIANTS`, `assert_all_land`
- Produces: `reeds_shepp::ccc(frame: Frame) -> Vec<Word>`

`L⁺R⁻L⁻` et ses variantes : trois arcs, sans segment droit. Elles n'existent
que lorsque les poses sont proches — au-delà de quatre radii entre les centres
de braquage, les cercles ne se rencontrent plus. C'est le même seuil que chez
Dubins, et le lot 2a a montré qu'il porte sur les **centres**, pas sur les
poses : les caps déplacent chaque centre d'un radius, si bien que deux poses
distantes de quatre radii et demi peuvent encore admettre une solution.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` :

```rust
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
        let words = ccc(frame);
        for word in &words {
            let error = landing_error(word, frame);
            assert!(error < 1e-9, "{word:?} misses by {error}");
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `ccc` ``.

- [ ] **Step 3: Write minimal implementation**

```rust
/// `L⁺R⁻L⁻` — three arcs, no straight run.
///
/// The two turning centres are `u1` apart. Three arcs can bridge them only
/// while that stays within four radii, which is what the `acos(u1 / 4)` says:
/// beyond it the argument leaves `[-1, 1]` and the family does not apply.
fn lp_rm_lm(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x - f.phi.sin();
    let eta = f.y - 1.0 + f.phi.cos();
    let (u1, theta) = polar(xi, eta);
    if u1 > 4.0 {
        return None;
    }
    let a = (u1 / 4.0).acos();
    let t = wrap_pi(theta + PI / 2.0 + a);
    let u = wrap_pi(PI - 2.0 * a);
    let v = wrap_pi(f.phi - t - u);
    Some((t, -u, -v))
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
        // distinct word rather than the same one: the middle arc keeps its
        // gear while the outer two swap.
        let mirrored = Frame {
            x: f.x * f.phi.cos() + f.y * f.phi.sin(),
            y: f.x * f.phi.sin() - f.y * f.phi.cos(),
            phi: f.phi,
        };
        if let Some((t, u, v)) = lp_rm_lm(mirrored) {
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 16 tests.

**Si `three_arcs_land_on_a_near_goal` échoue sur la seconde forme**, c'est la
réécriture du repère `mirrored` qui est en cause plutôt que `lp_rm_lm` : la
première forme est testée par les deux autres cas. Le supprimer temporairement
isole la cause.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): add the Reeds-Shepp three-arc families"
```

---

### Task 5: Les familles à quatre arcs

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 à 4
- Produces: `reeds_shepp::cccc(frame: Frame) -> Vec<Word>`

Deux arcs intérieurs de même longueur, encadrés par deux autres. Ces familles
sont celles qui manœuvrent sur place — elles apparaissent quand le but est
proche mais mal orienté, ce qui est exactement le cas d'un portail étroit.

Elles partagent un calcul, `tau_omega`, qui résout les deux arcs extérieurs une
fois les intérieurs connus.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn four_arcs_land_on_a_near_goal() {
        let frame = Frame {
            x: 0.4,
            y: 1.1,
            phi: 0.3,
        };
        let words = cccc(frame);
        for word in &words {
            let error = landing_error(word, frame);
            assert!(error < 1e-9, "{word:?} misses by {error}");
        }
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `cccc` ``.

- [ ] **Step 3: Write minimal implementation**

```rust
/// Solves the two outer arcs once the two inner ones are known.
///
/// Shared by both four-arc families, which differ only in how they choose the
/// inner pair.
fn tau_omega(u: f64, v: f64, xi: f64, eta: f64, phi: f64) -> (f64, f64) {
    let delta = wrap_pi(u - v);
    let a = u.sin() - delta.sin();
    let b = u.cos() - delta.cos() - 1.0;
    let t1 = (eta * a - xi * b).atan2(xi * a + eta * b);
    let t2 = 2.0 * (delta.cos() - v.cos() - u.cos()) + 3.0;
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
    (t >= 0.0 && v <= 0.0).then_some((t, u, -u, v))
}

/// `L⁺R⁻L⁻R⁺` — the inner arcs turn opposite ways for the same length.
fn lp_rum_lum_rp(f: Frame) -> Option<(f64, f64, f64, f64)> {
    let xi = f.x + f.phi.sin();
    let eta = f.y - 1.0 - f.phi.cos();
    let rho = (20.0 - xi * xi - eta * eta) / 16.0;
    if !(0.0..=1.0).contains(&rho) {
        return None;
    }
    let u = -rho.acos();
    if u < -PI / 2.0 {
        return None;
    }
    let (t, v) = tau_omega(u, u, xi, eta, f.phi);
    (t >= 0.0 && v >= 0.0).then_some((t, u, u, v))
}

/// Every four-arc word between the frame's two poses.
#[must_use]
pub fn cccc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right};
    let mut out = Vec::new();

    for (transform, flip, mirror) in VARIANTS {
        let f = transform(frame);
        let sign = if flip { -1.0 } else { 1.0 };
        let (a, b) = if mirror { (Right, Left) } else { (Left, Right) };
        for solved in [lp_rup_lum_rm(f), lp_rum_lum_rp(f)] {
            if let Some((t, u, w, v)) = solved {
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 18 tests.

**Si `four_arcs_land_on_a_near_goal` échoue**, suspecter `tau_omega` avant les
deux appelants : il porte le branchement sur `t2`, qui choisit entre deux
solutions et dont le signe est ce que les publications transcrivent le plus mal.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): add the Reeds-Shepp four-arc families"
```

---

### Task 6: Les familles à arc-arc-droite-arc

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 à 5
- Produces: `reeds_shepp::ccsc(frame: Frame) -> Vec<Word>`

Deux arcs, un segment droit, un arc. Ce sont les familles longues, celles qui
servent quand le but est loin **et** mal orienté.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn arc_arc_straight_arc_lands_on_a_turned_goal() {
        let frame = Frame {
            x: 2.5,
            y: 2.0,
            phi: 2.2,
        };
        let words = ccsc(frame);
        for word in &words {
            let error = landing_error(word, frame);
            assert!(error < 1e-9, "{word:?} misses by {error}");
        }
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `ccsc` ``.

- [ ] **Step 3: Write minimal implementation**

```rust
/// `L⁺R⁻S⁻L⁻` — the straight run leaves the second arc on the same side.
fn lp_rm_sm_lm(f: Frame) -> Option<(f64, f64, f64)> {
    let xi = f.x - f.phi.sin();
    let eta = f.y - 1.0 + f.phi.cos();
    let (rho, theta) = polar(xi, eta);
    if rho < 2.0 {
        return None;
    }
    let r = rho.mul_add(rho, -4.0).sqrt();
    let u = 2.0 - r;
    let t = wrap_pi(theta + r.atan2(-2.0));
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
/// Eight in all: two base functions, four variants each — and each also read
/// from the goal backwards, which reverses the order of the elements.
#[must_use]
pub fn ccsc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right, Straight};
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
                    length: sign * (-PI / 2.0),
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
                    length: sign * (-PI / 2.0),
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 20 tests.

Le quart de tour codé en dur, `−π/2`, n'est pas un choix : ces familles sont
définies par un arc qui tourne exactement d'un quart de tour avant le segment
droit, ce qui est ce qui rend leur forme close si courte.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): add the Reeds-Shepp arc-arc-straight-arc families"
```

---

### Task 7: La famille à cinq éléments

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 à 6
- Produces: `reeds_shepp::ccscc(frame: Frame) -> Vec<Word>`

Un arc, un quart de tour, un segment droit, un quart de tour, un arc. La plus
longue des familles, et la seule à cinq éléments.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn the_five_element_family_lands_on_its_goal() {
        let frame = Frame {
            x: 3.5,
            y: 3.0,
            phi: 0.4,
        };
        let words = ccscc(frame);
        for word in &words {
            let error = landing_error(word, frame);
            assert!(error < 1e-9, "{word:?} misses by {error}");
        }
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `ccscc` ``.

- [ ] **Step 3: Write minimal implementation**

```rust
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
    let t = wrap_pi(((4.0 - u) * xi - 2.0 * eta).atan2(-2.0f64.mul_add(xi, (u - 4.0) * eta)));
    let v = wrap_pi(t - f.phi);
    (t >= 0.0 && v >= 0.0).then_some((t, u, v))
}

/// Every five-element word between the frame's two poses.
#[must_use]
pub fn ccscc(frame: Frame) -> Vec<Word> {
    use Steering::{Left, Right, Straight};
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
                    length: sign * (-PI / 2.0),
                },
                Element {
                    steering: Straight,
                    length: sign * u,
                },
                Element {
                    steering: a,
                    length: sign * (-PI / 2.0),
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
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: PASS — 22 tests.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): add the Reeds-Shepp five-element family"
```

---

### Task 8: La surface publique

**Files:**
- Modify: `crates/swept-core/src/curves/reeds_shepp.rs`

**Interfaces:**
- Consumes: les Tasks 1 à 7
- Produces: `reeds_shepp::all(from: Pose, to: Pose, radius: f64) -> Vec<CurvePath>`,
  `reeds_shepp::shortest(from: Pose, to: Pose, radius: f64) -> Option<CurvePath>`,
  `reeds_shepp::fewest_reversals(from: Pose, to: Pose, radius: f64) -> Option<CurvePath>`

Même surface que [`super::dubins`], plus une troisième porte d'entrée. Car
Reeds-Shepp minimise aussi le **nombre de changements de sens**, et c'est
précisément ce que ce projet appelle une manœuvre — la grandeur que
l'utilisateur compte.

- [ ] **Step 1: Write the failing test**

```rust
    #[test]
    fn all_returns_paths_that_land_on_the_goal() {
        let from = Pose::new(-2.0, 1.0, Radians::new(0.3));
        let to = Pose::new(3.0, 2.5, Radians::new(1.9));
        let radius = 4.0;
        let paths = all(from, to, radius);
        assert!(!paths.is_empty());
        for path in &paths {
            let end = path.end(from);
            assert!((end.x - to.x).abs() < 1e-6, "x off by {}", end.x - to.x);
            assert!((end.y - to.y).abs() < 1e-6, "y off by {}", end.y - to.y);
            let error = (end.heading.get() - to.heading.get()).rem_euclid(TAU);
            assert!(error.min(TAU - error) < 1e-6, "heading off by {error}");
        }
    }

    #[test]
    fn a_goal_behind_the_start_is_reachable() {
        // What Reeds-Shepp buys over Dubins. Backing up in a straight line is
        // the obvious answer, and Dubins cannot express it at all.
        let from = Pose::default();
        let to = Pose::new(-5.0, 0.0, Radians::default());
        let best = shortest(from, to, 3.0).expect("reversing gets there");
        assert!(best.length() < 5.5, "took {} m to back up 5 m", best.length());
        assert!(best.reversals() <= 1);
    }

    #[test]
    fn the_shortest_is_the_shortest_of_all() {
        let from = Pose::new(1.0, 1.0, Radians::new(0.5));
        let to = Pose::new(-2.0, 3.0, Radians::new(2.0));
        let paths = all(from, to, 2.5);
        let best = shortest(from, to, 2.5).expect("some family applies");
        for path in &paths {
            assert!(best.length() <= path.length() + 1e-12);
        }
    }

    #[test]
    fn fewest_reversals_prefers_a_longer_path_with_fewer_shunts() {
        // The distinction the interface counts. Where two words reach the same
        // goal, one with a reversal and one without, this must pick the one
        // without — even if it is longer.
        let from = Pose::default();
        let to = Pose::new(6.0, 3.0, Radians::new(0.4));
        let radius = 3.0;
        let smooth = fewest_reversals(from, to, radius).expect("some family applies");
        let short = shortest(from, to, radius).expect("some family applies");
        assert!(smooth.reversals() <= short.reversals());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib reeds_shepp`
Expected: FAIL — ``cannot find function `all` ``.

- [ ] **Step 3: Write minimal implementation**

```rust
/// Every Reeds-Shepp path between two poses, at this radius.
///
/// This — and not [`shortest`] — is what a clearance-seeking caller wants. The
/// shortest path is the one that grazes most, a point the whole of this crate
/// turns on.
///
/// An empty vector means the radius is unusable; between two real poses at a
/// real radius, Reeds-Shepp always finds something.
#[must_use]
pub fn all(from: Pose, to: Pose, radius: f64) -> Vec<CurvePath> {
    let Some(frame) = Frame::between(from, to, radius) else {
        return Vec::new();
    };
    let mut words = csc(frame);
    words.extend(ccc(frame));
    words.extend(cccc(frame));
    words.extend(ccsc(frame));
    words.extend(ccscc(frame));
    words.iter().map(|w| w.path(radius)).collect()
}

/// The shortest of [`all`], by distance travelled.
#[must_use]
pub fn shortest(from: Pose, to: Pose, radius: f64) -> Option<CurvePath> {
    all(from, to, radius)
        .into_iter()
        .min_by(|a, b| a.length().total_cmp(&b.length()))
}

/// The path of [`all`] with the fewest direction changes, length breaking ties.
///
/// A reversal is what a driver counts as a manoeuvre, and Reeds-Shepp minimises
/// that as well as length — but not with the same word. Where the two disagree,
/// this is the one to show a driver.
#[must_use]
pub fn fewest_reversals(from: Pose, to: Pose, radius: f64) -> Option<CurvePath> {
    all(from, to, radius).into_iter().min_by(|a, b| {
        a.reversals()
            .cmp(&b.reversals())
            .then_with(|| a.length().total_cmp(&b.length()))
    })
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core`
Expected: PASS.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/curves/reeds_shepp.rs
git commit -m "feat(core): expose every Reeds-Shepp path, the shortest, and the smoothest"
```

---

### Task 9: Les propriétés

**Files:**
- Create: `crates/swept-core/tests/reeds_shepp_properties.rs`

**Interfaces:**
- Consumes: `swept_core::curves::reeds_shepp::{all, shortest, fewest_reversals}`
- Produces: rien. Cette tâche cherche ce que les cas nommés ont manqué.

**C'est la tâche qui trouve les défauts.** Au lot 2a, les cas nommés passaient
tous et les propriétés ont exhibé, à vingt mille tirages, un arc de 0,86 mm
dont l'omission déplaçait l'arrivée de 13 mm. Reeds-Shepp a deux fois plus de
familles et des conditions de signe plus nombreuses ; il faut s'attendre à ce
que cette tâche échoue au premier essai, et c'est son rôle.

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-core/tests/reeds_shepp_properties.rs` :

```rust
//! Properties every Reeds-Shepp path must satisfy, whatever the pose pair.
//!
//! The unit tests check named cases. These check the regimes nobody thought to
//! name — coincident poses, opposed headings, separations sitting exactly on
//! the thresholds where families appear and disappear. That is where closed
//! forms fail, by leaving the domain of an `acos` or dividing by a vanishing
//! term.

use proptest::prelude::*;
use std::f64::consts::TAU;
use swept_core::curves::reeds_shepp::{all, fewest_reversals, shortest};
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// How far apart the generated poses may sit, in metres.
///
/// ARBITRARY — wide enough to straddle every family threshold at the radii
/// generated below, narrow enough that close pairs keep coming up, which is
/// where the multi-arc families live.
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
                "x off by {} on {} segments",
                end.x - to.x,
                path.segments().len(),
            );
            prop_assert!((end.y - to.y).abs() < 1e-6, "y off by {}", end.y - to.y);
            let error = (end.heading.get() - to.heading.get()).rem_euclid(TAU);
            prop_assert!(error.min(TAU - error) < 1e-6, "heading off by {error}");
        }
    }

    /// A vehicle cannot turn tighter than its minimum radius.
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

    /// No length is ever a NaN or an infinity.
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

    /// A bounded-curvature path cannot beat the straight line between the two
    /// points — not even by reversing.
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

    /// Reeds-Shepp reaches every pose. Where Dubins can fail to apply, this
    /// must not: reversing is always available.
    #[test]
    fn some_path_always_exists(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        prop_assert!(!all(from, to, radius).is_empty());
    }

    /// `shortest` must be the minimum of `all`.
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
                for path in paths {
                    prop_assert!(best.length() <= path.length() + 1e-12);
                }
            }
        }
    }

    /// `fewest_reversals` must never be beaten on reversals.
    #[test]
    fn the_smoothest_has_the_fewest_reversals(
        from in any_pose(),
        to in any_pose(),
        radius in 1.0..8.0f64,
    ) {
        let paths = all(from, to, radius);
        if let Some(best) = fewest_reversals(from, to, radius) {
            for path in paths {
                prop_assert!(best.reversals() <= path.reversals());
            }
        }
    }
}

/// Coincident poses are the degenerate case worth naming: the separation is
/// zero and the line of sight is undefined. Nothing may panic.
#[test]
fn coincident_poses_do_not_panic() {
    let pose = Pose::new(3.0, -1.0, Radians::new(0.8));
    for path in all(pose, pose, 4.0) {
        assert!(path.length().is_finite());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --test reeds_shepp_properties`
Expected: probablement FAIL sur `every_path_lands_on_the_goal`, avec un contre-exemple
réduit par proptest. C'est le résultat utile de cette tâche.

Puis, pour chercher plus loin que les 256 tirages par défaut :

```bash
PROPTEST_CASES=20000 cargo test --release -p swept-core --test reeds_shepp_properties
```

- [ ] **Step 3: Write minimal implementation**

Corriger la famille que le contre-exemple désigne. La démarche :

1. Lire le mot du message d'échec — il nomme les steerings et le nombre de
   segments, ce qui identifie la famille.
2. Reproduire le cas en test unitaire nommé, dans `reeds_shepp.rs`, avec les
   coordonnées exactes du contre-exemple.
3. Corriger la formule, puis **garder ce test nommé** : un contre-exemple
   trouvé par tirage doit être gelé, sinon il repart au premier remaniement.
4. Committer le fichier `.proptest-regressions` que proptest écrit à côté du
   test, comme le lot 2a l'a fait pour Dubins.

Si aucune propriété n'échoue même à 20 000 tirages, ne rien changer et le dire
dans la PR : c'est une information, pas une absence de résultat.

- [ ] **Step 4: Run test to verify it passes**

Run: `PROPTEST_CASES=20000 cargo test --release -p swept-core --test reeds_shepp_properties`
Expected: PASS.

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/tests/reeds_shepp_properties.rs crates/swept-core/src/curves/reeds_shepp.rs
git add crates/swept-core/tests/reeds_shepp_properties.proptest-regressions 2>/dev/null || true
git commit -m "test(core): check Reeds-Shepp paths against generated pose pairs"
```

---

### Task 10: Documentation

**Files:**
- Modify: `docs/ALGORITHME.md`

**Interfaces:**
- Consumes: tout ce qui précède
- Produces: rien.

- [ ] **Step 1: Write the failing test**

Le test est `#![deny(missing_docs)]` plus `cargo doc`.

Run: `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
Expected: aucun warning — c'est ce que fait le CI.

- [ ] **Step 2: Run test to verify it fails**

Si un élément public manque sa documentation, la commande ci-dessus échoue en
nommant le fichier et la ligne. La corriger avant d'aller plus loin.

- [ ] **Step 3: Write minimal implementation**

Dans `docs/ALGORITHME.md`, ajouter après la section 6 sur Dubins :

```markdown
## 6 bis. Les courbes de Reeds-Shepp

Reeds et Shepp (1990) étendent Dubins au véhicule qui peut reculer. Le plus
court chemin entre deux poses à courbure bornée est alors l'un de quarante-huit
mots, bâtis sur douze familles fondamentales — toutes en forme close.

Deux choses les distinguent de Dubins, et les deux comptent ici.

**Elles atteignent tout.** Dubins peut échouer à relier deux poses proches et
mal orientées ; en reculant, Reeds-Shepp y arrive toujours. C'est ce qui en
fait le bon outil pour un portail étroit, où le véhicule est précisément proche
et mal orienté.

**Elles minimisent aussi le nombre de changements de sens** — la grandeur que
ce projet appelle une manœuvre et que l'utilisateur compte. Le mot le plus
court et le mot le plus lisse ne sont pas le même : le noyau expose donc les
deux, `shortest` et `fewest_reversals`, en plus de `all`.

### Quarante-huit mots, huit fonctions

Écrire quarante-huit formes closes serait quarante-huit occasions de se
tromper de signe. Deux involutions suffisent à les engendrer, appliquées à
l'entrée plutôt qu'à la sortie :

- le **retournement du temps** parcourt le chemin à l'envers, ce qui nie `x` et
  le cap, et échange marche avant et marche arrière ;
- la **réflexion** échange la gauche et la droite, ce qui nie `y` et le cap.

Huit fonctions de base, quatre lectures chacune, et le compte y est.

### Ce qui les vérifie

Les formules se transcrivent mal, et les publications se contredisent — le lot
2a l'avait déjà constaté pour Dubins sur `LSR` et `LRL`. Chaque famille est
donc validée en intégrant son résultat par le modèle cinématique et en
regardant où il tombe. Ce test ne dépend d'aucune source et tranche tout
désaccord.

Ce module ne fait encore rien d'autre qu'exister : le lot 2c-2 le branchera
dans le planificateur comme expansion analytique, et s'en servira pour la
réduction par raccourcis.
```

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add docs/ALGORITHME.md
git commit -m "docs: describe the Reeds-Shepp families and what verifies them"
```

---

## Ce que ce lot ne fait pas

- **Il ne branche rien.** `multi.rs` n'est pas touché, `exact.rs` non plus.
  Aucun résultat existant ne change, ce qui est vérifiable : `git diff main`
  ne doit toucher que `curves/`, `tests/` et `docs/`.
- Pas d'expansion analytique dans l'A\*, pas de réduction par raccourcis : lot
  2c-2.
- Pas de fonction de coût modifiée. Le terme de marge du lot précédent reste
  tel quel.

## Vérification finale du lot

- [ ] `just ci` passe.
- [ ] `PROPTEST_CASES=20000 cargo test --release -p swept-core` passe.
- [ ] `git diff main --stat` ne touche que `crates/swept-core/src/curves/`,
      `crates/swept-core/tests/` et `docs/ALGORITHME.md`. Aucun fichier de
      `swept-solver`, `swept-wasm` ou `web/`.
- [ ] `swept-core` n'a toujours **aucune dépendance de production** :
      `cargo tree -p swept-core --edges normal` ne montre que la crate.
- [ ] Chaque famille a son test d'atterrissage, et aucun seuil n'a été relâché
      au-dessus de `1e-9`.
- [ ] Tout contre-exemple trouvé par proptest a été gelé en test nommé **et**
      dans le fichier `.proptest-regressions` commité.
