import { describe, expect, it } from "vitest";
import type { ManeuverDto, PoseDto, VehicleDto } from "../domain/types";
import { bodyAt, pathToPrimitives } from "./path";

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

describe("path rendering", () => {
  // The whole trip is drawn separately, underneath: these tests are about how
  // the travelled path is cut into bands, not about that backdrop.
  // Two filters rather than one condition: a conjunction loses the narrowing
  // that `type === "polyline"` gives, and `dashed` stops type-checking.
  const lines = (m: ManeuverDto) =>
    pathToPrimitives(m, lbx)
      .filter((p) => p.type === "polyline")
      .filter((p) => p.role !== "upcoming");

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
        .filter((p) => p.type === "polyline" && p.role !== "upcoming")
        .flatMap((p) => (p.type === "polyline" ? p.points : []));
    expect(travelled(early).length).toBeLessThan(travelled(late).length);
  });

  it("leaves the whole trace behind once playback has finished", () => {
    const finished = pathToPrimitives(ten, lbx, 1);
    const points = finished
      .filter((p) => p.type === "polyline" && p.role !== "upcoming")
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
