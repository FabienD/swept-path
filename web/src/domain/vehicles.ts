/**
 * The vehicle table, read from `data/vehicles.json`.
 *
 * That file is the single source: it records provenance field by field, and
 * says `null` where nobody has published a figure. This module only
 * translates — it shapes the entries for the form and derives the one value
 * the solver needs but no manufacturer prints, the rear-axle pivot radius.
 *
 * **Nothing here invents a number.** A field the database does not know stays
 * `null` all the way to the form, which then leaves its input alone rather
 * than asserting a default the driver would take for measured.
 */
import table from "../../../data/vehicles.json";

/**
 * How much narrower the track is than the body, in metres.
 *
 * ESTIMATED. Wheels sit inboard of the bodywork; the gap is roughly a
 * handspan each side on a modern car. Only used to convert a published
 * turning radius, where an error of a few centimetres moves the result by
 * about as much.
 */
const BODY_TO_TRACK_M = 0.26;

/** What a published turning radius is measured on. */
type RadiusKind = "kerb" | "wall" | "pivot" | null;

/** One entry, with `null` wherever the database has no figure. */
export interface VehiclePreset {
  id: string;
  label: string;
  body: string;
  wheelbase: number | null;
  length: number | null;
  front_overhang: number | null;
  width: number | null;
  mirror_width: number | null;
  mirror_width_folded: number | null;
  ground_clearance: number | null;
  /** Radius traced by the rear axle centre, derived. `null` when it cannot be. */
  min_turning_radius: number | null;
  /** The figure as published, kept so the interface can say where its own came from. */
  published_radius: number | null;
  published_radius_kind: RadiusKind;
}

/**
 * Converts a kerb-to-kerb radius into the rear-axle pivot radius.
 *
 * Mirrors `swept_core::vehicle::pivot_radius_from_kerb`. Manufacturers
 * publish the circle traced by the outer front wheel; the bicycle model turns
 * about the rear axle, which runs well inside it. Using the published figure
 * directly makes every vehicle turn about half again as wide as it really
 * can — and the simulator then invents manoeuvres to make up for it.
 */
function pivotRadius(kerbRadius: number, wheelbase: number, width: number): number | null {
  const track = width - BODY_TO_TRACK_M;
  const atFrontAxle = kerbRadius - track / 2;
  const squared = atFrontAxle * atFrontAxle - wheelbase * wheelbase;
  return squared > 0 ? Math.sqrt(squared) : null;
}

/**
 * The pivot radius, or `null` when the database cannot supply one.
 *
 * A `wall` radius is traced by the bodywork and needs a conversion this code
 * does not have, so it yields nothing rather than a figure that would be
 * quietly too large.
 */
function pivotFrom(
  published: number | null,
  kind: RadiusKind,
  wheelbase: number | null,
  width: number | null,
): number | null {
  if (published === null) return null;
  if (kind === "pivot") return published;
  if (kind !== "kerb" || wheelbase === null || width === null) return null;
  return pivotRadius(published, wheelbase, width);
}

/** A field as the database stores it: a value that may be absent. */
interface Field {
  v: number | null;
}

const value = (field: Field | undefined): number | null => field?.v ?? null;

export const VEHICLES: readonly VehiclePreset[] = table.vehicles.map((entry) => {
  const wheelbase = value(entry.wheelbase);
  const width = value(entry.width);
  const published = value(entry.turning_radius);
  const kind = (entry.turning_radius.kind ?? null) as RadiusKind;
  return {
    id: entry.id,
    label: `${entry.make} ${entry.model}`,
    body: entry.body,
    wheelbase,
    length: value(entry.length),
    front_overhang: value(entry.front_overhang),
    width,
    mirror_width: value(entry.width_mirrors),
    mirror_width_folded: value(entry.width_mirrors_folded),
    ground_clearance: value(entry.ground_clearance),
    min_turning_radius: pivotFrom(published, kind, wheelbase, width),
    published_radius: published,
    published_radius_kind: kind,
  };
});

/** Looks a preset up by id. */
export function vehicleById(id: string): VehiclePreset | undefined {
  return VEHICLES.find((v) => v.id === id);
}

/**
 * The entries whose make or model matches `query`.
 *
 * Case- and accent-insensitive, so that "911", "lexus" and "ev" all work
 * without the driver having to know how the table spells things.
 */
export function searchVehicles(query: string): readonly VehiclePreset[] {
  const needle = fold(query);
  if (needle === "") return VEHICLES;
  return VEHICLES.filter((v) => fold(v.label).includes(needle));
}

function fold(text: string): string {
  return text
    .normalize("NFD")
    .replace(/\p{Diacritic}/gu, "")
    .toLowerCase()
    .trim();
}
