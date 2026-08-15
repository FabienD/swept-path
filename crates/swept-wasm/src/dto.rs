//! Types crossing the WebAssembly boundary.
//!
//! These exist so that the domain never has to know about serialisation.
//! Nothing here decides anything: it converts, delegates validation, and
//! turns domain errors into codes the interface can translate.

use serde::{Deserialize, Serialize};
use swept_core::clearance::{Clearance, ClearanceField};
use swept_core::kinematics::Direction;
use swept_core::scene::{GateKind, Post, Scene};
use swept_core::units::Radians;
use swept_core::vehicle::{Vehicle, VehicleError};
use swept_solver::budget::{SearchBudget, Silent};
use swept_solver::result::{Confidence, Maneuver, Outcome};
use swept_solver::solve::alternatives;

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
            // Placeholder until Task 6 carries the field across the boundary.
            0.18,
            self.min_turning_radius,
        )
        .map_err(|e| match e {
            VehicleError::NonPositive(field) => ErrorDto {
                code: String::from("non_positive"),
                field: Some(field.to_owned()),
            },
            VehicleError::FrontOverhangTooLarge => ErrorDto {
                code: String::from("front_overhang_too_large"),
                field: Some(String::from("front_overhang")),
            },
            VehicleError::MirrorsNarrowerThanBody => ErrorDto {
                code: String::from("mirrors_narrower_than_body"),
                field: Some(String::from("mirror_width")),
            },
            VehicleError::ImplausibleTurningRadius => ErrorDto {
                code: String::from("implausible_turning_radius"),
                field: Some(String::from("min_turning_radius")),
            },
        })
    }
}

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
    /// of the gate does not mean the same thing to a driver as grazing a post
    /// — and batch 1b showed the planner does exactly that.
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

/// How deep the constrained corridor runs, in metres of `y`.
///
/// Everything between the outer face of the wall and the far side of the
/// posts, plus the leaves when they swing into the way.
fn corridor_depth(scene: &Scene) -> f64 {
    let gate = match scene.gate {
        GateKind::Sliding => 0.0,
        GateKind::Swinging { leaf_length, .. } => leaf_length,
    };
    scene.left_post.depth.max(scene.right_post.depth) + gate
}

/// Distances, in metres, below which a stretch counts as an alert.
const ALERT_BANDS_M: [f64; 2] = [0.25, 0.10];

/// Annotates a manoeuvre with everything the interface needs to draw it.
fn describe(
    maneuver: &Maneuver,
    vehicle: &Vehicle,
    field: &ClearanceField,
    corridor: f64,
) -> ManeuverDto {
    let poses: Vec<PoseDto> = maneuver
        .poses
        .iter()
        .map(|step| PoseDto {
            x: step.pose.x,
            y: step.pose.y,
            heading: step.pose.heading.get(),
            reverse: step.direction == Direction::Reverse,
            clearance: match field.at(step.pose) {
                Clearance::Clear(margin) => margin,
                Clearance::Collision => 0.0,
            },
        })
        .collect();

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

    // Which poses count as "in the gateway" is decided on the *vehicle*, not
    // on the pose. A pose is the rear axle; what threads the opening is the
    // nose, up to a wheelbase and an overhang ahead of it, and the mirrors
    // with it. Filtering on the axle alone got this backwards twice over: it
    // dropped the moment the nose passes between the posts — the tightest of
    // the whole entry — and it kept poses where the axle has reached the
    // corridor but the vehicle is already out in the yard, which is roomy.
    //
    // The figure that came out could therefore exceed `(W - w) / 2`, the room
    // the opening physically has, and it is the figure the verdict leads with.
    let ahead = vehicle.wheelbase + vehicle.front_overhang;
    let in_gateway = poses
        .iter()
        .filter(|p| p.y >= -ahead && p.y <= corridor + vehicle.rear_overhang)
        .map(|p| p.clearance)
        .fold(f64::INFINITY, f64::min);

    ManeuverDto {
        poses,
        min_clearance: maneuver.min_clearance,
        // A path that never crosses the corridor has no gateway clearance;
        // report the overall figure rather than infinity.
        min_clearance_in_gateway: if in_gateway.is_finite() {
            in_gateway
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

    Ok(match outcome {
        Outcome::NotFound { budget_exhausted } => SolveResponse {
            alternatives: Vec::new(),
            budget_exhausted,
        },
        Outcome::Found(list) => SolveResponse {
            alternatives: list
                .iter()
                .map(|m| describe(m, &vehicle, &field, corridor))
                .collect(),
            budget_exhausted: false,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scene_dto() -> SceneDto {
        SceneDto {
            left_post: PostDto {
                inner_edge_x: -1.20,
                width: 0.55,
                depth: 0.55,
            },
            right_post: PostDto {
                inner_edge_x: 1.20,
                width: 0.55,
                depth: 0.55,
            },
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
        assert_eq!(err.field.as_deref(), Some("mirror_width"));
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
            GateKind::Swinging { open_angle, .. } => {
                assert!((open_angle.to_degrees() - 90.0).abs() < 1e-9);
            }
            GateKind::Sliding => panic!("expected a swinging gate"),
        }
    }

    #[test]
    fn a_solve_crosses_back_with_everything_the_interface_needs() {
        let response = run_solve(SolveRequest {
            scene: SceneDto {
                left_post: PostDto {
                    inner_edge_x: -2.5,
                    width: 0.55,
                    depth: 0.55,
                },
                right_post: PostDto {
                    inner_edge_x: 2.5,
                    width: 0.55,
                    depth: 0.55,
                },
                dropped_kerb_width: 5.8,
                ..scene_dto()
            },
            vehicle: vehicle_dto(),
            forward_only: Some(true),
        })
        .expect("valid request");

        let first = response.alternatives.first().expect("5 m admits an entry");
        assert!(!first.poses.is_empty());
        assert!(first.distance > 0.0);
        assert!(first.min_clearance > 0.0);
        assert_eq!(first.confidence, ConfidenceDto::Exact);
    }

    #[test]
    fn the_gateway_clearance_ignores_what_happens_out_on_the_road() {
        // Batch 1b found the tightest point can sit against a kerb metres
        // short of the gate. The two figures must then differ.
        let response = run_solve(SolveRequest {
            scene: scene_dto(),
            vehicle: vehicle_dto(),
            forward_only: Some(true),
        })
        .expect("valid request");

        for m in &response.alternatives {
            assert!(
                m.min_clearance_in_gateway >= m.min_clearance - 1e-12,
                "the gateway is part of the path, so it cannot be tighter than the whole"
            );
        }
    }

    #[test]
    fn alert_distances_never_exceed_the_distance_travelled() {
        let response = run_solve(SolveRequest {
            scene: scene_dto(),
            vehicle: vehicle_dto(),
            forward_only: Some(true),
        })
        .expect("valid request");

        for m in &response.alternatives {
            assert!(m.metres_under_25cm <= m.distance + 1e-9);
            assert!(m.metres_under_10cm <= m.metres_under_25cm + 1e-9);
        }
    }

    /// The clearance the verdict leads with can never exceed the room the
    /// opening physically has.
    ///
    /// `(W - w) / 2` is the ceiling whatever the path, and the gateway figure
    /// used to sail past it: it selected poses by the rear axle, so it dropped
    /// the moment the nose threads the posts and kept the roomy moments after
    /// the whole vehicle is through. A number above the ceiling is not good
    /// news, it is a number answering a different question.
    #[test]
    fn the_gateway_clearance_never_exceeds_what_the_opening_holds() {
        let mut scene = scene_dto();
        scene.left_post.inner_edge_x = -2.29 / 2.0;
        scene.right_post.inner_edge_x = 2.29 / 2.0;
        scene.pavement_width = 1.30;
        scene.road_width = 5.90;
        scene.gate = GateDto::Swinging {
            leaf_length: 1.15,
            leaf_thickness: 0.04,
            hinge_offset: 0.035,
            hinge_depth_ratio: 0.5,
            open_angle: std::f64::consts::PI * 118.0 / 180.0,
        };
        let mut vehicle = vehicle_dto();
        vehicle.min_turning_radius = 3.59;

        let response = run_solve(SolveRequest {
            scene,
            vehicle,
            forward_only: None,
        })
        .expect("valid dimensions");

        let opening = scene.right_post.inner_edge_x - scene.left_post.inner_edge_x;
        let ceiling = (opening - vehicle.mirror_width) / 2.0;
        for alternative in &response.alternatives {
            assert!(
                alternative.min_clearance_in_gateway <= ceiling + 1e-9,
                "{} moves: {:.1} cm in a gateway that holds {:.1} cm",
                alternative.moves,
                alternative.min_clearance_in_gateway * 100.0,
                ceiling * 100.0
            );
        }
    }
}
