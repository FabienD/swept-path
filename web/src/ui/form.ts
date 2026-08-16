import type { SceneDto, SolveRequest, VehicleDto } from "../domain/types";
import type { Magnitude, UnitSystem } from "../domain/units";
import { fromDisplay } from "../domain/units";

/**
 * Reads a numeric input as metres, or throws if the control is missing.
 *
 * The field holds what the reader typed, in the unit they were shown. The
 * core only ever speaks metres, so the conversion happens here, at the one
 * boundary every measurement crosses — a field read straight would send 90.2
 * *metres* to the solver the moment someone switched to inches.
 *
 * Which unit a field is in is declared by the markup, in `data-magnitude`.
 * A field without it carries no length — an angle, a percentage — and is
 * passed through untouched.
 */
function num(id: string, units: UnitSystem): number {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`missing input: ${id}`);
  }
  const magnitude = element.dataset["magnitude"] as Magnitude | undefined;
  if (!magnitude) return element.valueAsNumber;
  return fromDisplay(element.valueAsNumber, magnitude, units);
}

/** Reads a checkbox, or throws if the control is missing. */
function ticked(id: string): boolean {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`missing checkbox: ${id}`);
  }
  return element.checked;
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
export function readScene(units: UnitSystem): SceneDto {
  const opening = num("opening", units);
  const postWidth = num("post-width", units);
  const postDepth = num("post-depth", units);

  return {
    left_post: { inner_edge_x: -opening / 2, width: postWidth, depth: postDepth },
    right_post: { inner_edge_x: opening / 2, width: postWidth, depth: postDepth },
    wall_thickness: num("wall", units),
    sidewalk_width: num("sidewalk", units),
    curb_cut_width: num("curb", units),
    road_width: num("road", units),
    curb_height: num("curb-height", units),
    gate:
      choice("gate-kind") === "swinging"
        ? {
            kind: "swinging",
            leaf_length: num("leaf-length", units),
            leaf_thickness: num("leaf-thickness", units),
            hinge_offset: num("hinge-offset", units),
            hinge_depth_ratio: num("hinge-depth", units) / 100,
            open_angle: (num("open-angle", units) * Math.PI) / 180,
          }
        : { kind: "sliding" },
  };
}

/** Builds the vehicle from the form, honouring the mirror setting. */
export function readVehicle(units: UnitSystem): VehicleDto {
  const folded = ticked("mirrors-folded");
  return {
    wheelbase: num("wheelbase", units),
    length: num("length", units),
    front_overhang: num("front-overhang", units),
    width: num("body-width", units),
    mirror_width: folded ? num("mirror-width-folded", units) : num("mirror-width", units),
    ground_clearance: num("ground-clearance", units),
    min_turning_radius: num("radius", units),
  };
}

/** The whole request. */
export function readRequest(units: UnitSystem): SolveRequest {
  const direction = choice("direction");
  return {
    scene: readScene(units),
    vehicle: readVehicle(units),
    forward_only:
      direction === "forward" ? true : direction === "reverse" ? false : null,
  };
}
