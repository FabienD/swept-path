import { bandOf } from "../domain/bands";
import { poseAt, timelineOf } from "./playback";
import type { ManeuverDto, PoseDto, VehicleDto } from "../domain/types";
import type { Point, Primitive, Role } from "./primitives";
import { trackOf } from "../domain/track";

const BAND_ROLES: readonly Role[] = [
  "band-clear",
  "band-watch",
  "band-close",
  "band-tight",
];

/**
 * How many ghost vehicles the playback leaves behind.
 *
 * ARBITRARY. Enough that what remains at the end reads as a swept envelope
 * rather than as scattered outlines, few enough that they do not fill the
 * gateway they are meant to show the vehicle passing through.
 */
const GHOSTS = 6;

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

/**
 * The curvature of the path at pose `index`, in 1/m.
 *
 * Signed the way the kinematic model signs it, so that a positive value is
 * the wheels turned left whichever gear the stretch was driven in.
 */
export function curvatureAt(poses: readonly PoseDto[], index: number): number {
  // The segment arriving at the pose, or — on the first one, which none
  // arrives at — the one leaving it. Either says where the wheels point.
  const across = (from?: PoseDto, to?: PoseDto): number | null => {
    if (!from || !to) return null;
    // A segment is a circular arc, whose chord is `2R·sin(Δθ/2)`. Reading the
    // chord as the arc would inflate the curvature — a tenth of a degree of
    // steering on a 20 cm step, which is visible on a drawn wheel.
    //
    // Signed by the gear: `ds < 0` in reverse, so the same steering turns the
    // heading the other way. Dividing by the signed step gives the wheels back.
    const chord = Math.hypot(to.x - from.x, to.y - from.y);
    // A gear change can emit the same pose twice. Dividing by that chord puts
    // an infinity into a coordinate and blanks the whole plan.
    if (chord < 1e-9) return null;
    const sign = to.reverse ? -1 : 1;
    return (2 * Math.sin((to.heading - from.heading) / 2)) / (chord * sign);
  };

  return (
    across(poses[index - 1], poses[index]) ??
    across(poses[index], poses[index + 1]) ??
    0
  );
}

/** How wide a drawn tyre is, as a fraction of its length. */
const TYRE_ASPECT = 0.35;

/** The steering angle of the wheel offset `ly` from the axle centre. */
function steerAt(vehicle: VehicleDto, curvature: number, ly: number): number {
  // The turn centre sits `1/κ` to the left, so a wheel offset towards it sees
  // a shorter lever and turns further — Ackermann, and the reason a pair drawn
  // parallel reads as a toy. Written over `κ` rather than over the radius, so
  // a straight stretch needs no special case.
  return Math.atan((vehicle.wheelbase * curvature) / (1 - curvature * ly));
}

/**
 * How long a drawn tyre may be, in metres.
 *
 * Sized off the wheelbase, so a quadricycle does not get the wheels of an
 * estate car — then cut down to whatever fits. A steered wheel swings its
 * corner sideways, and the plan must never show any part of the vehicle
 * outside the envelope the solver actually tested: the widest tested point is
 * the mirror, and at full lock a life-sized wheel would reach past it by
 * several centimetres. Fitted here once per vehicle rather than per pose, so
 * the wheels do not breathe as the car turns.
 */
function tyreLength(vehicle: VehicleDto): number {
  const halfTrack = trackOf(vehicle.width) / 2;
  const lock = Math.abs(
    steerAt(vehicle, 1 / vehicle.min_turning_radius, halfTrack),
  );
  const room = vehicle.mirror_width / 2 - halfTrack;
  const reach = Math.sin(lock) + TYRE_ASPECT * Math.cos(lock);
  return Math.min(vehicle.wheelbase * 0.24, (2 * room) / reach);
}

/** A tyre footprint, centred at `(lx, ly)` and turned by `steer` radians. */
function tyreAt(
  pose: PoseDto,
  vehicle: VehicleDto,
  lx: number,
  ly: number,
  steer: number,
  role: Role,
): Primitive {
  const half = tyreLength(vehicle) / 2;
  const halfWidth = half * TYRE_ASPECT;
  const cos = Math.cos(steer);
  const sin = Math.sin(steer);
  return {
    type: "polygon",
    role,
    points: (
      [
        [-half, -halfWidth],
        [half, -halfWidth],
        [half, halfWidth],
        [-half, halfWidth],
      ] as const
    ).map(([px, py]) =>
      localToWorld(pose, lx + px * cos - py * sin, ly + px * sin + py * cos),
    ),
  };
}

/**
 * The vehicle outline, its wheels and its mirrors, at one pose.
 *
 * `curvature` steers the front wheels. Ackermann, not a single angle: the
 * wheel on the inside of the bend follows a tighter circle, so it turns
 * further, and a pair drawn parallel would read as a toy.
 */
export function vehicleAt(
  pose: PoseDto,
  vehicle: VehicleDto,
  role: "vehicle" | "ghost",
  curvature = 0,
): Primitive[] {
  const halfMirrors = vehicle.mirror_width / 2;
  const out: Primitive[] = [
    { type: "polygon", role, points: bodyAt(pose, vehicle) },
  ];

  // The nose, drawn inside the rectangle rather than bitten out of it: a
  // chamfered polygon would show a car smaller than the one tested. Two lines,
  // which is cheap enough that the ghosts can afford them — and a trail of
  // outlines that says nothing about which way it was travelling is half a
  // drawing.
  const front = vehicle.wheelbase + vehicle.front_overhang;
  const halfBody = vehicle.width / 2;
  const cut = vehicle.width / 6;
  for (const side of [1, -1]) {
    out.push({
      type: "polyline",
      role: role === "vehicle" ? "chamfer" : "ghost",
      points: [
        localToWorld(pose, front - cut, side * halfBody),
        localToWorld(pose, front, side * (halfBody - cut)),
      ],
    });
  }
  if (role === "vehicle") {
    // The mirrors, at the size they are rather than as a marker. They are the
    // widest thing on the car and almost always the first to touch a pillar,
    // and `swept_core::vehicle::envelope` samples them: a silhouette stopping
    // at the body would draw a narrower car than the one that was answered
    // about. Level with the front axle, which is where the core puts them.
    const beyondBody = halfMirrors - halfBody;
    // Half the housing, fore and aft of the axle. A mirror is about a hand
    // long whatever the car; scaled off the wheelbase so it stays in
    // proportion rather than dominating a small one.
    const housing = vehicle.wheelbase * 0.04;
    for (const side of [1, -1]) {
      out.push({
        type: "polygon",
        role: "mirror",
        points: (
          [
            [-housing, halfBody],
            [housing, halfBody],
            [housing, halfBody + beyondBody],
            [-housing, halfBody + beyondBody],
          ] as const
        ).map(([lx, ly]) =>
          localToWorld(pose, vehicle.wheelbase + lx, side * ly),
        ),
      });
    }

    const halfTrack = trackOf(vehicle.width) / 2;
    for (const side of [halfTrack, -halfTrack]) {
      out.push(tyreAt(pose, vehicle, 0, side, 0, "wheel-rear"));
    }
    for (const side of [halfTrack, -halfTrack]) {
      const steer = steerAt(vehicle, curvature, side);
      out.push(tyreAt(pose, vehicle, vehicle.wheelbase, side, steer, "wheel-front"));
    }

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

  // Distance, not pose index: see `playback.ts` for why the two differ.
  const timeline = timelineOf(poses);
  const reached = poseAt(timeline, position);

  const out: Primitive[] = [];

  // Where the trip goes, under everything else. Drawn whole and in one piece:
  // a trace that stopped at the vehicle would leave a paused playback saying
  // nothing about where it was heading, and this line is not split by band
  // because it carries no clearance information — only a destination.
  out.push({
    type: "polyline",
    role: "upcoming",
    points: poses.map((p) => ({ x: p.x, y: p.y })),
  });

  // The overhang joins the key, which splits the path where it begins.
  const keyOf = (pose: PoseDto) =>
    `${pose.reverse}|${bandOf(pose.clearance)}|${pose.overhanging}`;

  // Only as far as the vehicle has got. The coloured stretches carry the
  // proximity bands, and a red one drawn ahead of the vehicle would report a
  // danger that has not happened yet.
  let start = 0;
  while (start < reached) {
    const key = keyOf(poses[start + 1]!);
    let end = start + 1;
    while (end < reached && keyOf(poses[end + 1]!) === key) end++;

    const last = poses[end]!;
    out.push({
      type: "polyline",
      role: last.overhanging ? "overhang" : BAND_ROLES[bandOf(last.clearance)]!,
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

  // Ghosts at fixed points along the trip, revealed as they are passed, so
  // that what remains once playback ends is the épure itself — built by the
  // animation rather than drawn beside it.
  for (let i = 0; i < GHOSTS; i++) {
    const fraction = i / (GHOSTS - 1);
    if (fraction > position) break;
    out.push(...vehicleAt(poses[poseAt(timeline, fraction)]!, vehicle, "ghost"));
  }

  out.push(
    ...vehicleAt(poses[reached]!, vehicle, "vehicle", curvatureAt(poses, reached)),
  );

  return out;
}
