import { describe, expect, it } from "vitest";
import {
  FOOT_M,
  INCH_M,
  fromDisplay,
  measure,
  stepFor,
  toDisplay,
  unitOf,
} from "./units";

describe("which unit a quantity is shown in", () => {
  it("keeps metres and centimetres apart in metric", () => {
    // A margin is reported in centimetres because it is measured in
    // centimetres; a gateway is not.
    expect(unitOf("clearance", "metric")).toBe("cm");
    expect(unitOf("dimension", "metric")).toBe("m");
    expect(unitOf("distance", "metric")).toBe("m");
  });

  it("follows American practice rather than one unit for everything", () => {
    // A manufacturer's sheet gives a wheelbase in inches; a street is
    // described in feet. Reporting a 5,90 m roadway as 232 in would be
    // arithmetically right and unreadable.
    expect(unitOf("clearance", "us")).toBe("in");
    expect(unitOf("dimension", "us")).toBe("in");
    expect(unitOf("distance", "us")).toBe("ft");
  });
});

describe("converting for display", () => {
  it("uses the exact international definitions", () => {
    // Both are exact by definition, not approximations: an inch is 25.4 mm
    // and a foot is twelve of them.
    expect(INCH_M).toBe(0.0254);
    expect(FOOT_M).toBeCloseTo(12 * INCH_M, 15);
  });

  it("shows a margin in centimetres or inches", () => {
    expect(toDisplay(0.045, "clearance", "metric")).toBeCloseTo(4.5, 9);
    expect(toDisplay(0.045, "clearance", "us")).toBeCloseTo(1.7717, 4);
  });

  it("shows a vehicle in metres or inches", () => {
    // The reference gateway and the LBX, as an American sheet would print it.
    expect(toDisplay(2.29, "dimension", "metric")).toBeCloseTo(2.29, 9);
    expect(toDisplay(2.58, "dimension", "us")).toBeCloseTo(101.57, 2);
  });

  it("shows a street in metres or feet", () => {
    expect(toDisplay(5.9, "distance", "us")).toBeCloseTo(19.36, 2);
  });
});

describe("reading a value back", () => {
  it("returns exactly what was displayed, in metres", () => {
    for (const magnitude of ["clearance", "dimension", "distance"] as const) {
      for (const system of ["metric", "us"] as const) {
        const there = toDisplay(2.29, magnitude, system);
        expect(fromDisplay(there, magnitude, system)).toBeCloseTo(2.29, 12);
      }
    }
  });

  it("never turns a typed figure into a different one by round-tripping", () => {
    // What the visitor typed is what the solver receives. A gateway entered
    // as 90.2 in must not come back as 90.19 in after a redraw.
    const typed = 90.2;
    const stored = fromDisplay(typed, "dimension", "us");
    expect(toDisplay(stored, "dimension", "us")).toBeCloseTo(typed, 9);
  });
});

describe("how finely a field can be typed", () => {
  it("steps by the millimetre in metric and by a hundredth of an inch", () => {
    // The tool answers to the centimetre, so the input must not be coarser
    // than the answer it feeds.
    expect(stepFor("dimension", "metric")).toBeCloseTo(0.001, 9);
    expect(stepFor("dimension", "us")).toBeCloseTo(0.01, 9);
  });

  it("is fine enough that a millimetre is still expressible", () => {
    // A step coarser than a millimetre would quietly refuse measurements the
    // metric side accepts.
    expect(fromDisplay(stepFor("dimension", "us"), "dimension", "us")).toBeLessThan(
      0.001,
    );
  });
});

describe("a measure ready to be written out", () => {
  it("carries its unit and how many decimals it deserves", () => {
    expect(measure(0.045, "clearance", "metric")).toEqual({
      value: expect.closeTo(4.5, 9),
      unit: "cm",
      decimals: 1,
    });
  });

  it("does not print a vehicle to the millimetre in inches", () => {
    // 101.57 in, not 101.5748 in: the extra digits are noise from the
    // conversion, not precision anyone measured.
    expect(measure(2.58, "dimension", "us").decimals).toBe(1);
    expect(measure(2.58, "dimension", "metric").decimals).toBe(2);
  });
});
