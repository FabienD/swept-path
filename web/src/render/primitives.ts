/**
 * What to draw, independently of how.
 *
 * A pure function produces this list from a scene and a result; a backend
 * turns it into pixels. Keeping the two apart is what makes the renderer
 * replaceable — a Canvas or 3D backend would consume exactly this — and what
 * makes the interesting half testable without a browser.
 */

/** A point in world coordinates, in metres. */
export interface Point {
  x: number;
  y: number;
}

/** Which visual role a primitive plays. The backend maps roles to tokens; no
 *  colour is ever chosen here. */
export type Role =
  | "grid"
  | "wall"
  | "post"
  | "leaf"
  | "pavement"
  | "road-edge"
  | "road-centre"
  | "vehicle"
  | "ghost"
  | "mirror"
  | "nose"
  | "reversal"
  | "band-clear"
  | "band-watch"
  | "band-close"
  | "band-tight"
  | "overhang"
  | "annotation";

export type Primitive =
  | { type: "polygon"; role: Role; points: Point[] }
  | { type: "polyline"; role: Role; points: Point[]; dashed?: boolean }
  | { type: "circle"; role: Role; centre: Point; radius: number }
  | {
      type: "label";
      role: Role;
      at: Point;
      text: string;
      anchor?: "start" | "middle" | "end";
    };

/** An axis-aligned rectangle, as the four corners of a polygon. */
export function rectangle(
  role: Role,
  x0: number,
  x1: number,
  y0: number,
  y1: number,
): Primitive {
  return {
    type: "polygon",
    role,
    points: [
      { x: x0, y: y0 },
      { x: x1, y: y0 },
      { x: x1, y: y1 },
      { x: x0, y: y1 },
    ],
  };
}
