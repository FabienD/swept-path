import type { Primitive, Role } from "./primitives";
import type { Projection } from "./projection";

const NS = "http://www.w3.org/2000/svg";

/** How each role is painted. The backend maps roles to styles; the primitive
 *  producers never choose a colour. */
const STYLE: Record<Role, { fill?: string; stroke?: string; width?: number; opacity?: number }> = {
  grid: { stroke: "#e7e5e4", width: 1 },
  wall: { fill: "#d6d3d1", stroke: "#78716c" },
  post: { fill: "#c7c2be", stroke: "#57534e" },
  leaf: { fill: "#a8a29e", stroke: "#44403c" },
  pavement: { fill: "#ebe9e7", stroke: "#a8a29e" },
  "road-edge": { stroke: "#44403c", width: 2 },
  "road-centre": { stroke: "#a8a29e", width: 1.5 },
  vehicle: { fill: "#1e40af22", stroke: "#1e40af", width: 1.6 },
  ghost: { fill: "#1e40af11", stroke: "#1e40af", width: 1, opacity: 0.35 },
  mirror: { fill: "#c42b1c" },
  nose: { fill: "#1e40af" },
  reversal: { fill: "#fafaf9", stroke: "#1c1917", width: 1.4 },
  "band-clear": { stroke: "var(--color-band-clear)", width: 1.8 },
  "band-watch": { stroke: "var(--color-band-watch)", width: 3 },
  "band-close": { stroke: "var(--color-band-close)", width: 3 },
  "band-tight": { stroke: "var(--color-band-tight)", width: 3 },
  annotation: { fill: "#57534e" },
};

function element(name: string, attributes: Record<string, string | number>) {
  const node = document.createElementNS(NS, name);
  for (const [key, value] of Object.entries(attributes)) {
    node.setAttribute(key, String(value));
  }
  return node;
}

function pointsAttribute(
  points: readonly { x: number; y: number }[],
  projection: Projection,
): string {
  return points
    .map((p) => `${projection.x(p.x).toFixed(1)},${projection.y(p.y).toFixed(1)}`)
    .join(" ");
}

/**
 * Draws a list of primitives into an SVG element, replacing its contents.
 *
 * Decides nothing: every shape and its role were settled upstream.
 */
export function renderSvg(
  primitives: readonly Primitive[],
  target: SVGSVGElement,
  projection: Projection,
): void {
  target.replaceChildren();
  const group = element("g", {});

  for (const primitive of primitives) {
    const style = STYLE[primitive.role];
    switch (primitive.type) {
      case "polygon":
        group.appendChild(
          element("polygon", {
            points: pointsAttribute(primitive.points, projection),
            fill: style.fill ?? "none",
            stroke: style.stroke ?? "none",
            "stroke-width": style.width ?? 1,
            ...(style.opacity === undefined ? {} : { opacity: style.opacity }),
          }),
        );
        break;
      case "polyline":
        group.appendChild(
          element("polyline", {
            points: pointsAttribute(primitive.points, projection),
            fill: "none",
            stroke: style.stroke ?? "currentColor",
            "stroke-width": style.width ?? 1,
            "stroke-linecap": "round",
            ...(primitive.dashed ? { "stroke-dasharray": "10 8" } : {}),
          }),
        );
        break;
      case "circle":
        group.appendChild(
          element("circle", {
            cx: projection.x(primitive.centre.x).toFixed(1),
            cy: projection.y(primitive.centre.y).toFixed(1),
            r: primitive.radius,
            fill: style.fill ?? "none",
            stroke: style.stroke ?? "none",
            "stroke-width": style.width ?? 1,
            ...(style.opacity === undefined ? {} : { opacity: style.opacity }),
          }),
        );
        break;
      case "label": {
        const text = element("text", {
          x: projection.x(primitive.at.x).toFixed(1),
          y: projection.y(primitive.at.y).toFixed(1),
          fill: style.fill ?? "currentColor",
          "font-size": 13,
          "text-anchor": primitive.anchor ?? "start",
        });
        text.textContent = primitive.text;
        group.appendChild(text);
        break;
      }
    }
  }

  target.appendChild(group);
}
