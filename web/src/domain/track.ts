/**
 * How wide apart the wheels sit, when nobody has published the figure.
 *
 * Two places need it and they must not drift: the conversion of a published
 * turning radius into the rear-axle pivot radius the solver wants, and the
 * drawing of the wheels on the plan. A second copy of this number would let
 * the plan show a car turning on a circle it was not given.
 */

/**
 * How much narrower the track is than the body, in metres.
 *
 * ESTIMATED. Wheels sit inboard of the bodywork; the gap is roughly a
 * handspan each side on a modern car. The database publishes a measured
 * `track_front` for most entries — `track.test.ts` holds this estimate against
 * every one of them, so it stays a calibration rather than a guess.
 */
export const BODY_TO_TRACK_M = 0.26;

/** The distance between the wheels of one axle, in metres. */
export function trackOf(width: number): number {
  return width - BODY_TO_TRACK_M;
}
