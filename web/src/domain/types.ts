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
  sidewalk_width: number;
  curb_cut_width: number;
  road_width: number;
  /** Curb height, in metres. Infinite for a curb nothing passes over. */
  curb_height: number;
  gate: GateDto;
}

export interface VehicleDto {
  wheelbase: number;
  length: number;
  front_overhang: number;
  width: number;
  mirror_width: number;
  /** Lowest point of the bodywork, wheels excluded, in metres. */
  ground_clearance: number;
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
  /** True when part of the body sits over a low obstacle at this pose. */
  overhanging: boolean;
}

export type ConfidenceDto = "exact" | "heuristic" | "heuristic_exhausted";

export interface ManeuverDto {
  poses: PoseDto[];
  min_clearance: number;
  /** Tightest point within the gateway, which is what a driver asked about. */
  min_clearance_in_gateway: number;
  /** Distance travelled with part of the body over a low obstacle, in metres. */
  metres_overhanging: number;
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
  /**
   * Sent while the planner expands nodes, at most once per 500 of them.
   *
   * The exhaustive sweep runs first and sends nothing, so the first of these
   * is what tells the interface the sweep is over and planning has begun.
   */
  | {
      kind: "progress";
      id: number;
      moves: number;
      expanded: number;
      /** Node ceiling the planner stops at, so the interface can say where it ends. */
      budget: number;
    }
  | { kind: "minRoad"; id: number; response: number | null }
  | { kind: "maxGateAngle"; id: number; radians: number }
  | { kind: "failed"; id: number; error: ErrorDto };
