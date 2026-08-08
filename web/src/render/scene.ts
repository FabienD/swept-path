import type { SceneDto } from "../domain/types";
import { metres } from "../domain/labels";
import type { Primitive } from "./primitives";
import { rectangle } from "./primitives";
import type { Bounds } from "./projection";

/**
 * How far either side of the opening the plan is drawn, in metres.
 *
 * The scene model extends to 18 m, but showing that much would shrink the
 * gateway to nothing. Eleven metres covers the whole approach.
 */
const HALF_VIEW_M = 11;

/** Head-room drawn beyond the deepest obstacle, in metres. */
const YARD_MARGIN_M = 6;

/** Depth the gate leaves reach into the yard, or zero for a sliding gate. */
function gateDepth(scene: SceneDto): number {
  return scene.gate.kind === "swinging" ? scene.gate.leaf_length : 0;
}

/** The rectangle of world the plan has to show. */
export function boundsFor(scene: SceneDto): Bounds {
  const deepest = Math.max(
    scene.left_post.depth,
    scene.right_post.depth,
    scene.wall_thickness,
  );
  return {
    xMin: -HALF_VIEW_M,
    xMax: HALF_VIEW_M,
    yMin: -scene.pavement_width - scene.road_width - 1.2,
    yMax: deepest + gateDepth(scene) + YARD_MARGIN_M,
  };
}

/** The rectangle occupied by one gate leaf, as a rotated polygon. */
function leaf(scene: SceneDto, side: 1 | -1): Primitive | null {
  if (scene.gate.kind !== "swinging") return null;
  const { leaf_length, leaf_thickness, hinge_offset, hinge_depth_ratio, open_angle } =
    scene.gate;
  const post = side === 1 ? scene.right_post : scene.left_post;

  const hingeX = post.inner_edge_x - side * hinge_offset;
  const hingeY = hinge_depth_ratio * post.depth;
  const dx = -side * Math.cos(open_angle);
  const dy = Math.sin(open_angle);
  // Perpendicular to the leaf, for its thickness.
  const px = -dy * (leaf_thickness / 2);
  const py = dx * (leaf_thickness / 2);

  return {
    type: "polygon",
    role: "leaf",
    points: [
      { x: hingeX + px, y: hingeY + py },
      { x: hingeX + dx * leaf_length + px, y: hingeY + dy * leaf_length + py },
      { x: hingeX + dx * leaf_length - px, y: hingeY + dy * leaf_length - py },
      { x: hingeX - px, y: hingeY - py },
    ],
  };
}

/**
 * Everything a scene draws, before any result is known.
 *
 * Mirrors what the domain builds as obstacles, so that what is shown and what
 * is computed cannot drift apart.
 */
export function sceneToPrimitives(scene: SceneDto): Primitive[] {
  const left = scene.left_post.inner_edge_x;
  const right = scene.right_post.inner_edge_x;
  const leftOuter = left - scene.left_post.width;
  const rightOuter = right + scene.right_post.width;
  const roadEdge = -scene.pavement_width - scene.road_width;

  const out: Primitive[] = [];

  // A one-metre grid, for reading distances off the plan directly.
  const bounds = boundsFor(scene);
  for (let m = Math.ceil(bounds.xMin); m <= bounds.xMax; m++) {
    out.push({
      type: "polyline",
      role: "grid",
      points: [
        { x: m, y: bounds.yMin },
        { x: m, y: bounds.yMax },
      ],
    });
  }
  for (let m = Math.ceil(bounds.yMin); m <= bounds.yMax; m++) {
    out.push({
      type: "polyline",
      role: "grid",
      points: [
        { x: bounds.xMin, y: m },
        { x: bounds.xMax, y: m },
      ],
    });
  }

  // The pavement, split either side of the dropped kerb.
  if (scene.pavement_width > 0.001) {
    const halfKerb = scene.dropped_kerb_width / 2;
    const centre = (left + right) / 2;
    out.push(
      rectangle("pavement", bounds.xMin, centre - halfKerb, -scene.pavement_width, 0),
      rectangle("pavement", centre + halfKerb, bounds.xMax, -scene.pavement_width, 0),
    );
  }

  // The wall either side of the posts, then the posts themselves.
  out.push(
    rectangle("wall", bounds.xMin, leftOuter, 0, scene.wall_thickness),
    rectangle("wall", rightOuter, bounds.xMax, 0, scene.wall_thickness),
    rectangle("post", leftOuter, left, 0, scene.left_post.depth),
    rectangle("post", right, rightOuter, 0, scene.right_post.depth),
  );

  for (const side of [1, -1] as const) {
    const primitive = leaf(scene, side);
    if (primitive) out.push(primitive);
  }

  // The far kerb, and the centre line when there is room for one.
  out.push({
    type: "polyline",
    role: "road-edge",
    points: [
      { x: bounds.xMin, y: roadEdge },
      { x: bounds.xMax, y: roadEdge },
    ],
  });
  if (scene.road_width > 0.5) {
    const middle = -scene.pavement_width - scene.road_width / 2;
    out.push({
      type: "polyline",
      role: "road-centre",
      dashed: true,
      points: [
        { x: bounds.xMin, y: middle },
        { x: bounds.xMax, y: middle },
      ],
    });
  }

  // Dimensions worth reading without measuring on screen.
  const clear = clearWidth(scene);
  out.push(
    {
      type: "label",
      role: "annotation",
      at: { x: bounds.xMin + 0.3, y: roadEdge + 0.35 },
      text: `chaussée ${metres(scene.road_width)}`,
      anchor: "start",
    },
    {
      type: "label",
      role: "annotation",
      at: { x: (left + right) / 2, y: gateDepth(scene) + Math.max(scene.left_post.depth, scene.wall_thickness) + 0.5 },
      text: `passage libre ${metres(clear)}`,
      anchor: "middle",
    },
  );

  return out;
}

/**
 * The width actually left between the leaves, in metres.
 *
 * Open leaves eat into the opening: each hinge sits back from its post, and
 * the leaf has thickness. A sliding gate leaves the full span.
 */
export function clearWidth(scene: SceneDto): number {
  const span = scene.right_post.inner_edge_x - scene.left_post.inner_edge_x;
  if (scene.gate.kind !== "swinging") return span;
  const { hinge_offset, leaf_thickness, open_angle } = scene.gate;
  return span - 2 * hinge_offset - leaf_thickness * Math.abs(Math.sin(open_angle));
}
