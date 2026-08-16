import { describe, expect, it } from "vitest";
import type { Preferences } from "../i18n/preferences";
import type { SceneDto } from "../domain/types";
import { boundsFor, clearWidth, sceneToPrimitives } from "./scene";

function scene(opening: number, swinging = false): SceneDto {
  return {
    left_post: { inner_edge_x: -opening / 2, width: 0.55, depth: 0.55 },
    right_post: { inner_edge_x: opening / 2, width: 0.55, depth: 0.55 },
    wall_thickness: 0.3,
    sidewalk_width: 1.2,
    curb_cut_width: opening + 0.8,
    road_width: 4.5,
    curb_height: 0.12,
    gate: swinging
      ? {
          kind: "swinging",
          leaf_length: 1.15,
          leaf_thickness: 0.1,
          hinge_offset: 0.05,
          hinge_depth_ratio: 0.5,
          open_angle: Math.PI / 2,
        }
      : { kind: "sliding" },
  };
}

const shapes = (s: SceneDto) =>
  sceneToPrimitives(s, FR).filter((p) => p.type === "polygon");

const FR: Preferences = { locale: "fr", units: "metric" };
const US: Preferences = { locale: "en", units: "us" };

describe("scene rendering", () => {
  it("draws the wall, the posts and the split sidewalk", () => {
    const roles = shapes(scene(2.4)).map((p) => p.role);
    expect(roles.filter((r) => r === "wall")).toHaveLength(2);
    expect(roles.filter((r) => r === "post")).toHaveLength(2);
    expect(roles.filter((r) => r === "sidewalk")).toHaveLength(2);
    expect(roles.filter((r) => r === "leaf")).toHaveLength(0);
  });

  it("adds two leaves for a swinging gate", () => {
    const roles = shapes(scene(2.4, true)).map((p) => p.role);
    expect(roles.filter((r) => r === "leaf")).toHaveLength(2);
  });

  it("omits the sidewalk when there is none", () => {
    const none = { ...scene(2.4), sidewalk_width: 0 };
    expect(shapes(none).filter((p) => p.role === "sidewalk")).toHaveLength(0);
  });

  it("leaves the full span clear for a sliding gate", () => {
    expect(clearWidth(scene(2.4))).toBeCloseTo(2.4, 9);
  });

  it("loses opening to the leaves when they swing", () => {
    // Each hinge sits 5 cm back, and a 10 cm leaf stands square across.
    expect(clearWidth(scene(2.4, true))).toBeCloseTo(2.4 - 0.1 - 0.1, 9);
  });

  it("shows the yard, the road and everything between", () => {
    const bounds = boundsFor(scene(2.4));
    expect(bounds.yMin).toBeLessThan(-5.5);
    expect(bounds.yMax).toBeGreaterThan(6);
  });

  it("annotates the plan with the figures that matter", () => {
    const labels = sceneToPrimitives(scene(2.4), FR).filter((p) => p.type === "label");
    expect(labels.some((l) => l.text.includes("passage libre"))).toBe(true);
    expect(labels.some((l) => l.text.includes("chaussée"))).toBe(true);
  });

  it("annotates in the reader's language and unit", () => {
    // The plan is read by whoever set the language, so its own figures have
    // to follow it — a French annotation on an English page would be the one
    // thing left untranslated, and the most visible.
    const labels = sceneToPrimitives(scene(2.4), US).filter((p) => p.type === "label");
    expect(labels.some((l) => l.text.includes("clear opening"))).toBe(true);
    // A roadway is a distance, so it is feet; the opening is a dimension, so
    // it is inches.
    expect(labels.some((l) => l.text.includes("ft"))).toBe(true);
    expect(labels.some((l) => l.text.includes("in"))).toBe(true);
  });
});
