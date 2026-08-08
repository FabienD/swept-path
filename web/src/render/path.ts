import type { ManeuverDto, PoseDto, VehicleDto } from "../domain/types";
import type { Point, Primitive, Role } from "./primitives";

/**
 * Clearance thresholds separating the proximity bands, in metres.
 *
 * Carried over from the prototype (`index.html:604`): beyond 50 cm, 25 to 50,
 * 10 to 25, under 10. Reading a path by colour is what tells a driver *where*
 * it gets tight, which a single minimum never says.
 */
export const BANDS = [0.5, 0.25, 0.1] as const;

const BAND_ROLES: readonly Role[] = [
  "band-clear",
  "band-watch",
  "band-close",
  "band-tight",
];

/** How many ghost vehicles are drawn along the path. */
const GHOSTS = 4;

/** Which band a clearance falls into, 0 being the roomiest. */
export function bandOf(clearance: number): number {
  if (clearance >= BANDS[0]) return 0;
  if (clearance >= BANDS[1]) return 1;
  if (clearance >= BANDS[2]) return 2;
  return 3;
}

/** The four corners of the body at a given pose, in world coordinates. */
export function bodyAt(pose: PoseDto, vehicle: VehicleDto): Point[] {
  const cos = Math.cos(pose.heading);
  const sin = Math.sin(pose.heading);
  const halfWidth = vehicle.width / 2;
  const rear = -(vehicle.length - vehicle.wheelbase - vehicle.front_overhang);
  const front = vehicle.wheelbase + vehicle.front_overhang;

  return (
    [
      [rear, -halfWidth],
      [front, -halfWidth],
      [front, halfWidth],
      [rear, halfWidth],
    ] as const
  ).map(([lx, ly]) => ({
    x: pose.x + lx * cos - ly * sin,
    y: pose.y + lx * sin + ly * cos,
  }));
}

/** Where a point sitting at `(lx, ly)` in vehicle coordinates ends up. */
function localToWorld(pose: PoseDto, lx: number, ly: number): Point {
  const cos = Math.cos(pose.heading);
  const sin = Math.sin(pose.heading);
  return { x: pose.x + lx * cos - ly * sin, y: pose.y + lx * sin + ly * cos };
}

/** The vehicle outline, its mirrors and its nose, at one pose. */
export function vehicleAt(
  pose: PoseDto,
  vehicle: VehicleDto,
  role: "vehicle" | "ghost",
): Primitive[] {
  const halfMirrors = vehicle.mirror_width / 2;
  const out: Primitive[] = [
    { type: "polygon", role, points: bodyAt(pose, vehicle) },
  ];
  if (role === "vehicle") {
    // The mirrors are almost always what touches first, so they are marked.
    for (const side of [halfMirrors, -halfMirrors]) {
      out.push({
        type: "circle",
        role: "mirror",
        centre: localToWorld(pose, vehicle.wheelbase, side),
        radius: 3,
      });
    }
    out.push({
      type: "circle",
      role: "nose",
      centre: localToWorld(pose, vehicle.wheelbase + vehicle.front_overhang, 0),
      radius: 2.5,
    });
  }
  return out;
}

/**
 * Everything a manoeuvre draws.
 *
 * The path is cut wherever the proximity band or the direction of travel
 * changes, so each stretch carries one colour and one line style. Reversals
 * are marked with a circle — that is where a driver has to stop and change
 * gear, which the path shape alone does not show.
 *
 * `position` selects which pose carries the solid vehicle, from 0 to 1.
 */
export function pathToPrimitives(
  maneuver: ManeuverDto,
  vehicle: VehicleDto,
  position = 1,
): Primitive[] {
  const poses = maneuver.poses;
  if (poses.length === 0) return [];

  const out: Primitive[] = [];
  const keyOf = (pose: PoseDto) => `${pose.reverse}|${bandOf(pose.clearance)}`;

  let start = 0;
  while (start < poses.length - 1) {
    const key = keyOf(poses[start + 1]!);
    let end = start + 1;
    while (end < poses.length - 1 && keyOf(poses[end + 1]!) === key) end++;

    const last = poses[end]!;
    out.push({
      type: "polyline",
      role: BAND_ROLES[bandOf(last.clearance)]!,
      dashed: last.reverse,
      points: poses.slice(start, end + 1).map((p) => ({ x: p.x, y: p.y })),
    });

    const next = poses[end + 1];
    if (next && next.reverse !== last.reverse) {
      out.push({
        type: "circle",
        role: "reversal",
        centre: { x: last.x, y: last.y },
        radius: 4.5,
      });
    }
    start = end;
  }

  // Ghosts along the way, then the vehicle at the requested position.
  for (let i = 0; i < GHOSTS; i++) {
    const at = Math.round((i / (GHOSTS - 1)) * (poses.length - 1));
    out.push(...vehicleAt(poses[at]!, vehicle, "ghost"));
  }
  const index = Math.round(Math.min(Math.max(position, 0), 1) * (poses.length - 1));
  out.push(...vehicleAt(poses[index]!, vehicle, "vehicle"));

  return out;
}
