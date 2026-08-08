# Lot 1a — Noyau géométrique Rust — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Porter la géométrie, la cinématique et le calcul de marge du prototype JavaScript vers une crate Rust `swept-core` testée, documentée et sans dépendance externe.

**Architecture:** Une seule crate de domaine dans un workspace Cargo à trois membres. `swept-core` ne dépend de rien : ni sérialisation, ni plateforme, ni horloge. Chaque brique portée est validée soit par des tests écrits à la main, soit par des vecteurs de référence extraits du prototype gelé et comparés à 1e-9.

**Tech Stack:** Rust 1.97.1 (édition 2024), Cargo workspace, GitHub Actions. Node 24 pour le seul harnais d'extraction des vecteurs de référence.

## Global Constraints

- Toolchain Rust **1.97.1**, **édition 2024**, épinglée par `rust-toolchain.toml`.
- `swept-core` n'a **aucune dépendance externe**. Toute tentative d'en ajouter une est un signal d'erreur de conception.
- `#![deny(missing_docs)]` sur la crate. La documentation manquante casse le build.
- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, rustdoc, noms de tests, noms de branches et messages de commit. Seule la documentation projet (`docs/`) reste en français.
- Longueurs en **mètres** (`f64`), angles en **radians** (type `Radians`). Les degrés n'existent qu'à l'affichage, hors de cette crate.
- **Aucune constante numérique nue.** Chaque valeur est une `const` nommée, documentée par sa justification et sa provenance. Celles reprises du prototype sans justification connue sont marquées `ARBITRARY — carried over from the prototype, to be revalidated`.
- Licence de `swept-core` : `MIT OR Apache-2.0`.
- Repère : origine au milieu du passage, `y = 0` au nu extérieur du mur, `y > 0` vers la cour, `x` le long de la voie.
- `nvm` ne se charge pas dans les shells non interactifs. Tout script Node référence Node explicitement ou passe par `npm run`.
- Une seule PR ouverte à la fois, branchée sur `main`. Chaque tâche de ce plan est une PR.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `Cargo.toml` | Workspace à trois membres, métadonnées partagées |
| `rust-toolchain.toml` | Épinglage de la toolchain |
| `.github/workflows/ci.yml` | fmt, clippy, test, doc |
| `crates/swept-core/src/lib.rs` | Déclaration des modules, `deny(missing_docs)`, doc de crate |
| `crates/swept-core/src/units.rs` | `Radians` |
| `crates/swept-core/src/geometry.rs` | `Point`, `Obb`, coins, distance point→OBB, recouvrement SAT |
| `crates/swept-core/src/kinematics.rs` | `Pose`, `Direction`, intégration à courbure constante |
| `crates/swept-core/src/vehicle.rs` | `Vehicle`, validation, enveloppe échantillonnée |
| `crates/swept-core/src/scene/mod.rs` | `Scene`, `GateKind`, `Post` |
| `crates/swept-core/src/scene/gate.rs` | Vantail battant, collision avec le pilier, angle maximal |
| `crates/swept-core/src/scene/obstacles.rs` | Génération de la liste d'obstacles |
| `crates/swept-core/src/clearance.rs` | `Clearance`, marge d'une pose contre une scène |
| `tools/extract-golden/` | Harnais Node jetable produisant les vecteurs de référence |
| `crates/swept-core/tests/golden.rs` | Comparaison aux vecteurs de référence |

---

### Task 1: Fondations du dépôt

Le dépôt ne contient qu'un commit initial avec `LICENSE`. Tout le reste — `CLAUDE.md`, `README.md`, `docs/`, `data/`, `prototype/` — est non suivi. Cette tâche fige la base de comparaison.

**Files:**
- Create: `.gitignore`
- Commit: `CLAUDE.md`, `README.md`, `docs/`, `data/`, `prototype/`

**Interfaces:**
- Consumes: rien
- Produces: un arbre de travail propre, `prototype/index.html` figé comme référence de comportement pour toutes les tâches suivantes

- [ ] **Step 1: Créer le `.gitignore`**

```gitignore
# macOS
.DS_Store

# Rust
/target
**/*.rs.bk

# Node
node_modules/
dist/

# Wasm généré
crates/swept-wasm/pkg/

# Environnements locaux
.env
.env.local
.vercel
```

- [ ] **Step 2: Vérifier ce qui sera ajouté**

Run: `git status --short`
Expected: `.DS_Store` n'apparaît plus ; `CLAUDE.md`, `README.md`, `data/`, `docs/`, `prototype/` et `.gitignore` sont listés en `??`.

- [ ] **Step 3: Commiter**

```bash
git checkout -b chore/repo-foundations
git add .gitignore CLAUDE.md README.md data/ docs/ prototype/
git commit -m "chore: check in existing work and freeze the reference prototype

The single-file prototype becomes the behavioural reference for the port.
It no longer changes: every fix from here lands in the Rust core."
```

- [ ] **Step 4: Vérifier que l'arbre est propre**

Run: `git status --short`
Expected: aucune sortie.

---

### Task 2: Workspace Cargo, toolchain et intégration continue

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml`, `crates/swept-core/Cargo.toml`, `crates/swept-core/src/lib.rs`, `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: rien
- Produces: la crate `swept_core` compilable et testable ; `cargo test -p swept-core`, `cargo clippy -- -D warnings`, `cargo fmt --check` et `cargo doc` sont les commandes de vérification de toutes les tâches suivantes

- [ ] **Step 1: Installer la cible Wasm et wasm-pack**

Ces outils ne sont pas encore présents sur le poste. Ils ne servent qu'au lot 1c, mais l'installation est faite ici pour que la toolchain soit complète d'emblée.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-pack
```

Expected: `rustup target list --installed` mentionne `wasm32-unknown-unknown`.

- [ ] **Step 2: Créer le manifeste du workspace**

Fichier `Cargo.toml` :

```toml
[workspace]
members = ["crates/swept-core"]
resolver = "3"

[workspace.package]
edition = "2024"
rust-version = "1.97.1"
authors = ["Fabien D."]
repository = "https://github.com/FabienD/swept-path"

[workspace.lints.rust]
missing_docs = "deny"

[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
missing_panics_doc = "warn"
missing_errors_doc = "warn"
```

- [ ] **Step 3: Épingler la toolchain**

Fichier `rust-toolchain.toml` :

```toml
[toolchain]
channel = "1.97.1"
components = ["rustfmt", "clippy"]
targets = ["wasm32-unknown-unknown"]
```

- [ ] **Step 4: Créer la crate de domaine**

Fichier `crates/swept-core/Cargo.toml` :

```toml
[package]
name = "swept-core"
version = "0.1.0"
description = "Geometry and kinematics for swept path analysis of a vehicle through a narrow opening"
license = "MIT OR Apache-2.0"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
repository.workspace = true

[dependencies]

[lints]
workspace = true
```

La section `[dependencies]` est vide et doit le rester : c'est ce qui rend cette crate publiable sous une licence distincte de l'application.

- [ ] **Step 5: Écrire le test qui échoue**

Fichier `crates/swept-core/src/lib.rs` :

```rust
//! Geometry, kinematics and clearance computation for swept path analysis.
//!
//! This crate answers one question: can a given vehicle get through a given
//! opening, and with how much room to spare. It knows nothing about the web,
//! about serialisation, or about wall-clock time — the same inputs always
//! produce the same outputs.
//!
//! # Frame of reference
//!
//! The origin sits at the middle of the opening. `y = 0` is the outer face of
//! the wall, `y > 0` points into the yard, and `x` runs along the road.
//! Lengths are metres, angles are [`units::Radians`].

/// Crate version, exposed so that callers can report which core produced a
/// result.
#[must_use]
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_its_version() {
        assert_eq!(version(), "0.1.0");
    }
}
```

- [ ] **Step 6: Vérifier que tout passe**

Run: `cargo test -p swept-core && cargo clippy --all-targets -- -D warnings && cargo fmt --check && cargo doc -p swept-core --no-deps`
Expected: le test passe, aucun avertissement clippy, le formatage est conforme, la documentation se génère.

- [ ] **Step 7: Vérifier que `deny(missing_docs)` mord**

Ajouter temporairement dans `lib.rs` :

```rust
pub fn undocumented() {}
```

Run: `cargo build -p swept-core`
Expected: FAIL avec `error: missing documentation for a function`. Retirer ensuite la fonction et vérifier que le build repasse.

Cette étape n'est pas décorative : elle prouve que la contrainte de documentation est réellement appliquée, plutôt que déclarée.

- [ ] **Step 8: Écrire la CI**

Fichier `.github/workflows/ci.yml` :

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:

env:
  CARGO_TERM_COLOR: always

jobs:
  rust:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: 1.97.1
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: Format
        run: cargo fmt --check
      - name: Clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: Tests
        run: cargo test --workspace
      - name: Docs
        run: cargo doc --workspace --no-deps
        env:
          RUSTDOCFLAGS: -D warnings
```

- [ ] **Step 9: Commiter**

```bash
git checkout -b feat/cargo-workspace
git add Cargo.toml Cargo.lock rust-toolchain.toml crates/ .github/
git commit -m "feat: Cargo workspace, pinned toolchain and quality CI

deny(missing_docs) at the workspace level, clippy pedantic treated as an
error in CI. The domain crate has no dependencies, which is the condition
for publishing it under a separate licence."
```

---

### Task 3: Le type `Radians`

Seule unité à recevoir un type dédié. Les longueurs restent des `f64` en mètres : la confusion entre unités de longueur n'existe pas dans ce domaine, alors que la conversion degrés/radians traverse toute l'interface.

**Files:**
- Create: `crates/swept-core/src/units.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: la crate de la Task 2
- Produces: `units::Radians`, avec `Radians::new(f64)`, `Radians::from_degrees(f64)`, `.get() -> f64`, `.to_degrees() -> f64`, `.sin_cos() -> (f64, f64)`, et les opérateurs `Add`, `Sub`, `Neg`. Utilisé par toutes les tâches suivantes.

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/units.rs`, section de test uniquement pour l'instant :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    const EPS: f64 = 1e-12;

    #[test]
    fn converts_degrees_to_radians() {
        assert!((Radians::from_degrees(180.0).get() - PI).abs() < EPS);
        assert!((Radians::from_degrees(90.0).get() - FRAC_PI_2).abs() < EPS);
    }

    #[test]
    fn round_trips_through_degrees() {
        assert!((Radians::from_degrees(37.5).to_degrees() - 37.5).abs() < EPS);
    }

    #[test]
    fn adds_subtracts_and_negates() {
        let a = Radians::from_degrees(90.0);
        let b = Radians::from_degrees(30.0);
        assert!(((a + b).to_degrees() - 120.0).abs() < EPS);
        assert!(((a - b).to_degrees() - 60.0).abs() < EPS);
        assert!(((-b).to_degrees() + 30.0).abs() < EPS);
    }

    #[test]
    fn yields_sine_and_cosine_together() {
        let (sin, cos) = Radians::from_degrees(90.0).sin_cos();
        assert!((sin - 1.0).abs() < EPS);
        assert!(cos.abs() < EPS);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core units`
Expected: FAIL — `cannot find type Radians in this scope`, le module n'étant pas encore déclaré ni le type défini.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/units.rs`, avant le module de test :

```rust
//! Units used across the domain.
//!
//! Lengths are plain `f64` in metres. Angles get a dedicated type because the
//! user interface works in degrees while every computation here works in
//! radians, and that is the one unit confusion this domain actually suffers
//! from.

use std::ops::{Add, Neg, Sub};

/// An angle, in radians.
///
/// ```
/// use swept_core::units::Radians;
///
/// let right = Radians::from_degrees(90.0);
/// assert!((right.get() - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct Radians(f64);

impl Radians {
    /// Builds an angle from a value already expressed in radians.
    #[must_use]
    pub const fn new(radians: f64) -> Self {
        Self(radians)
    }

    /// Builds an angle from a value expressed in degrees.
    #[must_use]
    pub fn from_degrees(degrees: f64) -> Self {
        Self(degrees.to_radians())
    }

    /// The value in radians.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }

    /// The value in degrees. Display only — no computation in this crate
    /// should need it.
    #[must_use]
    pub fn to_degrees(self) -> f64 {
        self.0.to_degrees()
    }

    /// Sine and cosine in a single call, which is how this type is almost
    /// always consumed.
    #[must_use]
    pub fn sin_cos(self) -> (f64, f64) {
        self.0.sin_cos()
    }
}

impl Add for Radians {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Radians {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl Neg for Radians {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}
```

Déclarer le module dans `crates/swept-core/src/lib.rs`, après la documentation de crate :

```rust
pub mod units;
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core units && cargo clippy --all-targets -- -D warnings`
Expected: quatre tests unitaires et un doctest passent, aucun avertissement.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/units-radians
git add crates/swept-core/src/units.rs crates/swept-core/src/lib.rs
git commit -m "feat(core): Radians newtype for domain angles

The only typed unit: degree/radian conversion crosses the whole interface,
whereas lengths in metres have never caused trouble."
```

---

### Task 4: Points et rectangles orientés

Porte `ob()`, `box()` et `corners()` du prototype (`prototype/index.html:214-221`).

**Files:**
- Create: `crates/swept-core/src/geometry.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `units::Radians` (Task 3)
- Produces: `geometry::Point { x: f64, y: f64 }` ; `geometry::Obb` avec `Obb::new(center: Point, angle: Radians, half_width: f64, half_height: f64)`, `Obb::from_bounds(x0: f64, x1: f64, y0: f64, y1: f64) -> Obb`, `.corners() -> [Point; 4]`, et les champs publics `center`, `angle`, `half_width`, `half_height`

- [ ] **Step 1: Écrire les tests qui échouent**

Dans `crates/swept-core/src/geometry.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    fn assert_point_near(actual: Point, x: f64, y: f64) {
        assert!(
            (actual.x - x).abs() < EPS && (actual.y - y).abs() < EPS,
            "expected ({x}, {y}), got ({}, {})",
            actual.x,
            actual.y
        );
    }

    #[test]
    fn builds_an_axis_aligned_box_from_bounds() {
        let obb = Obb::from_bounds(-1.0, 3.0, 0.0, 2.0);
        assert_point_near(obb.center, 1.0, 1.0);
        assert!((obb.half_width - 2.0).abs() < EPS);
        assert!((obb.half_height - 1.0).abs() < EPS);
        assert!(obb.angle.get().abs() < EPS);
    }

    #[test]
    fn lists_corners_counterclockwise_from_the_near_left() {
        // Same order as the prototype: (-1,-1), (1,-1), (1,1), (-1,1) in local
        // coordinates.
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 4.0);
        let corners = obb.corners();
        assert_point_near(corners[0], 0.0, 0.0);
        assert_point_near(corners[1], 2.0, 0.0);
        assert_point_near(corners[2], 2.0, 4.0);
        assert_point_near(corners[3], 0.0, 4.0);
    }

    #[test]
    fn rotates_corners_about_the_centre() {
        // A 2x2 square centred on the origin, turned a quarter turn, maps each
        // corner onto the next one.
        let obb = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(90.0), 1.0, 1.0);
        let corners = obb.corners();
        assert_point_near(corners[0], 1.0, -1.0);
        assert_point_near(corners[1], 1.0, 1.0);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core geometry`
Expected: FAIL — `cannot find type Obb in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/geometry.rs` :

```rust
//! Plane geometry: points and oriented bounding boxes.
//!
//! Every obstacle in a scene is an oriented rectangle, which keeps collision
//! detection to two primitives: the distance from a point to a rectangle, and
//! the overlap of two rectangles.

use crate::units::Radians;

/// A point in the plane, in metres.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// Along the road.
    pub x: f64,
    /// Away from the road; `0` is the outer face of the wall.
    pub y: f64,
}

impl Point {
    /// Builds a point from its coordinates, in metres.
    #[must_use]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// An oriented bounding box: a rectangle free to rotate about its centre.
///
/// ```
/// use swept_core::geometry::Obb;
///
/// let wall = Obb::from_bounds(-3.0, 3.0, 0.0, 0.3);
/// assert_eq!(wall.corners().len(), 4);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Obb {
    /// Centre of the rectangle.
    pub center: Point,
    /// Rotation of the local axes about the centre.
    pub angle: Radians,
    /// Half-size along the local `x` axis.
    pub half_width: f64,
    /// Half-size along the local `y` axis.
    pub half_height: f64,
}

impl Obb {
    /// Builds a rectangle from its centre, rotation and half-sizes.
    #[must_use]
    pub const fn new(center: Point, angle: Radians, half_width: f64, half_height: f64) -> Self {
        Self {
            center,
            angle,
            half_width,
            half_height,
        }
    }

    /// Builds an axis-aligned rectangle from its bounds.
    ///
    /// Most of a scene is walls and kerbs, which are axis-aligned; this is the
    /// constructor they use.
    #[must_use]
    pub fn from_bounds(x0: f64, x1: f64, y0: f64, y1: f64) -> Self {
        Self::new(
            Point::new(f64::midpoint(x0, x1), f64::midpoint(y0, y1)),
            Radians::default(),
            (x1 - x0) / 2.0,
            (y1 - y0) / 2.0,
        )
    }

    /// The four corners, counter-clockwise from the local `(-1, -1)` corner.
    ///
    /// The order matters: it is the order the prototype used, and the golden
    /// vectors are recorded in it.
    #[must_use]
    pub fn corners(&self) -> [Point; 4] {
        let (sin, cos) = self.angle.sin_cos();
        let signs: [(f64, f64); 4] = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)];
        signs.map(|(sx, sy)| {
            let (dx, dy) = (sx * self.half_width, sy * self.half_height);
            Point::new(
                self.center.x + dx * cos - dy * sin,
                self.center.y + dx * sin + dy * cos,
            )
        })
    }
}
```

Déclarer le module dans `lib.rs` :

```rust
pub mod geometry;
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core geometry && cargo clippy --all-targets -- -D warnings`
Expected: trois tests unitaires et un doctest passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/geometry-obb
git add crates/swept-core/src/geometry.rs crates/swept-core/src/lib.rs
git commit -m "feat(core): points and oriented bounding boxes

Ports ob(), box() and corners() from the prototype (index.html:214-221).
Corner ordering is preserved: the golden vectors are recorded in it."
```

---

### Task 5: Distance d'un point à un rectangle

Porte `distOB()` (`prototype/index.html:222-227`). Le prototype renvoie `-1` quand le point est à l'intérieur ; ce plan remplace cette convention par un type explicite, en conservant exactement la même sémantique.

**Files:**
- Modify: `crates/swept-core/src/geometry.rs`

**Interfaces:**
- Consumes: `geometry::Obb` (Task 4)
- Produces: `geometry::PointDistance` (variantes `Inside` et `Outside(f64)`) et `Obb::distance_to(&self, point: Point) -> PointDistance`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le `mod tests` de `geometry.rs` :

```rust
    #[test]
    fn reports_a_point_inside_the_rectangle() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        assert_eq!(obb.distance_to(Point::new(1.0, 1.0)), PointDistance::Inside);
    }

    #[test]
    fn measures_perpendicular_distance_to_an_edge() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        match obb.distance_to(Point::new(3.5, 1.0)) {
            PointDistance::Outside(d) => assert!((d - 1.5).abs() < EPS),
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }

    #[test]
    fn measures_diagonal_distance_to_a_corner() {
        let obb = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        match obb.distance_to(Point::new(5.0, 6.0)) {
            PointDistance::Outside(d) => assert!((d - 5.0).abs() < EPS), // 3-4-5
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }

    #[test]
    fn accounts_for_rotation() {
        // A 2x1 rectangle turned a quarter turn is 1 wide and 2 tall.
        let obb = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(90.0), 1.0, 0.5);
        match obb.distance_to(Point::new(2.0, 0.0)) {
            PointDistance::Outside(d) => assert!((d - 1.5).abs() < EPS),
            PointDistance::Inside => panic!("point is outside the rectangle"),
        }
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core geometry`
Expected: FAIL — `cannot find type PointDistance in this scope`.

- [ ] **Step 3: Implémenter**

Ajouter dans `geometry.rs`, avant le module de test :

```rust
/// How far a point lies from a rectangle.
///
/// The prototype folded both cases into a single number, returning `-1` for a
/// point inside the rectangle. That sentinel is easy to forget to check, and
/// forgetting it turns a collision into a very small clearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PointDistance {
    /// The point lies within the rectangle.
    Inside,
    /// The point lies outside, at this distance from the nearest edge, in
    /// metres.
    Outside(f64),
}

impl Obb {
    /// Distance from `point` to this rectangle.
    ///
    /// The point is taken into the rectangle's local frame, where the distance
    /// reduces to the length of the componentwise overshoot beyond the
    /// half-sizes.
    ///
    /// ```
    /// use swept_core::geometry::{Obb, Point, PointDistance};
    ///
    /// let pillar = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
    /// assert_eq!(pillar.distance_to(Point::new(0.5, 0.5)), PointDistance::Inside);
    /// assert_eq!(pillar.distance_to(Point::new(3.0, 0.5)), PointDistance::Outside(2.0));
    /// ```
    #[must_use]
    pub fn distance_to(&self, point: Point) -> PointDistance {
        let (sin, cos) = self.angle.sin_cos();
        let (dx, dy) = (point.x - self.center.x, point.y - self.center.y);
        let local_x = dx * cos + dy * sin;
        let local_y = -dx * sin + dy * cos;

        let overshoot_x = (local_x.abs() - self.half_width).max(0.0);
        let overshoot_y = (local_y.abs() - self.half_height).max(0.0);

        if overshoot_x == 0.0 && overshoot_y == 0.0 {
            PointDistance::Inside
        } else {
            PointDistance::Outside(overshoot_x.hypot(overshoot_y))
        }
    }
}
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core geometry && cargo clippy --all-targets -- -D warnings`
Expected: sept tests unitaires et deux doctests passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/geometry-distance
git add crates/swept-core/src/geometry.rs
git commit -m "feat(core): distance from a point to an oriented rectangle

Ports distOB() (index.html:222-227). The prototype's -1 sentinel becomes
PointDistance::Inside, a case that can no longer be left unhandled —
overlooking it turned a collision into zero clearance."
```

---

### Task 6: Recouvrement de deux rectangles

Porte `overlapOBB()` (`prototype/index.html:251-261`), théorème des axes séparateurs. Le prototype applique une tolérance de 6 mm avant de déclarer un contact : deux rectangles qui se chevauchent de moins que cela sont considérés disjoints.

**Files:**
- Modify: `crates/swept-core/src/geometry.rs`

**Interfaces:**
- Consumes: `geometry::Obb` (Task 4)
- Produces: `Obb::overlaps(&self, other: &Obb) -> bool` et la constante publique `geometry::OVERLAP_TOLERANCE_M`

- [ ] **Step 1: Écrire les tests qui échouent**

Ajouter dans le `mod tests` :

```rust
    #[test]
    fn detects_plainly_overlapping_rectangles() {
        let a = Obb::from_bounds(0.0, 2.0, 0.0, 2.0);
        let b = Obb::from_bounds(1.0, 3.0, 1.0, 3.0);
        assert!(a.overlaps(&b));
        assert!(b.overlaps(&a));
    }

    #[test]
    fn separates_disjoint_rectangles() {
        let a = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
        let b = Obb::from_bounds(2.0, 3.0, 0.0, 1.0);
        assert!(!a.overlaps(&b));
    }

    #[test]
    fn tolerates_an_overlap_below_the_threshold() {
        // Carried over from the prototype: an overlap of less than 6 mm does
        // not count as contact.
        let a = Obb::from_bounds(0.0, 1.0, 0.0, 1.0);
        let barely = Obb::from_bounds(1.0 - 0.005, 2.0, 0.0, 1.0);
        assert!(!a.overlaps(&barely));

        let clearly = Obb::from_bounds(1.0 - 0.02, 2.0, 0.0, 1.0);
        assert!(a.overlaps(&clearly));
    }

    #[test]
    fn separates_rotated_rectangles_that_axis_aligned_bounds_would_not() {
        // Two squares turned a half-quarter turn. Each spans ±1.414 on both
        // axes, so their axis-aligned bounds overlap on [1.086, 1.414] — but
        // the shapes themselves are 3.54 apart, against a combined reach of
        // 2.83. Only a proper separating-axis test gets this right.
        let a = Obb::new(Point::new(0.0, 0.0), Radians::from_degrees(45.0), 1.0, 1.0);
        let b = Obb::new(Point::new(2.5, 2.5), Radians::from_degrees(45.0), 1.0, 1.0);
        assert!(!a.overlaps(&b));
    }
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core geometry`
Expected: FAIL — `no method named overlaps found for struct Obb`.

- [ ] **Step 3: Implémenter**

Ajouter dans `geometry.rs` :

```rust
/// Overlap below which two rectangles are still considered disjoint, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:258`), where it
/// absorbs the floating-point noise of gate leaves that come to rest exactly
/// against a pillar. No measurement backs the specific value; it should be
/// revalidated against real tolerances.
pub const OVERLAP_TOLERANCE_M: f64 = 0.006;

impl Obb {
    /// Whether two rectangles overlap, by the separating axis theorem.
    ///
    /// Two convex shapes are disjoint if and only if some axis exists on which
    /// their projections do not meet. For rectangles it is enough to test the
    /// four edge normals — two per rectangle.
    #[must_use]
    pub fn overlaps(&self, other: &Obb) -> bool {
        let (a_sin, a_cos) = self.angle.sin_cos();
        let (b_sin, b_cos) = other.angle.sin_cos();
        let axes = [
            (a_cos, a_sin),
            (-a_sin, a_cos),
            (b_cos, b_sin),
            (-b_sin, b_cos),
        ];

        let mine = self.corners();
        let theirs = other.corners();

        for (ux, uy) in axes {
            let project = |corners: &[Point; 4]| {
                corners.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                    let v = p.x * ux + p.y * uy;
                    (lo.min(v), hi.max(v))
                })
            };
            let (a_lo, a_hi) = project(&mine);
            let (b_lo, b_hi) = project(&theirs);

            if a_hi < b_lo + OVERLAP_TOLERANCE_M || b_hi < a_lo + OVERLAP_TOLERANCE_M {
                return false;
            }
        }
        true
    }
}
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core geometry && cargo clippy --all-targets -- -D warnings`
Expected: onze tests unitaires passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/geometry-overlap
git add crates/swept-core/src/geometry.rs
git commit -m "feat(core): rectangle overlap by separating axes

Ports overlapOBB() (index.html:251-261). The prototype's 6 mm tolerance
becomes OVERLAP_TOLERANCE_M, documented as arbitrary and due for
revalidation."
```

---

### Task 7: Poses et intégration à courbure constante

Porte `arc()`, `line()` et `move()` (`prototype/index.html:312-324` et `398-412`). C'est le cœur cinématique : modèle bicyclette, état réduit à la pose de l'essieu arrière.

**Files:**
- Create: `crates/swept-core/src/kinematics.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `units::Radians` (Task 3), `geometry::Point` (Task 4)
- Produces: `kinematics::Pose { x: f64, y: f64, heading: Radians }` avec `Pose::new(x, y, heading)` et `.position() -> Point` ; `kinematics::Direction` (`Forward` / `Reverse`) ; `Pose::advance(&self, curvature: f64, distance: f64) -> Pose` ; `kinematics::sample_arc(from: Pose, curvature: f64, distance: f64, step: f64) -> Vec<Pose>`

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/kinematics.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    const EPS: f64 = 1e-12;

    #[test]
    fn a_straight_segment_keeps_the_heading() {
        let start = Pose::new(1.0, 2.0, Radians::default());
        let end = start.advance(0.0, 3.0);
        assert!((end.x - 4.0).abs() < EPS);
        assert!((end.y - 2.0).abs() < EPS);
        assert!(end.heading.get().abs() < EPS);
    }

    #[test]
    fn a_straight_segment_backwards_moves_the_other_way() {
        let start = Pose::new(1.0, 2.0, Radians::default());
        let end = start.advance(0.0, -3.0);
        assert!((end.x + 2.0).abs() < EPS);
        assert!((end.y - 2.0).abs() < EPS);
    }

    #[test]
    fn a_quarter_circle_turns_ninety_degrees() {
        // Radius 5, heading east, turning left: the rear axle ends up 5 m
        // ahead and 5 m to the left, pointing north.
        let start = Pose::new(0.0, 0.0, Radians::default());
        let radius = 5.0;
        let end = start.advance(1.0 / radius, radius * FRAC_PI_2);
        assert!((end.x - 5.0).abs() < 1e-9);
        assert!((end.y - 5.0).abs() < 1e-9);
        assert!((end.heading.get() - FRAC_PI_2).abs() < 1e-9);
    }

    #[test]
    fn a_full_circle_returns_to_the_start() {
        let start = Pose::new(2.0, -1.0, Radians::from_degrees(30.0));
        let radius = 4.0;
        let end = start.advance(1.0 / radius, 2.0 * std::f64::consts::PI * radius);
        assert!((end.x - start.x).abs() < 1e-9);
        assert!((end.y - start.y).abs() < 1e-9);
    }

    #[test]
    fn sampling_lands_exactly_on_the_endpoint() {
        let start = Pose::new(0.0, 0.0, Radians::default());
        let poses = sample_arc(start, 1.0 / 5.0, 5.0 * FRAC_PI_2, 0.06);
        let last = *poses.last().expect("at least one sample");
        let direct = start.advance(1.0 / 5.0, 5.0 * FRAC_PI_2);
        assert!((last.x - direct.x).abs() < 1e-9);
        assert!((last.y - direct.y).abs() < 1e-9);
        assert!((last.heading.get() - direct.heading.get()).abs() < 1e-9);
    }

    #[test]
    fn sampling_respects_the_requested_step() {
        let start = Pose::new(0.0, 0.0, Radians::default());
        let poses = sample_arc(start, 0.0, 1.0, 0.25);
        assert_eq!(poses.len(), 4);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core kinematics`
Expected: FAIL — `cannot find type Pose in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/kinematics.rs` :

```rust
//! Vehicle motion under the bicycle model.
//!
//! The vehicle's state reduces to the pose of its rear axle: two coordinates
//! and a heading. A manoeuvre is a chain of constant-curvature segments, each
//! integrated in closed form:
//!
//! ```text
//! θ₁ = θ + κ·ds
//! x += R·(sin θ₁ − sin θ)      with R = 1/κ
//! y −= R·(cos θ₁ − cos θ)
//! ```
//!
//! A negative `ds` is reverse motion. A curvature of zero degenerates to a
//! straight segment, handled separately because `R` is then infinite.

use crate::geometry::Point;
use crate::units::Radians;

/// Which way the vehicle is moving along its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Moving forward.
    Forward,
    /// Backing up.
    Reverse,
}

/// The pose of the vehicle's rear axle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Pose {
    /// Along the road, in metres.
    pub x: f64,
    /// Away from the road, in metres.
    pub y: f64,
    /// Direction the vehicle points in.
    pub heading: Radians,
}

impl Pose {
    /// Builds a pose from its coordinates and heading.
    #[must_use]
    pub const fn new(x: f64, y: f64, heading: Radians) -> Self {
        Self { x, y, heading }
    }

    /// The position alone, dropping the heading.
    #[must_use]
    pub const fn position(&self) -> Point {
        Point::new(self.x, self.y)
    }

    /// Advances the pose along a constant-curvature segment.
    ///
    /// `curvature` is the inverse of the turning radius, in reciprocal metres;
    /// zero means a straight line. `distance` is the arc length travelled,
    /// negative when reversing.
    ///
    /// ```
    /// use swept_core::kinematics::Pose;
    /// use swept_core::units::Radians;
    ///
    /// let start = Pose::new(0.0, 0.0, Radians::default());
    /// let end = start.advance(0.0, 2.5);
    /// assert!((end.x - 2.5).abs() < 1e-12);
    /// ```
    #[must_use]
    pub fn advance(&self, curvature: f64, distance: f64) -> Self {
        if curvature == 0.0 {
            let (sin, cos) = self.heading.sin_cos();
            return Self::new(
                self.x + cos * distance,
                self.y + sin * distance,
                self.heading,
            );
        }

        let radius = 1.0 / curvature;
        let end_heading = self.heading + Radians::new(curvature * distance);
        let (sin_0, cos_0) = self.heading.sin_cos();
        let (sin_1, cos_1) = end_heading.sin_cos();

        Self::new(
            self.x + radius * (sin_1 - sin_0),
            self.y - radius * (cos_1 - cos_0),
            end_heading,
        )
    }
}

/// Longest sampling step allowed, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:321`), which caps
/// a straight segment at 400 samples. The cap exists to bound memory on long
/// approach runs, not for any geometric reason.
pub const MAX_SAMPLES_PER_SEGMENT: usize = 400;

/// Samples a constant-curvature segment into successive poses.
///
/// The returned poses exclude `from` and always end exactly on the segment's
/// endpoint, so that chaining segments introduces no drift. `step` is an upper
/// bound on the spacing, in metres.
///
/// # Panics
///
/// Panics if `step` is not strictly positive.
#[must_use]
pub fn sample_arc(from: Pose, curvature: f64, distance: f64, step: f64) -> Vec<Pose> {
    assert!(step > 0.0, "sampling step must be strictly positive");

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let count = ((distance.abs() / step).ceil() as usize)
        .max(1)
        .min(MAX_SAMPLES_PER_SEGMENT);

    (1..=count)
        .map(|i| {
            #[allow(clippy::cast_precision_loss)]
            let fraction = i as f64 / count as f64;
            from.advance(curvature, distance * fraction)
        })
        .collect()
}
```

Déclarer le module dans `lib.rs` :

```rust
pub mod kinematics;
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core kinematics && cargo clippy --all-targets -- -D warnings`
Expected: six tests unitaires et un doctest passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/kinematics
git add crates/swept-core/src/kinematics.rs crates/swept-core/src/lib.rs
git commit -m "feat(core): poses and constant-curvature integration

Ports arc(), line() and move() (index.html:312-324, 398-412). Sampling
lands exactly on the endpoint, so chaining segments introduces no drift."
```

---

### Task 8: Vecteurs de référence extraits du prototype

Les tâches 4 à 7 sont validées par des tests écrits à la main, qui prouvent la correction mathématique mais pas l'équivalence avec le prototype. Cette tâche ajoute cet oracle-là.

Le prototype est un fichier HTML monolithique dont le script touche au DOM dès son chargement : on ne peut pas l'importer tel quel dans Node. Le harnais **recopie** donc les fonctions géométriques pures, et un test vérifie que le prototype n'a pas bougé depuis la copie — le prototype étant gelé, cette copie ne peut pas diverger silencieusement.

**Files:**
- Create: `tools/extract-golden/package.json`, `tools/extract-golden/proto.js`, `tools/extract-golden/extract.js`
- Create: `crates/swept-core/tests/golden.rs`
- Create: `crates/swept-core/tests/fixtures/geometry.json`, `crates/swept-core/tests/fixtures/kinematics.json` (générés)
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: `Obb::distance_to` (Task 5), `Obb::overlaps` (Task 6), `Pose::advance` (Task 7)
- Produces: deux fichiers de fixtures et le test d'intégration qui les consomme. Aucune API de crate.

- [ ] **Step 1: Confirmer l'empreinte du prototype**

Run: `shasum -a 256 prototype/index.html`
Expected: `267e24e50dcfaad166e4b68dfec895cf3cf2c534608e824b436b0e9caf5e41ab`

Si l'empreinte diffère, le prototype a été modifié depuis la rédaction de ce plan. Ne pas se contenter de mettre la nouvelle valeur dans l'extracteur : vérifier d'abord ce qui a changé, puisque le prototype est censé être gelé et sert d'oracle.

- [ ] **Step 2: Copier les fonctions du prototype**

Fichier `tools/extract-golden/proto.js` :

```js
// Copie littérale des fonctions géométriques pures de prototype/index.html.
// Source : lignes 214-227, 251-261 et 312-324 du prototype, qui est gelé.
// La fidélité de cette copie est vérifiée par extract.js, qui refuse de
// tourner si l'empreinte du prototype a changé.

export function ob(cx, cy, ang, hw, hh) {
  return { cx, cy, ang, hw, hh, c: Math.cos(ang), s: Math.sin(ang) };
}

export function box(x0, x1, y0, y1) {
  return ob((x0 + x1) / 2, (y0 + y1) / 2, 0, (x1 - x0) / 2, (y1 - y0) / 2);
}

export function corners(o) {
  const p = [];
  for (const [a, b] of [[-1, -1], [1, -1], [1, 1], [-1, 1]])
    p.push([o.cx + a * o.hw * o.c - b * o.hh * o.s, o.cy + a * o.hw * o.s + b * o.hh * o.c]);
  return p;
}

export function distOB(px, py, o) {
  const dx = px - o.cx, dy = py - o.cy;
  const lx = dx * o.c + dy * o.s, ly = -dx * o.s + dy * o.c;
  const ax = Math.max(Math.abs(lx) - o.hw, 0), ay = Math.max(Math.abs(ly) - o.hh, 0);
  return (ax === 0 && ay === 0) ? -1 : Math.hypot(ax, ay);
}

export function overlapOBB(a, b) {
  const axes = [[a.c, a.s], [-a.s, a.c], [b.c, b.s], [-b.s, b.c]];
  const ca = corners(a), cb = corners(b);
  for (const [ux, uy] of axes) {
    let a0 = Infinity, a1 = -Infinity, b0 = Infinity, b1 = -Infinity;
    for (const p of ca) { const v = p[0] * ux + p[1] * uy; if (v < a0) a0 = v; if (v > a1) a1 = v; }
    for (const p of cb) { const v = p[0] * ux + p[1] * uy; if (v < b0) b0 = v; if (v > b1) b1 = v; }
    if (a1 < b0 + 0.006 || b1 < a0 + 0.006) return false;
  }
  return true;
}

export function move(p, kap, dist) {
  const th1 = p.th + kap * dist;
  let x = p.x, y = p.y;
  if (kap === 0) { x += Math.cos(p.th) * dist; y += Math.sin(p.th) * dist; }
  else { const R = 1 / kap; x += R * (Math.sin(th1) - Math.sin(p.th)); y -= R * (Math.cos(th1) - Math.cos(p.th)); }
  return { x, y, th: th1 };
}
```

- [ ] **Step 3: Écrire l'extracteur**

Fichier `tools/extract-golden/extract.js` :

```js
// Produit les vecteurs de référence consommés par crates/swept-core/tests/golden.rs.
// Usage : node tools/extract-golden/extract.js

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { box, ob, distOB, overlapOBB, move } from "./proto.js";

// Empreinte relevée à l'étape 1 de la Task 8. Le prototype est gelé : s'il
// change, la copie de proto.js n'est plus fidèle et l'oracle est caduc.
const EXPECTED_SHA256 = "267e24e50dcfaad166e4b68dfec895cf3cf2c534608e824b436b0e9caf5e41ab";

const source = readFileSync("prototype/index.html");
const actual = createHash("sha256").update(source).digest("hex");
if (actual !== EXPECTED_SHA256) {
  console.error(`Le prototype a changé.\n  attendu : ${EXPECTED_SHA256}\n  obtenu  : ${actual}`);
  console.error("Le prototype est gelé. Si le changement est voulu, revoir proto.js puis mettre l'empreinte à jour.");
  process.exit(1);
}

// Générateur déterministe : les fixtures doivent être reproductibles.
let seed = 20260808;
const rand = () => {
  seed = (seed * 1103515245 + 12345) & 0x7fffffff;
  return seed / 0x7fffffff;
};
const between = (lo, hi) => lo + rand() * (hi - lo);

const geometry = [];
for (let i = 0; i < 400; i++) {
  const a = ob(between(-5, 5), between(-5, 5), between(-Math.PI, Math.PI), between(0.05, 3), between(0.05, 3));
  const b = ob(between(-5, 5), between(-5, 5), between(-Math.PI, Math.PI), between(0.05, 3), between(0.05, 3));
  const px = between(-8, 8), py = between(-8, 8);
  geometry.push({
    a: { cx: a.cx, cy: a.cy, ang: a.ang, hw: a.hw, hh: a.hh },
    b: { cx: b.cx, cy: b.cy, ang: b.ang, hw: b.hw, hh: b.hh },
    point: { x: px, y: py },
    distance_to_a: distOB(px, py, a),
    overlaps: overlapOBB(a, b),
  });
}

const kinematics = [];
for (let i = 0; i < 400; i++) {
  const start = { x: between(-5, 5), y: between(-5, 5), th: between(-Math.PI, Math.PI) };
  // Une courbure nulle une fois sur cinq, pour couvrir le segment droit.
  const curvature = i % 5 === 0 ? 0 : 1 / between(-12, 12);
  const distance = between(-8, 8);
  kinematics.push({ start, curvature, distance, end: move(start, curvature, distance) });
}

mkdirSync("crates/swept-core/tests/fixtures", { recursive: true });
writeFileSync("crates/swept-core/tests/fixtures/geometry.json", `${JSON.stringify(geometry, null, 1)}\n`);
writeFileSync("crates/swept-core/tests/fixtures/kinematics.json", `${JSON.stringify(kinematics, null, 1)}\n`);
console.log(`${geometry.length} cas de géométrie et ${kinematics.length} cas de cinématique écrits.`);
```

Fichier `tools/extract-golden/package.json` :

```json
{
  "name": "extract-golden",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "description": "Harnais jetable : extrait du prototype gelé les vecteurs de référence du noyau Rust.",
  "scripts": {
    "extract": "node extract.js"
  }
}
```

- [ ] **Step 4: Inscrire l'empreinte et générer les fixtures**

Remplacer `REMPLACER_PAR_L_EMPREINTE_RELEVEE` par l'empreinte relevée à l'étape 1.

Run: `node tools/extract-golden/extract.js`
Expected: `400 cas de géométrie et 400 cas de cinématique écrits.`

Vérifier ensuite que le garde-fou fonctionne : modifier une empreinte d'un caractère, relancer, constater la sortie en erreur, puis rétablir.

- [ ] **Step 5: Écrire le test qui échoue**

`swept-core` n'ayant aucune dépendance, le test lit le JSON avec un analyseur minimal écrit sur place. Les fixtures ont une forme fixe et connue : un tableau d'objets plats de nombres et de booléens.

Fichier `crates/swept-core/tests/golden.rs` :

```rust
//! Checks the Rust core against vectors recorded from the frozen prototype.
//!
//! Regenerate with `node tools/extract-golden/extract.js`.

use swept_core::geometry::{Obb, Point, PointDistance};
use swept_core::kinematics::Pose;
use swept_core::units::Radians;

/// Largest acceptable difference from the prototype, in metres or radians.
const TOLERANCE: f64 = 1e-9;

/// Pulls every `"name": value` pair out of a flat JSON fixture, in order.
///
/// The fixtures are generated by our own tool and have a known, flat shape, so
/// a full JSON parser would be a dependency this crate deliberately refuses.
fn scalars(json: &str, name: &str) -> Vec<f64> {
    let needle = format!("\"{name}\":");
    json.match_indices(&needle)
        .map(|(at, _)| {
            let rest = &json[at + needle.len()..];
            let end = rest
                .find([',', '}', ']', '\n'])
                .unwrap_or(rest.len());
            rest[..end]
                .trim()
                .parse::<f64>()
                .unwrap_or_else(|_| panic!("field {name} is not a number: {}", &rest[..end]))
        })
        .collect()
}

/// Same, for boolean fields.
fn flags(json: &str, name: &str) -> Vec<bool> {
    let needle = format!("\"{name}\":");
    json.match_indices(&needle)
        .map(|(at, _)| json[at + needle.len()..].trim_start().starts_with("true"))
        .collect()
}

#[test]
fn matches_the_prototype_on_point_to_rectangle_distance() {
    let json = include_str!("fixtures/geometry.json");
    let cx = scalars(json, "cx");
    let cy = scalars(json, "cy");
    let ang = scalars(json, "ang");
    let hw = scalars(json, "hw");
    let hh = scalars(json, "hh");
    let px = scalars(json, "x");
    let py = scalars(json, "y");
    let expected = scalars(json, "distance_to_a");

    assert!(!expected.is_empty(), "fixtures are missing; run the extractor");

    for (case, want) in expected.iter().enumerate() {
        // Each case records rectangle a then rectangle b, so a is at index 2i.
        let i = case * 2;
        let obb = Obb::new(
            Point::new(cx[i], cy[i]),
            Radians::new(ang[i]),
            hw[i],
            hh[i],
        );
        let got = match obb.distance_to(Point::new(px[case], py[case])) {
            PointDistance::Inside => -1.0,
            PointDistance::Outside(d) => d,
        };
        assert!(
            (got - want).abs() < TOLERANCE,
            "case {case}: prototype says {want}, core says {got}"
        );
    }
}

#[test]
fn matches_the_prototype_on_rectangle_overlap() {
    let json = include_str!("fixtures/geometry.json");
    let cx = scalars(json, "cx");
    let cy = scalars(json, "cy");
    let ang = scalars(json, "ang");
    let hw = scalars(json, "hw");
    let hh = scalars(json, "hh");
    let expected = flags(json, "overlaps");

    assert!(!expected.is_empty(), "fixtures are missing; run the extractor");

    for (case, want) in expected.iter().enumerate() {
        let (i, j) = (case * 2, case * 2 + 1);
        let a = Obb::new(Point::new(cx[i], cy[i]), Radians::new(ang[i]), hw[i], hh[i]);
        let b = Obb::new(Point::new(cx[j], cy[j]), Radians::new(ang[j]), hw[j], hh[j]);
        assert_eq!(a.overlaps(&b), *want, "case {case}");
    }
}

#[test]
fn matches_the_prototype_on_constant_curvature_integration() {
    let json = include_str!("fixtures/kinematics.json");
    let xs = scalars(json, "x");
    let ys = scalars(json, "y");
    let ths = scalars(json, "th");
    let curvature = scalars(json, "curvature");
    let distance = scalars(json, "distance");

    assert!(!curvature.is_empty(), "fixtures are missing; run the extractor");

    for case in 0..curvature.len() {
        // Each case records start then end, so start is at index 2i.
        let (s, e) = (case * 2, case * 2 + 1);
        let start = Pose::new(xs[s], ys[s], Radians::new(ths[s]));
        let got = start.advance(curvature[case], distance[case]);

        assert!((got.x - xs[e]).abs() < TOLERANCE, "case {case}: x");
        assert!((got.y - ys[e]).abs() < TOLERANCE, "case {case}: y");
        assert!(
            (got.heading.get() - ths[e]).abs() < TOLERANCE,
            "case {case}: heading"
        );
    }
}
```

- [ ] **Step 6: Vérifier**

Run: `cargo test -p swept-core --test golden`
Expected: trois tests passent, chacun sur 400 cas. Si l'un échoue, c'est un écart réel entre le portage et le prototype — le message nomme le cas et les deux valeurs.

- [ ] **Step 7: Ajouter la vérification à la CI**

Dans `.github/workflows/ci.yml`, ajouter un job qui garantit que les fixtures commitées correspondent bien au prototype :

```yaml
  fixtures:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - uses: actions/setup-node@v4
        with:
          node-version: 24
      - name: Regenerate golden vectors
        run: node tools/extract-golden/extract.js
      - name: Fail if they differ from what is committed
        run: git diff --exit-code crates/swept-core/tests/fixtures/
```

- [ ] **Step 8: Commiter**

```bash
git checkout -b test/golden-vectors
git add tools/extract-golden/ crates/swept-core/tests/ .github/workflows/ci.yml
git commit -m "test(core): golden vectors extracted from the frozen prototype

800 cases compared at 1e-9: point-to-rectangle distance, overlap and
constant-curvature integration. The harness refuses to run if the
prototype's hash has changed, and CI checks that the committed fixtures
are the ones it produces."
```

---

### Task 9: Le véhicule

Porte la construction du véhicule, les règles de `badInputs()` qui le concernent (`prototype/index.html:194-211`) et `samplePoints()` (`276-281`).

**Files:**
- Create: `crates/swept-core/src/vehicle.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `geometry::Point` (Task 4)
- Produces: `vehicle::Vehicle` (champs `wheelbase`, `front_overhang`, `rear_overhang`, `width`, `mirror_width`, `min_turning_radius`, tous `f64` en mètres) ; `Vehicle::new(wheelbase, length, front_overhang, width, mirror_width, min_turning_radius) -> Result<Vehicle, VehicleError>` ; `vehicle::VehicleError` ; `Vehicle::envelope(&self) -> Vec<Point>` en coordonnées locales

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/vehicle.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    /// Lexus LBX, the prototype's default vehicle.
    fn lbx() -> Vehicle {
        Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).expect("valid vehicle")
    }

    #[test]
    fn derives_the_rear_overhang() {
        // 4.190 − 2.580 − 0.850 = 0.760
        assert!((lbx().rear_overhang - 0.760).abs() < 1e-9);
    }

    #[test]
    fn rejects_a_front_overhang_that_leaves_no_rear() {
        let err = Vehicle::new(2.580, 4.190, 1.700, 1.825, 2.029, 5.2).unwrap_err();
        assert_eq!(err, VehicleError::FrontOverhangTooLarge);
    }

    #[test]
    fn rejects_mirrors_narrower_than_the_body() {
        let err = Vehicle::new(2.580, 4.190, 0.850, 1.825, 1.700, 5.2).unwrap_err();
        assert_eq!(err, VehicleError::MirrorsNarrowerThanBody);
    }

    #[test]
    fn rejects_non_positive_dimensions() {
        assert_eq!(
            Vehicle::new(0.0, 4.190, 0.850, 1.825, 2.029, 5.2).unwrap_err(),
            VehicleError::NonPositive("wheelbase")
        );
        assert_eq!(
            Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, -1.0).unwrap_err(),
            VehicleError::NonPositive("min_turning_radius")
        );
    }

    #[test]
    fn samples_the_envelope_including_the_mirrors() {
        let v = lbx();
        let envelope = v.envelope();

        // Five stations on each side, plus front and rear centres, plus the
        // two mirrors: fourteen points.
        assert_eq!(envelope.len(), 14);

        // The widest points are the mirrors, level with the front axle.
        let widest = envelope
            .iter()
            .map(|p| p.y.abs())
            .fold(0.0_f64, f64::max);
        assert!((widest - v.mirror_width / 2.0).abs() < EPS);

        // The envelope spans from the rear bumper to the front bumper.
        let rearmost = envelope.iter().map(|p| p.x).fold(f64::MAX, f64::min);
        let frontmost = envelope.iter().map(|p| p.x).fold(f64::MIN, f64::max);
        assert!((rearmost + v.rear_overhang).abs() < EPS);
        assert!((frontmost - (v.wheelbase + v.front_overhang)).abs() < EPS);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core vehicle`
Expected: FAIL — `cannot find type Vehicle in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/vehicle.rs` :

```rust
//! The vehicle: its dimensions, and the outline used for collision checks.
//!
//! Coordinates are local to the vehicle, with the origin on the rear axle and
//! `x` pointing forward. The rear bumper therefore sits at `-rear_overhang`
//! and the front bumper at `wheelbase + front_overhang`.

use crate::geometry::Point;

/// Why a set of vehicle dimensions was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleError {
    /// A dimension that must be strictly positive was not. Carries the field
    /// name so the caller can point at it; the caller owns the wording shown
    /// to a user.
    NonPositive(&'static str),
    /// The front overhang leaves no room for a rear overhang, meaning the
    /// wheelbase and front overhang already exceed the total length.
    FrontOverhangTooLarge,
    /// Mirrors cannot be narrower than the body they fold against.
    MirrorsNarrowerThanBody,
}

/// Number of stations sampled along the body, from rear bumper to front.
///
/// ARBITRARY — carried over from the prototype (`index.html:278`), which walks
/// five stations. Denser sampling costs time in the inner loop of every
/// search; five was never justified by measurement.
pub const BODY_STATIONS: usize = 5;

/// A vehicle, validated at construction.
///
/// ```
/// use swept_core::vehicle::Vehicle;
///
/// // Lexus LBX
/// let v = Vehicle::new(2.580, 4.190, 0.850, 1.825, 2.029, 5.2).unwrap();
/// assert!((v.rear_overhang - 0.760).abs() < 1e-9);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vehicle {
    /// Distance between axles, in metres.
    pub wheelbase: f64,
    /// From the front axle to the front bumper, in metres.
    pub front_overhang: f64,
    /// From the rear axle to the rear bumper, in metres. Derived.
    pub rear_overhang: f64,
    /// Body width, mirrors excluded, in metres.
    pub width: f64,
    /// Width over the mirrors, in metres. This is almost always the critical
    /// dimension.
    pub mirror_width: f64,
    /// Tightest turning radius the vehicle can hold, in metres.
    pub min_turning_radius: f64,
}

impl Vehicle {
    /// Builds a vehicle, deriving the rear overhang from the total length.
    ///
    /// # Errors
    ///
    /// Returns [`VehicleError`] when a dimension is not strictly positive, when
    /// the front overhang leaves no rear overhang, or when the mirrors are
    /// narrower than the body.
    pub fn new(
        wheelbase: f64,
        length: f64,
        front_overhang: f64,
        width: f64,
        mirror_width: f64,
        min_turning_radius: f64,
    ) -> Result<Self, VehicleError> {
        for (value, name) in [
            (wheelbase, "wheelbase"),
            (length, "length"),
            (front_overhang, "front_overhang"),
            (width, "width"),
            (mirror_width, "mirror_width"),
            (min_turning_radius, "min_turning_radius"),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(VehicleError::NonPositive(name));
            }
        }

        let rear_overhang = length - wheelbase - front_overhang;
        if rear_overhang <= 0.0 {
            return Err(VehicleError::FrontOverhangTooLarge);
        }
        if mirror_width < width {
            return Err(VehicleError::MirrorsNarrowerThanBody);
        }

        Ok(Self {
            wheelbase,
            front_overhang,
            rear_overhang,
            width,
            mirror_width,
            min_turning_radius,
        })
    }

    /// The points sampled along the vehicle outline, in local coordinates.
    ///
    /// Five stations along each flank, the two bumper centres, and the two
    /// mirrors level with the front axle. The mirrors matter more than the
    /// rest: they are almost always the first thing to touch a pillar.
    #[must_use]
    pub fn envelope(&self) -> Vec<Point> {
        let half_body = self.width / 2.0;
        let half_mirrors = self.mirror_width / 2.0;
        let rear = -self.rear_overhang;
        let front = self.wheelbase + self.front_overhang;

        let mut points = Vec::with_capacity(2 * BODY_STATIONS + 4);
        for i in 0..BODY_STATIONS {
            #[allow(clippy::cast_precision_loss)]
            let fraction = i as f64 / (BODY_STATIONS - 1) as f64;
            let x = rear + (front - rear) * fraction;
            points.push(Point::new(x, half_body));
            points.push(Point::new(x, -half_body));
        }
        points.push(Point::new(rear, 0.0));
        points.push(Point::new(front, 0.0));
        points.push(Point::new(self.wheelbase, half_mirrors));
        points.push(Point::new(self.wheelbase, -half_mirrors));
        points
    }
}
```

Déclarer le module dans `lib.rs` :

```rust
pub mod vehicle;
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core vehicle && cargo clippy --all-targets -- -D warnings`
Expected: cinq tests unitaires et un doctest passent.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/vehicle
git add crates/swept-core/src/vehicle.rs crates/swept-core/src/lib.rs
git commit -m "feat(core): validated vehicle and sampled envelope

Ports samplePoints() and the vehicle-related rules of badInputs()
(index.html:194-211, 276-281). Errors name the offending field; the
wording shown to a user stays in the interface layer."
```

---

### Task 10: La scène et ses obstacles

Porte `obstacles()` (`prototype/index.html:238-249`). Le prototype suppose la scène symétrique autour de `x = 0` ; c'est le défaut 4 du `CLAUDE.md`, corrigé ici en positionnant les deux montants indépendamment.

**Files:**
- Create: `crates/swept-core/src/scene/mod.rs`, `crates/swept-core/src/scene/obstacles.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `geometry::Obb` (Task 4), `units::Radians` (Task 3)
- Produces: `scene::Scene`, `scene::Post { inner_edge_x: f64, width: f64, depth: f64 }`, `scene::GateKind` (`Sliding` / `Swinging { leaf_length, leaf_thickness, hinge_offset, hinge_depth_ratio, open_angle }`) ; `Scene::opening_width(&self) -> f64` ; `Scene::obstacles(&self) -> Vec<Obb>`

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/scene/mod.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f64 = 1e-12;

    /// The prototype's default scene: a 2.40 m opening between two 0.55 m
    /// pillars, symmetric about the origin.
    fn symmetric() -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -1.20, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: 1.20, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Sliding,
        }
    }

    #[test]
    fn measures_the_opening_between_the_posts() {
        assert!((symmetric().opening_width() - 2.40).abs() < EPS);
    }

    #[test]
    fn measures_an_off_centre_opening() {
        let mut scene = symmetric();
        scene.left_post.inner_edge_x = -0.80;
        assert!((scene.opening_width() - 2.00).abs() < EPS);
    }

    #[test]
    fn builds_the_expected_obstacles_for_a_sliding_gate() {
        // Two wall stretches, two pillars, the far kerb, and the pavement
        // split either side of the dropped kerb: seven rectangles.
        assert_eq!(symmetric().obstacles().len(), 7);
    }

    #[test]
    fn omits_the_pavement_when_there_is_none() {
        let mut scene = symmetric();
        scene.pavement_width = 0.0;
        assert_eq!(scene.obstacles().len(), 5);
    }

    #[test]
    fn places_the_pillars_against_the_opening() {
        let scene = symmetric();
        let obstacles = scene.obstacles();
        // The right pillar spans from the opening edge outwards by its width.
        let right = obstacles
            .iter()
            .find(|o| (o.center.x - (1.20 + 0.55 / 2.0)).abs() < EPS)
            .expect("right pillar");
        assert!((right.half_width - 0.55 / 2.0).abs() < EPS);
        assert!((right.half_height - 0.55 / 2.0).abs() < EPS);
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core scene`
Expected: FAIL — `cannot find type Scene in this scope`.

- [ ] **Step 3: Implémenter le modèle**

En tête de `crates/swept-core/src/scene/mod.rs` :

```rust
//! The scene the vehicle has to get through.
//!
//! A road, a pavement broken by a dropped kerb, a wall pierced by an opening
//! between two posts, and a free yard beyond. Everything is expressed in the
//! frame described at the crate root.
//!
//! Unlike the prototype, the two posts are placed independently: nothing here
//! assumes the opening is centred on `x = 0`.

pub mod obstacles;

use crate::geometry::Obb;
use crate::units::Radians;

/// One side of the opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Post {
    /// Where the post's inner face sits, along `x`. Negative on the left of
    /// the opening, positive on the right.
    pub inner_edge_x: f64,
    /// How wide the post is, along `x`, in metres.
    pub width: f64,
    /// How deep the post is, along `y`, in metres.
    pub depth: f64,
}

/// What closes the opening.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GateKind {
    /// A sliding gate: it retracts alongside the wall and obstructs nothing.
    /// The usable corridor is then the depth of the posts alone.
    Sliding,
    /// A pair of swinging leaves, which stand in the opening once open.
    Swinging {
        /// Length of one leaf, in metres.
        leaf_length: f64,
        /// Thickness of a leaf, in metres.
        leaf_thickness: f64,
        /// Gap between the hinge axis and the post's inner face, in metres.
        /// Every centimetre here costs two centimetres of clear opening.
        hinge_offset: f64,
        /// Where the hinge sits through the post's depth, from `0.0` at the
        /// road face to `1.0` at the yard face.
        hinge_depth_ratio: f64,
        /// How far the leaves are opened.
        open_angle: Radians,
    },
}

/// A complete scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scene {
    /// The post on the negative `x` side.
    pub left_post: Post,
    /// The post on the positive `x` side.
    pub right_post: Post,
    /// Thickness of the wall running away from the posts, in metres.
    pub wall_thickness: f64,
    /// Width of the pavement between road and wall, in metres. Zero means no
    /// pavement.
    pub pavement_width: f64,
    /// Width of the dropped kerb across the pavement, in metres.
    pub dropped_kerb_width: f64,
    /// Width of the carriageway available to manoeuvre in, in metres.
    pub road_width: f64,
    /// What closes the opening.
    pub gate: GateKind,
}

impl Scene {
    /// Clear width between the two posts, in metres.
    ///
    /// ```
    /// use swept_core::scene::{GateKind, Post, Scene};
    ///
    /// let scene = Scene {
    ///     left_post: Post { inner_edge_x: -1.2, width: 0.55, depth: 0.55 },
    ///     right_post: Post { inner_edge_x: 1.2, width: 0.55, depth: 0.55 },
    ///     wall_thickness: 0.3,
    ///     pavement_width: 1.2,
    ///     dropped_kerb_width: 3.2,
    ///     road_width: 4.5,
    ///     gate: GateKind::Sliding,
    /// };
    /// assert!((scene.opening_width() - 2.4).abs() < 1e-12);
    /// ```
    #[must_use]
    pub fn opening_width(&self) -> f64 {
        self.right_post.inner_edge_x - self.left_post.inner_edge_x
    }

    /// Every obstacle in the scene, as oriented rectangles.
    #[must_use]
    pub fn obstacles(&self) -> Vec<Obb> {
        obstacles::build(self)
    }
}
```

- [ ] **Step 4: Implémenter la génération des obstacles**

Fichier `crates/swept-core/src/scene/obstacles.rs` :

```rust
//! Turns a scene into the list of rectangles a vehicle can hit.

use super::Scene;
use crate::geometry::Obb;

/// How far the scene extends either side of the opening, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:239`). Walls have
/// to end somewhere; 18 m is far enough that no manoeuvre reaches the edge.
pub const SCENE_HALF_EXTENT_M: f64 = 18.0;

/// Thickness given to the wall across the road, in metres.
///
/// It only has to be thicker than any vehicle is long, so that no search can
/// tunnel through it. ARBITRARY in magnitude, deliberate in intent.
pub const FAR_SIDE_THICKNESS_M: f64 = 1000.0;

/// Below this, a pavement is treated as absent, in metres.
///
/// ARBITRARY — carried over from the prototype (`index.html:243`), where it
/// guards against a zero-height rectangle.
pub const PAVEMENT_EPSILON_M: f64 = 0.001;

/// Builds the obstacle list for a scene.
pub(super) fn build(scene: &Scene) -> Vec<Obb> {
    let left = scene.left_post.inner_edge_x;
    let right = scene.right_post.inner_edge_x;
    let left_outer = left - scene.left_post.width;
    let right_outer = right + scene.right_post.width;

    let mut obstacles = vec![
        // The wall either side of the posts.
        Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            left_outer,
            0.0,
            scene.wall_thickness,
        ),
        Obb::from_bounds(
            right_outer,
            SCENE_HALF_EXTENT_M,
            0.0,
            scene.wall_thickness,
        ),
        // The posts themselves.
        Obb::from_bounds(left_outer, left, 0.0, scene.left_post.depth),
        Obb::from_bounds(right, right_outer, 0.0, scene.right_post.depth),
        // Whatever stands across the road.
        Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            SCENE_HALF_EXTENT_M,
            -(scene.pavement_width + scene.road_width) - FAR_SIDE_THICKNESS_M,
            -(scene.pavement_width + scene.road_width),
        ),
    ];

    // The pavement, split either side of the dropped kerb.
    if scene.pavement_width > PAVEMENT_EPSILON_M {
        let half_kerb = scene.dropped_kerb_width / 2.0;
        let centre = f64::midpoint(left, right);
        obstacles.push(Obb::from_bounds(
            -SCENE_HALF_EXTENT_M,
            centre - half_kerb,
            -scene.pavement_width,
            0.0,
        ));
        obstacles.push(Obb::from_bounds(
            centre + half_kerb,
            SCENE_HALF_EXTENT_M,
            -scene.pavement_width,
            0.0,
        ));
    }

    obstacles
}
```

Cette tâche ne traite que le portail coulissant, qui n'obstrue rien. Les vantaux battants sont ajoutés à cette liste par la Task 11, qui détient la géométrie du vantail : c'est pourquoi `GateKind` n'est pas encore importé ici.

- [ ] **Step 5: Vérifier que les tests passent**

Run: `cargo test -p swept-core scene && cargo clippy --all-targets -- -D warnings`
Expected: cinq tests unitaires et un doctest passent.

- [ ] **Step 6: Commiter**

```bash
git checkout -b feat/scene-obstacles
git add crates/swept-core/src/scene/ crates/swept-core/src/lib.rs
git commit -m "feat(core): scene with independent posts and its obstacles

Ports obstacles() (index.html:238-249), dropping the assumption of
symmetry about x = 0 (defect 4 in CLAUDE.md): each post carries its own
inner-face abscissa."
```

---

### Task 11: Le portail battant et son angle maximal

Porte `leafOB()`, `leafHitsPillar()` et `maxAngle()` (`prototype/index.html:229-236`, `262-274`). Cette tâche valide le **quatrième résultat de référence** du `CLAUDE.md`.

**Files:**
- Create: `crates/swept-core/src/scene/gate.rs`
- Modify: `crates/swept-core/src/scene/mod.rs`, `crates/swept-core/src/scene/obstacles.rs`

**Interfaces:**
- Consumes: `scene::Scene`, `scene::GateKind` (Task 10), `Obb::overlaps` (Task 6)
- Produces: `scene::gate::leaves(scene: &Scene) -> Vec<Obb>` (visible dans la crate) ; `Scene::max_open_angle(&self) -> Radians`

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/scene/gate.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// The reference scene from CLAUDE.md: 0.55 m pillars, hinge halfway
    /// through their depth, 5 cm of offset.
    fn swinging(hinge_offset: f64, hinge_depth_ratio: f64, open_degrees: f64) -> Scene {
        Scene {
            left_post: Post { inner_edge_x: -1.20, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: 1.20, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateKind::Swinging {
                leaf_length: 1.15,
                leaf_thickness: 0.10,
                hinge_offset,
                hinge_depth_ratio,
                open_angle: Radians::from_degrees(open_degrees),
            },
        }
    }

    #[test]
    fn a_swinging_gate_contributes_two_leaves() {
        assert_eq!(leaves(&swinging(0.05, 0.5, 90.0)).len(), 2);
    }

    #[test]
    fn a_sliding_gate_contributes_none() {
        let mut scene = swinging(0.05, 0.5, 90.0);
        scene.gate = GateKind::Sliding;
        assert!(leaves(&scene).is_empty());
    }

    #[test]
    fn leaves_stand_clear_of_the_pillars_at_ninety_degrees() {
        assert!(!hits_a_post(&swinging(0.05, 0.5, 90.0)));
    }

    #[test]
    fn leaves_foul_the_pillars_when_opened_too_far() {
        assert!(hits_a_post(&swinging(0.05, 0.5, 150.0)));
    }

    /// Fourth reference result from CLAUDE.md: with the hinge halfway through
    /// a 0.55 m pillar and 5 cm of offset, the leaf clears up to about 91°;
    /// moving the hinge to the yard face buys about 118°.
    #[test]
    fn reproduces_the_reference_opening_angles() {
        let halfway = swinging(0.05, 0.5, 90.0).max_open_angle().to_degrees();
        assert!(
            (halfway - 91.0).abs() <= 1.0,
            "hinge halfway: expected about 91 degrees, got {halfway}"
        );

        let yard_face = swinging(0.05, 1.0, 90.0).max_open_angle().to_degrees();
        assert!(
            (yard_face - 118.0).abs() <= 2.0,
            "hinge on the yard face: expected about 118 degrees, got {yard_face}"
        );
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core scene::gate`
Expected: FAIL — `cannot find function leaves in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/scene/gate.rs` :

```rust
//! Swinging gate leaves, and how far they can open.
//!
//! A leaf pivots about a hinge set a little inside the post's face. That
//! offset is what lets the leaf swing past 90° without fouling the post — but
//! every centimetre of offset costs two centimetres of clear opening, one on
//! each side, so past roughly 120° the trade stops paying.

use super::{GateKind, Post, Scene};
use crate::geometry::{Obb, Point};
use crate::units::Radians;

/// Narrowest opening angle considered, in degrees.
///
/// Below this a gate is barely open and the question does not arise.
/// ARBITRARY — carried over from the prototype (`index.html:269`).
const MIN_OPEN_DEGREES: f64 = 70.0;

/// Widest opening angle considered, in degrees.
///
/// A leaf folded back flat against the wall. ARBITRARY — carried over from the
/// prototype (`index.html:270`).
const MAX_OPEN_DEGREES: f64 = 180.0;

/// Step used when searching for the widest workable angle, in degrees.
const OPEN_ANGLE_STEP_DEGREES: f64 = 1.0;

/// Past this angle, the first fouling ends the search.
///
/// Below it, a leaf may foul and then clear again as it swings; above it the
/// geometry is monotonic. ARBITRARY — carried over from the prototype
/// (`index.html:271`).
const MONOTONIC_ABOVE_DEGREES: f64 = 85.0;

/// The rectangle swept out by one leaf at rest, or `None` for a sliding gate.
fn leaf(scene: &Scene, post: &Post, side: f64) -> Option<Obb> {
    let GateKind::Swinging {
        leaf_length,
        leaf_thickness,
        hinge_offset,
        hinge_depth_ratio,
        open_angle,
    } = scene.gate
    else {
        return None;
    };

    // The hinge sits `hinge_offset` back from the post's inner face, and
    // `hinge_depth_ratio` of the way through its depth.
    let hinge = Point::new(
        post.inner_edge_x - side * hinge_offset,
        hinge_depth_ratio * post.depth,
    );

    let (sin, cos) = open_angle.sin_cos();
    let (dx, dy) = (-side * cos, sin);

    Some(Obb::new(
        Point::new(
            hinge.x + dx * leaf_length / 2.0,
            hinge.y + dy * leaf_length / 2.0,
        ),
        Radians::new(dy.atan2(dx)),
        leaf_length / 2.0,
        leaf_thickness / 2.0,
    ))
}

/// Both leaves of a swinging gate; empty for a sliding one.
pub(super) fn leaves(scene: &Scene) -> Vec<Obb> {
    [
        leaf(scene, &scene.right_post, 1.0),
        leaf(scene, &scene.left_post, -1.0),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// Whether either leaf currently overlaps its own post.
pub(super) fn hits_a_post(scene: &Scene) -> bool {
    let posts = [
        (
            &scene.right_post,
            Obb::from_bounds(
                scene.right_post.inner_edge_x,
                scene.right_post.inner_edge_x + scene.right_post.width,
                0.0,
                scene.right_post.depth,
            ),
            1.0,
        ),
        (
            &scene.left_post,
            Obb::from_bounds(
                scene.left_post.inner_edge_x - scene.left_post.width,
                scene.left_post.inner_edge_x,
                0.0,
                scene.left_post.depth,
            ),
            -1.0,
        ),
    ];

    posts.iter().any(|(post, body, side)| {
        leaf(scene, post, *side).is_some_and(|leaf| leaf.overlaps(body))
    })
}
```

Ajouter dans `crates/swept-core/src/scene/mod.rs` :

```rust
pub mod gate;

impl Scene {
    /// The widest angle these leaves can open to without fouling their posts.
    ///
    /// Returns [`MIN_OPEN_DEGREES`](gate) as a floor, and the maximum angle for
    /// a sliding gate, which nothing constrains.
    #[must_use]
    pub fn max_open_angle(&self) -> Radians {
        gate::max_open_angle(self)
    }
}
```

Et dans `gate.rs` :

```rust
/// Searches for the widest angle the leaves can open to.
///
/// The search walks degree by degree rather than solving in closed form: the
/// leaf-versus-post test already exists, and the answer is only ever shown to
/// a degree of precision.
pub(super) fn max_open_angle(scene: &Scene) -> Radians {
    if matches!(scene.gate, GateKind::Sliding) {
        return Radians::from_degrees(MAX_OPEN_DEGREES);
    }

    let mut best = MIN_OPEN_DEGREES;
    let mut degrees = MIN_OPEN_DEGREES;
    while degrees <= MAX_OPEN_DEGREES {
        let mut probe = *scene;
        if let GateKind::Swinging {
            ref mut open_angle, ..
        } = probe.gate
        {
            *open_angle = Radians::from_degrees(degrees);
        }

        if hits_a_post(&probe) {
            if degrees > MONOTONIC_ABOVE_DEGREES {
                break;
            }
        } else {
            best = degrees;
        }
        degrees += OPEN_ANGLE_STEP_DEGREES;
    }
    Radians::from_degrees(best)
}
```

Brancher enfin les vantaux dans `obstacles.rs`. Élargir l'import :

```rust
use super::{GateKind, Scene};
```

et ajouter, juste avant le `obstacles` final de `build` :

```rust
    if matches!(scene.gate, GateKind::Swinging { .. }) {
        obstacles.extend(super::gate::leaves(scene));
    }
```

Ajouter dans le `mod tests` de `scene/mod.rs` — où vit déjà l'assistante `symmetric()` — la contrepartie du test de la Task 10 :

```rust
    #[test]
    fn adds_two_leaves_for_a_swinging_gate() {
        let mut scene = symmetric();
        scene.gate = GateKind::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.10,
            hinge_offset: 0.05,
            hinge_depth_ratio: 0.5,
            open_angle: Radians::from_degrees(90.0),
        };
        // Seven for a sliding gate, plus the two leaves.
        assert_eq!(scene.obstacles().len(), 9);
    }
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core scene && cargo clippy --all-targets -- -D warnings`
Expected: dix tests passent, dont `reproduces_the_reference_opening_angles`.

Si ce dernier échoue, ne pas ajuster la tolérance pour le faire passer : l'écart signale soit une erreur de portage, soit que la valeur du `CLAUDE.md` reposait sur une convention différente pour `hinge_depth_ratio`. Comparer avec le prototype ouvert dans un navigateur, aux mêmes réglages, avant de toucher au code.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/scene-gate
git add crates/swept-core/src/scene/
git commit -m "feat(core): swinging leaves and maximum opening angle

Ports leafOB(), leafHitsPillar() and maxAngle() (index.html:229-236,
262-274). Validates the fourth reference result from CLAUDE.md: about 91
degrees with the hinge halfway through the pillar, about 118 with it moved
to the yard face."
```

---

### Task 12: La marge d'une pose

Porte `clearance()` et `obCorners()` (`prototype/index.html:282-310`). Dernière brique du noyau géométrique : elle réunit véhicule, scène et pose.

**Files:**
- Create: `crates/swept-core/src/clearance.rs`
- Modify: `crates/swept-core/src/lib.rs`

**Interfaces:**
- Consumes: `Vehicle::envelope` (Task 9), `Scene::obstacles` (Task 10), `Obb::distance_to` (Task 5), `Pose` (Task 7)
- Produces: `clearance::Clearance` (`Collision` / `Clear(f64)`) ; `clearance::ClearanceField::new(scene: &Scene, vehicle: &Vehicle) -> ClearanceField` ; `ClearanceField::at(&self, pose: Pose) -> Clearance`

Le champ précalcule ce qui ne dépend pas de la pose — obstacles, coins retenus, enveloppe — parce que la marge est évaluée des centaines de milliers de fois par recherche.

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `crates/swept-core/src/clearance.rs` :

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{GateKind, Post};
    use crate::units::Radians;

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
    fn reports_clearance_in_the_open_yard() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        // Well past the wall, pointing into the yard.
        let pose = Pose::new(0.0, 6.0, Radians::from_degrees(90.0));
        match field.at(pose) {
            Clearance::Clear(margin) => assert!(margin > 1.0, "got {margin}"),
            Clearance::Collision => panic!("the yard is empty"),
        }
    }

    #[test]
    fn reports_a_collision_through_the_wall() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        // Straddling the wall, across the opening rather than through it.
        let pose = Pose::new(6.0, 0.15, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn catches_an_obstacle_corner_inside_the_body() {
        // The case the reverse test exists for: the body swallows a corner
        // whole, while every sampled point sits in clear air.
        //
        // The vehicle lies broadside at y = 0.275, so its flanks pass at
        // y = 1.1875 and y = -0.6375 — above every obstacle and short of the
        // pavement. Its bumper centres pass at y = 0.275, clear of a wall only
        // 0.10 m thick. Yet the body spans x from -0.26 to 3.93, which
        // encloses the whole right-hand post.
        let scene = Scene {
            left_post: Post { inner_edge_x: -0.30, width: 0.55, depth: 0.55 },
            right_post: Post { inner_edge_x: 0.30, width: 0.55, depth: 0.55 },
            wall_thickness: 0.10,
            pavement_width: 1.20,
            dropped_kerb_width: 12.0,
            road_width: 4.50,
            gate: GateKind::Sliding,
        };
        let field = ClearanceField::new(&scene, &lbx());
        let pose = Pose::new(0.5, 0.275, Radians::default());
        assert_eq!(field.at(pose), Clearance::Collision);
    }

    #[test]
    fn margin_shrinks_as_the_vehicle_approaches_a_post() {
        let field = ClearanceField::new(&wide_scene(), &lbx());
        let far = Pose::new(0.0, 6.0, Radians::from_degrees(90.0));
        let near = Pose::new(1.2, 6.0, Radians::from_degrees(90.0));
        match (field.at(far), field.at(near)) {
            (Clearance::Clear(a), Clearance::Clear(b)) => assert!(b < a, "{b} should be under {a}"),
            _ => panic!("both poses are clear of the obstacles"),
        }
    }
}
```

- [ ] **Step 2: Vérifier l'échec**

Run: `cargo test -p swept-core clearance`
Expected: FAIL — `cannot find type ClearanceField in this scope`.

- [ ] **Step 3: Implémenter**

En tête de `crates/swept-core/src/clearance.rs` :

```rust
//! How much room a given pose leaves.
//!
//! Two tests run, and both are needed. The forward test walks the sampled
//! points of the vehicle outline against every obstacle. The reverse test
//! walks the obstacle corners against the vehicle's body rectangle — because a
//! pillar corner can sit inside the body without any sampled point falling
//! inside the pillar, and the forward test alone would call that clear.

use crate::geometry::{Obb, Point, PointDistance};
use crate::kinematics::Pose;
use crate::scene::Scene;
use crate::vehicle::Vehicle;

/// How much room a pose leaves.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clearance {
    /// The vehicle is touching something.
    Collision,
    /// Smallest distance from the vehicle outline to any obstacle, in metres.
    Clear(f64),
}

/// Above this half-size, an obstacle's corners are ignored by the reverse
/// test, in metres.
///
/// The scene's outer walls and the far side of the road are modelled as very
/// large rectangles whose corners sit far outside the scene; feeding them to
/// the reverse test would only waste work. ARBITRARY in magnitude — carried
/// over from the prototype (`index.html:285`).
pub const CORNER_TEST_MAX_HALF_SIZE_M: f64 = 12.0;

/// Everything about a scene and a vehicle that does not depend on the pose.
///
/// A single search evaluates hundreds of thousands of poses against one
/// unchanging scene, so the obstacle list, the retained corners and the
/// sampled outline are all built once.
#[derive(Debug, Clone)]
pub struct ClearanceField {
    obstacles: Vec<Obb>,
    corners: Vec<Point>,
    envelope: Vec<Point>,
    half_width: f64,
    rear: f64,
    front: f64,
}

impl ClearanceField {
    /// Prepares the field for one scene and one vehicle.
    #[must_use]
    pub fn new(scene: &Scene, vehicle: &Vehicle) -> Self {
        let obstacles = scene.obstacles();
        let corners = obstacles
            .iter()
            .filter(|o| {
                o.half_width <= CORNER_TEST_MAX_HALF_SIZE_M
                    && o.half_height <= CORNER_TEST_MAX_HALF_SIZE_M
            })
            .flat_map(|o| o.corners())
            .collect();

        Self {
            obstacles,
            corners,
            envelope: vehicle.envelope(),
            half_width: vehicle.width / 2.0,
            rear: -vehicle.rear_overhang,
            front: vehicle.wheelbase + vehicle.front_overhang,
        }
    }

    /// The clearance left by one pose.
    #[must_use]
    pub fn at(&self, pose: Pose) -> Clearance {
        let (sin, cos) = pose.heading.sin_cos();

        let mut smallest = f64::MAX;
        for local in &self.envelope {
            let point = Point::new(
                pose.x + local.x * cos - local.y * sin,
                pose.y + local.x * sin + local.y * cos,
            );
            for obstacle in &self.obstacles {
                match obstacle.distance_to(point) {
                    PointDistance::Inside => return Clearance::Collision,
                    PointDistance::Outside(d) => smallest = smallest.min(d),
                }
            }
        }

        // Reverse test: an obstacle corner inside the vehicle body.
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

Déclarer le module dans `lib.rs` :

```rust
pub mod clearance;
```

- [ ] **Step 4: Vérifier que les tests passent**

Run: `cargo test -p swept-core && cargo clippy --all-targets -- -D warnings && cargo doc --workspace --no-deps`
Expected: la totalité de la suite passe, aucun avertissement, la documentation se génère.

- [ ] **Step 5: Commiter**

```bash
git checkout -b feat/clearance
git add crates/swept-core/src/clearance.rs crates/swept-core/src/lib.rs
git commit -m "feat(core): clearance of a pose against a scene

Ports clearance() and obCorners() (index.html:282-310). The reverse corner
test is kept: a pillar corner can sit inside the body without any sampled
point sitting inside the pillar."
```

---

## Ce que ce plan ne fait pas

À l'issue de la Task 12, `swept-core` sait décrire une scène, un véhicule, une trajectoire et mesurer une marge — mais rien ne cherche encore de trajectoire. Cela relève du **lot 1b** : recherche exacte à un mouvement, dichotomie de chaussée minimale, A\* hybride, et les trois premiers résultats de référence du `CLAUDE.md`, qui ne peuvent être vérifiés qu'une fois les solveurs en place.

Le harnais `tools/extract-golden/` reste volontairement limité aux primitives. Les sorties de solveurs ne sont jamais figées en fixtures : leur comportement doit changer.

Les six véhicules en dur du prototype (`index.html:158-165`) ne sont pas portés ici non plus. La spec du lot 1 les conserve, mais ce sont des données d'interface, pas du domaine : les inscrire dans `swept-core` chargerait d'un catalogue une crate destinée à être publiée comme bibliothèque géométrique générique. Ils rejoignent donc la couche web au **lot 1c**, où `data/vehicles.json` les remplacera au lot 5.
