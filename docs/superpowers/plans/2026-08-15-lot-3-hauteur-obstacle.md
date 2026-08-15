# Lot 3 — La hauteur des obstacles — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Donner une hauteur aux obstacles et une garde au sol au véhicule, pour
qu'une carrosserie puisse surplomber une bordure de trottoir au lieu d'être
arrêtée par un mur de hauteur infinie.

**Architecture:** Un obstacle devient un `Obb` plus une hauteur. À la
construction du champ de marge, les hauteurs sont comparées **une seule fois** à
la garde au sol et les obstacles rangés en deux listes disjointes : ce que la
carrosserie heurte, et ce qu'elle survole. La boucle chaude ne connaît plus
jamais une hauteur. Quatre points de roue, aux coins de la caisse, heurtent tout.

**Tech Stack:** Rust 1.97.1 (édition 2024), TypeScript, Vite, Tailwind v4.
Spec : `docs/superpowers/specs/2026-08-15-hauteur-obstacle-design.md`.

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**, épinglée par `rust-toolchain.toml`.
- `swept-core` garde **zéro dépendance de production**.
- `#![deny(missing_docs)]` sur les crates Rust. La documentation manquante casse
  le build.
- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms
  de tests, noms de branches, messages de commit et **descriptions de PR**.
  Seuls `docs/` et l'interface restent en français.
- Longueurs en **mètres** (`f64`), angles en **radians** (type `Radians`).
- **Aucune constante numérique nue.** Chaque valeur est une `const` nommée,
  documentée par sa justification et sa provenance (`ARBITRARY` ou `MEASURED`).
- Clippy `pedantic` en warning, `missing_panics_doc` et `missing_errors_doc`
  actifs. Le CI échoue sur un warning.
- Une seule PR, branchée sur `main`, une tâche par commit. Branche :
  `feat/lot-3-obstacle-height`.
- `main` est protégée : `rust`, `web` et `fixtures` doivent être verts.
- **Aucune assertion existante n'est affaiblie.** Les scènes des tests actuels
  déclarent leur bordure pleine, ce qui préserve leurs résultats au bit près.

## Ce que ce lot ne fait pas

- Pas d'obstacles arbitraires (jardinière, borne, poteau).
- Pas de franchissement : une roue ne monte jamais sur un trottoir.
- Rien dans la fonction de coût : le surplomb est signalé, jamais pénalisé.
- Pas de hauteurs détaillées côté véhicule (bas de pare-chocs, rétros).
- Pas de valeurs de garde au sol dans `data/vehicles.json` : elles demandent les
  documents constructeurs, que ce lot ne va pas chercher.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-core/src/vehicle.rs` | `ground_clearance`, `wheels()`, `new` à sept paramètres |
| `crates/swept-core/src/scene/obstacles.rs` | Le type `Obstacle` et sa hauteur ; le trottoir devient bas |
| `crates/swept-core/src/scene/mod.rs` | `Scene::kerb_height`, `obstacles() -> Vec<Obstacle>` |
| `crates/swept-core/src/scene/gate.rs` | Les vantaux rendent des `Obstacle::wall` |
| `crates/swept-core/src/clearance.rs` | Le pré-tri, les trois passes, `overhangs()` |
| `crates/swept-wasm/src/dto.rs` | `kerb_height`, `ground_clearance`, `metres_overhanging` |
| `web/index.html` | Deux champs de saisie |
| `web/src/domain/types.ts` | Les trois champs traversant la frontière |
| `web/src/main.ts` | Lecture des champs, carte de surplomb |
| `web/src/render/path.ts` | Le segment surplombant, tracé distinctement |
| `data/vehicles.json` | Champ `ground_clearance`, `schema_version` 2 |
| `docs/ALGORITHME.md` | La section sur les hauteurs |

Les tâches 1 et 2 sont mécaniquement lourdes et conceptuellement vides : elles
changent deux signatures que tout le dépôt appelle. Elles viennent en premier
pour que le bruit soit derrière nous avant que la logique commence, en Task 3.

---

### Task 1: La garde au sol et les roues

**Files:**
- Modify: `crates/swept-core/src/vehicle.rs`
- Modify: tous les appelants de `Vehicle::new` (22 sites)

**Interfaces:**
- Consumes: `crate::geometry::Point`
- Produces: `Vehicle::ground_clearance: f64` ;
  `Vehicle::new(wheelbase, length, front_overhang, width, mirror_width, ground_clearance, min_turning_radius) -> Result<Self, VehicleError>` ;
  `Vehicle::wheels(&self) -> [Point; 4]`

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-core/src/vehicle.rs` :

```rust
    #[test]
    fn the_wheels_sit_at_the_corners_of_the_body() {
        let wheels = lbx().wheels();
        let half = 1.825 / 2.0;
        // Rear axle at the origin, front axle a wheelbase ahead, both at the
        // body's own half-width — see `wheels` for why the body and not the
        // track.
        assert!((wheels[0].x - 0.0).abs() < EPS);
        assert!((wheels[0].y - half).abs() < EPS);
        assert!((wheels[1].x - 0.0).abs() < EPS);
        assert!((wheels[1].y + half).abs() < EPS);
        assert!((wheels[2].x - 2.580).abs() < EPS);
        assert!((wheels[2].y - half).abs() < EPS);
        assert!((wheels[3].x - 2.580).abs() < EPS);
        assert!((wheels[3].y + half).abs() < EPS);
    }

    #[test]
    fn rejects_a_ground_clearance_of_zero() {
        // A vehicle flat on the ground overhangs nothing, which would silently
        // turn every kerb back into a wall — the very state this batch leaves.
        let error = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.0, 5.2)
            .expect_err("zero is not a ground clearance");
        assert_eq!(error, VehicleError::NonPositive("ground_clearance"));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib vehicle`
Expected: FAIL — ``no method named `wheels` `` et ``this function takes 6 arguments but 7 arguments were supplied``.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-core/src/vehicle.rs`, ajouter le champ au struct, après
`mirror_width` :

```rust
    /// Height of the lowest point of the bodywork, wheels excluded, in metres.
    ///
    /// What decides whether the vehicle can overhang a kerb rather than be
    /// stopped by it. Manufacturers publish this figure, and three caveats
    /// come with it: it is quoted **unladen** — a loaded vehicle settles two
    /// to four centimetres — the measuring convention varies between makers,
    /// and **deformable parts are usually excluded**, though the front bumper
    /// lip is precisely what overhangs first when the vehicle turns.
    ///
    /// The bias therefore runs both ways: conservative on the flank, where a
    /// figure taken under the whole vehicle underestimates the sill, and
    /// possibly optimistic on the nose.
    pub ground_clearance: f64,
```

Étendre `new`, en insérant le paramètre avant le rayon pour garder les
dimensions groupées :

```rust
    pub fn new(
        wheelbase: f64,
        length: f64,
        front_overhang: f64,
        width: f64,
        mirror_width: f64,
        ground_clearance: f64,
        min_turning_radius: f64,
    ) -> Result<Self, VehicleError> {
```

Ajouter `ground_clearance` à la boucle de validation des valeurs strictement
positives, et au littéral `Self { .. }` construit en fin de fonction.

Puis, dans `impl Vehicle`, après `envelope` :

```rust
    /// The four contact patches, in the vehicle's own frame.
    ///
    /// At the corners of a `width × wheelbase` rectangle — the **body** width,
    /// not the track. The track separates the wheels' median planes, so the
    /// outer edge of a tyre sits half a tyre width further out: on the LBX,
    /// 1.56 m of track and 225 mm tyres put that edge 0.893 m from the axis
    /// against 0.913 m for half the body, because wheel arches follow tyres.
    /// Using half the body width therefore costs about two centimetres and
    /// spares the caller two measurements nobody has to hand — where the track
    /// alone would have cost thirteen.
    ///
    /// The two centimetres are not guaranteed to fall on the safe side: wide
    /// rims or a different offset can bring a tyre flush with the arch.
    #[must_use]
    pub fn wheels(&self) -> [Point; 4] {
        let half = self.width / 2.0;
        [
            Point::new(0.0, half),
            Point::new(0.0, -half),
            Point::new(self.wheelbase, half),
            Point::new(self.wheelbase, -half),
        ]
    }
```

Mettre ensuite à jour les 22 appels. La plupart tiennent sur une ligne :

```bash
python3 - <<'PY'
import pathlib, re
# ARBITRARY fixture value: 0.18 m is the order of magnitude for a compact SUV,
# which is what every fixture in the repo models.
pattern = re.compile(r'Vehicle::new\(([^()]*?),\s*([0-9.]+)\)')
for path in pathlib.Path('crates').rglob('*.rs'):
    text = path.read_text()
    fixed = pattern.sub(lambda m: f'Vehicle::new({m.group(1)}, 0.18, {m.group(2)})', text)
    if fixed != text:
        path.write_text(fixed)
        print(path)
PY
cargo fmt --all
```

Les appels multi-lignes ne sont pas capturés par ce motif ; le compilateur les
nommera à l'étape suivante. Il y en a **un**, dans
`crates/swept-wasm/src/dto.rs` (`VehicleDto::into_domain`), traité en Task 6 —
en attendant, y insérer `0.18` avant `self.min_turning_radius` pour que le
workspace compile.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib vehicle`
Expected: PASS.

Run: `cargo test --workspace`
Expected: PASS — les résultats numériques ne bougent pas, puisque rien ne lit
encore `ground_clearance`.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning. `too_many_arguments` se déclenche à huit paramètres ;
sept passent.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(core): give a vehicle a ground clearance and four wheels"
```

---

### Task 2: La hauteur portée par l'obstacle

**Files:**
- Modify: `crates/swept-core/src/scene/obstacles.rs`
- Modify: `crates/swept-core/src/scene/mod.rs`
- Modify: `crates/swept-core/src/scene/gate.rs`
- Modify: tous les littéraux `Scene { .. }` (une vingtaine)

**Interfaces:**
- Consumes: `crate::geometry::Obb`
- Produces: `scene::Obstacle { shape: Obb, height: f64 }`,
  `Obstacle::wall(shape: Obb) -> Self`, `Obstacle::low(shape: Obb, height: f64) -> Self` ;
  `Scene::kerb_height: f64` ; `Scene::obstacles(&self) -> Vec<Obstacle>`

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-core/src/scene/mod.rs` :

```rust
    #[test]
    fn only_the_pavement_is_low() {
        // Everything a scene contains is a wall except the two strips of
        // pavement, which a body can overhang. Getting this wrong in either
        // direction is invisible until a result is wrong.
        let mut scene = symmetric();
        scene.kerb_height = 0.12;
        let obstacles = scene.obstacles();
        let low: Vec<_> = obstacles.iter().filter(|o| o.height.is_finite()).collect();
        assert_eq!(low.len(), 2, "the pavement is split either side of the kerb");
        for obstacle in low {
            assert!((obstacle.height - 0.12).abs() < 1e-12);
        }
    }

    #[test]
    fn a_scene_without_a_pavement_has_nothing_low() {
        let mut scene = symmetric();
        scene.pavement_width = 0.0;
        scene.kerb_height = 0.12;
        assert!(scene.obstacles().iter().all(|o| !o.height.is_finite()));
    }

    #[test]
    fn a_kerb_declared_full_height_leaves_no_low_obstacle() {
        // How every pre-existing test keeps its results: an infinite kerb is
        // a wall, and the scene is exactly what it was before this batch.
        let mut scene = symmetric();
        scene.kerb_height = f64::INFINITY;
        assert!(scene.obstacles().iter().all(|o| !o.height.is_finite()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib scene`
Expected: FAIL — ``no field `kerb_height` on type `Scene` `` et
``no field `height` on type `Obb` ``.

- [ ] **Step 3: Write minimal implementation**

En tête de `crates/swept-core/src/scene/obstacles.rs`, après les constantes :

```rust
/// A rectangle a vehicle can hit, and how tall it stands.
///
/// Height is what lets a body pass over a kerb it would otherwise be stopped
/// by. It lives here rather than on [`Obb`] because it is a fact about the
/// scene, not about geometry: an [`Obb`] is a rectangle and should stay one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obstacle {
    /// Where it stands.
    pub shape: Obb,
    /// How tall it stands, in metres. Infinite for anything unclimbable.
    pub height: f64,
}

impl Obstacle {
    /// Something nothing can pass over: a wall, a post, a gate leaf.
    #[must_use]
    pub fn wall(shape: Obb) -> Self {
        Self {
            shape,
            height: f64::INFINITY,
        }
    }

    /// Something a high enough body can overhang.
    #[must_use]
    pub fn low(shape: Obb, height: f64) -> Self {
        Self { shape, height }
    }
}
```

Puis réécrire `build` pour envelopper chaque rectangle :

```rust
pub(super) fn build(scene: &Scene) -> Vec<Obstacle> {
    let left = scene.left_post.inner_edge_x;
    let right = scene.right_post.inner_edge_x;
    let left_outer = left - scene.left_post.width;
    let right_outer = right + scene.right_post.width;

    let mut obstacles = vec![
        // The wall either side of the posts.
        Obstacle::wall(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            left_outer,
            0.0,
            scene.wall_thickness,
        )),
        Obstacle::wall(Obb::from_bounds(
            right_outer,
            SCENE_HALF_EXTENT_M,
            0.0,
            scene.wall_thickness,
        )),
        // The posts themselves.
        Obstacle::wall(Obb::from_bounds(left_outer, left, 0.0, scene.left_post.depth)),
        Obstacle::wall(Obb::from_bounds(right, right_outer, 0.0, scene.right_post.depth)),
        // Whatever stands across the road.
        Obstacle::wall(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            SCENE_HALF_EXTENT_M,
            -(scene.pavement_width + scene.road_width) - FAR_SIDE_THICKNESS_M,
            -(scene.pavement_width + scene.road_width),
        )),
    ];

    // The pavement, split either side of the dropped kerb. The only thing in
    // a scene a vehicle can overhang.
    if scene.pavement_width > PAVEMENT_EPSILON_M {
        let half_kerb = scene.dropped_kerb_width / 2.0;
        let centre = f64::midpoint(left, right);
        obstacles.push(Obstacle::low(
            Obb::from_bounds(
                -SCENE_HALF_EXTENT_M,
                centre - half_kerb,
                -scene.pavement_width,
                0.0,
            ),
            scene.kerb_height,
        ));
        obstacles.push(Obstacle::low(
            Obb::from_bounds(
                centre + half_kerb,
                SCENE_HALF_EXTENT_M,
                -scene.pavement_width,
                0.0,
            ),
            scene.kerb_height,
        ));
    }

    if matches!(scene.gate, GateKind::Swinging { .. }) {
        obstacles.extend(super::gate::leaves(scene).into_iter().map(Obstacle::wall));
    }

    obstacles
}
```

`gate::leaves` garde sa signature — il rend des `Obb`, que `build` enveloppe.

Dans `crates/swept-core/src/scene/mod.rs`, ajouter le champ après
`road_width` :

```rust
    /// Height of the pavement kerb, in metres.
    ///
    /// The one thing in a scene a body can pass over. Set it to
    /// `f64::INFINITY` to get the pre-height behaviour, where a kerb stops
    /// everything — which is what the reference tests do, so that their
    /// results keep describing the world they were established in.
    pub kerb_height: f64,
```

Réexporter le type et corriger la signature :

```rust
pub use obstacles::Obstacle;

    /// Every rectangle a vehicle can hit, with the height each one stands at.
    #[must_use]
    pub fn obstacles(&self) -> Vec<Obstacle> {
        obstacles::build(self)
    }
```

Enfin, ajouter le champ à tous les littéraux `Scene { .. }` du dépôt. Ceux des
tests et des exemples prennent `f64::INFINITY` — c'est ce qui préserve leurs
résultats :

```bash
python3 - <<'PY'
import pathlib, re
pattern = re.compile(r'(\n(\s*)road_width: [^,\n]+,)')
for path in list(pathlib.Path('crates').rglob('*.rs')):
    text = path.read_text()
    fixed = pattern.sub(lambda m: f'{m.group(1)}\n{m.group(2)}kerb_height: f64::INFINITY,', text)
    if fixed != text:
        path.write_text(fixed)
        print(path)
PY
cargo fmt --all
```

Le motif n'attrape pas les littéraux qui n'écrivent pas `road_width` sur sa
propre ligne, ni la construction dans `crates/swept-wasm/src/dto.rs`
(`SceneDto::into_domain`) qui sera reprise en Task 6 — y ajouter
`kerb_height: f64::INFINITY` en attendant, pour que le workspace compile.
Le compilateur nomme tout ce qui manque : `missing field kerb_height`.

Dans les trois nouveaux tests, remplacer ensuite le `f64::INFINITY` par la
valeur que chacun exige (`0.12`, ou pas de trottoir du tout).

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --workspace`
Expected: PASS — **aucun résultat numérique ne bouge**, puisque toute bordure
est déclarée infinie et que `ClearanceField` ignore encore la hauteur.

Run: `cargo clippy --all-targets -- -D warnings`
Expected: aucun warning.

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat(core): let an obstacle know how tall it stands"
```

---

### Task 3: Le pré-tri et les trois passes

**Files:**
- Modify: `crates/swept-core/src/clearance.rs`

**Interfaces:**
- Consumes: `Vehicle::ground_clearance`, `Vehicle::wheels()`, `Scene::obstacles()`
- Produces: `ClearanceField` inchangé de l'extérieur — `new` et `at` gardent
  leurs signatures. Ce qui change est ce que `at` considère.

C'est la tâche qui porte le lot. Les deux précédentes n'ont fait que déplacer
des signatures ; celle-ci change ce qu'une marge veut dire.

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-core/src/clearance.rs` :

```rust
    /// A scene with a pavement the body can pass over.
    fn low_kerb_scene() -> Scene {
        let mut scene = wide_scene();
        scene.kerb_height = 0.12;
        scene
    }

    /// A pose with the nose over the pavement and every wheel off it.
    ///
    /// **Only an overhang can overhang.** The wheels sit at the corners of the
    /// body, so the flank is over a kerb exactly when a tyre is — which is
    /// physically right, a wheel arch following its tyre to within a couple of
    /// centimetres. What can pass over a kerb is therefore what sticks out
    /// beyond an axle: the front overhang, or the rear.
    ///
    /// Here the vehicle points into the yard on `wide_scene`, whose pavement
    /// runs from y = -1.20 to 0 and whose carriageway runs from -5.70 to
    /// -1.20. Rear axle at -4.20, front axle at -1.62 — both on the
    /// carriageway — and the nose 3.43 m ahead of the rear axle, reaching
    /// -0.77, which is over the pavement. `x = -6` is clear of the dropped
    /// kerb, which spans -1.60 to 1.60.
    fn overhanging_pose() -> Pose {
        Pose::new(-6.0, -4.2, Radians::from_degrees(90.0))
    }

    #[test]
    fn a_kerb_lower_than_the_ground_clearance_does_not_stop_the_body() {
        let vehicle = lbx();
        let over = ClearanceField::new(&low_kerb_scene(), &vehicle);
        assert_ne!(
            over.at(overhanging_pose()),
            Clearance::Collision,
            "a 12 cm kerb passes under an 18 cm ground clearance"
        );
    }

    #[test]
    fn the_same_pose_is_a_collision_when_the_kerb_is_a_wall() {
        // The other half of the previous test: without the height, this is
        // exactly the refusal the batch exists to remove.
        let field = ClearanceField::new(&wide_scene(), &lbx());
        assert_eq!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn a_low_kerb_still_stops_a_wheel() {
        // Straddling the kerb line, along the road. The near-side wheels land
        // at y = -0.09, up on the pavement, while the off-side pair stay at
        // -1.91 on the carriageway. The body may fly over a kerb; a tyre may
        // not leave what it can roll on, and that alone must refuse this.
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        let pose = Pose::new(-6.0, -1.0, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn a_wall_taller_than_the_ground_clearance_stops_everything() {
        let mut scene = wide_scene();
        scene.kerb_height = 0.40;
        let field = ClearanceField::new(&scene, &lbx());
        assert_eq!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn a_kerb_exactly_at_the_ground_clearance_is_overhung() {
        // The boundary, pinned deliberately: blocking is `height > clearance`,
        // so equality passes. A model that hesitated on the millimetre would
        // serve nobody.
        let mut scene = wide_scene();
        scene.kerb_height = 0.18;
        let field = ClearanceField::new(&scene, &lbx());
        assert_ne!(field.at(overhanging_pose()), Clearance::Collision);
    }

    #[test]
    fn an_overhung_obstacle_contributes_no_distance() {
        // The subtle half of the rule. If a kerb the body flies over still
        // counted towards the margin, the margin would collapse to zero the
        // moment a bumper crossed the line — which is the very refusal this
        // batch removes, wearing a different mask.
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        match field.at(overhanging_pose()) {
            Clearance::Clear(margin) => assert!(margin > 0.05, "got {margin}"),
            Clearance::Collision => panic!("the body overhangs this kerb"),
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib clearance`
Expected: FAIL — `a_kerb_lower_than_the_ground_clearance_does_not_stop_the_body`
et les deux qui en dépendent échouent, la hauteur n'étant pas encore lue.
`the_same_pose_is_a_collision_when_the_kerb_is_a_wall` passe déjà : c'est le
comportement actuel, et il doit survivre.

- [ ] **Step 3: Write minimal implementation**

Remplacer la doc de module, le struct et `new` dans
`crates/swept-core/src/clearance.rs` :

```rust
//! How much room a given pose leaves.
//!
//! Three tests run, and each exists for a case the others miss. The body walks
//! the sampled outline against what it cannot pass over. The wheels walk four
//! contact points against everything, kerbs included — a body may overhang a
//! kerb, a tyre may not leave what it can roll on. The reverse test walks
//! obstacle corners against the body rectangle, because a pillar corner can
//! sit inside the body without any sampled point falling inside the pillar.
//!
//! # Heights are compared once
//!
//! [`ClearanceField::at`] is the hot path of the whole project — a fine sweep
//! calls it hundreds of thousands of times. So no height is ever compared
//! there. They are compared once, here, when the field is built, and the
//! obstacles filed into two disjoint lists: what the body hits, and what it
//! flies over.

use crate::geometry::{Obb, Point, PointDistance};
use crate::kinematics::Pose;
use crate::scene::Scene;
use crate::vehicle::Vehicle;

// … Clearance et CORNER_TEST_MAX_HALF_SIZE_M inchangés …

/// Everything about a scene and a vehicle that does not depend on the pose.
#[derive(Debug, Clone)]
pub struct ClearanceField {
    /// What the body hits: taller than the vehicle's ground clearance.
    blocking: Vec<Obb>,
    /// What the body flies over, and only the wheels hit.
    overhung: Vec<Obb>,
    corners: Vec<Point>,
    envelope: Vec<Point>,
    wheels: [Point; 4],
    half_width: f64,
    rear: f64,
    front: f64,
}

impl ClearanceField {
    /// Prepares the field for one scene and one vehicle.
    #[must_use]
    pub fn new(scene: &Scene, vehicle: &Vehicle) -> Self {
        // Strictly taller blocks: a kerb exactly at the ground clearance is
        // overhung. See the boundary test in this module.
        let (blocking, overhung): (Vec<_>, Vec<_>) = scene
            .obstacles()
            .into_iter()
            .partition(|o| o.height > vehicle.ground_clearance);
        let blocking: Vec<Obb> = blocking.into_iter().map(|o| o.shape).collect();
        let overhung: Vec<Obb> = overhung.into_iter().map(|o| o.shape).collect();

        // Only blocking obstacles need the corner test: a kerb corner inside
        // the body is an overhang, not a collision.
        let corners = blocking
            .iter()
            .filter(|o| {
                o.half_width <= CORNER_TEST_MAX_HALF_SIZE_M
                    && o.half_height <= CORNER_TEST_MAX_HALF_SIZE_M
            })
            .flat_map(Obb::corners)
            .collect();

        Self {
            blocking,
            overhung,
            corners,
            envelope: vehicle.envelope(),
            wheels: vehicle.wheels(),
            half_width: vehicle.width / 2.0,
            rear: -vehicle.rear_overhang,
            front: vehicle.wheelbase + vehicle.front_overhang,
        }
    }

    /// The clearance left by one pose.
    #[must_use]
    pub fn at(&self, pose: Pose) -> Clearance {
        let (sin, cos) = pose.heading.sin_cos();
        let place = |local: &Point| {
            Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            )
        };

        let mut smallest = f64::MAX;

        // The body, against what it cannot pass over. An overhung obstacle is
        // ignored outright — neither collision nor distance.
        for local in &self.envelope {
            let point = place(local);
            for obstacle in &self.blocking {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // The wheels, against everything.
        for local in &self.wheels {
            let point = place(local);
            for obstacle in self.blocking.iter().chain(&self.overhung) {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // Reverse test: a blocking obstacle's corner inside the vehicle body.
        for corner in &self.corners {
            let (dx, dy) = (corner.x - pose.x, corner.y - pose.y);
            let local_x = dx * cos + dy * sin;
            let local_y = -dx * sin + dy * cos;
            if local_x > self.rear
                && local_x < self.front
                && local_y > -self.half_width
                && local_y < self.half_width
            {
                return Clearance::Collision;
            }
        }

        Clearance::Clear(smallest)
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-core --lib clearance`
Expected: PASS — les quatre tests d'origine et les six nouveaux.

Run: `cargo test --workspace`
Expected: PASS. Toute bordure des tests existants étant infinie, `overhung` y
est vide et le comportement est identique au bit près.

**Si un test existant régresse**, ce n'est pas la hauteur : c'est le pré-tri qui
a rangé un obstacle du mauvais côté, ou la boucle des roues qui trouve une
collision que l'enveloppe manquait. Le comparer contre `git stash` avant de
toucher à une assertion. **Ne jamais relâcher une assertion existante.**

- [ ] **Step 5: Commit**

```bash
git add crates/swept-core/src/clearance.rs
git commit -m "feat(core): let a body overhang what is lower than its underside"
```

---

### Task 4: Le prédicat de surplomb

**Files:**
- Modify: `crates/swept-core/src/clearance.rs`

**Interfaces:**
- Consumes: le champ `overhung` de la Task 3
- Produces: `ClearanceField::overhangs(&self, pose: Pose) -> bool`

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-core/src/clearance.rs` :

```rust
    #[test]
    fn a_pose_over_the_pavement_is_reported_as_overhanging() {
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        assert!(field.overhangs(overhanging_pose()));
    }

    #[test]
    fn a_pose_out_on_the_road_overhangs_nothing() {
        let field = ClearanceField::new(&low_kerb_scene(), &lbx());
        assert!(!field.overhangs(Pose::new(-6.0, -3.5, Radians::default())));
    }

    #[test]
    fn nothing_overhangs_a_scene_whose_kerb_is_a_wall() {
        // With no overhung obstacle there is nothing to overhang, so the
        // reference tests have no new quantity to account for.
        let field = ClearanceField::new(&wide_scene(), &lbx());
        assert!(!field.overhangs(overhanging_pose()));
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-core --lib clearance::tests::a_pose_over`
Expected: FAIL — ``no method named `overhangs` ``.

- [ ] **Step 3: Write minimal implementation**

Ajouter à `impl ClearanceField`, après `at` :

```rust
    /// Does any part of the body sit over an obstacle it is passing above?
    ///
    /// Reported rather than penalised. A bumper crossing a pavement is legal
    /// geometry and worth knowing about all the same: the model is flat, and
    /// knows nothing of the bollard, sign or post that so often stands there.
    ///
    /// Measured on a finished trajectory, never inside a search — like the
    /// alert distances, and for the same reason.
    #[must_use]
    pub fn overhangs(&self, pose: Pose) -> bool {
        if self.overhung.is_empty() {
            return false;
        }
        let (sin, cos) = pose.heading.sin_cos();
        self.envelope.iter().any(|local| {
            let point = Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            );
            self.overhung
                .iter()
                .any(|o| matches!(o.distance_to(point), PointDistance::Inside))
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
git add crates/swept-core/src/clearance.rs
git commit -m "feat(core): report when a body is passing over something"
```

---

### Task 5: Le test qui porte le lot

**Files:**
- Modify: `crates/swept-solver/tests/reference_results.rs`

**Interfaces:**
- Consumes: tout ce qui précède, via `swept_solver::exact::search`
- Produces: rien de nouveau. Cette tâche atteste que le lot répond à la
  question qui l'a motivé.

- [ ] **Step 1: Write the failing test**

Ajouter à `crates/swept-solver/tests/reference_results.rs` :

```rust
/// What this batch was built for.
///
/// The pure 2D model treats a kerb as a wall of infinite height, so the only
/// candidates the exhaustive sweep refused on this gateway were those whose
/// front overhang swings over the pavement beside the dropped kerb. Declaring
/// the kerb for what it is can only help — never hinder, since every candidate
/// that was drivable before still is.
#[test]
fn a_low_kerb_never_costs_room_and_may_buy_some() {
    let vehicle = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 0.18, 3.59)
        .expect("valid vehicle");

    let mut walled = fabiens_gateway();
    walled.kerb_height = f64::INFINITY;
    let mut low = fabiens_gateway();
    // MEASURED — a standard French T2 kerb stands 12 cm above the gutter.
    low.kerb_height = 0.12;

    let walled_best = search(&vehicle, &walled, Approach::Forward, Grid::fine());
    let low_best = search(&vehicle, &low, Approach::Forward, Grid::fine());

    match (walled_best.best(), low_best.best()) {
        (Some(w), Some(l)) => assert!(
            l.min_clearance >= w.min_clearance - 1e-9,
            "a wall gave {:.1} cm, a kerb gave {:.1} cm",
            w.min_clearance * 100.0,
            l.min_clearance * 100.0
        ),
        (Some(_), None) => panic!("lowering the kerb removed an entry that existed"),
        (None, _) => { /* nothing to compare, and the batch is not at fault */ }
    }
}
```

Vérifier que `fabiens_gateway()` définit `kerb_height` — la Task 2 y a inscrit
`f64::INFINITY`, ce que ce test remplace localement.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-solver --test reference_results a_low_kerb`
Expected: PASS. **C'est un test de non-régression, pas de fonctionnalité** : il
peut passer dès l'abord si la bordure ne contraignait aucune trajectoire
retenue. Ce qu'il interdit, c'est qu'abaisser une bordure *retire* de la marge,
ce qui ne pourrait venir que d'un pré-tri fautif.

Pour savoir ce que la hauteur achète réellement, mesurer :

```bash
cargo run -p swept-solver --release --example bench
```

- [ ] **Step 3: Write minimal implementation**

Aucun code de production. Si l'assertion échoue, la cause est dans la Task 3 —
un obstacle rangé du mauvais côté, ou la boucle des roues.

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-solver/tests/reference_results.rs
git commit -m "test(solver): a kerb declared low may buy room and never costs any"
```

---

### Task 6: La frontière WebAssembly

**Files:**
- Modify: `crates/swept-wasm/src/dto.rs`

**Interfaces:**
- Consumes: `ClearanceField::overhangs`, `Vehicle::new` à sept paramètres,
  `Scene::kerb_height`
- Produces: `SceneDto::kerb_height: f64`, `VehicleDto::ground_clearance: f64`,
  `ManeuverDto::metres_overhanging: f64`

- [ ] **Step 1: Write the failing test**

Ajouter au bloc `mod tests` de `crates/swept-wasm/src/dto.rs` :

```rust
    #[test]
    fn a_walled_kerb_leaves_nothing_overhanging() {
        let mut scene = scene_dto();
        scene.kerb_height = f64::INFINITY;
        let response = run_solve(SolveRequest {
            scene,
            vehicle: vehicle_dto(),
            forward_only: None,
        })
        .expect("valid dimensions");
        for alternative in &response.alternatives {
            assert_eq!(alternative.metres_overhanging, 0.0);
        }
    }

    #[test]
    fn overhang_never_exceeds_the_distance_travelled() {
        // The same guard the alert bands carry: a length summed from pose
        // spacing cannot exceed the length it is summed from.
        let mut scene = scene_dto();
        scene.kerb_height = 0.12;
        let response = run_solve(SolveRequest {
            scene,
            vehicle: vehicle_dto(),
            forward_only: None,
        })
        .expect("valid dimensions");
        for alternative in &response.alternatives {
            assert!(
                alternative.metres_overhanging <= alternative.distance + 1e-9,
                "{} m overhanging out of {} m travelled",
                alternative.metres_overhanging,
                alternative.distance
            );
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p swept-wasm`
Expected: FAIL — ``no field `kerb_height` on type `SceneDto` `` et
``no field `metres_overhanging` on type `ManeuverDto` ``.

- [ ] **Step 3: Write minimal implementation**

Dans `crates/swept-wasm/src/dto.rs` :

Ajouter à `SceneDto`, après `road_width`, et le reporter dans `into_domain` :

```rust
    /// Kerb height, in metres. Infinite for a kerb nothing passes over.
    pub kerb_height: f64,
```

Ajouter à `VehicleDto`, après `mirror_width`, et le passer à `Vehicle::new`
comme sixième argument :

```rust
    /// Lowest point of the bodywork, wheels excluded, in metres.
    pub ground_clearance: f64,
```

Ajouter à `ManeuverDto`, après `metres_under_10cm` :

```rust
    /// Distance travelled with part of the body over a low obstacle, in metres.
    ///
    /// Legal geometry, and worth reporting: the model is flat and knows
    /// nothing of the bollard or sign that so often stands on a pavement.
    pub metres_overhanging: f64,
```

**Le drapeau va aussi sur la pose, et il le faut.** Un total ne suffit pas à
dessiner : le rendu segmente le tracé en comparant des poses voisines
(`pathToPrimitives`, via `keyOf`), donc il lui faut savoir, pose par pose, ce
qui surplombe. Ajouter à `PoseDto`, après `clearance` :

```rust
    /// `true` when part of the body sits over a low obstacle at this pose.
    pub overhanging: bool,
```

Il se renseigne dans le `map` qui construit les `PoseDto`, à côté de
`clearance` :

```rust
            overhanging: field.overhangs(step.pose),
```

La distance se somme ensuite dans la boucle qui existe déjà — même parcours,
même espacement réel entre poses, et le drapeau est déjà là :

```rust
    let mut distance = 0.0;
    let mut under = [0.0_f64; 2];
    let mut overhanging = 0.0;
    for pair in poses.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let span = (b.x - a.x).hypot(b.y - a.y);
        distance += span;
        for (i, threshold) in ALERT_BANDS_M.iter().enumerate() {
            if b.clearance < *threshold {
                under[i] += span;
            }
        }
        if b.overhanging {
            overhanging += span;
        }
    }
```

et le champ se renseigne avec `metres_overhanging: overhanging`.

Ajouter enfin `kerb_height: f64::INFINITY` et `ground_clearance: 0.18` aux
fixtures `scene_dto()` et `vehicle_dto()` du bloc de tests, puis les trois
tests ci-dessus les surchargent.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test -p swept-wasm`
Expected: PASS — les neuf tests existants et les deux nouveaux.

Run: `just wasm`
Expected: le paquet se construit.

- [ ] **Step 5: Commit**

```bash
git add crates/swept-wasm/src/dto.rs
git commit -m "feat(wasm): carry kerb height, ground clearance and overhang across"
```

---

### Task 7: L'interface

**Files:**
- Modify: `web/index.html`
- Modify: `web/src/domain/types.ts`
- Modify: `web/src/main.ts`
- Modify: `web/src/render/path.ts`
- Test: `web/src/render/path.test.ts`

**Interfaces:**
- Consumes: les trois champs de la Task 6
- Produces: rien pour le Rust.

- [ ] **Step 1: Write the failing test**

Le rendu segmente déjà le tracé en groupant les poses voisines qui partagent
une clé — `pathToPrimitives`, dont `keyOf` combine le sens de marche et la
bande de marge. Le surplomb entre dans cette clé, ce qui découpe le tracé au
bon endroit sans toucher à l'algorithme.

Ajouter à `web/src/render/path.test.ts`, en reprenant le style des cas déjà
présents dans ce fichier :

```ts
  it("splits the path where the body starts overhanging", () => {
    const maneuver = {
      ...manoeuvre(),
      poses: [
        { x: -8, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: false },
        { x: -7, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: false },
        { x: -6, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: true },
        { x: -5, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: true },
      ],
    };
    const roles = pathToPrimitives(maneuver, vehicle())
      .filter((p) => p.type === "polyline")
      .map((p) => p.role);
    expect(roles).toContain("overhang");
    expect(roles.filter((r) => r === "overhang")).toHaveLength(1);
  });

  it("marks nothing when nothing overhangs", () => {
    const maneuver = {
      ...manoeuvre(),
      poses: [
        { x: -8, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: false },
        { x: -7, y: -3.5, heading: 0, reverse: false, clearance: 1.0, overhanging: false },
      ],
    };
    const roles = pathToPrimitives(maneuver, vehicle())
      .filter((p) => p.type === "polyline")
      .map((p) => p.role);
    expect(roles).not.toContain("overhang");
  });
```

`manoeuvre()` et `vehicle()` sont les fixtures déjà définies en tête de
`path.test.ts` ; si elles portent d'autres noms, reprendre ceux du fichier et
compléter leurs poses avec `overhanging: false`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/render/path.test.ts`
Expected: FAIL.

- [ ] **Step 3: Write minimal implementation**

Dans `web/index.html`, à côté de `road`, un champ de scène :

```html
          <label class="grid gap-1">
            <span class="text-sm text-stone-600">Hauteur de bordure</span>
            <input id="kerb-height" type="number" min="0" step="0.01" value="0.12" class="rounded border border-stone-300 px-3 py-2" />
          </label>
```

et à côté de `body-width`, un champ de véhicule :

```html
          <label class="grid gap-1">
            <span class="text-sm text-stone-600">Garde au sol</span>
            <input id="ground-clearance" type="number" min="0" step="0.001" value="0.180" class="rounded border border-stone-300 px-3 py-2" />
          </label>
```

Dans `web/src/domain/types.ts`, ajouter `kerb_height: number` à la scène,
`ground_clearance: number` au véhicule, `metres_overhanging: number` à la
manœuvre et `overhanging: boolean` à la pose.

Dans `web/src/main.ts`, lire les deux champs dans `readRequest`, exactement
comme les voisins, et ajouter la carte au bloc de statistiques — affichée
seulement quand elle apprend quelque chose :

```ts
    ...(maneuver.metres_overhanging > 0
      ? [card("Surplomb du trottoir", metres(maneuver.metres_overhanging))]
      : []),
```

Dans `web/src/render/path.ts`, faire entrer le surplomb dans la clé de
segmentation et lui donner son rôle :

```ts
  const keyOf = (pose: PoseDto) =>
    `${pose.reverse}|${bandOf(pose.clearance)}|${pose.overhanging}`;
```

puis, au moment de pousser la polyligne, choisir le rôle `"overhang"` lorsque
`last.overhanging` et le rôle de bande sinon :

```ts
      role: last.overhanging ? "overhang" : BAND_ROLES[bandOf(last.clearance)]!,
```

Déclarer `"overhang"` dans le type de rôle et lui donner un trait distinct dans
la feuille de style, puis l'ajouter à la légende de `web/index.html` à côté des
quatre bandes de marge, avec le libellé « surplomb du trottoir ».

- [ ] **Step 4: Run test to verify it passes**

Run: `cd web && npx vitest run`
Expected: PASS.

Run: `just ci`
Expected: PASS de bout en bout, `tsc --noEmit` compris.

- [ ] **Step 5: Commit**

```bash
git add web/
git commit -m "feat(web): let a kerb have a height, and say when the body clears it"
```

---

### Task 8: Les données et la documentation

**Files:**
- Modify: `data/vehicles.json`
- Modify: `docs/ALGORITHME.md`

**Interfaces:**
- Consumes: tout ce qui précède
- Produces: rien de nouveau.

- [ ] **Step 1: Write the failing test**

Le test, ici, est `tsc` plus la lecture de la base par l'interface. Dans
`data/vehicles.json`, porter `schema_version` à `2` et ajouter à chacun des
quatre modèles, après `width_mirrors_folded` :

```json
      "ground_clearance": { "v": null, "source": null },
```

Aucune valeur n'est renseignée : elles demandent les documents constructeurs,
et la règle du projet interdit de les prendre chez un agrégateur.

- [ ] **Step 2: Run test to verify it fails**

Run: `cd web && npx vitest run src/domain/vehicles.test.ts`
Expected: PASS si le lecteur ignore les champs inconnus, FAIL s'il valide la
version. Dans les deux cas, vérifier ensuite dans `web/src/domain/vehicles.ts`
qu'un `ground_clearance` nul se lit **« à saisir »** et jamais zéro : une garde
au sol nulle ferait de tout obstacle un mur, ce qui est l'état dont ce lot sort.

- [ ] **Step 3: Write minimal implementation**

Dans `docs/ALGORITHME.md`, ajouter une section après celle qui décrit le
calcul de marge :

```markdown
## La hauteur des obstacles

Un obstacle n'est plus un mur, mais un mur d'une certaine hauteur. Une bordure
de trottoir de douze centimètres se surplombe ; un muret de quarante arrête le
pare-chocs. La comparaison se fait entre cette hauteur et la **garde au sol**
du véhicule — le point le plus bas de la carrosserie, roues exclues.

Trois règles, et chacune existe pour un cas que les autres manquent :

1. **La carrosserie** ne voit que ce qu'elle ne peut pas survoler. Un obstacle
   surplombé est ignoré *entièrement* : ni collision, ni distance. Compter la
   distance ferait tomber la marge à zéro dès qu'un pare-chocs déborde, ce qui
   serait le même refus sous un autre masque.
2. **Les quatre roues** voient tout, bordures comprises. Une carrosserie
   surplombe, un pneu ne quitte pas ce sur quoi il roule.
3. **Le test inverse des coins** ne porte que sur les obstacles bloquants : un
   coin de trottoir à l'intérieur de la caisse est un surplomb.

Les hauteurs ne sont jamais comparées dans la boucle chaude. Elles le sont une
fois, à la construction du champ de marge, qui range les obstacles en deux
listes disjointes. Le coût par pose est celui d'avant, plus quatre points.

Le surplomb est **signalé, jamais pénalisé** : le résultat rapporte la distance
parcourue au-dessus d'une bordure, et le solveur ne change pas de comportement.
Le modèle reste plan — il ne connaît ni la borne, ni le panneau, ni le poteau
qui se dresse si souvent sur un trottoir, et c'est précisément pourquoi cette
distance est dite plutôt que tue.
```

- [ ] **Step 4: Run the full suite**

Run: `just ci`
Expected: PASS de bout en bout.

- [ ] **Step 5: Commit**

```bash
git add data/vehicles.json docs/ALGORITHME.md
git commit -m "docs: describe obstacle heights, and make room for ground clearance in the data"
```

---

## Vérification finale du lot

- [ ] `just ci` passe.
- [ ] `git diff main --stat` ne touche pas `crates/swept-solver/src/` : le lot
      change ce qu'une marge signifie, pas la façon dont elle est cherchée.
- [ ] Aucune assertion existante n'a été affaiblie. Les scènes des tests
      d'origine déclarent `kerb_height: f64::INFINITY` et rendent les mêmes
      chiffres qu'avant le lot.
- [ ] Les vecteurs dorés passent sans modification : ils portent sur la
      distance point-rectangle, le chevauchement et l'intégration, dont aucun
      ne connaît la scène.
- [ ] `cargo run -p swept-solver --release --example bench` : une passe fine
      reste sous la seconde. Les quatre points de roue ajoutent environ 13 % de
      tests par pose ; si le coût dépasse, le dire dans la PR plutôt que de
      réduire une grille en silence.
- [ ] Sur le portail de Fabien, l'interface affiche la carte « Surplomb du
      trottoir » quand la trajectoire déborde, et rien quand elle ne déborde
      pas.
- [ ] La garde au sol est saisissable et sa valeur par défaut est visible :
      c'est le paramètre qui fait basculer le verdict au centimètre.
