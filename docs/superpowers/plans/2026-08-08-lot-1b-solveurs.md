# Lot 1b — Solveurs — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Porter les trois solveurs du prototype — recherche exacte à un mouvement, chaussée minimale, planificateur multi-manœuvres — dans une crate `swept-solver` déterministe et testée.

**Architecture:** Une crate qui dépend de `swept-core` et ignore tout du Web. Elle n'a pas d'horloge : les budgets s'expriment en nœuds explorés, jamais en millisecondes, si bien que les mêmes entrées donnent toujours le même résultat. Chaque résultat porte sa provenance, `Exact` ou `Heuristic`.

**Tech Stack:** Rust 1.97.1 (édition 2024), `proptest` en dépendance de développement uniquement.

## Global Constraints

- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms de tests, noms de branches, messages de commit. Seule la documentation projet (`docs/`) reste en français.
- `#![deny(missing_docs)]`, `clippy` pedantic traité comme erreur, `cargo fmt` conforme, `cargo doc` sans avertissement.
- **Aucune constante numérique nue.** Chaque valeur reprise du prototype devient une `const` nommée et documentée par sa provenance. Celles que rien ne justifie sont marquées `ARBITRARY — carried over from the prototype, to be revalidated`.
- **Le noyau n'a pas d'horloge.** Aucun appel à `std::time` dans `swept-solver`. L'annulation par l'utilisateur relèvera du Worker au lot 1c.
- Longueurs en mètres, angles en `Radians`.
- `swept-solver` est sous **AGPL-3.0**, contrairement à `swept-core`.
- Une seule PR ouverte à la fois, branchée sur `main`. Chaque tâche est une PR.
- Sur ce poste, `cp` est aliasé en interactif et `node` n'est pas dans le `PATH` des shells non interactifs — utiliser `git checkout --` ou Python pour restaurer un fichier, et le chemin absolu de Node.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-solver/Cargo.toml` | Paquet AGPL, dépend de `swept-core`, `proptest` en dev |
| `crates/swept-solver/src/lib.rs` | Modules, doc de crate |
| `crates/swept-solver/src/result.rs` | `Confidence`, `DirectedPose`, `Maneuver`, `Outcome` |
| `crates/swept-solver/src/budget.rs` | `Discretisation`, `SearchBudget`, trait `Progress` |
| `crates/swept-solver/src/path.rs` | Trajectoires candidates avant et arrière, évaluation |
| `crates/swept-solver/src/exact.rs` | Balayage exhaustif à un mouvement |
| `crates/swept-solver/src/min_road.rs` | Dichotomie sur la largeur de chaussée |
| `crates/swept-solver/src/landing.rs` | Manœuvre d'atterrissage dans le passage |
| `crates/swept-solver/src/multi.rs` | A\* hybride sur `(x, y, θ, sens)` |
| `crates/swept-solver/tests/reference_results.rs` | Les trois premiers résultats de référence du `CLAUDE.md` |
| `crates/swept-solver/tests/invariants.rs` | Propriétés vérifiées par `proptest` |

---

### Task 1: La crate et ses types de résultat

**Files:**
- Create: `crates/swept-solver/Cargo.toml`, `crates/swept-solver/src/lib.rs`, `crates/swept-solver/src/result.rs`
- Modify: `Cargo.toml` (membre du workspace)

**Interfaces:**
- Consumes: `swept_core::kinematics::{Direction, Pose}`
- Produces: `result::Confidence` (`Exact` | `Heuristic { budget_exhausted: bool }`) ; `result::DirectedPose { pose: Pose, direction: Direction }` ; `result::Maneuver { poses: Vec<DirectedPose>, min_clearance: f64, moves: u8, confidence: Confidence }` avec `Maneuver::is_exact() -> bool` ; `result::Outcome` (`Found(Vec<Maneuver>)` | `NotFound { budget_exhausted: bool }`) avec `Outcome::best() -> Option<&Maneuver>`

- [ ] **Step 1: Déclarer le paquet**

Fichier `crates/swept-solver/Cargo.toml` :

```toml
[package]
name = "swept-solver"
version = "0.1.0"
description = "Manoeuvre search for swept path analysis"
license = "AGPL-3.0-only"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
repository.workspace = true

[dependencies]
swept-core = { path = "../swept-core" }

[dev-dependencies]
proptest = "1.11.0"

[lints]
workspace = true
```

Ajouter le membre dans le `Cargo.toml` racine :

```toml
members = ["crates/swept-core", "crates/swept-solver"]
```

- [ ] **Step 2: Écrire le test qui échoue**

Fichier `crates/swept-solver/src/result.rs` :

```rust
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
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p swept-solver`
Expected: FAIL — `cannot find type Maneuver in this scope`.

- [ ] **Step 4: Implémenter**

Fichier `crates/swept-solver/src/lib.rs` :

```rust
//! Manoeuvre search: can this vehicle get through this opening, and how.
//!
//! Three solvers live here. [`exact`] sweeps every one-move approach on a
//! grid and is therefore trustworthy in both directions: when it finds
//! nothing, nothing exists on that grid. [`multi`] plans up to four moves with
//! a hybrid A\*, and is trustworthy in one direction only — what it finds is
//! verified, what it fails to find may still exist. [`min_road`] bisects on
//! carriageway width.
//!
//! # No clock
//!
//! Nothing here reads wall-clock time. Budgets are counted in expanded nodes,
//! so the same inputs always produce the same output. The prototype stopped
//! after 2.2 seconds per depth and returned whatever the machine had managed
//! to reach by then, which made its results impossible to test.

pub mod result;
```

En tête de `crates/swept-solver/src/result.rs` :

```rust
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
```

Déclarer le module dans `lib.rs` (déjà fait au Step 4).

- [ ] **Step 5: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: trois tests passent, aucun avertissement.

- [ ] **Step 6: Commiter**

```bash
git checkout -b feat/solver-crate
git add Cargo.toml Cargo.lock crates/swept-solver/
git commit -m "feat(solver): crate skeleton and result types

Confidence is carried by every result rather than tracked alongside it:
CLAUDE.md forbids showing a clearance without saying where it came from,
and a type makes that impossible to forget. Finding nothing is an Outcome
variant, not an error, which preserves the difference between no solution
found and no solution exists."
```

---

### Task 2: Budget, discrétisation et progression

Le prototype mélange deux limites : un plafond de nœuds (`expanded < budget`) et une échéance en millisecondes (`Date.now() > deadline`). Seule la première survit.

**Files:**
- Create: `crates/swept-solver/src/budget.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `swept_core::units::Radians`
- Produces: `budget::Discretisation { primitive_length, heading_step, position_step, sample_step }` avec `Discretisation::default()` et `Discretisation::prototype()` ; `budget::SearchBudget { max_nodes: u32, max_solutions: u16, discretisation: Discretisation }` avec `SearchBudget::default()` ; le trait `budget::Progress` avec `fn nodes_expanded(&mut self, moves: u8, expanded: u32)` ; `budget::Silent` qui l'implémente sans rien faire

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-solver/src/budget.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_grid_is_finer_than_the_prototype() {
        let now = Discretisation::default();
        let before = Discretisation::prototype();
        assert!(now.primitive_length < before.primitive_length);
        assert!(now.heading_step.get() < before.heading_step.get());
        assert!(now.position_step < before.position_step);
    }

    #[test]
    fn the_visit_grid_stays_finer_than_a_whole_primitive() {
        // Otherwise two states a full move apart collapse into one cell and
        // the planner discards one of them.
        let d = Discretisation::default();
        assert!(
            d.position_step * 2.0 < d.primitive_length,
            "position step {} is too coarse for primitives of {}",
            d.position_step,
            d.primitive_length
        );
    }

    #[test]
    fn silent_progress_accepts_reports_and_does_nothing() {
        let mut silent = Silent;
        silent.nodes_expanded(3, 12_000);
    }

    #[test]
    fn counting_progress_records_the_last_report() {
        #[derive(Default)]
        struct Counter {
            calls: u32,
            last: u32,
        }
        impl Progress for Counter {
            fn nodes_expanded(&mut self, _moves: u8, expanded: u32) {
                self.calls += 1;
                self.last = expanded;
            }
        }

        let mut counter = Counter::default();
        counter.nodes_expanded(2, 100);
        counter.nodes_expanded(2, 250);
        assert_eq!((counter.calls, counter.last), (2, 250));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver budget`
Expected: FAIL — `cannot find type Discretisation in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/budget.rs` :

```rust
//! What bounds a search, and how it reports on itself.
//!
//! Budgets are counted in expanded nodes, never in elapsed time. That is what
//! makes a search reproducible: the prototype gave up after 2.2 seconds per
//! depth, so its answer depended on the machine it ran on and could not be
//! asserted in a test.

use swept_core::units::Radians;

/// How finely the planner chops space and motion.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Discretisation {
    /// Length of one motion primitive, in metres. Shorter primitives find
    /// tighter solutions and cost more nodes.
    pub primitive_length: f64,
    /// Heading resolution of the visited-state grid.
    pub heading_step: Radians,
    /// Position resolution of the visited-state grid, in metres.
    pub position_step: f64,
    /// Spacing between poses sampled along a segment for collision checks, in
    /// metres.
    pub sample_step: f64,
}

impl Default for Discretisation {
    /// The target grid from `CLAUDE.md`: 20 cm primitives and a 1° heading
    /// step.
    ///
    /// The position step is not simply scaled down with the rest. It has to
    /// stay well finer than one primitive, or two states a whole move apart
    /// land in the same cell and the planner drops one of them. The prototype
    /// held that ratio at a fifth; a third is kept here, which is more
    /// cautious.
    fn default() -> Self {
        Self {
            primitive_length: 0.20,
            heading_step: Radians::from_degrees(1.0),
            position_step: 0.06,
            sample_step: 0.04,
        }
    }
}

impl Discretisation {
    /// The prototype's own grid, kept so that the cost of refining it can be
    /// measured rather than guessed.
    #[must_use]
    pub fn prototype() -> Self {
        Self {
            primitive_length: 0.90,
            heading_step: Radians::from_degrees(6.0),
            position_step: 0.18,
            sample_step: 0.18,
        }
    }
}

/// Node ceiling for one planning depth.
///
/// ARBITRARY — carried over from the prototype (`index.html:809`), which
/// allowed 18 000 nodes per depth alongside a 2.2 second deadline. Only the
/// node count survives.
pub const DEFAULT_MAX_NODES: u32 = 18_000;

/// How many landing solutions are collected before a depth stops early.
///
/// ARBITRARY — carried over from the prototype (`index.html:496`).
pub const DEFAULT_MAX_SOLUTIONS: u16 = 14;

/// What bounds one planning run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SearchBudget {
    /// Largest number of nodes the planner may expand per depth.
    pub max_nodes: u32,
    /// How many landings to collect before settling for the best so far.
    pub max_solutions: u16,
    /// How finely to chop space and motion.
    pub discretisation: Discretisation,
}

impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_MAX_NODES,
            max_solutions: DEFAULT_MAX_SOLUTIONS,
            discretisation: Discretisation::default(),
        }
    }
}

/// Receives progress reports from a running search.
///
/// This is an output port: the solver knows nothing about workers or
/// messages, it only says how far it has got.
pub trait Progress {
    /// Called periodically with the depth being searched and the number of
    /// nodes expanded so far.
    fn nodes_expanded(&mut self, moves: u8, expanded: u32);
}

/// A [`Progress`] that discards everything, for callers that do not care.
#[derive(Debug, Clone, Copy, Default)]
pub struct Silent;

impl Progress for Silent {
    fn nodes_expanded(&mut self, _moves: u8, _expanded: u32) {}
}
```

Déclarer dans `lib.rs` :

```rust
pub mod budget;
```

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: sept tests passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-budget
git add crates/swept-solver/src/
git commit -m "feat(solver): node-counted budget, tunable grid, progress port

The prototype bounded its planner with both a node ceiling and a 2.2 second
deadline; only the node ceiling survives, because a wall-clock cutoff makes
the result depend on the machine and cannot be asserted in a test.

The position step is not scaled proportionally with the primitive length: it
must stay well finer than one primitive, or two states a whole move apart
collapse into the same visited cell."
```

---

### Task 3: Trajectoires candidates

Porte `deep()`, `pathForward()`, `pathReverse()` et `evaluate()` (`prototype/index.html:326-357`).

**Files:**
- Create: `crates/swept-solver/src/path.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `swept_core::{clearance::{Clearance, ClearanceField}, kinematics::{Pose, sample_arc}, scene::{GateKind, Scene}, vehicle::Vehicle}`
- Produces: `path::entry_depth(scene, vehicle) -> f64` ; `path::forward_path(vehicle, scene, radius, lateral_y, entry_x, step) -> Vec<Pose>` ; `path::reverse_path(vehicle, scene, radius, entry_x, setback, approach_angle, step) -> Option<Vec<Pose>>` ; `path::evaluate(poses, field) -> Option<f64>` rendant la marge minimale, `None` en cas de collision

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-solver/src/path.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;
    use swept_core::units::Radians;

    fn wide_scene() -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -2.50, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: 2.50, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn the_entry_depth_clears_the_posts_and_the_whole_vehicle() {
        let depth = entry_depth(&wide_scene(), &lbx());
        // Post depth, plus the vehicle ahead of its rear axle, plus a margin.
        assert!(depth > 0.55 + 2.580 + 0.850, "got {depth}");
    }

    #[test]
    fn a_swinging_gate_pushes_the_entry_depth_back_by_a_leaf() {
        let mut scene = wide_scene();
        let sliding = entry_depth(&scene, &lbx());
        scene.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.10,
            hinge_offset: 0.05,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        };
        assert!((entry_depth(&scene, &lbx()) - sliding - 1.15).abs() < 1e-9);
    }

    #[test]
    fn a_forward_path_starts_on_the_road_and_ends_in_the_yard() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let poses = forward_path(&vehicle, &scene, 5.2, -2.0, 0.0, 0.1);
        let first = poses.first().expect("a path has poses");
        let last = poses.last().expect("a path has poses");

        assert!(first.y < 0.0, "starts on the road, got y={}", first.y);
        assert!(first.heading.get().abs() < 1e-9, "starts along the road");
        assert!(
            last.y >= entry_depth(&scene, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.y
        );
        assert!(
            (last.heading.to_degrees() - 90.0).abs() < 1e-6,
            "ends pointing into the yard"
        );
    }

    #[test]
    fn a_reverse_path_that_never_reaches_the_road_is_rejected() {
        let (scene, vehicle) = (wide_scene(), lbx());
        // A very tight radius with no setback cannot swing far enough out.
        assert!(reverse_path(&vehicle, &scene, 5.2, 0.0, 0.0, Radians::default(), 0.1).is_none());
    }

    #[test]
    fn evaluating_a_clear_path_returns_its_tightest_point() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses = forward_path(&vehicle, &scene, 5.2, -2.0, 0.0, 0.1);
        match evaluate(&poses, &field) {
            Some(margin) => assert!(margin >= 0.0 && margin.is_finite(), "got {margin}"),
            None => panic!("a 5 m opening admits this approach"),
        }
    }

    #[test]
    fn evaluating_a_path_through_a_wall_returns_nothing() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        // Drive straight along the wall line rather than through the opening.
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, 0.15, Radians::default()))
            .collect();
        assert_eq!(evaluate(&poses, &field), None);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver path`
Expected: FAIL — `cannot find function entry_depth in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/path.rs` :

```rust
//! The one-move approaches a search tries, and how they are scored.
//!
//! Each candidate is a fixed shape: a run-up along the road, a quarter turn
//! at a chosen radius, and a straight push into the yard. Sweeping the radius,
//! the lateral start position and the entry point covers the useful space of
//! single-move approaches.

use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::{Pose, sample_arc};
use swept_core::scene::{GateKind, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use std::f64::consts::FRAC_PI_2;

/// Extra depth required beyond the vehicle itself before an entry counts as
/// complete, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:326`).
pub const ENTRY_CLEARANCE_M: f64 = 0.6;

/// Length of the straight run-up before the turn, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:331`). Long
/// enough that the approach starts well clear of the opening.
pub const RUN_UP_M: f64 = 5.0;

/// How far a reverse path must reach beyond the pavement to count as having
/// rejoined the road, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:343`).
pub const REVERSE_EXIT_MARGIN_M: f64 = 0.2;

/// How deep into the yard the vehicle must be for the entry to be complete.
///
/// Behind the posts, behind the gate leaves when they swing into the way, and
/// far enough that the whole vehicle is through.
#[must_use]
pub fn entry_depth(scene: &Scene, vehicle: &Vehicle) -> f64 {
    let gate_depth = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth)
        + gate_depth
        + vehicle.wheelbase
        + vehicle.front_overhang
        + ENTRY_CLEARANCE_M
}

/// Builds a forward approach: run up along the road, turn, push in.
///
/// `radius` is the turning radius held through the quarter turn, `lateral_y`
/// the distance out from the kerb the run-up is driven at, and `entry_x` where
/// the turn is aimed along the opening.
#[must_use]
pub fn forward_path(
    vehicle: &Vehicle,
    scene: &Scene,
    radius: f64,
    lateral_y: f64,
    entry_x: f64,
    step: f64,
) -> Vec<Pose> {
    let start = Pose::new(entry_x - radius - RUN_UP_M, lateral_y, Radians::default());
    let mut poses = vec![start];

    poses.extend(sample_arc(start, 0.0, RUN_UP_M, step));
    let before_turn = *poses.last().expect("the run-up produced poses");

    let curvature = 1.0 / radius;
    poses.extend(sample_arc(before_turn, curvature, radius * FRAC_PI_2, step));
    let after_turn = *poses.last().expect("the turn produced poses");

    let needed = entry_depth(scene, vehicle);
    if after_turn.y < needed {
        poses.extend(sample_arc(after_turn, 0.0, needed - after_turn.y, step));
    }
    poses
}

/// Builds a reverse approach, read forwards.
///
/// The path is generated from the finished position outwards — that is the
/// only end whose pose is known — then reversed, so the caller always sees a
/// path that starts on the road.
///
/// Returns `None` when the swing never reaches back out to the road, which
/// means this combination of radius and setback cannot be driven.
#[must_use]
pub fn reverse_path(
    vehicle: &Vehicle,
    scene: &Scene,
    radius: f64,
    entry_x: f64,
    setback: f64,
    approach_angle: Radians,
    step: f64,
) -> Option<Vec<Pose>> {
    let top = entry_depth(scene, vehicle);
    let start = Pose::new(entry_x, top, Radians::new(-FRAC_PI_2));
    let mut poses = vec![start];

    if setback > 0.0 {
        poses.extend(sample_arc(start, 0.0, setback, step));
    }
    let before_turn = *poses.last().expect("the setback produced poses");

    let curvature = 1.0 / radius;
    let sweep = (-approach_angle.get() + FRAC_PI_2) / curvature;
    poses.extend(sample_arc(before_turn, curvature, sweep, step));
    let after_turn = *poses.last().expect("the turn produced poses");

    if after_turn.y > -scene.pavement_width - REVERSE_EXIT_MARGIN_M {
        return None;
    }

    poses.extend(sample_arc(after_turn, 0.0, RUN_UP_M, step));
    poses.reverse();
    Some(poses)
}

/// Scores a path: its tightest clearance, or `None` if it collides anywhere.
#[must_use]
pub fn evaluate(poses: &[Pose], field: &ClearanceField) -> Option<f64> {
    let mut smallest = f64::MAX;
    for pose in poses {
        match field.at(*pose) {
            Clearance::Collision => return None,
            Clearance::Clear(margin) => smallest = smallest.min(margin),
        }
    }
    (smallest < f64::MAX).then_some(smallest)
}
```

Déclarer dans `lib.rs` :

```rust
pub mod path;
```

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: treize tests passent.

Si `a_reverse_path_that_never_reaches_the_road_is_rejected` échoue, la géométrie du virage arrière est à revoir avant d'aller plus loin : c'est le seul garde-fou de `reverse_path`.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-paths
git add crates/swept-solver/src/
git commit -m "feat(solver): candidate approach paths and their scoring

Ports deep(), pathForward(), pathReverse() and evaluate()
(index.html:326-357). A reverse path is generated from the finished
position outwards, since that is the only end whose pose is known, then
reversed so callers always receive a path starting on the road."
```

---

### Task 4: Recherche exacte à un mouvement

Porte `search()` (`prototype/index.html:359-395`). C'est la référence : exhaustive sur sa grille, donc un échec y prouve quelque chose.

**Files:**
- Create: `crates/swept-solver/src/exact.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `path::{forward_path, reverse_path, evaluate, entry_depth}` (Task 3), `result::{Confidence, DirectedPose, Maneuver, Outcome}` (Task 1), `budget::Discretisation` (Task 2)
- Produces: `exact::Grid { radius_steps, lateral_steps, entry_steps, angle_steps, setback_steps }` avec `Grid::fine()` et `Grid::coarse()` ; `exact::Approach` (`Forward` | `Reverse`) ; `exact::search(vehicle, scene, approach, grid) -> Outcome`

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-solver/src/exact.rs`, section de test :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;

    fn scene_with_opening(width: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -width / 2.0, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: width / 2.0, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_generous_opening_admits_a_forward_entry() {
        let outcome = search(&lbx(), &scene_with_opening(5.0), Approach::Forward, Grid::fine());
        let best = outcome.best().expect("5 m is plenty");
        assert_eq!(best.moves, 1);
        assert!(best.min_clearance > 0.0);
    }

    #[test]
    fn every_exact_result_says_it_is_exact() {
        let outcome = search(&lbx(), &scene_with_opening(5.0), Approach::Forward, Grid::fine());
        assert!(outcome.best().expect("a solution").is_exact());
    }

    #[test]
    fn an_opening_narrower_than_the_mirrors_admits_nothing() {
        // The LBX measures 2.029 m over its mirrors.
        let outcome = search(&lbx(), &scene_with_opening(1.6), Approach::Forward, Grid::fine());
        match outcome {
            Outcome::NotFound { budget_exhausted } => assert!(
                !budget_exhausted,
                "an exhaustive sweep never runs out of budget"
            ),
            Outcome::Found(_) => panic!("the vehicle cannot fit through 1.6 m"),
        }
    }

    #[test]
    fn a_wider_opening_is_never_tighter_than_a_narrower_one() {
        let vehicle = lbx();
        let narrow = search(&vehicle, &scene_with_opening(3.0), Approach::Forward, Grid::fine());
        let wide = search(&vehicle, &scene_with_opening(4.0), Approach::Forward, Grid::fine());
        if let (Some(n), Some(w)) = (narrow.best(), wide.best()) {
            assert!(
                w.min_clearance >= n.min_clearance - 1e-9,
                "3 m gave {}, 4 m gave {}",
                n.min_clearance,
                w.min_clearance
            );
        }
    }

    #[test]
    fn the_returned_path_is_actually_collision_free() {
        let (vehicle, scene) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &scene, Approach::Forward, Grid::fine());
        let best = outcome.best().expect("a solution");
        let field = ClearanceField::new(&scene, &vehicle);
        for step in &best.poses {
            assert_ne!(field.at(step.pose), Clearance::Collision);
        }
    }

    #[test]
    fn a_coarse_grid_visits_fewer_candidates_than_a_fine_one() {
        assert!(Grid::coarse().candidate_count(Approach::Forward)
            < Grid::fine().candidate_count(Approach::Forward));
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver exact`
Expected: FAIL — `cannot find function search in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/exact.rs` :

```rust
//! Exhaustive search for a one-move entry.
//!
//! Every combination on the grid is tried and the roomiest is kept. Because
//! the sweep is complete, a failure here means something: there is no one-move
//! entry *on this grid*. That is what makes this solver the reference the
//! planner is seeded from.

use crate::budget::Discretisation;
use crate::path::{evaluate, forward_path, reverse_path};
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use swept_core::clearance::ClearanceField;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Which way the vehicle drives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Approach {
    /// Driving in nose first.
    Forward,
    /// Backing in.
    Reverse,
}

/// Increment between the turning radii tried, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:368`).
pub const RADIUS_STEP_M: f64 = 0.5;

/// How far either side of the opening centre the turn is aimed, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:372`).
pub const ENTRY_SPAN_M: f64 = 0.9;

/// Clearance kept between the vehicle's widest point and the lane edges when
/// choosing where the run-up is driven, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:362`).
pub const LANE_MARGIN_M: f64 = 0.02;

/// Increment between the approach angles tried when reversing, in degrees.
///
/// ARBITRARY — carried over from the prototype (`index.html:382`).
pub const REVERSE_ANGLE_STEP_DEGREES: f64 = 8.0;

/// Increment between the setbacks tried when reversing, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:384`).
pub const REVERSE_SETBACK_STEP_M: f64 = 0.5;

/// How many values of each parameter the sweep tries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Turning radii, from the vehicle's tightest upwards.
    pub radius_steps: u16,
    /// Lateral positions across the carriageway.
    pub lateral_steps: u16,
    /// Entry points along the opening.
    pub entry_steps: u16,
    /// Approach angles, reversing only.
    pub angle_steps: u16,
    /// Setbacks before the turn, reversing only.
    pub setback_steps: u16,
}

impl Grid {
    /// The full sweep, used whenever the answer is shown to a user.
    #[must_use]
    pub fn fine() -> Self {
        Self {
            radius_steps: 12,
            lateral_steps: 18,
            entry_steps: 29,
            angle_steps: 5,
            setback_steps: 4,
        }
    }

    /// A cheaper sweep, for callers that run the search many times over —
    /// the carriageway bisection in particular.
    #[must_use]
    pub fn coarse() -> Self {
        Self {
            radius_steps: 8,
            lateral_steps: 12,
            entry_steps: 17,
            angle_steps: 5,
            setback_steps: 4,
        }
    }

    /// How many candidates this grid produces, useful for reporting cost.
    #[must_use]
    pub fn candidate_count(self, approach: Approach) -> u64 {
        let base = u64::from(self.radius_steps + 1) * u64::from(self.entry_steps + 1);
        match approach {
            Approach::Forward => base * u64::from(self.lateral_steps + 1),
            Approach::Reverse => {
                base * u64::from(self.angle_steps + 1) * u64::from(self.setback_steps + 1)
            }
        }
    }
}

/// Sweeps every one-move approach on `grid` and keeps the roomiest.
#[must_use]
pub fn search(vehicle: &Vehicle, scene: &Scene, approach: Approach, grid: Grid) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let step = Discretisation::default().sample_step;
    let half_width = vehicle.mirror_width / 2.0;

    // Where along the carriageway the run-up may be driven.
    let low = -scene.pavement_width - scene.road_width + half_width + LANE_MARGIN_M;
    let high = -half_width - LANE_MARGIN_M;
    if low > high {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    }

    let mut best: Option<(Vec<Pose>, f64)> = None;
    // A closure keeping `best` mutably borrowed across the sweep. If the
    // borrow checker objects, turn it into a free function taking
    // `&mut Option<(Vec<Pose>, f64)>` — the logic is unchanged.
    let mut consider = |poses: Vec<Pose>| {
        if let Some(margin) = evaluate(&poses, &field) {
            if best.as_ref().is_none_or(|(_, m)| margin > *m) {
                best = Some((poses, margin));
            }
        }
    };

    for i in 0..=grid.radius_steps {
        let radius = vehicle.min_turning_radius + f64::from(i) * RADIUS_STEP_M;
        for k in 0..=grid.entry_steps {
            let entry_x = -ENTRY_SPAN_M
                + 2.0 * ENTRY_SPAN_M * f64::from(k) / f64::from(grid.entry_steps);
            match approach {
                Approach::Forward => {
                    for j in 0..=grid.lateral_steps {
                        let lateral =
                            low + (high - low) * f64::from(j) / f64::from(grid.lateral_steps);
                        consider(forward_path(vehicle, scene, radius, lateral, entry_x, step));
                    }
                }
                Approach::Reverse => {
                    for a in 0..=grid.angle_steps {
                        let angle =
                            Radians::from_degrees(f64::from(a) * REVERSE_ANGLE_STEP_DEGREES);
                        for d in 0..=grid.setback_steps {
                            let setback = f64::from(d) * REVERSE_SETBACK_STEP_M;
                            if let Some(poses) =
                                reverse_path(vehicle, scene, radius, entry_x, setback, angle, step)
                            {
                                consider(poses);
                            }
                        }
                    }
                }
            }
        }
    }

    match best {
        None => Outcome::NotFound {
            // An exhaustive sweep has no budget to exhaust.
            budget_exhausted: false,
        },
        Some((poses, min_clearance)) => {
            let direction = match approach {
                Approach::Forward => Direction::Forward,
                Approach::Reverse => Direction::Reverse,
            };
            Outcome::Found(vec![Maneuver {
                poses: poses
                    .into_iter()
                    .map(|pose| DirectedPose { pose, direction })
                    .collect(),
                min_clearance,
                moves: 1,
                confidence: Confidence::Exact,
            }])
        }
    }
}
```

Déclarer dans `lib.rs` :

```rust
pub mod exact;
```

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: dix-neuf tests passent.

`a_wider_opening_is_never_tighter_than_a_narrower_one` est le test qui compte : il échoue si le balayage rate des candidats de façon dépendante de la scène.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-exact
git add crates/swept-solver/src/
git commit -m "feat(solver): exhaustive one-move search

Ports search() (index.html:359-395). Because the sweep is complete, a
failure is informative: there is no one-move entry on this grid. That is
what makes it the reference the planner will be seeded from, and why its
results carry Confidence::Exact."
```

---

### Task 5: Les trois premiers résultats de référence

Ces valeurs sont le seul savoir mesuré que le projet possède sur son propre domaine. Elles deviennent des tests de non-régression.

**Files:**
- Create: `crates/swept-solver/tests/reference_results.rs`

**Interfaces:**
- Consumes: `exact::{search, Approach, Grid}` (Task 4), `swept_core::clearance::ClearanceField`
- Produces: aucune API. Uniquement des tests.

- [ ] **Step 1: Écrire les tests**

Fichier `crates/swept-solver/tests/reference_results.rs` :

```rust
//! The reference results recorded in `CLAUDE.md`, as regression tests.
//!
//! These are measurements, not derivations. If one of them starts failing,
//! either the port drifted or the recorded value was wrong — decide which
//! before touching a tolerance.

use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::Pose;
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use swept_solver::exact::{Approach, Grid, search};

fn lbx() -> Vehicle {
    Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
}

fn scene(opening: f64, gate: GateKind) -> Scene {
    Scene {
        left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
        right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
        wall_thickness: 0.30,
        pavement_width: 1.20,
        dropped_kerb_width: 3.20,
        road_width: 4.50,
        gate,
    }
}

fn swinging(open_degrees: f64) -> GateKind {
    GateKind::Swinging {
        leaf_length: 1.15,
        leaf_thickness: 0.10,
        hinge_offset: 0.05,
        hinge_depth_ratio: 0.5,
        open_angle: Radians::from_degrees(open_degrees),
    }
}

/// How deep the constrained corridor runs, measured from the obstacles
/// themselves rather than assumed.
///
/// Keeps only the obstacles bordering the opening — the posts, and the gate
/// leaves when they swing into the way — and reports how far into the yard
/// they reach.
fn corridor_depth(scene: &Scene) -> f64 {
    let reach = scene.opening_width() / 2.0 + 1.0;
    scene
        .obstacles()
        .iter()
        .filter(|o| o.center.x.abs() < reach && o.center.y >= 0.0)
        .map(|o| o.center.y + o.half_height.max(o.half_width))
        .fold(0.0_f64, f64::max)
}

/// The widest span of approach angles that clears the opening, in degrees.
///
/// The vehicle is placed dead centre in the opening, at the depth where the
/// corridor is tightest, and rotated. This measures the corridor, not the
/// driver's skill.
fn angular_tolerance(scene: &Scene, vehicle: &Vehicle, depth: f64) -> f64 {
    let field = ClearanceField::new(scene, vehicle);
    let mut widest = 0.0_f64;
    let mut run = 0.0_f64;
    let mut degrees = 60.0;
    while degrees <= 120.0 {
        let pose = Pose::new(0.0, depth, Radians::from_degrees(degrees));
        if field.at(pose) == Clearance::Collision {
            run = 0.0;
        } else {
            run += 0.5;
            widest = widest.max(run);
        }
        degrees += 0.5;
    }
    widest
}

/// Reference 1: above `sqrt(w² + L²)`, every approach angle gets through;
/// below it, the angular tolerance collapses.
#[test]
fn critical_width_admits_every_approach_angle() {
    let vehicle = lbx();
    let corridor = 0.55_f64; // sliding gate: the posts alone
    let critical = (vehicle.mirror_width.powi(2) + corridor.powi(2)).sqrt();

    let generous = scene(critical + 0.30, GateKind::Sliding);
    let tolerance = angular_tolerance(&generous, &vehicle, corridor / 2.0);
    assert!(
        tolerance >= 55.0,
        "above the critical width the tolerance should span the sweep, got {tolerance}°"
    );

    let tight = scene(critical - 0.30, GateKind::Sliding);
    let collapsed = angular_tolerance(&tight, &vehicle, corridor / 2.0);
    assert!(
        collapsed < tolerance,
        "below the critical width the tolerance should collapse: {collapsed}° vs {tolerance}°"
    );
}

/// Reference 3: with a sliding gate the corridor is the post depth alone.
#[test]
fn a_sliding_gate_leaves_only_the_post_depth() {
    let depth = corridor_depth(&scene(2.40, GateKind::Sliding));
    assert!(
        (depth - 0.55).abs() < 0.01,
        "expected the 0.55 m post depth, got {depth}"
    );
}

/// Reference 2: swinging leaves open to 90° stretch the corridor to about one
/// leaf length, and the angular tolerance drops to roughly 4°.
#[test]
fn swinging_leaves_stretch_the_corridor_and_squeeze_the_tolerance() {
    let vehicle = lbx();
    let open = scene(2.40, swinging(90.0));

    let depth = corridor_depth(&open);
    assert!(
        depth >= 1.15 - 0.15,
        "the corridor should reach about a leaf length (1.15 m), got {depth}"
    );

    let tolerance = angular_tolerance(&open, &vehicle, depth / 2.0);
    assert!(
        tolerance <= 8.0,
        "the tolerance should be a handful of degrees, got {tolerance}°"
    );

    let sliding = angular_tolerance(&scene(2.40, GateKind::Sliding), &vehicle, 0.275);
    assert!(
        tolerance < sliding,
        "leaves must squeeze the tolerance: {tolerance}° with, {sliding}° without"
    );
}

/// The headline conclusion: extra moves buy no room, because the ceiling is
/// `(W - w) / 2` whatever the path.
#[test]
fn clearance_never_exceeds_the_geometric_ceiling() {
    let vehicle = lbx();
    for opening in [2.4, 3.0, 4.0, 5.0] {
        let ceiling = (opening - vehicle.mirror_width) / 2.0;
        let outcome = search(&vehicle, &scene(opening, GateKind::Sliding), Approach::Forward, Grid::fine());
        if let Some(best) = outcome.best() {
            assert!(
                best.min_clearance <= ceiling + 1e-6,
                "{opening} m opening: clearance {} exceeds the ceiling {ceiling}",
                best.min_clearance
            );
        }
    }
}
```

- [ ] **Step 2: Lancer et confronter**

Run: `cargo test -p swept-solver --test reference_results`
Expected: les quatre tests passent.

**Si l'un échoue, ne pas élargir la tolérance pour le faire passer.** Ouvrir `prototype/index.html` dans un navigateur, saisir les mêmes valeurs, et comparer. Trois issues possibles : le portage a dérivé, la mesure du plan ne mesure pas ce que le `CLAUDE.md` décrivait, ou la valeur enregistrée était fausse. Les trois demandent une décision, pas un ajustement de seuil. Consigner la conclusion dans le message de commit.

- [ ] **Step 3: Commiter**

```bash
git checkout -b test/reference-results
git add crates/swept-solver/tests/
git commit -m "test(solver): first three reference results from CLAUDE.md

Critical width sqrt(w² + L²), the post-depth corridor of a sliding gate,
and the few degrees of tolerance left by leaves open at 90 degrees. Also
pins the headline conclusion: clearance never exceeds (W - w) / 2, so extra
moves buy no room.

These are measurements, not derivations. A failure here means the port
drifted or the recorded value was wrong — never that the tolerance needs
widening."
```

---

### Task 6: Chaussée minimale

Porte `minRoad()` (`prototype/index.html:586-602`).

**Files:**
- Create: `crates/swept-solver/src/min_road.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `exact::{search, Approach, Grid}` (Task 4)
- Produces: `min_road::minimum_road_width(vehicle, scene) -> Option<f64>`, et les constantes `MIN_ROAD_SEARCH_LOW_M`, `MIN_ROAD_SEARCH_HIGH_M`, `MIN_ROAD_BISECTIONS`

- [ ] **Step 1: Écrire les tests qui échouent**

Section de test de `crates/swept-solver/src/min_road.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;

    fn scene(opening: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
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
    fn the_answer_actually_admits_an_entry_and_a_hair_less_does_not() {
        let (vehicle, base) = (lbx(), scene(4.0));
        let width = minimum_road_width(&vehicle, &base).expect("4 m admits an entry");

        let mut enough = base;
        enough.road_width = width + 0.05;
        assert!(
            search(&vehicle, &enough, Approach::Forward, Grid::coarse()).best().is_some(),
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
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver min_road`
Expected: FAIL — `cannot find function minimum_road_width in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/min_road.rs` :

```rust
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
/// Twelve halvings of a 16 m span land within 4 mm, well under the
/// centimetre this tool reports. Carried over from the prototype
/// (`index.html:589`).
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
```

Déclarer dans `lib.rs` :

```rust
pub mod min_road;
```

`GateKind` n'est utilisé que par les tests : l'importer dans `mod tests`, pas au niveau du module.

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: vingt-trois tests passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-min-road
git add crates/swept-solver/src/
git commit -m "feat(solver): minimum carriageway width by bisection

Ports minRoad() (index.html:586-602). Runs on the coarse grid because the
search is invoked a dozen times over and the answer is reported to the
centimetre; twelve halvings of a 16 m span land within 4 mm."
```

---

### Task 7: L'atterrissage

Porte `finalPush()` (`prototype/index.html:415-444`). C'est la manœuvre qui termine une planification : depuis un état quelconque, s'aligner et pousser dans le passage.

**Files:**
- Create: `crates/swept-solver/src/landing.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `path::{entry_depth, evaluate}` (Task 3), `budget::Discretisation` (Task 2)
- Produces: `landing::Landing { poses: Vec<Pose>, min_clearance: f64, direction: Direction }` ; `landing::land(from: Pose, vehicle: &Vehicle, scene: &Scene, field: &ClearanceField, allowed: Option<Direction>) -> Option<Landing>` ; `landing::landing_radii(vehicle) -> Vec<f64>`

- [ ] **Step 1: Écrire les tests qui échouent**

Section de test de `crates/swept-solver/src/landing.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::Post;

    fn scene(opening: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn radii_start_at_the_tightest_the_vehicle_can_hold() {
        let radii = landing_radii(&lbx());
        assert!(!radii.is_empty());
        let tightest = radii.iter().copied().fold(f64::MAX, f64::min);
        assert!((tightest - 5.2).abs() < 1e-9, "got {tightest}");
    }

    #[test]
    fn a_vehicle_squared_up_in_the_opening_lands_forwards() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        // Already pointing into the yard, just short of the wall.
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        let landing = land(from, &vehicle, &sc, &field, None).expect("a clear run in");
        assert_eq!(landing.direction, Direction::Forward);
        assert!(landing.min_clearance > 0.0);
    }

    #[test]
    fn a_landing_ends_past_the_entry_depth() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        let landing = land(from, &vehicle, &sc, &field, None).expect("a clear run in");
        let last = landing.poses.last().expect("a landing has poses");
        assert!(last.y >= entry_depth(&sc, &vehicle) - 1e-6, "got y={}", last.y);
    }

    #[test]
    fn restricting_the_direction_is_honoured() {
        let (vehicle, sc) = (lbx(), scene(5.0));
        let field = ClearanceField::new(&sc, &vehicle);
        let from = Pose::new(0.0, -2.0, Radians::from_degrees(90.0));
        if let Some(landing) = land(from, &vehicle, &sc, &field, Some(Direction::Reverse)) {
            assert_eq!(landing.direction, Direction::Reverse);
        }
    }

    #[test]
    fn a_vehicle_facing_away_from_a_narrow_opening_cannot_land() {
        let (vehicle, sc) = (lbx(), scene(2.0));
        let field = ClearanceField::new(&sc, &vehicle);
        // Pointing along the road, far off to the side.
        let from = Pose::new(-6.0, -3.0, Radians::default());
        assert!(land(from, &vehicle, &sc, &field, None).is_none());
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver landing`
Expected: FAIL — `cannot find function land in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/landing.rs` :

```rust
//! The move that finishes an entry.
//!
//! From any state, try to swing onto the opening's axis and push through.
//! Every candidate is checked against collisions before being returned, which
//! is why a planner result is trustworthy even though the planner itself is
//! heuristic.

use crate::budget::Discretisation;
use crate::path::{entry_depth, evaluate};
use swept_core::clearance::ClearanceField;
use swept_core::kinematics::{Direction, Pose, sample_arc};
use swept_core::scene::Scene;
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use std::f64::consts::{FRAC_PI_2, PI};

/// How many turning radii the landing tries, beyond the tightest.
///
/// ARBITRARY — carried over from the prototype (`index.html:459`).
pub const LANDING_RADIUS_COUNT: usize = 6;

/// Spacing between those radii, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:459`).
pub const LANDING_RADIUS_SPREAD_M: f64 = 1.1;

/// Longest swing allowed when lining up, in metres of arc.
///
/// Beyond this the manoeuvre stops resembling anything a driver would do.
/// ARBITRARY — carried over from the prototype (`index.html:432`).
pub const MAX_LANDING_ARC_M: f64 = 22.0;

/// A completed entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Landing {
    /// The poses of the landing move.
    pub poses: Vec<Pose>,
    /// Tightest clearance along it, in metres.
    pub min_clearance: f64,
    /// Which way the vehicle drives through.
    pub direction: Direction,
}

/// The turning radii a landing will try, tightest first.
#[must_use]
pub fn landing_radii(vehicle: &Vehicle) -> Vec<f64> {
    (0..LANDING_RADIUS_COUNT)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let offset = i as f64 * LANDING_RADIUS_SPREAD_M;
            vehicle.min_turning_radius + offset
        })
        .collect()
}

/// Normalises an angle into `(-π, π]`.
fn wrap(angle: f64) -> f64 {
    let mut a = angle;
    while a > PI {
        a -= 2.0 * PI;
    }
    while a <= -PI {
        a += 2.0 * PI;
    }
    a
}

/// Tries to finish the entry from `from`.
///
/// `allowed` restricts which direction may be used; `None` allows both. The
/// first collision-free landing found is returned — radii are tried tightest
/// first, so that is also the most compact one.
#[must_use]
pub fn land(
    from: Pose,
    vehicle: &Vehicle,
    scene: &Scene,
    field: &ClearanceField,
    allowed: Option<Direction>,
) -> Option<Landing> {
    let needed = entry_depth(scene, vehicle);
    let step = Discretisation::default().sample_step;

    for direction in [Direction::Forward, Direction::Reverse] {
        if allowed.is_some_and(|only| only != direction) {
            continue;
        }
        let target = match direction {
            Direction::Forward => FRAC_PI_2,
            Direction::Reverse => -FRAC_PI_2,
        };
        let turn = wrap(target - from.heading.get());

        for radius in landing_radii(vehicle) {
            for sign in [1.0, -1.0] {
                let mut poses = Vec::new();
                let mut at = from;

                if turn.abs() > 1e-4 {
                    let curvature = sign / radius;
                    let arc = turn / curvature;
                    // Forwards must swing forwards, reverse must swing back.
                    match direction {
                        Direction::Forward if arc <= 0.0 => continue,
                        Direction::Reverse if arc >= 0.0 => continue,
                        _ => {}
                    }
                    if arc.abs() > MAX_LANDING_ARC_M {
                        continue;
                    }
                    poses.extend(sample_arc(at, curvature, arc, step));
                    at = *poses.last().expect("the swing produced poses");
                } else if direction == Direction::Reverse {
                    continue;
                }

                if at.y >= needed {
                    continue;
                }
                let push = needed - at.y;
                let signed = match direction {
                    Direction::Forward => push,
                    Direction::Reverse => -push,
                };
                poses.extend(sample_arc(at, 0.0, signed, step));

                if let Some(min_clearance) = evaluate(&poses, field) {
                    return Some(Landing {
                        poses,
                        min_clearance,
                        direction,
                    });
                }
            }
        }
    }
    None
}
```

Déclarer dans `lib.rs` :

```rust
pub mod landing;
```

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: vingt-huit tests passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-landing
git add crates/swept-solver/src/
git commit -m "feat(solver): the move that finishes an entry

Ports finalPush() (index.html:415-444). Every candidate is checked against
collisions before being returned, which is what makes a planner result
trustworthy even though the planner itself is heuristic."
```

---

### Task 8: Le planificateur multi-manœuvres

Porte `planMulti()` (`prototype/index.html:457-533`). A\* hybride sur `(x, y, θ, sens)`.

Deux écarts délibérés avec le prototype. Le chaînage parent se fait par indices dans une arène, les références cycliques du JavaScript n'ayant pas d'équivalent simple en Rust. Et le tas est un `BinaryHeap` de nœuds ordonnés à l'envers, `f64` n'implémentant pas `Ord`.

**Files:**
- Create: `crates/swept-solver/src/multi.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `landing::land` (Task 7), `budget::{Progress, SearchBudget}` (Task 2), `result::{Confidence, DirectedPose, Maneuver, Outcome}` (Task 1)
- Produces: `multi::plan(vehicle, scene, max_moves, budget, progress, allowed) -> Outcome`

- [ ] **Step 1: Écrire les tests qui échouent**

Section de test de `crates/swept-solver/src/multi.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Silent;
    use swept_core::scene::Post;

    fn scene(opening: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_generous_opening_is_planned_within_a_few_moves() {
        let outcome = plan(&lbx(), &scene(4.0), 3, SearchBudget::default(), &mut Silent, None);
        let best = outcome.best().expect("4 m should be plannable");
        assert!(best.moves >= 1 && best.moves <= 3, "got {} moves", best.moves);
    }

    #[test]
    fn planner_results_never_claim_to_be_exact() {
        let outcome = plan(&lbx(), &scene(4.0), 3, SearchBudget::default(), &mut Silent, None);
        assert!(!outcome.best().expect("a plan").is_exact());
    }

    #[test]
    fn the_planned_path_is_actually_collision_free() {
        let (vehicle, sc) = (lbx(), scene(3.5));
        let outcome = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        if let Some(best) = outcome.best() {
            let field = ClearanceField::new(&sc, &vehicle);
            for step in &best.poses {
                assert_ne!(field.at(step.pose), Clearance::Collision);
            }
        }
    }

    #[test]
    fn the_same_inputs_always_give_the_same_result() {
        // The whole point of counting nodes instead of milliseconds.
        let (vehicle, sc) = (lbx(), scene(3.5));
        let once = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        let twice = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        assert_eq!(once, twice);
    }

    #[test]
    fn a_starved_budget_reports_that_it_ran_out() {
        let budget = SearchBudget {
            max_nodes: 5,
            ..SearchBudget::default()
        };
        let outcome = plan(&lbx(), &scene(2.2), 4, budget, &mut Silent, None);
        match outcome {
            Outcome::NotFound { budget_exhausted } => assert!(budget_exhausted),
            Outcome::Found(list) => {
                assert!(list.iter().all(|m| !m.is_exact()));
            }
        }
    }

    #[test]
    fn progress_is_reported_while_searching() {
        #[derive(Default)]
        struct Spy(u32);
        impl Progress for Spy {
            fn nodes_expanded(&mut self, _moves: u8, _expanded: u32) {
                self.0 += 1;
            }
        }
        let mut spy = Spy::default();
        plan(&lbx(), &scene(2.6), 3, SearchBudget::default(), &mut spy, None);
        assert!(spy.0 > 0, "the planner never reported progress");
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver multi`
Expected: FAIL — `cannot find function plan in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/multi.rs` :

```rust
//! Multi-move planning by hybrid A\*.
//!
//! Search runs over `(x, y, heading, direction)`. The dominant cost is the
//! number of direction changes — reversing is what a driver counts — with
//! distance as a tie-breaker.
//!
//! Unlike the prototype, nothing here consults a clock: the search stops when
//! it runs out of nodes, not when it runs out of time, so the same inputs
//! always yield the same plan.

use crate::budget::{Progress, SearchBudget};
use crate::landing::land;
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::{Direction, Pose, sample_arc};
use swept_core::scene::{GateKind, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashSet};
use std::f64::consts::FRAC_PI_2;

/// Cost charged for each direction change.
///
/// Dominant by design: a driver counts reverses, not metres. ARBITRARY in
/// magnitude — carried over from the prototype (`index.html:513`).
pub const MOVE_COST: f64 = 5.0;

/// Cost charged per metre travelled, as a tie-breaker.
///
/// ARBITRARY — carried over from the prototype (`index.html:513`).
pub const LENGTH_COST_PER_M: f64 = 0.18;

/// Weight given to heading error in the heuristic.
///
/// ARBITRARY — carried over from the prototype (`index.html:471`).
pub const HEADING_ERROR_WEIGHT: f64 = 2.2;

/// Where along the road the planner starts, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:474`).
pub const START_X_M: f64 = -6.5;

/// How many lateral start positions are seeded.
///
/// ARBITRARY — carried over from the prototype (`index.html:472`).
pub const START_POSITIONS: u8 = 10;

/// Distance from the opening centre within which a landing is attempted.
///
/// ARBITRARY — carried over from the prototype (`index.html:490`).
pub const LANDING_TRIGGER_X_M: f64 = 2.4;

/// Heading error within which a landing is attempted, in radians.
///
/// ARBITRARY — carried over from the prototype (`index.html:490`).
pub const LANDING_TRIGGER_HEADING_RAD: f64 = 1.0;

/// Fractions of the tightest turning radius used as motion primitives.
///
/// Hard left, gentle left, straight, gentle right, hard right. The 1.6
/// divisor is ARBITRARY — carried over from the prototype
/// (`index.html:461`).
pub const CURVATURE_FRACTIONS: [f64; 5] = [-1.0, -1.0 / 1.6, 0.0, 1.0 / 1.6, 1.0];

/// How often progress is reported, in expanded nodes.
const PROGRESS_EVERY: u32 = 500;

/// A node in the search, held in an arena and referred to by index.
#[derive(Debug, Clone)]
struct Node {
    pose: Pose,
    direction: Direction,
    moves: u8,
    travelled: f64,
    parent: Option<usize>,
    segment: Vec<Pose>,
}

/// A heap entry ordered so that `BinaryHeap` pops the cheapest score first.
#[derive(Debug, Clone, Copy)]
struct Ranked {
    score: f64,
    index: usize,
}

impl PartialEq for Ranked {
    fn eq(&self, other: &Self) -> bool {
        self.score == other.score
    }
}
impl Eq for Ranked {}
impl Ord for Ranked {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reversed: BinaryHeap is a max-heap, we want the smallest score.
        other
            .score
            .partial_cmp(&self.score)
            .unwrap_or(Ordering::Equal)
    }
}
impl PartialOrd for Ranked {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// How deep the vehicle must reach for the goal heuristic, in metres.
///
/// Shallower than the full entry depth: the heuristic only has to point the
/// search the right way. ARBITRARY — carried over from the prototype
/// (`index.html:413`).
const GOAL_MARGIN_M: f64 = 0.45;

fn goal_depth(scene: &Scene) -> f64 {
    let gate = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth) + gate + GOAL_MARGIN_M
}

/// Plans an entry in at most `max_moves` moves.
#[must_use]
pub fn plan(
    vehicle: &Vehicle,
    scene: &Scene,
    max_moves: u8,
    budget: SearchBudget,
    progress: &mut impl Progress,
    allowed: Option<Direction>,
) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let grid = budget.discretisation;
    let goal = goal_depth(scene);
    let half_width = vehicle.mirror_width / 2.0;

    let low = -scene.pavement_width - scene.road_width + half_width + 0.03;
    let high = -half_width - 0.05;
    if low > high {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    }

    let heuristic = |pose: &Pose| {
        pose.x.hypot((goal - pose.y).max(0.0))
            + HEADING_ERROR_WEIGHT * (FRAC_PI_2 - pose.heading.get()).abs()
    };

    let cell = |pose: &Pose, direction: Direction| {
        #[allow(clippy::cast_possible_truncation)]
        let x = (pose.x / grid.position_step).round() as i64;
        #[allow(clippy::cast_possible_truncation)]
        let y = (pose.y / grid.position_step).round() as i64;
        #[allow(clippy::cast_possible_truncation)]
        let h = (pose.heading.get() / grid.heading_step.get()).round() as i64;
        (x, y, h, direction == Direction::Forward)
    };

    let mut arena: Vec<Node> = Vec::new();
    let mut heap: BinaryHeap<Ranked> = BinaryHeap::new();
    let mut seen: HashSet<(i64, i64, i64, bool)> = HashSet::new();

    for j in 0..=START_POSITIONS {
        let y = low + (high - low) * f64::from(j) / f64::from(START_POSITIONS);
        let pose = Pose::new(START_X_M, y, Radians::default());
        if field.at(pose) == Clearance::Collision {
            continue;
        }
        let score = MOVE_COST + heuristic(&pose);
        seen.insert(cell(&pose, Direction::Forward));
        arena.push(Node {
            pose,
            direction: Direction::Forward,
            moves: 1,
            travelled: 0.0,
            parent: None,
            segment: Vec::new(),
        });
        heap.push(Ranked {
            score,
            index: arena.len() - 1,
        });
    }

    let mut expanded: u32 = 0;
    let mut best: Option<(usize, crate::landing::Landing)> = None;
    let mut solutions: u16 = 0;
    let mut exhausted = false;

    while let Some(Ranked { index, .. }) = heap.pop() {
        if expanded >= budget.max_nodes {
            exhausted = true;
            break;
        }
        expanded += 1;
        if expanded % PROGRESS_EVERY == 0 {
            progress.nodes_expanded(max_moves, expanded);
        }

        let (pose, direction, moves, travelled) = {
            let node = &arena[index];
            (node.pose, node.direction, node.moves, node.travelled)
        };

        let heading_error = (FRAC_PI_2 - pose.heading.get())
            .abs()
            .min((-FRAC_PI_2 - pose.heading.get()).abs());
        if pose.x.abs() < LANDING_TRIGGER_X_M && heading_error < LANDING_TRIGGER_HEADING_RAD {
            if let Some(landing) = land(pose, vehicle, scene, &field, allowed) {
                let better = best
                    .as_ref()
                    .is_none_or(|(_, b)| landing.min_clearance > b.min_clearance);
                if better {
                    best = Some((index, landing));
                }
                solutions += 1;
                if solutions >= budget.max_solutions {
                    break;
                }
            }
        }

        if moves > max_moves {
            continue;
        }

        for fraction in CURVATURE_FRACTIONS {
            for step_direction in [Direction::Forward, Direction::Reverse] {
                let next_moves = moves + u8::from(step_direction != direction);
                if next_moves > max_moves {
                    continue;
                }

                let curvature = fraction / vehicle.min_turning_radius;
                let signed = match step_direction {
                    Direction::Forward => grid.primitive_length,
                    Direction::Reverse => -grid.primitive_length,
                };
                let segment = sample_arc(pose, curvature, signed, grid.sample_step);
                if segment
                    .iter()
                    .any(|p| field.at(*p) == Clearance::Collision)
                {
                    continue;
                }
                let end = *segment.last().expect("a primitive produced poses");

                let key = cell(&end, step_direction);
                if !seen.insert(key) {
                    continue;
                }

                let next_travelled = travelled + grid.primitive_length;
                let score = f64::from(next_moves) * MOVE_COST
                    + next_travelled * LENGTH_COST_PER_M
                    + heuristic(&end);
                arena.push(Node {
                    pose: end,
                    direction: step_direction,
                    moves: next_moves,
                    travelled: next_travelled,
                    parent: Some(index),
                    segment,
                });
                heap.push(Ranked {
                    score,
                    index: arena.len() - 1,
                });
            }
        }
    }

    let Some((goal_index, landing)) = best else {
        return Outcome::NotFound {
            budget_exhausted: exhausted,
        };
    };

    // Walk the arena back to the seed, then unwind.
    let mut chain: Vec<&Node> = Vec::new();
    let mut cursor = Some(goal_index);
    while let Some(i) = cursor {
        chain.push(&arena[i]);
        cursor = arena[i].parent;
    }
    chain.reverse();

    let mut poses: Vec<DirectedPose> = Vec::new();
    for node in &chain {
        for pose in &node.segment {
            poses.push(DirectedPose {
                pose: *pose,
                direction: node.direction,
            });
        }
    }
    let last_direction = arena[goal_index].direction;
    for pose in &landing.poses {
        poses.push(DirectedPose {
            pose: *pose,
            direction: landing.direction,
        });
    }

    let min_clearance = poses
        .iter()
        .filter_map(|p| match field.at(p.pose) {
            Clearance::Clear(margin) => Some(margin),
            Clearance::Collision => None,
        })
        .fold(f64::MAX, f64::min);

    let extra = u8::from(landing.direction != last_direction);
    Outcome::Found(vec![Maneuver {
        poses,
        min_clearance: if min_clearance == f64::MAX {
            landing.min_clearance
        } else {
            min_clearance
        },
        moves: arena[goal_index].moves + extra,
        confidence: Confidence::Heuristic {
            budget_exhausted: exhausted,
        },
    }])
}
```

- [ ] **Step 4: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: trente-quatre tests passent.

`the_same_inputs_always_give_the_same_result` est le test qui justifie toute la conception : s'il échoue, c'est qu'une source de non-déterminisme s'est glissée dans le solveur.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/solver-multi
git add crates/swept-solver/src/
git commit -m "feat(solver): multi-move planning by hybrid A*

Ports planMulti() (index.html:457-533) with two deliberate departures. The
parent chain lives in an arena addressed by index, JavaScript's cyclic
references having no simple Rust equivalent. And the frontier is a
BinaryHeap of reverse-ordered entries, f64 not implementing Ord.

The 2.2 second deadline is gone: the search stops when it runs out of
nodes, which is what makes the same inputs give the same plan."
```

---

### Task 9: Amorçage et invariants

La propriété que le `CLAUDE.md` désigne comme acquise et à ne pas perdre : **le multi ne doit jamais être moins bon que le simple**. Elle ne tient que si la planification est amorcée par la recherche exacte.

**Files:**
- Create: `crates/swept-solver/src/solve.rs`, `crates/swept-solver/tests/invariants.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `exact::search` (Task 4), `multi::plan` (Task 8)
- Produces: `solve::alternatives(vehicle, scene, budget, progress, allowed) -> Outcome`, rendant une manœuvre par nombre de mouvements, de 1 à [`MAX_MOVES`]

- [ ] **Step 1: Écrire les tests qui échouent**

Section de test de `crates/swept-solver/src/solve.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::Silent;
    use swept_core::scene::Post;

    fn scene(opening: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn a_one_move_entry_is_returned_exactly() {
        let outcome = alternatives(&lbx(), &scene(5.0), SearchBudget::default(), &mut Silent, None);
        let best = outcome.best().expect("5 m admits a one-move entry");
        assert_eq!(best.moves, 1);
        assert!(best.is_exact(), "a one-move entry comes from the exact sweep");
    }

    #[test]
    fn alternatives_are_ordered_by_move_count_without_duplicates() {
        let outcome = alternatives(&lbx(), &scene(3.0), SearchBudget::default(), &mut Silent, None);
        if let Outcome::Found(list) = outcome {
            let moves: Vec<u8> = list.iter().map(|m| m.moves).collect();
            let mut sorted = moves.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(moves, sorted, "got {moves:?}");
        }
    }

    #[test]
    fn more_moves_never_yield_less_room_than_the_one_move_answer() {
        // The invariant CLAUDE.md calls out as acquired and not to be lost.
        let (vehicle, sc) = (lbx(), scene(3.2));
        let outcome = alternatives(&vehicle, &sc, SearchBudget::default(), &mut Silent, None);
        if let Outcome::Found(list) = outcome {
            if let Some(one) = list.iter().find(|m| m.moves == 1) {
                for other in list.iter().filter(|m| m.moves > 1) {
                    assert!(
                        other.min_clearance >= one.min_clearance - 1e-9,
                        "{} moves gave {} against {} for one move",
                        other.moves,
                        other.min_clearance,
                        one.min_clearance
                    );
                }
            }
        }
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-solver solve`
Expected: FAIL — `cannot find function alternatives in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-solver/src/solve.rs` :

```rust
//! The entry point callers actually use: every alternative, best first.
//!
//! Planning is always seeded by the exhaustive one-move search. That ordering
//! is not an optimisation — it is what guarantees the property `CLAUDE.md`
//! records as acquired: a multi-move answer is never worse than the one-move
//! answer, because the one-move answer is always among the candidates.

use crate::budget::{Progress, SearchBudget};
use crate::exact::{Approach, Grid, search};
use crate::multi::plan;
use crate::result::{Maneuver, Outcome};
use swept_core::kinematics::Direction;
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// Deepest plan offered.
///
/// Past four moves the answer stops being useful advice. ARBITRARY — carried
/// over from the prototype (`index.html:806`).
pub const MAX_MOVES: u8 = 4;

/// Every way in, one alternative per move count.
///
/// The one-move sweep runs first, in both directions unless `allowed`
/// restricts it. Deeper plans follow, and any that fails to beat the one-move
/// clearance is dropped rather than shown.
#[must_use]
pub fn alternatives(
    vehicle: &Vehicle,
    scene: &Scene,
    budget: SearchBudget,
    progress: &mut impl Progress,
    allowed: Option<Direction>,
) -> Outcome {
    let mut found: Vec<Maneuver> = Vec::new();
    let mut exhausted = false;

    for (approach, direction) in [
        (Approach::Forward, Direction::Forward),
        (Approach::Reverse, Direction::Reverse),
    ] {
        if allowed.is_some_and(|only| only != direction) {
            continue;
        }
        if let Outcome::Found(list) = search(vehicle, scene, approach, Grid::fine()) {
            found.extend(list);
            break;
        }
    }

    let one_move_clearance = found
        .iter()
        .filter(|m| m.moves == 1)
        .map(|m| m.min_clearance)
        .fold(f64::MIN, f64::max);

    for depth in 2..=MAX_MOVES {
        match plan(vehicle, scene, depth, budget, progress, allowed) {
            Outcome::Found(list) => {
                for candidate in list {
                    // Never present a deeper plan that is worse than the exact
                    // one-move answer.
                    if one_move_clearance > f64::MIN
                        && candidate.min_clearance < one_move_clearance
                    {
                        continue;
                    }
                    match found.iter_mut().find(|m| m.moves == candidate.moves) {
                        Some(existing) if candidate.min_clearance > existing.min_clearance => {
                            *existing = candidate;
                        }
                        Some(_) => {}
                        None => found.push(candidate),
                    }
                }
            }
            Outcome::NotFound { budget_exhausted } => exhausted |= budget_exhausted,
        }
    }

    if found.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: exhausted,
        };
    }
    found.sort_by_key(|m| m.moves);
    Outcome::Found(found)
}
```

Déclarer dans `lib.rs` :

```rust
pub mod solve;
```

- [ ] **Step 4: Ajouter les tests de propriétés**

Fichier `crates/swept-solver/tests/invariants.rs` :

```rust
//! Properties that must hold whatever the scene.
//!
//! These cover the solvers, whose behaviour is allowed to change as the grid
//! is refined — unlike the geometry primitives, which are pinned to golden
//! vectors.

use proptest::prelude::*;
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::exact::{Approach, Grid, search};
use swept_solver::solve::alternatives;

fn scene(opening: f64, post_depth: f64, road: f64) -> Scene {
    Scene {
        left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: post_depth },
        right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: post_depth },
        wall_thickness: 0.30,
        pavement_width: 1.20,
        dropped_kerb_width: 3.20,
        road_width: road,
        gate: GateKind::Sliding,
    }
}

fn lbx() -> Vehicle {
    Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(24))]

    /// Whatever comes back is drivable: no pose along it touches anything.
    #[test]
    fn returned_paths_never_collide(
        opening in 2.0_f64..6.0,
        post_depth in 0.2_f64..0.9,
        road in 3.0_f64..8.0,
    ) {
        let (vehicle, sc) = (lbx(), scene(opening, post_depth, road));
        if let Some(best) = search(&vehicle, &sc, Approach::Forward, Grid::coarse()).best() {
            let field = ClearanceField::new(&sc, &vehicle);
            for step in &best.poses {
                prop_assert_ne!(field.at(step.pose), Clearance::Collision);
            }
        }
    }

    /// The reported clearance is the one the path actually has.
    #[test]
    fn reported_clearance_matches_the_path(
        opening in 2.5_f64..6.0,
        road in 3.5_f64..8.0,
    ) {
        let (vehicle, sc) = (lbx(), scene(opening, 0.55, road));
        if let Some(best) = search(&vehicle, &sc, Approach::Forward, Grid::coarse()).best() {
            let field = ClearanceField::new(&sc, &vehicle);
            let actual = best.poses.iter()
                .filter_map(|s| match field.at(s.pose) {
                    Clearance::Clear(m) => Some(m),
                    Clearance::Collision => None,
                })
                .fold(f64::MAX, f64::min);
            prop_assert!((actual - best.min_clearance).abs() < 1e-9);
        }
    }

    /// Clearance can never exceed the geometric ceiling, whatever the path.
    #[test]
    fn clearance_stays_under_the_ceiling(opening in 2.2_f64..6.0) {
        let vehicle = lbx();
        let sc = scene(opening, 0.55, 4.5);
        let ceiling = (opening - vehicle.mirror_width) / 2.0;
        if let Some(best) = alternatives(&vehicle, &sc, SearchBudget::default(), &mut Silent, None).best() {
            prop_assert!(best.min_clearance <= ceiling + 1e-6);
        }
    }

    /// A wider opening never admits less room than a narrower one.
    #[test]
    fn wider_is_never_tighter(opening in 2.5_f64..5.0, extra in 0.1_f64..1.0) {
        let vehicle = lbx();
        let narrow = search(&vehicle, &scene(opening, 0.55, 4.5), Approach::Forward, Grid::coarse());
        let wide = search(&vehicle, &scene(opening + extra, 0.55, 4.5), Approach::Forward, Grid::coarse());
        if let (Some(n), Some(w)) = (narrow.best(), wide.best()) {
            prop_assert!(w.min_clearance >= n.min_clearance - 1e-9);
        }
    }
}
```

- [ ] **Step 5: Vérifier**

Run: `cargo test -p swept-solver && cargo clippy --all-targets -- -D warnings && cargo fmt --check`
Expected: tous les tests passent, y compris les quatre propriétés.

Si `proptest` exhibe un contre-exemple, il l'écrit dans `crates/swept-solver/proptest-regressions/`. **Commiter ce fichier** : c'est un cas limite trouvé par la machine, il doit rejouer à chaque exécution.

- [ ] **Step 6: Commiter**

```bash
git checkout -b feat/solver-alternatives
git add crates/swept-solver/
git commit -m "feat(solver): seed planning with the exact sweep, pin the invariants

Planning is always seeded by the exhaustive one-move search, and any deeper
plan that fails to beat its clearance is dropped. That ordering is what
guarantees the property CLAUDE.md records as acquired: multi is never worse
than simple.

Adds proptest coverage for the properties the solvers must hold whatever
the scene — paths never collide, reported clearance matches the path, and
clearance never exceeds the (W - w) / 2 ceiling."
```

---

### Task 10: Mesurer le coût de la grille fine

Le risque principal identifié par la spec. Passer de 90 cm et 6° à 20 cm et 1° multiplie fortement l'espace de recherche ; la réponse prévue en cas de dépassement est un raffinement progressif, pas un retour à une grille grossière.

**Files:**
- Create: `crates/swept-solver/benches/grid_cost.rs` ou `crates/swept-solver/examples/grid_cost.rs`
- Modify: `docs/ALGORITHME.md` (créé ici), `crates/swept-solver/src/budget.rs` si la mesure impose un ajustement

**Interfaces:**
- Consumes: `multi::plan` (Task 8), `budget::{Discretisation, SearchBudget}` (Task 2)
- Produces: aucune API. Une mesure et une décision consignée.

- [ ] **Step 1: Écrire le programme de mesure**

Fichier `crates/swept-solver/examples/grid_cost.rs` :

```rust
//! Measures what refining the planning grid costs.
//!
//! Run with `cargo run -p swept-solver --release --example grid_cost`.
//!
//! The prototype planned on 90 cm primitives with a 6° heading step. CLAUDE.md
//! calls for 20 cm and 1°. This reports whether the node budget still suffices
//! at that resolution, on scenes tight enough to make the planner work.

use swept_core::scene::{GateKind, Post, Scene};
use swept_core::vehicle::Vehicle;
use swept_solver::budget::{Discretisation, Progress, SearchBudget};
use swept_solver::multi::plan;
use swept_solver::result::Outcome;

#[derive(Default)]
struct Counter(u32);
impl Progress for Counter {
    fn nodes_expanded(&mut self, _moves: u8, expanded: u32) {
        self.0 = expanded;
    }
}

fn scene(opening: f64) -> Scene {
    Scene {
        left_post: Post { inner_edge_x: -opening / 2.0, width: 0.55, depth: 0.55 },
        right_post: Post { inner_edge_x: opening / 2.0, width: 0.55, depth: 0.55 },
        wall_thickness: 0.30,
        pavement_width: 1.20,
        dropped_kerb_width: 3.20,
        road_width: 4.50,
        gate: GateKind::Sliding,
    }
}

fn main() {
    let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle");

    println!("{:<10} {:<12} {:>10} {:>12} {:>10}", "grid", "opening", "nodes", "clearance", "moves");
    for (name, discretisation) in [
        ("prototype", Discretisation::prototype()),
        ("fine", Discretisation::default()),
    ] {
        for opening in [2.2, 2.6, 3.0, 4.0] {
            let budget = SearchBudget {
                discretisation,
                ..SearchBudget::default()
            };
            let mut counter = Counter::default();
            let outcome = plan(&vehicle, &scene(opening), 4, budget, &mut counter, None);
            let (clearance, moves) = match outcome.best() {
                Some(m) => (format!("{:.3}", m.min_clearance), m.moves.to_string()),
                None => ("none".to_string(), "-".to_string()),
            };
            let exhausted = matches!(outcome, Outcome::NotFound { budget_exhausted: true });
            println!(
                "{name:<10} {opening:<12.2} {:>10} {clearance:>12} {moves:>10}{}",
                counter.0,
                if exhausted { "  (budget exhausted)" } else { "" }
            );
        }
    }
}
```

- [ ] **Step 2: Mesurer**

Run: `cargo run -p swept-solver --release --example grid_cost`
Expected: un tableau comparant les deux grilles.

Lire le résultat et décider :

- **Si la grille fine trouve des solutions dans le budget** : rien à changer, consigner les chiffres.
- **Si elle épuise systématiquement le budget** : ne pas revenir à la grille grossière. Relever `DEFAULT_MAX_NODES` si le temps reste acceptable, ou implémenter un raffinement progressif — planifier d'abord sur `Discretisation::prototype()`, puis raffiner localement autour de la solution trouvée. Créer une tâche dédiée plutôt que de bricoler ici.
- **Si la grille fine trouve strictement moins que la grossière** : c'est un bug, pas un arbitrage. La grille fine explore un sur-ensemble ; trouver moins signale une erreur dans la clé de grille ou l'heuristique.

- [ ] **Step 3: Écrire le narratif**

Créer `docs/ALGORITHME.md`, en français — c'est de la documentation projet — avec exactement ces sections :

1. **Le repère et les unités.** Origine au milieu du passage, `y = 0` au nu extérieur du mur, `y > 0` vers la cour. Mètres et radians.
2. **Le modèle de véhicule.** Bicyclette, état réduit à la pose de l'essieu arrière, enveloppe échantillonnée en quatorze points, les rétroviseurs comme point le plus large.
3. **La détection de collision.** Les deux tests et pourquoi les deux sont nécessaires : le test direct des points d'enveloppe contre les obstacles, et le test inverse des coins d'obstacle dans la caisse, avec le cas du coin de pilier qu'aucun point échantillonné n'atteint.
4. **L'intégration cinématique.** La formule à courbure constante, le cas dégénéré de la ligne droite, et pourquoi l'échantillonnage atterrit exactement sur le point d'arrivée.
5. **Les trois solveurs.** Ce que chacun prouve : le balayage exhaustif prouve l'absence sur sa grille, l'A\* ne prouve rien en cas d'échec, la dichotomie hérite de la garantie du premier. Expliquer `Confidence` et pourquoi il est porté par le type.
6. **Pourquoi il n'y a pas d'horloge.** Le budget en nœuds, et ce que le prototype perdait en s'arrêtant au chronomètre.
7. **Les résultats de référence.** Les quatre valeurs mesurées et ce qu'elles impliquent, dont la conclusion principale : le plafond vaut `(W − w) / 2` quelle que soit la trajectoire, donc multiplier les manœuvres n'achète pas de marge.
8. **Coût de la grille.** Le tableau relevé au Step 2 et la décision prise.
9. **Dette assumée.** Le tableau de toutes les constantes marquées `ARBITRARY`, avec pour chacune sa valeur, son origine dans le prototype, et ce qu'il faudrait mesurer pour la justifier.

La section 9 se construit mécaniquement :

```bash
grep -rn "ARBITRARY" crates/*/src/ | sed 's/:.*ARBITRARY/ →/'
```

- [ ] **Step 4: Commiter**

```bash
git checkout -b docs/algorithm-and-grid-cost
git add crates/swept-solver/examples/ docs/ALGORITHME.md
git commit -m "docs: narrative walkthrough and measured cost of the fine grid

Records what refining the planning grid from 90 cm / 6 degrees to 20 cm /
1 degree actually costs in expanded nodes, so the choice rests on a
measurement rather than an assumption. Also lists every constant carried
over from the prototype that nothing justifies, as acknowledged debt."
```

---

## Ce que ce plan ne fait pas

À l'issue de la Task 10, le noyau sait répondre à la question du projet mais rien ne l'expose : ni WebAssembly, ni worker, ni interface. C'est le **lot 1c**, qui portera aussi les six véhicules en dur du prototype et fera tomber les deux derniers critères d'acceptation du lot 1 — l'onglet qui ne gèle plus, et le déploiement Vercel.

L'enveloppe d'usage et les trois verdicts de `docs/SPEC.md` restent au lot 3 : ce plan ne rend qu'une marge de collision, sans jamais se prononcer sur le fait qu'on puisse ouvrir sa portière une fois garé.
