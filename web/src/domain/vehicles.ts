import type { VehicleDto } from "./types";

/**
 * The six vehicles the prototype shipped with (`index.html:158-165`).
 *
 * PROVISIONAL. Mirror width is derived rather than measured for most of
 * these, and front overhang is estimated — `data/vehicles.json` records the
 * provenance field by field and supersedes this table in batch 5.
 *
 * Two minutes with a tape measure across the mirrors would be worth more than
 * any of it: `CLAUDE.md` notes that 3 cm of error there inverts a conclusion.
 */
export interface VehiclePreset extends VehicleDto {
  id: string;
  label: string;
  /** Width over the mirrors once folded, in metres. */
  mirror_width_folded: number;
}

/**
 * Folded mirrors, when unmeasured, are assumed to add this much to the body.
 *
 * ARBITRARY — carried over from the prototype (`index.html:178`), which had
 * no measurement behind it either. It decides whether a passage impossible
 * with mirrors out becomes possible with them folded, so it deserves a real
 * measurement.
 */
const FOLDED_MARGIN_M = 0.04;

function preset(
  id: string,
  label: string,
  wheelbase: number,
  length: number,
  front_overhang: number,
  width: number,
  mirror_width: number,
  min_turning_radius: number,
): VehiclePreset {
  return {
    id,
    label,
    wheelbase,
    length,
    front_overhang,
    width,
    mirror_width,
    mirror_width_folded: width + FOLDED_MARGIN_M,
    min_turning_radius,
  };
}

export const VEHICLES: readonly VehiclePreset[] = [
  preset("lexus-lbx", "Lexus LBX", 2.58, 4.19, 0.85, 1.825, 2.029, 5.2),
  preset("kia-ev3", "Kia EV3", 2.68, 4.3, 0.83, 1.85, 2.06, 5.2),
  preset("renault-scenic-4", "Renault Scénic IV", 2.734, 4.406, 0.9, 1.866, 2.128, 5.6),
  preset("lexus-nx-450h-plus", "Lexus NX 450h+", 2.69, 4.66, 0.99, 1.865, 2.15, 5.7),
  preset("tesla-model-y", "Tesla Model Y", 2.89, 4.79, 0.94, 1.982, 2.129, 5.8),
  preset("skoda-superb-combi", "Skoda Superb Combi", 2.841, 4.902, 0.94, 1.849, 2.12, 5.7),
];

/** Looks a preset up by id. */
export function vehicleById(id: string): VehiclePreset | undefined {
  return VEHICLES.find((v) => v.id === id);
}
