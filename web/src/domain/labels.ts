/**
 * Every sentence the reader sees that has a figure in it.
 *
 * The fixed strings live in `i18n/dictionary.ts`. What is here is what cannot
 * be looked up: anything built around a measurement or a count, which each
 * language has to assemble in its own word order. Gluing translated fragments
 * together is what makes an interface read like a translation.
 *
 * The core and the boundary return codes, never sentences — that is what
 * keeps the domain free of language, as `CLAUDE.md` requires. All the wording
 * lives here.
 */
import type { Locale, Preferences } from "../i18n/preferences";
import type { ConfidenceDto, ErrorDto } from "./types";
import type { Magnitude } from "./units";
import { measure } from "./units";
import type { Verdict } from "./verdict";

const FIELDS: Record<Locale, Record<string, string>> = {
  fr: {
    wheelbase: "l'empattement",
    length: "la longueur totale",
    front_overhang: "le porte-à-faux avant",
    width: "la largeur de caisse",
    mirror_width: "la largeur aux rétroviseurs",
    ground_clearance: "la garde au sol",
    min_turning_radius: "le rayon de braquage",
  },
  en: {
    wheelbase: "the wheelbase",
    length: "the overall length",
    front_overhang: "the front overhang",
    width: "the body width",
    mirror_width: "the width across the mirrors",
    ground_clearance: "the ground clearance",
    min_turning_radius: "the turning radius",
  },
};

/** A number, with the decimal mark of its language. */
function figure(value: number, decimals: number, locale: Locale): string {
  return new Intl.NumberFormat(locale === "fr" ? "fr-FR" : "en-US", {
    minimumFractionDigits: decimals,
    maximumFractionDigits: decimals,
  }).format(value);
}

/**
 * A length in metres, written in the reader's language and unit.
 *
 * The one way lengths reach the screen. `magnitude` decides the unit — see
 * `units.ts` — so no caller has to know whether it is holding something to be
 * shown in inches or in feet.
 */
export function length(
  metres: number,
  magnitude: Magnitude,
  preferences: Preferences,
): string {
  const shown = measure(metres, magnitude, preferences.units);
  return `${figure(shown.value, shown.decimals, preferences.locale)} ${shown.unit}`;
}

/** Turns a rejected input into a sentence naming what to fix. */
export function errorMessage(error: ErrorDto, locale: Locale): string {
  const named = error.field ? FIELDS[locale][error.field] : undefined;
  if (locale === "en") {
    switch (error.code) {
      case "non_positive":
        // Covers both an empty field and a nonsensical one: the boundary
        // reports the same code for a NaN and for a negative, and a field the
        // vehicle table could not fill arrives here empty.
        return `Fill in ${named ?? "this measurement"}: it is missing, or not greater than zero.`;
      case "front_overhang_too_large":
        return "The front overhang exceeds the length available: check the overall length and the wheelbase.";
      case "mirrors_narrower_than_body":
        return "The width across the mirrors cannot be less than the body width.";
      case "bad_request":
        return "The measurements entered could not be read.";
      case "worker_failed":
        return "The computation stopped. Start the search again.";
      default:
        return "The computation failed for an unexpected reason.";
    }
  }
  switch (error.code) {
    case "non_positive":
      return `Renseigne ${named ?? "cette mesure"} : la mesure manque, ou n'est pas supérieure à zéro.`;
    case "front_overhang_too_large":
      return "Le porte-à-faux avant dépasse la longueur disponible : vérifie la longueur totale et l'empattement.";
    case "mirrors_narrower_than_body":
      return "La largeur aux rétroviseurs ne peut pas être inférieure à la largeur de caisse.";
    case "bad_request":
      return "Les mesures saisies n'ont pas pu être lues.";
    case "worker_failed":
      return "Le calcul s'est interrompu. Relance la recherche.";
    default:
      return "Le calcul a échoué pour une raison inattendue.";
  }
}

/** How a result was obtained, said plainly. */
export function confidenceLabel(confidence: ConfidenceDto, locale: Locale): string {
  if (locale === "en") {
    switch (confidence) {
      case "exact":
        return "exact search";
      case "heuristic":
        return "heuristic search";
      case "heuristic_exhausted":
        return "heuristic search, budget exhausted";
    }
  }
  switch (confidence) {
    case "exact":
      return "recherche exacte";
    case "heuristic":
      return "recherche heuristique";
    case "heuristic_exhausted":
      return "recherche heuristique, budget épuisé";
  }
}

/** "1 manœuvre" / "3 moves". */
export function moves(count: number, locale: Locale): string {
  if (locale === "en") return `${count} move${count > 1 ? "s" : ""}`;
  return `${count} manœuvre${count > 1 ? "s" : ""}`;
}

/** "under 25 cm" / "sous 10 in", with the threshold in the reader's unit. */
export function underThreshold(
  threshold: number,
  preferences: Preferences,
): string {
  const shown = length(threshold, "clearance", preferences);
  return preferences.locale === "en" ? `Under ${shown}` : `Sous ${shown}`;
}

/**
 * The headline: the question the visitor came with, answered in three words.
 *
 * "Rien trouvé" rather than "Ça ne passe pas" for an exhausted budget,
 * because the two are not the same claim and only one of them is proved.
 */
export function verdictHeadline(verdict: Verdict, locale: Locale): string {
  const english = locale === "en";
  switch (verdict.outcome) {
    case "passes":
      return english ? "It fits." : "Ça passe.";
    case "too-narrow":
    case "blocked":
      return english ? "It does not fit." : "Ça ne passe pas.";
    case "unproven":
      return english ? "Nothing found." : "Rien trouvé.";
  }
}

/**
 * The second half of the headline, set in the accent colour.
 *
 * Null is a real answer: a comfortable pass needs no qualifier, and inventing
 * one would make every result sound like a warning.
 */
export function verdictNuance(verdict: Verdict, locale: Locale): string | null {
  const english = locale === "en";
  if (verdict.outcome === "too-narrow") {
    return english ? "The vehicle is too wide." : "Le véhicule est trop large.";
  }
  if (verdict.outcome === "blocked") return null;
  if (verdict.outcome === "unproven") return english ? "Not proven." : "Sans preuve.";
  switch (verdict.tone) {
    case "roomy":
      return english ? "Easily." : "Largement.";
    case "fine":
      return null;
    case "snug":
      return english ? "Not comfortably." : "Sans confort.";
    case "hairline":
      return english ? "Barely." : "De justesse.";
  }
}

/** The line under the headline: how many moves, how much room, and from where. */
export function verdictDetail(verdict: Verdict, preferences: Preferences): string {
  const { locale } = preferences;
  const english = locale === "en";

  switch (verdict.outcome) {
    case "passes": {
      const room = length(verdict.clearance, "clearance", preferences);
      const from = confidenceLabel(verdict.confidence, locale);
      const count = moves(verdict.moves, locale);
      if (verdict.tightestElsewhere === null) {
        return english
          ? `${count}, ${room} of clearance in the gateway (${from}).`
          : `${count}, ${room} de marge dans le passage (${from}).`;
      }
      const tighter = length(verdict.tightestElsewhere, "clearance", preferences);
      return english
        ? `${count}, ${room} of clearance in the gateway (${from}). Elsewhere on the trip it drops to ${tighter} — on the street, not in the gateway.`
        : `${count}, ${room} de marge dans le passage (${from}). Ailleurs sur le trajet, la marge descend à ${tighter} — sur la voirie, pas dans le passage.`;
    }
    case "too-narrow": {
      const wide = length(verdict.vehicleWidth, "dimension", preferences);
      const gap = length(verdict.opening, "dimension", preferences);
      return english
        ? `The vehicle is ${wide} at its widest, for an opening of ${gap}. No manoeuvre changes that: the greatest possible clearance is (opening − vehicle) ÷ 2, whatever the trajectory. There is nothing to search for.`
        : `Le véhicule mesure ${wide} à son point le plus large, pour un passage de ${gap}. Aucune manœuvre n'y change rien : la marge maximale est (passage − véhicule) ÷ 2, quelle que soit la trajectoire. Inutile de chercher.`;
    }
    case "blocked":
      return english
        ? "No entry is possible with these measurements."
        : "Aucune entrée n'est possible avec ces mesures.";
    case "unproven":
      return english
        ? "No entry found within the budget. The search is heuristic: this does not prove that none exists."
        : "Aucune entrée trouvée dans le budget imparti. La recherche est heuristique : cela ne prouve pas que l'entrée soit impossible.";
  }
}

/** "2 mesures manquent, signalées en rouge dans le formulaire." */
export function missingMeasurements(count: number, locale: Locale): string {
  if (locale === "en") {
    return count === 1
      ? "One measurement is missing, marked in red in the form."
      : `${count} measurements are missing, marked in red in the form.`;
  }
  return count === 1
    ? "Une mesure manque, signalée en rouge dans le formulaire."
    : `${count} mesures manquent, signalées en rouge dans le formulaire.`;
}

/** What the narrowest-roadway search came back with. */
export function minRoadResult(
  width: number | null,
  preferences: Preferences,
): string {
  const english = preferences.locale === "en";
  if (width === null) {
    return english
      ? "No roadway width allows a single-move entry: the gateway itself is what blocks."
      : "Aucune largeur de chaussée ne permet l'entrée en un mouvement : le passage lui-même est bloquant.";
  }
  const shown = length(width, "distance", preferences);
  return english
    ? `A single-move entry needs at least ${shown} of roadway.`
    : `Il faut au minimum ${shown} de chaussée pour entrer en un seul mouvement.`;
}

/** Refuses a leaf angle the post itself forbids. */
export function leafTooOpen(
  degrees: number,
  maximum: number,
  locale: Locale,
): string {
  const asked = degrees.toFixed(0);
  return locale === "en"
    ? `A leaf cannot open to ${asked}° on this hinge: it would pass through the post. The most is ${maximum}°.`
    : `Un vantail ne peut pas s'ouvrir à ${asked}° avec cet axe : il traverserait le pilier. Le maximum est ${maximum}°.`;
}

/**
 * How far the planner has got.
 *
 * "Situations" rather than nodes: what the planner counts is the vehicle
 * placed somewhere, facing some way, in some gear. And the ceiling is named,
 * because a running count without its scale says nothing about where it ends.
 */
export function searchProgress(
  moveCount: number,
  expanded: number,
  budget: number,
  locale: Locale,
): string {
  const tag = locale === "en" ? "en-US" : "fr-FR";
  const tried = expanded.toLocaleString(tag);
  const ceiling = budget.toLocaleString(tag);
  return locale === "en"
    ? `Computing ${moveCount}-move trajectories — ${tried} situations tried, of at most ${ceiling}`
    : `Calcul des trajectoires en ${moveCount} manœuvres — ${tried} situations essayées sur ${ceiling} au plus`;
}
