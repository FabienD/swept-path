import type { Primitive, Role } from "./primitives";
import type { Projection } from "./projection";

const NS = "http://www.w3.org/2000/svg";

/** How each role is painted. The backend maps roles to styles; the primitive
 *  producers never choose a colour. */
const STYLE: Record<Role, { fill?: string; stroke?: string; width?: number; opacity?: number }> = {
  grid: { stroke: "var(--color-plan-grid)", width: 1 },
  wall: { fill: "var(--color-wall)", stroke: "var(--color-wall-edge)" },
  post: { fill: "var(--color-post)", stroke: "var(--color-post-edge)" },
  leaf: { fill: "var(--color-leaf)", stroke: "var(--color-leaf-edge)" },
  sidewalk: {
    fill: "var(--color-sidewalk)",
    stroke: "var(--color-sidewalk-edge)",
  },
  "road-edge": { stroke: "var(--color-road-edge)", width: 2 },
  "road-centre": { stroke: "var(--color-road-centre)", width: 1.5 },
  // The vehicle wears the accent: it is the one thing on the plan that is
  // neither scenery nor a measurement, and the trace beside it carries the
  // proximity colours, so the two never compete.
  vehicle: {
    fill: "color-mix(in srgb, var(--color-accent) 13%, transparent)",
    stroke: "var(--color-accent)",
    width: 1.6,
  },
  ghost: { stroke: "var(--color-ghost)", width: 1, opacity: 0.5 },
  mirror: { fill: "var(--color-accent)" },
  nose: { fill: "var(--color-accent)" },
  // Hollow, so it reads as a marker on the path rather than a point of it.
  reversal: { fill: "var(--color-plan)", stroke: "var(--color-fg)", width: 1.4 },
  "band-clear": { stroke: "var(--color-band-clear)", width: 1.8 },
  "band-watch": { stroke: "var(--color-band-watch)", width: 3 },
  "band-close": { stroke: "var(--color-band-close)", width: 3 },
  "band-tight": { stroke: "var(--color-band-tight)", width: 3 },
  overhang: { stroke: "var(--color-overhang)", width: 3 },
  // Where the trip goes, drawn once under everything else. Quiet enough not
  // to be mistaken for a proximity band, present enough that the destination
  // is visible before playback reaches it.
  upcoming: { stroke: "var(--color-ghost)", width: 1.5, opacity: 0.55 },
  annotation: { fill: "var(--color-annotation)" },
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
