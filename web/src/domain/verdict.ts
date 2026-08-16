/**
 * What a search result is worth, decided without a word of French.
 *
 * `labels.ts` owns the sentences; this module owns the judgement behind them.
 * Keeping the two apart is what lets the wording be translated later without
 * anyone having to re-derive which margin counts as tight.
 */
import { bandOf } from "./bands";
import type { ConfidenceDto, SolveResponse } from "./types";

/**
 * How comfortable an entry is, from the proximity band of its tightest point
 * in the gateway.
 *
 * The four tones are the four bands, so the sentence and the colour of the
 * path always agree.
 */
export type VerdictTone = "roomy" | "fine" | "snug" | "hairline";

const TONES: readonly VerdictTone[] = ["roomy", "fine", "snug", "hairline"];

export type Verdict =
  | {
      outcome: "passes";
      tone: VerdictTone;
      moves: number;
      /** Tightest point inside the gateway, in metres — what was asked about. */
      clearance: number;
      confidence: ConfidenceDto;
      /**
       * Tightest point anywhere else on the trip, in metres, when the trip is
       * tighter than the gateway. Null when the gateway is the worst of it.
       *
       * Kept apart because it answers a different question: the driver asked
       * whether the gate admits the car, not whether the street does.
       */
      tightestElsewhere: number | null;
    }
  /**
   * The vehicle is at least as wide as the opening. Settled before any
   * search, and settled for good — see [`refuseOnWidth`].
   */
  | { outcome: "too-narrow"; opening: number; vehicleWidth: number }
  /** Searched exhaustively and found nothing: no entry exists. */
  | { outcome: "blocked" }
  /** Ran out of budget. Nothing found, and nothing proved either. */
  | { outcome: "unproven" };

/**
 * The most clearance any trajectory can leave in the gateway, in metres.
 *
 * `(W − w) / 2` — half of what the opening leaves beside the vehicle, since
 * the margin is counted on one side. It is a property of the two widths
 * alone: no manoeuvre, however clever, buys room that is not there. This is
 * the project's main conclusion, and it is what gives the displayed margin a
 * scale.
 *
 * Floored at zero: a vehicle wider than its opening has no room, not negative
 * room, and a negative ceiling would send the gauge marker off its scale.
 */
export function clearanceCeiling(opening: number, mirrorWidth: number): number {
  return Math.max((opening - mirrorWidth) / 2, 0);
}

/** Where a clearance sits on the gauge, from 0 at nothing to 1 at the ceiling. */
export function gaugeFraction(clearance: number, ceiling: number): number {
  // A zero ceiling is a real case — an opening no wider than the vehicle —
  // and dividing by it would hand the renderer a NaN.
  if (ceiling <= 0) return 0;
  return Math.min(Math.max(clearance / ceiling, 0), 1);
}

/** Judges a completed search. */
export function verdictOf(response: SolveResponse): Verdict {
  const best = response.alternatives[0];
  if (!best) {
    // An exhausted budget proves nothing: the planner is heuristic, and its
    // failing to find an entry is not the same as there being none.
    return response.budget_exhausted ? { outcome: "unproven" } : { outcome: "blocked" };
  }

  const tighter = best.min_clearance < best.min_clearance_in_gateway - 1e-9;
  return {
    outcome: "passes",
    tone: TONES[bandOf(best.min_clearance_in_gateway)]!,
    moves: best.moves,
    clearance: best.min_clearance_in_gateway,
    confidence: best.confidence,
    tightestElsewhere: tighter ? best.min_clearance : null,
  };
}

/**
 * Refuses, before any search, a gateway the vehicle cannot fit through.
 *
 * The ceiling `(W − w) / 2` is nil or negative here, and it is a ceiling: no
 * trajectory, no number of manoeuvres and no angle of approach beats it. So
 * the impossibility is *proved*, and proved more strongly than a search could
 * — the planner is heuristic, and its finding nothing proves nothing.
 *
 * Running the search anyway would spend a budget to reach a weaker
 * conclusion. Pass the mirror width the run would actually use, folded or
 * not: a car that does not fit with its mirrors out may well fit with them in.
 */
export function refuseOnWidth(opening: number, mirrorWidth: number): Verdict | null {
  // Equality refuses too. Fitting with exactly nothing to spare is not
  // fitting: it is touching both posts at once, for the whole depth.
  if (mirrorWidth < opening) return null;
  return { outcome: "too-narrow", opening, vehicleWidth: mirrorWidth };
}
