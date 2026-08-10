# Lot 2b — La recherche exacte par Dubins — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remplacer les trajectoires candidates construites à la main par une
énumération de courbes de Dubins entre des poses de départ et des poses
d'arrivée balayées, de sorte que la recherche exhaustive trouve une entrée à
une manœuvre là où elle ne trouve rien aujourd'hui.

**Architecture:** `exact::search` garde exactement sa signature — c'est ce qui
permet à `solve`, `min_road`, aux tests d'intégration et à l'exemple de
compiler sans changement. Ce qui change est ce qu'elle essaie : au lieu d'une
forme figée (droite, quart de tour, droite), elle balaie des paires de poses et
demande à `swept_core::curves::dubins::all` toutes les courbes qui les
relient. La marche arrière s'obtient par symétrie temporelle sur les mêmes
courbes. Un élagage par seuil rend le balayage abordable.

**Tech Stack:** Rust 1.97.1 (édition 2024). `swept-solver` dépend de
`swept-core`, qui a livré `curves::dubins` au lot 2a.

## Écarts constatés à l'exécution

Consignés ici plutôt que réécrits dans les tâches : le plan reste ce qui était
prévu, cette section dit ce que la mesure a imposé.

1. **Les grilles proposées étaient mal réparties.** Mesuré axe par axe :
   doubler `entry_steps`, `heading_steps` ou `radius_steps` n'achète aucune
   marge, `start_x_steps` la fait passer de 0,1 à 4,2 cm. Retenu :
   `fine = (2, 64, 12, 4, 2)`, `coarse = (1, 16, 4, 2, 2)`. Les comptes doivent
   rester **pairs**, sans quoi la grille enjambe la valeur centrale au lieu de
   tomber dessus.
2. **`start_poses` bornait la voie au mur, pas au trottoir.** Défaut hérité du
   prototype : les poses de départ hautes avaient les rétros au-dessus du
   trottoir, qui est un obstacle plein. Corrigé.
3. **Une passe de reconnaissance a été ajoutée à `evaluate_at_least`.** Sans
   elle la passe fine tenait 10,5 s. Huit sondes réparties, correction intacte.
   Résultat : 0,77 s en marche avant, **1,1 s en marche arrière** — la cible
   d'une seconde n'est donc pas tout à fait tenue dans ce sens.
4. **Le test décisif a changé de scène.** Le portail mesuré avec des vantaux à
   90° n'admet aucune entrée en un mouvement, et ce n'est pas la grille : la
   trajectoire idéale construite à la main collisionne aussi. Le test porte
   désormais sur des vantaux à 118°, et un second test épingle le refus à 90°
   comme un fait de géométrie.
5. **Le critère « point le plus serré dans le passage » a dû être élargi** de
   `wheelbase + front_overhang` vers l'arrière : la pose enregistrée est celle
   de l'essieu arrière, alors que ce qui frotte est un rétroviseur 3,43 m plus
   avant.

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**, épinglée par `rust-toolchain.toml`.
- `swept-core` garde **zéro dépendance de production**. Ce lot ne touche pas à
  ses dépendances. `swept-solver` garde les siennes inchangées.
- `#![deny(missing_docs)]` sur les deux crates. La documentation manquante
  casse le build.
- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms
  de tests, noms de branches, messages de commit. Seule la documentation projet
  (`docs/`) reste en français.
- Longueurs en **mètres** (`f64`), angles en **radians** (type `Radians`).
- **Aucune constante numérique nue.** Chaque valeur est une `const` nommée,
  documentée par sa justification et sa provenance (`ARBITRARY` ou `MEASURED`).
- Clippy `pedantic` en warning, `missing_panics_doc` et `missing_errors_doc`
  actifs. Le CI échoue sur un warning.
- **Aucune horloge.** Les budgets se comptent en candidats ou en nœuds, jamais
  en millisecondes. Un balayage exhaustif n'a pas de budget à épuiser.
- Une seule PR ouverte à la fois, branchée sur `main`. Ce lot entier est **une**
  PR ; chaque tâche est un commit.
- Repère : origine au milieu du passage, `y = 0` au nu extérieur du mur,
  `y > 0` vers la cour, `x` le long de la voie.
- `main` est protégée : `rust`, `web` et `fixtures` doivent être verts pour
  fusionner.

## Ce que ce lot ne fait pas

- Il n'implémente **pas** Reeds-Shepp, ni l'expansion analytique dans l'A\*,
  ni la réduction par raccourcis — lot 2c.
- Il ne touche **pas** au planificateur multi-manœuvres (`multi.rs`) ni à sa
  fonction de coût. Celle-ci reste explicitement hors périmètre jusqu'après 2c.
- Il ne traite **pas** l'arrivée par la droite de la scène. Tant que la scène
  est symétrique autour de `x = 0`, une approche par la droite est le miroir
  exact d'une approche par la gauche et n'apporte rien. Elle deviendra utile
  quand les deux montants seront positionnés indépendamment (défaut connu n°4).
- Il ne touche **ni à `swept-wasm`, ni à l'interface** : `search` garde sa
  signature, donc la frontière ne bouge pas.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-solver/src/path.rs` | **Réduit à l'évaluation.** `entry_depth`, `evaluate`, `evaluate_at_least`. Les constructeurs `forward_path` / `reverse_path` et leurs constantes disparaissent. |
| `crates/swept-solver/src/poses.rs` | **Nouveau.** Où une entrée peut commencer et où elle doit finir : les grilles de poses de départ et d'arrivée. |
| `crates/swept-solver/src/exact.rs` | La boucle de balayage, réécrite : pour chaque paire de poses et chaque rayon, énumérer Dubins et garder la plus dégagée. `Grid` change de champs. |
| `crates/swept-solver/src/lib.rs` | Déclaration du module `poses`. |
| `crates/swept-solver/examples/bench.rs` | Mesure du coût du nouveau balayage. |
| `docs/ALGORITHME.md` | Mise à jour des §5 et §6. |

Trois fichiers plutôt qu'un : *où l'on peut partir et arriver* est une question
de scène et de véhicule, *quelle courbe les relie* une question de géométrie,
*laquelle garder* une question de recherche. Les mélanger dans `exact.rs`
donnerait un fichier que le lot 2c aurait à démêler, puisqu'il réutilisera les
mêmes grilles de poses pour l'expansion analytique.

---

## Les deux idées qui portent ce lot

Deux remarques avant les tâches, parce qu'elles expliquent des choix qui
paraîtraient sinon arbitraires.

### L'ancienne famille est un cas particulier de la nouvelle

`forward_path` construit une droite, un arc, une droite. C'est littéralement le
mot Dubins `LSL` ou `RSR`, avec deux contraintes gratuites en plus : l'arc fait
exactement 90°, et la droite d'approche exactement 5 m. Remplacer n'est donc
pas un pari : c'est lever deux contraintes. Les tests existants de `exact.rs`,
`invariants.rs` et `reference_results.rs` doivent tous continuer de passer, et
**c'est le principal garde-fou de ce lot**.

Si l'un d'eux régresse, la cause n'est pas « Dubins est moins bon » — c'est que
la grille de poses ne couvre pas ce que l'ancienne forme couvrait. On corrige
la grille, pas le test.

### Le balayage ne serait pas abordable sans élagage

L'ancienne recherche évaluait 7 410 candidats. Une paire de poses donne jusqu'à
six courbes, et il y a beaucoup plus de paires que de candidats autrefois. Sans
précaution, on multiplie le coût par un ordre de grandeur.

L'élagage qui sauve tout est simple : **le balayage cherche la trajectoire la
plus dégagée, donc il n'a pas besoin de savoir à quel point une mauvaise est
mauvaise.** Dès qu'un point du chemin passe sous la meilleure marge déjà
trouvée, le candidat est mort. La plupart le sont en quelques poses au lieu de
deux cents. C'est la Task 1, et elle vient en premier parce que tout le reste
en dépend.

---

### Task 1: L'évaluation qui abandonne tôt

**Files:**
- Modify: `crates/swept-solver/src/path.rs`

**Interfaces:**
- Consumes: `swept_core::clearance::{Clearance, ClearanceField}`,
  `swept_core::kinematics::Pose`
- Produces: `path::evaluate_at_least(poses: &[Pose], field: &ClearanceField, floor: f64) -> Option<f64>`.
  `path::evaluate` est conservée avec sa signature actuelle et devient un appel
  à la précédente.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/path.rs` :

```rust
    #[test]
    fn a_floor_below_everything_gives_the_same_answer_as_evaluating() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, -3.5, Radians::default()))
            .collect();
        let plain = evaluate(&poses, &field).expect("a clear path");
        let floored = evaluate_at_least(&poses, &field, f64::NEG_INFINITY).expect("a clear path");
        assert!((plain - floored).abs() < 1e-12);
    }

    #[test]
    fn a_path_that_cannot_beat_the_floor_is_abandoned() {
        // The sweep only wants to know whether a candidate beats the best so
        // far. Once a pose falls below that, how much worse it gets is of no
        // interest, and finishing the walk would be wasted work.
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, -3.5, Radians::default()))
            .collect();
        let reachable = evaluate(&poses, &field).expect("a clear path");
        assert_eq!(evaluate_at_least(&poses, &field, reachable + 0.01), None);
    }

    #[test]
    fn a_colliding_path_is_refused_whatever_the_floor() {
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        let poses: Vec<Pose> = (0..40)
            .map(|i| Pose::new(-8.0 + f64::from(i) * 0.4, 0.15, Radians::default()))
            .collect();
        assert_eq!(evaluate_at_least(&poses, &field, f64::NEG_INFINITY), None);
    }

    #[test]
    fn an_empty_path_scores_nothing() {
        // Guards the `smallest < f64::MAX` sentinel: an empty path must not
        // come back as infinitely roomy and win the sweep.
        let (scene, vehicle) = (wide_scene(), lbx());
        let field = ClearanceField::new(&scene, &vehicle);
        assert_eq!(evaluate_at_least(&[], &field, f64::NEG_INFINITY), None);
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib path`
Expected: FAIL — ``cannot find function `evaluate_at_least` ``.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-solver/src/path.rs`, remplacer la fonction `evaluate` par ce
couple :

```rust
/// Scores a path: its tightest clearance, or `None` if it collides anywhere.
#[must_use]
pub fn evaluate(poses: &[Pose], field: &ClearanceField) -> Option<f64> {
    evaluate_at_least(poses, field, f64::NEG_INFINITY)
}

/// Scores a path, giving up as soon as it can no longer beat `floor`.
///
/// Returns the same answer as [`evaluate`] whenever it returns `Some`. The
/// difference is what it does with a bad candidate: a sweep looking for the
/// roomiest path does not need to know *how* bad a worse one is, only that it
/// is worse. Passing the best clearance found so far as `floor` rejects most
/// candidates within a few poses instead of walking all two hundred.
///
/// Pass `f64::NEG_INFINITY` to score unconditionally.
#[must_use]
pub fn evaluate_at_least(poses: &[Pose], field: &ClearanceField, floor: f64) -> Option<f64> {
    let mut smallest = f64::MAX;
    for pose in poses {
        match field.at(*pose) {
            Clearance::Collision => return None,
            Clearance::Clear(margin) => {
                if margin <= floor {
                    return None;
                }
                smallest = smallest.min(margin);
            }
        }
    }
    (smallest < f64::MAX).then_some(smallest)
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver --lib path`
Expected: PASS — les tests existants plus les 4 nouveaux.

Puis `cargo clippy -p swept-solver --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/path.rs
git commit -m "perf(solver): let a candidate be abandoned once it cannot win"
```

---

### Task 2: Où une entrée commence et où elle finit

**Files:**
- Create: `crates/swept-solver/src/poses.rs`
- Modify: `crates/swept-solver/src/lib.rs`

**Interfaces:**
- Consumes: `path::entry_depth`, `swept_core::kinematics::Pose`,
  `swept_core::scene::Scene`, `swept_core::units::Radians`,
  `swept_core::vehicle::Vehicle`
- Produces: `poses::LANE_MARGIN_M`, `poses::ENTRY_SPAN_M`,
  `poses::APPROACH_REACH_M`, `poses::GOAL_HEADING_SPAN_DEGREES`,
  `poses::start_poses(vehicle: &Vehicle, scene: &Scene, x_steps: u16, lateral_steps: u16) -> Vec<Pose>`,
  `poses::goal_poses(vehicle: &Vehicle, scene: &Scene, entry_steps: u16, heading_steps: u16) -> Vec<Pose>`

**La pose d'arrivée devient explicite, et c'est un gain.** Le critère d'arrivée
actuel — « avoir franchi `entry_depth` » — ne contraint ni la position ni le
cap final, d'où le véhicule qui termine de travers dans la cour. Une courbe
exige une pose complète, ce qui corrige le défaut au passage : on ne balaie que
des caps proches de la perpendiculaire.

**La droite d'approche de 5 m disparaît**, remplacée par un balayage de la
position de départ le long de la voie. C'est plus général : le conducteur
choisit où il commence à manœuvrer, et Dubins choisit la forme.

- [ ] **Step 1: Write the failing test**

Créer `crates/swept-solver/src/poses.rs` avec pour tout contenu ce bloc :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;
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
            gate: GateKind::Sliding,
        }
    }

    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn every_start_sits_on_the_carriageway_facing_along_it() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        let starts = start_poses(&vehicle, &sc, 4, 6);
        assert!(!starts.is_empty());
        for pose in &starts {
            assert!(pose.y < 0.0, "a start belongs on the road, got y={}", pose.y);
            assert!(
                pose.y > -sc.pavement_width - sc.road_width,
                "a start belongs on the carriageway, got y={}",
                pose.y
            );
            assert!(pose.heading.get().abs() < 1e-12, "a start faces along the road");
            assert!(pose.x < 0.0, "a start is short of the opening, got x={}", pose.x);
        }
    }

    #[test]
    fn the_lateral_sweep_keeps_the_mirrors_inside_the_lane() {
        // A start pose whose mirrors already overhang the kerb is not a start
        // at all: the sweep would spend its budget on candidates that collide
        // before they move.
        let (vehicle, sc) = (lbx(), scene(3.0));
        let half = vehicle.mirror_width / 2.0;
        for pose in start_poses(&vehicle, &sc, 4, 6) {
            assert!(pose.y + half <= -LANE_MARGIN_M + 1e-12, "got y={}", pose.y);
            assert!(
                pose.y - half >= -sc.pavement_width - sc.road_width + LANE_MARGIN_M - 1e-12,
                "got y={}",
                pose.y
            );
        }
    }

    #[test]
    fn every_goal_sits_in_the_yard_facing_into_it() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        let goals = goal_poses(&vehicle, &sc, 8, 2);
        assert!(!goals.is_empty());
        let depth = crate::path::entry_depth(&sc, &vehicle);
        for pose in &goals {
            assert!((pose.y - depth).abs() < 1e-12, "a goal sits at the entry depth");
            assert!(pose.x.abs() <= ENTRY_SPAN_M + 1e-12, "got x={}", pose.x);
        }
    }

    #[test]
    fn no_goal_heading_strays_further_than_the_design_allows() {
        // Criterion 3 of the design: the vehicle ends within five degrees of
        // square to the opening. Enforcing it in the generator means no later
        // stage has to check it.
        let (vehicle, sc) = (lbx(), scene(3.0));
        for pose in goal_poses(&vehicle, &sc, 8, 4) {
            let off_square = (pose.heading.get() - FRAC_PI_2).abs();
            assert!(
                off_square <= GOAL_HEADING_SPAN_DEGREES.to_radians() + 1e-12,
                "got {} degrees off square",
                off_square.to_degrees()
            );
        }
    }

    #[test]
    fn a_single_step_still_yields_the_square_centred_goal() {
        // Zero steps must not mean zero poses. The bisection in `min_road`
        // runs the coarse grid a dozen times over and would otherwise start
        // returning nothing at all.
        let (vehicle, sc) = (lbx(), scene(3.0));
        let goals = goal_poses(&vehicle, &sc, 0, 0);
        assert_eq!(goals.len(), 1);
        assert!(goals[0].x.abs() < 1e-12);
        assert!((goals[0].heading.get() - FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn a_carriageway_narrower_than_the_vehicle_yields_no_start() {
        // Not an error — a result. The sweep reports NotFound rather than
        // pretending a lane it cannot sit in is drivable.
        let mut sc = scene(3.0);
        sc.road_width = 0.5;
        assert!(start_poses(&lbx(), &sc, 4, 6).is_empty());
    }

    #[test]
    fn more_steps_never_yield_fewer_poses() {
        let (vehicle, sc) = (lbx(), scene(3.0));
        assert!(start_poses(&vehicle, &sc, 8, 12).len() >= start_poses(&vehicle, &sc, 4, 6).len());
        assert!(goal_poses(&vehicle, &sc, 16, 4).len() >= goal_poses(&vehicle, &sc, 8, 2).len());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib poses`
Expected: FAIL — ``failed to resolve: use of undeclared crate or module `poses` ``, puis
``cannot find function `start_poses` `` une fois le module déclaré.

- [ ] **Step 3: Write minimal implementation**

Ajouter au-dessus du bloc de tests dans `crates/swept-solver/src/poses.rs` :

```rust
//! Where a one-move entry may start, and where it must end.
//!
//! A Dubins curve joins two *poses*, not two positions. That is a stricter
//! requirement than the search used to work under — its arrival test was
//! merely "past the entry depth", which constrained neither where along the
//! opening the vehicle ended up nor which way it pointed, hence the vehicle
//! finishing askew in the yard. Naming the arrival pose fixes that by
//! construction: this module only ever produces goals square to the opening,
//! within the few degrees the design allows.
//!
//! Both grids are inclusive of their bounds and never empty when the geometry
//! admits anything at all — a zero step count still yields the centre pose,
//! because the carriageway bisection runs the coarse grid a dozen times over
//! and must not silently start returning nothing.

use crate::path::entry_depth;
use std::f64::consts::FRAC_PI_2;
use swept_core::kinematics::Pose;
use swept_core::scene::Scene;
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;

/// Clearance kept between the vehicle's widest point and the lane edges when
/// choosing where the approach is driven, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:362`).
pub const LANE_MARGIN_M: f64 = 0.02;

/// How far either side of the opening centre a goal is aimed, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:372`).
pub const ENTRY_SPAN_M: f64 = 0.9;

/// How far back along the road the earliest start pose sits, in metres.
///
/// ARBITRARY. The old search drove a fixed 5 m run-up and started the turn at
/// `entry_x - radius - 5`, which for the radii tried put the first pose
/// between 8 and 12 m short of the opening. Fourteen metres covers that and
/// leaves room for a wider turn; anything further back is straight road that
/// buys no clearance.
pub const APPROACH_REACH_M: f64 = 14.0;

/// How far the final heading may sit from square to the opening, in degrees.
///
/// Criterion 3 of the design: the vehicle must end within five degrees of the
/// perpendicular. Enforced here, in the generator, so that no later stage has
/// to re-check it.
pub const GOAL_HEADING_SPAN_DEGREES: f64 = 5.0;

/// Spreads `steps + 1` values evenly across `low..=high`.
///
/// A step count of zero yields the midpoint alone rather than nothing, which
/// is what keeps a coarse grid usable.
fn spread(low: f64, high: f64, steps: u16) -> Vec<f64> {
    if steps == 0 {
        return vec![f64::midpoint(low, high)];
    }
    (0..=steps)
        .map(|i| low + (high - low) * f64::from(i) / f64::from(steps))
        .collect()
}

/// Every pose an approach may start from, on the carriageway facing the
/// opening.
///
/// Returns an empty vector when the carriageway is too narrow for the vehicle
/// to sit in at all — a result, not an error.
#[must_use]
pub fn start_poses(
    vehicle: &Vehicle,
    scene: &Scene,
    x_steps: u16,
    lateral_steps: u16,
) -> Vec<Pose> {
    let half_width = vehicle.mirror_width / 2.0;
    let low = -scene.pavement_width - scene.road_width + half_width + LANE_MARGIN_M;
    let high = -half_width - LANE_MARGIN_M;
    if low > high {
        return Vec::new();
    }

    let mut out = Vec::new();
    for x in spread(-APPROACH_REACH_M, -ENTRY_SPAN_M, x_steps) {
        for y in spread(low, high, lateral_steps) {
            out.push(Pose::new(x, y, Radians::default()));
        }
    }
    out
}

/// Every pose an entry may finish on: in the yard, square to the opening.
#[must_use]
pub fn goal_poses(
    vehicle: &Vehicle,
    scene: &Scene,
    entry_steps: u16,
    heading_steps: u16,
) -> Vec<Pose> {
    let depth = entry_depth(scene, vehicle);
    let span = GOAL_HEADING_SPAN_DEGREES.to_radians();

    let mut out = Vec::new();
    for x in spread(-ENTRY_SPAN_M, ENTRY_SPAN_M, entry_steps) {
        for heading in spread(FRAC_PI_2 - span, FRAC_PI_2 + span, heading_steps) {
            out.push(Pose::new(x, depth, Radians::new(heading)));
        }
    }
    out
}
```

Puis déclarer le module dans `crates/swept-solver/src/lib.rs`, en gardant
l'ordre alphabétique des `pub mod` :

```rust
pub mod budget;
pub mod exact;
pub mod landing;
pub mod min_road;
pub mod multi;
pub mod path;
pub mod poses;
pub mod result;
pub mod solve;
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver --lib poses`
Expected: PASS — 7 tests.

Puis `cargo clippy -p swept-solver --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/poses.rs crates/swept-solver/src/lib.rs
git commit -m "feat(solver): name where an entry may start and where it must end"
```

---

### Task 3: Le balayage en marche avant par Dubins

**Files:**
- Modify: `crates/swept-solver/src/exact.rs`
- Modify: `crates/swept-solver/src/path.rs` (suppression de `forward_path`)

**Interfaces:**
- Consumes: `poses::{start_poses, goal_poses}`, `path::evaluate_at_least`,
  `swept_core::curves::dubins::all`, `swept_core::curves::CurvePath`
- Produces: `exact::Grid` avec les champs
  `radius_steps: u16`, `start_x_steps: u16`, `lateral_steps: u16`,
  `entry_steps: u16`, `heading_steps: u16` ;
  `exact::Grid::fine()`, `exact::Grid::coarse()`,
  `exact::Grid::candidate_count(self) -> u64` (l'argument `Approach` disparaît :
  les deux sens balaient désormais le même nombre de paires) ;
  `exact::search(vehicle, scene, approach, grid) -> Outcome` **inchangée**.

C'est la tâche centrale. Elle remplace la boucle en marche avant et supprime
**les deux** constructeurs à la main, `forward_path` et `reverse_path`.

**La marche arrière ne rend rien pendant une tâche**, et c'est délibéré :
recopier l'ancienne boucle réversible pour la supprimer à la tâche suivante
serait du code mort écrit exprès. Vérifié avant d'en décider — aucun test du
dépôt n'appelle `search` avec `Approach::Reverse` ; le seul appelant est
`solve::alternatives`, qui essaie la marche avant d'abord et s'arrête dès
qu'elle répond. La suite reste donc verte, et la Task 4 rétablit la marche
arrière avec les mêmes courbes.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/exact.rs` :

```rust
    #[test]
    fn a_narrow_opening_that_defeated_the_old_shape_now_admits_an_entry() {
        // Fabien's gateway: 2.29 m clear, leaves at 90 degrees, 1.25 m
        // pavement, 6.20 m road, and the pivot radius rather than the kerb
        // radius. The old fixed shape — straight, quarter turn, straight —
        // found nothing at all here, out of 7 410 candidates.
        let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 3.59).expect("valid");
        let mut sc = scene_with_opening(2.29);
        sc.pavement_width = 1.25;
        sc.road_width = 6.20;
        sc.dropped_kerb_width = 3.20;
        sc.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        };

        let outcome = search(&vehicle, &sc, Approach::Forward, Grid::fine());
        let best = outcome.best().expect("Dubins finds what the fixed shape could not");
        assert_eq!(best.moves, 1);
        assert!(best.is_exact(), "an exhaustive sweep says so");
        assert!(best.min_clearance > 0.0, "got {}", best.min_clearance);
    }

    #[test]
    fn the_vehicle_finishes_square_to_the_opening() {
        // Criterion 3 of the design. The old arrival test only demanded depth,
        // so the vehicle could end up askew in the yard.
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits an entry");
        let last = best.poses.last().expect("a manoeuvre has poses");
        let off_square = (last.pose.heading.get() - std::f64::consts::FRAC_PI_2).abs();
        assert!(
            off_square.to_degrees() <= 5.0 + 1e-9,
            "finished {} degrees off square",
            off_square.to_degrees()
        );
    }

    #[test]
    fn a_forward_sweep_never_reverses() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Forward,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits an entry");
        for step in &best.poses {
            assert_eq!(step.direction, Direction::Forward);
        }
    }

    #[test]
    fn the_path_starts_on_the_road_and_ends_in_the_yard() {
        let (vehicle, sc) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &sc, Approach::Forward, Grid::fine());
        let best = outcome.best().expect("4 m admits an entry");
        let first = best.poses.first().expect("a manoeuvre has poses");
        let last = best.poses.last().expect("a manoeuvre has poses");
        assert!(first.pose.y < 0.0, "starts on the road, got y={}", first.pose.y);
        assert!(
            last.pose.y >= crate::path::entry_depth(&sc, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.pose.y
        );
    }

    #[test]
    fn the_coarse_grid_divides_the_fine_one_on_every_axis() {
        // This is what makes the next test a property instead of a hope. Two
        // sweeps at unrelated step counts share almost no values, so a coarse
        // grid could legitimately beat a fine one. Making each coarse count
        // divide its fine counterpart makes the coarse sweep try a strict
        // subset, bit for bit.
        let (fine, coarse) = (Grid::fine(), Grid::coarse());
        for (f, c, axis) in [
            (fine.start_x_steps, coarse.start_x_steps, "start_x"),
            (fine.lateral_steps, coarse.lateral_steps, "lateral"),
            (fine.entry_steps, coarse.entry_steps, "entry"),
            (fine.heading_steps, coarse.heading_steps, "heading"),
        ] {
            assert!(c > 0, "{axis}: a coarse count of zero cannot divide anything");
            assert_eq!(f % c, 0, "{axis}: {c} does not divide {f}");
        }
        // Radii step by a fixed increment, so a smaller count is a prefix.
        assert!(coarse.radius_steps <= fine.radius_steps);
    }

    #[test]
    fn a_finer_grid_never_finds_less_room_than_a_coarser_one() {
        let (vehicle, sc) = (lbx(), scene_with_opening(3.0));
        let coarse = search(&vehicle, &sc, Approach::Forward, Grid::coarse());
        let fine = search(&vehicle, &sc, Approach::Forward, Grid::fine());
        if let (Some(c), Some(f)) = (coarse.best(), fine.best()) {
            assert!(
                f.min_clearance >= c.min_clearance - 1e-9,
                "coarse gave {}, fine gave {}",
                c.min_clearance,
                f.min_clearance
            );
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib exact`
Expected: FAIL — `a_narrow_opening_that_defeated_the_old_shape_now_admits_an_entry`
panique sur `Dubins finds what the fixed shape could not`, puisque la boucle
actuelle ne trouve rien sur cette scène. Les autres échouent ou passent selon
le hasard de la forme figée ; c'est le premier qui compte.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-solver/src/exact.rs`, remplacer les imports, les constantes,
`Grid` et `search` par ce qui suit. `Approach` ne change pas.

```rust
use crate::budget::Discretisation;
use crate::path::evaluate_at_least;
use crate::poses::{goal_poses, start_poses};
use crate::result::{Confidence, DirectedPose, Maneuver, Outcome};
use swept_core::clearance::ClearanceField;
use swept_core::curves::dubins;
use swept_core::kinematics::{Direction, Pose};
use swept_core::scene::Scene;
use swept_core::vehicle::Vehicle;

/// Increment between the turning radii tried, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:368`).
pub const RADIUS_STEP_M: f64 = 0.5;

/// How many values of each parameter the sweep tries.
///
/// Every count is a number of *intervals*, so a count of `n` yields `n + 1`
/// values, and zero yields the midpoint alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Grid {
    /// Turning radii, from the vehicle's tightest upwards.
    pub radius_steps: u16,
    /// Positions along the road the approach may start from.
    pub start_x_steps: u16,
    /// Positions across the carriageway the approach may start from.
    pub lateral_steps: u16,
    /// Points along the opening the entry is aimed at.
    pub entry_steps: u16,
    /// Final headings tried, spread about square to the opening.
    pub heading_steps: u16,
}

impl Grid {
    /// The full sweep, used whenever the answer is shown to a user.
    ///
    /// Coarser per axis than the shape-based sweep it replaces, and richer
    /// overall: one pair of poses yields up to six curves where the old
    /// parameters yielded one path. The counts are calibrated in
    /// `examples/bench.rs`.
    #[must_use]
    pub fn fine() -> Self {
        Self {
            radius_steps: 8,
            start_x_steps: 4,
            lateral_steps: 12,
            entry_steps: 16,
            heading_steps: 4,
        }
    }

    /// A cheaper sweep, for callers that run the search many times over —
    /// the carriageway bisection in particular.
    ///
    /// **Every count divides its counterpart in [`Grid::fine`], on purpose.**
    /// The pose grids place value `i` of `n` at `low + (high - low) * i / n`,
    /// so halving the count keeps exactly every other value — and keeps it
    /// *bit for bit*, since IEEE division is correctly rounded and `2i / 2n`
    /// and `i / n` are the same rational. The coarse sweep therefore
    /// tries a strict subset of what the fine one tries, which makes "finer is
    /// never worse" a property rather than a hope. The radii need no such care:
    /// they step by a fixed increment from the vehicle's tightest, so a smaller
    /// count is simply a prefix.
    #[must_use]
    pub fn coarse() -> Self {
        Self {
            radius_steps: 4,
            start_x_steps: 2,
            lateral_steps: 6,
            entry_steps: 8,
            heading_steps: 2,
        }
    }

    /// How many pose pairs this grid produces, useful for reporting cost.
    ///
    /// Each pair yields up to six Dubins curves, so the number of paths
    /// actually evaluated is at most six times this.
    #[must_use]
    pub fn candidate_count(self) -> u64 {
        let starts = u64::from(self.start_x_steps + 1) * u64::from(self.lateral_steps + 1);
        let goals = u64::from(self.entry_steps + 1) * u64::from(self.heading_steps + 1);
        u64::from(self.radius_steps + 1) * starts * goals
    }
}

/// Sweeps every one-move approach on `grid` and keeps the roomiest.
#[must_use]
pub fn search(vehicle: &Vehicle, scene: &Scene, approach: Approach, grid: Grid) -> Outcome {
    let field = ClearanceField::new(scene, vehicle);
    let step = Discretisation::default().sample_step;

    let starts = start_poses(vehicle, scene, grid.start_x_steps, grid.lateral_steps);
    let goals = goal_poses(vehicle, scene, grid.entry_steps, grid.heading_steps);
    if starts.is_empty() || goals.is_empty() {
        return Outcome::NotFound {
            budget_exhausted: false,
        };
    }

    let mut best: Option<(Vec<Pose>, f64)> = None;

    for i in 0..=grid.radius_steps {
        let radius = vehicle.min_turning_radius + f64::from(i) * RADIUS_STEP_M;
        for &start in &starts {
            for &goal in &goals {
                for curve in curves_between(approach, start, goal, radius) {
                    // The path is sampled from `start`, which the curve
                    // excludes, so the starting pose is prepended by hand.
                    let mut path = vec![start];
                    path.extend(curve_poses(approach, &curve, start, goal, step));

                    // Only a strictly roomier candidate is worth walking to
                    // the end. `floor` is the best clearance so far, so most
                    // candidates die within a few poses.
                    let floor = best.as_ref().map_or(f64::NEG_INFINITY, |(_, m)| *m);
                    if let Some(margin) = evaluate_at_least(&path, &field, floor) {
                        best = Some((path, margin));
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

/// Every curve joining two poses for the given approach.
///
/// Reverse is filled in by the next task. Returning nothing meanwhile is
/// deliberate: no test drives a reverse sweep, and `solve::alternatives` tries
/// forward first and stops as soon as it answers.
fn curves_between(approach: Approach, start: Pose, goal: Pose, radius: f64) -> Vec<CurvePath> {
    match approach {
        Approach::Forward => dubins::all(start, goal, radius),
        Approach::Reverse => Vec::new(),
    }
}

/// Samples a curve into the poses the vehicle actually occupies.
fn curve_poses(
    approach: Approach,
    curve: &CurvePath,
    start: Pose,
    _goal: Pose,
    step: f64,
) -> Vec<Pose> {
    match approach {
        Approach::Forward => curve.poses(start, step),
        Approach::Reverse => Vec::new(),
    }
}
```

Ajouter `use swept_core::curves::CurvePath;` aux imports listés plus haut.

**Un import se déplace.** `exact.rs` importe aujourd'hui
`swept_core::units::Radians`, utilisé par la boucle réversible que cette tâche
supprime. Plus aucun code de production ne s'en sert ici, mais les tests si —
donc le retirer de la tête du fichier et l'ajouter dans le bloc `mod tests` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use swept_core::clearance::Clearance;
    use swept_core::scene::{GateKind, Post};
    use swept_core::units::Radians;
```

Sans ce déplacement, `cargo clippy --all-targets -- -D warnings` échoue sur
`unused_imports` avant même que les tests tournent. La Task 4 le remontera en
tête, `turned_about` en ayant besoin.

Enfin, dans `crates/swept-solver/src/path.rs` :

- supprimer `forward_path`, `reverse_path`, et les constantes `RUN_UP_M` et
  `REVERSE_EXIT_MARGIN_M` — la première est sans objet puisque la position de
  départ est désormais balayée, la seconde n'avait de sens que pour l'ancien
  générateur réversible ;
- supprimer les tests `a_forward_path_starts_on_the_road_and_ends_in_the_yard`
  et `a_reverse_path_that_never_reaches_the_road_is_rejected`, dont la Task 3
  et la Task 4 fournissent les équivalents dans `exact.rs` ;
- garder `entry_depth`, `evaluate`, `evaluate_at_least`, les fixtures
  `wide_scene()` et `lbx()`, et les tests qui les emploient ;
- retirer des imports de `path.rs` ce qui n'est plus utilisé — au minimum
  `std::f64::consts::FRAC_PI_2`, `swept_core::kinematics::sample_arc` et
  `swept_core::units::Radians` en production. `Radians` reste nécessaire au
  bloc `mod tests`, qui en construit des poses ; l'y déplacer comme ci-dessus.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver`
Expected: PASS — les tests unitaires, `invariants.rs` et `reference_results.rs`
inclus.

**Si un test existant régresse**, la cause est presque toujours la couverture
de la grille, pas Dubins. Vérifier dans cet ordre :

1. `start_poses` couvre-t-il la position latérale que l'ancienne forme
   utilisait ? L'ancien balayage allait de `low` à `high` avec 18 pas ; le
   nouveau en a 12. Augmenter `lateral_steps` avant de suspecter la géométrie.
2. `APPROACH_REACH_M` est-il assez grand pour le rayon le plus large ? Un
   départ trop proche du passage ne laisse pas la place de tourner, et toutes
   les familles Dubins rendent alors des courbes qui rasent.
3. `entry_steps` couvre-t-il le point d'entrée que l'ancienne forme visait ?

Ne jamais relâcher une assertion d'un test existant pour faire passer ce lot.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/exact.rs crates/swept-solver/src/path.rs
git commit -m "feat(solver): sweep Dubins curves instead of one fixed shape"
```

---

### Task 4: La marche arrière, par symétrie temporelle

**Files:**
- Modify: `crates/swept-solver/src/exact.rs`
- Modify: `crates/swept-solver/src/path.rs` (suppression de `reverse_path`)

**Interfaces:**
- Consumes: tout ce que la Task 3 produit
- Produces: `curves_between` et `curve_poses` répondent pour
  `Approach::Reverse` ; `legacy_reverse_search` et `reverse_path` disparaissent.

**Le principe.** Reculer le long d'une trajectoire, sous le modèle bicyclette,
c'est exactement la parcourir à l'envers. Un trajet en marche arrière de `A`
vers `B` est donc le trajet en marche avant de `B'` vers `A'`, où `'` retourne
le cap d'un demi-tour. On énumère ce trajet-là avec Dubins, on inverse l'ordre
des poses, et on remet les caps à l'endroit.

C'est déjà l'idée que l'ancien `reverse_path` appliquait — il générait le
chemin depuis la position finale vers l'extérieur puis l'inversait — mais sans
pouvoir choisir la forme.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-solver/src/exact.rs` :

```rust
    #[test]
    fn a_reverse_entry_is_driven_backwards_from_the_road_into_the_yard() {
        let (vehicle, sc) = (lbx(), scene_with_opening(4.0));
        let outcome = search(&vehicle, &sc, Approach::Reverse, Grid::fine());
        let best = outcome.best().expect("4 m admits a reverse entry");
        let first = best.poses.first().expect("a manoeuvre has poses");
        let last = best.poses.last().expect("a manoeuvre has poses");

        assert!(first.pose.y < 0.0, "starts on the road, got y={}", first.pose.y);
        assert!(
            last.pose.y >= crate::path::entry_depth(&sc, &vehicle) - 1e-6,
            "ends past the entry depth, got y={}",
            last.pose.y
        );
        for step in &best.poses {
            assert_eq!(step.direction, Direction::Reverse);
        }
    }

    #[test]
    fn a_reverse_entry_also_finishes_square_to_the_opening() {
        let outcome = search(
            &lbx(),
            &scene_with_opening(4.0),
            Approach::Reverse,
            Grid::fine(),
        );
        let best = outcome.best().expect("4 m admits a reverse entry");
        let last = best.poses.last().expect("a manoeuvre has poses");
        let off_square = (last.pose.heading.get() - std::f64::consts::FRAC_PI_2).abs();
        assert!(
            off_square.to_degrees() <= 5.0 + 1e-9,
            "finished {} degrees off square",
            off_square.to_degrees()
        );
    }

    #[test]
    fn a_reverse_path_is_the_forward_path_of_the_turned_about_problem() {
        // The symmetry the whole task rests on, checked on its own rather than
        // through a sweep: backing from A to B covers the same ground as
        // driving forward from B to A with both headings turned about.
        use std::f64::consts::PI;
        let start = Pose::new(-6.0, -2.5, Radians::default());
        let goal = Pose::new(0.3, 5.0, Radians::new(std::f64::consts::FRAC_PI_2));
        let radius = 4.0;

        let curves = curves_between(Approach::Reverse, start, goal, radius);
        assert!(!curves.is_empty(), "some family applies here");

        for curve in &curves {
            let poses = curve_poses(Approach::Reverse, curve, start, goal, 0.05);
            let last = *poses.last().expect("a sampled path is never empty");
            assert!((last.x - goal.x).abs() < 1e-6, "x off by {}", last.x - goal.x);
            assert!((last.y - goal.y).abs() < 1e-6, "y off by {}", last.y - goal.y);
            let error = (last.heading.get() - goal.heading.get()).rem_euclid(2.0 * PI);
            assert!(
                error.min(2.0 * PI - error) < 1e-6,
                "heading off by {error}"
            );
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib exact::tests::a_reverse_path_is_the_forward_path_of_the_turned_about_problem`
Expected: FAIL — panique sur `some family applies here`, puisque
`curves_between` rend un vecteur vide pour `Approach::Reverse`.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-solver/src/exact.rs`, remplacer `curves_between` et
`curve_poses` par :

```rust
/// The same pose, turned about.
///
/// A vehicle backing along a path covers exactly the ground a vehicle driving
/// forward along the same path in the other direction covers. Turning both
/// poses about is what turns a reverse problem into a Dubins one.
fn turned_about(pose: Pose) -> Pose {
    Pose::new(pose.x, pose.y, pose.heading + Radians::new(PI))
}

/// Every curve joining two poses for the given approach.
///
/// Forward is Dubins directly. Reverse is Dubins on the turned-about problem,
/// read from the goal back to the start — which is why the arguments are
/// swapped here and the samples reversed in [`curve_poses`].
fn curves_between(approach: Approach, start: Pose, goal: Pose, radius: f64) -> Vec<CurvePath> {
    match approach {
        Approach::Forward => dubins::all(start, goal, radius),
        Approach::Reverse => dubins::all(turned_about(goal), turned_about(start), radius),
    }
}

/// Samples a curve into the poses the vehicle actually occupies.
///
/// For a reverse approach the curve runs from goal to start with the headings
/// turned about, so the samples are turned back and read in reverse. The
/// vehicle's own pose is what the caller wants, not the direction it happens
/// to be travelling in.
fn curve_poses(
    approach: Approach,
    curve: &CurvePath,
    start: Pose,
    goal: Pose,
    step: f64,
) -> Vec<Pose> {
    match approach {
        Approach::Forward => curve.poses(start, step),
        Approach::Reverse => {
            let mut sampled = curve.poses(turned_about(goal), step);
            sampled.pop();
            sampled.push(turned_about(start));
            sampled.reverse();
            sampled.into_iter().map(turned_about).collect()
        }
    }
}
```

Les deux lignes autour du `pop` méritent un mot, parce qu'elles ne sont pas
cosmétiques : `CurvePath::poses` exclut sa pose de départ et inclut sa pose
d'arrivée. Pour un trajet réversible, l'arrivée de la courbe est le *départ* du
trajet, et le départ de la courbe — exclu — en est l'*arrivée*. Remplacer le
dernier échantillon par la pose de départ retournée, puis inverser, rend une
liste qui commence juste après le départ et finit exactement sur le but,
exactement comme dans le cas avant.

Ajouter `use std::f64::consts::PI;` en tête de fichier, et **remonter
`use swept_core::units::Radians;`** du bloc `mod tests` — où la Task 3 l'avait
déplacé — vers la tête du fichier, `turned_about` en ayant maintenant besoin.
Le laisser aux deux endroits est une erreur de compilation (`E0252`), pas un
warning : il faut bien le retirer du bloc de tests, qui y accède alors par
`use super::*`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver`
Expected: PASS.

**Si `a_reverse_path_is_the_forward_path_of_the_turned_about_problem` échoue
sur le cap**, l'erreur est presque toujours un demi-tour manquant ou appliqué
deux fois. Le test dit sur quelle grandeur porte l'écart. Un écart de
exactement π sur le cap final signifie que `turned_about` n'a pas été appliqué
au retour ; un écart sur `x` et `y` signifie que l'ordre des poses n'a pas été
inversé.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/src/exact.rs crates/swept-solver/src/path.rs
git commit -m "feat(solver): back in along Dubins curves, by time symmetry"
```

---

### Task 5: Le coût, mesuré et non supposé

**Files:**
- Modify: `crates/swept-solver/examples/bench.rs`
- Modify: `crates/swept-solver/src/exact.rs` (les constantes de `Grid`, si la
  mesure l'impose)

**Interfaces:**
- Consumes: `exact::{Approach, Grid, search}`
- Produces: rien de nouveau — cette tâche substitue une mesure à une
  supposition, et corrige les grilles si nécessaire.

Le risque nommé au §9 de la spec est le volume de candidats : balayer des poses
de départ *et* d'arrivée, six familles chacune, peut faire exploser le nombre
de chemins. La recherche exhaustive coûtait 150 ms. Cette tâche mesure ce
qu'elle coûte maintenant, et arbitre.

- [ ] **Step 1: Write the failing test**

Le test, ici, est une mesure — mais un garde-fou reste assertible. Ajouter au
bloc `mod tests` de `crates/swept-solver/src/exact.rs` :

```rust
    #[test]
    fn a_coarse_grid_visits_fewer_pairs_than_a_fine_one() {
        assert!(Grid::coarse().candidate_count() < Grid::fine().candidate_count());
    }

    #[test]
    fn the_fine_grid_stays_within_a_workable_number_of_pairs() {
        // ARBITRARY ceiling, and deliberately generous: this is not a
        // performance target but a tripwire. A grid that quietly grew by an
        // order of magnitude would still return correct answers, just far too
        // slowly for a worker the interface waits on — and nothing else in the
        // suite would notice.
        assert!(
            Grid::fine().candidate_count() <= 200_000,
            "the fine grid now visits {} pairs",
            Grid::fine().candidate_count()
        );
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --lib exact::tests::the_fine_grid_stays_within`
Expected: PASS avec les grilles proposées en Task 3
(`9 × 5 × 13 × 17 × 5 = 49 725` paires). Si la Task 3 a dû élargir une grille
pour rattraper une régression, ce test dira de combien.

- [ ] **Step 3: Write minimal implementation**

Remplacer `crates/swept-solver/examples/bench.rs` par :

```rust
//! What the exhaustive sweep costs, measured rather than supposed.
//!
//! Run with `cargo run -p swept-solver --release --example bench`.
//!
//! The figure that matters is the wall time of one fine sweep: the interface
//! waits on it inside a worker, and beyond a second or so the tool stops
//! feeling like it answers. Reported here, not asserted anywhere — timings
//! belong in a report, never in a test, or the suite starts depending on the
//! machine it runs on.

use std::time::Instant;
use swept_core::scene::{GateKind, Post};
use swept_core::units::Radians;
use swept_core::vehicle::Vehicle;
use swept_solver::exact::{Approach, Grid, search};

fn scene(opening: f64) -> swept_core::scene::Scene {
    swept_core::scene::Scene {
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
        pavement_width: 1.25,
        dropped_kerb_width: 3.20,
        road_width: 6.20,
        gate: GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        },
    }
}

fn main() {
    // Fabien's gateway and vehicle, with the pivot radius.
    let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 3.59).expect("valid vehicle");

    println!(
        "fine grid:   {} pose pairs, up to {} curves",
        Grid::fine().candidate_count(),
        Grid::fine().candidate_count() * 6
    );
    println!(
        "coarse grid: {} pose pairs, up to {} curves",
        Grid::coarse().candidate_count(),
        Grid::coarse().candidate_count() * 6
    );

    for opening in [2.29_f64, 2.60, 3.00, 4.00] {
        let sc = scene(opening);
        for (label, grid) in [("fine", Grid::fine()), ("coarse", Grid::coarse())] {
            for approach in [Approach::Forward, Approach::Reverse] {
                let started = Instant::now();
                let outcome = search(&vehicle, &sc, approach, grid);
                let elapsed = started.elapsed();
                let found = outcome
                    .best()
                    .map_or_else(|| "nothing".to_string(), |m| format!("{:.1} cm", m.min_clearance * 100.0));
                println!("{opening:.2} m  {label:6}  {approach:?}  {elapsed:>8.1?}  {found}");
            }
        }
    }
}
```

Puis exécuter et arbitrer :

```bash
cargo run -p swept-solver --release --example bench
```

**La règle d'arbitrage.** Si une passe fine dépasse **une seconde**, réduire
d'abord `entry_steps`, puis `lateral_steps` : ce sont les deux axes les plus
nombreux et les plus redondants, puisque Dubins compense un point d'entrée
voisin par une courbe voisine. Ne jamais réduire `radius_steps` en premier :
le rayon change la forme de toutes les familles à la fois, et c'est l'axe qui
achète le plus de marge.

Si à l'inverse une passe fine tient largement sous 300 ms, **augmenter**
`lateral_steps` et `entry_steps` : ce lot cherche de la marge, et chaque pas
supplémentaire en achète.

Consigner le résultat dans la doc de `Grid::fine`, en remplaçant le mot
`calibrées dans examples/bench.rs` par les chiffres mesurés, marqués `MEASURED`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-solver`
Expected: PASS, y compris les deux garde-fous ci-dessus après tout ajustement.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/examples/bench.rs crates/swept-solver/src/exact.rs
git commit -m "perf(solver): measure what the Dubins sweep costs and size the grid to it"
```

---

### Task 6: Le critère d'acceptation, en test d'intégration

**Files:**
- Modify: `crates/swept-solver/tests/reference_results.rs`

**Interfaces:**
- Consumes: `swept_solver::solve::alternatives`, `swept_solver::budget::{SearchBudget, Silent}`
- Produces: rien de nouveau. Cette tâche vérifie de bout en bout ce que les
  tâches précédentes ont rendu possible.

Les tests des tâches 3 et 4 portent sur `search`. Celui-ci porte sur la
réponse que l'interface reçoit vraiment, via `alternatives` — c'est-à-dire
avec l'amorçage du planificateur et le filtre des alternatives dominées. C'est
lui qui atteste les critères 1 et 2 de la spec.

- [ ] **Step 1: Write the failing test**

Ajouter à `crates/swept-solver/tests/reference_results.rs` :

```rust
/// Criteria 1 and 2 of the Dubins design, on the scene that motivated it.
///
/// Before this lot the exhaustive sweep returned nothing here, so the answer
/// came from the planner and was labelled heuristic — it proved nothing, and
/// its tightest point sat against a kerb six metres short of the gateway
/// rather than in the opening.
#[test]
fn fabiens_gateway_admits_a_proved_one_move_entry() {
    let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 3.59).expect("valid vehicle");
    let scene = fabiens_gateway();

    let Outcome::Found(list) = alternatives(
        &vehicle,
        &scene,
        SearchBudget::default(),
        &mut Silent,
        None,
    ) else {
        panic!("this gateway admits an entry");
    };

    let one = list
        .iter()
        .find(|m| m.moves == 1)
        .expect("a one-move entry exists");
    assert!(
        one.is_exact(),
        "a one-move entry must come from the exhaustive sweep, not the planner"
    );

    // The geometric ceiling: clearance can never exceed half the difference
    // between the opening and the vehicle's widest point, whatever the path.
    let ceiling = (scene.opening_width() - vehicle.mirror_width) / 2.0;
    assert!(
        one.min_clearance <= ceiling + 1e-9,
        "{:.1} cm claimed against a {:.1} cm ceiling",
        one.min_clearance * 100.0,
        ceiling * 100.0
    );

    // Criterion 2, first half: at least what the planner already managed.
    //
    // MEASURED — the heuristic planner returned 7.7 cm on this scene before
    // this lot, against a 13.1 cm ceiling. Replacing a heuristic answer by a
    // proved one must not cost room; if it does, the pose grid is too coarse.
    const PLANNER_CLEARANCE_M: f64 = 0.077;
    assert!(
        one.min_clearance >= PLANNER_CLEARANCE_M - 1e-9,
        "{:.1} cm, below the {:.1} cm the planner already found",
        one.min_clearance * 100.0,
        PLANNER_CLEARANCE_M * 100.0
    );

    // Criterion 2, second half: the tightest point is in the gateway, not
    // against a kerb six metres short of it. A path whose worst moment is out
    // on the road has not been squeezed by the opening at all, and its figure
    // answers a different question than the one the user asked.
    let field = ClearanceField::new(&scene, &vehicle);
    let tightest = one
        .poses
        .iter()
        .filter_map(|step| match field.at(step.pose) {
            Clearance::Clear(margin) => Some((margin, step.pose)),
            Clearance::Collision => None,
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .expect("a collision-free manoeuvre has a tightest point");
    let (_, where_it_is) = tightest;
    // The gateway runs from the outer face of the wall to behind the leaves,
    // and the approach across the pavement counts as part of threading it.
    let gateway_far_side = scene.left_post.depth.max(scene.right_post.depth)
        + match scene.gate {
            GateKind::Swinging { leaf_length, .. } => leaf_length,
            GateKind::Sliding => 0.0,
        };
    assert!(
        (-scene.pavement_width..=gateway_far_side).contains(&where_it_is.y),
        "tightest point at y={:.2} m, outside the gateway (which spans {:.2} to {:.2})",
        where_it_is.y,
        -scene.pavement_width,
        gateway_far_side
    );
}
```

Ajouter aussi, au même fichier, la fonction de scène si elle n'y est pas déjà :

```rust
/// Fabien's gateway: 2.29 m clear, leaves at 90 degrees, 1.25 m pavement,
/// 6.20 m carriageway.
fn fabiens_gateway() -> Scene {
    Scene {
        left_post: Post {
            inner_edge_x: -2.29 / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        right_post: Post {
            inner_edge_x: 2.29 / 2.0,
            width: 0.55,
            depth: 0.55,
        },
        wall_thickness: 0.30,
        pavement_width: 1.25,
        dropped_kerb_width: 3.20,
        road_width: 6.20,
        gate: GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        },
    }
}
```

Compléter les `use` en tête du fichier avec ce qui manque parmi :

```rust
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::result::Outcome;
use swept_solver::solve::alternatives;
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --test reference_results fabiens_gateway`
Expected: PASS si les tâches 3 à 5 ont fait leur travail. **Un échec ici est le
signal le plus important du lot** : il dit que la grille de poses ne couvre pas
la trajectoire que ce passage exige. Élargir `entry_steps` et `lateral_steps`,
puis `radius_steps`, et remesurer avec `bench`.

Une réserve sur `PLANNER_CLEARANCE_M`. Les 7,7 cm sont relevés sur une capture
de l'interface, dont on ne connaît pas avec certitude le rayon de braquage
configuré — le test, lui, impose 3,59 m. Si l'assertion échoue de peu, **ne pas
la baisser sans vérifier d'abord** : lancer `bench`, qui affiche la marge
obtenue sur cette scène exacte, et comparer. Si la recherche exacte plafonne
nettement sous 7,7 cm, la grille est en cause. Si elle en est à un ou deux
millimètres, c'est la valeur relevée qui n'était pas mesurée dans les mêmes
conditions ; la corriger alors en la remplaçant par la valeur que `bench`
donne, et en disant dans le commentaire d'où elle vient.

- [ ] **Step 3: Write minimal implementation**

Aucun code de production. Si le test échoue, c'est une grille qu'on corrige en
Task 5, pas une assertion qu'on affaiblit ici. Deux assertions en particulier
ne se négocient pas :

- **le plafond géométrique** — une marge annoncée au-dessus de `(W − w) / 2` ne
  serait pas une bonne nouvelle mais la preuve que le champ de marge ment ;
- **la localisation du point le plus serré** — c'est elle qui distingue une
  réponse qui parle du passage d'une réponse qui parle d'une bordure à six
  mètres de là. Un chiffre juste répondant à la mauvaise question reste faux.

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/tests/reference_results.rs
git commit -m "test(solver): prove a one-move entry on the gateway that motivated this"
```

---

### Task 7: Documentation

**Files:**
- Modify: `docs/ALGORITHME.md`
- Modify: `crates/swept-solver/src/exact.rs` (doc de module)

**Interfaces:**
- Consumes: tout ce qui précède
- Produces: rien de nouveau.

- [ ] **Step 1: Write the failing test**

Le test, ici, est `#![deny(missing_docs)]` plus `cargo doc`. Remplacer la doc de
module en tête de `crates/swept-solver/src/exact.rs` par :

```rust
//! Exhaustive search for a one-move entry.
//!
//! Every pair of poses on the grid is tried, joined by every Dubins curve that
//! applies at every radius, and the roomiest collision-free result is kept.
//! Because the sweep is complete, a failure here means something: there is no
//! one-move entry *on this grid*. That is what makes this solver the reference
//! the planner is seeded from.
//!
//! # Why every curve and not the shortest
//!
//! Dubins curves minimise length. This search maximises clearance, and the
//! shortest path is the one that grazes most — so it asks for all of them and
//! sorts by room. Length never enters the comparison.
//!
//! # Reverse
//!
//! Backing along a path covers exactly the ground that driving forward along
//! it in the other direction covers. A reverse entry is therefore a Dubins
//! problem too: the same curves, read from the goal back to the start with
//! both headings turned about.
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo doc -p swept-solver --no-deps`
Expected: aucun warning. `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
doit également passer, puisque c'est ce que fait le CI.

- [ ] **Step 3: Write minimal implementation**

Dans `docs/ALGORITHME.md`, remplacer le premier paragraphe de la §5 par :

```markdown
**La recherche exacte** balaie des poses de départ le long de la chaussée et
des poses d'arrivée dans l'axe du passage, relie chaque paire par toutes les
courbes de Dubins applicables à chaque rayon de braquage, et retient la plus
dégagée. Comme le balayage est complet, **son échec est informatif** : il n'y a
pas d'entrée en un mouvement sur cette grille.

L'entrée en marche arrière emprunte les mêmes courbes. Reculer le long d'un
trajet, sous le modèle bicyclette, c'est le parcourir à l'envers : il suffit
donc de résoudre le problème retourné — de l'arrivée vers le départ, les deux
caps pivotés d'un demi-tour — puis de relire le résultat à l'endroit.
```

Puis, dans la §6, remplacer le dernier paragraphe — « Ce module ne fait encore
rien d'autre qu'exister » — par :

```markdown
Ces courbes sont désormais ce que la recherche exacte essaie. La forme figée
d'autrefois — une droite, un quart de tour, une droite — n'était rien d'autre
qu'un mot `LSL` ou `RSR` avec deux contraintes gratuites en plus : l'arc faisait
exactement 90°, et la droite d'approche exactement 5 m. Les lever ne coûte rien
et donne accès aux crochets à trois arcs, qui sont précisément ce qu'exige une
entrée serrée.

La pose d'arrivée est devenue explicite au passage, et cela corrige un défaut
qui n'avait rien à voir : le critère d'arrivée était « avoir franchi la
profondeur d'entrée », qui ne contraint ni la position ni le cap, d'où le
véhicule qui terminait de travers dans la cour. Une courbe exige une pose
complète, donc on ne balaie que des arrivées à moins de 5° de la
perpendiculaire.
```

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add docs/ALGORITHME.md crates/swept-solver/src/exact.rs
git commit -m "docs: describe the Dubins sweep and how reverse reuses it"
```

---

## Ce que le lot 2c consommera

```rust
swept_solver::poses::start_poses(vehicle: &Vehicle, scene: &Scene, x_steps: u16, lateral_steps: u16) -> Vec<Pose>
swept_solver::poses::goal_poses(vehicle: &Vehicle, scene: &Scene, entry_steps: u16, heading_steps: u16) -> Vec<Pose>
swept_solver::path::evaluate_at_least(poses: &[Pose], field: &ClearanceField, floor: f64) -> Option<f64>
```

Le lot 2c réutilisera `goal_poses` pour l'expansion analytique — à chaque nœud
développé, tenter une connexion Reeds-Shepp vers chacune de ces poses — et
`evaluate_at_least` pour élaguer ces tentatives contre la meilleure marge déjà
atteinte.

## Vérification finale du lot

- [ ] `just ci` passe.
- [ ] `crates/swept-solver/src/path.rs` ne contient plus ni `forward_path`, ni
      `reverse_path`, ni `RUN_UP_M`, ni `REVERSE_EXIT_MARGIN_M`.
- [ ] `git diff main --stat` ne touche que `crates/swept-solver/` et
      `docs/ALGORITHME.md`. Aucun fichier de `swept-core`, `swept-wasm` ou
      `web/` : la frontière n'a pas bougé, puisque `search` a gardé sa
      signature.
- [ ] `reference_results.rs` et `invariants.rs` passent **sans qu'aucune
      assertion ait été affaiblie**. C'est le garde-fou principal du lot.
- [ ] Une passe fine tient sous une seconde, chiffre mesuré et consigné dans la
      doc de `Grid::fine`.
- [ ] Les trois critères d'acceptation de la spec sont chacun couverts par un
      test qui les nomme : **1** l'entrée à une manœuvre marquée `Exact`, **2**
      la marge au moins égale à celle du planificateur *et* son point le plus
      serré dans le passage, **3** le véhicule à moins de 5° de l'axe. Le
      critère 4 relève du lot 2c.
- [ ] Sur la scène de Fabien, l'interface affiche « recherche exhaustive » et
      non plus « recherche heuristique » pour l'entrée à une manœuvre.
