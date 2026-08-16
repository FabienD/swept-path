/**
 * The documentation's figures, drawn by the renderer that draws the plan.
 *
 * Not hand-written SVG. A figure explaining that a vehicle at 15° takes more
 * room than its own width is worth nothing if it is a drawing of that claim
 * rather than a consequence of it — and a hand-drawn one would go on looking
 * convincing long after the code stopped agreeing with it.
 *
 * These need no solver and no Wasm: a scene, a pose, and the same pure
 * functions the plan uses. That is what keeps the documentation page free of
 * the planner it documents.
 */
import type { SceneDto, VehicleDto } from "../domain/types";
import type { Primitive } from "./primitives";
import { vehicleAt } from "./path";
import { projectionFor } from "./projection";
import { boundsFor, sceneToPrimitives } from "./scene";
import { renderSvg } from "./svg";
import type { Preferences } from "../i18n/preferences";

/** A gateway of the given opening, otherwise the reference geometry. */
export function gateway(opening: number): SceneDto {
  return {
    left_post: { inner_edge_x: -opening / 2, width: 0.55, depth: 0.55 },
    right_post: { inner_edge_x: opening / 2, width: 0.55, depth: 0.55 },
    wall_thickness: 0.3,
    sidewalk_width: 1.3,
    curb_cut_width: opening + 0.8,
    road_width: 5.9,
    curb_height: 0.12,
    gate: { kind: "sliding" },
  };
}

/** The reference vehicle: a Lexus LBX, as the tests use it. */
export const LBX: VehicleDto = {
  wheelbase: 2.58,
  length: 4.19,
  front_overhang: 0.85,
  width: 1.825,
  mirror_width: 2.029,
  ground_clearance: 0.18,
  min_turning_radius: 5.2,
};

/** Draws a scene and a vehicle at one pose into an existing `<svg>`. */
function paint(
  svg: SVGSVGElement,
  scene: SceneDto,
  extra: Primitive[],
  preferences: Preferences,
): void {
  const viewport = { width: 900, height: 520 };
  const projection = projectionFor(boundsFor(scene), viewport, false);
  svg.setAttribute("viewBox", `0 0 ${viewport.width} ${viewport.height}`);
  renderSvg([...sceneToPrimitives(scene, preferences), ...extra], svg, projection);
}

/**
 * The vehicle square in the opening: what the clearance ceiling looks like.
 *
 * Placed at the wall line, centred, pointing into the yard — the pose every
 * successful entry passes through, and the one the ceiling is computed for.
 */
export function squareInTheGateway(
  svg: SVGSVGElement,
  opening: number,
  preferences: Preferences,
): void {
  const scene = gateway(opening);
  const pose = {
    x: 0,
    y: 0.3,
    heading: Math.PI / 2,
    reverse: false,
    clearance: 1,
    overhanging: false,
  };
  paint(svg, scene, vehicleAt(pose, LBX, "vehicle"), preferences);
}

/**
 * The same vehicle, square and then askew.
 *
 * The point of the figure is that the second one is wider *in the opening*
 * without being wider — which is why both are drawn on the same gateway
 * rather than described.
 */
export function squareAgainstAskew(
  svg: SVGSVGElement,
  opening: number,
  degrees: number,
  preferences: Preferences,
): void {
  const scene = gateway(opening);
  const at = (heading: number, role: "vehicle" | "ghost") =>
    vehicleAt(
      {
        x: 0,
        y: 0.3,
        heading,
        reverse: false,
        clearance: 1,
        overhanging: false,
      },
      LBX,
      role,
    );
  paint(
    svg,
    scene,
    [
      ...at(Math.PI / 2, "ghost"),
      ...at(Math.PI / 2 - (degrees * Math.PI) / 180, "vehicle"),
    ],
    preferences,
  );
}

/** How much of the opening a vehicle takes at `degrees` off square, in metres. */
export function spanTaken(
  width: number,
  depth: number,
  degrees: number,
): number {
  const alpha = (degrees * Math.PI) / 180;
  return width / Math.cos(alpha) + depth * Math.tan(alpha);
}
