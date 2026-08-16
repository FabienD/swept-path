/**
 * The proximity bands a clearance falls into.
 *
 * These are domain thresholds, not drawing ones: the plan colours its path by
 * them, and the verdict picks its wording from them, so both say the same
 * thing about the same trajectory. A path drawn in red and described as
 * comfortable would be the interface contradicting itself.
 */

/**
 * Clearance thresholds separating the bands, in metres.
 *
 * Carried over from the prototype (`index.html:604`): beyond 50 cm, 25 to 50,
 * 10 to 25, under 10. Reading a path by colour is what tells a driver *where*
 * it gets tight, which a single minimum never says.
 */
export const BANDS = [0.5, 0.25, 0.1] as const;

/** Which band a clearance falls into, 0 being the roomiest. */
export function bandOf(clearance: number): number {
  if (clearance >= BANDS[0]) return 0;
  if (clearance >= BANDS[1]) return 1;
  if (clearance >= BANDS[2]) return 2;
  return 3;
}
