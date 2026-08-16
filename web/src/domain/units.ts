/**
 * Turning metres into what the reader measures in.
 *
 * The core speaks metres and only metres, as `CLAUDE.md` requires. This is
 * the one place that converts, so no caller ever holds a length whose unit it
 * has to remember.
 *
 * The imperial side is not one unit throughout. A manufacturer's sheet gives
 * a wheelbase in inches — "Wheelbase 101.6 in" — and a street is described in
 * feet. Reporting a 5,90 m roadway as 232 in would be arithmetically correct
 * and unreadable, so what a length *is* decides how it is shown.
 */

/** Metres in an inch. Exact by international definition, not a rounding. */
export const INCH_M = 0.0254;

/** Metres in a foot: twelve inches, so also exact. */
export const FOOT_M = 12 * INCH_M;

export type UnitSystem = "metric" | "us";

/**
 * What a length represents, which is what decides its unit.
 *
 * - `clearance` — a margin, measured in centimetres or inches.
 * - `dimension` — a vehicle or a gateway, in metres or inches.
 * - `distance` — a street or a trip, in metres or feet.
 */
export type Magnitude = "clearance" | "dimension" | "distance";

export type Unit = "cm" | "m" | "in" | "ft";

/** A length converted, with everything needed to write it out. */
export interface Measure {
  value: number;
  unit: Unit;
  /** How many decimals are worth printing — beyond them is conversion noise. */
  decimals: number;
}

const UNITS: Record<Magnitude, Record<UnitSystem, Unit>> = {
  clearance: { metric: "cm", us: "in" },
  dimension: { metric: "m", us: "in" },
  distance: { metric: "m", us: "ft" },
};

/** Metres in one of the display unit. */
const SCALE: Record<Unit, number> = {
  cm: 0.01,
  m: 1,
  in: INCH_M,
  ft: FOOT_M,
};

/**
 * Decimals worth printing.
 *
 * One in inches and feet rather than two: 2,58 m is 101.5748 in, and every
 * digit past the first comes from the conversion, not from anything anyone
 * measured with a tape.
 */
const DECIMALS: Record<Magnitude, Record<UnitSystem, number>> = {
  clearance: { metric: 1, us: 1 },
  dimension: { metric: 2, us: 1 },
  distance: { metric: 2, us: 1 },
};

/** Which unit a quantity of this kind is shown in. */
export function unitOf(magnitude: Magnitude, system: UnitSystem): Unit {
  return UNITS[magnitude][system];
}

/** A length in metres, as the number the reader sees. */
export function toDisplay(
  metres: number,
  magnitude: Magnitude,
  system: UnitSystem,
): number {
  return metres / SCALE[unitOf(magnitude, system)];
}

/** A number the reader typed, back into the metres the core expects. */
export function fromDisplay(
  value: number,
  magnitude: Magnitude,
  system: UnitSystem,
): number {
  return value * SCALE[unitOf(magnitude, system)];
}

/**
 * How finely a field of this kind can be typed, in display units.
 *
 * A millimetre in metric; a hundredth of an inch in imperial, which is finer
 * still (0.254 mm). Coarser would quietly refuse measurements the other
 * system accepts, and the tool answers to the centimetre.
 */
export function stepFor(magnitude: Magnitude, system: UnitSystem): number {
  if (system === "metric") return magnitude === "clearance" ? 0.1 : 0.001;
  return 0.01;
}

/** A length in metres, converted and ready to be written out. */
export function measure(
  metres: number,
  magnitude: Magnitude,
  system: UnitSystem,
): Measure {
  return {
    value: toDisplay(metres, magnitude, system),
    unit: unitOf(magnitude, system),
    decimals: DECIMALS[magnitude][system],
  };
}
