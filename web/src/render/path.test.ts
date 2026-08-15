import { describe, expect, it } from "vitest";
import type { ManeuverDto, PoseDto, VehicleDto } from "../domain/types";
import { bandOf, bodyAt, pathToPrimitives } from "./path";

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

describe("proximity bands", () => {
  it("classifies clearances into the four bands", () => {
    expect(bandOf(0.8)).toBe(0);
    expect(bandOf(0.3)).toBe(1);
    expect(bandOf(0.15)).toBe(2);
    expect(bandOf(0.02)).toBe(3);
  });

  it("puts the thresholds themselves in the roomier band", () => {
    expect(bandOf(0.5)).toBe(0);
    expect(bandOf(0.25)).toBe(1);
    expect(bandOf(0.1)).toBe(2);
  });
});

describe("path rendering", () => {
  const lines = (m: ManeuverDto) =>
    pathToPrimitives(m, lbx).filter((p) => p.type === "polyline");

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

  it("draws four ghosts and one solid vehicle", () => {
    const shapes = pathToPrimitives(maneuver([0.9, 0.8, 0.7, 0.6, 0.5]), lbx).filter(
      (p) => p.type === "polygon",
    );
    expect(shapes.filter((p) => p.role === "ghost")).toHaveLength(4);
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
