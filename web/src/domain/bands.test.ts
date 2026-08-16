import { describe, expect, it } from "vitest";
import { BANDS, bandOf } from "./bands";

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

describe("the thresholds themselves", () => {
  it("descends from roomiest to tightest", () => {
    // The renderer indexes its colours by band number and the verdict picks
    // its wording the same way. Both read `BANDS` in order; an unsorted table
    // would silently mislabel every path.
    expect([...BANDS]).toEqual([...BANDS].sort((a, b) => b - a));
  });
});
