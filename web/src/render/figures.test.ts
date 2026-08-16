import { describe, expect, it } from "vitest";
import { LBX, gateway, spanTaken } from "./figures";

describe("the span a vehicle takes in a passage", () => {
  it("is its own width when it is square", () => {
    expect(spanTaken(2.029, 0.55, 0)).toBeCloseTo(2.029, 9);
  });

  it("grows with the angle, and never shrinks", () => {
    let previous = 0;
    for (const degrees of [0, 5, 10, 15, 20, 30]) {
      const span = spanTaken(2.029, 0.55, degrees);
      expect(span).toBeGreaterThan(previous);
      previous = span;
    }
  });

  it("grows without bound as the angle approaches square-on", () => {
    // The claim the documentation makes, and the one that disproved the old
    // "critical width" note: there is no opening wide enough to admit every
    // approach angle.
    expect(spanTaken(2.029, 0.55, 89)).toBeGreaterThan(50);
  });

  it("makes 15 degrees of skew cost more than 15 cm of width", () => {
    // Stated in the page, so it is held by a test rather than by my word.
    const skewed = spanTaken(2.029, 0.55, 15);
    expect(skewed - 2.029).toBeGreaterThan(0.15);
  });

  it("is what closes the reference gateway at 21.6 degrees", () => {
    // Measured, not guessed: on a 2,40 m opening the LBX runs out of room
    // between 21 and 22 degrees off square. Below that the angle is merely
    // expensive; beyond it, the opening is gone.
    expect(spanTaken(2.029, 0.55, 21)).toBeLessThan(2.4);
    expect(spanTaken(2.029, 0.55, 22)).toBeGreaterThan(2.4);
  });
});

describe("the gateway a figure is drawn on", () => {
  it("is symmetrical about the opening it is given", () => {
    const scene = gateway(2.4);
    expect(scene.right_post.inner_edge_x - scene.left_post.inner_edge_x).toBeCloseTo(
      2.4,
      9,
    );
  });

  it("keeps the reference vehicle narrower than the reference opening", () => {
    // Otherwise the figures would illustrate an impossible entry.
    expect(LBX.mirror_width).toBeLessThan(2.4);
  });
});
