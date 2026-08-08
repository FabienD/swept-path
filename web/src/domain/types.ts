/**
 * The TypeScript mirror of the Rust DTOs.
 *
 * This is the one place in the system where the same object is defined twice,
 * and therefore the one place the two can drift. `serde` treats a missing
 * field as `undefined` without complaining, so a divergence here fails
 * silently — hence the round-trip test in `types.test.ts`.
 */

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
  /** Tightest point within the gateway, which is what a driver asked about. */
  min_clearance_in_gateway: number;
  metres_under_25cm: number;
  metres_under_10cm: number;
  distance: number;
  moves: number;
  confidence: ConfidenceDto;
}

export interface SolveResponse {
  alternatives: ManeuverDto[];
  /** True when the search stopped on its budget rather than exhausting the
   *  space — an empty result is then not proof of anything. */
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
