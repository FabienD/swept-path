import { describe, expect, it } from "vitest";
import { VEHICLES, vehicleById } from "./vehicles";

describe("vehicle presets", () => {
  it("ships the six the prototype had", () => {
    expect(VEHICLES).toHaveLength(6);
    expect(new Set(VEHICLES.map((v) => v.id)).size).toBe(6);
  });

  it("holds dimensions the core will accept", () => {
    // Catches a typo that would only surface as a rejected request.
    for (const v of VEHICLES) {
      const rearOverhang = v.length - v.wheelbase - v.front_overhang;
      expect(rearOverhang, `${v.label}: rear overhang`).toBeGreaterThan(0);
      expect(v.mirror_width, `${v.label}: mirrors`).toBeGreaterThanOrEqual(v.width);
      expect(v.min_turning_radius, `${v.label}: radius`).toBeGreaterThan(0);
    }
  });

  it("keeps folded mirrors between the body and the extended width", () => {
    for (const v of VEHICLES) {
      expect(v.mirror_width_folded).toBeGreaterThanOrEqual(v.width);
      expect(v.mirror_width_folded).toBeLessThanOrEqual(v.mirror_width);
    }
  });

  it("turns tighter than the published radius, on every preset", () => {
    // Published figures are kerb to kerb, traced by the outer front wheel.
    // The bicycle model pivots about the rear axle, well inside that circle.
    for (const v of VEHICLES) {
      expect(v.min_turning_radius, `${v.label}`).toBeLessThan(v.kerb_radius);
      expect(v.min_turning_radius, `${v.label}`).toBeGreaterThan(v.kerb_radius / 2);
    }
  });

  it("converts the LBX to the radius the geometry implies", () => {
    const lbx = vehicleById("lexus-lbx");
    expect(lbx?.kerb_radius).toBe(5.2);
    expect(lbx?.min_turning_radius).toBeCloseTo(3.59, 1);
  });

  it("finds a preset by id, and nothing by a wrong one", () => {
    expect(vehicleById("lexus-lbx")?.label).toBe("Lexus LBX");
    expect(vehicleById("no-such-car")).toBeUndefined();
  });
});
