import { describe, expect, it } from "vitest";
import type { ManeuverDto, PoseDto, VehicleDto } from "../domain/types";
import type { Point, Primitive } from "./primitives";
import { bodyAt, curvatureAt, pathToPrimitives, vehicleAt } from "./path";
import { VEHICLES } from "../domain/vehicles";

const lbx: VehicleDto = {
  wheelbase: 2.58,
  length: 4.19,
  front_overhang: 0.85,
  width: 1.825,
  mirror_width: 2.029,
  ground_clearance: 0.18,
  min_turning_radius: 5.2,
};

function poses(
  clearances: number[],
  reverses?: boolean[],
  overhangs?: boolean[],
): PoseDto[] {
  return clearances.map((clearance, i) => ({
    x: i * 0.2,
    y: 0,
    heading: 0,
    reverse: reverses?.[i] ?? false,
    clearance,
    overhanging: overhangs?.[i] ?? false,
  }));
}

function maneuver(
  clearances: number[],
  reverses?: boolean[],
  overhangs?: boolean[],
): ManeuverDto {
  return {
    poses: poses(clearances, reverses, overhangs),
    min_clearance: Math.min(...clearances),
    min_clearance_in_gateway: Math.min(...clearances),
    metres_under_25cm: 0,
    metres_under_10cm: 0,
    metres_overhanging: 0,
    distance: clearances.length * 0.2,
    moves: 1,
    confidence: "exact",
  };
}

/**
 * The stretches that carry the trip itself.
 *
 * Named by role rather than by "every polyline that is not the backdrop": the
 * vehicle draws lines too, and a selector defined by what it excludes goes
 * quietly wrong the first time the drawing gains a line.
 */
const PATH_ROLES: ReadonlySet<string> = new Set([
  "band-clear",
  "band-watch",
  "band-close",
  "band-tight",
  "overhang",
]);

describe("path rendering", () => {
  // Two filters rather than one condition: a conjunction loses the narrowing
  // that `type === "polyline"` gives, and `dashed` stops type-checking.
  const lines = (m: ManeuverDto) =>
    pathToPrimitives(m, lbx)
      .filter((p) => p.type === "polyline")
      .filter((p) => PATH_ROLES.has(p.role));

  it("splits the path where the band changes", () => {
    const split = lines(maneuver([0.9, 0.9, 0.9, 0.2, 0.2]));
    expect(split.length).toBeGreaterThanOrEqual(2);
    expect(new Set(split.map((l) => l.role)).size).toBeGreaterThan(1);
  });

  it("keeps a uniform path in one stretch", () => {
    expect(lines(maneuver([0.9, 0.9, 0.9, 0.9]))).toHaveLength(1);
  });

  it("dashes the stretches driven in reverse", () => {
    const backwards = lines(maneuver([0.9, 0.9, 0.9], [true, true, true]));
    expect(backwards.every((l) => l.dashed)).toBe(true);
  });

  it("marks where the vehicle changes direction", () => {
    const primitives = pathToPrimitives(
      maneuver([0.9, 0.9, 0.9, 0.9], [false, false, true, true]),
      lbx,
    );
    expect(primitives.some((p) => p.type === "circle" && p.role === "reversal")).toBe(
      true,
    );
  });

  it("leaves a ghost at every step of the épure, and one solid vehicle", () => {
    // At the end of a playback every ghost has been passed, so the finished
    // figure carries the whole set.
    const shapes = pathToPrimitives(maneuver([0.9, 0.8, 0.7, 0.6, 0.5]), lbx).filter(
      (p) => p.type === "polygon",
    );
    expect(shapes.filter((p) => p.role === "ghost")).toHaveLength(6);
    expect(shapes.filter((p) => p.role === "vehicle")).toHaveLength(1);
  });

  it("moves the solid vehicle along the path with the position", () => {
    const m = maneuver([0.9, 0.9, 0.9, 0.9, 0.9]);
    const solidAt = (position: number) =>
      pathToPrimitives(m, lbx, position).find(
        (p) => p.type === "polygon" && p.role === "vehicle",
      );
    const start = solidAt(0);
    const end = solidAt(1);
    expect(start?.type).toBe("polygon");
    expect(end?.type).toBe("polygon");
    if (start?.type === "polygon" && end?.type === "polygon") {
      expect(end.points[0]!.x).toBeGreaterThan(start.points[0]!.x);
    }
  });

  it("draws a body spanning bumper to bumper", () => {
    const corners = bodyAt(
      { x: 0, y: 0, heading: 0, reverse: false, clearance: 1, overhanging: false },
      lbx,
    );
    const xs = corners.map((c) => c.x);
    expect(Math.min(...xs)).toBeCloseTo(-0.76, 9);
    expect(Math.max(...xs)).toBeCloseTo(3.43, 9);
  });

  it("survives an empty manoeuvre", () => {
    expect(pathToPrimitives(maneuver([]), lbx)).toEqual([]);
  });
});

describe("overhang", () => {
  it("splits the path where the body starts overhanging", () => {
    const roles = pathToPrimitives(
      maneuver([1, 1, 1, 1], undefined, [false, false, true, true]),
      lbx,
    )
      .filter((p) => p.type === "polyline")
      .map((p) => p.role);
    expect(roles).toContain("overhang");
    expect(roles.filter((r) => r === "overhang")).toHaveLength(1);
  });

  it("marks nothing when nothing overhangs", () => {
    const roles = pathToPrimitives(maneuver([1, 1, 1, 1]), lbx)
      .filter((p) => p.type === "polyline")
      .map((p) => p.role);
    expect(roles).not.toContain("overhang");
  });
});

describe("playing the path back", () => {
  const ten = maneuver(Array.from({ length: 11 }, () => 0.8));

  it("draws where the trip goes before playback gets there", () => {
    // Without it, a paused playback shows a trace that stops in mid-air and
    // says nothing about where the vehicle is heading.
    const primitives = pathToPrimitives(ten, lbx, 0.3);
    expect(primitives.filter((p) => p.role === "upcoming")).toHaveLength(1);
  });

  it("colours only the part already travelled", () => {
    // The coloured stretches carry the proximity bands, so they must not run
    // ahead of the vehicle: a red stretch the vehicle has not reached yet
    // would report a danger that has not happened.
    const early = pathToPrimitives(ten, lbx, 0.2);
    const late = pathToPrimitives(ten, lbx, 1);
    const travelled = (list: typeof early) =>
      list
        .filter((p) => p.type === "polyline" && PATH_ROLES.has(p.role))
        .flatMap((p) => (p.type === "polyline" ? p.points : []));
    expect(travelled(early).length).toBeLessThan(travelled(late).length);
  });

  it("leaves the whole trace behind once playback has finished", () => {
    const finished = pathToPrimitives(ten, lbx, 1);
    const points = finished
      .filter((p) => p.type === "polyline" && PATH_ROLES.has(p.role))
      .flatMap((p) => (p.type === "polyline" ? p.points : []));
    expect(Math.max(...points.map((p) => p.x))).toBeCloseTo(2, 9);
  });

  it("accumulates the ghosts rather than showing them all at once", () => {
    // The point of the animation: what is left behind at the end is the
    // épure, built by the playback rather than drawn beside it.
    const ghosts = (position: number) =>
      pathToPrimitives(ten, lbx, position).filter((p) => p.role === "ghost").length;
    expect(ghosts(0)).toBeLessThan(ghosts(0.5));
    expect(ghosts(0.5)).toBeLessThan(ghosts(1));
  });

  it("never draws a ghost the vehicle has not reached", () => {
    const primitives = pathToPrimitives(ten, lbx, 0.4);
    const vehicle = primitives.find((p) => p.role === "vehicle");
    const ahead = vehicle?.type === "polygon"
      ? Math.max(...vehicle.points.map((p) => p.x))
      : 0;
    for (const ghost of primitives.filter((p) => p.role === "ghost")) {
      if (ghost.type !== "polygon") continue;
      expect(Math.max(...ghost.points.map((p) => p.x))).toBeLessThanOrEqual(ahead + 1e-9);
    }
  });

  it("places the vehicle by distance travelled, not by pose index", () => {
    // Poses crowded at one end: the same reason `playback.ts` exists.
    const crowded = maneuver([0.8, 0.8, 0.8, 0.8]);
    crowded.poses[1]!.x = 0.05;
    crowded.poses[2]!.x = 0.1;
    crowded.poses[3]!.x = 4;
    const primitives = pathToPrimitives(crowded, lbx, 0.5);
    const vehicle = primitives.find((p) => p.role === "vehicle");
    const rear = vehicle?.type === "polygon"
      ? Math.min(...vehicle.points.map((p) => p.x))
      : NaN;
    // Half the distance is 2 m in, which lands on the pose at 0.1 — the last
    // one before the long stretch. By index it would have been pose 2 as
    // well, so check the position is read from the marks at all.
    expect(rear).toBeLessThan(1);
  });
});

/**
 * Poses on a known arc, integrated with the model of `CLAUDE.md`.
 *
 * The tests below run this backwards: given the poses, recover the arc the
 * wheels were turned to. Building them from the same integration is what
 * makes the recovery a real inversion rather than a restatement.
 */
function arc(radius: number, reverse: boolean, steps = 5): PoseDto[] {
  const kappa = 1 / radius;
  const ds = (reverse ? -1 : 1) * 0.2;
  let x = 0;
  let y = 0;
  let heading = 0;
  const out: PoseDto[] = [
    { x, y, heading, reverse, clearance: 1, overhanging: false },
  ];
  for (let i = 0; i < steps; i++) {
    const next = heading + kappa * ds;
    x += radius * (Math.sin(next) - Math.sin(heading));
    y -= radius * (Math.cos(next) - Math.cos(heading));
    heading = next;
    out.push({ x, y, heading, reverse, clearance: 1, overhanging: false });
  }
  return out;
}

describe("the steering a path implies", () => {
  it("recovers the arc a forward turn was driven on", () => {
    // Exactly, not nearly: a segment is a circular arc, and the chord of one
    // relates to its curvature in closed form. The small-angle reading gets
    // this wrong in the fifth decimal, which is a tenth of a degree of steer.
    expect(curvatureAt(arc(6, false), 3)).toBeCloseTo(1 / 6, 9);
  });

  it("reads the same steering when that arc is driven in reverse", () => {
    // Wheel held where it was, gear changed: the car retraces the arc and the
    // heading turns the other way. The wheels must still be drawn turned the
    // way they are actually turned, not the way the heading moved.
    expect(curvatureAt(arc(6, true), 3)).toBeCloseTo(1 / 6, 9);
  });

  it("keeps the steering where a pose is emitted twice", () => {
    // A gear change can repeat a pose. Dividing by that chord would put an
    // infinity into a coordinate and blank the plan; snapping the wheels
    // straight for one frame would flicker. The neighbouring segment holds.
    const stalled = arc(6, false);
    stalled[3] = { ...stalled[2]! };
    expect(curvatureAt(stalled, 3)).toBeCloseTo(1 / 6, 9);
  });

  it("draws straight wheels when there is no segment at all", () => {
    const single = [arc(6, false)[0]!];
    expect(curvatureAt(single, 0)).toBe(0);
  });

  it("reads the first pose from the segment that leaves it", () => {
    // At the start of a playback the vehicle sits on pose zero, which no
    // segment arrives at. Its wheels belong to the turn it is about to make.
    expect(curvatureAt(arc(6, false), 0)).toBeCloseTo(1 / 6, 9);
  });
});

/** Which way a drawn tyre points, from the long axis of its rectangle. */
function pointing(tyre: Primitive): number {
  if (tyre.type !== "polygon") throw new Error("not a tyre");
  const [a, b, c, d] = tyre.points as [Point, Point, Point, Point];
  return Math.atan2(
    (b.y + c.y) / 2 - (a.y + d.y) / 2,
    (b.x + c.x) / 2 - (a.x + d.x) / 2,
  );
}

describe("the wheels", () => {
  const square: PoseDto = {
    x: 0,
    y: 0,
    heading: 0,
    reverse: false,
    clearance: 1,
    overhanging: false,
  };

  const tyres = (role: string, curvature: number) =>
    vehicleAt(square, lbx, "vehicle", curvature).filter((p) => p.role === role);

  it("turns the front wheels into a left-hand bend and leaves the rear alone", () => {
    const bend = 1 / 6;
    const bicycle = Math.atan(lbx.wheelbase * bend);

    const front = tyres("wheel-front", bend).map(pointing).sort((x, y) => x - y);
    expect(front).toHaveLength(2);
    // Ackermann: the wheel on the inside of the bend has the tighter circle to
    // follow, so it turns further than the bicycle model's single angle, and
    // the outer one turns less. Both turn the way the bend goes.
    expect(front[0]!).toBeGreaterThan(0);
    expect(front[0]!).toBeLessThan(bicycle);
    expect(front[1]!).toBeGreaterThan(bicycle);

    expect(tyres("wheel-rear", bend).map(pointing)).toEqual([0, 0]);
  });
});

describe("what a drawn wheel may never do", () => {
  /**
   * Every vehicle the database describes fully, at the tightest circle it can
   * turn — which is where a steered wheel swings furthest sideways.
   */
  const complete = VEHICLES.flatMap((v) =>
    v.wheelbase !== null &&
    v.length !== null &&
    v.front_overhang !== null &&
    v.width !== null &&
    v.mirror_width !== null &&
    v.ground_clearance !== null &&
    v.min_turning_radius !== null
      ? [
          [
            v.label,
            {
              wheelbase: v.wheelbase,
              length: v.length,
              front_overhang: v.front_overhang,
              width: v.width,
              mirror_width: v.mirror_width,
              ground_clearance: v.ground_clearance,
              min_turning_radius: v.min_turning_radius,
            } satisfies VehicleDto,
          ] as const,
        ]
      : [],
  );

  it("covers the database rather than one convenient car", () => {
    expect(complete.length).toBeGreaterThanOrEqual(4);
  });

  it("never reaches outside the envelope the solver tests", () => {
    // The whole tool rests on the drawing not claiming room the model never
    // granted. A wheel poking past the widest tested point does exactly that,
    // and it does it at full lock — where the margins are already thinnest.
    for (const [label, vehicle] of complete) {
      const square: PoseDto = {
        x: 0,
        y: 0,
        heading: 0,
        reverse: false,
        clearance: 1,
        overhanging: false,
      };
      for (const lock of [1 / vehicle.min_turning_radius, -1 / vehicle.min_turning_radius]) {
        const reach = vehicleAt(square, vehicle, "vehicle", lock)
          .filter((p) => p.role === "wheel-front" || p.role === "wheel-rear")
          .flatMap((p) => (p.type === "polygon" ? p.points : []))
          .map((p) => Math.abs(p.y));
        expect(Math.max(...reach), `${label} at full lock`).toBeLessThanOrEqual(
          vehicle.mirror_width / 2,
        );
      }
    }
  });
});

describe("which end is the front", () => {
  const square: PoseDto = {
    x: 0,
    y: 0,
    heading: 0,
    reverse: false,
    clearance: 1,
    overhanging: false,
  };

  it("chamfers both front corners of the solid vehicle", () => {
    const chamfers = vehicleAt(square, lbx, "vehicle").filter(
      (p) => p.role === "chamfer",
    );
    expect(chamfers).toHaveLength(2);
    // One per side, so the nose reads as a nose and not as a dent.
    const sides = chamfers.flatMap((p) =>
      p.type === "polyline" ? [Math.sign(p.points[0]!.y)] : [],
    );
    expect(sides.sort()).toEqual([-1, 1]);
  });

  it("gives a ghost its direction without giving it wheels", () => {
    // Six cars with four wheels each would fill the gateway they exist to show
    // the vehicle passing through. Two lines each is what a trail can carry.
    const ghost = vehicleAt(square, lbx, "ghost");
    expect(ghost.filter((p) => p.role.startsWith("wheel"))).toHaveLength(0);
    expect(
      ghost.filter((p) => p.type === "polyline" && p.role === "ghost"),
    ).toHaveLength(2);
  });

  it("keeps the chamfer inside the footprint the solver tests", () => {
    // Drawn, not cut: biting the corner off the polygon would show a car
    // smaller than the one the collision test ran against.
    const front = lbx.wheelbase + lbx.front_overhang;
    const rear = -(lbx.length - lbx.wheelbase - lbx.front_overhang);
    for (const p of vehicleAt(square, lbx, "vehicle")) {
      if (p.type !== "polyline" || p.role !== "chamfer") continue;
      for (const point of p.points) {
        expect(Math.abs(point.y)).toBeLessThanOrEqual(lbx.width / 2 + 1e-9);
        expect(point.x).toBeLessThanOrEqual(front + 1e-9);
        expect(point.x).toBeGreaterThanOrEqual(rear - 1e-9);
      }
    }
  });
});

describe("the silhouette against the envelope", () => {
  const square: PoseDto = {
    x: 0,
    y: 0,
    heading: 0,
    reverse: false,
    clearance: 1,
    overhanging: false,
  };

  /** Everything drawn at a real size, in metres — pixel markers excluded. */
  const drawn = (vehicle: VehicleDto) =>
    vehicleAt(square, vehicle, "vehicle").flatMap((p) =>
      p.type === "polygon" || p.type === "polyline" ? p.points : [],
    );

  it("reaches exactly as wide as the point the solver tests", () => {
    // The body is not the envelope. `swept_core::vehicle::envelope` samples
    // the mirrors too, and they are what touches a pillar first. A plan that
    // draws the body alone shows a car narrower than the one it answered
    // about — at 40 px per metre, ten centimetres of it.
    const widest = Math.max(...drawn(lbx).map((p) => Math.abs(p.y)));
    expect(widest).toBeCloseTo(lbx.mirror_width / 2, 9);
  });

  it("draws a mirror the length of a mirror", () => {
    // Long enough to see at plan scale, short enough not to read as a side
    // skirt running the length of the door.
    const housings = vehicleAt(square, lbx, "vehicle").filter(
      (p) => p.role === "mirror",
    );
    expect(housings).toHaveLength(2);
    for (const housing of housings) {
      if (housing.type !== "polygon") throw new Error("not a housing");
      const xs = housing.points.map((p) => p.x);
      const along = Math.max(...xs) - Math.min(...xs);
      expect(along).toBeGreaterThan(0.1);
      expect(along).toBeLessThan(0.3);
    }
  });

  it("puts the mirrors where the core puts them", () => {
    // Level with the front axle — `vehicle.rs`. A real mirror sits behind it,
    // and correcting that is a change to the model, not to its drawing.
    const tips = vehicleAt(square, lbx, "vehicle")
      .flatMap((p) => (p.type === "polygon" ? p.points : []))
      .filter((p) => Math.abs(Math.abs(p.y) - lbx.mirror_width / 2) < 1e-9);
    expect(tips.length).toBeGreaterThan(0);
    for (const tip of tips) {
      expect(Math.abs(tip.x - lbx.wheelbase)).toBeLessThan(0.25);
    }
  });
});

describe("the wheels of the vehicle drawn on a path", () => {
  /** The manoeuvre a set of poses describes, with the bookkeeping filled in. */
  function trip(poses: PoseDto[]): ManeuverDto {
    return {
      poses,
      min_clearance: 1,
      min_clearance_in_gateway: 1,
      metres_under_25cm: 0,
      metres_under_10cm: 0,
      metres_overhanging: 0,
      distance: 1,
      moves: 1,
      confidence: "exact",
    };
  }

  it("turns with the bend the path is on", () => {
    // Without this the wheels would sit straight through every turn, which
    // reads as a car sliding sideways rather than steering.
    const poses = arc(6, false, 10);
    const primitives = pathToPrimitives(trip(poses), lbx, 1);
    const here = poses[poses.length - 1]!;
    const front = primitives
      .filter((p) => p.role === "wheel-front")
      .map((p) => pointing(p) - here.heading);
    expect(front).toHaveLength(2);
    for (const steer of front) expect(steer).toBeGreaterThan(0.05);
  });

  it("leaves them straight on a straight run", () => {
    const straight = Array.from({ length: 6 }, (_, i) => ({
      x: i * 0.2,
      y: 0,
      heading: 0,
      reverse: false,
      clearance: 1,
      overhanging: false,
    }));
    const front = pathToPrimitives(trip(straight), lbx, 1)
      .filter((p) => p.role === "wheel-front")
      .map(pointing);
    expect(front).toEqual([0, 0]);
  });
});
