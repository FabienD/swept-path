import type { SceneDto, SolveRequest, VehicleDto } from "../domain/types";

/** Reads a numeric input, or throws if the control is missing. */
function num(id: string): number {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`missing input: ${id}`);
  }
  return element.valueAsNumber;
}

/** Reads a select, or throws if the control is missing. */
function choice(id: string): string {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLSelectElement)) {
    throw new Error(`missing select: ${id}`);
  }
  return element.value;
}

/** Whether the vehicle arrives from the right, which mirrors the scene. */
export function arrivesFromTheRight(): boolean {
  return choice("side") === "-1";
}

/** Builds the scene from the form. */
export function readScene(): SceneDto {
  const opening = num("opening");
  const postWidth = num("post-width");
  const postDepth = num("post-depth");

  return {
    left_post: { inner_edge_x: -opening / 2, width: postWidth, depth: postDepth },
    right_post: { inner_edge_x: opening / 2, width: postWidth, depth: postDepth },
    wall_thickness: num("wall"),
    pavement_width: num("pavement"),
    dropped_kerb_width: num("kerb"),
    road_width: num("road"),
    kerb_height: num("kerb-height"),
    gate:
      choice("gate-kind") === "swinging"
        ? {
            kind: "swinging",
            leaf_length: num("leaf-length"),
            leaf_thickness: num("leaf-thickness"),
            hinge_offset: num("hinge-offset"),
            hinge_depth_ratio: num("hinge-depth") / 100,
            open_angle: (num("open-angle") * Math.PI) / 180,
          }
        : { kind: "sliding" },
  };
}

/** Builds the vehicle from the form, honouring the mirror setting. */
export function readVehicle(): VehicleDto {
  const folded = choice("mirrors-state") === "folded";
  return {
    wheelbase: num("wheelbase"),
    length: num("length"),
    front_overhang: num("front-overhang"),
    width: num("body-width"),
    mirror_width: folded ? num("mirror-width-folded") : num("mirror-width"),
    ground_clearance: num("ground-clearance"),
    min_turning_radius: num("radius"),
  };
}

/** The whole request. */
export function readRequest(): SolveRequest {
  const direction = choice("direction");
  return {
    scene: readScene(),
    vehicle: readVehicle(),
    forward_only:
      direction === "forward" ? true : direction === "reverse" ? false : null,
  };
}
