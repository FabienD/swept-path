import { describe, expect, it } from "vitest";
import { projectionFor } from "./projection";

const viewport = { width: 1000, height: 600 };
const bounds = { xMin: -10, xMax: 10, yMin: -6, yMax: 6 };

describe("projection", () => {
  it("fits the scene inside the viewport", () => {
    const p = projectionFor(bounds, viewport, false);
    expect(p.x(-10)).toBeGreaterThanOrEqual(0);
    expect(p.x(10)).toBeLessThanOrEqual(viewport.width);
    expect(p.y(-6)).toBeLessThanOrEqual(viewport.height);
    expect(p.y(6)).toBeGreaterThanOrEqual(0);
  });

  it("puts positive y upwards on screen", () => {
    // The yard lies beyond the wall; on screen it must sit above it.
    const p = projectionFor(bounds, viewport, false);
    expect(p.y(3)).toBeLessThan(p.y(0));
  });

  it("mirrors along x when the vehicle arrives from the other side", () => {
    const plain = projectionFor(bounds, viewport, false);
    const mirrored = projectionFor(bounds, viewport, true);
    expect(mirrored.x(4)).toBeCloseTo(plain.x(-4), 6);
  });

  it("keeps one scale for both axes, so nothing is distorted", () => {
    // A stretched plan would misrepresent exactly the distances this tool
    // exists to measure.
    const p = projectionFor({ xMin: -10, xMax: 10, yMin: -2, yMax: 2 }, viewport, false);
    const across = Math.abs(p.x(1) - p.x(0));
    const up = Math.abs(p.y(1) - p.y(0));
    expect(across).toBeCloseTo(up, 6);
  });
});
