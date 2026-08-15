/**
 * Every word the user reads.
 *
 * The core and the boundary return codes, never sentences: that is what keeps
 * the domain free of language, as `CLAUDE.md` requires. All the French lives
 * here.
 */
import type { ConfidenceDto, ErrorDto } from "./types";

const FIELDS: Record<string, string> = {
  wheelbase: "l'empattement",
  length: "la longueur totale",
  front_overhang: "le porte-à-faux avant",
  width: "la largeur de caisse",
  mirror_width: "la largeur aux rétroviseurs",
  ground_clearance: "la garde au sol",
  min_turning_radius: "le rayon de braquage",
};

/** Turns a rejected input into a sentence naming what to fix. */
export function errorMessage(error: ErrorDto): string {
  const field = error.field ? (FIELDS[error.field] ?? error.field) : null;
  switch (error.code) {
    case "non_positive":
      // Covers both an empty field and a nonsensical one: the boundary
      // reports the same code for a NaN and for a negative, and a field the
      // vehicle table could not fill arrives here empty.
      return `Renseigne ${field ?? "cette mesure"} : la mesure manque, ou n'est pas supérieure à zéro.`;
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
export function confidenceLabel(confidence: ConfidenceDto): string {
  switch (confidence) {
    case "exact":
      return "recherche exacte";
    case "heuristic":
      return "recherche heuristique";
    case "heuristic_exhausted":
      return "recherche heuristique, budget épuisé";
  }
}

/** A length in metres, shown in centimetres as the tool reports them. */
export function centimetres(metres: number): string {
  return `${(metres * 100).toFixed(1).replace(".", ",")} cm`;
}

/** A length in metres, shown as metres. */
export function metres(value: number): string {
  return `${value.toFixed(2).replace(".", ",")} m`;
}

/** "1 manœuvre" / "3 manœuvres". */
export function moves(count: number): string {
  return `${count} manœuvre${count > 1 ? "s" : ""}`;
}
