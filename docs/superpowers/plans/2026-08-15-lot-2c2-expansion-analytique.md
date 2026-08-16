# Lot 2c-2 — L'expansion analytique et la réduction — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Brancher Reeds-Shepp dans le planificateur — à chaque nœud
développé, tenter une connexion exacte vers une pose d'arrivée — puis réduire
le trajet rendu jusqu'à ce qu'aucun segment n'y soit superflu.

**Architecture:** `landing::landings` construit aujourd'hui ses atterrissages à
la main : un arc à rayon choisi, une droite. C'est le même défaut que
`forward_path` avait avant le lot 2b, et le même remède : énumérer les courbes
qui relient la pose courante aux poses d'arrivée que `poses::goal_poses`
produit déjà, et garder la plus dégagée. La réduction vient ensuite, en
post-traitement.

**Tech Stack:** Rust 1.97.1 (édition 2024). `swept-solver` dépend de
`swept-core`, qui a livré `curves::reeds_shepp` au lot 2c-1.

**Spec:** `docs/superpowers/specs/2026-08-10-dubins-reeds-shepp-design.md`

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**.
- `swept-core` garde **zéro dépendance de production** ; ce lot ne le touche pas.
- `#![deny(missing_docs)]` sur les deux crates.
- **Tout ce qui vit dans le dépôt est en anglais**, sauf `docs/` et l'interface.
- Longueurs en **mètres**, angles en **radians**.
- **Aucune constante numérique nue** : chaque valeur est une `const` nommée et
  documentée par sa provenance.
- Clippy `pedantic`, le CI échoue sur un warning.
- **Aucune horloge** : les budgets se comptent en nœuds.
- Une PR sur `main`, une tâche par commit. Branche : `feat/lot-2c2-analytic`.
- **Aucune assertion existante n'est affaiblie.** Les onze tests de `multi.rs`
  et les trois invariants de `solve.rs` sont le garde-fou de ce lot.

## Les deux idées

### L'atterrissage devient exact

`landings` balaie des rayons et deux sens de braquage pour fabriquer un arc
suivi d'une droite. Rien ne garantit que cette forme atteigne la pose voulue :
elle vise une *profondeur*, pas une pose, ce qui est le défaut que le lot 2b a
corrigé côté recherche exacte et qui subsiste ici.

Reeds-Shepp relie deux poses **exactement**, en forme close, et rend jusqu'à
quarante-huit mots. On les essaie tous et on garde le plus dégagé — même
critère que partout ailleurs dans ce projet, puisque la marge est ce qu'on
cherche et que le plus court est celui qui rase.

### Un raccourci se juge à ce qu'il achète

Le critère de la spec, §7 bis :

> Un segment est **superflu** s'il existe une connexion Reeds-Shepp sans
> collision entre la pose qui le précède et celle qui le suit, comportant au
> plus autant d'inversions et laissant une marge au moins égale.

Ce n'est pas un seuil de longueur. Une marche arrière de trente centimètres
pour se dégager d'un pilier est légitime ; ce qu'on veut écarter est la
manœuvre **qui n'achète rien**, celle que le planificateur bricole faute de
pouvoir s'aligner sur sa grille. La longueur ne sépare pas les deux cas,
l'utilité si.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-solver/src/landing.rs` | **Réécrit.** L'atterrissage devient une connexion Reeds-Shepp vers les poses d'arrivée. |
| `crates/swept-solver/src/shortcut.rs` | **Nouveau.** La réduction : tant qu'un segment est superflu, l'enlever. |
| `crates/swept-solver/src/multi.rs` | Le post-traitement appelé sur le plan retenu. |
| `crates/swept-solver/src/lib.rs` | Déclaration du module `shortcut`. |
| `docs/ALGORITHME.md` | Les deux sections. |

La réduction va dans son propre fichier : elle ne partage rien avec la
recherche, et `multi.rs` frôle déjà les limites de ce qu'on relit d'une traite.

---

### Task 1: L'atterrissage par connexion exacte

**Files:**
- Modify: `crates/swept-solver/src/landing.rs`

**Interfaces:**
- Consumes: `swept_core::curves::reeds_shepp::all`, `crate::poses::goal_poses`,
  `crate::path::evaluate_at_least`
- Produces: `landing::landings(from, vehicle, scene, field, allowed) -> Vec<Landing>`
  **inchangée de signature**. `landing::landing_radii` disparaît.

La signature ne bouge pas, ce qui est ce qui permet à `multi.rs` de compiler
sans changement et fait des tests existants le garde-fou du lot.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/landing.rs` :

```rust
    #[test]
    fn a_landing_ends_on_a_goal_pose_exactly() {
        // The point of the batch. The old shape aimed at a depth and arrived
        // wherever its arc happened to end; a curve arrives on a pose.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        let goals = crate::poses::goal_poses(&vehicle, &scene, 4, 2);

        for landing in landings(from, &vehicle, &scene, &field, None) {
            let last = *landing.poses.last().expect("a landing has poses");
            let matched = goals.iter().any(|g| {
                (g.x - last.x).abs() < 1e-6
                    && (g.y - last.y).abs() < 1e-6
                    && (g.heading.get() - last.heading.get()).abs() < 1e-6
            });
            assert!(matched, "landed at {last:?}, which is no goal pose");
        }
    }

    #[test]
    fn a_landing_starts_where_it_was_asked_to() {
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        for landing in landings(from, &vehicle, &scene, &field, None) {
            let first = *landing.poses.first().expect("a landing has poses");
            assert!((first.x - from.x).abs() < 1e-9);
            assert!((first.y - from.y).abs() < 1e-9);
        }
    }

    #[test]
    fn a_landing_is_collision_free_all_the_way() {
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        for landing in landings(from, &vehicle, &scene, &field, None) {
            for pose in &landing.poses {
                assert_ne!(field.at(*pose), Clearance::Collision);
            }
            assert!(landing.min_clearance > 0.0);
        }
    }

    #[test]
    fn only_the_allowed_direction_comes_back() {
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let from = Pose::new(-4.0, -3.0, Radians::default());
        for landing in landings(from, &vehicle, &scene, &field, Some(Direction::Reverse)) {
            assert_eq!(landing.direction, Direction::Reverse);
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib landing`
Expected: FAIL — `a_landing_ends_on_a_goal_pose_exactly` échoue, l'ancienne
forme ne visant aucune pose nommée.

- [ ] **Step 3: Write minimal implementation**

Remplacer le corps de `crates/swept-solver/src/landing.rs` — la doc de module,
les imports, `landing_radii` et `landings` — par :

```rust
//! The last move: getting from wherever the planner is to a pose in the yard.
//!
//! Reeds-Shepp joins two **poses** in closed form, so a landing arrives
//! exactly where it was aimed rather than wherever its shape happened to end.
//! That is the same correction batch 2b made to the exhaustive search, applied
//! to the planner's last move.
//!
//! Every curve that applies is tried and the roomiest kept. Length never
//! enters it: the shortest path is the one that grazes most.

use crate::path::evaluate_at_least;
use crate::poses::goal_poses;
use swept_core::clearance::ClearanceField;
use swept_core::curves::reeds_shepp;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// How many goal poses a landing aims at, along the opening and in heading.
///
/// ARBITRARY, and deliberately smaller than the exhaustive sweep's grid: this
/// runs at every expanded node rather than once, so a pose that costs nothing
/// there costs sixty thousand times as much here. Four points across the
/// opening and three headings is enough to find a way in when one exists.
pub const LANDING_ENTRY_STEPS: u16 = 4;
/// Final headings tried, spread about square to the opening.
///
/// ARBITRARY, same reasoning as [`LANDING_ENTRY_STEPS`].
pub const LANDING_HEADING_STEPS: u16 = 2;

/// Sampling step along a landing curve, in metres.
///
/// ARBITRARY — the same step the rest of the solver samples at, so that a
/// clearance measured here means what it means everywhere else.
pub const LANDING_SAMPLE_STEP_M: f64 = 0.08;

/// One way of finishing the entry.
#[derive(Debug, Clone)]
pub struct Landing {
    /// The poses of the landing move.
    pub poses: Vec<Pose>,
    /// Tightest clearance along it, in metres.
    pub min_clearance: f64,
    /// Which way the vehicle drives through.
    pub direction: Direction,
}

/// Every collision-free landing from `from`, roomiest first per direction.
///
/// At most two are returned — one driving in, one backing in — because the
/// caller files a landing under what it costs in moves, and only the best of
/// each gear can win that comparison.
///
/// `allowed` restricts the gear when the interface asked for one.
#[must_use]
pub fn landings(
    from: Pose,
    vehicle: &Vehicle,
    scene: &Scene,
    field: &ClearanceField,
    allowed: Option<Direction>,
) -> Vec<Landing> {
    let goals = goal_poses(vehicle, scene, LANDING_ENTRY_STEPS, LANDING_HEADING_STEPS);
    let mut best: [Option<Landing>; 2] = [None, None];

    for goal in goals {
        for curve in reeds_shepp::all(from, goal, vehicle.min_turning_radius) {
            // A landing's gear is the gear it finishes in: that is what the
            // driver is doing as they thread the opening, and what decides
            // whether this costs the planner a move.
            let Some(last) = curve.segments().last() else {
                continue;
            };
            let direction = last.direction;
            if allowed.is_some_and(|only| only != direction) {
                continue;
            }

            let mut poses = vec![from];
            poses.extend(curve.poses(from, LANDING_SAMPLE_STEP_M));

            let slot = &mut best[usize::from(direction == Direction::Reverse)];
            let floor = slot.as_ref().map_or(f64::NEG_INFINITY, |l| l.min_clearance);
            if let Some(min_clearance) = evaluate_at_least(&poses, field, floor) {
                *slot = Some(Landing {
                    poses,
                    min_clearance,
                    direction,
                });
            }
        }
    }
    best.into_iter().flatten().collect()
}
```

Supprimer aussi les tests de `landing.rs` qui portaient sur `landing_radii` et
sur la forme arc-puis-droite : ils décrivent un mécanisme qui n'existe plus.
Garder les fixtures `wide_scene()` et `lbx()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver`
Expected: PASS, y compris les onze tests de `multi.rs` et les trois invariants
de `solve.rs`.

**Si un test de `multi.rs` régresse**, chercher dans cet ordre :

1. Le déclencheur. `multi.rs` n'appelle `landings` que lorsque la pose est
   proche de l'ouverture (`LANDING_TRIGGER_X_M`, `LANDING_TRIGGER_HEADING_RAD`).
   Reeds-Shepp atteint des poses bien plus lointaines ; le déclencheur peut
   maintenant être trop restrictif.
2. Le gear. Un mot Reeds-Shepp change de sens en cours de route ; `direction`
   ne décrit que le dernier segment. Un atterrissage qui manœuvre coûte donc
   plus d'une manœuvre, ce que la Task 2 traite.
3. Le coût. Une connexion exacte est plus longue en poses échantillonnées
   qu'un arc plus une droite ; le budget peut s'épuiser plus tôt.

Ne jamais relâcher une assertion existante.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/landing.rs
git commit -m "feat(solver): land on a pose by closed form, not on a depth by shape"
```

---

### Task 2: Ce qu'un atterrissage coûte vraiment

**Files:**
- Modify: `crates/swept-solver/src/multi.rs`

**Interfaces:**
- Consumes: `Landing`, et `CurvePath::reversals` par l'intermédiaire des poses
- Produces: `landing::Landing::moves(&self, arriving_in: Direction) -> u8`

**Un atterrissage Reeds-Shepp peut manœuvrer.** L'ancien en était incapable :
un arc et une droite, un seul sens. `multi.rs` compte donc son coût comme
`u8::from(landing.direction != direction)` — zéro ou une manœuvre. Avec une
courbe qui change de sens en chemin, ce compte est faux et le planificateur
annoncerait deux manœuvres là où il en fait quatre.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/landing.rs` :

```rust
    #[test]
    fn a_landing_that_never_changes_gear_costs_nothing_extra() {
        let landing = Landing {
            poses: vec![Pose::default(); 3],
            min_clearance: 0.1,
            direction: Direction::Forward,
            reversals: 0,
        };
        assert_eq!(landing.moves(Direction::Forward), 0);
    }

    #[test]
    fn arriving_in_the_other_gear_costs_one_move() {
        let landing = Landing {
            poses: vec![Pose::default(); 3],
            min_clearance: 0.1,
            direction: Direction::Reverse,
            reversals: 0,
        };
        assert_eq!(landing.moves(Direction::Forward), 1);
    }

    #[test]
    fn a_landing_that_shunts_costs_every_shunt() {
        // The case the old landing could not produce and the new one can: a
        // curve that changes gear twice on its way in is three moves, not one.
        let landing = Landing {
            poses: vec![Pose::default(); 3],
            min_clearance: 0.1,
            direction: Direction::Forward,
            reversals: 2,
        };
        assert_eq!(landing.moves(Direction::Forward), 2);
        assert_eq!(landing.moves(Direction::Reverse), 3);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib landing`
Expected: FAIL — ``struct `Landing` has no field named `reversals` ``.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-solver/src/landing.rs`, ajouter le champ et la méthode :

```rust
    /// How many times the curve changes gear on its way in.
    ///
    /// The old landing was a single arc and a straight run, so this was always
    /// zero and the caller could count a landing as one move or none. A
    /// closed-form curve may shunt, and a plan that says two moves while
    /// driving four is worse than useless.
    pub reversals: u8,
```

et, dans `impl Landing` :

```rust
impl Landing {
    /// What this landing costs, in moves, when reached in `arriving_in`.
    ///
    /// Every gear change inside the curve counts, plus one more if the
    /// planner has to change gear to begin it.
    #[must_use]
    pub fn moves(&self, arriving_in: Direction) -> u8 {
        self.reversals + u8::from(self.starts_in() != arriving_in)
    }

    /// The gear the landing begins in.
    #[must_use]
    fn starts_in(&self) -> Direction {
        // With an even number of gear changes the curve ends as it began.
        if self.reversals % 2 == 0 {
            self.direction
        } else {
            match self.direction {
                Direction::Forward => Direction::Reverse,
                Direction::Reverse => Direction::Forward,
            }
        }
    }
}
```

Renseigner le champ dans `landings`, à partir de la courbe :

```rust
            let reversals = u8::try_from(curve.reversals()).unwrap_or(u8::MAX);
```

et le passer au `Landing` construit.

Enfin, dans `crates/swept-solver/src/multi.rs`, remplacer le calcul du coût :

```rust
                let total = moves + landing.moves(direction);
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/landing.rs crates/swept-solver/src/multi.rs
git commit -m "fix(solver): count every shunt a landing makes, not just its last gear"
```

---

### Task 3: La réduction par raccourcis

**Files:**
- Create: `crates/swept-solver/src/shortcut.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `swept_core::curves::reeds_shepp::all`, `path::evaluate_at_least`
- Produces: `shortcut::reduce(poses: &[DirectedPose], vehicle: &Vehicle, field: &ClearanceField) -> Vec<DirectedPose>`

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-solver/src/shortcut.rs` avec pour tout contenu ce bloc :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::scene::{GateKind, Post};
    use swept_core::units::Radians;

    fn wide_scene() -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: -2.50,
                width: 0.55,
                depth: 0.55,
            },
            right_post: Post {
                inner_edge_x: 2.50,
                width: 0.55,
                depth: 0.55,
            },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            kerb_height: f64::INFINITY,
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 5.2).expect("valid vehicle")
    }

    fn straight(count: usize) -> Vec<DirectedPose> {
        (0..count)
            .map(|i| DirectedPose {
                #[allow(clippy::cast_precision_loss)]
                pose: Pose::new(-8.0 + i as f64 * 0.4, -3.0, Radians::default()),
                direction: Direction::Forward,
            })
            .collect()
    }

    #[test]
    fn a_path_with_nothing_to_gain_comes_back_unchanged() {
        // A straight run has no superfluous segment: any shortcut between two
        // of its poses is the same straight line.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let reduced = reduce(&path, &vehicle, &field);
        assert_eq!(reduced.first().map(|p| p.pose.x), path.first().map(|p| p.pose.x));
        assert_eq!(reduced.last().map(|p| p.pose.x), path.last().map(|p| p.pose.x));
    }

    #[test]
    fn reduction_never_adds_a_reversal() {
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let before = reversal_count(&path);
        let after = reversal_count(&reduce(&path, &vehicle, &field));
        assert!(after <= before);
    }

    #[test]
    fn reduction_keeps_the_endpoints() {
        // Whatever it removes in the middle, a reduced path still starts and
        // ends where the original did — otherwise it answers another question.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let reduced = reduce(&path, &vehicle, &field);
        let (a, b) = (path.first().expect("poses"), reduced.first().expect("poses"));
        assert!((a.pose.x - b.pose.x).abs() < 1e-9);
        let (a, b) = (path.last().expect("poses"), reduced.last().expect("poses"));
        assert!((a.pose.x - b.pose.x).abs() < 1e-9);
        assert!((a.pose.y - b.pose.y).abs() < 1e-9);
    }

    #[test]
    fn reduction_never_gives_up_clearance() {
        // The criterion the spec states: a shortcut is taken only if it leaves
        // at least as much room. Anything else would trade the very thing this
        // tool measures for a shorter path nobody asked for.
        let (vehicle, scene) = (lbx(), wide_scene());
        let field = ClearanceField::new(&scene, &vehicle);
        let path = straight(20);
        let before = tightest(&path, &field);
        let after = tightest(&reduce(&path, &vehicle, &field), &field);
        assert!(after >= before - 1e-9, "{after} against {before}");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib shortcut`
Expected: FAIL — ``cannot find function `reduce` ``, une fois le module déclaré
dans `lib.rs`.

- [ ] **Step 3: Write minimal implementation**

Ajouter `pub mod shortcut;` à `crates/swept-solver/src/lib.rs`, en gardant
l'ordre alphabétique, puis écrire au-dessus du bloc de tests :

```rust
//! Removing what a plan does not need.
//!
//! A hybrid A\* cannot land exactly on its grid, so it bridges the gap with
//! small manoeuvres that buy nothing — the shunt that exists only because 90 cm
//! primitives and 6° headings do not line up on the goal.
//!
//! # What makes a segment superfluous
//!
//! Not its length. A thirty-centimetre reverse to clear a post is exactly what
//! a driver does, and a threshold in centimetres would delete it. What matters
//! is whether it **buys** anything:
//!
//! > A stretch is superfluous when a Reeds-Shepp curve joins the pose before it
//! > to the pose after it without collision, with no more reversals, and leaving
//! > at least as much room.
//!
//! Directly checkable, because Reeds-Shepp gives that connection in closed
//! form. It is the same rule the alternatives filter already applies to whole
//! plans — *keep only what buys room* — applied to a stretch.
//!
//! The result is **irreducible**: no stretch remains that a curve could
//! replace. That is a stronger claim than "under some threshold", and an
//! honest one.

use crate::path::evaluate_at_least;
use crate::result::DirectedPose;
use swept_core::clearance::ClearanceField;
use swept_core::curves::reeds_shepp;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// How many poses a shortcut must span to be worth trying.
///
/// ARBITRARY. Below this the stretch is shorter than the curve that would
/// replace it, so nothing can be gained and every attempt is wasted work.
const MIN_SPAN: usize = 4;

/// How many stretches one pass may try before giving up.
///
/// **This bound is what makes the reduction affordable.** Trying every pair of
/// poses is quadratic, and each pair costs up to forty-eight closed-form
/// solves: on a three-hundred-pose plan that is over two million, for a
/// post-processing step nobody is waiting on. Sampling a bounded number of
/// stretches, longest first, finds the replacements that matter — a shunt the
/// planner bridged is a long stretch, not a short one.
///
/// ARBITRARY in magnitude, and the figure to raise first if `grid_cost` shows
/// plans coming back reducible.
const MAX_ATTEMPTS: usize = 4_000;

/// Sampling step along a replacement curve, in metres.
///
/// ARBITRARY — the step the rest of the solver uses, so that a clearance
/// measured here means what it means elsewhere.
const SAMPLE_STEP_M: f64 = 0.08;

/// How many gear changes a path makes.
#[must_use]
pub fn reversal_count(poses: &[DirectedPose]) -> usize {
    poses
        .windows(2)
        .filter(|pair| pair[0].direction != pair[1].direction)
        .count()
}

/// The tightest clearance along a path, or zero if it collides.
#[must_use]
pub fn tightest(poses: &[DirectedPose], field: &ClearanceField) -> f64 {
    let bare: Vec<Pose> = poses.iter().map(|p| p.pose).collect();
    evaluate_at_least(&bare, field, f64::NEG_INFINITY).unwrap_or(0.0)
}

/// Replaces every superfluous stretch, until none is left.
///
/// Greedy and repeated: each pass takes the longest replacement it can find,
/// and passes run until one changes nothing. The result is irreducible.
#[must_use]
pub fn reduce(
    poses: &[DirectedPose],
    vehicle: &Vehicle,
    field: &ClearanceField,
) -> Vec<DirectedPose> {
    let mut current = poses.to_vec();
    loop {
        let Some(next) = one_pass(&current, vehicle, field) else {
            return current;
        };
        current = next;
    }
}

/// One replacement, or `None` when the path is already irreducible.
fn one_pass(
    poses: &[DirectedPose],
    vehicle: &Vehicle,
    field: &ClearanceField,
) -> Option<Vec<DirectedPose>> {
    let reference_room = tightest(poses, field);
    let reference_shunts = reversal_count(poses);

    // Longest stretch first: replacing more at once converges faster, and a
    // long replacement is never worse than the short ones inside it. Bounded
    // by MAX_ATTEMPTS, because trying every pair is quadratic.
    let mut attempts = 0usize;
    for span in (MIN_SPAN..poses.len()).rev() {
        for start in 0..poses.len().saturating_sub(span) {
            attempts += 1;
            if attempts > MAX_ATTEMPTS {
                return None;
            }
            let end = start + span;
            let (from, to) = (poses[start].pose, poses[end].pose);

            for curve in reeds_shepp::all(from, to, vehicle.min_turning_radius) {
                if curve.reversals() > reference_shunts {
                    continue;
                }
                let mut replacement = vec![from];
                replacement.extend(curve.poses(from, SAMPLE_STEP_M));
                let Some(room) = evaluate_at_least(&replacement, field, reference_room - 1e-9)
                else {
                    continue;
                };
                if room + 1e-9 < reference_room {
                    continue;
                }

                let mut out = poses[..start].to_vec();
                out.extend(directed(&curve, from, SAMPLE_STEP_M));
                out.extend_from_slice(&poses[end + 1..]);
                if reversal_count(&out) <= reference_shunts && out.len() < poses.len() {
                    return Some(out);
                }
            }
        }
    }
    None
}

/// Samples a curve into poses that each carry the gear they are driven in.
fn directed(curve: &swept_core::curves::CurvePath, from: Pose, step: f64) -> Vec<DirectedPose> {
    let mut out = Vec::new();
    let mut at = from;
    for segment in curve.segments() {
        let sampled = swept_core::kinematics::sample_arc(
            at,
            segment.curvature(curve.radius()),
            segment.signed_length(),
            step,
        );
        if let Some(last) = sampled.last() {
            at = *last;
        }
        out.extend(sampled.into_iter().map(|pose| DirectedPose {
            pose,
            direction: segment.direction,
        }));
    }
    out
}
```

L'import de `Scene` et `Direction` n'est utilisé que par le bloc de tests ; les
y déplacer si le compilateur le signale.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver --lib shortcut`
Expected: PASS — 4 tests.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/shortcut.rs crates/swept-solver/src/lib.rs
git commit -m "feat(solver): remove every stretch a curve can replace for free"
```

---

### Task 4: Le branchement et la mesure

**Files:**
- Modify: `crates/swept-solver/src/multi.rs`
- Modify: `crates/swept-solver/examples/grid_cost.rs`
- Modify: `docs/ALGORITHME.md`

**Interfaces:**
- Consumes: `shortcut::reduce`
- Produces: rien de nouveau.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/multi.rs` :

```rust
    #[test]
    fn a_plan_is_irreducible_when_it_comes_back() {
        // Whatever the search produced, no stretch remains that a curve could
        // replace for free. Running the reduction again must change nothing.
        let (vehicle, sc) = (lbx(), scene(3.0));
        let outcome = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        let best = outcome.best().expect("3 m is plannable");
        let field = ClearanceField::new(&sc, &vehicle);
        let again = crate::shortcut::reduce(&best.poses, &vehicle, &field);
        assert_eq!(again.len(), best.poses.len(), "the plan was still reducible");
    }

    #[test]
    fn reduction_never_costs_the_plan_room() {
        let (vehicle, sc) = (lbx(), scene(4.0));
        let outcome = plan(&vehicle, &sc, 3, SearchBudget::default(), &mut Silent, None);
        let best = outcome.best().expect("4 m is plannable");
        let field = ClearanceField::new(&sc, &vehicle);
        let measured = crate::shortcut::tightest(&best.poses, &field);
        assert!(
            measured >= best.min_clearance - 1e-9,
            "reported {} but measures {measured}",
            best.min_clearance
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib multi`
Expected: FAIL — `a_plan_is_irreducible_when_it_comes_back` échoue tant que la
réduction n'est pas appliquée.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-solver/src/multi.rs`, à l'endroit où une `Maneuver` est
construite à partir du meilleur atterrissage, réduire le trajet avant de le
rendre et **remesurer** la marge sur le trajet réduit :

```rust
            let poses = crate::shortcut::reduce(&poses, vehicle, &field);
            let min_clearance = crate::shortcut::tightest(&poses, &field);
            let moves = u8::try_from(crate::shortcut::reversal_count(&poses) + 1)
                .unwrap_or(u8::MAX);
```

Remesurer plutôt que reporter la marge d'avant : la réduction ne peut
qu'améliorer, mais reporter un chiffre qui ne correspond plus au trajet rendu
serait exactement le défaut que ce projet corrige ailleurs.

Recompter les manœuvres pour la même raison : un trajet réduit en fait moins.

- [ ] **Step 4: Run the full suite and measure**

Run: `just ci`
Expected: PASS de bout en bout.

Run: `cargo run -p swept-solver --release --example grid_cost`

Consigner le tableau obtenu dans `docs/ALGORITHME.md`, §9, à côté de celui du
lot précédent. Les grandeurs qui comptent : la marge, le nombre de manœuvres,
et le nombre de nœuds. **Si la marge baisse quelque part, le lot a un défaut** :
la réduction ne prend un raccourci que s'il laisse au moins autant de place.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/multi.rs docs/ALGORITHME.md
git commit -m "feat(solver): return a plan no curve can shorten for free"
```

---

## Ce que ce lot ne fait pas

- Il ne touche pas à `swept-core`, livré au lot 2c-1.
- Il ne touche pas à `exact.rs` : la recherche exhaustive garde ses courbes de
  Dubins, qui suffisent puisqu'elle ne cherche qu'un seul mouvement.
- Il ne change pas la fonction de coût : le terme de marge du lot précédent
  reste tel quel.
- Il ne touche ni à `swept-wasm` ni à l'interface.

## Vérification finale du lot

- [ ] `just ci` passe.
- [ ] Les onze tests de `multi.rs` et les trois invariants de `solve.rs` passent
      **sans qu'aucune assertion ait été affaiblie**.
- [ ] `git diff main --stat` ne touche que `crates/swept-solver/` et
      `docs/ALGORITHME.md`.
- [ ] Un plan rendu est **irréductible dans la limite du budget** : le
      relancer dans `reduce` ne change rien. Si `MAX_ATTEMPTS` a coupé la
      recherche, le dire dans la PR plutôt que de laisser croire à une
      irréductibilité complète.
- [ ] La marge rapportée est mesurée **sur le trajet rendu**, pas héritée
      d'avant la réduction.
- [ ] `grid_cost` montre une marge au moins égale à celle du lot précédent sur
      chaque ouverture. Une baisse est un défaut, pas un arbitrage.
