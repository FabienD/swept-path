# Lot 1c — Frontière Wasm et interface — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rendre le noyau utilisable : frontière WebAssembly, calcul en Web Worker, interface TypeScript avec vue en plan, et déploiement statique sur Vercel.

**Architecture:** L'interface ne connaît que le Worker, le Worker ne connaît que le Wasm, le Wasm ne connaît que le domaine. Le rendu se fait en deux temps — une fonction pure produit une liste de primitives de dessin, un backend SVG la traduit — de sorte que le moteur de rendu reste remplaçable.

**Tech Stack:** Rust + `wasm-bindgen`, TypeScript, Vite, Tailwind CSS 4, Vitest, déploiement Vercel prébuild depuis GitHub Actions.

## Global Constraints

- **Tout ce qui vit dans le dépôt est en anglais** : identifiants, documentation du code, noms de tests, branches, messages de commit. Seule la documentation projet (`docs/`) reste en français.
- **L'interface, elle, est en français.** Les libellés vivent dans une table côté TypeScript ; le noyau et le Wasm ne renvoient que des codes. C'est ce qui permet de tenir « code en anglais, interface en français ».
- `#![deny(missing_docs)]` et clippy pedantic sur le Rust ; `tsc --noEmit` sans erreur et Vitest vert sur le TypeScript.
- **Aucune règle métier dans `swept-wasm` ni dans le web.** Toute décision géométrique appartient à `swept-core` ou `swept-solver`. Si une formule apparaît dans le TypeScript, elle est au mauvais endroit.
- **Le thread principal ne calcule jamais.** Toute recherche passe par le Worker, sans exception : c'est le défaut n°1 du `CLAUDE.md`, et le seul moyen de ne pas le reproduire est de ne jamais offrir de chemin synchrone.
- Longueurs en mètres, angles en radians dans les échanges. Les degrés n'apparaissent qu'à l'affichage.
- `swept-wasm` et `web/` sont sous **AGPL-3.0**.
- Une seule PR ouverte à la fois, branchée sur `main`. Chaque tâche est une PR.
- Sur ce poste : `cp` et `rm` sont aliasés en interactif — utiliser Python ou git pour manipuler des fichiers en script. `node` n'est pas dans le `PATH` des shells non interactifs : passer par `$HOME/.nvm/versions/node/<version>/bin/node`.

---

## File Structure

| Fichier | Responsabilité |
|---|---|
| `crates/swept-wasm/Cargo.toml` | Paquet AGPL, cible `cdylib`, `wasm-bindgen` |
| `crates/swept-wasm/src/lib.rs` | Les trois fonctions exportées |
| `crates/swept-wasm/src/dto.rs` | Types de transfert et conversions vers le domaine |
| `web/package.json`, `vite.config.ts`, `tsconfig.json` | Chaîne de build |
| `web/index.html` | Page unique |
| `web/src/main.ts` | Câblage : formulaire → worker → rendu |
| `web/src/style.css` | Tailwind 4 et jetons de thème |
| `web/src/domain/types.ts` | Miroir TypeScript des DTO Wasm |
| `web/src/domain/vehicles.ts` | Les six véhicules du prototype |
| `web/src/domain/labels.ts` | Tous les libellés français, y compris les codes d'erreur |
| `web/src/worker/solver.worker.ts` | Charge le Wasm, exécute, rend compte |
| `web/src/worker/client.ts` | API typée du worker, annulation par remplacement |
| `web/src/state/store.ts` | Petit magasin observable |
| `web/src/render/primitives.ts` | La liste de primitives, indépendante du backend |
| `web/src/render/scene.ts` | Scène → primitives |
| `web/src/render/path.ts` | Trajectoire → primitives, bandes de proximité |
| `web/src/render/svg.ts` | Backend SVG |
| `web/src/ui/*.ts` | Formulaire, verdict, alternatives, curseur |
| `.github/workflows/ci.yml` | Ajout du build web et du déploiement |

---

### Task 1: La frontière Wasm

Frontière étroite : trois fonctions, un échange JSON. Aucune règle métier ici, uniquement des conversions.

**Files:**
- Create: `crates/swept-wasm/Cargo.toml`, `crates/swept-wasm/src/lib.rs`, `crates/swept-wasm/src/dto.rs`
- Modify: `Cargo.toml` (membre du workspace)

**Interfaces:**
- Consumes: `swept_core::{scene::*, vehicle::*}`, `swept_solver::{solve::alternatives, min_road::minimum_road_width, budget::*, result::*}`
- Produces: trois fonctions `#[wasm_bindgen]` — `solve(request: JsValue) -> Result<JsValue, JsValue>`, `min_road(request: JsValue) -> Result<JsValue, JsValue>`, `max_gate_angle(scene: JsValue) -> Result<f64, JsValue>` (radians) ; les DTO `SceneDto`, `VehicleDto`, `SolveRequest`, `SolveResponse`, `ManeuverDto`, `PoseDto`, `ErrorDto`

- [ ] **Step 1: Déclarer le paquet**

Fichier `crates/swept-wasm/Cargo.toml` :

```toml
[package]
name = "swept-wasm"
version = "0.1.0"
description = "WebAssembly boundary for swept path analysis"
license = "AGPL-3.0-only"
edition.workspace = true
rust-version.workspace = true
authors.workspace = true
repository.workspace = true

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
swept-core = { path = "../swept-core" }
swept-solver = { path = "../swept-solver" }
wasm-bindgen = "0.2.127"
serde = { version = "1.0.229", features = ["derive"] }
serde-wasm-bindgen = "0.6.5"
console_error_panic_hook = { version = "0.1.7", optional = true }

[dev-dependencies]
wasm-bindgen-test = "0.3.77"

[features]
default = ["console_error_panic_hook"]

[lints]
workspace = true
```

Ajouter `"crates/swept-wasm"` aux membres du workspace racine.

- [ ] **Step 2: Écrire le test qui échoue**

Fichier `crates/swept-wasm/src/dto.rs`, section de test :

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn scene_dto() -> SceneDto {
        SceneDto {
            left_post: PostDto { inner_edge_x: -1.20, width: 0.55, depth: 0.55 },
            right_post: PostDto { inner_edge_x: 1.20, width: 0.55, depth: 0.55 },
            wall_thickness: 0.30,
            pavement_width: 1.20,
            dropped_kerb_width: 3.20,
            road_width: 4.50,
            gate: GateDto::Sliding,
        }
    }

    fn vehicle_dto() -> VehicleDto {
        VehicleDto {
            wheelbase: 2.580,
            length: 4.190,
            front_overhang: 0.850,
            width: 1.825,
            mirror_width: 2.029,
            min_turning_radius: 5.2,
        }
    }

    #[test]
    fn a_valid_vehicle_converts() {
        let vehicle = vehicle_dto().into_domain().expect("valid");
        assert!((vehicle.rear_overhang - 0.760).abs() < 1e-9);
    }

    #[test]
    fn an_invalid_vehicle_reports_the_offending_field() {
        let mut dto = vehicle_dto();
        dto.mirror_width = 1.0;
        let err = dto.into_domain().unwrap_err();
        assert_eq!(err.code, "mirrors_narrower_than_body");
        // The message is the caller's business, not ours.
        assert!(err.field.is_none() || err.field.as_deref() == Some("mirror_width"));
    }

    #[test]
    fn a_non_positive_dimension_names_its_field() {
        let mut dto = vehicle_dto();
        dto.wheelbase = 0.0;
        let err = dto.into_domain().unwrap_err();
        assert_eq!(err.code, "non_positive");
        assert_eq!(err.field.as_deref(), Some("wheelbase"));
    }

    #[test]
    fn a_sliding_scene_converts() {
        let scene = scene_dto().into_domain();
        assert!((scene.opening_width() - 2.40).abs() < 1e-12);
    }

    #[test]
    fn a_swinging_scene_carries_its_angle_in_radians() {
        let mut dto = scene_dto();
        dto.gate = GateDto::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.10,
            hinge_offset: 0.05,
            hinge_depth_ratio: 0.5,
            open_angle: std::f64::consts::FRAC_PI_2,
        };
        match dto.into_domain().gate {
            swept_core::scene::GateKind::Swinging { open_angle, .. } => {
                assert!((open_angle.to_degrees() - 90.0).abs() < 1e-9);
            }
            swept_core::scene::GateKind::Sliding => panic!("expected a swinging gate"),
        }
    }
}
```

- [ ] **Step 3: Vérifier l'échec**

Run: `cargo test -p swept-wasm`
Expected: FAIL — `cannot find type SceneDto in this scope`.

- [ ] **Step 4: Implémenter les DTO**

En tête de `crates/swept-wasm/src/dto.rs` :

```rust
//! Types crossing the WebAssembly boundary.
//!
//! These exist so that the domain never has to know about serialisation.
//! Nothing here decides anything: it converts, validates by delegation, and
//! turns domain errors into codes the interface can translate.

use serde::{Deserialize, Serialize};
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::{Vehicle, VehicleError};

/// A rejected input, as a code the interface turns into French.
///
/// The wording belongs to the interface. Sending a message from here would
/// put language in the domain layer, which is exactly what `CLAUDE.md`
/// separates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorDto {
    /// Machine-readable reason, e.g. `non_positive`.
    pub code: String,
    /// Which field is at fault, when one can be named.
    pub field: Option<String>,
}

/// One side of the opening.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PostDto {
    /// Abscissa of the inner face, in metres.
    pub inner_edge_x: f64,
    /// Width along `x`, in metres.
    pub width: f64,
    /// Depth along `y`, in metres.
    pub depth: f64,
}

/// What closes the opening.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GateDto {
    /// Retracts alongside the wall.
    Sliding,
    /// A pair of leaves standing in the opening.
    Swinging {
        /// Length of one leaf, in metres.
        leaf_length: f64,
        /// Thickness of a leaf, in metres.
        leaf_thickness: f64,
        /// Gap between hinge axis and the post's inner face, in metres.
        hinge_offset: f64,
        /// Where the hinge sits through the post depth, 0 to 1.
        hinge_depth_ratio: f64,
        /// Opening angle, **in radians**.
        open_angle: f64,
    },
}

/// A scene, as the interface sends it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SceneDto {
    /// Post on the negative `x` side.
    pub left_post: PostDto,
    /// Post on the positive `x` side.
    pub right_post: PostDto,
    /// Wall thickness, in metres.
    pub wall_thickness: f64,
    /// Pavement width, in metres; zero for none.
    pub pavement_width: f64,
    /// Dropped kerb width, in metres.
    pub dropped_kerb_width: f64,
    /// Carriageway width, in metres.
    pub road_width: f64,
    /// What closes the opening.
    pub gate: GateDto,
}

impl SceneDto {
    /// Converts to the domain type. A scene has no validation of its own.
    #[must_use]
    pub fn into_domain(self) -> Scene {
        Scene {
            left_post: Post {
                inner_edge_x: self.left_post.inner_edge_x,
                width: self.left_post.width,
                depth: self.left_post.depth,
            },
            right_post: Post {
                inner_edge_x: self.right_post.inner_edge_x,
                width: self.right_post.width,
                depth: self.right_post.depth,
            },
            wall_thickness: self.wall_thickness,
            pavement_width: self.pavement_width,
            dropped_kerb_width: self.dropped_kerb_width,
            road_width: self.road_width,
            gate: match self.gate {
                GateDto::Sliding => GateKind::Sliding,
                GateDto::Swinging {
                    leaf_length,
                    leaf_thickness,
                    hinge_offset,
                    hinge_depth_ratio,
                    open_angle,
                } => GateKind::Swinging {
                    leaf_length,
                    leaf_thickness,
                    hinge_offset,
                    hinge_depth_ratio,
                    open_angle: Radians::new(open_angle),
                },
            },
        }
    }
}

/// A vehicle, as the interface sends it.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct VehicleDto {
    /// Distance between axles, in metres.
    pub wheelbase: f64,
    /// Total length, in metres.
    pub length: f64,
    /// Front axle to front bumper, in metres.
    pub front_overhang: f64,
    /// Body width, in metres.
    pub width: f64,
    /// Width over the mirrors, in metres.
    pub mirror_width: f64,
    /// Tightest turning radius, in metres.
    pub min_turning_radius: f64,
}

impl VehicleDto {
    /// Converts to the domain type, delegating validation to it.
    ///
    /// # Errors
    ///
    /// Returns an [`ErrorDto`] naming the rule that was broken.
    pub fn into_domain(self) -> Result<Vehicle, ErrorDto> {
        Vehicle::new(
            self.wheelbase,
            self.length,
            self.front_overhang,
            self.width,
            self.mirror_width,
            self.min_turning_radius,
        )
        .map_err(|e| match e {
            VehicleError::NonPositive(field) => ErrorDto {
                code: "non_positive".to_owned(),
                field: Some(field.to_owned()),
            },
            VehicleError::FrontOverhangTooLarge => ErrorDto {
                code: "front_overhang_too_large".to_owned(),
                field: Some("front_overhang".to_owned()),
            },
            VehicleError::MirrorsNarrowerThanBody => ErrorDto {
                code: "mirrors_narrower_than_body".to_owned(),
                field: Some("mirror_width".to_owned()),
            },
        })
    }
}
```

- [ ] **Step 5: Ajouter les types de sortie**

Toujours dans `dto.rs` :

```rust
/// One sampled pose along a manoeuvre.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PoseDto {
    /// Along the road, in metres.
    pub x: f64,
    /// Away from the road, in metres.
    pub y: f64,
    /// Heading, **in radians**.
    pub heading: f64,
    /// `true` when reversing.
    pub reverse: bool,
    /// Clearance at this pose, in metres.
    pub clearance: f64,
}

/// Where a result came from, flattened for the interface.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceDto {
    /// Exhaustive sweep: a failure proves absence on the grid.
    Exact,
    /// Heuristic search: a failure proves nothing.
    Heuristic,
    /// Heuristic search that ran out of budget.
    HeuristicExhausted,
}

/// One way in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManeuverDto {
    /// The sampled path.
    pub poses: Vec<PoseDto>,
    /// Tightest clearance anywhere, in metres.
    pub min_clearance: f64,
    /// Tightest clearance **within the gateway**, in metres.
    ///
    /// Separate from `min_clearance` because grazing a kerb six metres short
    /// of the gate does not mean the same thing to a driver as grazing a post.
    pub min_clearance_in_gateway: f64,
    /// Distance travelled within 25 cm of an obstacle, in metres.
    pub metres_under_25cm: f64,
    /// Distance travelled within 10 cm of an obstacle, in metres.
    pub metres_under_10cm: f64,
    /// Total distance travelled, in metres.
    pub distance: f64,
    /// Number of moves.
    pub moves: u8,
    /// Where this came from.
    pub confidence: ConfidenceDto,
}

/// What a solve returns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveResponse {
    /// One alternative per move count, fewest first. Empty when none found.
    pub alternatives: Vec<ManeuverDto>,
    /// Set when the search stopped on its budget rather than exhausting the
    /// space — the interface must not present an empty result as proof.
    pub budget_exhausted: bool,
}
```

Note pour l'implémenteur : `metres_under_25cm` et `metres_under_10cm` se calculent en **sommant les distances réelles entre poses consécutives**, pas en comptant les poses. Le prototype multipliait un nombre de poses par un pas supposé constant (`index.html:614`) ; ce pas est désormais paramétrable, et la formule serait fausse.

- [ ] **Step 6: Définir la requête et le travail de conversion**

Toujours dans `dto.rs` :

```rust
/// What the interface asks for.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SolveRequest {
    /// The scene to get into.
    pub scene: SceneDto,
    /// The vehicle attempting it.
    pub vehicle: VehicleDto,
    /// `Some(true)` to force driving in, `Some(false)` to force reversing,
    /// `None` to consider both.
    pub forward_only: Option<bool>,
}

/// Bounds of the constrained corridor, in metres of `y`.
///
/// Everything between the outer face of the wall and the far side of the
/// posts — plus the leaves when they swing into the way. Used to report the
/// clearance that actually concerns a driver, separately from the tightest
/// point anywhere on the approach.
fn corridor_depth(scene: &Scene) -> f64 {
    let gate = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth) + gate
}

/// Runs a search and converts everything back.
///
/// # Errors
///
/// Returns an [`ErrorDto`] when the vehicle dimensions are rejected.
pub fn run_solve(request: SolveRequest) -> Result<SolveResponse, ErrorDto> {
    let vehicle = request.vehicle.into_domain()?;
    let scene = request.scene.into_domain();
    let allowed = request.forward_only.map(|forward| {
        if forward {
            Direction::Forward
        } else {
            Direction::Reverse
        }
    });

    let outcome = alternatives(
        &vehicle,
        &scene,
        SearchBudget::default(),
        &mut Silent,
        allowed,
    );

    let field = ClearanceField::new(&scene, &vehicle);
    let corridor = corridor_depth(&scene);

    match outcome {
        Outcome::NotFound { budget_exhausted } => Ok(SolveResponse {
            alternatives: Vec::new(),
            budget_exhausted,
        }),
        Outcome::Found(list) => Ok(SolveResponse {
            alternatives: list
                .into_iter()
                .map(|m| describe(&m, &field, corridor))
                .collect(),
            budget_exhausted: false,
        }),
    }
}

/// Distances, in metres, below which a stretch counts as an alert.
const ALERT_BANDS_M: [f64; 2] = [0.25, 0.10];

/// Annotates a manoeuvre with everything the interface needs to draw it.
fn describe(maneuver: &Maneuver, field: &ClearanceField, corridor: f64) -> ManeuverDto {
    let mut poses = Vec::with_capacity(maneuver.poses.len());
    for step in &maneuver.poses {
        let clearance = match field.at(step.pose) {
            Clearance::Clear(margin) => margin,
            Clearance::Collision => 0.0,
        };
        poses.push(PoseDto {
            x: step.pose.x,
            y: step.pose.y,
            heading: step.pose.heading.get(),
            reverse: step.direction == Direction::Reverse,
            clearance,
        });
    }

    // Alert distances are summed from real spacing between poses. The
    // prototype multiplied a pose count by a fixed step (`index.html:614`),
    // which stopped being true once the sampling step became tunable.
    let mut distance = 0.0;
    let mut under = [0.0_f64; 2];
    for pair in poses.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let span = (b.x - a.x).hypot(b.y - a.y);
        distance += span;
        for (i, threshold) in ALERT_BANDS_M.iter().enumerate() {
            if b.clearance < *threshold {
                under[i] += span;
            }
        }
    }

    let min_clearance_in_gateway = poses
        .iter()
        .filter(|p| p.y >= 0.0 && p.y <= corridor)
        .map(|p| p.clearance)
        .fold(f64::INFINITY, f64::min);

    ManeuverDto {
        poses,
        min_clearance: maneuver.min_clearance,
        // A path that never crosses the corridor has no gateway clearance;
        // report the overall figure rather than infinity.
        min_clearance_in_gateway: if min_clearance_in_gateway.is_finite() {
            min_clearance_in_gateway
        } else {
            maneuver.min_clearance
        },
        metres_under_25cm: under[0],
        metres_under_10cm: under[1],
        distance,
        moves: maneuver.moves,
        confidence: match maneuver.confidence {
            Confidence::Exact => ConfidenceDto::Exact,
            Confidence::Heuristic {
                budget_exhausted: false,
            } => ConfidenceDto::Heuristic,
            Confidence::Heuristic {
                budget_exhausted: true,
            } => ConfidenceDto::HeuristicExhausted,
        },
    }
}
```

Ajouter un test vérifiant que `min_clearance_in_gateway` diffère bien de `min_clearance` quand le point le plus serré est hors du couloir — c'est précisément le cas que le lot 1b a mis au jour.

- [ ] **Step 7: Exposer les trois fonctions**

Fichier `crates/swept-wasm/src/lib.rs` :

```rust
//! The WebAssembly boundary.
//!
//! Three functions, JSON in and JSON out. No rule lives here: this layer
//! converts, calls the solvers, and converts back.

pub mod dto;

use dto::{ErrorDto, SceneDto, SolveRequest, SolveResponse, VehicleDto};
use wasm_bindgen::prelude::*;

/// Installs a panic hook that reports to the console, in debug builds only.
#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Finds every way in, one alternative per move count.
///
/// # Errors
///
/// Returns an [`ErrorDto`] when the request cannot be decoded or the vehicle
/// dimensions are rejected.
#[wasm_bindgen]
pub fn solve(request: JsValue) -> Result<JsValue, JsValue> {
    let request: SolveRequest = serde_wasm_bindgen::from_value(request).map_err(decode_error)?;
    let response = dto::run_solve(request)?;
    serde_wasm_bindgen::to_value(&response).map_err(|e| encode_error(&e))
}
```

Les deux autres, en entier :

```rust
/// The narrowest carriageway admitting a one-move forward entry, in metres.
///
/// Returns `null` when no width up to the search ceiling works, which means
/// the opening itself is blocking rather than the road.
///
/// # Errors
///
/// Returns an [`ErrorDto`] when the request cannot be decoded or the vehicle
/// dimensions are rejected.
#[wasm_bindgen]
pub fn min_road(request: JsValue) -> Result<JsValue, JsValue> {
    let request: SolveRequest = serde_wasm_bindgen::from_value(request).map_err(decode_error)?;
    let vehicle = request.vehicle.into_domain().map_err(domain_error)?;
    let scene = request.scene.into_domain();
    let width = swept_solver::min_road::minimum_road_width(&vehicle, &scene);
    serde_wasm_bindgen::to_value(&width).map_err(|e| encode_error(&e))
}

/// The widest angle the leaves can open to without fouling their posts.
///
/// In **radians**, like everything crossing this boundary.
///
/// # Errors
///
/// Returns an [`ErrorDto`] when the scene cannot be decoded.
#[wasm_bindgen]
pub fn max_gate_angle(scene: JsValue) -> Result<f64, JsValue> {
    let scene: SceneDto = serde_wasm_bindgen::from_value(scene).map_err(decode_error)?;
    Ok(scene.into_domain().max_open_angle().get())
}

/// Turns a decoding failure into an `ErrorDto` the interface can translate.
fn decode_error(error: serde_wasm_bindgen::Error) -> JsValue {
    to_js(&ErrorDto {
        code: "bad_request".to_owned(),
        field: None,
    })
    .unwrap_or_else(|_| JsValue::from_str(&error.to_string()))
}

/// Same, for an encoding failure on the way out.
fn encode_error(error: &serde_wasm_bindgen::Error) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Same, for a rejected set of dimensions.
fn domain_error(error: ErrorDto) -> JsValue {
    to_js(&error).unwrap_or_else(|_| JsValue::from_str("bad_request"))
}

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::to_value(value)
}
```

`solve` utilise `domain_error` de la même façon lorsque `run_solve` échoue.

- [ ] **Step 8: Test d'intégration en Node**

Fichier `crates/swept-wasm/tests/boundary.rs` :

```rust
//! Exercises the boundary the way the worker will.
//!
//! Run with `wasm-pack test --node crates/swept-wasm`.

use wasm_bindgen_test::wasm_bindgen_test;

#[wasm_bindgen_test]
fn a_generous_opening_solves_across_the_boundary() {
    let request = serde_wasm_bindgen::to_value(&sample_request(5.0)).expect("encodes");
    let response = swept_wasm::solve(request).expect("solves");
    let decoded: swept_wasm::dto::SolveResponse =
        serde_wasm_bindgen::from_value(response).expect("decodes");
    assert!(!decoded.alternatives.is_empty());
    assert!(decoded.alternatives[0].min_clearance > 0.0);
}

#[wasm_bindgen_test]
fn an_invalid_vehicle_crosses_back_as_a_code() {
    // A vehicle whose mirrors are narrower than its body.
    let mut request = sample_request(5.0);
    request.vehicle.mirror_width = 1.0;
    let encoded = serde_wasm_bindgen::to_value(&request).expect("encodes");
    let err = swept_wasm::solve(encoded).expect_err("must be rejected");
    let decoded: swept_wasm::dto::ErrorDto =
        serde_wasm_bindgen::from_value(err).expect("decodes");
    assert_eq!(decoded.code, "mirrors_narrower_than_body");
}
```

Run: `wasm-pack test --node crates/swept-wasm`
Expected: les deux tests passent.

- [ ] **Step 9: Commiter**

```bash
git checkout -b feat/wasm-boundary
git add Cargo.toml Cargo.lock crates/swept-wasm/
git commit -m "feat(wasm): narrow boundary, three functions, JSON both ways

No rule lives at the boundary: it converts, calls the solvers, converts
back. Domain errors cross as codes rather than messages, so that no French
ends up inside the domain — the interface owns the wording.

Adds min_clearance_in_gateway alongside min_clearance: grazing a kerb six
metres short of the gate does not mean the same thing to a driver as
grazing a post, and batch 1b showed the planner does exactly that.

The alert distances are summed from real pose-to-pose spacing rather than
counted in poses times a fixed step, which the prototype assumed
(index.html:614) and which stopped being true once the sampling step became
tunable."
```

---

### Task 2: La coque web et le Worker

Le calcul ne doit jamais toucher au thread principal. Cette tâche établit la chaîne complète — formulaire, worker, wasm, verdict textuel — avant tout rendu graphique, pour que l'ossature soit prouvée avant d'être habillée.

**Files:**
- Create: `web/package.json`, `web/tsconfig.json`, `web/vite.config.ts`, `web/index.html`, `web/src/main.ts`, `web/src/style.css`, `web/src/domain/types.ts`, `web/src/domain/labels.ts`, `web/src/worker/solver.worker.ts`, `web/src/worker/client.ts`, `web/src/state/store.ts`
- Modify: `.github/workflows/ci.yml`

**Interfaces:**
- Consumes: le paquet Wasm produit par `wasm-pack build --target web crates/swept-wasm`
- Produces: `SolverClient` avec `solve(request): Promise<SolveResponse>` et `onProgress(cb)` ; le magasin `createStore<T>(initial)` avec `get()`, `set(patch)`, `subscribe(fn)`

- [ ] **Step 1: Mettre en place la chaîne**

```bash
mkdir -p web/src/{domain,worker,state,render,ui}
cd web && npm init -y
npm install -D vite@latest typescript@latest vitest@latest @tailwindcss/vite@latest tailwindcss@latest
```

**Vérifier immédiatement la version de TypeScript installée.** Si `npx tsc --version` annonce une 7.x, c'est la réécriture native : plus rapide, mais jeune. Compiler un fichier trivial et lancer `vite build` avant d'aller plus loin. En cas de friction avec Vite ou Vitest, revenir à la dernière 5.x — c'est un repli prévu, pas un échec.

- [ ] **Step 2: Écrire le test qui échoue**

Fichier `web/src/state/store.test.ts` :

```ts
import { describe, expect, it, vi } from "vitest";
import { createStore } from "./store";

describe("store", () => {
  it("notifies subscribers when a field changes", () => {
    const store = createStore({ openingWidth: 2.4, busy: false });
    const seen = vi.fn();
    store.subscribe(seen);

    store.set({ openingWidth: 3.0 });

    expect(seen).toHaveBeenCalledTimes(1);
    expect(store.get().openingWidth).toBe(3.0);
    expect(store.get().busy).toBe(false);
  });

  it("stays silent when a set changes nothing", () => {
    const store = createStore({ openingWidth: 2.4 });
    const seen = vi.fn();
    store.subscribe(seen);

    store.set({ openingWidth: 2.4 });

    expect(seen).not.toHaveBeenCalled();
  });

  it("stops notifying after unsubscribe", () => {
    const store = createStore({ n: 0 });
    const seen = vi.fn();
    const stop = store.subscribe(seen);
    stop();

    store.set({ n: 1 });

    expect(seen).not.toHaveBeenCalled();
  });
});
```

Run: `npx vitest run`
Expected: FAIL — `Failed to resolve import "./store"`.

- [ ] **Step 3: Implémenter le magasin**

Fichier `web/src/state/store.ts` :

```ts
/** A subscriber, called after any change that actually changed something. */
export type Listener = () => void;

/** The smallest observable store that does the job. */
export interface Store<T> {
  get(): Readonly<T>;
  /** Merges a patch. Silent when nothing changes — the SVG is rebuilt on
   *  every notification, so spurious ones cost real work. */
  set(patch: Partial<T>): void;
  subscribe(listener: Listener): () => void;
}

export function createStore<T extends object>(initial: T): Store<T> {
  let state = { ...initial };
  const listeners = new Set<Listener>();

  return {
    get: () => state,
    set(patch) {
      const changed = Object.entries(patch).some(
        ([key, value]) => state[key as keyof T] !== value,
      );
      if (!changed) return;
      state = { ...state, ...patch };
      for (const listener of listeners) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
  };
}
```

- [ ] **Step 4: Déclarer les types partagés**

Fichier `web/src/domain/types.ts` — le miroir exact des DTO Rust. Toute
divergence ici est un bug silencieux : `serde` accepte les champs manquants
comme `undefined` sans se plaindre.

```ts
export interface PostDto {
  inner_edge_x: number;
  width: number;
  depth: number;
}

export type GateDto =
  | { kind: "sliding" }
  | {
      kind: "swinging";
      leaf_length: number;
      leaf_thickness: number;
      hinge_offset: number;
      hinge_depth_ratio: number;
      /** Radians. */
      open_angle: number;
    };

export interface SceneDto {
  left_post: PostDto;
  right_post: PostDto;
  wall_thickness: number;
  pavement_width: number;
  dropped_kerb_width: number;
  road_width: number;
  gate: GateDto;
}

export interface VehicleDto {
  wheelbase: number;
  length: number;
  front_overhang: number;
  width: number;
  mirror_width: number;
  min_turning_radius: number;
}

export interface SolveRequest {
  scene: SceneDto;
  vehicle: VehicleDto;
  /** `null` considers both directions. */
  forward_only: boolean | null;
}

export interface PoseDto {
  x: number;
  y: number;
  /** Radians. */
  heading: number;
  reverse: boolean;
  clearance: number;
}

export type ConfidenceDto = "exact" | "heuristic" | "heuristic_exhausted";

export interface ManeuverDto {
  poses: PoseDto[];
  min_clearance: number;
  min_clearance_in_gateway: number;
  metres_under_25cm: number;
  metres_under_10cm: number;
  distance: number;
  moves: number;
  confidence: ConfidenceDto;
}

export interface SolveResponse {
  alternatives: ManeuverDto[];
  budget_exhausted: boolean;
}

export interface ErrorDto {
  code: string;
  field: string | null;
}

/** Messages the worker accepts. */
export type WorkerIn =
  | { kind: "solve"; id: number; request: SolveRequest }
  | { kind: "minRoad"; id: number; request: SolveRequest }
  | { kind: "maxGateAngle"; id: number; scene: SceneDto };

/** Messages the worker sends back. */
export type WorkerOut =
  | { kind: "solved"; id: number; response: SolveResponse }
  | { kind: "minRoad"; id: number; response: number | null }
  | { kind: "maxGateAngle"; id: number; radians: number }
  | { kind: "failed"; id: number; error: ErrorDto };
```

Un test garde la correspondance : construire une requête depuis ces types,
la faire traverser le Wasm, et vérifier que la réponse porte bien tous les
champs déclarés. C'est le seul point du système où deux définitions du même
objet coexistent, donc le seul endroit où elles peuvent diverger.

- [ ] **Step 5: Écrire le worker et son client**

Fichier `web/src/worker/solver.worker.ts` :

```ts
/// <reference lib="webworker" />
import init, { solve, min_road, max_gate_angle } from "swept-wasm";
import type { SolveRequest, WorkerIn, WorkerOut } from "../domain/types";

let ready: Promise<unknown> | null = null;

/** Loads the Wasm module once, lazily. */
function ensureReady() {
  ready ??= init();
  return ready;
}

self.onmessage = async (event: MessageEvent<WorkerIn>) => {
  const message = event.data;
  try {
    await ensureReady();
    const post = (out: WorkerOut) => self.postMessage(out);
    switch (message.kind) {
      case "solve":
        post({ kind: "solved", id: message.id, response: solve(message.request) });
        break;
      case "minRoad":
        post({ kind: "minRoad", id: message.id, response: min_road(message.request) });
        break;
      case "maxGateAngle":
        post({ kind: "maxGateAngle", id: message.id, radians: max_gate_angle(message.scene) });
        break;
    }
  } catch (error) {
    self.postMessage({ kind: "failed", id: message.id, error });
  }
};
```

Fichier `web/src/worker/client.ts` :

```ts
import type { SolveRequest, SolveResponse } from "../domain/types";

/**
 * Talks to the solver worker.
 *
 * Cancellation is termination: starting a new search kills the worker still
 * running the old one. The core has no notion of being interrupted, and does
 * not need one — which is why it stayed free of any clock.
 */
export class SolverClient {
  #worker: Worker | null = null;
  #nextId = 0;

  #spawn(): Worker {
    return new Worker(new URL("./solver.worker.ts", import.meta.url), {
      type: "module",
    });
  }

  /** Runs a search, abandoning any search already in flight. */
  solve(request: SolveRequest): Promise<SolveResponse> {
    this.cancel();
    const worker = this.#spawn();
    this.#worker = worker;
    const id = ++this.#nextId;

    return new Promise((resolve, reject) => {
      worker.onmessage = (event) => {
        const out = event.data;
        if (out.id !== id) return;
        if (out.kind === "solved") resolve(out.response);
        else if (out.kind === "failed") reject(out.error);
      };
      worker.onerror = reject;
      worker.postMessage({ kind: "solve", id, request });
    });
  }

  /** Kills the worker, abandoning whatever it was doing. */
  cancel(): void {
    this.#worker?.terminate();
    this.#worker = null;
  }
}
```

- [ ] **Step 6: Câbler un verdict textuel**

`web/index.html` porte le formulaire minimal — passage entre piliers, type de portail, largeur de chaussée, choix de véhicule — plus une zone de verdict. `main.ts` écoute le formulaire, appelle le client, et affiche le verdict en français via `labels.ts`.

Vérification manuelle exigée : lancer `npm run dev`, saisir une ouverture de 2 m, **et faire défiler la page pendant le calcul**. Si le défilement saccade, une part du calcul est restée sur le thread principal et la tâche n'est pas finie.

- [ ] **Step 7: Ajouter le web à la CI**

```yaml
  web:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 24
          cache: npm
          cache-dependency-path: web/package-lock.json
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.97.1
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2
      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Build wasm
        run: wasm-pack build --target web --out-dir ../../web/src/generated crates/swept-wasm
      - name: Install
        run: npm ci
        working-directory: web
      - name: Typecheck
        run: npx tsc --noEmit
        working-directory: web
      - name: Tests
        run: npx vitest run
        working-directory: web
      - name: Build
        run: npm run build
        working-directory: web
```

Ajouter `web/src/generated/` au `.gitignore` : c'est un artefact de build.

- [ ] **Step 8: Commiter**

```bash
git checkout -b feat/web-shell-and-worker
git add web/ .github/ .gitignore
git commit -m "feat(web): Vite shell, Tailwind, and the solver worker

The chain is proved before it is dressed: form, worker, wasm, textual
verdict, no drawing yet. Cancellation is termination — starting a search
kills the worker running the previous one, which is why the core never
needed a notion of interruption.

The main thread never computes. That is defect 1 in CLAUDE.md, and the only
way not to reproduce it is to offer no synchronous path at all."
```

---

### Task 3: Déploiement Vercel

Mise à l'épreuve tôt de la chaîne de livraison, avant que l'interface ne représente un investissement.

**Files:**
- Create: `vercel.json`
- Modify: `.github/workflows/ci.yml`, `.github/workflows/deploy.yml`

**Interfaces:**
- Consumes: la sortie de `npm run build` dans `web/dist`
- Produces: une preview par PR, la production sur `main`

- [ ] **Step 1: Configurer**

Vercel ne compile jamais de Rust : la CI construit tout et pousse un artefact prébuild.

Fichier `vercel.json` :

```json
{
  "$schema": "https://openapi.vercel.sh/vercel.json",
  "buildCommand": null,
  "outputDirectory": "web/dist",
  "framework": null,
  "headers": [
    {
      "source": "/(.*)\\.wasm",
      "headers": [{ "key": "Content-Type", "value": "application/wasm" }]
    }
  ]
}
```

- [ ] **Step 2: Écrire le workflow de déploiement**

Fichier `.github/workflows/deploy.yml` :

```yaml
name: Deploy

on:
  push:
    branches: [main]
  pull_request:

concurrency:
  group: deploy-${{ github.ref }}
  cancel-in-progress: true

env:
  VERCEL_ORG_ID: ${{ secrets.VERCEL_ORG_ID }}
  VERCEL_PROJECT_ID: ${{ secrets.VERCEL_PROJECT_ID }}

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v7
      - uses: actions/setup-node@v7
        with:
          node-version: 24
          cache: npm
          cache-dependency-path: web/package-lock.json
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: 1.97.1
          targets: wasm32-unknown-unknown
      - uses: Swatinem/rust-cache@v2

      - name: Install wasm-pack
        run: curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
      - name: Build wasm
        run: wasm-pack build --target web --out-dir ../../web/src/generated crates/swept-wasm
      - name: Install web dependencies
        run: npm ci
        working-directory: web
      - name: Build site
        run: npm run build
        working-directory: web

      - name: Install Vercel CLI
        run: npm install -g vercel@latest
      - name: Pull Vercel environment
        run: vercel pull --yes --token=${{ secrets.VERCEL_TOKEN }}
          --environment=${{ github.ref == 'refs/heads/main' && 'production' || 'preview' }}
      - name: Prepare the prebuilt artefact
        run: vercel build --token=${{ secrets.VERCEL_TOKEN }}
          ${{ github.ref == 'refs/heads/main' && '--prod' || '' }}
      - name: Deploy
        run: vercel deploy --prebuilt --token=${{ secrets.VERCEL_TOKEN }}
          ${{ github.ref == 'refs/heads/main' && '--prod' || '' }}
```

Trois secrets à créer dans le dépôt : `VERCEL_TOKEN`, `VERCEL_ORG_ID`, `VERCEL_PROJECT_ID`. Les deux derniers se lisent dans `.vercel/project.json` après un `vercel link` local — **ce répertoire est déjà dans le `.gitignore`** et doit y rester.

Ce workflow reconstruit le Wasm plutôt que de récupérer l'artefact du job `web` : un `actions/upload-artifact` économiserait deux minutes, au prix d'un couplage entre deux workflows. À reconsidérer si la CI devient lente.

- [ ] **Step 3: Vérifier**

Ouvrir la PR et suivre le lien de preview. La page doit charger, le Wasm doit s'initialiser (vérifier l'onglet réseau : le `.wasm` arrive en `application/wasm`), et une recherche doit rendre un verdict.

Si le Wasm ne charge pas, c'est presque toujours le type MIME ou le chemin de `import.meta.url` dans le worker après bundling — vérifier ces deux points avant tout le reste.

- [ ] **Step 4: Commiter**

```bash
git checkout -b feat/vercel-deploy
git add vercel.json .github/
git commit -m "feat(deploy): prebuilt Vercel deploys from CI

Vercel serves a static directory and never compiles Rust: CI already has
the toolchain, so it builds the wasm and the bundle, then ships the
artefact. Preview per PR, production on main.

Deliberately early: the delivery chain is worth breaking now, while the
interface is a form and a paragraph."
```

---

### Task 4: La liste de primitives et le backend SVG

Le rendu se fait en deux temps. Une fonction pure décide *quoi* dessiner, un backend décide *comment*. C'est ce qui rend le moteur remplaçable — et, accessoirement, testable sans navigateur.

**Files:**
- Create: `web/src/render/primitives.ts`, `web/src/render/projection.ts`, `web/src/render/scene.ts`, `web/src/render/svg.ts`, et leurs tests

**Interfaces:**
- Produces: le type `Primitive` (union discriminée : `polygon`, `polyline`, `circle`, `label`, `grid`) ; `projectionFor(bounds, viewport, mirrored)` rendant `{ x(m), y(m), scale }` ; `sceneToPrimitives(scene)` ; `renderSvg(primitives, svgElement)`

- [ ] **Step 1: Écrire les tests qui échouent**

Fichier `web/src/render/projection.test.ts` :

```ts
import { describe, expect, it } from "vitest";
import { projectionFor } from "./projection";

describe("projection", () => {
  const viewport = { width: 1000, height: 600 };

  it("fits the scene inside the viewport", () => {
    const p = projectionFor({ xMin: -10, xMax: 10, yMin: -6, yMax: 6 }, viewport, false);
    expect(p.x(-10)).toBeGreaterThanOrEqual(0);
    expect(p.x(10)).toBeLessThanOrEqual(viewport.width);
    expect(p.y(-6)).toBeLessThanOrEqual(viewport.height);
    expect(p.y(6)).toBeGreaterThanOrEqual(0);
  });

  it("puts positive y upwards on screen", () => {
    // The yard is beyond the wall; on screen it must be above it.
    const p = projectionFor({ xMin: -10, xMax: 10, yMin: -6, yMax: 6 }, viewport, false);
    expect(p.y(3)).toBeLessThan(p.y(0));
  });

  it("mirrors along x when the vehicle arrives from the other side", () => {
    const plain = projectionFor({ xMin: -10, xMax: 10, yMin: -6, yMax: 6 }, viewport, false);
    const mirrored = projectionFor({ xMin: -10, xMax: 10, yMin: -6, yMax: 6 }, viewport, true);
    expect(mirrored.x(4)).toBeCloseTo(plain.x(-4), 6);
  });

  it("keeps one scale for both axes, so nothing is distorted", () => {
    const p = projectionFor({ xMin: -10, xMax: 10, yMin: -2, yMax: 2 }, viewport, false);
    const oneMetreAcross = Math.abs(p.x(1) - p.x(0));
    const oneMetreUp = Math.abs(p.y(1) - p.y(0));
    expect(oneMetreAcross).toBeCloseTo(oneMetreUp, 6);
  });
});
```

Fichier `web/src/render/scene.test.ts` : une scène coulissante produit les sept rectangles attendus plus les étiquettes ; une scène battante en produit deux de plus. Aucun test ne touche au DOM.

- [ ] **Step 2: Vérifier l'échec, puis implémenter**

Run: `npx vitest run` → FAIL, puis écrire `projection.ts`, `primitives.ts`, `scene.ts`.

La projection conserve **une seule échelle pour les deux axes** — sans quoi une cour paraîtrait plus large qu'elle n'est, ce qui est précisément le genre d'erreur que cet outil doit éviter.

- [ ] **Step 3: Le backend SVG**

`renderSvg` traduit chaque primitive en élément SVG et remplace le contenu du nœud fourni. Il ne prend aucune décision : pas de couleur choisie ici, pas de mise en forme conditionnelle. Testable par instantané sur une scène figée.

- [ ] **Step 4: Commiter**

```bash
git checkout -b feat/render-primitives-and-svg
git add web/src/render/
git commit -m "feat(web): drawing primitives and an SVG backend

Rendering happens in two steps: a pure function decides what to draw, a
backend decides how. The moteur stays replaceable — a Canvas or 3D backend
would consume the same list — and the interesting half is testable without
a browser.

One scale for both axes: a distorted plan would misrepresent exactly the
distances this tool exists to measure."
```

---

### Task 5: La trajectoire et les bandes de proximité

**Files:**
- Create: `web/src/render/path.ts` et son test

**Interfaces:**
- Consumes: `ManeuverDto` (Task 1), `Primitive` (Task 4)
- Produces: `pathToPrimitives(maneuver, vehicle)` ; `BANDS`, `bandOf(clearance)`

- [ ] **Step 1: Écrire les tests**

Les bandes reprennent le prototype : au-delà de 50 cm, 25 à 50, 10 à 25, en dessous de 10.

```ts
describe("proximity bands", () => {
  it("classifies clearances into the four bands", () => {
    expect(bandOf(0.80)).toBe(0);
    expect(bandOf(0.30)).toBe(1);
    expect(bandOf(0.15)).toBe(2);
    expect(bandOf(0.02)).toBe(3);
  });

  it("splits the path where the band changes", () => {
    const maneuver = maneuverWith([0.9, 0.9, 0.9, 0.2, 0.2]);
    const lines = pathToPrimitives(maneuver, lbx()).filter((p) => p.type === "polyline");
    expect(lines.length).toBeGreaterThanOrEqual(2);
  });

  it("splits the path where the direction changes, and marks the reversal", () => {
    const maneuver = maneuverWith([0.9, 0.9], [false, true]);
    const primitives = pathToPrimitives(maneuver, lbx());
    expect(primitives.some((p) => p.type === "circle" && p.role === "reversal")).toBe(true);
  });

  it("draws ghost vehicles along the path", () => {
    const ghosts = pathToPrimitives(longManeuver(), lbx())
      .filter((p) => p.type === "polygon" && p.role === "ghost");
    expect(ghosts).toHaveLength(4);
  });
});
```

- [ ] **Step 2: Implémenter, puis commiter**

Trait plein en marche avant, pointillé en marche arrière, cercle à chaque inversion, quatre positions fantômes le long du parcours. Les couleurs sont des jetons de thème, pas des littéraux.

```bash
git commit -m "feat(web): path rendering with proximity bands"
```

---

### Task 6: Le curseur de parcours

**Files:**
- Create: `web/src/ui/scrubber.ts` et son test

- [ ] **Step 1: Tests, puis implémentation**

Le curseur déplace le véhicule le long du parcours. Le rendu étant une fonction pure du couple (résultat, position), il suffit de régénérer la liste de primitives — mais **à la fréquence d'affichage**, via `requestAnimationFrame`, et non à chaque événement `input`.

Vérification manuelle : balayer le curseur d'un bout à l'autre. Le déplacement doit être continu ; toute saccade signale un rendu par événement plutôt que par frame.

- [ ] **Step 2: Commiter**

---

### Task 7: Les alternatives, la provenance et les indicateurs

C'est ici que le travail des lots 1a et 1b devient visible — et notamment tout ce qui distingue une réponse fiable d'une réponse plausible.

**Files:**
- Create: `web/src/ui/alternatives.ts`, `web/src/ui/verdict.ts`, et leurs tests
- Modify: `web/src/domain/labels.ts`

**Interfaces:**
- Consumes: `SolveResponse`, `ConfidenceDto` (Task 1)

- [ ] **Step 1: Écrire les tests**

```ts
describe("verdict", () => {
  it("says an exact search proves absence, and a heuristic one does not", () => {
    expect(verdictFor(empty("exact"))).toMatch(/aucune entrée n'est possible/i);
    expect(verdictFor(empty("heuristic_exhausted"))).toMatch(/ne prouve pas/i);
  });

  it("never shows a clearance without its provenance", () => {
    const rendered = verdictFor(found({ minClearance: 0.12, confidence: "heuristic" }));
    expect(rendered).toContain("12,0 cm");
    expect(rendered).toMatch(/recherche heuristique/i);
  });

  it("reports the gateway clearance separately when it differs", () => {
    const rendered = verdictFor(
      found({ minClearance: 0.001, minClearanceInGateway: 0.14 }),
    );
    // The tightest point was not at the gate; saying only "1 mm" would
    // frighten a driver about the wrong thing.
    expect(rendered).toContain("14,0 cm");
  });
});
```

- [ ] **Step 2: Implémenter**

Trois exigences, chacune issue d'un constat des lots précédents.

**Aucune marge sans sa provenance.** Le `CLAUDE.md` l'impose, `ConfidenceDto` le rend disponible : *recherche exacte* ou *recherche heuristique*. Et quand une recherche heuristique ne trouve rien, le verdict doit dire qu'elle **ne prouve pas** l'impossibilité — la nuance que le prototype affichait déjà et qu'il ne faut pas perdre.

**Deux marges, pas une.** Le lot 1b a montré que le point le plus serré peut se trouver contre une bordure, à six mètres du portail. Afficher ce chiffre seul induirait en erreur sur ce qui compte. Le verdict présente la marge dans le passage, et signale la marge globale quand elle est plus faible.

**Les alternatives sont sélectionnables**, une par nombre de manœuvres, chacune avec sa marge et son sens d'entrée. Sélectionner redessine.

- [ ] **Step 3: Indicateurs de durée d'alerte**

Distance parcourue sous 25 cm et sous 10 cm, telles que calculées côté Rust. Elles disent quelque chose que la marge minimale ne dit pas : *pendant combien de temps* c'est serré.

- [ ] **Step 4: Commiter**

```bash
git commit -m "feat(web): alternatives, provenance, and alert distances

No clearance is shown without saying where it came from, and an empty
heuristic result says plainly that it proves nothing — the distinction
between no solution found and no solution exists survives all the way to
the screen.

Two clearances rather than one: batch 1b showed the tightest point can sit
against a kerb six metres short of the gate, and reporting that number
alone would alarm a driver about the wrong thing."
```

---

### Task 8: Les véhicules et le formulaire complet

**Files:**
- Create: `web/src/domain/vehicles.ts` et son test
- Modify: `web/index.html`, `web/src/ui/form.ts`

- [ ] **Step 1: Porter les six véhicules**

Repris de `prototype/index.html:158-165`, à l'identique :

```ts
/**
 * The six vehicles the prototype shipped with.
 *
 * PROVISIONAL. Mirror width is derived rather than measured for most of
 * these, and front overhang is estimated — `data/vehicles.json` records the
 * provenance field by field and supersedes this table in batch 5. Two
 * minutes with a tape measure across the mirrors would be worth more than
 * any of it: CLAUDE.md notes that 3 cm of error there inverts a conclusion.
 */
export const VEHICLES = [
  { id: "lexus-lbx", label: "Lexus LBX", wheelbase: 2.580, length: 4.190, frontOverhang: 0.850, width: 1.825, mirrorWidth: 2.029, minTurningRadius: 5.2 },
  // … les cinq autres, valeurs inchangées
] as const;
```

Un test vérifie que chaque entrée est acceptée par le noyau — c'est-à-dire que `solve` ne les rejette pas — ce qui attrape toute faute de frappe dans les dimensions.

- [ ] **Step 2: Compléter le formulaire**

Les vingt-neuf contrôles du prototype : scène, portail battant avec son angle d'ouverture borné par `max_gate_angle`, voirie, véhicule, sens d'arrivée, rétros déployés ou rabattus. Stylés en Tailwind, éléments natifs.

L'angle d'ouverture est **borné dynamiquement** par l'appel à `max_gate_angle` : laisser saisir un angle où le vantail traverse le pilier n'aurait aucun sens.

- [ ] **Step 3: Commiter**

---

## Critères d'acceptation du lot 1

À l'issue de ce lot, les cinq critères de la spec doivent être remplis :

1. ✅ *(lot 1a/1b)* Les résultats de référence sont couverts par des tests — deux d'entre eux ayant été corrigés en chemin.
2. **Aucune interaction ne bloque le thread principal**, quelle que soit la durée du calcul. Vérifiable en faisant défiler la page pendant une recherche sur une ouverture serrée.
3. ✅ *(lot 1a/1b)* `fmt`, `clippy`, `test`, `doc` passent en CI ; s'y ajoutent `tsc --noEmit`, Vitest et le build web.
4. **L'application est déployée et accessible en production sur Vercel.**
5. ✅ *(lot 1a/1b)* Chaque constante du noyau est nommée et documentée.

---

## Ce que ce plan ne fait pas

Pas de tests bout en bout — la spec les exclut du lot 1. Pas de hauteur d'obstacles, pas d'enveloppe d'usage, pas de box de parking : lots 2 à 4. `data/vehicles.json` reste inutilisé jusqu'au lot 5.

Le correctif identifié au lot 1b — **une fonction de coût qui valorise la marge** — n'est pas ici non plus. L'interface le rendra simplement visible, en montrant où se situe réellement le point le plus serré.

**Un écart assumé avec la spec** : la progression n'est pas remontée. Le § 5 du design montre un message `Progress` circulant du Worker vers l'interface, et `swept-solver` expose le trait `Progress` prévu pour cela — mais rien ne l'implémente ici. La raison est un arbitrage : faire remonter la progression depuis le Rust demande de passer une closure JavaScript à travers `wasm-bindgen` et d'appeler `postMessage` depuis le domaine compilé, pour un bénéfice de confort. Le Worker suffit à ce que l'interface reste vivante, ce qui était l'objectif. Une recherche affiche donc « calcul en cours » sans compteur.

À reprendre si les temps de calcul le justifient — le trait est là, la place est prête, il ne manque que le pont.

Enfin, `docs/ALGORITHME.md`, que la spec plaçait en PR 17 de ce lot, a déjà été écrit au lot 1b : il devra être complété d'une section sur l'interface, pas créé.

## Vérification finale du lot 1

Une fois la Task 8 mergée, reprendre les cinq critères d'acceptation de la spec un par un, et **exécuter les vérifications plutôt que les supposer** — en particulier la deuxième, qui ne se prouve qu'en faisant défiler la page pendant un calcul long. Un critère coché sans preuve ne vaut rien.
