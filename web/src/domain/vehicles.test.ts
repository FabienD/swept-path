import { describe, expect, it } from "vitest";
import { VEHICLES, searchVehicles, vehicleById } from "./vehicles";

describe("vehicle table", () => {
  it("reads every entry from the database, with unique ids", () => {
    expect(VEHICLES.length).toBeGreaterThan(0);
    expect(new Set(VEHICLES.map((v) => v.id)).size).toBe(VEHICLES.length);
  });

  it("holds dimensions the core will accept, wherever it holds them", () => {
    // Catches a typo that would only surface as a rejected request. Skips
    // what the database does not know rather than inventing it.
    for (const v of VEHICLES) {
      if (v.length !== null && v.wheelbase !== null && v.front_overhang !== null) {
        expect(v.length - v.wheelbase - v.front_overhang, `${v.label}: rear overhang`)
          .toBeGreaterThan(0);
      }
      if (v.mirror_width !== null && v.width !== null) {
        expect(v.mirror_width, `${v.label}: mirrors`).toBeGreaterThanOrEqual(v.width);
      }
      if (v.min_turning_radius !== null) {
        expect(v.min_turning_radius, `${v.label}: radius`).toBeGreaterThan(0);
      }
    }
  });

  it("keeps folded mirrors between the body and the extended width", () => {
    for (const v of VEHICLES) {
      if (v.mirror_width_folded === null) continue;
      if (v.width !== null) {
        expect(v.mirror_width_folded, v.label).toBeGreaterThanOrEqual(v.width);
      }
      if (v.mirror_width !== null) {
        expect(v.mirror_width_folded, v.label).toBeLessThanOrEqual(v.mirror_width);
      }
    }
  });

  it("turns tighter than the published radius, wherever one is published", () => {
    // Published figures are curb to curb, traced by the outer front wheel.
    // The bicycle model pivots about the rear axle, well inside that circle.
    for (const v of VEHICLES) {
      if (v.min_turning_radius === null || v.published_radius === null) continue;
      expect(v.min_turning_radius, v.label).toBeLessThan(v.published_radius);
      expect(v.min_turning_radius, v.label).toBeGreaterThan(v.published_radius / 2);
    }
  });

  it("converts the LBX to the radius the geometry implies", () => {
    const lbx = vehicleById("lexus-lbx");
    expect(lbx?.published_radius).toBe(5.2);
    expect(lbx?.min_turning_radius).toBeCloseTo(3.59, 1);
  });

  it("leaves the pivot radius unknown when nothing is published", () => {
    // A missing figure must stay missing all the way to the form. Filling it
    // with something plausible is exactly how a guess becomes a measurement.
    //
    // Stated over the whole table rather than one named vehicle: the database
    // is edited by hand, and a test that pins an example fails the day that
    // example is completed — which says nothing about the rule.
    for (const v of VEHICLES) {
      if (v.published_radius === null) {
        expect(v.min_turning_radius, v.label).toBeNull();
      }
    }
  });

  it("never turns an absent measurement into a number", () => {
    // Whatever the database leaves out must arrive as null, not as zero and
    // not as a neighbour's figure.
    for (const v of VEHICLES) {
      for (const [name, field] of Object.entries(v)) {
        if (typeof field === "number") {
          expect(field, `${v.label}: ${name}`).not.toBe(0);
        }
      }
    }
  });
});

describe("searching the table", () => {
  it("returns everything on an empty query", () => {
    expect(searchVehicles("")).toHaveLength(VEHICLES.length);
    expect(searchVehicles("   ")).toHaveLength(VEHICLES.length);
  });

  it("matches on make and on model, whatever the case", () => {
    expect(searchVehicles("lexus").length).toBeGreaterThan(1);
    expect(searchVehicles("LEXUS").length).toBe(searchVehicles("lexus").length);
    expect(searchVehicles("911").every((v) => v.label.includes("911"))).toBe(true);
  });

  it("ignores accents, so that a French keyboard finds what it types", () => {
    expect(searchVehicles("téslà").length).toBe(searchVehicles("tesla").length);
  });

  it("returns nothing rather than everything when nothing matches", () => {
    expect(searchVehicles("zzzz")).toHaveLength(0);
  });
});
