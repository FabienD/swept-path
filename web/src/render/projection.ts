/** The rectangle of world the view has to show, in metres. */
export interface Bounds {
  xMin: number;
  xMax: number;
  yMin: number;
  yMax: number;
}

/** The drawing surface, in pixels. */
export interface Viewport {
  width: number;
  height: number;
}

/** Maps metres to pixels. */
export interface Projection {
  /** Along the road. */
  x(metres: number): number;
  /** Away from the road; positive `y` goes up on screen. */
  y(metres: number): number;
  /** Pixels per metre, the same on both axes. */
  scale: number;
}

/**
 * Builds a projection fitting `bounds` inside `viewport`.
 *
 * One scale serves both axes: a plan stretched on one of them would
 * misrepresent exactly the distances this tool exists to measure.
 *
 * `mirrored` flips along `x`, for a vehicle arriving from the other side.
 */
export function projectionFor(
  bounds: Bounds,
  viewport: Viewport,
  mirrored: boolean,
): Projection {
  const spanX = bounds.xMax - bounds.xMin;
  const spanY = bounds.yMax - bounds.yMin;
  const scale = Math.min(viewport.width / spanX, viewport.height / spanY);

  const centreX = (bounds.xMin + bounds.xMax) / 2;
  const originX = viewport.width / 2;
  const originY = viewport.height - (viewport.height - spanY * scale) / 2;
  const side = mirrored ? -1 : 1;

  return {
    x: (metres) => originX + side * (metres - centreX) * scale,
    y: (metres) => originY - (metres - bounds.yMin) * scale,
    scale,
  };
}
